# RIZUM_SILICATE.md

This is the working guide for the Rizum fork of Silicate. Read it before large
changes. The goal is not to reimplement Silicate from scratch; the goal is to
keep Silicate's native Rust + egui + wgpu strengths and add the missing
ProcreateViewer product features around animation, export, batch workflows, and
OS integration.

For a fast inherited-vs-new scan, see
[`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md).
For UI prototype roles and inherited Silicate control decisions, see
[`docs/UI_REFERENCES.md`](docs/UI_REFERENCES.md).

## Product Direction

Rizum Silicate is the native-first successor to ProcreateViewer. It should make
`.procreate` files feel first-class on Windows and macOS while keeping the
snappy preview behavior that made upstream Silicate attractive.

The old ProcreateViewer Tauri branch proved the feature set, but native WGPU
preview as a WebView child surface fought platform compositors. This fork should
use one native UI/rendering domain:

- egui owns app chrome, canvas overlays, playback controls, layer panel, export
  sheets, settings, and system-integration actions.
- wgpu owns live compositing and animation preview.
- Rust owns parsing, export, batch jobs, video tooling, and platform
  integration.
- Web builds can remain useful, but the native app is the quality bar.

Do not port React code directly. Use ProcreateViewer and the prototypes under
`docs/ux-prototypes/` as product references, not as architecture.

## Current Upstream Baseline

These capabilities already exist in this fork because they are inherited from
Silicate. Treat them as foundations to preserve, not features to rebuild.

### Runtime and App Shell

- Native desktop app through `eframe`/egui/wgpu.
- Web build through `trunk`/wasm where WebGPU is available.
- CLI initial file loading.
- Open-file dialog and drag/drop loading.
- Multi-document tabs through `egui_dock`.
- Toasts for load/export feedback.
- Persistent theme preference.
- High-performance native file loading through memory mapping.

### Procreate Parsing

- Reads `.procreate` ZIP archives.
- Parses `Document.archive` through the `libs/silica` NSKeyedArchive decoder.
- Parses canvas size, tile size, document name, author, stroke count,
  orientation, canvas flip flags, background color, background hidden state,
  layers, groups, masks, clipping, opacity, visibility, UUIDs, versions, and
  blend mode IDs.
- Loads layer and mask chunks into GPU-friendly structures under
  `libs/silica-gpu`.
- Uses native parallel loading on desktop where available.

### GPU Rendering

- Builds a WGPU atlas texture for layer/mask chunks.
- Uses `libs/compositor` for GPU compositing.
- Supports layer visibility, hidden ancestors through group flattening, opacity,
  clipping, masks, background color, canvas flip flags, and many Procreate blend
  modes.
- Keeps live preview in the same egui/wgpu compositor rather than a separate
  native overlay window.
- Generates layer/group/mask preview textures for the layer panel.

### Existing UI

- Canvas view with zoom/pan-style interaction, grid/crosshair options, rotation,
  nearest/linear sampling, and resettable native texture binding.
- Actions pane with document metadata and current-view export.
- Layers pane with nested groups, expand/collapse, hidden toggles, mask rows,
  opacity slider, blend mode control, clipped toggle, and background color
  control.
- Empty state with open-file action.

### Existing Export

- Exports the current rendered view by reading the WGPU output texture.
- Native save dialog currently offers PNG, JPEG, TGA, TIFF, WebP, and BMP.
- Web save path exports PNG.

## Actual Product Gaps

These are the parts this fork must add or substantially extend.

### Metadata Gaps

- Animation Assist document settings.
- Per-layer animation hold duration.
- Robust Procreate field aliases seen in ProcreateViewer's parser.
- Archived video segment metadata and numeric ordering.
- QuickLook preview/thumbnail extraction as a shared service.
- More explicit tests for sample documents such as
  `Art_SystemPet_Default.procreate`.

### Preview and UI Gaps

- Rizum/ProcreateViewer visual shell and first-screen layout.
- Playback panel and animation HUD.
- Export settings sheet matching Procreate-style controls.
- Batch export panel.
- Settings panel for system integration and video tool status.
- Clearer distinction between inherited layer controls and new Animation Assist
  frame controls.
- A product pass over inherited Silicate controls so technical viewer/debug
  controls do not appear in the primary artist-facing UI by default.

### Export Gaps

- Still export presets beyond current "export current view" behavior.
- Batch export for folders/multiple files.
- Animation GIF/APNG/PNG sequence/MP4/HEVC export.
- PNG sequence timing metadata and repeat-held-frames option.
- Archived video full-length and 30-second export from `video/segments`.
- Bundled LGPL ffmpeg sidecar detection and command construction.

### Platform Gaps

- Windows file association install/repair/uninstall.
- Windows Explorer thumbnail provider.
- Windows Explorer restart/cache refresh actions.
- macOS document type declarations.
- macOS Finder thumbnail and Quick Look preview extensions.
- Linux MIME/desktop integration later.

## Architecture Rules

Keep the current workspace split and extend it deliberately:

```text
libs/
  silica/                 # Procreate archive/domain parsing
  silica-gpu/             # GPU-friendly document/tile packages
  compositor/             # wgpu compositor and blend pipeline
  platform-thumbnail/     # egui-free PNG thumbnail loading for OS extensions
  windows-thumbnail-provider/ # Windows thumbnail DLL/bitmap provider boundary
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

- Parser and exporter behavior should live in pure Rust modules with tests.
- egui code should orchestrate and present state, not parse archive internals.
- Platform thumbnail/Quick Look extensions must not depend on egui.
- Keep the WGPU compositor path as the default live-preview path.
- Do not introduce a Tauri/WebView dependency into this fork.

## Migration Order

1. Stabilize baseline:
   - run upstream app with `Art_SystemPet_Default.procreate`
   - confirm orientation, alpha, group visibility, masks, and current export
   - document any upstream rendering mismatch as a test fixture
2. Parser parity:
   - add ProcreateViewer sample tests
   - port robust field aliases only where upstream parser lacks them
   - add Animation Assist, QuickLook, and archived video metadata parsing
3. UI shell:
   - rename/brand app
   - reshape existing egui panes into the Rizum layout
   - follow `concept18_rizum_glass_perfect.html` for primary layout
   - use `concept22_playback_morph_focus.html` and
     `concept23_rizum_glass_animated_panels.html` only for motion/playback
     behavior
   - translate `docs/ux-prototypes/DESIGN.md` into egui visual primitives
   - preserve existing layer controls while adding playback/export/settings
   - move technical controls behind Advanced/Debug unless Rizum chooses to keep
     them in the default UI
4. Animation:
   - build frame source model from layers/folders
   - support loop, ping-pong, one-shot, FPS, and hold duration
   - preview with GPU texture reuse for repeated held frames
5. Export:
   - turn current-view export into still export presets
   - add animation export formats
   - add archived video export
   - add batch export queue
6. Platform integration:
   - Windows association and thumbnails first
   - macOS document types and Quick Look extensions second
   - Linux MIME/desktop integration later

## Verification

Baseline commands:

```powershell
cargo fmt --check
cargo test --locked
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

Focused tests to add while porting:

- Animation Assist setting parsing
- animation frame expansion and hold duration
- hidden folder/layer exclusion from animation
- background color visibility
- QuickLook PNG extraction fallback order
- archived video segment numeric ordering
- batch export progress and error collection
- ffmpeg command construction with injected tool path
- Windows registry status parsing without writes

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
