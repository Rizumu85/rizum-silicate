# Silicate Runtime

`silicate-runtime` is the UI-independent document interface shared by current
and future presentation adapters. It owns parsed Procreate document state and
exposes serializable commands, immutable snapshots, and bounded revisioned
events.

## Ownership

The runtime owns durable document identity, metadata, layer snapshots,
Animation Assist settings and derived frame timing, and command state. It does
not own presentation instances, WGPU atlas resources, compositor scheduling,
GPU handles, pixels, or framework values.

The production adapter opens a document in this order:

1. Parse bytes once with `silica::ProcreateFile::open`.
2. Inspect archive-only capabilities while the bytes are available.
3. Project the parsed document into `DocumentRuntime::open_document`.
4. Release the runtime lock.
5. Move the same parsed document into `silica-gpu` for chunk upload.
6. Close the runtime document if GPU loading fails.
7. Compare runtime and GPU hierarchy identities before creating the
   presentation instance.

A mismatch fails the open and rolls back runtime state rather than allowing a
UI command to mutate the wrong renderer node.

## Identity Contract

- `DocumentId` identifies one runtime document session.
- `silica::HierarchyId` is assigned by the renderer-independent parser in
  stable depth-first preorder across groups, layers, and masks.
- `LayerId` preserves that hierarchy identity in runtime snapshots.
- The GPU hierarchy preserves the same hierarchy identity when constructing
  renderer objects.
- `SilicaLayer::id` and `SilicaGroup::id` are renderer-local preview indices;
  they are never command identities.
- `InstanceKey` is presentation-local and remains separate from `DocumentId`.

Topology is currently immutable, so hierarchy identities remain stable for the
document session. Any future topology editing must define identity lifetime
before changing this contract.

## Command Contract

Commands are idempotent and return only the events produced by that operation.
The runtime does not accumulate an unbounded internal event queue. Adapters
apply returned events to GPU state and update their local snapshot only after
GPU application succeeds.

Current commands cover open/close, layer/group/mask visibility, background
visibility and color, canvas flips, ordinary-layer clipping with a sibling
base, blend mode, and opacity, plus Animation Assist onion-skin settings.
Capability is explicit in snapshots: groups and masks reject clipping,
blend-mode, and opacity commands instead of accepting values they cannot
represent. Enabling clipping also requires a non-clipped raster sibling below
the target in the same group scope.

Mutable commands participate in a bounded 256-entry undo history. Adapters may
attach a `HistoryGroupId` to updates from one continuous interaction so the
gesture remains one undo step; the runtime does not infer presentation timing.
Snapshots expose dirty, undo, and redo state relative to an explicit saved
position. Closing a dirty document requires explicit discard intent, and future
save-back adapters must mark the new saved position after durable persistence.

Blend modes use `silica::BlendingMode` as the document contract with a stable
`snake_case` transport representation. Compositor enums remain adapter-local.
Do not route egui values, GPUIX values, Node objects, renderer handles, or pixels
through this crate.

Animation frames are derived from the current visible top-level layer snapshot
in bottom-to-top timeline order, excluding designated background and foreground
items. Hold slots repeat the same `LayerId`; they do not duplicate parsed
images, GPU handles, or textures. The runtime owns explicit Loop, Ping Pong, and
One Shot playback modes, forward/reverse traversal, seeking, and a
fraction-preserving `Duration` clock. Clock advancement returns a compact
playback snapshot rather than cloning the layer tree. Stored Procreate mode and
direction values remain raw until controlled fixtures establish their enum
mapping. Frame-isolated rendering activates only when the stored assist flag is
confirmed enabled or an adapter explicitly plays, seeks, or enables it.
Onion skins select up to the configured number of distinct drawing sources on
each side of the current source. The nearest source uses the configured opacity
and each farther source fades linearly. This selection remains independent of
renderer phase ordering and does not duplicate held slots or GPU resources.

## Evidence

Reproducible fixture measurements and verifier scope are recorded in
[`../../docs/PERFORMANCE_BASELINES.md`](../../docs/PERFORMANCE_BASELINES.md).

```bash
cargo run --release -p silicate-runtime --example benchmark_open -- /path/to/document.procreate 10
cargo run --release -p silicate-runtime --example verify_animation_snapshot -- /path/to/document.procreate
cargo run --release -p silicate-runtime --example verify_animation_playback -- /path/to/document.procreate
```
