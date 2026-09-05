# Architecture

Rizum Silicate is a native-first Procreate viewer, inspector, and export tool.
It preserves Silicate's Rust and WGPU rendering strengths while adding the
artist-facing workflows previously explored in ProcreateViewer.

Read the related documents only when their subject is relevant:

- `docs/FEATURE_BACKLOG.md` for unfinished product work.
- `docs/UI_REFERENCES.md` for approved prototype roles.
- `docs/adr/0001-gpuix-shell-with-native-rust-runtime.md` for presentation
  migration gates.
- `docs/PERFORMANCE_BASELINES.md` for reproducible measurements.

## Product Contract

- `.procreate` files should feel first-class on Windows and macOS.
- Opening, inspecting, toggling, and previewing artwork must remain native and
  responsive.
- Rust owns parsing, durable document state, export, batch work, video tooling,
  and platform integration.
- WGPU owns live compositing and animation preview.
- eframe/egui/WGPU is the production presentation adapter.
- GPUIX is a candidate shell only after its native canvas path passes the
  accepted lifecycle, input, packaging, and performance gates.
- The web build is useful, but the native app defines the quality bar.

The earlier Tauri experiment proved product workflows, but a WGPU preview as a
WebView child surface conflicted with platform compositor behavior. This project
therefore keeps one native Rust runtime and qualifies presentation adapters
around it instead of returning to a WebView architecture.

## Workspace Ownership

- `libs/silica`: Procreate archive and renderer-independent domain parsing.
- `libs/silicate-runtime`: UI-independent document commands, immutable
  snapshots, stable identities, and bounded revisioned events.
- `libs/silica-gpu`: GPU-ready document and tile upload plus renderer-local
  hierarchy objects.
- `libs/compositor`: WGPU compositing, blend pipelines, and presentation
  textures.
- `libs/platform-thumbnail`: presentation-independent QuickLook PNG loading.
- `libs/windows-thumbnail-provider`: Windows thumbnail DLL and bitmap handoff.
- `src/app`: document instances and compositor scheduling.
- `src/gui`: the current egui presentation adapter.
- `src/window`: native dialogs, file loading, and app events.
- `design/rizum-glass`: pinned canonical design-system repository.

The production open path parses `Document.archive` once, projects editable
state into `silicate-runtime`, and moves the parsed document into `silica-gpu`
for one-time chunk upload. After upload, the atlas assets and hierarchy are
immutable; opacity, blend, clipping, visibility, background, and canvas flip
state come only from runtime snapshots. The compositor receives a compact
render projection when a runtime command changes state instead of cloning or
comparing the full document. Chunk indirection is rebuilt only when a clipping
edit changes clip-source topology. Runtime setup is rolled back when GPU loading
or identity validation fails. See `libs/silicate-runtime/README.md` for the owned
identity and command contracts.

## Architectural Invariants

- Parser, runtime, export, and platform APIs remain free of React, egui, GPUIX,
  and compositor types.
- Pure archive parsing does not require a GPU or presentation runtime.
- Presentation adapters orchestrate commands and snapshots; they do not parse
  archive internals or own durable document state.
- Archive, decoded image, tile-atlas, thumbnail, and archived-video paths keep
  explicit resource limits; large disk-backed media is streamed where possible.
- Platform thumbnail and Quick Look hosts do not depend on egui.
- Interactive preview stays on the WGPU compositor path.
- Stacked clipped siblings share their nearest non-clipped raster base, and
  clipping scopes do not cross group boundaries. Coverage comes from that
  raster and its visible mask; group containers and base-layer opacity do not
  become clipping alpha without controlled Procreate fixture evidence.
- Non-separable `Hue` and `Saturation` blending follows the W3C compositing
  definition. Procreate-specific deviations require a reproducible render
  comparison before changing the shader.
- Main-canvas pixels do not cross N-API, Base64, encoded-image, CPU-copy, or
  GPU-readback bridges during interaction.
- The compositor is the performance spine. Change it for correctness, required
  Procreate semantics, or measured performance improvement.
- A fallback exists only for a documented current capability or platform
  requirement; completed direction changes remove the superseded path.

## Presentation Strategy

Use the pinned Rizum Glass specification and an approved interactive browser
reference as design evidence. Translate that evidence through an explicit
target contract; never leak browser or native-framework types into the product
runtime.

Keep the current egui adapter production-ready while a GPUIX vertical slice
proves the same runtime contract. A screenshot-capable bridge is insufficient:
the candidate must preserve same-device GPU presentation, physical input,
window lifecycle behavior, packaging, and recorded performance.

## Delivery Order

1. Preserve the inherited parser, compositor, and native interaction baseline
   with reproducible fixture smoke runs and measurements.
2. Extend parser semantics only where real files demonstrate a missing field or
   contract.
3. Move durable document operations behind `silicate-runtime` in coherent
   vertical slices without exposing renderer identities or pixels.
4. Qualify presentation adapters against the approved Rizum Glass reference and
   the same runtime and performance gates.
5. Build Animation Assist, export formats, and batch work on shared Rust
   boundaries.
6. Finish Windows packaging and Explorer validation, then add macOS document
   and Quick Look integration; keep Linux integration as a later platform pass.

## Upstream Strategy

Keep `upstream` pointed at Avarel/silicate and integrate selectively. Separate
upstream synchronization from product, parser, renderer, export, and platform
changes so regressions and performance effects remain attributable.
