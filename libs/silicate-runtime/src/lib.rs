use serde::{Deserialize, Serialize};
use silica::{ProcreateFile, SilicaHierarchy};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LayerId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerKind {
    Layer,
    Group,
    Mask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerSnapshot {
    pub layer_id: LayerId,
    pub parent_id: Option<LayerId>,
    pub kind: LayerKind,
    pub name: Option<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub document_id: DocumentId,
    pub revision: u64,
    pub title: Option<String>,
    pub author: Option<String>,
    pub canvas_size: CanvasSize,
    pub stroke_count: u64,
    pub layer_count: u32,
    pub layers: Vec<LayerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCommand {
    CloseDocument {
        document_id: DocumentId,
    },
    SetLayerVisibility {
        document_id: DocumentId,
        layer_id: LayerId,
        visible: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEvent {
    DocumentOpened {
        snapshot: DocumentSnapshot,
    },
    DocumentClosed {
        document_id: DocumentId,
        revision: u64,
    },
    LayerVisibilityChanged {
        document_id: DocumentId,
        layer_id: LayerId,
        visible: bool,
        revision: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    #[error("revision space is exhausted for document {0:?}")]
    RevisionExhausted(DocumentId),
}

struct DocumentRecord {
    snapshot: DocumentSnapshot,
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
        self.next_document_id = self
            .next_document_id
            .checked_add(1)
            .ok_or(RuntimeError::DocumentIdExhausted)?;

        let record = DocumentRecord {
            snapshot: snapshot(document_id, document),
        };
        let snapshot = record.snapshot.clone();
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

    pub fn dispatch(
        &mut self,
        command: DocumentCommand,
    ) -> Result<RuntimeUpdate<()>, RuntimeError> {
        match command {
            DocumentCommand::CloseDocument { document_id } => {
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
            DocumentCommand::SetLayerVisibility {
                document_id,
                layer_id,
                visible,
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

                Ok(RuntimeUpdate {
                    value: (),
                    events: vec![RuntimeEvent::LayerVisibilityChanged {
                        document_id,
                        layer_id,
                        visible,
                        revision,
                    }],
                })
            }
        }
    }
}

fn snapshot(document_id: DocumentId, document: &ProcreateFile) -> DocumentSnapshot {
    let layers = layer_snapshots(&document.layers);
    DocumentSnapshot {
        document_id,
        revision: 0,
        title: document.name.clone(),
        author: document.author_name.clone(),
        canvas_size: CanvasSize {
            width: document.size.width,
            height: document.size.height,
        },
        stroke_count: document.stroke_count as u64,
        layer_count: document.layers.iter().map(layer_count).sum(),
        layers,
    }
}

fn layer_snapshots(nodes: &[SilicaHierarchy]) -> Vec<LayerSnapshot> {
    fn append(
        snapshots: &mut Vec<LayerSnapshot>,
        next_id: &mut u64,
        parent_id: Option<LayerId>,
        node: &SilicaHierarchy,
    ) {
        let layer_id = LayerId(*next_id);
        *next_id += 1;

        match node {
            SilicaHierarchy::Layer(layer) => {
                snapshots.push(LayerSnapshot {
                    layer_id,
                    parent_id,
                    kind: LayerKind::Layer,
                    name: layer.name.clone(),
                    visible: !layer.hidden,
                });

                if let Some(mask) = &layer.mask {
                    let mask_id = LayerId(*next_id);
                    *next_id += 1;
                    snapshots.push(LayerSnapshot {
                        layer_id: mask_id,
                        parent_id: Some(layer_id),
                        kind: LayerKind::Mask,
                        name: mask.name.clone(),
                        visible: !mask.hidden,
                    });
                }
            }
            SilicaHierarchy::Group(group) => {
                snapshots.push(LayerSnapshot {
                    layer_id,
                    parent_id,
                    kind: LayerKind::Group,
                    name: group.name.clone(),
                    visible: !group.hidden,
                });
                for child in &group.children {
                    append(snapshots, next_id, Some(layer_id), child);
                }
            }
        }
    }

    let mut snapshots = Vec::new();
    let mut next_id = 0;
    for node in nodes {
        append(&mut snapshots, &mut next_id, None, node);
    }
    snapshots
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
                },
                LayerSnapshot {
                    layer_id: LayerId(1),
                    parent_id: Some(LayerId(0)),
                    kind: LayerKind::Layer,
                    name: Some("Pencil".to_owned()),
                    visible: true,
                },
                LayerSnapshot {
                    layer_id: LayerId(2),
                    parent_id: Some(LayerId(1)),
                    kind: LayerKind::Mask,
                    name: Some("Pencil mask".to_owned()),
                    visible: false,
                },
            ]
        );
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
    fn close_command_removes_document_and_emits_one_event() {
        let mut runtime = DocumentRuntime::new();
        let opened = runtime.open(&minimal_procreate_archive()).unwrap().value;

        let update = runtime
            .dispatch(DocumentCommand::CloseDocument {
                document_id: opened.document_id,
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
