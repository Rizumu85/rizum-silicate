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

Metadata rows and tab titles read the runtime snapshot. Closing a production
tab dispatches `CloseDocument`. The adapter currently discards open/close
events after applying their local lifecycle effect; a future transport adapter
may publish them.

## Identity Invariant

`DocumentId` and `LayerId` are runtime identities. `LayerId` is assigned in a
stable depth-first preorder over layers, groups, and masks when the document is
opened. Topology is currently immutable, so IDs remain stable for the document
session.

Do not compare `LayerId` with `silica_gpu::SilicaLayer::id` or
`silica_gpu::SilicaGroup::id`. GPU IDs are renderer-local preview indices and
are allocated with atomics during parallel loading, so their values and order
are not the runtime identity contract.

`InstanceKey` is also presentation-local. It remains separate from
`DocumentId` until dock and compositor call sites can migrate in one tested
change.

## Next Vertical Slice

Route `SetLayerVisibility` through the production adapter:

1. make the layer UI emit intent instead of mutating `silica-gpu` directly;
2. dispatch the command to `DocumentRuntime`;
3. apply only returned `LayerVisibilityChanged` events to a deterministic GPU
   hierarchy mapping;
4. update the `Instance` snapshot to the returned runtime snapshot;
5. submit the GPU document to the compositor only when an event exists;
6. test layer, group, mask, no-op, missing-ID, and close-during-command paths;
7. measure command-to-present latency on the real fixture.

Prefer attaching a renderer-neutral preorder identity during GPU hierarchy
construction over searching by the current atomic preview ID. Keep that
identity in Rust; do not expose renderer handles or pixels through the runtime
command interface.

## Verified Evidence

On 2026-09-01, the debug native app opened
`Art_SystemPet_Default.procreate` (169,646,073 bytes), loaded 236 renderer
nodes, created the instance, and remained running until the verification
session was terminated. The runtime-only baseline and fixture hash are in
`docs/PERFORMANCE_BASELINES.md`.

Required checks for this slice:

```powershell
cargo test --workspace --all-targets --locked
cargo clippy -p silicate-runtime --all-targets --no-deps --locked -- -D warnings
```

The repository still has historical full-workspace `rustfmt` drift in files
outside this migration. Format touched Rust files directly and keep unrelated
formatting out of focused commits.
