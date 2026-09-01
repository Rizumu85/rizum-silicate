# ADR 0001: Qualify GPUIX Without Regressing The Native Canvas

- Status: Accepted
- Date: 2026-09-01

## Context

Silicate currently composites Procreate layers into a `wgpu::Texture` and
registers that texture directly with `egui_wgpu` on the same device. This is
the performance spine of the application: interactive preview does not read
pixels back to the CPU, encode an image, or move frames across a process or FFI
seam.

Rizum Glass uses React, TypeScript, Vite, Tailwind CSS, and shadcn/ui for
browser references. Its default native delivery stack is Bun, React 19, and a
pinned GPUIX release. GPUIX provides a high-performance React reconciler over
GPUI without Electron or a web view, but the assessed 0.7.0 release does not
provide a completed Canvas element, multiple windows, or a stable interface for
presenting an existing Silicate `wgpu::Texture`.

## Decision

Preserve the Rust parser, export, platform, and WGPU compositor modules while
qualifying GPUIX as a presentation adapter through incremental vertical
slices. The current eframe/egui application remains the production adapter
until a GPUIX CanvasHost satisfies the acceptance gates below.

Use the pinned Rizum Glass submodule at `design/rizum-glass` as the canonical
design source. Browser references and native implementations share semantic
tokens, state contracts, and named interaction states; they do not share DOM,
Tailwind, or unsupported CSS assumptions.

The target module seams are:

- `silica`: parse Procreate archives and metadata without requiring a GPU or UI.
- `silica-gpu` and `silicate-compositor`: own GPU upload and compositing.
- `SilicateRuntime`: expose document commands, immutable snapshots, and bounded
  events without egui, GPUIX, Node, or renderer types.
- `CanvasHost`: attach a document to a native presentation surface while hiding
  renderer, lifecycle, and input implementation details.
- Presentation adapters: eframe/egui today and GPUIX only after qualification.

Do not expose pixels, WGPU handles, archive internals, or complete compositor
state through the commands/snapshots interface. Document state and long-running
work stay in Rust. React owns transient presentation state such as open panels,
popover state, form drafts, and motion.

## CanvasHost Acceptance Gates

A GPUIX CanvasHost may replace the current adapter only when all of the
following are demonstrated on representative Procreate fixtures:

- Interactive preview performs no GPU-to-CPU readback, PNG/Base64 encoding, or
  per-frame pixel transfer through N-API.
- Pointer, wheel, drag, zoom, playback, resize, minimize/restore, and close
  behavior remain correct under physical input.
- Renderer queries are lifecycle-safe and cannot terminate document work when
  a window is hidden, minimized, transitioning, or closing.
- File-open, layer-toggle-to-present, pan/zoom, animation frame time, memory,
  and export benchmarks do not materially regress from the recorded eframe
  baseline without an explicitly accepted tradeoff.
- The target OS and architecture matrix has build, packaging, and GPU-backed
  automation coverage.

In-memory or data-URL `<img>` support is suitable for bounded thumbnails and
static spike evidence. It is not an accepted main-canvas bridge.

## Dependency And Upgrade Policy

- Pin exact `@gpuix/react` and `@gpuix/native` versions. Do not use semver ranges.
- Evaluate upgrades in a dedicated commit or branch; never mix them with
  product features.
- Read the release notes and pinned source, then run type checks, Rust tests,
  GPU-backed automation, lifecycle checks, physical-input checks, and the
  performance suite.
- Advance the Rizum Glass gitlink only after reviewing its changelog, generated
  assets, reference contract, and consuming-project results.
- Promote reusable findings back to Rizum Glass only after they are stated
  without Silicate-specific product details and survive a transfer test.
- Revert a failed upgrade by reverting its isolated dependency commit.

## Consequences

Silicate can adopt the design velocity and native UI quality of GPUIX without a
big-bang renderer rewrite. The temporary cost is maintaining the existing egui
adapter while the GPUIX shell is proven. This duplication is limited to
presentation; document parsing, commands, exports, platform work, and the GPU
compositor must remain shared Rust modules.

Direct GPUI or a platform-native surface island remains a possible CanvasHost
implementation if GPUIX cannot expose the required GPU capability. Neither is
approved merely because it can display a screenshot; the same acceptance gates
apply.
