use crate::{DocumentSnapshot, LayerId, RuntimeError};
use serde::{Deserialize, Serialize};
use silica::ProcreateFile;
use std::time::Duration;

const FRAME_UNITS_PER_SECOND: u128 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimationSnapshot {
    pub assist_mode_raw: Option<u64>,
    pub frame_rate: u32,
    pub playback_mode_raw: u64,
    pub playback_direction_raw: u64,
    pub onion_skin_count: u32,
    pub onion_skin_opacity: f32,
    pub blend_primary_frame: bool,
    pub first_item_is_foreground: bool,
    pub last_item_is_background: bool,
}

impl AnimationSnapshot {
    pub const fn assist_enabled(self) -> Option<bool> {
        match self.assist_mode_raw {
            Some(0) => Some(false),
            Some(1) => Some(true),
            Some(_) | None => None,
        }
    }

    pub const fn onion_skin_settings(self) -> AnimationOnionSkinSettings {
        AnimationOnionSkinSettings {
            frame_count: self.onion_skin_count,
            opacity: self.onion_skin_opacity,
            blend_primary_frame: self.blend_primary_frame,
        }
    }

    pub(crate) fn set_onion_skin_settings(&mut self, settings: AnimationOnionSkinSettings) {
        self.onion_skin_count = settings.frame_count;
        self.onion_skin_opacity = settings.opacity;
        self.blend_primary_frame = settings.blend_primary_frame;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimationOnionSkinSettings {
    pub frame_count: u32,
    pub opacity: f32,
    pub blend_primary_frame: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationOnionSkinDirection {
    Behind,
    Ahead,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimationOnionSkinFrame {
    pub source_layer_id: LayerId,
    pub direction: AnimationOnionSkinDirection,
    pub distance: u32,
    pub opacity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationPlaybackMode {
    Loop,
    PingPong,
    OneShot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationPlaybackDirection {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationPlaybackSnapshot {
    pub active: bool,
    pub playing: bool,
    pub mode: AnimationPlaybackMode,
    pub direction: AnimationPlaybackDirection,
    pub slot_index: u64,
    pub slot_count: u64,
    pub source_layer_id: Option<LayerId>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlaybackAnchor {
    source_layer_id: Option<LayerId>,
    offset_within_hold: u64,
    slot_index: u64,
}

pub(crate) fn project(document: &ProcreateFile) -> Result<Option<AnimationSnapshot>, RuntimeError> {
    let Some(animation) = &document.animation else {
        return Ok(None);
    };

    if !(1..=60).contains(&animation.frame_rate) {
        return Err(RuntimeError::InvalidAnimationFrameRate(
            animation.frame_rate,
        ));
    }
    if animation.onion_skin_count > 12 {
        return Err(RuntimeError::InvalidAnimationOnionSkinCount(
            animation.onion_skin_count,
        ));
    }
    if !animation.onion_skin_opacity.is_finite()
        || !(0.0..=1.0).contains(&animation.onion_skin_opacity)
    {
        return Err(RuntimeError::InvalidAnimationOnionSkinOpacity(
            animation.onion_skin_opacity,
        ));
    }

    Ok(Some(AnimationSnapshot {
        assist_mode_raw: animation.assist_mode.map(|mode| mode.raw()),
        frame_rate: animation.frame_rate,
        playback_mode_raw: animation.playback_mode.raw(),
        playback_direction_raw: animation.playback_direction.raw(),
        onion_skin_count: animation.onion_skin_count,
        onion_skin_opacity: animation.onion_skin_opacity,
        blend_primary_frame: animation.blend_primary_frame,
        first_item_is_foreground: animation.first_item_is_foreground,
        last_item_is_background: animation.last_item_is_background,
    }))
}

pub(crate) fn initialize_playback(snapshot: &mut DocumentSnapshot) {
    if snapshot.animation.is_none() {
        snapshot.animation_playback = None;
        return;
    }

    let slot_count = snapshot.animation_timeline_slot_count();
    snapshot.animation_playback = Some(AnimationPlaybackSnapshot {
        active: snapshot
            .animation
            .is_some_and(|animation| animation.assist_enabled() == Some(true)),
        playing: false,
        // Procreate writes the same raw value for user-confirmed Loop and Ping Pong
        // documents, so runtime behavior must remain explicit instead of inferred.
        mode: AnimationPlaybackMode::Loop,
        direction: AnimationPlaybackDirection::Forward,
        slot_index: 0,
        slot_count,
        source_layer_id: snapshot.animation_timeline_slot(0),
    });
}

pub(crate) fn playback_anchor(snapshot: &DocumentSnapshot) -> Option<PlaybackAnchor> {
    let playback = snapshot.animation_playback?;
    let first_slot = playback
        .source_layer_id
        .and_then(|layer_id| first_slot_for_layer(snapshot, layer_id));

    Some(PlaybackAnchor {
        source_layer_id: playback.source_layer_id,
        offset_within_hold: first_slot
            .and_then(|first| playback.slot_index.checked_sub(first))
            .unwrap_or(0),
        slot_index: playback.slot_index,
    })
}

pub(crate) fn reconcile_playback(
    snapshot: &mut DocumentSnapshot,
    anchor: Option<PlaybackAnchor>,
) -> bool {
    let Some(before) = snapshot.animation_playback else {
        return false;
    };
    let slot_count = snapshot.animation_timeline_slot_count();
    let anchor = anchor.unwrap_or(PlaybackAnchor {
        source_layer_id: before.source_layer_id,
        offset_within_hold: 0,
        slot_index: before.slot_index,
    });

    let slot_index = if slot_count == 0 {
        0
    } else if let Some((first_slot, hold_duration)) = anchor
        .source_layer_id
        .and_then(|layer_id| frame_location(snapshot, layer_id))
    {
        first_slot + anchor.offset_within_hold.min(u64::from(hold_duration))
    } else {
        anchor.slot_index.min(slot_count - 1)
    };

    let after = AnimationPlaybackSnapshot {
        playing: before.playing && slot_count > 0,
        slot_count,
        slot_index,
        source_layer_id: snapshot.animation_timeline_slot(slot_index),
        ..before
    };
    snapshot.animation_playback = Some(after);
    after != before
}

pub(crate) fn set_playing(
    snapshot: &mut DocumentSnapshot,
    frame_units: &mut u64,
    playing: bool,
) -> Result<(), RuntimeError> {
    let document_id = snapshot.document_id;
    let Some(mut playback) = snapshot.animation_playback else {
        return Err(RuntimeError::AnimationUnavailable(document_id));
    };
    if playback.playing == playing {
        return Ok(());
    }
    if playing && playback.slot_count == 0 {
        return Err(RuntimeError::AnimationHasNoFrames(document_id));
    }

    if playing && playback.mode == AnimationPlaybackMode::OneShot {
        let terminal_slot = match playback.direction {
            AnimationPlaybackDirection::Forward => playback.slot_count - 1,
            AnimationPlaybackDirection::Reverse => 0,
        };
        if playback.slot_index == terminal_slot {
            playback.slot_index = match playback.direction {
                AnimationPlaybackDirection::Forward => 0,
                AnimationPlaybackDirection::Reverse => playback.slot_count - 1,
            };
            playback.source_layer_id = snapshot.animation_timeline_slot(playback.slot_index);
            *frame_units = 0;
        }
    }

    if playing {
        playback.active = true;
    }
    playback.playing = playing;
    snapshot.animation_playback = Some(playback);
    Ok(())
}

pub(crate) fn set_active(
    snapshot: &mut DocumentSnapshot,
    frame_units: &mut u64,
    active: bool,
) -> Result<(), RuntimeError> {
    let document_id = snapshot.document_id;
    let playback = snapshot
        .animation_playback
        .as_mut()
        .ok_or(RuntimeError::AnimationUnavailable(document_id))?;
    if playback.active != active {
        playback.active = active;
        if !active {
            playback.playing = false;
        }
        *frame_units = 0;
    }
    Ok(())
}

pub(crate) fn set_mode(
    snapshot: &mut DocumentSnapshot,
    frame_units: &mut u64,
    mode: AnimationPlaybackMode,
) -> Result<(), RuntimeError> {
    let document_id = snapshot.document_id;
    let playback = snapshot
        .animation_playback
        .as_mut()
        .ok_or(RuntimeError::AnimationUnavailable(document_id))?;
    if playback.mode != mode {
        playback.mode = mode;
        *frame_units = 0;
    }
    Ok(())
}

pub(crate) fn set_direction(
    snapshot: &mut DocumentSnapshot,
    frame_units: &mut u64,
    direction: AnimationPlaybackDirection,
) -> Result<(), RuntimeError> {
    let document_id = snapshot.document_id;
    let playback = snapshot
        .animation_playback
        .as_mut()
        .ok_or(RuntimeError::AnimationUnavailable(document_id))?;
    if playback.direction != direction {
        playback.direction = direction;
        *frame_units = 0;
    }
    Ok(())
}

pub(crate) fn seek(
    snapshot: &mut DocumentSnapshot,
    frame_units: &mut u64,
    slot_index: u64,
) -> Result<(), RuntimeError> {
    let document_id = snapshot.document_id;
    let Some(mut playback) = snapshot.animation_playback else {
        return Err(RuntimeError::AnimationUnavailable(document_id));
    };
    if slot_index >= playback.slot_count {
        return Err(RuntimeError::AnimationSlotOutOfRange {
            document_id,
            slot_index,
            slot_count: playback.slot_count,
        });
    }

    playback.slot_index = slot_index;
    playback.source_layer_id = snapshot.animation_timeline_slot(slot_index);
    playback.active = true;
    snapshot.animation_playback = Some(playback);
    *frame_units = 0;
    Ok(())
}

pub(crate) fn advance(
    snapshot: &mut DocumentSnapshot,
    frame_units: &mut u64,
    elapsed: Duration,
) -> Result<(), RuntimeError> {
    let Some(before) = snapshot.animation_playback else {
        return Err(RuntimeError::AnimationUnavailable(snapshot.document_id));
    };
    if !before.playing || elapsed.is_zero() {
        return Ok(());
    }

    let frame_rate = snapshot
        .animation
        .as_ref()
        .expect("playback exists only for animation documents")
        .frame_rate;
    let elapsed_units = elapsed
        .as_nanos()
        .saturating_mul(u128::from(frame_rate))
        .saturating_add(u128::from(*frame_units));
    let steps = elapsed_units / FRAME_UNITS_PER_SECOND;
    *frame_units = (elapsed_units % FRAME_UNITS_PER_SECOND) as u64;
    if steps == 0 {
        return Ok(());
    }

    let mut after = before;
    match after.mode {
        AnimationPlaybackMode::Loop => advance_loop(&mut after, steps),
        AnimationPlaybackMode::PingPong => advance_ping_pong(&mut after, steps),
        AnimationPlaybackMode::OneShot => advance_one_shot(&mut after, steps),
    }
    after.source_layer_id = snapshot.animation_timeline_slot(after.slot_index);
    snapshot.animation_playback = Some(after);
    Ok(())
}

fn advance_loop(playback: &mut AnimationPlaybackSnapshot, steps: u128) {
    if playback.slot_count <= 1 {
        return;
    }
    let slot_count = u128::from(playback.slot_count);
    let offset = steps % slot_count;
    playback.slot_index = match playback.direction {
        AnimationPlaybackDirection::Forward => {
            ((u128::from(playback.slot_index) + offset) % slot_count) as u64
        }
        AnimationPlaybackDirection::Reverse => {
            ((u128::from(playback.slot_index) + slot_count - offset) % slot_count) as u64
        }
    };
}

fn advance_ping_pong(playback: &mut AnimationPlaybackSnapshot, steps: u128) {
    if playback.slot_count <= 1 {
        return;
    }

    let last_slot = playback.slot_count - 1;
    let period = u128::from(last_slot) * 2;
    let phase = match playback.direction {
        AnimationPlaybackDirection::Forward => u128::from(playback.slot_index),
        AnimationPlaybackDirection::Reverse => period - u128::from(playback.slot_index),
    };
    let phase = (phase + steps) % period;
    if phase == 0 {
        playback.slot_index = 0;
        playback.direction = AnimationPlaybackDirection::Forward;
    } else if phase < u128::from(last_slot) {
        playback.slot_index = phase as u64;
        playback.direction = AnimationPlaybackDirection::Forward;
    } else {
        playback.slot_index = (period - phase) as u64;
        playback.direction = AnimationPlaybackDirection::Reverse;
    }
}

fn advance_one_shot(playback: &mut AnimationPlaybackSnapshot, steps: u128) {
    let distance = match playback.direction {
        AnimationPlaybackDirection::Forward => playback.slot_count - 1 - playback.slot_index,
        AnimationPlaybackDirection::Reverse => playback.slot_index,
    };
    if steps > u128::from(distance) {
        playback.slot_index = match playback.direction {
            AnimationPlaybackDirection::Forward => playback.slot_count - 1,
            AnimationPlaybackDirection::Reverse => 0,
        };
        playback.playing = false;
        return;
    }

    let steps = steps as u64;
    match playback.direction {
        AnimationPlaybackDirection::Forward => playback.slot_index += steps,
        AnimationPlaybackDirection::Reverse => playback.slot_index -= steps,
    }
}

fn first_slot_for_layer(snapshot: &DocumentSnapshot, layer_id: LayerId) -> Option<u64> {
    frame_location(snapshot, layer_id).map(|(slot, _)| slot)
}

fn frame_location(snapshot: &DocumentSnapshot, layer_id: LayerId) -> Option<(u64, u32)> {
    let mut first_slot = 0_u64;
    for frame in snapshot.animation_frame_sources() {
        if frame.source_layer_id == layer_id {
            return Some((first_slot, frame.hold_duration));
        }
        first_slot += u64::from(frame.hold_duration) + 1;
    }
    None
}
