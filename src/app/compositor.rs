use eframe::wgpu;

use silica_gpu::{ProcreateFile, SilicaHierarchy, SilicaLayer};
use silicate_compositor::tex::TextureExt;
use silicate_compositor::{
    ChunkTile, CompositeIsolation, CompositeLayer, CompositePhase, Compositor, pipeline::Pipeline,
};
use silicate_runtime::{
    AnimationOnionSkinFrame, CanvasFlipped, DocumentSnapshot, LayerId, LayerSnapshot,
};
use std::{collections::HashMap, num::NonZeroU32, sync::Arc};
use thiserror::Error;
use tokio::sync::watch::{Receiver, Sender};

use crate::app::instance::InstanceKey;

pub struct CompositorApp {
    target: Compositor,
    pipeline: Pipeline,
    rx: Receiver<Arc<CompositorRenderState>>,
    id: InstanceKey,
    chunk_source: Arc<ProcreateFile>,
    loaded_clip_sources: Option<Vec<Option<usize>>>,
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
    clip_sources: Vec<Option<usize>>,
    background: Option<[f32; 4]>,
    flipped: CanvasFlipped,
}

/// Compiled once because animation playback turns projection into a frame-cadence path.
/// Stable runtime identities let each update avoid rebuilding maps or walking topology.
struct CompositorProjectionPlan {
    layers: Vec<LayerProjection>,
    clipping_scope_count: usize,
}

struct LayerProjection {
    layer_id: LayerId,
    layer_index: usize,
    ancestors: Box<[(LayerId, usize)]>,
    mask: Option<(LayerId, usize)>,
    clipping_scope: usize,
    top_level_id: LayerId,
    top_level_index: NonZeroU32,
}

#[derive(Clone, Copy)]
struct TopLevelProjection {
    layer_id: LayerId,
    layer_index: NonZeroU32,
}

struct ProjectedLayers {
    layers: Vec<CompositeLayer>,
    clip_sources: Vec<Option<usize>>,
}

struct AnimationLayerSelection {
    source: Option<LayerId>,
    foreground: Option<LayerId>,
    background: Option<LayerId>,
    onion_skins: Vec<AnimationOnionSkinFrame>,
    primary_opacity: f32,
}

#[derive(Clone, Copy)]
struct AnimationLayerStyle {
    phase: CompositePhase,
    frame_opacity: f32,
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
                onion_skins: snapshot.animation_onion_skin_frames(),
                primary_opacity: snapshot.animation.map_or(1.0, |animation| {
                    if animation.blend_primary_frame {
                        animation.onion_skin_opacity
                    } else {
                        1.0
                    }
                }),
            })
    }

    fn style(&self, layer_id: LayerId) -> Option<AnimationLayerStyle> {
        if self.background == Some(layer_id) {
            return Some(AnimationLayerStyle {
                phase: CompositePhase::Base,
                frame_opacity: 1.0,
            });
        }
        if let Some(onion_skin) = self
            .onion_skins
            .iter()
            .find(|onion_skin| onion_skin.source_layer_id == layer_id)
        {
            return Some(AnimationLayerStyle {
                phase: CompositePhase::Base,
                frame_opacity: onion_skin.opacity,
            });
        }

        let primary_phase = if self.onion_skins.is_empty() {
            CompositePhase::Base
        } else {
            CompositePhase::Primary
        };
        if self.source == Some(layer_id) {
            return Some(AnimationLayerStyle {
                phase: primary_phase,
                frame_opacity: self.primary_opacity,
            });
        }
        (self.foreground == Some(layer_id)).then_some(AnimationLayerStyle {
            phase: primary_phase,
            frame_opacity: 1.0,
        })
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

    pub(crate) fn clipping_base_layer_id(
        &self,
        snapshot: &DocumentSnapshot,
        layer_id: LayerId,
    ) -> Result<Option<LayerId>, CompositorProjectionError> {
        let layer_index = self
            .projection_plan
            .layers
            .iter()
            .position(|projection| projection.layer_id == layer_id)
            .ok_or(CompositorProjectionError::MissingLayer(layer_id))?;
        let projected = self.projection_plan.project(snapshot)?;
        Ok(projected.clip_sources[layer_index]
            .map(|source_index| self.projection_plan.layers[source_index].layer_id))
    }
}

pub(crate) fn resolve_clipping_sources(
    layer_count: usize,
    scope_count: usize,
    mut layer: impl FnMut(usize) -> (bool, usize),
) -> Vec<Option<usize>> {
    let mut scope_bases = vec![None; scope_count];
    (0..layer_count)
        .map(|layer_index| {
            let (clipped, scope) = layer(layer_index);
            if clipped {
                scope_bases[scope]
            } else {
                scope_bases[scope] = Some(layer_index);
                None
            }
        })
        .collect()
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
        let mut next_clipping_scope = 1;
        let mut root_clipping_scope = 0;
        for (index, node) in file.layers.iter().rev().enumerate() {
            let top_level_id = match node {
                SilicaHierarchy::Group(group) => LayerId::from(group.hierarchy_id()),
                SilicaHierarchy::Layer(layer) => LayerId::from(layer.hierarchy_id()),
            };
            let top_level_index = NonZeroU32::new(
                u32::try_from(index + 1).expect("top-level hierarchy exceeds compositor limits"),
            )
            .expect("top-level compositor indices start at one");
            let top_level = TopLevelProjection {
                layer_id: top_level_id,
                layer_index: top_level_index,
            };
            root_clipping_scope = Self::append_layers(
                std::slice::from_ref(node),
                &layer_indices,
                &mut Vec::new(),
                root_clipping_scope,
                &mut next_clipping_scope,
                top_level,
                &mut layers,
            )?;
        }

        Ok(Self {
            layers,
            clipping_scope_count: next_clipping_scope,
        })
    }

    fn append_layers(
        nodes: &[SilicaHierarchy],
        layer_indices: &HashMap<LayerId, usize>,
        ancestors: &mut Vec<(LayerId, usize)>,
        clipping_scope: usize,
        next_clipping_scope: &mut usize,
        top_level: TopLevelProjection,
        projections: &mut Vec<LayerProjection>,
    ) -> Result<usize, CompositorProjectionError> {
        let mut clipping_scope = clipping_scope;
        for node in nodes.iter().rev() {
            let layer_id = match node {
                SilicaHierarchy::Group(group) => LayerId::from(group.hierarchy_id()),
                SilicaHierarchy::Layer(layer) => LayerId::from(layer.hierarchy_id()),
            };
            let layer_index = *layer_indices
                .get(&layer_id)
                .ok_or(CompositorProjectionError::MissingLayer(layer_id))?;
            match node {
                SilicaHierarchy::Group(group) => {
                    ancestors.push((layer_id, layer_index));
                    let child_clipping_scope = *next_clipping_scope;
                    *next_clipping_scope += 1;
                    Self::append_layers(
                        &group.children,
                        layer_indices,
                        ancestors,
                        child_clipping_scope,
                        next_clipping_scope,
                        top_level,
                        projections,
                    )?;
                    ancestors.pop();
                    // Groups are composition boundaries until group-backed clipping is modeled.
                    clipping_scope = *next_clipping_scope;
                    *next_clipping_scope += 1;
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
                        clipping_scope,
                        top_level_id: top_level.layer_id,
                        top_level_index: top_level.layer_index,
                    });
                }
            }
        }
        Ok(clipping_scope)
    }

    fn project(
        &self,
        snapshot: &DocumentSnapshot,
    ) -> Result<ProjectedLayers, CompositorProjectionError> {
        let animation_selection = AnimationLayerSelection::from_snapshot(snapshot);
        let mut layers = self
            .layers
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
                let animation_style = animation_selection
                    .as_ref()
                    .and_then(|selection| selection.style(projection.top_level_id));
                let animation_hidden = animation_selection.is_some() && animation_style.is_none();
                let animation_style = animation_style.unwrap_or(AnimationLayerStyle {
                    phase: CompositePhase::Base,
                    frame_opacity: 1.0,
                });
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
                    clip_mask_hidden: true,
                    phase: animation_style.phase,
                    isolation: (animation_style.frame_opacity < 1.0).then_some(
                        CompositeIsolation {
                            id: projection.top_level_index,
                            opacity: animation_style.frame_opacity,
                        },
                    ),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let clip_sources =
            resolve_clipping_sources(layers.len(), self.clipping_scope_count, |layer_index| {
                (
                    layers[layer_index].clipped,
                    self.layers[layer_index].clipping_scope,
                )
            });
        for (layer_index, clip_source) in clip_sources.iter().copied().enumerate() {
            if layers[layer_index].clipped {
                if let Some(source_index) = clip_source {
                    let source: &CompositeLayer = &layers[source_index];
                    let source_hidden = source.hidden;
                    let source_mask_hidden = source.mask_hidden;
                    layers[layer_index].hidden |= source_hidden;
                    layers[layer_index].clip_mask_hidden = source_mask_hidden;
                } else {
                    layers[layer_index].clipped = false;
                }
            }
        }

        Ok(ProjectedLayers {
            layers,
            clip_sources,
        })
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
        let projected = plan.project(snapshot)?;
        Ok(Self {
            layers: projected.layers,
            clip_sources: projected.clip_sources,
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
    fn flatten_layers(
        composite_layers: &mut Vec<CompositeLayer>,
        clip_sources: &mut Vec<Option<usize>>,
        layers: &[SilicaHierarchy],
    ) {
        composite_layers.clear();
        clip_sources.clear();

        fn inner(
            layers: &[SilicaHierarchy],
            composite_layers: &mut Vec<CompositeLayer>,
            clip_sources: &mut Vec<Option<usize>>,
            override_hidden: bool,
        ) {
            let mut clip_base = None;
            for layer in layers.iter().rev() {
                match layer {
                    SilicaHierarchy::Group(group) => {
                        inner(
                            &group.children,
                            composite_layers,
                            clip_sources,
                            group.hidden | override_hidden,
                        );
                        clip_base = None;
                    }
                    SilicaHierarchy::Layer(layer) => {
                        let clip_source = layer.clipped.then_some(clip_base).flatten();
                        let (source_hidden, clip_mask_hidden) =
                            clip_source.map_or((false, true), |source_index| {
                                let source: &CompositeLayer = &composite_layers[source_index];
                                (source.hidden, source.mask_hidden)
                            });
                        let layer_index = composite_layers.len();
                        composite_layers.push(CompositeLayer {
                            opacity: layer.opacity,
                            blend: super::blend::convert_blend(layer.blend),
                            clipped: clip_source.is_some(),
                            hidden: layer.hidden | override_hidden | source_hidden,
                            mask_hidden: layer.mask.as_ref().is_none_or(|mask| mask.hidden),
                            clip_mask_hidden,
                            phase: CompositePhase::Base,
                            isolation: None,
                        });
                        clip_sources.push(clip_source);
                        if !layer.clipped {
                            clip_base = Some(layer_index);
                        }
                    }
                }
            }
        }

        inner(layers, composite_layers, clip_sources, false);
    }

    fn flatten_chunks(
        chunks: &mut Vec<ChunkTile>,
        layers: &[SilicaHierarchy],
        clip_sources: &[Option<usize>],
        render_masks: bool,
    ) {
        chunks.clear();

        fn flatten_source_layers<'a>(
            layers: &'a [SilicaHierarchy],
            flat_layers: &mut Vec<&'a SilicaLayer>,
        ) {
            for layer in layers.iter().rev() {
                match layer {
                    SilicaHierarchy::Group(group) => {
                        flatten_source_layers(&group.children, flat_layers);
                    }
                    SilicaHierarchy::Layer(layer) => {
                        flat_layers.push(layer);
                    }
                }
            }
        }

        fn matching_chunk(layer: &SilicaLayer, col: u32, row: u32) -> Option<NonZeroU32> {
            layer
                .image
                .chunks
                .iter()
                .find(|chunk| chunk.col == col && chunk.row == row)
                .map(|chunk| chunk.atlas_index)
        }

        fn matching_mask_chunk(layer: &SilicaLayer, col: u32, row: u32) -> Option<NonZeroU32> {
            layer
                .mask
                .as_ref()
                .and_then(|mask| matching_chunk(mask, col, row))
        }

        let mut flat_layers = Vec::new();
        flatten_source_layers(layers, &mut flat_layers);
        debug_assert_eq!(flat_layers.len(), clip_sources.len());

        for (layer_index, layer) in flat_layers.iter().enumerate() {
            let clip_layer = clip_sources
                .get(layer_index)
                .copied()
                .flatten()
                .and_then(|source_index| flat_layers.get(source_index))
                .copied();
            for chunk in &layer.image.chunks {
                chunks.push(ChunkTile {
                    col: chunk.col,
                    row: chunk.row,
                    atlas_index: chunk.atlas_index,
                    mask_atlas_index: render_masks
                        .then(|| matching_mask_chunk(layer, chunk.col, chunk.row))
                        .flatten(),
                    clip_atlas_index: clip_layer
                        .and_then(|source| matching_chunk(source, chunk.col, chunk.row)),
                    clip_mask_atlas_index: clip_layer
                        .and_then(|source| matching_mask_chunk(source, chunk.col, chunk.row)),
                    layer_index: u32::try_from(layer_index)
                        .expect("layer count exceeds compositor limits"),
                });
            }
        }
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
                let mut clip_sources = Vec::new();
                CompositorApp::flatten_layers(&mut composite_layers, &mut clip_sources, layer);

                target.load_layer_buffer(composite_layers.as_slice());

                let mut composite_chunks = Vec::new();
                CompositorApp::flatten_chunks(&mut composite_chunks, layer, &clip_sources, false);
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
            chunk_source: file,
            loaded_clip_sources: None,
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

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn render_persisted_composite(&mut self, output_texture: &wgpu::Texture) -> bool {
        let Some(composite) = &self.chunk_source.composite else {
            return false;
        };
        // This diagnostic replaces the production chunk indirection. Invalidating the cache
        // guarantees a later runtime projection cannot accidentally reuse composite chunks.
        self.loaded_clip_sources = None;
        let mut chunks = composite
            .image
            .chunks
            .iter()
            .map(|chunk| ChunkTile {
                col: chunk.col,
                row: chunk.row,
                atlas_index: chunk.atlas_index,
                mask_atlas_index: None,
                clip_atlas_index: None,
                clip_mask_atlas_index: None,
                layer_index: 0,
            })
            .collect::<Vec<_>>();
        chunks.sort_by_key(|chunk| (chunk.col, chunk.row));
        self.target.load_chunk_buffer(&chunks);
        self.target.load_layer_buffer(&[CompositeLayer {
            opacity: 1.0,
            blend: silicate_compositor::blend::BlendingMode::Normal,
            clipped: false,
            hidden: false,
            mask_hidden: true,
            clip_mask_hidden: true,
            phase: CompositePhase::Base,
            isolation: None,
        }]);
        self.target.set_background(
            (!self.chunk_source.background_hidden).then_some(self.chunk_source.background_color),
        );
        self.target
            .render(&self.pipeline, output_texture.create_default_view());
        true
    }

    fn render_inner(&mut self, state: &CompositorRenderState, output_texture: &wgpu::Texture) {
        if self.loaded_clip_sources.as_deref() != Some(state.clip_sources.as_slice()) {
            Self::flatten_chunks(
                &mut self.flat_chunks,
                &self.chunk_source.layers,
                &state.clip_sources,
                true,
            );
            self.flat_chunks.sort_by_key(|v| (v.col, v.row));
            self.target.load_chunk_buffer(self.flat_chunks.as_slice());
            self.loaded_clip_sources = Some(state.clip_sources.clone());

            log::debug!(
                "{} Linearized {} chunks for updated clipping topology",
                self.id,
                self.flat_chunks.len()
            );
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
