# Feature Backlog

This backlog is scoped to work that is missing or incomplete in the Rizum fork.
Do not add inherited Silicate features here unless they need a concrete
ProcreateViewer extension or correctness fix.

## Already Inherited From Silicate

Use these as baseline capabilities:

- Native egui/wgpu desktop viewer.
- Web viewer.
- Multi-file tabs.
- Open dialog and drag/drop loading.
- GPU atlas upload and compositor.
- Nested groups/folders.
- Layer and group hidden toggles.
- Mask rows and mask hidden toggles.
- Hidden ancestor handling during compositing.
- Opacity slider.
- Blend mode selection.
- Clipped layer toggle.
- Background color row with visibility and color picker.
- Layer/group/mask preview thumbnails.
- Canvas grid/crosshair/rotation/sampling options.
- Current-view export to PNG, JPEG, TGA, TIFF, WebP, and BMP on native.

## P0: Fork Foundation

- Rename app title, bundle name, executable metadata, and icons.
- Keep upstream MIT attribution intact.
- Add `Art_SystemPet_Default.procreate` as the first local fixture workflow.
- Add a capability matrix test note for inherited Silicate behavior so future
  changes do not regress it.
- Done: add a shared QuickLook PNG extractor for `.procreate` ZIP archives.
- Done: add an egui-free platform thumbnail loader that reads QuickLook PNG
  bytes and decoded RGBA pixels from `.procreate` file paths and in-memory
  archive bytes for future OS extension hosts.

## P0: Parser Parity Extensions

- Add ProcreateViewer's robust field aliases where upstream parsing is too
  narrow.
- Parse Animation Assist settings:
  - enabled/disabled
  - FPS
  - loop/ping-pong/one-shot mode
  - frame count/source ordering
- Parse per-layer animation hold duration.
- Done: parse archived video segment paths in numeric order.
- Parse archived video timing metadata where available.
- Add parser tests for nested groups, hidden ancestors, masks, background color,
  flips, and Animation Assist.

## P0: Rendering Correctness Audit

Inherited Silicate rendering is the starting point, not a todo. Audit it against
the ProcreateViewer fixtures and only add work items for mismatches.

- Verify top-to-bottom orientation across preview and export.
- Verify alpha is composited once without gray/dark edge artifacts.
- Verify group visibility, masks, clipping, and background toggles match
  ProcreateViewer's sample cases.
- Verify current-view export matches the visible canvas.
- Record known upstream differences, especially `Hue` and `Saturation` blend
  modes, before changing shader behavior.

## P0: Animation Assist

- Model one visible layer or folder as one animation frame.
- Exclude hidden layers and hidden folders from animation frames.
- Support loop, ping-pong, and one-shot playback.
- Support FPS and per-frame hold duration.
- Reuse the same GPU frame handle/texture for held frames.
- Add an egui playback HUD integrated with the canvas compositor.
- Add timeline/scrubber UI inspired by the ProcreateViewer prototypes.

## P1: Export

Extend the existing current-view export instead of replacing it.

- Still export presets:
  - current view
  - full canvas
  - transparent/background modes
  - PNG, JPEG, TIFF, WebP first; keep TGA/BMP if useful
- Animation GIF:
  - maximum resolution/web ready
  - frames per second
  - dithering
  - per-frame color palette
  - transparent background
  - alpha threshold
- Animated PNG:
  - maximum resolution/web ready
  - frames per second
  - transparent background
- PNG sequence:
  - repeat held frames
  - unique source frame mode
  - timing metadata file
- MP4 and HEVC:
  - maximum resolution/web ready
  - frames per second
  - transparent background for HEVC where supported
- Archived video:
  - done: list segments in numeric order
  - export full length
  - export 30-second version
  - merge segments through bundled ffmpeg sidecar

## P1: Batch Export

- Select a folder or multiple files.
- Recursive scan option.
- Output folder and naming template.
- Per-format preset reuse from normal export.
- Parallel job queue with progress.
- Per-file result table.
- Retry failed files.
- Cancel safely without mutating source archives.
- Include still export, animation export when Animation Assist exists, and
  archived video export when segments exist.

## P1: System Integration

- Windows file association detection/install/uninstall:
  - done: pure status model for expected registry values
  - done: read-only HKCU registry snapshot reader
  - done: combined detection entry point for Settings status rows
  - done: current executable/co-located thumbnail DLL expected-path detection
  - done: pure install/repair registry write plan
  - done: explicit install/repair action wiring in Settings
  - done: uninstall action wiring in Settings
  - done: Explorer association-change notification after registration changes
  - done: explicit Restart Explorer action
  - add deeper thumbnail cache invalidation
- Windows Explorer thumbnail provider:
  - done: read-only registration status model
  - done: pure registration write plan for the ShellEx/provider DLL keys
  - done: Settings install/repair/uninstall actions apply thumbnail
    registration with the rest of Windows integration
  - done: shared platform thumbnail loader for QuickLook Preview/Thumbnail PNG
    bytes and decoded RGBA pixels from file paths and in-memory archives
  - done: Windows thumbnail provider crate boundary with path-level BGRA
    bitmap loading
  - done: Windows `HBITMAP` bridge for BGRA bitmap data
  - done: shell-ready HBITMAP/alpha handoff for `IThumbnailProvider`
  - done: COM object implementing `IThumbnailProvider` and
    `IInitializeWithFile`
  - done: Shell class factory and DLL exports for `DllGetClassObject` and
    `DllCanUnloadNow`
  - validate Explorer loading through the registered DLL
- Windows thumbnail cache refresh and Explorer restart actions.
- macOS document type registration in app bundle.
- macOS Finder thumbnail extension.
- macOS Quick Look preview extension.
- Linux MIME/desktop integration later.

## P2: UI Polish

- Use `docs/UI_REFERENCES.md` as the source of truth for prototype roles.
- Use `concept18_rizum_glass_perfect.html` as the primary visual/layout target.
- Use `concept22_playback_morph_focus.html` for playback focus/morph behavior.
- Use `concept23_rizum_glass_animated_panels.html` for panel motion.
- Use `docs/ux-prototypes/DESIGN.md` as design tokens/rules translated into
  egui.
- Preserve upstream Silicate responsiveness before adding visual complexity.
- Keep existing layer functionality visible while reshaping layout.
- Build export settings as one Procreate-like sheet, not a sequence of prompts.
- Add a first-class Settings panel for integration status and repair actions.
  - done: Windows integration status summary rows for file association and
    Explorer thumbnails
  - done: combined read-only detection for those Settings rows
  - done: read-only egui Settings panel UI
  - done: execution wiring for install/repair actions
  - done: uninstall action wiring
  - done: explicit Restart Explorer action
  - add deeper thumbnail cache invalidation action
- Keep canvas, layers, playback, info, export, and settings reachable from the
  first screen.
- Review inherited Silicate technical controls before exposing them in the
  default UI:
  - Grid View
  - Extended Crosshair
  - Sampling: Nearest/Linear
  - free rotation slider and middle-drag rotation
  - horizontal/vertical flip controls
  - boxed zoom
  - manual blend/clipping/background editing
  - debug/performance indicators
