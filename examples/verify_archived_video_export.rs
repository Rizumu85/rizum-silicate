use silicate::export::{
    archived_video::{
        ArchivedVideoExportMode, ArchivedVideoStageDirectory, FsArchivedVideoStageWriter,
        archived_video_segment_count, export_archived_video_segments_with_ffmpeg_status_and_mode,
    },
    ffmpeg::{ProcessFfmpegCommandRunner, detect_current_ffmpeg_tool_status},
};
use std::{env, io, path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let source_path = args
        .next()
        .ok_or("usage: verify_archived_video_export <document.procreate> <output.mp4>")?;
    let output_path = args
        .next()
        .ok_or("usage: verify_archived_video_export <document.procreate> <output.mp4>")?;
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let archive_bytes = silica::limits::read_procreate_archive(&source_path)?;
    let segment_count = archived_video_segment_count(&archive_bytes)?;
    let ffmpeg_status = detect_current_ffmpeg_tool_status()?;
    let stage_path;
    let started = Instant::now();

    {
        let stage = ArchivedVideoStageDirectory::create(&env::temp_dir(), &output_path)?;
        stage_path = stage.path().to_owned();
        let mut writer = FsArchivedVideoStageWriter;
        let mut runner = ProcessFfmpegCommandRunner;
        export_archived_video_segments_with_ffmpeg_status_and_mode(
            &archive_bytes,
            stage.path(),
            &ffmpeg_status,
            &output_path,
            ArchivedVideoExportMode::Preview30Seconds,
            &mut writer,
            &mut runner,
        )
        .map_err(|error| io::Error::other(format!("{error:?}")))?;
    }

    if stage_path.exists() {
        return Err(format!(
            "staging directory was not cleaned: {}",
            stage_path.display()
        )
        .into());
    }
    let output_bytes = output_path.metadata()?.len();
    if output_bytes == 0 {
        return Err("FFmpeg produced an empty output".into());
    }

    println!("verification=archived_video_export_v1");
    println!("fixture={}", source_path.display());
    println!("segments={segment_count}");
    println!("output={}", output_path.display());
    println!("output_bytes={output_bytes}");
    println!("stage_cleanup=verified");
    println!("elapsed_ms={:.3}", started.elapsed().as_secs_f64() * 1000.0);
    Ok(())
}
