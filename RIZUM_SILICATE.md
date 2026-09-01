# RIZUM_SILICATE.md

This is the stable product and architecture guide for the Rizum fork of
Silicate. Read it before large changes. The goal is not to reimplement Silicate
from scratch; it is to preserve Silicate's Rust and WGPU strengths, evolve
presentation through qualified adapters, and complete the ProcreateViewer
workflow around animation, export, batch work, and OS integration.

For a fast inherited-vs-new scan, see
[`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md).
For UI prototype roles and inherited Silicate control decisions, see
[`docs/UI_REFERENCES.md`](docs/UI_REFERENCES.md).
For the accepted GPUIX/runtime migration decision and performance gates, see
[`docs/adr/0001-gpuix-shell-with-native-rust-runtime.md`](docs/adr/0001-gpuix-shell-with-native-rust-runtime.md).
For reproducible measurements and their exact scope, see
[`docs/PERFORMANCE_BASELINES.md`](docs/PERFORMANCE_BASELINES.md).
For the current runtime ownership map and next adapter work, see
[`docs/RUNTIME_HANDOFF.md`](docs/RUNTIME_HANDOFF.md).

## Product Direction

Rizum Silicate is the native-first successor to ProcreateViewer. It should make
`.procreate` files feel first-class on Windows and macOS while keeping the
snappy preview behavior that made upstream Silicate attractive.

The old ProcreateViewer Tauri branch proved the feature set, but native WGPU
preview as a WebView child surface fought platform compositors. This fork keeps
one Rust document/runtime domain and qualifies presentation adapters around it:

- wgpu owns live compositing and animation preview.
- Rust owns parsing, document state, export, batch jobs, video tooling, and
  platform integration.
- eframe/egui is the current production presentation and CanvasHost adapter.
- GPUIX is the target shell candidate after its native canvas path passes the
  accepted performance, lifecycle, input, and packaging gates.
- Web builds can remain useful, but the native app is the quality bar.

Do not put React, egui, or GPUIX types into parser, runtime, export, or platform
interfaces. Use the pinned `design/rizum-glass/DESIGN.md` and approved browser
references as design evidence, then translate through target-specific adapters.

## Delivery State

Do not maintain capability completion lists in this guide. Use
[`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md) for implemented
behavior, [`docs/FEATURE_BACKLOG.md`](docs/FEATURE_BACKLOG.md) for remaining
product work, and [`docs/RUNTIME_HANDOFF.md`](docs/RUNTIME_HANDOFF.md) for the
active runtime boundary. This separation prevents a completed feature from
remaining documented elsewhere as a gap.

## Architecture Rules

Keep the current workspace split and extend it deliberately:

```text
libs/
  silica/                 # Procreate archive/domain parsing
  silica-gpu/             # GPU-friendly document/tile packages
  silicate-runtime/       # egui-free commands, snapshots, and bounded events
  compositor/             # wgpu compositor and blend pipeline
  platform-thumbnail/     # egui-free PNG thumbnail loading for OS extensions
  windows-thumbnail-provider/ # Windows thumbnail DLL/bitmap provider boundary
design/
  rizum-glass/            # pinned canonical design-system submodule
src/
  app/                    # document instances, compositor scheduling
  gui/                    # egui UI shell and widgets
  window/                 # native dialogs, file loading, app events
```

Add new modules only where the boundary is clear:

```text
src/platform/
  windows/
    association.rs
    thumbnails.rs
    explorer.rs
  macos/
    bundle.rs
    quicklook.rs
  linux/
    mime.rs

src/export/
  still.rs
  animation.rs
  archived_video.rs
  batch.rs
  ffmpeg.rs

libs/silica/src/
  animation.rs
  quicklook.rs
  video.rs
```

Rules:

- Parser and exporter behavior should live in pure Rust modules behind
  UI-independent contracts.
- Pure document parsing must not require a GPU device or presentation runtime.
- Presentation code should orchestrate commands and snapshots, not parse
  archive internals or own durable document state.
- Platform thumbnail/Quick Look extensions must not depend on egui.
- Keep the WGPU compositor path as the default live-preview path.
- Do not introduce a Tauri/WebView dependency into this fork.
- Do not move main-canvas pixels through N-API, Base64, encoded image files, or
  GPU-to-CPU readback during interactive preview.

## Migration Order

1. Preserve the inherited parser, WGPU compositor, and native interaction
   baseline with reproducible fixture smoke runs and measurements.
2. Complete parser parity only where real Procreate files demonstrate a missing
   field or semantic contract.
3. Move durable document commands behind `silicate-runtime` one vertical slice
   at a time while keeping renderer identities and pixels out of the interface.
4. Build the approved Rizum Glass browser reference, then qualify egui and
   GPUIX adapters against the same runtime, interaction, and performance gates.
5. Add Animation Assist, export presets, animation formats, and batch work on
   reusable runtime/export boundaries.
6. Finish Windows packaging and Explorer validation, then add macOS document
   and Quick Look integration; keep Linux integration as a later platform pass.

## Verification

Project validation uses compile/lint checks plus benchmarks, smoke tests, and
performance tests when performance evidence is required. Do not make TDD or a
growing unit-test matrix part of the delivery process.

Representative commands:

```powershell
cargo check --workspace --all-targets --locked
cargo run --release -p silicate-runtime --example benchmark_open -- `
  "C:\path\to\document.procreate" 10
cargo run --release -- "C:\Users\Rizum\iCloudDrive\Procreate\Art_SystemPet_Default.procreate"
```

Manual fixture checklist:

- opening feels immediate after file selection
- layer panel shows nested groups and background color
- toggling a layer, group, mask, or background does not flip, resize, fade, or
  reorder the image
- opacity, blend mode, clipping, and mask toggles visibly update the preview
- current-view export matches the visible canvas
- animation playback visibly advances once Animation Assist is implemented
- export previews show the complete artwork without clipping

## Upstream Strategy

Keep `upstream` pointed at Avarel/silicate and rebase selectively while this
fork is young.

Separate commits by concern:

- upstream sync
- branding/UI changes
- parser feature additions
- renderer feature additions
- export/platform integration

Do not rewrite `libs/compositor` for style. It is the performance spine. Change
it only for correctness, new Procreate semantics, or measured performance wins.
