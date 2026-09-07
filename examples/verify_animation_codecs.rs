use std::{env, path::PathBuf, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let fixture = args
        .next()
        .ok_or("usage: verify_animation_codecs <animation.procreate> <new-output-folder>")?;
    let output = args.next().ok_or("missing output folder")?;
    std::fs::create_dir(&output)?;
    let (fps, slots) = silicate::diagnostics::verify_animation_codecs(&fixture, &output)?;
    for filename in ["animation.gif", "animation.png", "h264.mp4", "hevc.mp4"] {
        let path = output.join(filename);
        let probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "packet=duration_time",
                "-of",
                "csv=p=0",
            ])
            .arg(&path)
            .output()?;
        if !probe.status.success() {
            return Err(String::from_utf8_lossy(&probe.stderr).into_owned().into());
        }
        let durations: Vec<f64> = String::from_utf8(probe.stdout)?
            .lines()
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        let duration: f64 = durations.iter().sum();
        let expected = slots as f64 / f64::from(fps);
        if (duration - expected).abs() > 0.011 {
            return Err(format!("{filename}: expected {expected}s, got {duration}s").into());
        }
        if !filename.ends_with("gif") && durations.len() as u64 != slots {
            return Err(format!(
                "{filename}: expected {slots} frames, got {}",
                durations.len()
            )
            .into());
        }
        let decoded = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(["-f", "null", "-"])
            .status()?;
        if !decoded.success() {
            return Err(format!("{filename}: decode failed").into());
        }
        println!(
            "file={filename} frames={} duration={duration:.6}s decode=true",
            durations.len()
        );
    }
    println!("verification=animation_codecs_v1");
    let decoded = output.join("decoded");
    std::fs::create_dir(&decoded)?;
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(output.join("animation.png"))
        .args(["-fps_mode", "passthrough"])
        .arg(decoded.join("frame-%06d.png"))
        .status()?;
    if !status.success() {
        return Err("APNG frame decode failed".into());
    }
    for frame in 1..=slots {
        let filename = format!("frame-{frame:06}.png");
        let expected = image::open(output.join("reference").join(&filename))?.into_rgba8();
        let actual = image::open(decoded.join(&filename))?.into_rgba8();
        if actual != expected {
            return Err(format!("APNG pixels differ at frame {frame}").into());
        }
    }
    use silicate::export::ffmpeg::{
        CancellableFfmpegCommandRunner, FfmpegCommand, FfmpegCommandRunner,
        detect_current_ffmpeg_tool_status,
    };
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let signal = cancelled.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        signal.store(true, Ordering::Relaxed);
    });
    let mut runner = CancellableFfmpegCommandRunner {
        cancelled: &cancelled,
        timeout: Duration::from_secs(5),
    };
    let command = FfmpegCommand {
        program: detect_current_ffmpeg_tool_status()?
            .executable_path
            .ok_or("ffmpeg missing")?,
        args: [
            "-v",
            "error",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "color=size=256x256:rate=24",
            "-t",
            "60",
            "-f",
            "null",
            "-",
        ]
        .map(str::to_owned)
        .into(),
    };
    let started = Instant::now();
    let result = runner.run(&command);
    canceller.join().unwrap();
    if result.is_ok() || started.elapsed() > Duration::from_secs(3) {
        return Err("Encoder cancellation failed".into());
    }
    println!("apng_pixel_roundtrip=true encoder_cancellation=true");
    Ok(())
}
