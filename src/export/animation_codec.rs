use super::{
    animation::AnimationExportProgress,
    ffmpeg::{FfmpegCommand, FfmpegCommandRunner},
};
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationExportFormat {
    #[default]
    PngSequence,
    Gif,
    Apng,
    Mp4,
    Hevc,
}

impl AnimationExportFormat {
    pub const ALL: [Self; 5] = [
        Self::PngSequence,
        Self::Gif,
        Self::Apng,
        Self::Mp4,
        Self::Hevc,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::PngSequence => "PNG sequence",
            Self::Gif => "GIF",
            Self::Apng => "Animated PNG",
            Self::Mp4 => "MP4 (H.264)",
            Self::Hevc => "MP4 (HEVC)",
        }
    }
    pub const fn extension(self) -> &'static str {
        match self {
            Self::PngSequence => "",
            Self::Gif => "gif",
            Self::Apng => "png",
            Self::Mp4 | Self::Hevc => "mp4",
        }
    }
    pub const fn supports_alpha(self) -> bool {
        !matches!(self, Self::Mp4 | Self::Hevc)
    }
}

pub struct AnimationEncodeRequest {
    pub ffmpeg: PathBuf,
    pub sequence: PathBuf,
    pub output: PathBuf,
    pub format: AnimationExportFormat,
    pub frame_rate: u32,
    pub total_slots: u64,
    pub one_shot: bool,
}

impl AnimationEncodeRequest {
    /// The output is a caller-owned temporary file, published only after success.
    pub fn command(&self, encoder: &str) -> io::Result<FfmpegCommand> {
        if !(1..=60).contains(&self.frame_rate)
            || self.total_slots == 0
            || self.total_slots > 100_000
            || self.format == AnimationExportFormat::PngSequence
        {
            return Err(io::Error::other("Invalid animation encoding plan"));
        }
        let mut args: Vec<String> = [
            "-hide_banner",
            "-v",
            "error",
            "-nostdin",
            "-y",
            "-framerate",
        ]
        .map(str::to_owned)
        .into();
        args.extend([
            self.frame_rate.to_string(),
            "-start_number".into(),
            "1".into(),
            "-i".into(),
            self.sequence
                .join("frame-%06d.png")
                .to_string_lossy()
                .into_owned(),
            "-frames:v".into(),
            self.total_slots.to_string(),
            "-an".into(),
        ]);
        match self.format {
            AnimationExportFormat::Gif => {
                // Per-frame palettes keep the filter graph bounded instead of buffering
                // the complete sequence while a global palette is being generated.
                args.extend(
                    [
                        "-vf",
                        "split[a][b];[a]palettegen=stats_mode=single[p];[b][p]paletteuse=new=1",
                        "-loop",
                        if self.one_shot { "-1" } else { "0" },
                        "-f",
                        "gif",
                    ]
                    .map(str::to_owned),
                );
                let last_delay = ((self.total_slots * 100 + u64::from(self.frame_rate) / 2)
                    / u64::from(self.frame_rate))
                    - (((self.total_slots - 1) * 100 + u64::from(self.frame_rate) / 2)
                        / u64::from(self.frame_rate));
                args.extend(["-final_delay".into(), last_delay.to_string()]);
            }
            AnimationExportFormat::Apng => {
                args.extend(
                    [
                        "-c:v",
                        "apng",
                        "-plays",
                        if self.one_shot { "1" } else { "0" },
                        "-f",
                        "apng",
                    ]
                    .map(str::to_owned),
                );
                args.extend(["-final_delay".into(), format!("1/{}", self.frame_rate)]);
            }
            AnimationExportFormat::Mp4 | AnimationExportFormat::Hevc => {
                args.extend(
                    [
                        "-vf",
                        "pad=ceil(iw/2)*2:ceil(ih/2)*2,format=yuv420p",
                        "-c:v",
                        encoder,
                        "-b:v",
                        "12M",
                        "-movflags",
                        "+faststart",
                        "-f",
                        "mp4",
                    ]
                    .map(str::to_owned),
                );
                if self.format == AnimationExportFormat::Hevc {
                    args.extend(["-tag:v".into(), "hvc1".into()]);
                }
            }
            AnimationExportFormat::PngSequence => {
                unreachable!("Sequence does not require an encoder")
            }
        }
        args.push(self.output.to_string_lossy().into_owned());
        Ok(FfmpegCommand {
            program: self.ffmpeg.clone(),
            args,
        })
    }
}

pub fn select_video_encoder(
    ffmpeg: &Path,
    format: AnimationExportFormat,
    runner: &mut impl FfmpegCommandRunner,
    progress: &AnimationExportProgress,
) -> io::Result<String> {
    let candidates: &[&str] = match format {
        AnimationExportFormat::Mp4 => {
            if cfg!(target_os = "windows") {
                &[
                    "h264_mf",
                    "h264_nvenc",
                    "h264_qsv",
                    "h264_amf",
                    "libopenh264",
                ]
            } else if cfg!(target_os = "macos") {
                &["h264_videotoolbox", "libopenh264"]
            } else {
                &["libopenh264", "h264_nvenc", "h264_qsv"]
            }
        }
        AnimationExportFormat::Hevc => {
            if cfg!(target_os = "windows") {
                &[
                    "hevc_mf",
                    "hevc_nvenc",
                    "hevc_qsv",
                    "hevc_amf",
                    "libkvazaar",
                ]
            } else if cfg!(target_os = "macos") {
                &["hevc_videotoolbox", "libkvazaar"]
            } else {
                &["libkvazaar", "hevc_nvenc", "hevc_qsv"]
            }
        }
        _ => return Ok(String::new()),
    };
    let mut errors = Vec::new();
    // Native encoders depend on installed drivers/codecs. Probe real input before
    // rendering the document; an advertised encoder alone is not a capability.
    for encoder in candidates {
        progress.check_cancelled()?;
        let command = FfmpegCommand {
            program: ffmpeg.to_owned(),
            args: [
                "-hide_banner",
                "-v",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "color=size=256x256:rate=24",
                "-frames:v",
                "2",
                "-c:v",
                encoder,
                "-b:v",
                "12M",
                "-f",
                "null",
                "-",
            ]
            .map(str::to_owned)
            .into(),
        };
        match runner.run(&command) {
            Ok(()) => return Ok((*encoder).into()),
            Err(error) => errors.push(format!("{encoder}: {}", error.message)),
        }
    }
    Err(io::Error::other(format!(
        "No working {} encoder. {}",
        format.label(),
        errors.join("\n")
    )))
}
