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
