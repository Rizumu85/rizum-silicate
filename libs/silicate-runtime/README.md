# Silicate Runtime

`silicate-runtime` is the UI-independent document interface shared by current
and future presentation adapters. It owns parsed Procreate document state and
exposes serializable commands, immutable snapshots, and bounded events.

Current vertical slice:

- open a Procreate archive from bytes;
- ingest an already parsed `silica::ProcreateFile` so production parses
  `Document.archive` only once;
- return a stable `DocumentId` and metadata snapshot;
- expose ordered layer, group, and mask snapshots with parser-assigned,
  renderer-neutral `LayerId` values and explicit parent relationships;
- expose background visibility in the document snapshot;
- emit the matching `DocumentOpened` event in the operation result;
- dispatch idempotent `SetLayerVisibility` commands and emit revisioned changes;
- dispatch idempotent `SetBackgroundVisibility` commands and emit revisioned
  changes;
- expose clipped state only for ordinary layers, dispatch idempotent
  `SetLayerClipped` commands, and reject groups or masks;
- expose `silica::BlendingMode` only for ordinary layers, dispatch idempotent
  `SetLayerBlendMode` commands, and reject groups or masks;
- dispatch `CloseDocument`, remove the document, and emit `DocumentClosed`;
- benchmark the public open path against a real fixture.

The operation result owns its events; the runtime does not accumulate an
unbounded internal event queue. Presentation adapters may publish those events
through their own channel or FFI transport.

The production application owns one `DocumentRuntime`, uses its snapshots for
egui metadata, tab titles, and background visibility, dispatches
`CloseDocument` when a tab closes, and routes hierarchy/background visibility
and layer clipped/blend-mode intent through runtime events before mutating the
GPU document. Blend modes use the parser-domain enum with opt-in serde and a
stable `snake_case` transport representation; compositor enums remain adapter
details. This crate does not own egui instances, the WGPU atlas, or compositor
scheduling. Do not route pixels, GPU handles, egui values, GPUIX values, or
Node objects through this interface.

Run the focused tests with:

```bash
cargo test -p silicate-runtime --locked
```

Run the parser/runtime baseline with:

```bash
cargo run --release -p silicate-runtime --example benchmark_open -- /path/to/document.procreate 10
```
