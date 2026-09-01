# Capability Matrix

This matrix separates inherited Silicate features from Rizum-specific work.

| Area | Current Silicate Status | Rizum Work |
| --- | --- | --- |
| Native app | Exists through eframe/egui/wgpu with same-device native texture registration | Keep egui as the production adapter while an isolated GPUIX shell and zero-readback CanvasHost are qualified under ADR 0001 |
| Presentation runtime seam | `silicate-runtime` opens document bytes into stable IDs and serializable metadata/layer snapshots, returns bounded revisioned events, and accepts idempotent visibility, clipped, blend-mode, and close commands; production open/close lifecycle, egui metadata/title reads, hierarchy/background visibility, clipping, and blend mode now use it | Define numeric value and input-coalescing contracts before moving opacity or color behind this interface and before the GPUIX vertical slice |
| Web app | Exists through trunk/wasm/WebGPU | Keep as secondary viewer |
| Multi-file open | Exists through CLI args, open dialog, tabs, drag/drop | Preserve while redesigning shell |
| Procreate archive parsing | Exists for core document/layer metadata | Add robust aliases and tests from ProcreateViewer fixtures |
| Groups/folders | Exists and renders nested children | Improve Procreate-like layer-panel polish if needed |
| Layer visibility | Layer/group/mask and background UI emit runtime intent and apply revisioned events to GPU state; the real baseline verifies layer/group/background and the synthetic parser/runtime fixture verifies mask identity | Add a mask-bearing GPU fixture, command-to-present measurement, and animation-frame rules |
| Masks | Exists, including mask chunks and UI rows | Audit edge cases from ProcreateViewer |
| Clipping | Ordinary layers expose `Some(clipped)` in runtime snapshots; egui emits runtime intent, revisioned events update GPU state, and group/mask commands are rejected | Audit consecutive clipping and hidden-base rendering cases |
| Background color | Row visibility is runtime-owned; color selection still mutates the GPU adapter directly | Define a renderer-neutral color value and input-coalescing policy before moving color into runtime |
| Blend modes | Ordinary layers expose canonical `silica::BlendingMode` values with stable `snake_case` transport; egui emits runtime intent, revisioned events update GPU state, group/mask commands are rejected, and the real verifier covers a layer plus a group rejection | Document known Procreate differences before shader changes and add a mask-bearing GPU fixture |
| Canvas flips/orientation | Exists in parser/compositor/UI | Verify all preview/export paths |
| Layer previews | Exists for layers, groups, and masks | Preserve performance under redesigned UI |
| Technical canvas controls | Grid/crosshair/sampling/rotation/flip controls exist | Rebrand, hide, or move to Advanced after user decision |
| Current-view export | Exists on native to PNG/JPEG/TGA/TIFF/WebP/BMP | Turn into richer still-export presets |
| QuickLook PNG extraction | Shared parser helper exists in `libs/silica` with Preview-before-Thumbnail tests; `libs/platform-thumbnail` loads PNG bytes and decoded RGBA pixels from `.procreate` paths and in-memory archive bytes without egui | Wire into in-app file thumbnails and future OS extension hosts |
| Animation Assist metadata | Not implemented; only comments exist in structs | Parse settings, FPS, playback mode, hold duration |
| Animation preview | Not implemented | Add native egui/wgpu playback HUD and scheduler |
| Animation export | Not implemented | GIF, APNG, PNG sequence, MP4, HEVC |
| Archived video segments | Segment path listing exists in `libs/silica` with numeric ordering tests; `libs/silica` can extract ordered segment bytes; `silicate` can stage segment/temp-list files through an injected writer, build full-length or 30-second ffmpeg concat-list merge command plans, run them through an injected ffmpeg runner, gate export on detected ffmpeg tool status before staging, expose native MP4 output selection actions for source-path-backed files, and hide those actions when no archived video segments exist | Add timing metadata |
| Batch export | Not implemented | Folder/multi-file queue, progress, retry |
| ffmpeg sidecar | Pure ffmpeg tool detection exists for bundled/system/missing status, Settings reports Video Tools, command construction uses an injected executable path, command execution uses an injected runner boundary, and archived-video export gates on the detected status | Bundle LGPL build and wire remaining animation/video export jobs to the detected tool |
| Windows file association | Read-only HKCU registry snapshot, status model, Settings summary detection, current-exe expected path detection, egui status panel, registry writer, install/repair action, uninstall action, Explorer association-change notification, explicit Restart Explorer action, and explicit Refresh Thumbnail Cache action exist | Add packaging/install validation |
| Windows thumbnails | Read-only registration status model, Settings summary detection, separate co-located DLL presence row, egui status panel, registry writer, install/repair registration action, uninstall action, Explorer association-change notification, explicit Restart Explorer action, explicit Refresh Thumbnail Cache action, shared PNG/RGBA platform thumbnail loader, and Windows provider crate with path/in-memory archive bitmap loading, `IThumbnailProvider`, `IInitializeWithFile`, `IInitializeWithStream`, `IClassFactory`, `DllGetClassObject`, `DllCanUnloadNow`, and local DLL export smoke verification exist | Validate Explorer loading through registered Explorer thumbnail registration |
| macOS document type | Basic app bundle metadata only | Add `.procreate` document type/UTI/icons |
| macOS Quick Look/Finder thumbnails | Not implemented | Add Thumbnail/Preview extensions |
| Linux MIME integration | Not implemented | Later `.desktop`/MIME/thumbnailer work |
