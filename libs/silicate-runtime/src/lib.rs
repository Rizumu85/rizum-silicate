use serde::{Deserialize, Serialize};
use silica::{ProcreateFile, SilicaHierarchy};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(u64);

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCommand {
    CloseDocument { document_id: DocumentId },
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
}

struct DocumentRecord {
    document: ProcreateFile,
    revision: u64,
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
        let document_id = DocumentId(self.next_document_id);
        self.next_document_id = self
            .next_document_id
            .checked_add(1)
            .ok_or(RuntimeError::DocumentIdExhausted)?;

        let record = DocumentRecord {
            document,
            revision: 0,
        };
        let snapshot = snapshot(document_id, &record);
        self.documents.insert(document_id, record);

        Ok(RuntimeUpdate {
            value: snapshot.clone(),
            events: vec![RuntimeEvent::DocumentOpened { snapshot }],
        })
    }

    pub fn snapshot(&self, document_id: DocumentId) -> Result<DocumentSnapshot, RuntimeError> {
        self.documents
            .get(&document_id)
            .map(|record| snapshot(document_id, record))
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
                        revision: record.revision,
                    }],
                })
            }
        }
    }
}

fn snapshot(document_id: DocumentId, record: &DocumentRecord) -> DocumentSnapshot {
    DocumentSnapshot {
        document_id,
        revision: record.revision,
        title: record.document.name.clone(),
        author: record.document.author_name.clone(),
        canvas_size: CanvasSize {
            width: record.document.size.width,
            height: record.document.size.height,
        },
        stroke_count: record.document.stroke_count as u64,
        layer_count: record.document.layers.iter().map(layer_count).sum(),
    }
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
        let mut composite = Dictionary::new();
        composite.insert("UUID".into(), Value::String("composite-uuid".into()));
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
