use crate::export::ffmpeg::FfmpegCommand;
use std::path::{Path, PathBuf};

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

pub fn build_archived_video_merge_plan(
    ffmpeg_path: &Path,
    concat_list_path: &Path,
    ordered_segments: &[PathBuf],
    output_path: &Path,
) -> Result<ArchivedVideoMergePlan, ArchivedVideoMergePlanError> {
    if ordered_segments.is_empty() {
        return Err(ArchivedVideoMergePlanError::NoSegments);
    }

    let concat_list = ordered_segments
        .iter()
        .map(|path| format!("file '{}'\n", escape_concat_file_path(path)))
        .collect();

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
}
