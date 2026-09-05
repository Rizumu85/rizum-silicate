use crate::{
    ns_archive::{NsRefDictionary, error::NsArchiveError},
    types::hierarchy::HierarchyId,
};
use plist::Dictionary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationAssistMode {
    Disabled,
    Enabled,
    Unknown(u64),
}

impl AnimationAssistMode {
    pub const fn from_raw(raw: u64) -> Self {
        match raw {
            0 => Self::Disabled,
            1 => Self::Enabled,
            value => Self::Unknown(value),
        }
    }

    pub const fn raw(self) -> u64 {
        match self {
            Self::Disabled => 0,
            Self::Enabled => 1,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationPlaybackMode(u64);

impl AnimationPlaybackMode {
    /// User-confirmed Loop and Ping Pong documents both store value 1, so this field
    /// remains lossless metadata rather than pretending to identify playback behavior.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationPlaybackDirection(u64);

impl AnimationPlaybackDirection {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentAnimation {
    pub assist_mode: Option<AnimationAssistMode>,
    pub frame_rate: u32,
    pub playback_mode: AnimationPlaybackMode,
    pub playback_direction: AnimationPlaybackDirection,
    pub onion_skin_count: u32,
    pub onion_skin_opacity: f32,
    pub blend_primary_frame: bool,
    pub first_item_is_foreground: bool,
    pub last_item_is_background: bool,
}

impl DocumentAnimation {
    pub(crate) fn from_document<'a>(
        document: &'a NsRefDictionary<'a>,
    ) -> Result<Option<Self>, NsArchiveError> {
        let Some(settings) = document.resolve::<Option<&Dictionary>>("animation")? else {
            return Ok(None);
        };
        let settings = document.archive().bind(settings);

        Ok(Some(Self {
            assist_mode: settings
                .resolve::<Option<u64>>("animationMode")?
                .map(AnimationAssistMode::from_raw),
            frame_rate: settings.resolve::<u32>("frameRate")?,
            playback_mode: AnimationPlaybackMode::from_raw(
                settings.resolve::<u64>("playbackMode")?,
            ),
            playback_direction: AnimationPlaybackDirection(
                settings.resolve::<u64>("playbackDirection")?,
            ),
            onion_skin_count: settings.resolve::<u32>("onionSkinCount")?,
            onion_skin_opacity: settings.resolve::<f32>("onionSkinOpacity")?,
            blend_primary_frame: settings.resolve::<bool>("primaryMixed")?,
            first_item_is_foreground: document.resolve::<bool>("isFirstItemAnimationForeground")?,
            last_item_is_background: document.resolve::<bool>("isLastItemAnimationBackground")?,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationFrameSource {
    pub hierarchy_id: HierarchyId,
    pub hold_duration: u32,
}
