use silica::{ProcreateFile, video::list_archived_video_segments};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: verify_archived_video_metadata <document.procreate>")?;
    let bytes = fs::read(path)?;
    let document = ProcreateFile::open(&bytes)?;
    let metadata = document
        .archived_video
        .as_ref()
        .ok_or("Document.archive does not contain archived-video metadata")?;
    let encoding = &metadata.encoding;

    if encoding.frame_size.width == 0 || encoding.frame_size.height == 0 {
        return Err("archived-video frame dimensions must be positive".into());
    }
    if encoding.frames_per_second == 0 || encoding.frames_per_second > 120 {
        return Err(format!(
            "archived-video frame rate {} is outside 1..=120",
            encoding.frames_per_second
        )
        .into());
    }
    if !encoding.bitrate.is_finite() || encoding.bitrate <= 0.0 {
        return Err(format!(
            "archived-video bitrate {} must be finite and positive",
            encoding.bitrate
        )
        .into());
    }
    if let Some(seconds) = document.tracked_time_seconds
        && (!seconds.is_finite() || seconds < 0.0)
    {
        return Err(format!("document tracked time {seconds} is invalid").into());
    }

    let segments = list_archived_video_segments(&bytes)?;
    let total_segment_bytes = segments.iter().try_fold(0_u64, |total, segment| {
        total
            .checked_add(segment.size)
            .ok_or("segment byte total overflow")
    })?;
    if segments
        .windows(2)
        .any(|pair| pair[0].index >= pair[1].index)
    {
        return Err("archived-video segment indices must be unique and increasing".into());
    }

    println!("archived_video_metadata=procreate_v1");
    println!("recording_enabled={}", metadata.recording_enabled);
    println!("purged={}", metadata.purged);
    println!("segment_ordinal={:?}", metadata.segment_ordinal);
    println!("tracked_time_seconds={:?}", document.tracked_time_seconds);
    println!(
        "frame_size={}x{}",
        encoding.frame_size.width, encoding.frame_size.height
    );
    println!("frames_per_second={}", encoding.frames_per_second);
    println!("bitrate={}", encoding.bitrate);
    println!("codec_raw={}", encoding.codec_raw);
    println!("codec_2020_raw={:?}", encoding.codec_2020_raw);
    println!("color_space_raw={}", encoding.color_space_raw);
    println!("quality_preference={}", encoding.quality_preference);
    println!("resolution_preference={}", encoding.resolution_preference);
    println!("source_orientation={:?}", encoding.source_orientation);
    println!("segment_count={}", segments.len());
    println!("segment_bytes={total_segment_bytes}");

    Ok(())
}
