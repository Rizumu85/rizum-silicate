use crate::export::ffmpeg::{
    FfmpegCommand, FfmpegCommandRunError, FfmpegCommandRunner, FfmpegToolStatus,
};
use silica::{
    error::SilicaError,
    video::{list_archived_video_segments, stream_archived_video_segments},
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_STAGE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedVideoMergePlan {
    pub concat_list_path: PathBuf,
    pub concat_list: String,
    pub command: FfmpegCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchivedVideoMergePlanError {
    NoSegments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchivedVideoExportMode {
    FullLength,
    Preview30Seconds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedVideoStaging {
    pub segment_paths: Vec<PathBuf>,
    pub concat_list_path: PathBuf,
    pub concat_list: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedVideoMergeResult {
    pub staging: ArchivedVideoStaging,
    pub command: FfmpegCommand,
}

#[derive(Debug)]
pub enum ArchivedVideoStageError {
    Archive(SilicaError),
    Io(io::Error),
    NoSegments,
}

#[derive(Debug)]
pub enum ArchivedVideoMergeError {
    Stage(ArchivedVideoStageError),
    Plan(ArchivedVideoMergePlanError),
    Run(FfmpegCommandRunError),
}

#[derive(Debug)]
pub enum ArchivedVideoExportError {
    MissingFfmpegTool { detail: String },
    Merge(ArchivedVideoMergeError),
}

pub trait ArchivedVideoStageWriter {
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()>;
    fn write_file(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    fn write_stream(&mut self, path: &Path, reader: &mut dyn io::Read) -> io::Result<()> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        self.write_file(path, &bytes)
    }
}

#[derive(Debug)]
pub struct ArchivedVideoStageDirectory {
    path: PathBuf,
}

impl ArchivedVideoStageDirectory {
    pub fn create(temp_root: &Path, output_path: &Path) -> io::Result<Self> {
        let root = temp_root.join("rizum-silicate").join("archived-video");
        fs::create_dir_all(&root)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let slug = export_output_stem_slug(output_path);
        for _ in 0..128 {
            let sequence = NEXT_STAGE_ID.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "{slug}-{}-{timestamp:x}-{sequence:x}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not reserve a unique archived-video staging directory",
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ArchivedVideoStageDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path)
            && error.kind() != io::ErrorKind::NotFound
        {
            log::warn!(
                "Could not clean archived-video staging directory {}: {error}",
                self.path.display()
            );
        }
    }
}

pub fn build_archived_video_merge_plan(
    ffmpeg_path: &Path,
    concat_list_path: &Path,
    ordered_segments: &[PathBuf],
    output_path: &Path,
) -> Result<ArchivedVideoMergePlan, ArchivedVideoMergePlanError> {
    build_archived_video_merge_plan_for_mode(
        ffmpeg_path,
        concat_list_path,
        ordered_segments,
        output_path,
        ArchivedVideoExportMode::FullLength,
    )
}

pub fn build_archived_video_merge_plan_for_mode(
    ffmpeg_path: &Path,
    concat_list_path: &Path,
    ordered_segments: &[PathBuf],
    output_path: &Path,
    export_mode: ArchivedVideoExportMode,
) -> Result<ArchivedVideoMergePlan, ArchivedVideoMergePlanError> {
    if ordered_segments.is_empty() {
        return Err(ArchivedVideoMergePlanError::NoSegments);
    }

    let concat_list = build_concat_list(ordered_segments);
    let mut args = vec![
        "-hide_banner".to_owned(),
        "-y".to_owned(),
        "-f".to_owned(),
        "concat".to_owned(),
        "-safe".to_owned(),
        "0".to_owned(),
        "-i".to_owned(),
        concat_list_path.to_string_lossy().into_owned(),
        "-c".to_owned(),
        "copy".to_owned(),
    ];
    append_export_mode_args(export_mode, &mut args);
    args.push(output_path.to_string_lossy().into_owned());

    Ok(ArchivedVideoMergePlan {
        concat_list_path: concat_list_path.to_owned(),
        concat_list,
        command: FfmpegCommand {
            program: ffmpeg_path.to_owned(),
            args,
        },
    })
}

pub fn stage_archived_video_segments(
    archive_bytes: &[u8],
    stage_dir: &Path,
    writer: &mut impl ArchivedVideoStageWriter,
) -> Result<ArchivedVideoStaging, ArchivedVideoStageError> {
    let segments = list_archived_video_segments(archive_bytes)?;
    if segments.is_empty() {
        return Err(ArchivedVideoStageError::NoSegments);
    }

    writer.create_dir_all(stage_dir)?;

    let mut segment_paths = Vec::with_capacity(segments.len());
    stream_archived_video_segments(archive_bytes, |segment, reader| {
        let segment_path = stage_dir.join(archived_segment_file_name(&segment.path));
        writer.write_stream(&segment_path, reader)?;
        segment_paths.push(segment_path);
        Ok(())
    })?;

    let concat_list_path = stage_dir.join("segments.ffconcat");
    let concat_list = build_concat_list(&segment_paths);
    writer.write_file(&concat_list_path, concat_list.as_bytes())?;

    Ok(ArchivedVideoStaging {
        segment_paths,
        concat_list_path,
        concat_list,
    })
}

pub fn merge_archived_video_segments(
    archive_bytes: &[u8],
    stage_dir: &Path,
    ffmpeg_path: &Path,
    output_path: &Path,
    writer: &mut impl ArchivedVideoStageWriter,
    runner: &mut impl FfmpegCommandRunner,
) -> Result<ArchivedVideoMergeResult, ArchivedVideoMergeError> {
    merge_archived_video_segments_with_mode(
        archive_bytes,
        stage_dir,
        ffmpeg_path,
        output_path,
        ArchivedVideoExportMode::FullLength,
        writer,
        runner,
    )
}

pub fn merge_archived_video_segments_with_mode(
    archive_bytes: &[u8],
    stage_dir: &Path,
    ffmpeg_path: &Path,
    output_path: &Path,
    export_mode: ArchivedVideoExportMode,
    writer: &mut impl ArchivedVideoStageWriter,
    runner: &mut impl FfmpegCommandRunner,
) -> Result<ArchivedVideoMergeResult, ArchivedVideoMergeError> {
    let staging = stage_archived_video_segments(archive_bytes, stage_dir, writer)?;
    let plan = build_archived_video_merge_plan_for_mode(
        ffmpeg_path,
        &staging.concat_list_path,
        &staging.segment_paths,
        output_path,
        export_mode,
    )?;
    runner.run(&plan.command)?;

    Ok(ArchivedVideoMergeResult {
        staging,
        command: plan.command,
    })
}

pub fn export_archived_video_segments_with_ffmpeg_status(
    archive_bytes: &[u8],
    stage_dir: &Path,
    ffmpeg_status: &FfmpegToolStatus,
    output_path: &Path,
    writer: &mut impl ArchivedVideoStageWriter,
    runner: &mut impl FfmpegCommandRunner,
) -> Result<ArchivedVideoMergeResult, ArchivedVideoExportError> {
    export_archived_video_segments_with_ffmpeg_status_and_mode(
        archive_bytes,
        stage_dir,
        ffmpeg_status,
        output_path,
        ArchivedVideoExportMode::FullLength,
        writer,
        runner,
    )
}

pub fn export_archived_video_segments_with_ffmpeg_status_and_mode(
    archive_bytes: &[u8],
    stage_dir: &Path,
    ffmpeg_status: &FfmpegToolStatus,
    output_path: &Path,
    export_mode: ArchivedVideoExportMode,
    writer: &mut impl ArchivedVideoStageWriter,
    runner: &mut impl FfmpegCommandRunner,
) -> Result<ArchivedVideoMergeResult, ArchivedVideoExportError> {
    let Some(ffmpeg_path) = ffmpeg_status.executable_path.as_deref() else {
        return Err(ArchivedVideoExportError::MissingFfmpegTool {
            detail: ffmpeg_status.detail.clone(),
        });
    };

    merge_archived_video_segments_with_mode(
        archive_bytes,
        stage_dir,
        ffmpeg_path,
        output_path,
        export_mode,
        writer,
        runner,
    )
    .map_err(ArchivedVideoExportError::Merge)
}

pub fn archived_video_segment_count(archive_bytes: &[u8]) -> Result<usize, SilicaError> {
    list_archived_video_segments(archive_bytes).map(|segments| segments.len())
}

pub struct FsArchivedVideoStageWriter;

impl ArchivedVideoStageWriter for FsArchivedVideoStageWriter {
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn write_file(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        fs::write(path, bytes)
    }

    fn write_stream(&mut self, path: &Path, reader: &mut dyn io::Read) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        io::copy(reader, &mut file)?;
        Ok(())
    }
}

impl From<SilicaError> for ArchivedVideoStageError {
    fn from(error: SilicaError) -> Self {
        Self::Archive(error)
    }
}

impl From<io::Error> for ArchivedVideoStageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ArchivedVideoStageError> for ArchivedVideoMergeError {
    fn from(error: ArchivedVideoStageError) -> Self {
        Self::Stage(error)
    }
}

impl From<ArchivedVideoMergePlanError> for ArchivedVideoMergeError {
    fn from(error: ArchivedVideoMergePlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<FfmpegCommandRunError> for ArchivedVideoMergeError {
    fn from(error: FfmpegCommandRunError) -> Self {
        Self::Run(error)
    }
}

fn build_concat_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| format!("file '{}'\n", escape_concat_file_path(path)))
        .collect()
}

fn append_export_mode_args(export_mode: ArchivedVideoExportMode, args: &mut Vec<String>) {
    match export_mode {
        ArchivedVideoExportMode::FullLength => {}
        ArchivedVideoExportMode::Preview30Seconds => {
            args.push("-t".to_owned());
            args.push("30".to_owned());
        }
    }
}

fn archived_segment_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn export_output_stem_slug(output_path: &Path) -> String {
    let stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("untitled-artwork");

    let mut slug = String::with_capacity(stem.len());
    let mut last_was_separator = false;
    for character in stem.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            slug.push('-');
            last_was_separator = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "untitled-artwork".to_owned()
    } else {
        slug.to_owned()
    }
}

fn escape_concat_file_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn builds_archived_video_merge_plan_for_ordered_segments() {
        let ffmpeg_path = PathBuf::from(r"C:\Rizum\tools\ffmpeg.exe");
        let concat_list_path = PathBuf::from(r"C:\Temp\rizum-segments.txt");
        let output_path = PathBuf::from(r"C:\Exports\Artwork.mp4");
        let segments = [
            PathBuf::from(r"C:\Temp\segment-1.mp4"),
            PathBuf::from(r"C:\Temp\segment-2.mp4"),
            PathBuf::from(r"C:\Temp\segment-10.mp4"),
        ];

        let plan = build_archived_video_merge_plan(
            &ffmpeg_path,
            &concat_list_path,
            &segments,
            &output_path,
        )
        .unwrap();

        assert_eq!(
            plan.concat_list,
            "file 'C:\\Temp\\segment-1.mp4'\nfile 'C:\\Temp\\segment-2.mp4'\nfile 'C:\\Temp\\segment-10.mp4'\n"
        );
        assert_eq!(plan.concat_list_path, concat_list_path);
        assert_eq!(
            plan.command,
            FfmpegCommand {
                program: ffmpeg_path,
                args: vec![
                    "-hide_banner".to_owned(),
                    "-y".to_owned(),
                    "-f".to_owned(),
                    "concat".to_owned(),
                    "-safe".to_owned(),
                    "0".to_owned(),
                    "-i".to_owned(),
                    r"C:\Temp\rizum-segments.txt".to_owned(),
                    "-c".to_owned(),
                    "copy".to_owned(),
                    r"C:\Exports\Artwork.mp4".to_owned(),
                ],
            }
        );
    }

    #[test]
    fn builds_thirty_second_archived_video_merge_plan() {
        let ffmpeg_path = PathBuf::from("/tools/ffmpeg");
        let concat_list_path = PathBuf::from("/tmp/segments.ffconcat");
        let output_path = PathBuf::from("/exports/Artwork Preview.mp4");
        let segments = [PathBuf::from("/tmp/segment-1.mp4")];

        let plan = build_archived_video_merge_plan_for_mode(
            &ffmpeg_path,
            &concat_list_path,
            &segments,
            &output_path,
            ArchivedVideoExportMode::Preview30Seconds,
        )
        .unwrap();

        assert_eq!(
            plan.command,
            FfmpegCommand {
                program: ffmpeg_path,
                args: vec![
                    "-hide_banner".to_owned(),
                    "-y".to_owned(),
                    "-f".to_owned(),
                    "concat".to_owned(),
                    "-safe".to_owned(),
                    "0".to_owned(),
                    "-i".to_owned(),
                    "/tmp/segments.ffconcat".to_owned(),
                    "-c".to_owned(),
                    "copy".to_owned(),
                    "-t".to_owned(),
                    "30".to_owned(),
                    "/exports/Artwork Preview.mp4".to_owned(),
                ],
            }
        );
    }

    #[test]
    fn derives_staging_slug_from_export_output_path() {
        assert_eq!(
            export_output_stem_slug(Path::new("/exports/Artwork Preview.mp4")),
            "artwork-preview"
        );
    }

    #[test]
    fn counts_available_archived_video_segments_without_extracting_bytes() {
        let archive = zip_with_files([
            ("video/segments/segment-2.mp4", b"two".as_slice()),
            ("QuickLook/Preview.png", b"png".as_slice()),
            ("video/segments/segment-1.mp4", b"one".as_slice()),
        ]);

        let count = archived_video_segment_count(&archive).unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn reports_zero_available_archived_video_segments_for_still_archives() {
        let archive = zip_with_files([("QuickLook/Preview.png", b"png".as_slice())]);

        let count = archived_video_segment_count(&archive).unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn rejects_empty_archived_video_segment_plans() {
        let error = build_archived_video_merge_plan(
            Path::new("ffmpeg"),
            Path::new("segments.txt"),
            &[],
            Path::new("output.mp4"),
        )
        .unwrap_err();

        assert_eq!(error, ArchivedVideoMergePlanError::NoSegments);
    }

    #[test]
    fn escapes_single_quotes_in_concat_list_paths() {
        let plan = build_archived_video_merge_plan(
            Path::new("ffmpeg"),
            Path::new("segments.txt"),
            &[PathBuf::from("/tmp/artist's segment.mp4")],
            Path::new("output.mp4"),
        )
        .unwrap();

        assert_eq!(plan.concat_list, "file '/tmp/artist'\\''s segment.mp4'\n");
    }

    #[test]
    fn stages_archived_video_segments_and_concat_list_in_numeric_order() {
        let archive = zip_with_files([
            ("video/segments/segment-10.mp4", b"ten".as_slice()),
            ("video/segments/segment-2.mp4", b"two".as_slice()),
            ("video/segments/segment-1.mp4", b"one".as_slice()),
        ]);
        let stage_dir = PathBuf::from("/tmp/rizum-archived-video");
        let mut writer = FakeStageWriter::default();

        let staged = stage_archived_video_segments(&archive, &stage_dir, &mut writer).unwrap();

        assert_eq!(
            staged.segment_paths,
            vec![
                stage_dir.join("segment-1.mp4"),
                stage_dir.join("segment-2.mp4"),
                stage_dir.join("segment-10.mp4"),
            ]
        );
        assert_eq!(staged.concat_list_path, stage_dir.join("segments.ffconcat"));
        assert_eq!(
            writer.created_dirs,
            vec![PathBuf::from("/tmp/rizum-archived-video")]
        );
        let expected_concat_list = format!(
            "file '{}'\nfile '{}'\nfile '{}'\n",
            stage_dir.join("segment-1.mp4").to_string_lossy(),
            stage_dir.join("segment-2.mp4").to_string_lossy(),
            stage_dir.join("segment-10.mp4").to_string_lossy(),
        );
        assert_eq!(staged.concat_list, expected_concat_list);
        assert_eq!(
            writer.writes,
            vec![
                (stage_dir.join("segment-1.mp4"), b"one".to_vec()),
                (stage_dir.join("segment-2.mp4"), b"two".to_vec()),
                (stage_dir.join("segment-10.mp4"), b"ten".to_vec()),
                (
                    stage_dir.join("segments.ffconcat"),
                    expected_concat_list.into_bytes(),
                ),
            ]
        );
    }

    #[test]
    fn rejects_archives_without_video_segments_before_writing_stage_files() {
        let archive = zip_with_files([("QuickLook/Preview.png", b"png".as_slice())]);
        let mut writer = FakeStageWriter::default();

        let error =
            stage_archived_video_segments(&archive, Path::new("/tmp/rizum-empty"), &mut writer)
                .unwrap_err();

        assert!(matches!(error, ArchivedVideoStageError::NoSegments));
        assert!(writer.created_dirs.is_empty());
        assert!(writer.writes.is_empty());
    }

    #[test]
    fn merges_archived_video_by_staging_segments_then_running_ffmpeg() {
        let archive = zip_with_files([
            ("video/segments/segment-2.mp4", b"two".as_slice()),
            ("video/segments/segment-1.mp4", b"one".as_slice()),
        ]);
        let stage_dir = PathBuf::from("/tmp/rizum-merge");
        let ffmpeg_path = PathBuf::from("/tools/ffmpeg");
        let output_path = PathBuf::from("/exports/artwork.mp4");
        let mut writer = FakeStageWriter::default();
        let mut runner = FakeFfmpegCommandRunner::default();

        let result = merge_archived_video_segments(
            &archive,
            &stage_dir,
            &ffmpeg_path,
            &output_path,
            &mut writer,
            &mut runner,
        )
        .unwrap();

        assert_eq!(
            result.staging.segment_paths,
            vec![
                stage_dir.join("segment-1.mp4"),
                stage_dir.join("segment-2.mp4")
            ]
        );
        assert_eq!(
            runner.commands,
            vec![FfmpegCommand {
                program: ffmpeg_path,
                args: vec![
                    "-hide_banner".to_owned(),
                    "-y".to_owned(),
                    "-f".to_owned(),
                    "concat".to_owned(),
                    "-safe".to_owned(),
                    "0".to_owned(),
                    "-i".to_owned(),
                    stage_dir
                        .join("segments.ffconcat")
                        .to_string_lossy()
                        .into_owned(),
                    "-c".to_owned(),
                    "copy".to_owned(),
                    output_path.to_string_lossy().into_owned(),
                ],
            }]
        );
        assert_eq!(result.command, runner.commands[0]);
    }

    #[test]
    fn reports_ffmpeg_failure_from_archived_video_merge_job() {
        let archive = zip_with_files([("video/segments/segment-1.mp4", b"one".as_slice())]);
        let ffmpeg_path = PathBuf::from("/tools/ffmpeg");
        let mut writer = FakeStageWriter::default();
        let mut runner = FakeFfmpegCommandRunner {
            fail_with_message: Some("planned ffmpeg failure".to_owned()),
            ..Default::default()
        };

        let error = merge_archived_video_segments(
            &archive,
            Path::new("/tmp/rizum-merge-fail"),
            &ffmpeg_path,
            Path::new("/exports/artwork.mp4"),
            &mut writer,
            &mut runner,
        )
        .unwrap_err();

        let ArchivedVideoMergeError::Run(error) = error else {
            panic!("expected ffmpeg run failure");
        };
        assert_eq!(error.command.program, ffmpeg_path);
        assert_eq!(error.message, "planned ffmpeg failure");
        assert_eq!(runner.commands.len(), 1);
    }

    #[test]
    fn rejects_archived_video_export_without_detected_ffmpeg_before_staging() {
        let archive = zip_with_files([("video/segments/segment-1.mp4", b"one".as_slice())]);
        let status = crate::export::ffmpeg::FfmpegToolStatus {
            source: crate::export::ffmpeg::FfmpegToolSource::Missing,
            executable_path: None,
            detail: "ffmpeg not found".to_owned(),
        };
        let mut writer = FakeStageWriter::default();
        let mut runner = FakeFfmpegCommandRunner::default();

        let error = export_archived_video_segments_with_ffmpeg_status(
            &archive,
            Path::new("/tmp/rizum-no-ffmpeg"),
            &status,
            Path::new("/exports/artwork.mp4"),
            &mut writer,
            &mut runner,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ArchivedVideoExportError::MissingFfmpegTool { .. }
        ));
        assert!(writer.created_dirs.is_empty());
        assert!(writer.writes.is_empty());
        assert!(runner.commands.is_empty());
    }

    #[test]
    fn exports_archived_video_with_detected_ffmpeg_executable_path() {
        let archive = zip_with_files([("video/segments/segment-1.mp4", b"one".as_slice())]);
        let ffmpeg_path = PathBuf::from("/app/tools/ffmpeg");
        let status = crate::export::ffmpeg::FfmpegToolStatus {
            source: crate::export::ffmpeg::FfmpegToolSource::Bundled,
            executable_path: Some(ffmpeg_path.clone()),
            detail: ffmpeg_path.to_string_lossy().into_owned(),
        };
        let mut writer = FakeStageWriter::default();
        let mut runner = FakeFfmpegCommandRunner::default();

        let result = export_archived_video_segments_with_ffmpeg_status(
            &archive,
            Path::new("/tmp/rizum-detected-ffmpeg"),
            &status,
            Path::new("/exports/artwork.mp4"),
            &mut writer,
            &mut runner,
        )
        .unwrap();

        assert_eq!(runner.commands.len(), 1);
        assert_eq!(runner.commands[0].program, ffmpeg_path);
        assert_eq!(result.command, runner.commands[0]);
        assert_eq!(
            writer.created_dirs,
            vec![PathBuf::from("/tmp/rizum-detected-ffmpeg")]
        );
    }

    #[test]
    fn exports_thirty_second_archived_video_with_detected_ffmpeg_status() {
        let archive = zip_with_files([("video/segments/segment-1.mp4", b"one".as_slice())]);
        let ffmpeg_path = PathBuf::from("/app/tools/ffmpeg");
        let status = crate::export::ffmpeg::FfmpegToolStatus {
            source: crate::export::ffmpeg::FfmpegToolSource::System,
            executable_path: Some(ffmpeg_path),
            detail: "/app/tools/ffmpeg".to_owned(),
        };
        let mut writer = FakeStageWriter::default();
        let mut runner = FakeFfmpegCommandRunner::default();

        export_archived_video_segments_with_ffmpeg_status_and_mode(
            &archive,
            Path::new("/tmp/rizum-detected-preview"),
            &status,
            Path::new("/exports/artwork-preview.mp4"),
            ArchivedVideoExportMode::Preview30Seconds,
            &mut writer,
            &mut runner,
        )
        .unwrap();

        assert!(
            runner.commands[0]
                .args
                .windows(2)
                .any(|pair| pair == ["-t", "30"])
        );
    }

    #[derive(Default)]
    struct FakeStageWriter {
        created_dirs: Vec<PathBuf>,
        writes: Vec<(PathBuf, Vec<u8>)>,
    }

    impl ArchivedVideoStageWriter for FakeStageWriter {
        fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
            self.created_dirs.push(path.to_owned());
            Ok(())
        }

        fn write_file(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
            self.writes.push((path.to_owned(), bytes.to_vec()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeFfmpegCommandRunner {
        commands: Vec<FfmpegCommand>,
        fail_with_message: Option<String>,
    }

    impl crate::export::ffmpeg::FfmpegCommandRunner for FakeFfmpegCommandRunner {
        fn run(
            &mut self,
            command: &FfmpegCommand,
        ) -> Result<(), crate::export::ffmpeg::FfmpegCommandRunError> {
            self.commands.push(command.clone());
            if let Some(message) = &self.fail_with_message {
                return Err(crate::export::ffmpeg::FfmpegCommandRunError {
                    command: command.clone(),
                    message: message.clone(),
                });
            }

            Ok(())
        }
    }

    fn zip_with_files<const N: usize>(files: [(&str, &[u8]); N]) -> Vec<u8> {
        use std::io::{Cursor, Write};

        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (path, bytes) in files {
            archive.start_file(path, options).unwrap();
            archive.write_all(bytes).unwrap();
        }

        archive.finish().unwrap().into_inner()
    }
}
