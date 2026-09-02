use crate::{
    data::{Flipped, Orientation},
    error::SilicaError,
    limits::{MAX_DOCUMENT_ARCHIVE_BYTES, read_bounded},
    ns_archive::{NsArchive, NsObjects, Size, error::NsArchiveError},
    types::{
        animation::{AnimationFrameSource, DocumentAnimation},
        hierarchy::{HierarchyId, SilicaHierarchy},
        layer::SilicaLayer,
    },
};
use std::io::Cursor;
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcreateFile {
    pub animation: Option<DocumentAnimation>,
    pub author_name: Option<String>,
    pub background_hidden: bool,
    pub background_color: [f32; 4],
    //     closedCleanlyKey:Bool?
    //     colorProfile:ValkyrieColorProfile?

    // //  public var drawingguide
    //     faceBackgroundHidden:Bool?
    //     1 => BlendingMode::featureSet:Int?
    pub flipped: Flipped,
    //     mask:SilicaLayer?
    pub name: Option<String>,
    pub orientation: Orientation,
    //     primaryItem:Any?
    // //  skipping a bunch of reference window related stuff here
    //     selectedLayer:Any?
    //     selectedSamplerLayer:SilicaLayer?
    //     SilicaDocumentArchiveDPIKey:Float?
    //     SilicaDocumentArchiveUnitKey:Int?
    //     SilicaDocumentTrackedTimeKey:Float?
    //     SilicaDocumentVideoPurgedKey:Bool?
    //     SilicaDocumentVideoSegmentInfoKey:VideoSegmentInfo? // not finished
    //     size: CGSize?
    //     solo: SilicaLayer?
    pub stroke_count: usize,
    //     videoEnabled: Bool? = true
    //     videoQualityKey: String?
    //     videoResolutionKey: String?
    //     videoDuration: String? = "Calculating..."
    pub tile_size: u32,

    pub size: Size<u32>,

    pub layers: Vec<SilicaHierarchy>,
    pub composite: Option<SilicaLayer>,
}

impl ProcreateFile {
    /// Parses document metadata and hierarchy without creating GPU resources.
    pub fn open(bytes: &[u8]) -> Result<Self, SilicaError> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))?;
        let mut document = archive.by_name("Document.archive")?;
        let document_size = document.size();
        let document_bytes = read_bounded(
            &mut document,
            document_size,
            "Document.archive",
            MAX_DOCUMENT_ARCHIVE_BYTES,
        )?;

        let keyed_archive = NsArchive::from_reader(Cursor::new(document_bytes))?;
        Self::from_ns(&keyed_archive)
    }

    pub fn from_ns(nka: &NsArchive) -> Result<Self, SilicaError> {
        let refs = nka.bind(nka.root()?);

        let size = refs.resolve::<Size<u32>>("size")?;
        let tile_size = refs.resolve::<u32>("tileSize")?;

        let mut layers = refs
            .resolve::<NsObjects<SilicaHierarchy>>("unwrappedLayers")?
            .objects;
        let mut composite = Some(refs.resolve::<SilicaLayer>("composite")?);
        let mut next_hierarchy_id = 0;
        assign_hierarchy_ids(&mut layers, &mut next_hierarchy_id);
        if let Some(composite) = &mut composite {
            assign_layer_id(composite, &mut next_hierarchy_id);
        }

        Ok(Self {
            animation: DocumentAnimation::from_document(&refs)?,
            author_name: refs.resolve::<Option<String>>("authorName")?,
            background_hidden: refs.resolve::<bool>("backgroundHidden")?,
            stroke_count: refs.resolve::<usize>("strokeCount")?,
            background_color: decode_background_color(refs.resolve::<&[u8]>("backgroundColor")?)?,
            name: refs.resolve::<Option<String>>("name")?,
            orientation: refs.resolve::<Orientation>("orientation")?,
            flipped: Flipped {
                horizontally: refs.resolve::<bool>("flippedHorizontally")?,
                vertically: refs.resolve::<bool>("flippedVertically")?,
            },
            tile_size,
            composite,
            layers,
            size,
        })
    }

    /// Returns visible top-level layers and groups in Procreate timeline order.
    pub fn animation_frame_sources(&self) -> impl Iterator<Item = AnimationFrameSource> + '_ {
        self.layers.iter().rev().filter_map(|hierarchy| {
            let (hierarchy_id, hold_duration, hidden) = match hierarchy {
                SilicaHierarchy::Layer(layer) => (
                    layer.hierarchy_id(),
                    layer.animation_hold_duration,
                    layer.hidden,
                ),
                SilicaHierarchy::Group(group) => (
                    group.hierarchy_id(),
                    group.animation_hold_duration,
                    group.hidden,
                ),
            };

            (!hidden).then_some(AnimationFrameSource {
                hierarchy_id,
                hold_duration,
            })
        })
    }
}

fn decode_background_color(bytes: &[u8]) -> Result<[f32; 4], NsArchiveError> {
    let bytes: &[u8; 16] = bytes
        .try_into()
        .map_err(|_| NsArchiveError::TypeMismatch("backgroundColor".to_string()))?;
    Ok(std::array::from_fn(|index| {
        let offset = index * 4;
        f32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }))
}

fn assign_hierarchy_ids(layers: &mut [SilicaHierarchy], next_id: &mut u64) {
    for hierarchy in layers {
        match hierarchy {
            SilicaHierarchy::Layer(layer) => assign_layer_id(layer, next_id),
            SilicaHierarchy::Group(group) => {
                group.set_hierarchy_id(HierarchyId::new(*next_id));
                *next_id += 1;
                assign_hierarchy_ids(&mut group.children, next_id);
            }
        }
    }
}

fn assign_layer_id(layer: &mut SilicaLayer, next_id: &mut u64) {
    layer.set_hierarchy_id(HierarchyId::new(*next_id));
    *next_id += 1;
    if let Some(mask) = &mut layer.mask {
        assign_layer_id(mask, next_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plist::{Dictionary, Uid, Value};
    use std::io::{Cursor, Write};

    #[test]
    fn opens_document_metadata_without_a_gpu() {
        let archive = minimal_procreate_archive();

        let file = ProcreateFile::open(&archive).unwrap();

        assert_eq!(file.name.as_deref(), Some("Runtime fixture"));
        assert_eq!(file.author_name.as_deref(), Some("Rizum"));
        assert_eq!(
            file.size,
            Size {
                width: 2048,
                height: 1536
            }
        );
        assert_eq!(file.tile_size, 256);
        assert_eq!(file.orientation, Orientation::Clockwise90);
        assert_eq!(
            file.flipped,
            Flipped {
                horizontally: true,
                vertically: false
            }
        );
        assert_eq!(file.stroke_count, 42);
        assert_eq!(file.background_color, [0.1, 0.2, 0.3, 1.0]);
        assert!(file.layers.is_empty());
        assert_eq!(
            file.composite
                .as_ref()
                .and_then(|layer| layer.name.as_deref()),
            Some("Composite")
        );
        assert_eq!(
            file.composite.as_ref().unwrap().hierarchy_id(),
            HierarchyId::new(0)
        );
    }

    fn minimal_procreate_archive() -> Vec<u8> {
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
        layers.insert("NS.objects".into(), Value::Array(Vec::new()));

        let background_color = [0.1_f32, 0.2, 0.3, 1.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();

        let mut root = Dictionary::new();
        root.insert("authorName".into(), Value::String("Rizum".into()));
        root.insert("backgroundHidden".into(), Value::Boolean(false));
        root.insert("backgroundColor".into(), Value::Data(background_color));
        root.insert("composite".into(), Value::Dictionary(composite));
        root.insert("flippedHorizontally".into(), Value::Boolean(true));
        root.insert("flippedVertically".into(), Value::Boolean(false));
        root.insert("name".into(), Value::String("Runtime fixture".into()));
        root.insert("orientation".into(), Value::Integer(4_u64.into()));
        root.insert("size".into(), Value::String("{2048, 1536}".into()));
        root.insert("strokeCount".into(), Value::Integer(42_u64.into()));
        root.insert("tileSize".into(), Value::Integer(256_u64.into()));
        root.insert("unwrappedLayers".into(), Value::Dictionary(layers));

        let mut top = Dictionary::new();
        top.insert("root".into(), Value::Uid(Uid::new(1)));

        let mut keyed_archive = Dictionary::new();
        keyed_archive.insert("$archiver".into(), Value::String("NSKeyedArchiver".into()));
        keyed_archive.insert(
            "$objects".into(),
            Value::Array(vec![Value::String("$null".into()), Value::Dictionary(root)]),
        );
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
