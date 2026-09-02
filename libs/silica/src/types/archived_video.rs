use crate::{
    data::Orientation,
    ns_archive::{NsRefDictionary, Size, error::NsArchiveError},
};
use plist::Dictionary;

#[derive(Debug, Clone, PartialEq)]
pub struct ArchivedVideoMetadata {
    pub recording_enabled: bool,
    pub purged: bool,
    pub segment_ordinal: Option<u32>,
    pub encoding: ArchivedVideoEncoding,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchivedVideoEncoding {
    pub frame_size: Size<u32>,
    pub frames_per_second: u32,
    pub bitrate: f64,
    /// Encoding identifiers stay raw until controlled Procreate fixtures establish
    /// stable codec and color-space mappings across archive versions.
    pub codec_raw: u64,
    pub codec_2020_raw: Option<u64>,
    pub color_space_raw: u64,
    pub quality_preference: String,
    pub resolution_preference: String,
    pub source_orientation: Orientation,
}

impl ArchivedVideoMetadata {
    pub(crate) fn from_document<'a>(
        document: &'a NsRefDictionary<'a>,
    ) -> Result<Option<Self>, NsArchiveError> {
        let Some(segment_info) =
            document.resolve::<Option<&Dictionary>>("SilicaDocumentVideoSegmentInfoKey")?
        else {
            return Ok(None);
        };
        let segment_info = document.archive().bind(segment_info);

        Ok(Some(Self {
            recording_enabled: document.resolve::<bool>("videoEnabled")?,
            purged: document.resolve::<bool>("SilicaDocumentVideoPurgedKey")?,
            segment_ordinal: document.resolve::<Option<u32>>("videoSegmentOrdinal")?,
            encoding: ArchivedVideoEncoding {
                frame_size: segment_info.resolve::<Size<u32>>("frameSize")?,
                frames_per_second: segment_info.resolve::<u32>("framesPerSecond")?,
                bitrate: segment_info.resolve::<f64>("bitrate")?,
                codec_raw: segment_info.resolve::<u64>("codec")?,
                codec_2020_raw: segment_info.resolve::<Option<u64>>("codec2020")?,
                color_space_raw: segment_info.resolve::<u64>("colorSpace")?,
                quality_preference: segment_info.resolve::<String>("qualityPreferenceKey")?,
                resolution_preference: segment_info.resolve::<String>("resolutionPreferenceKey")?,
                source_orientation: segment_info.resolve::<Orientation>("sourceOrientation")?,
            },
        }))
    }
}
