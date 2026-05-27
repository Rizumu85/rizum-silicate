# Capability Matrix

This matrix separates inherited Silicate features from Rizum-specific work.

| Area | Current Silicate Status | Rizum Work |
| --- | --- | --- |
| Native app | Exists through eframe/egui/wgpu | Rebrand and reshape UI |
| Web app | Exists through trunk/wasm/WebGPU | Keep as secondary viewer |
| Multi-file open | Exists through CLI args, open dialog, tabs, drag/drop | Preserve while redesigning shell |
| Procreate archive parsing | Exists for core document/layer metadata | Add robust aliases and tests from ProcreateViewer fixtures |
| Groups/folders | Exists and renders nested children | Improve Procreate-like layer-panel polish if needed |
| Layer visibility | Exists for layers/groups/masks/background | Verify against fixtures and animation-frame rules |
| Masks | Exists, including mask chunks and UI rows | Audit edge cases from ProcreateViewer |
| Clipping | Exists with UI toggle and compositor support | Audit consecutive clipping and hidden-base cases |
| Background color | Exists with row, toggle, and color picker | Preserve as a first-class layer-panel item |
| Blend modes | Many modes exist in compositor | Document known Procreate differences before shader changes |
| Canvas flips/orientation | Exists in parser/compositor/UI | Verify all preview/export paths |
| Layer previews | Exists for layers, groups, and masks | Preserve performance under redesigned UI |
| Technical canvas controls | Grid/crosshair/sampling/rotation/flip controls exist | Rebrand, hide, or move to Advanced after user decision |
| Current-view export | Exists on native to PNG/JPEG/TGA/TIFF/WebP/BMP | Turn into richer still-export presets |
| QuickLook PNG extraction | Shared parser helper exists in `libs/silica` with Preview-before-Thumbnail tests; `libs/platform-thumbnail` loads PNG bytes and decoded RGBA pixels from `.procreate` paths without egui | Wire into in-app file thumbnails and future OS extension hosts |
| Animation Assist metadata | Not implemented; only comments exist in structs | Parse settings, FPS, playback mode, hold duration |
| Animation preview | Not implemented | Add native egui/wgpu playback HUD and scheduler |
| Animation export | Not implemented | GIF, APNG, PNG sequence, MP4, HEVC |
| Archived video segments | Segment path listing exists in `libs/silica` with numeric ordering tests | Add timing metadata, merge, export full/30s |
| Batch export | Not implemented | Folder/multi-file queue, progress, retry |
| ffmpeg sidecar | Not implemented | Bundle LGPL build, detect system/bundled tools |
| Windows file association | Read-only HKCU registry snapshot, status model, Settings summary detection, current-exe expected path detection, egui status panel, registry writer, install/repair action, uninstall action, Explorer association-change notification, and explicit Restart Explorer action exist | Add deeper cache invalidation if needed |
| Windows thumbnails | Read-only registration status model, Settings summary detection, co-located DLL expected path detection, egui status panel, registry writer, install/repair registration action, uninstall action, Explorer association-change notification, explicit Restart Explorer action, shared PNG/RGBA platform thumbnail loader, and Windows provider crate with BGRA/HBITMAP/shell handoff boundary exist | Implement Shell COM object/class-factory wiring and deeper thumbnail cache handling |
| macOS document type | Basic app bundle metadata only | Add `.procreate` document type/UTI/icons |
| macOS Quick Look/Finder thumbnails | Not implemented | Add Thumbnail/Preview extensions |
| Linux MIME integration | Not implemented | Later `.desktop`/MIME/thumbnailer work |
