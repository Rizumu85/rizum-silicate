mod animation;

pub use animation::{
    AnimationPlaybackDirection, AnimationPlaybackMode, AnimationPlaybackSnapshot, AnimationSnapshot,
};

use serde::{Deserialize, Serialize};
use silica::{BlendingMode, ProcreateFile, SilicaHierarchy};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LayerId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HistoryGroupId(u64);

impl HistoryGroupId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl LayerId {
    pub const fn hierarchy_id(self) -> silica::HierarchyId {
        silica::HierarchyId::new(self.0)
    }
}

impl From<silica::HierarchyId> for LayerId {
    fn from(value: silica::HierarchyId) -> Self {
        Self(value.get())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerKind {
    Layer,
    Group,
    Mask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerSnapshot {
    pub layer_id: LayerId,
    pub parent_id: Option<LayerId>,
    pub kind: LayerKind,
    pub name: Option<String>,
    pub visible: bool,
    pub animation_hold_duration: Option<u32>,
    pub clipped: Option<bool>,
    pub blend_mode: Option<BlendingMode>,
    pub opacity: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasFlipped {
    pub horizontally: bool,
    pub vertically: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub document_id: DocumentId,
    pub revision: u64,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub animation: Option<AnimationSnapshot>,
    pub animation_playback: Option<AnimationPlaybackSnapshot>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub canvas_size: CanvasSize,
    pub background_visible: bool,
    pub background_color: [f32; 4],
    pub flipped: CanvasFlipped,
    pub stroke_count: u64,
    pub layer_count: u32,
    pub layers: Vec<LayerSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationFrameSnapshot {
    pub source_layer_id: LayerId,
    pub hold_duration: u32,
}

impl DocumentSnapshot {
    pub fn animation_foreground_layer_id(&self) -> Option<LayerId> {
        self.animation
            .as_ref()
            .is_some_and(|animation| animation.first_item_is_foreground)
            .then(|| self.layers.iter().find(|layer| layer.parent_id.is_none()))
            .flatten()
            .map(|layer| layer.layer_id)
    }

    pub fn animation_background_layer_id(&self) -> Option<LayerId> {
        self.animation
            .as_ref()
            .is_some_and(|animation| animation.last_item_is_background)
            .then(|| {
                self.layers
                    .iter()
                    .rev()
                    .find(|layer| layer.parent_id.is_none())
            })
            .flatten()
            .map(|layer| layer.layer_id)
    }

    /// Derives the current visible frame sequence from the mutable layer snapshot.
    pub fn animation_frame_sources(&self) -> impl Iterator<Item = AnimationFrameSnapshot> + '_ {
        let foreground_layer_id = self.animation_foreground_layer_id();
        let background_layer_id = self.animation_background_layer_id();
        self.layers.iter().rev().filter_map(move |layer| {
            (layer.parent_id.is_none()
                && layer.visible
                && Some(layer.layer_id) != foreground_layer_id
                && Some(layer.layer_id) != background_layer_id)
                .then_some(layer.animation_hold_duration)
                .flatten()
                .map(|hold_duration| AnimationFrameSnapshot {
                    source_layer_id: layer.layer_id,
                    hold_duration,
                })
        })
    }

    /// Repeats frame identities for hold slots without duplicating image or GPU resources.
    pub fn animation_timeline_slots(&self) -> impl Iterator<Item = LayerId> + '_ {
        self.animation_frame_sources().flat_map(|frame| {
            std::iter::repeat_n(frame.source_layer_id, frame.hold_duration as usize + 1)
        })
    }

    pub fn animation_timeline_slot_count(&self) -> u64 {
        self.animation_frame_sources()
            .map(|frame| u64::from(frame.hold_duration) + 1)
            .sum()
    }

    pub fn animation_timeline_slot(&self, slot_index: u64) -> Option<LayerId> {
        let mut first_slot = 0_u64;
        for frame in self.animation_frame_sources() {
            let next_first_slot = first_slot + u64::from(frame.hold_duration) + 1;
            if slot_index < next_first_slot {
                return Some(frame.source_layer_id);
            }
            first_slot = next_first_slot;
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DocumentCommand {
    CloseDocument {
        document_id: DocumentId,
        discard_changes: bool,
    },
    MarkDocumentSaved {
        document_id: DocumentId,
    },
    Redo {
        document_id: DocumentId,
    },
    SeekAnimationTimeline {
        document_id: DocumentId,
        slot_index: u64,
    },
    SetAnimationPlaybackActive {
        document_id: DocumentId,
        active: bool,
    },
    SetAnimationPlaybackDirection {
        document_id: DocumentId,
        direction: AnimationPlaybackDirection,
    },
    SetAnimationPlaybackMode {
        document_id: DocumentId,
        mode: AnimationPlaybackMode,
    },
    SetAnimationPlaying {
        document_id: DocumentId,
        playing: bool,
    },
    SetBackgroundVisibility {
        document_id: DocumentId,
        visible: bool,
    },
    SetBackgroundColor {
        document_id: DocumentId,
        color: [f32; 4],
    },
    SetCanvasFlipped {
        document_id: DocumentId,
        flipped: CanvasFlipped,
    },
    SetLayerBlendMode {
        document_id: DocumentId,
        layer_id: LayerId,
        blend_mode: BlendingMode,
    },
    SetLayerClipped {
        document_id: DocumentId,
        layer_id: LayerId,
        clipped: bool,
    },
    SetLayerOpacity {
        document_id: DocumentId,
        layer_id: LayerId,
        opacity: f32,
    },
    SetLayerVisibility {
        document_id: DocumentId,
        layer_id: LayerId,
        visible: bool,
    },
    Undo {
        document_id: DocumentId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeEvent {
    DocumentOpened {
        snapshot: DocumentSnapshot,
    },
    DocumentClosed {
        document_id: DocumentId,
        revision: u64,
    },
    AnimationPlaybackChanged {
        document_id: DocumentId,
        playback: AnimationPlaybackSnapshot,
        revision: u64,
    },
    BackgroundVisibilityChanged {
        document_id: DocumentId,
        visible: bool,
        revision: u64,
    },
    BackgroundColorChanged {
        document_id: DocumentId,
        color: [f32; 4],
        revision: u64,
    },
    CanvasFlippedChanged {
        document_id: DocumentId,
        flipped: CanvasFlipped,
        revision: u64,
    },
    LayerBlendModeChanged {
        document_id: DocumentId,
        layer_id: LayerId,
        blend_mode: BlendingMode,
        revision: u64,
    },
    LayerClippedChanged {
        document_id: DocumentId,
        layer_id: LayerId,
        clipped: bool,
        revision: u64,
    },
    LayerOpacityChanged {
        document_id: DocumentId,
        layer_id: LayerId,
        opacity: f32,
        revision: u64,
    },
    LayerVisibilityChanged {
        document_id: DocumentId,
        layer_id: LayerId,
        visible: bool,
        revision: u64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeUpdate<T> {
    pub value: T,
    pub events: Vec<RuntimeEvent>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to parse Procreate document: {0}")]
    Parse(#[from] silica::error::SilicaError),
    #[error("document id space is exhausted")]
    DocumentIdExhausted,
    #[error("document {0:?} is not open")]
    DocumentNotFound(DocumentId),
    #[error("layer {layer_id:?} is not present in document {document_id:?}")]
    LayerNotFound {
        document_id: DocumentId,
        layer_id: LayerId,
    },
    #[error("layer {layer_id:?} in document {document_id:?} does not support clipping")]
    LayerDoesNotSupportClipping {
        document_id: DocumentId,
        layer_id: LayerId,
    },
    #[error("layer {layer_id:?} in document {document_id:?} does not support blend modes")]
    LayerDoesNotSupportBlendMode {
        document_id: DocumentId,
        layer_id: LayerId,
    },
    #[error("layer {layer_id:?} in document {document_id:?} does not support opacity")]
    LayerDoesNotSupportOpacity {
        document_id: DocumentId,
        layer_id: LayerId,
    },
    #[error("layer {layer_id:?} opacity must be finite and within 0..=1 (actual: {opacity})")]
    InvalidLayerOpacity { layer_id: LayerId, opacity: f32 },
    #[error("background color channel {channel} must be finite and within 0..=1 (actual: {value})")]
    InvalidBackgroundColor { channel: usize, value: f32 },
    #[error("animation frame rate must be within 1..=60 (actual: {0})")]
    InvalidAnimationFrameRate(u32),
    #[error("animation onion skin count must be within 0..=12 (actual: {0})")]
    InvalidAnimationOnionSkinCount(u32),
    #[error("animation onion skin opacity must be finite and within 0..=1 (actual: {0})")]
    InvalidAnimationOnionSkinOpacity(f32),
    #[error("document {0:?} does not contain Animation Assist metadata")]
    AnimationUnavailable(DocumentId),
    #[error("document {0:?} has no visible Animation Assist frames")]
    AnimationHasNoFrames(DocumentId),
    #[error(
        "animation slot {slot_index} is outside the timeline for document {document_id:?} (slot count: {slot_count})"
    )]
    AnimationSlotOutOfRange {
        document_id: DocumentId,
        slot_index: u64,
        slot_count: u64,
    },
    #[error(
        "layer {layer_id:?} animation hold duration must be within 0..=120 (actual: {hold_duration})"
    )]
    InvalidAnimationHoldDuration {
        layer_id: LayerId,
        hold_duration: u32,
    },
    #[error("revision space is exhausted for document {0:?}")]
    RevisionExhausted(DocumentId),
    #[error("document {0:?} has unsaved changes")]
    UnsavedChanges(DocumentId),
}

const MAX_HISTORY_ENTRIES: usize = 256;

struct DocumentRecord {
    snapshot: DocumentSnapshot,
    animation_frame_units: u64,
    history: Vec<HistoryEntry>,
    history_index: usize,
    saved_history_index: Option<usize>,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    group_id: Option<HistoryGroupId>,
    changes: Vec<HistoryChange>,
}

#[derive(Debug, Clone)]
enum HistoryChange {
    BackgroundVisibility {
        before: bool,
        after: bool,
    },
    BackgroundColor {
        before: [f32; 4],
        after: [f32; 4],
    },
    CanvasFlipped {
        before: CanvasFlipped,
        after: CanvasFlipped,
    },
    LayerBlendMode {
        layer_id: LayerId,
        before: BlendingMode,
        after: BlendingMode,
    },
    LayerClipped {
        layer_id: LayerId,
        before: bool,
        after: bool,
    },
    LayerOpacity {
        layer_id: LayerId,
        before: f32,
        after: f32,
    },
    LayerVisibility {
        layer_id: LayerId,
        before: bool,
        after: bool,
    },
}

impl HistoryChange {
    fn same_target(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BackgroundVisibility { .. }, Self::BackgroundVisibility { .. })
            | (Self::BackgroundColor { .. }, Self::BackgroundColor { .. })
            | (Self::CanvasFlipped { .. }, Self::CanvasFlipped { .. }) => true,
            (
                Self::LayerBlendMode { layer_id: left, .. },
                Self::LayerBlendMode {
                    layer_id: right, ..
                },
            )
            | (
                Self::LayerClipped { layer_id: left, .. },
                Self::LayerClipped {
                    layer_id: right, ..
                },
            )
            | (
                Self::LayerOpacity { layer_id: left, .. },
                Self::LayerOpacity {
                    layer_id: right, ..
                },
            )
            | (
                Self::LayerVisibility { layer_id: left, .. },
                Self::LayerVisibility {
                    layer_id: right, ..
                },
            ) => left == right,
            _ => false,
        }
    }

    fn merge_after(&mut self, newer: Self) {
        match (self, newer) {
            (
                Self::BackgroundVisibility { after, .. },
                Self::BackgroundVisibility { after: value, .. },
            ) => *after = value,
            (Self::BackgroundColor { after, .. }, Self::BackgroundColor { after: value, .. }) => {
                *after = value
            }
            (Self::CanvasFlipped { after, .. }, Self::CanvasFlipped { after: value, .. }) => {
                *after = value
            }
            (Self::LayerBlendMode { after, .. }, Self::LayerBlendMode { after: value, .. }) => {
                *after = value
            }
            (Self::LayerClipped { after, .. }, Self::LayerClipped { after: value, .. }) => {
                *after = value
            }
            (Self::LayerOpacity { after, .. }, Self::LayerOpacity { after: value, .. }) => {
                *after = value
            }
            (Self::LayerVisibility { after, .. }, Self::LayerVisibility { after: value, .. }) => {
                *after = value
            }
            _ => unreachable!("history target checked before merge"),
        }
    }

    fn is_noop(&self) -> bool {
        match self {
            Self::BackgroundVisibility { before, after }
            | Self::LayerClipped { before, after, .. }
            | Self::LayerVisibility { before, after, .. } => before == after,
            Self::BackgroundColor { before, after } => before == after,
            Self::CanvasFlipped { before, after } => before == after,
            Self::LayerBlendMode { before, after, .. } => before == after,
            Self::LayerOpacity { before, after, .. } => before == after,
        }
    }

    fn apply(
        &self,
        snapshot: &mut DocumentSnapshot,
        use_after: bool,
    ) -> Result<RuntimeEvent, RuntimeError> {
        let document_id = snapshot.document_id;
        let revision = snapshot
            .revision
            .checked_add(1)
            .ok_or(RuntimeError::RevisionExhausted(document_id))?;

        let event = match self {
            Self::BackgroundVisibility { before, after } => {
                let visible = if use_after { *after } else { *before };
                snapshot.background_visible = visible;
                RuntimeEvent::BackgroundVisibilityChanged {
                    document_id,
                    visible,
                    revision,
                }
            }
            Self::BackgroundColor { before, after } => {
                let color = if use_after { *after } else { *before };
                snapshot.background_color = color;
                RuntimeEvent::BackgroundColorChanged {
                    document_id,
                    color,
                    revision,
                }
            }
            Self::CanvasFlipped { before, after } => {
                let flipped = if use_after { *after } else { *before };
                snapshot.flipped = flipped;
                RuntimeEvent::CanvasFlippedChanged {
                    document_id,
                    flipped,
                    revision,
                }
            }
            Self::LayerBlendMode {
                layer_id,
                before,
                after,
            } => {
                let blend_mode = if use_after { *after } else { *before };
                let layer = layer_mut(snapshot, *layer_id)?;
                *layer
                    .blend_mode
                    .as_mut()
                    .ok_or(RuntimeError::LayerDoesNotSupportBlendMode {
                        document_id,
                        layer_id: *layer_id,
                    })? = blend_mode;
                RuntimeEvent::LayerBlendModeChanged {
                    document_id,
                    layer_id: *layer_id,
                    blend_mode,
                    revision,
                }
            }
            Self::LayerClipped {
                layer_id,
                before,
                after,
            } => {
                let clipped = if use_after { *after } else { *before };
                let layer = layer_mut(snapshot, *layer_id)?;
                *layer
                    .clipped
                    .as_mut()
                    .ok_or(RuntimeError::LayerDoesNotSupportClipping {
                        document_id,
                        layer_id: *layer_id,
                    })? = clipped;
                RuntimeEvent::LayerClippedChanged {
                    document_id,
                    layer_id: *layer_id,
                    clipped,
                    revision,
                }
            }
            Self::LayerOpacity {
                layer_id,
                before,
                after,
            } => {
                let opacity = if use_after { *after } else { *before };
                let layer = layer_mut(snapshot, *layer_id)?;
                *layer
                    .opacity
                    .as_mut()
                    .ok_or(RuntimeError::LayerDoesNotSupportOpacity {
                        document_id,
                        layer_id: *layer_id,
                    })? = opacity;
                RuntimeEvent::LayerOpacityChanged {
                    document_id,
                    layer_id: *layer_id,
                    opacity,
                    revision,
                }
            }
            Self::LayerVisibility {
                layer_id,
                before,
                after,
            } => {
                let visible = if use_after { *after } else { *before };
                layer_mut(snapshot, *layer_id)?.visible = visible;
                RuntimeEvent::LayerVisibilityChanged {
                    document_id,
                    layer_id: *layer_id,
                    visible,
                    revision,
                }
            }
        };
        snapshot.revision = revision;
        Ok(event)
    }
}

impl DocumentRecord {
    fn new(snapshot: DocumentSnapshot) -> Self {
        Self {
            snapshot,
            animation_frame_units: 0,
            history: Vec::new(),
            history_index: 0,
            saved_history_index: Some(0),
        }
    }

    fn record_change(&mut self, change: HistoryChange, group_id: Option<HistoryGroupId>) {
        if self.history_index < self.history.len() {
            if self
                .saved_history_index
                .is_some_and(|saved| saved > self.history_index)
            {
                self.saved_history_index = None;
            }
            self.history.truncate(self.history_index);
        }

        let merge_with_last = group_id.is_some()
            && self
                .history
                .last()
                .is_some_and(|entry| entry.group_id == group_id);
        if merge_with_last {
            if self.saved_history_index == Some(self.history_index) {
                self.saved_history_index = None;
            }
            let entry = self.history.last_mut().expect("history entry exists");
            if let Some(existing) = entry
                .changes
                .iter_mut()
                .find(|existing| existing.same_target(&change))
            {
                existing.merge_after(change);
                entry.changes.retain(|change| !change.is_noop());
            } else {
                entry.changes.push(change);
            }
            if entry.changes.is_empty() {
                self.history.pop();
            }
        } else {
            self.history.push(HistoryEntry {
                group_id,
                changes: vec![change],
            });
        }

        self.history_index = self.history.len();
        while self.history.len() > MAX_HISTORY_ENTRIES {
            self.history.remove(0);
            self.history_index -= 1;
            self.saved_history_index = self
                .saved_history_index
                .and_then(|saved| saved.checked_sub(1));
        }
        self.refresh_history_state();
    }

    fn undo(&mut self) -> Result<Vec<RuntimeEvent>, RuntimeError> {
        if self.history_index == 0 {
            return Ok(Vec::new());
        }
        let entry = self.history[self.history_index - 1].clone();
        let playback_anchor = animation::playback_anchor(&self.snapshot);
        let mut snapshot = self.snapshot.clone();
        let mut events = Vec::with_capacity(entry.changes.len());
        for change in entry.changes.iter().rev() {
            events.push(change.apply(&mut snapshot, false)?);
        }
        let playback_changed = animation::reconcile_playback(&mut snapshot, playback_anchor);
        if playback_changed {
            self.animation_frame_units = 0;
            if let Some(playback) = snapshot.animation_playback {
                events.push(RuntimeEvent::AnimationPlaybackChanged {
                    document_id: snapshot.document_id,
                    playback,
                    revision: snapshot.revision,
                });
            }
        }
        self.snapshot = snapshot;
        self.history_index -= 1;
        self.refresh_history_state();
        Ok(events)
    }

    fn redo(&mut self) -> Result<Vec<RuntimeEvent>, RuntimeError> {
        if self.history_index == self.history.len() {
            return Ok(Vec::new());
        }
        let entry = self.history[self.history_index].clone();
        let playback_anchor = animation::playback_anchor(&self.snapshot);
        let mut snapshot = self.snapshot.clone();
        let mut events = Vec::with_capacity(entry.changes.len());
        for change in &entry.changes {
            events.push(change.apply(&mut snapshot, true)?);
        }
        let playback_changed = animation::reconcile_playback(&mut snapshot, playback_anchor);
        if playback_changed {
            self.animation_frame_units = 0;
            if let Some(playback) = snapshot.animation_playback {
                events.push(RuntimeEvent::AnimationPlaybackChanged {
                    document_id: snapshot.document_id,
                    playback,
                    revision: snapshot.revision,
                });
            }
        }
        self.snapshot = snapshot;
        self.history_index += 1;
        self.refresh_history_state();
        Ok(events)
    }

    fn mark_saved(&mut self) {
        self.saved_history_index = Some(self.history_index);
        self.refresh_history_state();
    }

    fn refresh_history_state(&mut self) {
        self.snapshot.can_undo = self.history_index > 0;
        self.snapshot.can_redo = self.history_index < self.history.len();
        self.snapshot.dirty = self.saved_history_index != Some(self.history_index);
    }

    fn update_animation_playback(
        &mut self,
        update: impl FnOnce(&mut DocumentSnapshot, &mut u64) -> Result<(), RuntimeError>,
    ) -> Result<Vec<RuntimeEvent>, RuntimeError> {
        let before = self.snapshot.animation_playback;
        let frame_units_before = self.animation_frame_units;
        if let Err(error) = update(&mut self.snapshot, &mut self.animation_frame_units) {
            self.snapshot.animation_playback = before;
            self.animation_frame_units = frame_units_before;
            return Err(error);
        }
        if self.snapshot.animation_playback == before {
            return Ok(Vec::new());
        }

        let document_id = self.snapshot.document_id;
        let Some(revision) = self.snapshot.revision.checked_add(1) else {
            self.snapshot.animation_playback = before;
            self.animation_frame_units = frame_units_before;
            return Err(RuntimeError::RevisionExhausted(document_id));
        };
        self.snapshot.revision = revision;
        let playback = self
            .snapshot
            .animation_playback
            .expect("playback updates preserve animation availability");

        Ok(vec![RuntimeEvent::AnimationPlaybackChanged {
            document_id,
            playback,
            revision,
        }])
    }
}

fn layer_mut(
    snapshot: &mut DocumentSnapshot,
    layer_id: LayerId,
) -> Result<&mut LayerSnapshot, RuntimeError> {
    let document_id = snapshot.document_id;
    snapshot
        .layers
        .iter_mut()
        .find(|layer| layer.layer_id == layer_id)
        .ok_or(RuntimeError::LayerNotFound {
            document_id,
            layer_id,
        })
}

#[derive(Default)]
pub struct DocumentRuntime {
    next_document_id: u64,
    documents: HashMap<DocumentId, DocumentRecord>,
}

impl DocumentRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, bytes: &[u8]) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        let document = ProcreateFile::open(bytes)?;
        self.open_document(&document)
    }

    pub fn open_document(
        &mut self,
        document: &ProcreateFile,
    ) -> Result<RuntimeUpdate<DocumentSnapshot>, RuntimeError> {
        let document_id = DocumentId(self.next_document_id);
        let next_document_id = self
            .next_document_id
            .checked_add(1)
            .ok_or(RuntimeError::DocumentIdExhausted)?;

        let record = DocumentRecord::new(snapshot(document_id, document)?);
        let snapshot = record.snapshot.clone();
        self.next_document_id = next_document_id;
        self.documents.insert(document_id, record);

        Ok(RuntimeUpdate {
            value: snapshot.clone(),
            events: vec![RuntimeEvent::DocumentOpened { snapshot }],
        })
    }

    pub fn snapshot(&self, document_id: DocumentId) -> Result<DocumentSnapshot, RuntimeError> {
        self.documents
            .get(&document_id)
            .map(|record| record.snapshot.clone())
            .ok_or(RuntimeError::DocumentNotFound(document_id))
    }

    pub fn animation_playback(
        &self,
        document_id: DocumentId,
    ) -> Result<AnimationPlaybackSnapshot, RuntimeError> {
        self.documents
            .get(&document_id)
            .ok_or(RuntimeError::DocumentNotFound(document_id))?
            .snapshot
            .animation_playback
            .ok_or(RuntimeError::AnimationUnavailable(document_id))
    }

    /// Advances the presentation clock without cloning the full document snapshot.
    pub fn advance_animation(
        &mut self,
        document_id: DocumentId,
        elapsed: std::time::Duration,
    ) -> Result<RuntimeUpdate<AnimationPlaybackSnapshot>, RuntimeError> {
        let record = self
            .documents
            .get_mut(&document_id)
            .ok_or(RuntimeError::DocumentNotFound(document_id))?;
        let events = record.update_animation_playback(|snapshot, frame_units| {
            animation::advance(snapshot, frame_units, elapsed)
        })?;
        let playback = record
            .snapshot
            .animation_playback
            .ok_or(RuntimeError::AnimationUnavailable(document_id))?;

        Ok(RuntimeUpdate {
            value: playback,
            events,
        })
    }

    pub fn dispatch(
        &mut self,
        command: DocumentCommand,
    ) -> Result<RuntimeUpdate<()>, RuntimeError> {
        self.dispatch_inner(command, None)
    }

    pub fn dispatch_grouped(
        &mut self,
        command: DocumentCommand,
        group_id: HistoryGroupId,
    ) -> Result<RuntimeUpdate<()>, RuntimeError> {
        self.dispatch_inner(command, Some(group_id))
    }

    fn dispatch_inner(
        &mut self,
        command: DocumentCommand,
        history_group: Option<HistoryGroupId>,
    ) -> Result<RuntimeUpdate<()>, RuntimeError> {
        match command {
            DocumentCommand::CloseDocument {
                document_id,
                discard_changes,
            } => {
                if !discard_changes
                    && self
                        .documents
                        .get(&document_id)
                        .ok_or(RuntimeError::DocumentNotFound(document_id))?
                        .snapshot
                        .dirty
                {
                    return Err(RuntimeError::UnsavedChanges(document_id));
                }
                let record = self
                    .documents
                    .remove(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::DocumentClosed {
                        document_id,
                        revision: record.snapshot.revision,
                    }],
                })
            }
            DocumentCommand::MarkDocumentSaved { document_id } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                record.mark_saved();
                Ok(RuntimeUpdate {
                    value: (),
                    events: Vec::new(),
                })
            }
            DocumentCommand::Undo { document_id } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.undo()?,
                })
            }
            DocumentCommand::Redo { document_id } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.redo()?,
                })
            }
            DocumentCommand::SeekAnimationTimeline {
                document_id,
                slot_index,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.update_animation_playback(|snapshot, frame_units| {
                        animation::seek(snapshot, frame_units, slot_index)
                    })?,
                })
            }
            DocumentCommand::SetAnimationPlaybackActive {
                document_id,
                active,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.update_animation_playback(|snapshot, frame_units| {
                        animation::set_active(snapshot, frame_units, active)
                    })?,
                })
            }
            DocumentCommand::SetAnimationPlaybackDirection {
                document_id,
                direction,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.update_animation_playback(|snapshot, frame_units| {
                        animation::set_direction(snapshot, frame_units, direction)
                    })?,
                })
            }
            DocumentCommand::SetAnimationPlaybackMode { document_id, mode } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.update_animation_playback(|snapshot, frame_units| {
                        animation::set_mode(snapshot, frame_units, mode)
                    })?,
                })
            }
            DocumentCommand::SetAnimationPlaying {
                document_id,
                playing,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                Ok(RuntimeUpdate {
                    value: (),
                    events: record.update_animation_playback(|snapshot, frame_units| {
                        animation::set_playing(snapshot, frame_units, playing)
                    })?,
                })
            }
            DocumentCommand::SetBackgroundColor { document_id, color } => {
                validate_background_color(color)?;
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let before = record.snapshot.background_color;
                if before == color {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                record.snapshot.background_color = color;
                record.snapshot.revision = revision;
                record.record_change(
                    HistoryChange::BackgroundColor {
                        before,
                        after: color,
                    },
                    history_group,
                );

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::BackgroundColorChanged {
                        document_id,
                        color,
                        revision,
                    }],
                })
            }
            DocumentCommand::SetBackgroundVisibility {
                document_id,
                visible,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let before = record.snapshot.background_visible;
                if before == visible {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                record.snapshot.background_visible = visible;
                record.snapshot.revision = revision;
                record.record_change(
                    HistoryChange::BackgroundVisibility {
                        before,
                        after: visible,
                    },
                    history_group,
                );

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::BackgroundVisibilityChanged {
                        document_id,
                        visible,
                        revision,
                    }],
                })
            }
            DocumentCommand::SetCanvasFlipped {
                document_id,
                flipped,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let before = record.snapshot.flipped;
                if before == flipped {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                record.snapshot.flipped = flipped;
                record.snapshot.revision = revision;
                record.record_change(
                    HistoryChange::CanvasFlipped {
                        before,
                        after: flipped,
                    },
                    history_group,
                );

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::CanvasFlippedChanged {
                        document_id,
                        flipped,
                        revision,
                    }],
                })
            }
            DocumentCommand::SetLayerClipped {
                document_id,
                layer_id,
                clipped,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let layer = record
                    .snapshot
                    .layers
                    .iter_mut()
                    .find(|layer| layer.layer_id == layer_id)
                    .ok_or(RuntimeError::LayerNotFound {
                        document_id,
                        layer_id,
                    })?;
                let current =
                    layer
                        .clipped
                        .as_mut()
                        .ok_or(RuntimeError::LayerDoesNotSupportClipping {
                            document_id,
                            layer_id,
                        })?;
                let before = *current;
                if *current == clipped {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                *current = clipped;
                record.snapshot.revision = revision;
                record.record_change(
                    HistoryChange::LayerClipped {
                        layer_id,
                        before,
                        after: clipped,
                    },
                    history_group,
                );

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::LayerClippedChanged {
                        document_id,
                        layer_id,
                        clipped,
                        revision,
                    }],
                })
            }
            DocumentCommand::SetLayerBlendMode {
                document_id,
                layer_id,
                blend_mode,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let layer = record
                    .snapshot
                    .layers
                    .iter_mut()
                    .find(|layer| layer.layer_id == layer_id)
                    .ok_or(RuntimeError::LayerNotFound {
                        document_id,
                        layer_id,
                    })?;
                let current = layer.blend_mode.as_mut().ok_or(
                    RuntimeError::LayerDoesNotSupportBlendMode {
                        document_id,
                        layer_id,
                    },
                )?;
                let before = *current;
                if *current == blend_mode {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                *current = blend_mode;
                record.snapshot.revision = revision;
                record.record_change(
                    HistoryChange::LayerBlendMode {
                        layer_id,
                        before,
                        after: blend_mode,
                    },
                    history_group,
                );

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::LayerBlendModeChanged {
                        document_id,
                        layer_id,
                        blend_mode,
                        revision,
                    }],
                })
            }
            DocumentCommand::SetLayerOpacity {
                document_id,
                layer_id,
                opacity,
            } => {
                validate_opacity(layer_id, opacity)?;
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let layer = record
                    .snapshot
                    .layers
                    .iter_mut()
                    .find(|layer| layer.layer_id == layer_id)
                    .ok_or(RuntimeError::LayerNotFound {
                        document_id,
                        layer_id,
                    })?;
                let current =
                    layer
                        .opacity
                        .as_mut()
                        .ok_or(RuntimeError::LayerDoesNotSupportOpacity {
                            document_id,
                            layer_id,
                        })?;
                let before = *current;
                if *current == opacity {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                *current = opacity;
                record.snapshot.revision = revision;
                record.record_change(
                    HistoryChange::LayerOpacity {
                        layer_id,
                        before,
                        after: opacity,
                    },
                    history_group,
                );

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::LayerOpacityChanged {
                        document_id,
                        layer_id,
                        opacity,
                        revision,
                    }],
                })
            }
            DocumentCommand::SetLayerVisibility {
                document_id,
                layer_id,
                visible,
            } => {
                let record = self
                    .documents
                    .get_mut(&document_id)
                    .ok_or(RuntimeError::DocumentNotFound(document_id))?;
                let playback_anchor = animation::playback_anchor(&record.snapshot);
                let layer = record
                    .snapshot
                    .layers
                    .iter_mut()
                    .find(|layer| layer.layer_id == layer_id)
                    .ok_or(RuntimeError::LayerNotFound {
                        document_id,
                        layer_id,
                    })?;
                let before = layer.visible;
                if layer.visible == visible {
                    return Ok(RuntimeUpdate {
                        value: (),
                        events: Vec::new(),
                    });
                }
                let revision = record
                    .snapshot
                    .revision
                    .checked_add(1)
                    .ok_or(RuntimeError::RevisionExhausted(document_id))?;
                layer.visible = visible;
                record.snapshot.revision = revision;
                let playback_changed =
                    animation::reconcile_playback(&mut record.snapshot, playback_anchor);
                if playback_changed {
                    record.animation_frame_units = 0;
                }
                record.record_change(
                    HistoryChange::LayerVisibility {
                        layer_id,
                        before,
                        after: visible,
                    },
                    history_group,
                );

                let mut events = vec![RuntimeEvent::LayerVisibilityChanged {
                    document_id,
                    layer_id,
                    visible,
                    revision,
                }];
                if playback_changed && let Some(playback) = record.snapshot.animation_playback {
                    events.push(RuntimeEvent::AnimationPlaybackChanged {
                        document_id,
                        playback,
                        revision,
                    });
                }

                Ok(RuntimeUpdate { value: (), events })
            }
        }
    }
}

fn snapshot(
    document_id: DocumentId,
    document: &ProcreateFile,
) -> Result<DocumentSnapshot, RuntimeError> {
    validate_background_color(document.background_color)?;
    let layers = layer_snapshots(&document.layers)?;
    let mut snapshot = DocumentSnapshot {
        document_id,
        revision: 0,
        dirty: false,
        can_undo: false,
        can_redo: false,
        animation: animation::project(document)?,
        animation_playback: None,
        title: document.name.clone(),
        author: document.author_name.clone(),
        canvas_size: CanvasSize {
            width: document.size.width,
            height: document.size.height,
        },
        background_visible: !document.background_hidden,
        background_color: document.background_color,
        flipped: CanvasFlipped {
            horizontally: document.flipped.horizontally,
            vertically: document.flipped.vertically,
        },
        stroke_count: document.stroke_count as u64,
        layer_count: document.layers.iter().map(layer_count).sum(),
        layers,
    };
    animation::initialize_playback(&mut snapshot);
    Ok(snapshot)
}

fn layer_snapshots(nodes: &[SilicaHierarchy]) -> Result<Vec<LayerSnapshot>, RuntimeError> {
    fn append(
        snapshots: &mut Vec<LayerSnapshot>,
        parent_id: Option<LayerId>,
        node: &SilicaHierarchy,
    ) -> Result<(), RuntimeError> {
        match node {
            SilicaHierarchy::Layer(layer) => {
                let layer_id = LayerId::from(layer.hierarchy_id());
                validate_opacity(layer_id, layer.opacity)?;
                validate_animation_hold_duration(layer_id, layer.animation_hold_duration)?;
                snapshots.push(LayerSnapshot {
                    layer_id,
                    parent_id,
                    kind: LayerKind::Layer,
                    name: layer.name.clone(),
                    visible: !layer.hidden,
                    animation_hold_duration: Some(layer.animation_hold_duration),
                    clipped: Some(layer.clipped),
                    blend_mode: Some(layer.blend),
                    opacity: Some(layer.opacity),
                });

                if let Some(mask) = &layer.mask {
                    snapshots.push(LayerSnapshot {
                        layer_id: LayerId::from(mask.hierarchy_id()),
                        parent_id: Some(layer_id),
                        kind: LayerKind::Mask,
                        name: mask.name.clone(),
                        visible: !mask.hidden,
                        animation_hold_duration: None,
                        clipped: None,
                        blend_mode: None,
                        opacity: None,
                    });
                }
            }
            SilicaHierarchy::Group(group) => {
                let layer_id = LayerId::from(group.hierarchy_id());
                validate_animation_hold_duration(layer_id, group.animation_hold_duration)?;
                snapshots.push(LayerSnapshot {
                    layer_id,
                    parent_id,
                    kind: LayerKind::Group,
                    name: group.name.clone(),
                    visible: !group.hidden,
                    animation_hold_duration: Some(group.animation_hold_duration),
                    clipped: None,
                    blend_mode: None,
                    opacity: None,
                });
                for child in &group.children {
                    append(snapshots, Some(layer_id), child)?;
                }
            }
        }
        Ok(())
    }

    let mut snapshots = Vec::new();
    for node in nodes {
        append(&mut snapshots, None, node)?;
    }
    Ok(snapshots)
}

fn validate_opacity(layer_id: LayerId, opacity: f32) -> Result<(), RuntimeError> {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        Ok(())
    } else {
        Err(RuntimeError::InvalidLayerOpacity { layer_id, opacity })
    }
}

fn validate_animation_hold_duration(
    layer_id: LayerId,
    hold_duration: u32,
) -> Result<(), RuntimeError> {
    if hold_duration <= 120 {
        Ok(())
    } else {
        Err(RuntimeError::InvalidAnimationHoldDuration {
            layer_id,
            hold_duration,
        })
    }
}

fn validate_background_color(color: [f32; 4]) -> Result<(), RuntimeError> {
    for (channel, value) in color.into_iter().enumerate() {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(RuntimeError::InvalidBackgroundColor { channel, value });
        }
    }
    Ok(())
}

fn layer_count(node: &SilicaHierarchy) -> u32 {
    match node {
        SilicaHierarchy::Layer(layer) => 1 + u32::from(layer.mask.is_some()),
        SilicaHierarchy::Group(group) => group.children.iter().map(layer_count).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::{Dictionary, Uid, Value};
    use silica::BlendingMode;
    use std::io::{Cursor, Write};

    #[test]
    fn opens_document_and_emits_the_same_snapshot() {
        let mut runtime = DocumentRuntime::new();

        let update = runtime.open(&minimal_procreate_archive()).unwrap();

        assert_eq!(update.value.revision, 0);
        assert_eq!(update.value.title.as_deref(), Some("Runtime fixture"));
        assert_eq!(update.value.author.as_deref(), Some("Rizum"));
        assert_eq!(
            update.value.canvas_size,
            CanvasSize {
                width: 2048,
                height: 1536,
            }
        );
        assert_eq!(update.value.stroke_count, 42);
        assert_eq!(update.value.layer_count, 0);
        assert!(update.value.background_visible);
        assert_eq!(
            update.events,
            vec![RuntimeEvent::DocumentOpened {
                snapshot: update.value.clone(),
            }]
        );
        assert_eq!(
            runtime.snapshot(update.value.document_id).unwrap(),
            update.value
        );
    }

    #[test]
    fn opens_an_already_parsed_document_through_the_same_runtime_seam() {
        let archive = procreate_archive_with_layer();
        let document = ProcreateFile::open(&archive).unwrap();
        let mut runtime = DocumentRuntime::new();

        let update = runtime.open_document(&document).unwrap();
        let document_id = update.value.document_id;
        drop(document);

        assert_eq!(update.value.title.as_deref(), Some("Runtime fixture"));
        assert_eq!(update.value.layers.len(), 1);
        assert_eq!(
            update.events,
            vec![RuntimeEvent::DocumentOpened {
                snapshot: update.value.clone(),
            }]
        );
        assert_eq!(runtime.snapshot(document_id).unwrap(), update.value);
    }

    #[test]
    fn opens_document_with_stable_layer_snapshot() {
        let mut runtime = DocumentRuntime::new();

        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;

        assert_eq!(opened.layer_count, 1);
        assert_eq!(
            opened.layers,
            vec![LayerSnapshot {
                layer_id: LayerId(0),
                parent_id: None,
                kind: LayerKind::Layer,
                name: Some("Line art".to_owned()),
                visible: true,
                animation_hold_duration: Some(0),
                clipped: Some(false),
                blend_mode: Some(BlendingMode::Normal),
                opacity: Some(1.0),
            }]
        );
        assert_eq!(
            runtime.snapshot(opened.document_id).unwrap().layers,
            opened.layers
        );
    }

    #[test]
    fn layer_snapshot_preserves_group_and_mask_parentage() {
        let mut runtime = DocumentRuntime::new();

        let opened = runtime
            .open(&procreate_archive_with_group_and_mask())
            .unwrap()
            .value;

        assert_eq!(opened.layer_count, 2);
        assert_eq!(
            opened.layers,
            vec![
                LayerSnapshot {
                    layer_id: LayerId(0),
                    parent_id: None,
                    kind: LayerKind::Group,
                    name: Some("Sketch".to_owned()),
                    visible: true,
                    animation_hold_duration: Some(0),
                    clipped: None,
                    blend_mode: None,
                    opacity: None,
                },
                LayerSnapshot {
                    layer_id: LayerId(1),
                    parent_id: Some(LayerId(0)),
                    kind: LayerKind::Layer,
                    name: Some("Pencil".to_owned()),
                    visible: true,
                    animation_hold_duration: Some(0),
                    clipped: Some(false),
                    blend_mode: Some(BlendingMode::Normal),
                    opacity: Some(1.0),
                },
                LayerSnapshot {
                    layer_id: LayerId(2),
                    parent_id: Some(LayerId(1)),
                    kind: LayerKind::Mask,
                    name: Some("Pencil mask".to_owned()),
                    visible: false,
                    animation_hold_duration: None,
                    clipped: None,
                    blend_mode: None,
                    opacity: None,
                },
            ]
        );
    }

    #[test]
    fn parsed_hierarchy_assigns_renderer_neutral_ids_in_preorder() {
        let document = ProcreateFile::open(&procreate_archive_with_group_and_mask()).unwrap();
        let SilicaHierarchy::Group(group) = &document.layers[0] else {
            panic!("expected a group at the document root");
        };
        let SilicaHierarchy::Layer(layer) = &group.children[0] else {
            panic!("expected a layer inside the group");
        };
        let mask = layer.mask.as_ref().unwrap();

        assert_eq!(group.hierarchy_id().get(), 0);
        assert_eq!(layer.hierarchy_id().get(), 1);
        assert_eq!(mask.hierarchy_id().get(), 2);
    }

    #[test]
    fn visibility_command_updates_snapshot_and_emits_one_event() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;
        let layer_id = opened.layers[0].layer_id;

        let update = runtime
            .dispatch(DocumentCommand::SetLayerVisibility {
                document_id: opened.document_id,
                layer_id,
                visible: false,
            })
            .unwrap();

        assert_eq!(
            update.events,
            vec![RuntimeEvent::LayerVisibilityChanged {
                document_id: opened.document_id,
                layer_id,
                visible: false,
                revision: 1,
            }]
        );
        let snapshot = runtime.snapshot(opened.document_id).unwrap();
        assert_eq!(snapshot.revision, 1);
        assert!(!snapshot.layers[0].visible);
    }

    #[test]
    fn repeated_visibility_command_is_a_revision_preserving_no_op() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;
        let layer_id = opened.layers[0].layer_id;

        let update = runtime
            .dispatch(DocumentCommand::SetLayerVisibility {
                document_id: opened.document_id,
                layer_id,
                visible: true,
            })
            .unwrap();

        assert!(update.events.is_empty());
        assert_eq!(runtime.snapshot(opened.document_id).unwrap().revision, 0);
    }

    #[test]
    fn clipped_command_updates_layer_snapshot_and_emits_one_event() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;
        let layer_id = opened.layers[0].layer_id;

        let update = runtime
            .dispatch(DocumentCommand::SetLayerClipped {
                document_id: opened.document_id,
                layer_id,
                clipped: true,
            })
            .unwrap();

        assert_eq!(
            update.events,
            vec![RuntimeEvent::LayerClippedChanged {
                document_id: opened.document_id,
                layer_id,
                clipped: true,
                revision: 1,
            }]
        );
        let snapshot = runtime.snapshot(opened.document_id).unwrap();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.layers[0].clipped, Some(true));
    }

    #[test]
    fn repeated_clipped_command_is_a_revision_preserving_no_op() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;
        let layer_id = opened.layers[0].layer_id;

        let update = runtime
            .dispatch(DocumentCommand::SetLayerClipped {
                document_id: opened.document_id,
                layer_id,
                clipped: false,
            })
            .unwrap();

        assert!(update.events.is_empty());
        assert_eq!(runtime.snapshot(opened.document_id).unwrap().revision, 0);
    }

    #[test]
    fn blend_mode_command_updates_layer_snapshot_and_emits_one_event() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;
        let layer_id = opened.layers[0].layer_id;

        let update = runtime
            .dispatch(DocumentCommand::SetLayerBlendMode {
                document_id: opened.document_id,
                layer_id,
                blend_mode: BlendingMode::Multiply,
            })
            .unwrap();

        assert_eq!(
            update.events,
            vec![RuntimeEvent::LayerBlendModeChanged {
                document_id: opened.document_id,
                layer_id,
                blend_mode: BlendingMode::Multiply,
                revision: 1,
            }]
        );
        let snapshot = runtime.snapshot(opened.document_id).unwrap();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.layers[0].blend_mode, Some(BlendingMode::Multiply));
    }

    #[test]
    fn repeated_blend_mode_command_is_a_revision_preserving_no_op() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&procreate_archive_with_layer()).unwrap().value;
        let layer_id = opened.layers[0].layer_id;

        let update = runtime
            .dispatch(DocumentCommand::SetLayerBlendMode {
                document_id: opened.document_id,
                layer_id,
                blend_mode: BlendingMode::Normal,
            })
            .unwrap();

        assert!(update.events.is_empty());
        assert_eq!(runtime.snapshot(opened.document_id).unwrap().revision, 0);
    }

    #[test]
    fn blend_mode_command_rejects_group_and_mask_without_advancing_revision() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime
            .open(&procreate_archive_with_group_and_mask())
            .unwrap()
            .value;

        for layer in opened
            .layers
            .iter()
            .filter(|layer| layer.kind != LayerKind::Layer)
        {
            assert!(matches!(
                runtime.dispatch(DocumentCommand::SetLayerBlendMode {
                    document_id: opened.document_id,
                    layer_id: layer.layer_id,
                    blend_mode: BlendingMode::Multiply,
                }),
                Err(RuntimeError::LayerDoesNotSupportBlendMode {
                    document_id,
                    layer_id,
                }) if document_id == opened.document_id && layer_id == layer.layer_id
            ));
        }

        assert_eq!(runtime.snapshot(opened.document_id).unwrap().revision, 0);
    }

    #[test]
    fn blend_mode_command_has_renderer_independent_transport() {
        let command = DocumentCommand::SetLayerBlendMode {
            document_id: DocumentId(3),
            layer_id: LayerId(7),
            blend_mode: BlendingMode::LinearBurn,
        };

        assert_eq!(
            serde_json::to_string(&command).unwrap(),
            r#"{"SetLayerBlendMode":{"document_id":3,"layer_id":7,"blend_mode":"linear_burn"}}"#
        );
    }

    #[test]
    fn clipped_command_rejects_group_and_mask_without_advancing_revision() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime
            .open(&procreate_archive_with_group_and_mask())
            .unwrap()
            .value;

        for layer in opened
            .layers
            .iter()
            .filter(|layer| layer.kind != LayerKind::Layer)
        {
            assert!(matches!(
                runtime.dispatch(DocumentCommand::SetLayerClipped {
                    document_id: opened.document_id,
                    layer_id: layer.layer_id,
                    clipped: true,
                }),
                Err(RuntimeError::LayerDoesNotSupportClipping {
                    document_id,
                    layer_id,
                }) if document_id == opened.document_id && layer_id == layer.layer_id
            ));
        }

        assert_eq!(runtime.snapshot(opened.document_id).unwrap().revision, 0);
    }

    #[test]
    fn background_visibility_command_updates_snapshot_and_emits_one_event() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&minimal_procreate_archive()).unwrap().value;

        let update = runtime
            .dispatch(DocumentCommand::SetBackgroundVisibility {
                document_id: opened.document_id,
                visible: false,
            })
            .unwrap();

        assert_eq!(
            update.events,
            vec![RuntimeEvent::BackgroundVisibilityChanged {
                document_id: opened.document_id,
                visible: false,
                revision: 1,
            }]
        );
        let snapshot = runtime.snapshot(opened.document_id).unwrap();
        assert_eq!(snapshot.revision, 1);
        assert!(!snapshot.background_visible);
    }

    #[test]
    fn repeated_background_visibility_command_preserves_revision() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&minimal_procreate_archive()).unwrap().value;

        let update = runtime
            .dispatch(DocumentCommand::SetBackgroundVisibility {
                document_id: opened.document_id,
                visible: true,
            })
            .unwrap();

        assert!(update.events.is_empty());
        assert_eq!(runtime.snapshot(opened.document_id).unwrap().revision, 0);
    }

    #[test]
    fn close_command_removes_document_and_emits_one_event() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&minimal_procreate_archive()).unwrap().value;

        let update = runtime
            .dispatch(DocumentCommand::CloseDocument {
                document_id: opened.document_id,
                discard_changes: false,
            })
            .unwrap();

        assert_eq!(update.value, ());
        assert_eq!(
            update.events,
            vec![RuntimeEvent::DocumentClosed {
                document_id: opened.document_id,
                revision: opened.revision,
            }]
        );
        assert!(matches!(
            runtime.snapshot(opened.document_id),
            Err(RuntimeError::DocumentNotFound(document_id))
                if document_id == opened.document_id
        ));
    }

    #[test]
    fn parse_failure_does_not_consume_a_document_id() {
        let mut runtime = DocumentRuntime::new();

        assert!(matches!(
            runtime.open(b"not a Procreate archive"),
            Err(RuntimeError::Parse(_))
        ));

        let opened = runtime.open(&minimal_procreate_archive()).unwrap().value;
        assert_eq!(opened.document_id, DocumentId(0));
    }

    fn minimal_procreate_archive() -> Vec<u8> {
        procreate_archive(Vec::new(), Vec::new())
    }

    fn procreate_archive_with_layer() -> Vec<u8> {
        let mut layer = layer_dictionary("Line art", "line-art-uuid", false);
        layer.insert("$class".into(), Value::Uid(Uid::new(3)));

        procreate_archive(
            vec![Uid::new(2)],
            vec![
                Value::Dictionary(layer),
                Value::Dictionary(class_dictionary("SilicaLayer")),
            ],
        )
    }

    fn procreate_archive_with_group_and_mask() -> Vec<u8> {
        let mut group = Dictionary::new();
        group.insert("$class".into(), Value::Uid(Uid::new(3)));
        group.insert("animationHeldLength".into(), Value::Integer(0_u64.into()));
        group.insert("isHidden".into(), Value::Boolean(false));
        group.insert("name".into(), Value::String("Sketch".into()));
        let mut children = Dictionary::new();
        children.insert(
            "NS.objects".into(),
            Value::Array(vec![Value::Uid(Uid::new(4))]),
        );
        group.insert("children".into(), Value::Dictionary(children));

        let mut layer = layer_dictionary("Pencil", "pencil-uuid", false);
        layer.insert("$class".into(), Value::Uid(Uid::new(5)));
        layer.insert("mask".into(), Value::Uid(Uid::new(6)));
        let mask = layer_dictionary("Pencil mask", "pencil-mask-uuid", true);

        procreate_archive(
            vec![Uid::new(2)],
            vec![
                Value::Dictionary(group),
                Value::Dictionary(class_dictionary("SilicaGroup")),
                Value::Dictionary(layer),
                Value::Dictionary(class_dictionary("SilicaLayer")),
                Value::Dictionary(mask),
            ],
        )
    }

    fn layer_dictionary(name: &str, uuid: &str, hidden: bool) -> Dictionary {
        let mut layer = Dictionary::new();
        layer.insert("UUID".into(), Value::String(uuid.into()));
        layer.insert("animationHeldLength".into(), Value::Integer(0_u64.into()));
        layer.insert("blend".into(), Value::Integer(0_u64.into()));
        layer.insert("clipped".into(), Value::Boolean(false));
        layer.insert("hidden".into(), Value::Boolean(hidden));
        layer.insert("opacity".into(), Value::Real(1.0));
        layer.insert("name".into(), Value::String(name.into()));
        layer.insert("version".into(), Value::Integer(1_u64.into()));
        layer
    }

    fn class_dictionary(name: &str) -> Dictionary {
        let mut class = Dictionary::new();
        class.insert("$classname".into(), Value::String(name.into()));
        class.insert(
            "$classes".into(),
            Value::Array(vec![Value::String(name.into())]),
        );
        class
    }

    fn procreate_archive(layer_ids: Vec<Uid>, extra_objects: Vec<Value>) -> Vec<u8> {
        let mut composite = Dictionary::new();
        composite.insert("UUID".into(), Value::String("composite-uuid".into()));
        composite.insert("animationHeldLength".into(), Value::Integer(0_u64.into()));
        composite.insert("blend".into(), Value::Integer(0_u64.into()));
        composite.insert("clipped".into(), Value::Boolean(false));
        composite.insert("hidden".into(), Value::Boolean(false));
        composite.insert("opacity".into(), Value::Real(1.0));
        composite.insert("name".into(), Value::String("Composite".into()));
        composite.insert("version".into(), Value::Integer(1_u64.into()));

        let mut layers = Dictionary::new();
        layers.insert(
            "NS.objects".into(),
            Value::Array(layer_ids.into_iter().map(Value::Uid).collect()),
        );

        let background_color = [0.1_f32, 0.2, 0.3, 1.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();

        let mut root = Dictionary::new();
        root.insert("authorName".into(), Value::String("Rizum".into()));
        root.insert("backgroundHidden".into(), Value::Boolean(false));
        root.insert("backgroundColor".into(), Value::Data(background_color));
        root.insert("composite".into(), Value::Dictionary(composite));
        root.insert("flippedHorizontally".into(), Value::Boolean(false));
        root.insert("flippedVertically".into(), Value::Boolean(false));
        root.insert("name".into(), Value::String("Runtime fixture".into()));
        root.insert("orientation".into(), Value::Integer(1_u64.into()));
        root.insert("size".into(), Value::String("{2048, 1536}".into()));
        root.insert("strokeCount".into(), Value::Integer(42_u64.into()));
        root.insert("tileSize".into(), Value::Integer(256_u64.into()));
        root.insert("unwrappedLayers".into(), Value::Dictionary(layers));

        let mut top = Dictionary::new();
        top.insert("root".into(), Value::Uid(Uid::new(1)));

        let mut keyed_archive = Dictionary::new();
        keyed_archive.insert("$archiver".into(), Value::String("NSKeyedArchiver".into()));
        let mut objects = vec![Value::String("$null".into()), Value::Dictionary(root)];
        objects.extend(extra_objects);
        keyed_archive.insert("$objects".into(), Value::Array(objects));
        keyed_archive.insert("$top".into(), Value::Dictionary(top));
        keyed_archive.insert("$version".into(), Value::Integer(100_000_u64.into()));

        let mut document = Vec::new();
        Value::Dictionary(keyed_archive)
            .to_writer_binary(&mut document)
            .unwrap();

        let cursor = Cursor::new(Vec::new());
        let mut archive = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        archive.start_file("Document.archive", options).unwrap();
        archive.write_all(&document).unwrap();
        archive.finish().unwrap().into_inner()
    }
}
