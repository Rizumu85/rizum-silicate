use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use silica::ProcreateFile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub const fn assist_enabled(&self) -> Option<bool> {
        match self.assist_mode_raw {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        }
    }
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
