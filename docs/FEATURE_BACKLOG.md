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
- Add a shared QuickLook PNG extractor for `.procreate` ZIP archives.

## P0: Parser Parity Extensions

- Add ProcreateViewer's robust field aliases where upstream parsing is too
  narrow.
- Parse Animation Assist settings:
  - enabled/disabled
  - FPS
  - loop/ping-pong/one-shot mode
  - frame count/source ordering
- Parse per-layer animation hold duration.
- Parse archived video segment list and timing metadata where available.
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
  - list segments in numeric order
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

- Windows file association detection/install/uninstall.
- Windows Explorer thumbnail provider.
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
- Use `ux-prototypes/DESIGN.md` as design tokens/rules translated into egui.
- Preserve upstream Silicate responsiveness before adding visual complexity.
- Keep existing layer functionality visible while reshaping layout.
- Build export settings as one Procreate-like sheet, not a sequence of prompts.
- Add a first-class Settings panel for integration status and repair actions.
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
