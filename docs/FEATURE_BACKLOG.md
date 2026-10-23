# Feature Backlog

This file contains only missing or incomplete product work. Implemented behavior
is established by the code and its recorded validation; durable ownership
and constraints belong in `../ARCHITECTURE.md` or the owning crate README.

## P1: Export

- Confirm macOS Intel/Apple Silicon sidecar builds on hosted macOS runners;
  the source-build and release packaging paths are implemented.

## P1: Platform Integration

- Validate the thumbnail COM provider through real registered Explorer loading.
- Compile the Windows installer on a host with Inno Setup and confirm its
  install/repair/uninstall lifecycle; packaging and shared registration are implemented.
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
