# Feature Backlog

This file contains only missing or incomplete product work. Implemented behavior
is established by the code and its smoke/benchmark evidence; durable ownership
and constraints belong in `../ARCHITECTURE.md` or the owning crate README.

## P0: Product Foundation

- Add robust Procreate field aliases only where real files show the parser is
  too narrow.

## P0: Rendering Correctness

- Smoke-check preview and export alpha edges, group visibility, masks, and
  clipping against representative files.
- Record fixture evidence before changing known `Hue` or `Saturation` blend
  behavior.

## P0: Animation Assist

- Render configurable onion skins around the current frame without duplicating
  decoded images or GPU resources.
- Map stored playback mode and direction values only after controlled fixtures
  establish Procreate's raw enum semantics.

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
