use eframe::wgpu;

use egui::load::SizedTexture;
use silica_gpu::ProcreateFile;
use silicate_compositor::{Compositor, pipeline::Pipeline, tex::TextureExt};
use silicate_runtime::{AnimationPlaybackSnapshot, DocumentSnapshot, RuntimeEvent, RuntimeUpdate};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{collections::HashMap, sync::Arc};

use crate::app::compositor::{CompositorApp, CompositorProjectionError};

use super::compositor::CompositorHandle;

#[derive(Hash, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct InstanceKey(usize);

impl InstanceKey {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for InstanceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[I:{}]", self.0)
    }
}

pub struct Instance {
    pub id: InstanceKey,
    pub snapshot: DocumentSnapshot,
    pub file: Arc<ProcreateFile>,
    pub archived_video_segment_count: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pub source_path: Option<PathBuf>,
    pub output_texture: wgpu::Texture,
    pub rotation: f32,
    pub preview_textures: Option<wgpu::Texture>,
    pub compositor: CompositorHandle,

    pub previews: HashMap<u32, SizedTexture>,
    pub canvas: Option<SizedTexture>,
    pub render_dirty: bool,
    pub(super) last_animation_tick: Option<Instant>,
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance")
            .field("snapshot", &self.snapshot)
            .field("file", &self.file)
            .field("output_texture", &self.output_texture)
            .field("rotation", &self.rotation)
            .field("preview_textures", &self.preview_textures)
            .field("compositor", &"..")
            .finish()
    }
}

impl Instance {
    pub fn has_archived_video_segments(&self) -> bool {
        self.archived_video_segment_count > 0
    }

    pub fn is_upright(&self) -> bool {
        !(45.0..135.0).contains(&self.rotation.to_degrees())
            && !(225.0..315.0).contains(&self.rotation.to_degrees())
    }

    pub fn submit_to_compositor(&mut self) -> Result<(), CompositorProjectionError> {
        if self.render_dirty {
            self.compositor.submit(&self.snapshot)?;
            self.render_dirty = false;
        }
        Ok(())
    }

    pub fn apply_runtime_update(&mut self, update: RuntimeUpdate<DocumentSnapshot>) {
        self.render_dirty |= !update.events.is_empty();
        self.snapshot = update.value;
    }

    pub fn apply_animation_playback_update(
        &mut self,
        update: RuntimeUpdate<AnimationPlaybackSnapshot>,
    ) {
        let was_playing = self
            .snapshot
            .animation_playback
            .is_some_and(|playback| playback.playing);
        if let Some(revision) = update
            .events
            .iter()
            .filter_map(|event| match event {
                RuntimeEvent::AnimationPlaybackChanged { revision, .. } => Some(*revision),
                _ => None,
            })
            .max()
        {
            self.snapshot.revision = revision;
            self.render_dirty = true;
        }
        self.snapshot.animation_playback = Some(update.value);
        if !update.value.playing || !was_playing {
            self.last_animation_tick = None;
        }
    }

    pub fn animation_tick_elapsed(&mut self, now: Instant) -> Option<Duration> {
        if !self
            .snapshot
            .animation_playback
            .is_some_and(|playback| playback.playing)
        {
            self.last_animation_tick = None;
            return None;
        }

        self.last_animation_tick
            .replace(now)
            .map(|previous| now.saturating_duration_since(previous))
    }

    pub fn generate_previews(
        &mut self,
        mut target: Compositor,
        device: &wgpu::Device,
        pipeline: &Pipeline,
    ) {
        let file = &self.file;
        let aspect_ratio = file.size.width as f32 / file.size.height as f32;
        let scaled_height = (256.0 * aspect_ratio) as u32;

        let preview_textures = {
            let preview_textures = wgpu::Texture::empty_layers(
                device,
                256,
                scaled_height,
                file.layer_count(true) + 1,
                wgpu::Texture::OUTPUT_USAGE,
            );

            CompositorApp::generate_layers_preview(
                pipeline,
                &mut target,
                &preview_textures,
                &file.layers,
            );

            preview_textures
        };

        self.preview_textures = Some(preview_textures);
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        log::info!(
            "{} Closing instance for Procreate document \"{}\"",
            self.id,
            self.snapshot.title.as_deref().unwrap_or("Untitled Artwork")
        );
    }
}
