# Platform Integration Plan

This document tracks file association, thumbnails, Quick Look, and packaging for
Rizum Silicate.

## Current Baseline

Upstream Silicate already has:

- Native Windows, macOS, and Linux build targets.
- A package metadata block in `Cargo.toml` with app name, bundle identifier,
  icon, and category.
- CLI file arguments, so OS-level "open with this app" can feed paths into the
  viewer once the file association exists.
- Open-file and save-file dialogs through `rfd`.
- Current-view export on native.

The Rizum fork adds per-user Windows association management, Explorer refresh
actions, a shared QuickLook image loader, and the native thumbnail-provider
boundary through the COM DLL export smoke stage. Registered Explorer-host
validation and packaging remain. macOS document/Quick Look integration and
Linux MIME integration have not started.

## Windows

### File Association

Goal: `.procreate` files open in Rizum Silicate on double click and show a
friendly file type/icon.

Implementation shape:

- Register the app under per-user `RegisteredApplications`, `Capabilities`,
  `OpenWithProgids`, and its owned ProgID.
- Register machine-wide only from an elevated installer.
- Use `RizumSilicate.procreate` without overwriting Windows `UserChoice` or the
  extension default; default selection remains an explicit system-owned action.
- Set `Content Type` to `application/x-procreate` and `PerceivedType` to
  `image`.
- Add `shell\open\command` pointing at the installed executable with `%1`.
- Add an icon under `DefaultIcon`.
- Add an "Open with Rizum Silicate" shell verb if useful.
- Notify Explorer with `SHChangeNotify` after install/uninstall.
- During uninstall, delete owned trees directly and remove values from shared
  extension keys only while they still match Rizum Silicate's registration.

The old ProcreateViewer installer has working registry keys and status UI. Port
the behavior, but do not keep the C# RegAsm dependency as the long-term design.

### Explorer Thumbnails

Goal: `.procreate` thumbnails appear in File Explorer folder views.

Implementation shape:

- Build a Windows Shell thumbnail provider DLL.
- Implement COM thumbnail provider interfaces for `.procreate`.
- Extract `QuickLook/Preview.png` or `QuickLook/Thumbnail.png` from the ZIP for
  the fast first version through `libs/platform-thumbnail`.
- Later, optionally render a higher-quality thumbnail through a small pure Rust
  parser/render service if QuickLook is missing.
- Register under `.procreate\ShellEx\{e357fccd-a995-4576-b01f-234630154e96}`.
- Keep thumbnail generation isolated and panic-safe because Explorer hosts
  providers out of process.

Current implementation:

- Read-only status and explicit write plans cover the `.procreate` ShellEx
  handler, provider CLSID, DLL presence, and `ThreadingModel=Apartment`.
- `libs/platform-thumbnail` extracts Preview/Thumbnail PNG bytes and decoded
  RGBA pixels from file paths or in-memory archives without egui.
- `libs/windows-thumbnail-provider` converts path or stream input to BGRA,
  creates an owned `HBITMAP`, and exposes `IInitializeWithFile`,
  `IInitializeWithStream`, `IThumbnailProvider`, and the shell class factory.
- The crate emits `rizum_silicate_thumb.dll`; the local smoke verifier loads
  its exports and creates a stream-initializable provider without registry
  writes.
- Live Explorer loading through the registered, packaged DLL remains the final
  host-level validation.

Reference projects and docs:

- Microsoft Explorer thumbnail provider sample:
  https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/recipethumbnailprovider/
- Microsoft COM `InprocServer32` registration:
  https://learn.microsoft.com/en-us/windows/win32/com/inprocserver32
- Rust Windows thumbnail handler example:
  https://github.com/ThioJoe/win-svg-thumbs-rust

Important notes from the Microsoft sample:

- Explorer hosts thumbnail providers in an isolated process.
- Explorer caches thumbnails; testing may require cache invalidation or file
  modification-time changes.
- Do not ship with `DisableProcessIsolation`; it is only a debugging aid.

Local DLL smoke check before registry testing:

```powershell
cargo build -p rizum-windows-thumbnail-provider --locked
cargo run -p rizum-windows-thumbnail-provider --example verify_windows_thumbnail_dll_exports --locked
```

### Settings UI

Settings should report:

- App Registration: installed/missing
- Default App: selected/not selected
- Explorer Thumbnails: installed/missing
- Thumbnail DLL: present/missing
- Video Tools: bundled/system/missing

Current implementation:

- Settings reports file association, Explorer thumbnail registration,
  co-located provider DLL presence, and bundled/system/missing ffmpeg status.
- Detection derives the expected executable and provider paths without writes.
- Explicit actions install, repair, or uninstall per-user registration and
  notify Explorer after changes.
- The Choose Default App action opens Windows Settings instead of writing the
  protected default-app registry state.
- Explicit Restart Explorer and Refresh Thumbnail Cache actions remain behind
  an injectable platform boundary.
- Archived-video actions use the detected ffmpeg path and appear only when the
  loaded source contains video segments.
- Video segments stream into an exclusive temporary directory; the directory
  is cleaned on every normal success or error return.

Actions:

- Install / Repair Everything
- Restart Explorer
- Refresh Thumbnail Cache
- Uninstall All Registrations
- Open install log

Keep detection read-only. System writes remain explicit user actions, never
startup side effects.

## macOS

### Document Type Registration

Goal: `.procreate` files open in Rizum Silicate and show a proper app/file icon
in Finder.

Implementation shape:

- Extend the app bundle `Info.plist` with `CFBundleDocumentTypes`.
- Declare/import a UTI for Procreate documents if macOS does not already expose
  a stable public one for `.procreate`.
- Set the app role as viewer/editor as appropriate.
- Include document icon resources in the `.app` bundle.
- Confirm that command-line file arguments still load correctly when Finder
  opens a document.

Reference docs:

- Apple bundle structure and `Info.plist` requirements:
  https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html
- Apple `CFBundleDocumentTypes` key:
  https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundledocumenttypes

### Finder Thumbnails and Quick Look

Goal: Finder and Quick Look can preview `.procreate` files without launching the
full app.

Implementation shape:

- Build a macOS Quick Look Thumbnail extension for Finder icons.
- Build a Quick Look Preview extension for spacebar preview if practical.
- The extension should use a small shared parser to extract QuickLook PNGs from
  the ZIP first.
- Higher-quality compositing can be a later native helper/XPC service if needed.
- Bundle extensions inside the macOS app, then sign/notarize the complete app.
- Provide troubleshooting UI/instructions for enabling the Quick Look extension.

Reference projects and docs:

- Apple `QLThumbnailGenerator` API:
  https://developer.apple.com/documentation/quicklookthumbnailing/qlthumbnailgenerator
- Modern Quick Look extension example:
  https://github.com/sbarex/SourceCodeSyntaxHighlight

Notes from existing macOS Quick Look projects:

- Users may need to launch the app once before the system discovers the
  extension.
- Quick Look uses UTIs, not only filename extensions.
- Older `qlgenerator` APIs are deprecated; use modern app-bundled extensions.
- Some formats or UTIs can be reserved or conflicted by the system or other
  apps.

## Linux

Linux is not the first platform-integration target, but keep the shape open:

- install a `.desktop` file
- register MIME type for `.procreate`
- install app and document icon assets
- optionally provide thumbnailer integration for desktops that support it

## Packaging

Windows:

- Installer should bundle the app executable, ffmpeg sidecars, thumbnail DLL,
  icon assets, and repair/uninstall commands.
- The app currently detects a bundled `tools/ffmpeg(.exe)` beside the executable
  before falling back to `PATH`; packaging still needs to ship that sidecar.
- Prefer a pure Rust or `windows-rs` thumbnail DLL long term.
- Signing is strongly preferred because shell extensions run in Explorer's
  trust boundary.

macOS:

- App bundle must include `Info.plist`, icons, document type declarations, and
  Quick Look extensions.
- Signing and notarization are required for a low-friction user install.
- The app should degrade gracefully if extensions are not enabled.

## Next Milestone

1. Validate the provider through registered Explorer loading with a packaged
   app/provider layout.
2. Package the app, provider DLL, icon, and per-user association lifecycle.
3. Reuse the shared QuickLook loader for in-app file-browser thumbnails.
4. Start macOS document declarations and Quick Look extension hosting after the
   Windows package path is reproducible.
