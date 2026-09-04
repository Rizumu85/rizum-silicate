mod blend;
pub mod compositor;
pub mod instance;

#[cfg(not(target_arch = "wasm32"))]
use crate::export::archived_video::ArchivedVideoExportMode;
use crate::export::archived_video::archived_video_segment_count;
use compositor::{CompositorApp, CompositorProjectionError};
use eframe::egui_wgpu::wgpu;
use egui_dock::NodePath;
use instance::{Instance, InstanceKey};
use silica_gpu::{ProcreateFile, ProcreateFileAtlas, error::SilicaError};
use silicate_compositor::{
    Compositor,
    buffer::BufferDimensions,
    canvas::{CompositorAtlasTiling, CompositorCanvasTiling},
    pipeline::Pipeline,
    tex::TextureExt,
};
use silicate_runtime::{
    AnimationOnionSkinSettings, AnimationPlaybackDirection, AnimationPlaybackMode,
    AnimationPlaybackSnapshot, CanvasFlipped, DocumentCommand, DocumentId, DocumentRuntime,
    DocumentSnapshot, HistoryGroupId, LayerId, RuntimeError, RuntimeUpdate,
};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicUsize, mpsc::Sender},
    time::Duration,
};
use thiserror::Error;

pub enum AppEvent {
    NewInstance(InstanceKey, Instance, CompositorApp),
    NewView(NodePath, InstanceKey),
    RebindTexture(InstanceKey),
    RebindPreviews(InstanceKey),
    RemoveInstance {
        key: InstanceKey,
        discard_changes: bool,
    },
    LoadFile {
        #[cfg(not(target_arch = "wasm32"))]
        path: PathBuf,
        #[cfg(target_arch = "wasm32")]
        bytes: Arc<[u8]>,
        node_path: Option<NodePath>,
    },
    #[cfg(not(target_arch = "wasm32"))]
    FileLoadCompleted {
        result: Result<InstanceKey, String>,
        node_path: Option<NodePath>,
    },
    LoadDialog(NodePath),
    SaveDialog {
        texture: wgpu::Texture,
        orientation: silica_gpu::Orientation,
    },
    #[cfg(not(target_arch = "wasm32"))]
    ExportArchivedVideoDialog {
        source_path: PathBuf,
        export_mode: ArchivedVideoExportMode,
    },
    Toast(egui_notify::Toast),
    SetTheme(egui::ThemePreference),
    #[cfg(target_arch = "wasm32")]
    LoadDemoFile,
}

#[derive(Debug, Error)]
pub enum AppLoadError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] silica::error::SilicaError),
    #[error(transparent)]
    Gpu(#[from] SilicaError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    CompositorProjection(#[from] CompositorProjectionError),
    #[error("runtime and GPU hierarchy identities diverged during document open")]
    HierarchyIdentityMismatch,
}

impl std::fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEvent::NewInstance(arg0, arg1, _) => f
                .debug_tuple("NewInstance")
                .field(arg0)
                .field(arg1)
                .finish(),
            AppEvent::NewView(node_path, instance_key) => f
                .debug_tuple("NewView")
                .field(node_path)
                .field(instance_key)
                .finish(),
            AppEvent::RebindTexture(arg0) => f.debug_tuple("RebindTexture").field(arg0).finish(),
            AppEvent::RebindPreviews(arg0) => f.debug_tuple("RebindPreviews").field(arg0).finish(),
            AppEvent::RemoveInstance {
                key,
                discard_changes,
            } => f
                .debug_struct("RemoveInstance")
                .field("key", key)
                .field("discard_changes", discard_changes)
                .finish(),
            AppEvent::Toast(_) => f.debug_tuple("Toast").field(&"...").finish(),
            AppEvent::LoadFile { .. } => f.debug_tuple("LoadFilePath").field(&"...").finish(),
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::FileLoadCompleted { .. } => {
                f.debug_tuple("FileLoadCompleted").field(&"...").finish()
            }
            AppEvent::LoadDialog(_) => f.debug_tuple("LoadDialog").field(&"...").finish(),
            AppEvent::SaveDialog { orientation, .. } => f
                .debug_struct("SaveDialog")
                .field("orientation", orientation)
                .finish(),
            #[cfg(not(target_arch = "wasm32"))]
            AppEvent::ExportArchivedVideoDialog { .. } => f
                .debug_tuple("ExportArchivedVideoDialog")
                .field(&"...")
                .finish(),
            AppEvent::SetTheme(theme) => f.debug_tuple("SetTheme").field(theme).finish(),
            #[cfg(target_arch = "wasm32")]
            AppEvent::LoadDemoFile => f.debug_tuple("LoadDemoFile").finish(),
        }
    }
}

pub struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    event_sender: Sender<AppEvent>,
    pipeline: Pipeline,
    curr_id: AtomicUsize,
    runtime: Mutex<DocumentRuntime>,
}

impl App {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, event_sender: Sender<AppEvent>) -> Self {
        Self {
            pipeline: Pipeline::new(&device),
            device,
            queue,
            event_sender,
            curr_id: AtomicUsize::new(0),
            runtime: Mutex::new(DocumentRuntime::new()),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_file(&self, path: &Path) -> Result<InstanceKey, AppLoadError> {
        let bytes = silica::limits::read_procreate_archive(path)?;
        self.load_bytes_with_source(&bytes, Some(path.to_owned()))
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn load_bytes(&self, bytes: &[u8]) -> Result<InstanceKey, AppLoadError> {
        self.load_bytes_with_source(bytes, None)
    }

    fn load_bytes_with_source(
        &self,
        bytes: &[u8],
        source_path: Option<PathBuf>,
    ) -> Result<InstanceKey, AppLoadError> {
        let id = InstanceKey::new(
            self.curr_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        log::info!("{id} Loading file");

        let (file, metadata, snapshot, archived_video_segment_count) = {
            let document = silica::ProcreateFile::open(bytes)?;
            let archived_video_segment_count = archived_video_segment_count(bytes)?;
            let opened = self
                .runtime
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .open_document(&document)?;
            let snapshot = opened.value;
            let document_id = snapshot.document_id;

            match ProcreateFile::open_document(document, bytes, &self.device, &self.queue) {
                Ok((file, metadata)) => {
                    let runtime_ids = snapshot
                        .layers
                        .iter()
                        .map(|layer| layer.layer_id.hierarchy_id())
                        .collect::<Vec<_>>();
                    if file.hierarchy_ids() != runtime_ids {
                        let _ = self.close_document(document_id, true);
                        return Err(AppLoadError::HierarchyIdentityMismatch);
                    }
                    (file, metadata, snapshot, archived_video_segment_count)
                }
                Err(error) => {
                    let _ = self.close_document(document_id, true);
                    return Err(error.into());
                }
            }
        };

        log::info!(
            "{id} Loaded Procreate document \"{}\" with {} layers",
            file.name.as_deref().unwrap_or("Untitled Artwork"),
            file.layer_count(true)
        );

        let ProcreateFileAtlas {
            atlas_texture,
            canvas_tiling,
        } = metadata;

        let canvas = CompositorCanvasTiling::new(
            (file.size.width, file.size.height),
            (canvas_tiling.cols, canvas_tiling.rows),
            canvas_tiling.size,
        );
        let composite_target = Compositor::new(
            &self.device,
            &self.queue,
            canvas,
            CompositorAtlasTiling::new(canvas_tiling.atlas.cols, canvas_tiling.atlas.rows),
            atlas_texture.clone(),
        );

        let output_texture = wgpu::Texture::empty(
            &self.device,
            file.size.width,
            file.size.height,
            wgpu::Texture::OUTPUT_USAGE,
        );

        let rotation = match file.orientation {
            silica_gpu::Orientation::NoRotation => 0.0,
            silica_gpu::Orientation::Clockwise180 => 180.0,
            silica_gpu::Orientation::Clockwise270 => 270.0,
            silica_gpu::Orientation::Clockwise90 => 90.0,
            _ => 0f32,
        }
        .to_radians();

        let document_title = file
            .name
            .clone()
            .unwrap_or_else(|| "Untitled Artwork".to_owned());
        let file = Arc::new(file);
        let compositor_result = CompositorApp::new(
            id,
            self.pipeline.clone(),
            file.clone(),
            &snapshot,
            composite_target,
        );
        let (compositor, handle) = match compositor_result {
            Ok(result) => result,
            Err(error) => {
                let _ = self.close_document(snapshot.document_id, true);
                return Err(error.into());
            }
        };

        let mut instance = Instance {
            id,
            snapshot,
            file,
            archived_video_segment_count,
            #[cfg(not(target_arch = "wasm32"))]
            source_path,
            output_texture: output_texture.clone(),
            preview_textures: None,
            compositor: handle,
            rotation,
            previews: HashMap::new(),
            canvas: None,
            render_dirty: false,
            last_animation_tick: None,
        };

        log::debug!(
            "{id} Generating previews for Procreate document \"{}\"",
            document_title
        );

        instance.generate_previews(
            Compositor::new(
                &self.device,
                &self.queue,
                canvas,
                CompositorAtlasTiling::new(canvas_tiling.atlas.cols, canvas_tiling.atlas.rows),
                atlas_texture,
            ),
            &self.device,
            &self.pipeline,
        );

        log::info!(
            "{id} Instance created for Procreate document \"{}\"",
            document_title
        );

        self.event_sender
            .send(AppEvent::NewInstance(id, instance, compositor))
            .unwrap();
        Ok(id)
    }

    pub fn close_document(
        &self,
        document_id: DocumentId,
        discard_changes: bool,
    ) -> Result<(), RuntimeError> {
        self.runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .dispatch(DocumentCommand::CloseDocument {
                document_id,
                discard_changes,
            })?;
        Ok(())
    }

    pub fn undo(
        &self,
        document_id: DocumentId,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(document_id, DocumentCommand::Undo { document_id }, None)
    }

    pub fn redo(
        &self,
        document_id: DocumentId,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(document_id, DocumentCommand::Redo { document_id }, None)
    }

    pub fn set_animation_playback_active(
        &self,
        document_id: DocumentId,
        active: bool,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        self.dispatch_animation_playback(
            document_id,
            DocumentCommand::SetAnimationPlaybackActive {
                document_id,
                active,
            },
        )
    }

    pub fn set_animation_playing(
        &self,
        document_id: DocumentId,
        playing: bool,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        self.dispatch_animation_playback(
            document_id,
            DocumentCommand::SetAnimationPlaying {
                document_id,
                playing,
            },
        )
    }

    pub fn set_animation_playback_mode(
        &self,
        document_id: DocumentId,
        mode: AnimationPlaybackMode,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        self.dispatch_animation_playback(
            document_id,
            DocumentCommand::SetAnimationPlaybackMode { document_id, mode },
        )
    }

    pub fn set_animation_playback_direction(
        &self,
        document_id: DocumentId,
        direction: AnimationPlaybackDirection,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        self.dispatch_animation_playback(
            document_id,
            DocumentCommand::SetAnimationPlaybackDirection {
                document_id,
                direction,
            },
        )
    }

    pub fn seek_animation_timeline(
        &self,
        document_id: DocumentId,
        slot_index: u64,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        self.dispatch_animation_playback(
            document_id,
            DocumentCommand::SeekAnimationTimeline {
                document_id,
                slot_index,
            },
        )
    }

    pub fn advance_animation(
        &self,
        document_id: DocumentId,
        elapsed: Duration,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        self.runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .advance_animation(document_id, elapsed)
    }

    pub fn set_animation_onion_skin_settings(
        &self,
        document_id: DocumentId,
        settings: AnimationOnionSkinSettings,
        history_group: Option<HistoryGroupId>,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetAnimationOnionSkinSettings {
                document_id,
                settings,
            },
            history_group,
        )
    }

    pub fn set_layer_visibility(
        &self,
        document_id: DocumentId,
        layer_id: LayerId,
        visible: bool,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetLayerVisibility {
                document_id,
                layer_id,
                visible,
            },
            None,
        )
    }

    pub fn set_layer_clipped(
        &self,
        document_id: DocumentId,
        layer_id: LayerId,
        clipped: bool,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetLayerClipped {
                document_id,
                layer_id,
                clipped,
            },
            None,
        )
    }

    pub fn set_layer_blend_mode(
        &self,
        document_id: DocumentId,
        layer_id: LayerId,
        blend_mode: silica::BlendingMode,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetLayerBlendMode {
                document_id,
                layer_id,
                blend_mode,
            },
            None,
        )
    }

    pub fn set_layer_opacity(
        &self,
        document_id: DocumentId,
        layer_id: LayerId,
        opacity: f32,
        history_group: Option<HistoryGroupId>,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetLayerOpacity {
                document_id,
                layer_id,
                opacity,
            },
            history_group,
        )
    }

    pub fn set_background_color(
        &self,
        document_id: DocumentId,
        color: [f32; 4],
        history_group: Option<HistoryGroupId>,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetBackgroundColor { document_id, color },
            history_group,
        )
    }

    pub fn set_background_visibility(
        &self,
        document_id: DocumentId,
        visible: bool,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetBackgroundVisibility {
                document_id,
                visible,
            },
            None,
        )
    }

    pub fn set_canvas_flipped(
        &self,
        document_id: DocumentId,
        flipped: CanvasFlipped,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        self.dispatch_document_mutation(
            document_id,
            DocumentCommand::SetCanvasFlipped {
                document_id,
                flipped,
            },
            None,
        )
    }

    fn dispatch_document_mutation(
        &self,
        document_id: DocumentId,
        command: DocumentCommand,
        history_group: Option<HistoryGroupId>,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let update = match history_group {
            Some(group_id) => runtime.dispatch_grouped(command, group_id)?,
            None => runtime.dispatch(command)?,
        };
        let snapshot = runtime.snapshot(document_id)?;

        Ok(RuntimeUpdate {
            value: snapshot,
            events: update.events,
        })
    }

    fn dispatch_animation_playback(
        &self,
        document_id: DocumentId,
        command: DocumentCommand,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        let mut runtime = self
            .runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let update = runtime.dispatch(command)?;
        let playback = runtime.animation_playback(document_id)?;

        Ok(RuntimeUpdate {
            value: playback,
            events: update.events,
        })
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub async fn export(
        texture: &wgpu::Texture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dimensions: BufferDimensions,
        orientation: silica::Orientation,
    ) -> image::ImageResult<image::RgbaImage> {
        let output_buffer = texture.export_buffer(device, queue, dimensions);
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = tokio::sync::oneshot::channel();

        // Mapping must be registered before polling or native export can wait forever.
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|error| export_error(format!("GPU export polling failed: {error}")))?;
        receiver
            .await
            .map_err(|error| export_error(format!("GPU export callback was cancelled: {error}")))?
            .map_err(|error| export_error(format!("GPU export buffer mapping failed: {error}")))?;

        let data = buffer_slice.get_mapped_range().to_vec();
        output_buffer.unmap();
        let buffer = image::RgbaImage::from_raw(
            dimensions.padded_bytes_per_row() / 4,
            dimensions.height(),
            data,
        )
        .ok_or_else(|| export_error("GPU export buffer dimensions do not match mapped data"))?;
        let image =
            image::imageops::crop_imm(&buffer, 0, 0, dimensions.width(), dimensions.height())
                .to_image();

        Ok(crate::export::still::from_premultiplied_rgba(
            image,
            orientation,
        ))
    }

    pub fn rebind_texture(&self, id: InstanceKey) {
        self.event_sender.send(AppEvent::RebindTexture(id)).unwrap();
    }
}

fn export_error(message: impl Into<String>) -> image::ImageError {
    image::ImageError::IoError(std::io::Error::other(message.into()))
}
