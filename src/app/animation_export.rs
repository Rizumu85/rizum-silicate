use super::{App, compositor::CompositorHandle};
use crate::export::{
    animation::{AnimationExportPlan, AnimationExportProgress, PngSequenceWriter},
    still::StillExportBackground,
};
use eframe::wgpu;
use silicate_compositor::{buffer::BufferDimensions, tex::TextureExt};
use silicate_runtime::DocumentSnapshot;
use std::{
    io,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};

pub struct AnimationExportJob {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub compositor: CompositorHandle,
    pub snapshot: DocumentSnapshot,
    pub orientation: silica::Orientation,
    pub background: StillExportBackground,
    pub repeat_holds: bool,
    pub progress: Arc<AnimationExportProgress>,
}

impl AnimationExportJob {
    pub async fn export_encoded(
        mut self,
        path: PathBuf,
        format: crate::export::animation_codec::AnimationExportFormat,
    ) -> io::Result<PathBuf> {
        use crate::export::{
            animation_codec::{AnimationEncodeRequest, select_video_encoder},
            ffmpeg::{
                CancellableFfmpegCommandRunner, FfmpegCommandRunner,
                detect_current_ffmpeg_tool_status,
            },
        };
        let plan = AnimationExportPlan::new(&self.snapshot)?;
        if !format.supports_alpha() && self.background.is_transparent() {
            return Err(io::Error::other("Video requires the document background"));
        }
        if !format.supports_alpha() {
            self.snapshot.background_color[3] = 1.0;
        }
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Choose a new filename; animation export preserves existing files",
            ));
        }
        let ffmpeg = detect_current_ffmpeg_tool_status()?
            .executable_path
            .ok_or_else(|| {
                io::Error::other(
                    "ffmpeg is not installed; choose PNG sequence or install the export tools",
                )
            })?;
        let probe_path = ffmpeg.clone();
        let progress = self.progress.clone();
        progress.encoding.store(true, Ordering::Relaxed);
        let encoder = tokio::task::spawn_blocking(move || {
            let mut runner = CancellableFfmpegCommandRunner {
                cancelled: &progress.cancelled,
                timeout: std::time::Duration::from_secs(15),
            };
            select_video_encoder(&probe_path, format, &mut runner, &progress)
        })
        .await
        .map_err(io::Error::other)??;
        self.progress.encoding.store(false, Ordering::Relaxed);
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        let staging = tempfile::Builder::new()
            .prefix(".rizum-animation-")
            .tempdir_in(parent)?;
        let encoded = tempfile::NamedTempFile::new_in(staging.path())?;
        let request = AnimationEncodeRequest {
            ffmpeg,
            sequence: staging.path().join("frames"),
            output: encoded.path().to_owned(),
            format,
            frame_rate: plan.frame_rate,
            total_slots: plan.total_slots,
            one_shot: self.snapshot.animation_playback.unwrap().mode
                == silicate_runtime::AnimationPlaybackMode::OneShot,
        };
        let progress = self.progress.clone();
        self.repeat_holds = true;
        self.export_sequence(request.sequence.clone()).await?;
        progress.encoding.store(true, Ordering::Relaxed);
        tokio::task::spawn_blocking(move || {
            let _staging = staging;
            let command = request.command(&encoder)?;
            let mut runner = CancellableFfmpegCommandRunner {
                cancelled: &progress.cancelled,
                timeout: std::time::Duration::from_secs(1800),
            };
            let result = runner.run(&command);
            progress.check_cancelled()?;
            result.map_err(|e| io::Error::other(e.message))?;
            encoded.persist_noclobber(&path).map_err(io::Error::other)?;
            Ok(path)
        })
        .await
        .map_err(io::Error::other)?
    }

    pub async fn export_sequence(mut self, path: PathBuf) -> io::Result<PathBuf> {
        let plan = AnimationExportPlan::new(&self.snapshot)?;
        let size = self.snapshot.canvas_size;
        let files = plan.validate_sequence_size(size.width, size.height, self.repeat_holds)?;
        self.progress.total.store(files, Ordering::Relaxed);
        self.progress.check_cancelled()?;
        let mut writer = PngSequenceWriter::create(&path, plan.frame_rate)?;
        // Freeze edits and remove preview-only onion skins without seeking the live
        // runtime or replacing its presentation texture.
        let animation = self.snapshot.animation.as_mut().unwrap();
        animation.onion_skin_count = 0;
        animation.blend_primary_frame = false;
        let texture = wgpu::Texture::empty(
            &self.device,
            size.width,
            size.height,
            wgpu::Texture::OUTPUT_USAGE,
        );
        for frame in plan.frames {
            self.progress.check_cancelled()?;
            let playback = self.snapshot.animation_playback.as_mut().unwrap();
            playback.active = true;
            playback.playing = false;
            playback.source_layer_id = Some(frame.source);
            self.compositor
                .render_still(&self.snapshot, self.background, &texture)
                .await
                .map_err(io::Error::other)?;
            let image = App::export(
                &texture,
                &self.device,
                &self.queue,
                BufferDimensions::from_extent(texture.size()),
                self.orientation,
            )
            .await
            .map_err(io::Error::other)?;
            let progress = self.progress.clone();
            let repeat = self.repeat_holds;
            writer = tokio::task::spawn_blocking(move || {
                writer.write_frame(&image, frame.slots, repeat, &progress)?;
                Ok::<_, io::Error>(writer)
            })
            .await
            .map_err(io::Error::other)??;
        }
        self.progress.check_cancelled()?;
        writer.finish()
    }
}
