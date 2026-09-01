# Runtime Handoff

This document records the current production ownership path and the next safe
integration step. Read it with ADR 0001 before changing the UI adapter or GPU
document model.

## Current Ownership

- `silica` parses `Document.archive` and owns the renderer-independent archive
  model.
- `silicate-runtime` projects that model into stable document/layer snapshots
  and owns revisioned command state.
- `silica-gpu` consumes the parsed archive model and archive bytes to upload
  chunks and build renderer-local hierarchy objects.
- `silicate-compositor` owns GPU compositing and presentation textures.
- the production eframe/egui adapter owns window, dock, and transient UI state.

The production open path in `src/app/mod.rs` is deliberately ordered:

1. parse bytes once with `silica::ProcreateFile::open`;
2. count archived video segments while bytes are available;
3. project the parsed document into `DocumentRuntime::open_document`;
4. release the runtime mutex;
5. move the same parsed document into
   `silica_gpu::ProcreateFile::open_document` for chunk upload;
6. roll back the runtime document with `CloseDocument` if GPU loading fails;
7. store the resulting `DocumentSnapshot` in the egui `Instance`.

Metadata rows, tab titles, and background visibility read the runtime snapshot.
Closing a production tab dispatches `CloseDocument`. The adapter currently
discards open/close events after applying their local lifecycle effect; a
future transport adapter may publish them.

## Identity Invariant

`DocumentId` is a runtime identity. `silica::HierarchyId` is assigned once by
the renderer-independent parser in a stable depth-first preorder over groups,
layers, and masks. `LayerId` preserves that value in runtime snapshots, and
the GPU hierarchy preserves the same value while building renderer objects.
Topology is currently immutable, so IDs remain stable for the document
session.

The production open path compares the complete runtime and GPU identity
sequences before creating an `Instance`. A mismatch rolls back the runtime
document and fails the open instead of allowing UI commands to target the
wrong renderer node.

Do not compare `LayerId` or `HierarchyId` with
`silica_gpu::SilicaLayer::id` or `silica_gpu::SilicaGroup::id`. Those GPU IDs
remain renderer-local preview indices allocated with atomics during parallel
loading; they are not the command identity contract.

`InstanceKey` is also presentation-local. It remains separate from
`DocumentId` until dock and compositor call sites can migrate in one tested
change.

## Next Vertical Slice

`SetLayerVisibility`, `SetBackgroundVisibility`, and `SetLayerClipped` now
cross the production adapter as UI intent, runtime commands, revisioned events,
and GPU document mutations. Clipped capability is explicit: ordinary layers
carry `Some(bool)`, while groups and masks carry `None` and reject the command.
The next slice should move one more layer-panel operation through the same path:

1. add a renderer-independent command and focused red tests;
2. keep the runtime result idempotent and event-bounded;
3. apply only returned events in the GPU adapter;
4. update the local snapshot after GPU application succeeds;
5. submit the changed GPU document to the compositor;
6. exercise the real fixture in the native app;
7. measure command-to-present latency separately from CPU state mutation.

Evaluate blend mode next because it is a bounded value and can establish a
renderer-neutral enum contract before high-frequency numeric controls. Before
moving opacity or background color, define scalar/color representations,
equality rules, and drag coalescing so input does not become ambiguous or
event-heavy. Opacity, blend mode, and background color remain adapter-owned for
now. Keep identities in Rust; do not expose renderer handles or pixels through
the runtime command interface.

## Verified Evidence

On 2026-09-01, the debug native app opened
`Art_SystemPet_Default.procreate` (169,646,073 bytes), loaded 236 renderer
nodes, created the instance, and remained running until the verification
session was terminated. The runtime-only baseline and fixture hash are in
`docs/PERFORMANCE_BASELINES.md`.

The release `verify_runtime_visibility` example opened the same fixture on an
NVIDIA GeForce RTX 5070 Ti, proved all 236 runtime/GPU hierarchy identities
equal, changed one layer, one group, background visibility, and one layer's
clipped state through runtime events, observed all requested GPU states, and
proved repeated commands emitted no events. It also proved that the GPU adapter
rejects clipping on a group. The fixture contains 208 layers, 28 groups, and no
masks, so the mask GPU branch still needs a mask-bearing real fixture. Exact
state-mutation timings and their exclusions are recorded in
`docs/PERFORMANCE_BASELINES.md`.

Required checks for this slice:

```powershell
cargo test --workspace --all-targets --locked
cargo clippy -p silicate-runtime --all-targets --no-deps --locked -- -D warnings
cargo run --release -p silica-gpu --example verify_runtime_visibility --locked -- `
  'C:\Users\Rizum\iCloudDrive\Procreate\Art_SystemPet_Default.procreate'
```

The repository still has historical full-workspace `rustfmt` drift in files
outside this migration. Format touched Rust files directly and keep unrelated
formatting out of focused commits.
