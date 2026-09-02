use silica::ProcreateFile;
use std::{env, error::Error, fs, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: verify_animation_metadata <document.procreate>")?;
    let bytes = fs::read(&path)?;
    let document = ProcreateFile::open(&bytes)?;
    let animation = document
        .animation
        .as_ref()
        .ok_or("document does not contain Animation Assist settings")?;

    if !(1..=60).contains(&animation.frame_rate) {
        return Err(format!("frame rate {} is outside 1..=60", animation.frame_rate).into());
    }
    if animation.onion_skin_count > 12 {
        return Err(format!("onion skin count {} exceeds 12", animation.onion_skin_count).into());
    }
    if !(0.0..=1.0).contains(&animation.onion_skin_opacity) {
        return Err(format!(
            "onion skin opacity {} is outside 0..=1",
            animation.onion_skin_opacity
        )
        .into());
    }

    let frames = document.animation_frame_sources().collect::<Vec<_>>();
    let max_hold_duration = frames
        .iter()
        .map(|frame| frame.hold_duration)
        .max()
        .unwrap_or_default();
    if max_hold_duration > 120 {
        return Err(format!("hold duration {max_hold_duration} exceeds 120").into());
    }
    let timeline_slots = frames
        .iter()
        .map(|frame| 1_u64 + u64::from(frame.hold_duration))
        .sum::<u64>();

    println!("animation_metadata=procreate_v1");
    println!("file={}", path.display());
    println!(
        "assist_mode={}",
        animation
            .assist_mode
            .map(|mode| mode.raw().to_string())
            .unwrap_or_else(|| "missing".to_owned())
    );
    println!("frame_rate={}", animation.frame_rate);
    println!("playback_mode={}", animation.playback_mode.raw());
    println!("playback_direction={}", animation.playback_direction.raw());
    println!("onion_skin_count={}", animation.onion_skin_count);
    println!("onion_skin_opacity={}", animation.onion_skin_opacity);
    println!("blend_primary_frame={}", animation.blend_primary_frame);
    println!(
        "first_item_is_foreground={}",
        animation.first_item_is_foreground
    );
    println!(
        "last_item_is_background={}",
        animation.last_item_is_background
    );
    println!("frame_sources={}", frames.len());
    println!("timeline_slots={timeline_slots}");
    println!("max_hold_duration={max_hold_duration}");
    Ok(())
}
