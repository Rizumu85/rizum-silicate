use crate::export::ffmpeg::FfmpegCommand;
use silica::{error::SilicaError, video::extract_archived_video_segments};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedVideoStaging {
    pub segment_paths: Vec<PathBuf>,
    pub concat_list_path: PathBuf,
    pub concat_list: String,
}

#[derive(Debug)]
pub enum ArchivedVideoStageError {
    Archive(SilicaError),
    Io(io::Error),
    NoSegments,
}

pub trait ArchivedVideoStageWriter {
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()>;
    fn write_file(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;
}

pub fn build_archived_video_merge_plan(
    ffmpeg_path: &Path,
    concat_list_path: &Path,
    ordered_segments: &[PathBuf],
    output_path: &Path,
) -> Result<ArchivedVideoMergePlan, ArchivedVideoMergePlanError> {
    if ordered_segments.is_empty() {
        return Err(ArchivedVideoMergePlanError::NoSegments);
    }

    let concat_list = build_concat_list(ordered_segments);

    Ok(ArchivedVideoMergePlan {
        concat_list_path: concat_list_path.to_owned(),
        concat_list,
        command: FfmpegCommand {
            program: ffmpeg_path.to_owned(),
            args: vec![
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
                output_path.to_string_lossy().into_owned(),
            ],
        },
    })
}

pub fn stage_archived_video_segments(
    archive_bytes: &[u8],
    stage_dir: &Path,
    writer: &mut impl ArchivedVideoStageWriter,
) -> Result<ArchivedVideoStaging, ArchivedVideoStageError> {
    let segments = extract_archived_video_segments(archive_bytes)?;
    if segments.is_empty() {
        return Err(ArchivedVideoStageError::NoSegments);
    }

    writer.create_dir_all(stage_dir)?;

    let mut segment_paths = Vec::with_capacity(segments.len());
    for segment in segments {
        let segment_path = stage_dir.join(archived_segment_file_name(&segment.segment.path));
        writer.write_file(&segment_path, &segment.bytes)?;
        segment_paths.push(segment_path);
    }

    let concat_list_path = stage_dir.join("segments.ffconcat");
    let concat_list = build_concat_list(&segment_paths);
    writer.write_file(&concat_list_path, concat_list.as_bytes())?;

    Ok(ArchivedVideoStaging {
        segment_paths,
        concat_list_path,
        concat_list,
    })
}

pub struct FsArchivedVideoStageWriter;

impl ArchivedVideoStageWriter for FsArchivedVideoStageWriter {
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn write_file(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        fs::write(path, bytes)
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

fn build_concat_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| format!("file '{}'\n", escape_concat_file_path(path)))
        .collect()
}

fn archived_segment_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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
