# Feature Backlog

This file contains only missing or incomplete product work. Implemented behavior
is established by the code and its smoke/benchmark evidence; durable ownership
and constraints belong in `../ARCHITECTURE.md` or the owning crate README.

## P0: Product Foundation

- Rename the app title, bundle name, executable metadata, and icons while
  preserving upstream MIT attribution.
- Establish `Art_SystemPet_Default.procreate` as the primary native smoke and
  benchmark fixture.
- Add robust Procreate field aliases only where real files show the parser is
  too narrow.
- Parse Animation Assist settings, frame source ordering, and per-layer hold
  duration.
- Parse archived-video timing metadata where available.

## P0: Rendering Correctness

- Smoke-check preview and export alpha edges, group visibility, masks, and
  clipping against representative files.
- Record fixture evidence before changing known `Hue` or `Saturation` blend
  behavior.
- Add a mask-bearing real fixture to the runtime/GPU identity verifier.

## P0: Animation Assist

- Model one visible layer or folder as one animation frame and exclude hidden
  sources.
- Support loop, ping-pong, one-shot, FPS, and per-frame hold duration.
- Reuse GPU frame handles or textures for repeated held frames.
- Add the playback HUD and timeline/scrubber through the runtime boundary.

## P1: Export

- Add full-canvas and transparent/background still presets around the existing
  current-view export.
- Add GIF, animated PNG, PNG sequence, MP4, and HEVC animation export presets.
- Preserve frame timing and optionally repeat held frames in PNG sequences.
- Bundle a compliant LGPL ffmpeg sidecar and connect animation jobs to the
  existing tool-detection and runner boundaries.

## P1: Batch Export

- Select folders or multiple files, with optional recursive scanning.
- Reuse normal export presets with an output folder and naming template.
- Run a bounded parallel queue with progress, cancellation, per-file results,
  and retry without mutating source archives.

## P1: Platform Integration

- Validate the thumbnail COM provider through real registered Explorer loading.
- Package the Windows app, provider DLL, icon, and per-user registration flow.
- Use the shared QuickLook loader for in-app file-browser thumbnails.
- Add macOS document declarations, Finder thumbnails, and Quick Look preview.
- Add Linux MIME and desktop integration after the native Windows/macOS paths.

## P2: Rizum Glass UI

- Use `docs/UI_REFERENCES.md` for prototype roles and the pinned
  `design/rizum-glass/DESIGN.md` for canonical visual rules.
- Produce an approved interactive browser reference and target-specific
  translation contract before native implementation.
- Keep eframe/egui production-ready while GPUIX passes ADR 0001's native canvas,
  lifecycle, input, packaging, and performance gates.
- Keep canvas, layers, playback, info, export, and settings reachable from the
  first screen.
- Build export settings as one Procreate-like sheet and keep technical viewer
  controls in an Advanced/Debug surface unless the product requires otherwise.
