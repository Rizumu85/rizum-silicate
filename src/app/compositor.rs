use eframe::wgpu;

use silica_gpu::{ProcreateFile, SilicaHierarchy, SilicaLayer};
use silicate_compositor::tex::TextureExt;
use silicate_compositor::{ChunkTile, CompositeLayer, Compositor, pipeline::Pipeline};
use silicate_runtime::{CanvasFlipped, DocumentSnapshot, LayerId, LayerSnapshot};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
use tokio::sync::watch::{Receiver, Sender};

use crate::app::instance::InstanceKey;

pub struct CompositorApp {
    target: Compositor,
    pipeline: Pipeline,
    rx: Receiver<Arc<CompositorRenderState>>,
    id: InstanceKey,
    chunk_source: Option<Arc<ProcreateFile>>,
    flat_chunks: Vec<ChunkTile>,
}

pub struct CompositorHandle {
    compositor_sender: Sender<Arc<CompositorRenderState>>,
    projection_plan: Arc<CompositorProjectionPlan>,
}

#[derive(Debug, Error)]
pub enum CompositorProjectionError {
    #[error("runtime layer {0:?} is missing from the render projection")]
    MissingLayer(LayerId),
    #[error("runtime layer {0:?} is missing editable layer properties")]
    MissingLayerProperties(LayerId),
}

struct CompositorRenderState {
    layers: Vec<CompositeLayer>,
    background: Option<[f32; 4]>,
    flipped: CanvasFlipped,
}

/// Compiled once because animation playback turns projection into a frame-cadence path.
/// Stable runtime identities let each update avoid rebuilding maps or walking topology.
struct CompositorProjectionPlan {
    layers: Vec<LayerProjection>,
}

struct LayerProjection {
    layer_id: LayerId,
    layer_index: usize,
    ancestors: Box<[(LayerId, usize)]>,
    mask: Option<(LayerId, usize)>,
    top_level_id: LayerId,
}

#[derive(Clone, Copy)]
struct AnimationLayerSelection {
    source: Option<LayerId>,
    foreground: Option<LayerId>,
    background: Option<LayerId>,
}

impl AnimationLayerSelection {
    fn from_snapshot(snapshot: &DocumentSnapshot) -> Option<Self> {
        snapshot
            .animation_playback
            .filter(|playback| playback.active)
            .map(|playback| Self {
                source: playback.source_layer_id,
                foreground: snapshot.animation_foreground_layer_id(),
                background: snapshot.animation_background_layer_id(),
            })
    }

    fn contains(self, layer_id: LayerId) -> bool {
        [self.source, self.foreground, self.background]
            .into_iter()
            .flatten()
            .any(|selected| selected == layer_id)
    }
}

impl CompositorHandle {
    pub fn submit(&self, snapshot: &DocumentSnapshot) -> Result<(), CompositorProjectionError> {
        let state = Arc::new(CompositorRenderState::project(
            &self.projection_plan,
            snapshot,
        )?);
        self.compositor_sender.send_replace(state);
        Ok(())
    }
}

impl CompositorProjectionPlan {
    fn new(
        file: &ProcreateFile,
        snapshot: &DocumentSnapshot,
    ) -> Result<Self, CompositorProjectionError> {
        let layer_indices = snapshot
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| (layer.layer_id, index))
            .collect::<HashMap<_, _>>();
        let mut layers = Vec::new();
        Self::append_layers(
            &file.layers,
            &layer_indices,
            &mut Vec::new(),
            None,
            &mut layers,
        )?;

        Ok(Self { layers })
    }

    fn append_layers(
        nodes: &[SilicaHierarchy],
        layer_indices: &HashMap<LayerId, usize>,
        ancestors: &mut Vec<(LayerId, usize)>,
        top_level_id: Option<LayerId>,
        projections: &mut Vec<LayerProjection>,
    ) -> Result<(), CompositorProjectionError> {
        for node in nodes.iter().rev() {
            let layer_id = match node {
                SilicaHierarchy::Group(group) => LayerId::from(group.hierarchy_id()),
                SilicaHierarchy::Layer(layer) => LayerId::from(layer.hierarchy_id()),
            };
            let layer_index = *layer_indices
                .get(&layer_id)
                .ok_or(CompositorProjectionError::MissingLayer(layer_id))?;
            let top_level_id = top_level_id.unwrap_or(layer_id);

            match node {
                SilicaHierarchy::Group(group) => {
                    ancestors.push((layer_id, layer_index));
                    Self::append_layers(
                        &group.children,
                        layer_indices,
                        ancestors,
                        Some(top_level_id),
                        projections,
                    )?;
                    ancestors.pop();
                }
                SilicaHierarchy::Layer(layer) => {
                    let mask = layer
                        .mask
                        .as_ref()
                        .map(|mask| {
                            let mask_id = LayerId::from(mask.hierarchy_id());
                            layer_indices
                                .get(&mask_id)
                                .copied()
                                .map(|index| (mask_id, index))
                                .ok_or(CompositorProjectionError::MissingLayer(mask_id))
                        })
                        .transpose()?;
                    projections.push(LayerProjection {
                        layer_id,
                        layer_index,
                        ancestors: ancestors.clone().into_boxed_slice(),
                        mask,
                        top_level_id,
                    });
                }
            }
        }
        Ok(())
    }

    fn project(
        &self,
        snapshot: &DocumentSnapshot,
    ) -> Result<Vec<CompositeLayer>, CompositorProjectionError> {
        let animation_selection = AnimationLayerSelection::from_snapshot(snapshot);
        self.layers
            .iter()
            .map(|projection| {
                let state = snapshot_layer(snapshot, projection.layer_id, projection.layer_index)?;
                let (Some(opacity), Some(blend_mode), Some(clipped)) =
                    (state.opacity, state.blend_mode, state.clipped)
                else {
                    return Err(CompositorProjectionError::MissingLayerProperties(
                        projection.layer_id,
                    ));
                };
                let ancestor_hidden = projection.ancestors.iter().try_fold(
                    false,
                    |hidden, &(ancestor_id, ancestor_index)| {
                        let ancestor = snapshot_layer(snapshot, ancestor_id, ancestor_index)?;
                        Ok::<_, CompositorProjectionError>(hidden || !ancestor.visible)
                    },
                )?;
                let animation_hidden = animation_selection
                    .is_some_and(|selection| !selection.contains(projection.top_level_id));
                let mask_hidden = match projection.mask {
                    Some((mask_id, mask_index)) => {
                        !snapshot_layer(snapshot, mask_id, mask_index)?.visible
                    }
                    None => true,
                };

                Ok(CompositeLayer {
                    opacity,
                    blend: super::blend::convert_blend(blend_mode),
                    clipped,
                    hidden: ancestor_hidden || !state.visible || animation_hidden,
                    mask_hidden,
                })
            })
            .collect()
    }
}

fn snapshot_layer(
    snapshot: &DocumentSnapshot,
    layer_id: LayerId,
    layer_index: usize,
) -> Result<&LayerSnapshot, CompositorProjectionError> {
    snapshot
        .layers
        .get(layer_index)
        .filter(|layer| layer.layer_id == layer_id)
        .ok_or(CompositorProjectionError::MissingLayer(layer_id))
}

impl CompositorRenderState {
    fn project(
        plan: &CompositorProjectionPlan,
        snapshot: &DocumentSnapshot,
    ) -> Result<Self, CompositorProjectionError> {
        Ok(Self {
            layers: plan.project(snapshot)?,
            background: snapshot
                .background_visible
                .then_some(snapshot.background_color),
            flipped: snapshot.flipped,
        })
    }
}

impl CompositorApp {
    /// Transform tree structure of layers into a linear list of
    /// layers for rendering.
    fn flatten_layers(composite_layers: &mut Vec<CompositeLayer>, layers: &[SilicaHierarchy]) {
        composite_layers.clear();

        fn inner(
            layers: &[SilicaHierarchy],
            composite_layers: &mut Vec<CompositeLayer>,
            override_hidden: bool,
        ) {
            for layer in layers.iter().rev() {
                match layer {
                    SilicaHierarchy::Group(group) => {
                        inner(
                            &group.children,
                            composite_layers,
                            group.hidden | override_hidden,
                        );
                    }
                    SilicaHierarchy::Layer(layer) => {
                        composite_layers.push(CompositeLayer {
                            opacity: layer.opacity,
                            blend: super::blend::convert_blend(layer.blend),
                            clipped: layer.clipped,
                            hidden: layer.hidden | override_hidden,
                            mask_hidden: layer.mask.as_ref().map_or(true, |mask| mask.hidden),
                        });
                    }
                }
            }
        }

        inner(layers, composite_layers, false);
    }

    fn flatten_chunks(
        composite_layers: &mut Vec<ChunkTile>,
        layers: &[SilicaHierarchy],
        render_masks: bool,
    ) {
        composite_layers.clear();

        let mut layer_counter = 0;

        fn inner<'a>(
            layers: &'a [SilicaHierarchy],
            chunks: &mut Vec<ChunkTile>,
            clip_layer: &mut Option<&'a SilicaLayer>,
            layer_counter: &mut u32,
            render_masks: bool,
        ) {
            for layer in layers.iter().rev() {
                match layer {
                    SilicaHierarchy::Group(group) => {
                        inner(&group.children, chunks, clip_layer, layer_counter, true);
                    }
                    SilicaHierarchy::Layer(layer) => {
                        for chunk in layer.image.chunks.iter() {
                            let clip_atlas_index = clip_layer.as_ref().and_then(|clip_layer| {
                                clip_layer
                                    .image
                                    .chunks
                                    .iter()
                                    .find(|clip_chunk| {
                                        clip_chunk.col == chunk.col && clip_chunk.row == chunk.row
                                    })
                                    .map(|clip_chunk| clip_chunk.atlas_index)
                            });

                            let mask_atlas_index = if render_masks {
                                layer.mask.as_ref().and_then(|mask| {
                                    mask.image
                                        .chunks
                                        .iter()
                                        .find(|mask_chunk| {
                                            mask_chunk.col == chunk.col
                                                && mask_chunk.row == chunk.row
                                        })
                                        .map(|mask_chunk| mask_chunk.atlas_index)
                                })
                            } else {
                                None
                            };

                            chunks.push(ChunkTile {
                                col: chunk.col,
                                row: chunk.row,
                                atlas_index: chunk.atlas_index,
                                mask_atlas_index,
                                clip_atlas_index,
                                layer_index: *layer_counter,
                            });
                        }
                        *clip_layer = Some(layer);
                        *layer_counter += 1;
                    }
                }
            }
        }

        inner(
            layers,
            composite_layers,
            &mut None,
            &mut layer_counter,
            render_masks,
        );
    }

    pub fn generate_layers_preview(
        pipeline: &Pipeline,
        target: &mut Compositor,
        preview_textures: &wgpu::Texture,
        layers: &[SilicaHierarchy],
    ) {
        for layer in layers.iter() {
            {
                let layer = std::slice::from_ref(layer);
                let mut composite_layers = Vec::new();
                CompositorApp::flatten_layers(&mut composite_layers, layer);

                target.load_layer_buffer(composite_layers.as_slice());

                let mut composite_chunks = Vec::new();
                CompositorApp::flatten_chunks(&mut composite_chunks, layer, false);
                composite_chunks.sort_by_key(|v| (v.col, v.row));
                target.load_chunk_buffer(composite_chunks.as_slice());
            }
            match layer {
                SilicaHierarchy::Group(group) => {
                    target.render(pipeline, preview_textures.create_view_layer(group.id));
                    Self::generate_layers_preview(
                        pipeline,
                        target,
                        preview_textures,
                        &group.children,
                    );
                }

                SilicaHierarchy::Layer(layer) => {
                    target.render(pipeline, preview_textures.create_view_layer(layer.id));
                    if let Some(mask_layer) = &layer.mask {
                        Self::generate_layers_preview(
                            pipeline,
                            target,
                            preview_textures,
                            std::slice::from_ref(&SilicaHierarchy::Layer(*mask_layer.clone())),
                        );
                    }
                }
            }
        }
    }

    pub fn new(
        id: InstanceKey,
        pipeline: Pipeline,
        file: Arc<ProcreateFile>,
        snapshot: &DocumentSnapshot,
        target: Compositor,
    ) -> Result<(Self, CompositorHandle), CompositorProjectionError> {
        let projection_plan = Arc::new(CompositorProjectionPlan::new(&file, snapshot)?);
        let state = Arc::new(CompositorRenderState::project(&projection_plan, snapshot)?);
        let (tx, mut rx) = tokio::sync::watch::channel(state);

        rx.mark_changed();

        let compositor = Self {
            id,
            rx,
            target,
            pipeline,
            chunk_source: Some(file),
            flat_chunks: Vec::new(),
        };

        let handle = CompositorHandle {
            compositor_sender: tx,
            projection_plan,
        };

        Ok((compositor, handle))
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub async fn rendering_thread(mut self, output_texture: wgpu::Texture) {
        loop {
            let file = match self.rx.changed().await {
                Ok(()) => (*self.rx.borrow_and_update()).clone(),
                Err(_) => break,
            };

            self.render_inner(&file, &output_texture);
        }

        log::debug!("{} Done rendering", self.id)
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn rendering_tick_blocking(&mut self, output_texture: &wgpu::Texture) {
        match self.rx.has_changed() {
            Ok(true) => {
                let file = (*self.rx.borrow_and_update()).clone();
                self.render_inner(&file, output_texture);
            }
            Ok(false) => {}
            Err(_) => {
                panic!("{} Compositor channel closed unexpectedly", self.id);
            }
        }
    }

    fn render_inner(&mut self, state: &CompositorRenderState, output_texture: &wgpu::Texture) {
        if let Some(file) = self.chunk_source.take() {
            Self::flatten_chunks(&mut self.flat_chunks, &file.layers, true);
            self.flat_chunks.sort_by_key(|v| (v.col, v.row));
            self.target.load_chunk_buffer(self.flat_chunks.as_slice());

            log::debug!("{} Linearized {} chunks", self.id, self.flat_chunks.len());
        }

        self.target.load_layer_buffer(&state.layers);

        self.target.set_background(state.background);
        self.target
            .set_flipped(state.flipped.horizontally, state.flipped.vertically);
        self.target
            .render(&self.pipeline, output_texture.create_default_view());
        // ENABLE TO DEBUG: hold the lock to make sure the GUI is responsive
        // {
        //     const { assert!(cfg!(debug_assertions)); }
        //     std::thread::sleep(std::time::Duration::from_secs(1));
        // }
        // Debugging notes: if the GPU is highly contended, the main
        // GUI rendering can still be somewhat sluggish.
    }
}
