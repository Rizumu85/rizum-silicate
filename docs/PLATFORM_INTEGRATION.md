# Platform Integration Plan

This document tracks file association, thumbnails, Quick Look, and packaging for
Rizum Silicate.

## Current Baseline

Upstream Silicate already has:

- Native Windows, macOS, and Linux build targets.
- CLI file arguments, so OS-level "open with this app" can feed paths into the
  viewer once the file association exists.
- Open-file and save-file dialogs through `rfd`.
- Current-view export on native.

The Rizum fork adds per-user Windows association management, Explorer refresh
actions, a shared QuickLook image loader, and the native thumbnail-provider
boundary. Windows installer and portable package recipes include the provider
and shared per-user registration lifecycle. Installer compilation and registered
Explorer-host validation remain. macOS document delivery, bundle metadata, and
signed/notarized packaging are implemented but await native builds. macOS Quick
Look extensions and Linux MIME integration have not started.

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

Current implementation:

- `packaging/macos/bundle_info.py` owns `Info.plist`: the app is a Viewer and
  Alternate handler for `.procreate`, with an imported document type and an
  embedded ICNS icon. It does not claim ownership of the Procreate format or
  force the default application.
- Finder delivers files through Apple Events, not command-line arguments.
  `src/platform/macos` registers a `kAEOpenDocuments` receiver during AppKit's
  will-finish-launching notification, preserving winit's application delegate.
  File URLs enter a bounded 1,024-path queue, including cold-launch events.
  The presentation adapter drains it into the normal file-loading path and
  requests a repaint when more files arrive. CLI paths remain supported.
- Native compilation and Finder cold/running-app document delivery still need
  confirmation on macOS; Windows compilation cannot validate the AppKit bridge.

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

- `packaging/windows/silicate.iss` builds an Inno Setup installer for Windows 10+
  x64 systems. ARM64 requires a native provider/package and is not advertised
  by this installer. Installation is per-user under
  `%LOCALAPPDATA%\Programs\Rizum Silicate`, with a Start menu shortcut and an
  Installed Apps uninstall entry. Re-running Setup repairs the installed files
  and registration; it uses Restart Manager for files in use without forced
  termination. Artwork and app preferences are not installer-owned files.
- The x64 MSVC target statically links its C runtime for both the application
  and provider, avoiding a machine-wide Visual C++ runtime prerequisite.
- The executable contains the application/document icon. Both the portable ZIP
  and installer contain the app, `rizum_silicate_thumb.dll`, and complete FFmpeg
  sidecars including source archives and licenses. Portable extraction itself
  performs no registration.
- Setup calls `--windows-integration install` after copying files and
  `--windows-integration uninstall` before removing them. These commands exit
  before GPU/window initialization and use the same platform implementation as
  Settings. Missing provider DLLs fail registration before registry writes.
  Uninstall checks the registered executable and DLL paths independently so an
  older copy does not remove registration owned by another copy. UserChoice is
  never written; select the default application through Windows Settings.
- Build both Rust packages with `cargo build --release --locked -p silicate -p
  rizum-windows-thumbnail-provider`, then run
  `./scripts/package-windows.ps1 -BinaryDirectory target/release -SidecarDirectory
  <source-built-sidecar-root> -Compiler <path-to-ISCC.exe>` with Inno Setup 6.5.4
  or later. `dist` receives the portable ZIP and setup EXE. The release workflow
  uses the Windows runner's Inno Setup compiler, uploads both packages as build
  artifacts, and attaches them to tagged releases. Manual dispatch builds
  packages without publishing a release.
- Setup logs registration exit codes in its standard log; nonzero registration
  results are reported as errors. Re-run Setup to restore missing app files
  before uninstalling a damaged installation.
- All release archives use the shared pinned-source recipe in
  `scripts/build-ffmpeg.sh`, with GPL/nonfree and automatic dependency discovery
  disabled. Windows ZIPs and Linux tarballs place `tools/ffmpeg` beside the app;
  macOS DMGs include it inside `.app/Contents/MacOS/tools`. Matching FFmpeg/zlib
  sources, optional NVIDIA headers, licenses, hashes, configuration, and build
  instructions travel with the executable and replaceable shared libraries.
- Linux uses `$ORIGIN` for sibling shared libraries; macOS uses `@loader_path`
  and signs the tools and final app according to the configured identity. The workflow builds each macOS
  architecture natively on a matching macOS 15 runner and preserves executable
  permissions inside the intermediate tarball. Ad-hoc signing is not Developer
  ID signing or notarization.
- The app prefers the bundled tool before `PATH`. Video encoding requires OS
  codecs/drivers: Media Foundation or NVENC on Windows, VideoToolbox on macOS,
  and NVENC on Linux. Runtime probes determine actual availability.
- Prefer a pure Rust or `windows-rs` thumbnail DLL long term.
- Signing is strongly preferred because shell extensions run in Explorer's
  trust boundary.

macOS:

- `scripts/package-macos.sh` replaces cargo-bundle in release builds. It creates
  the app, ICNS icon, document declarations, complete FFmpeg sidecars, and a
  compressed DMG with an Applications link. The app and sidecars target macOS
  11.0 or later; Intel and Apple Silicon are separate native builds.
- The script verifies sidecar input hashes, Mach-O architecture and relocatable
  library paths, signs nested code before sealing the app, and verifies its
  signature. Re-signing changes binary bytes: the original hashes are retained
  as `source-build-binary-sha256.txt`, and `binary-sha256.txt` describes the
  shipped sidecars after signing.
- Local build invocation:
  `bash scripts/package-macos.sh <silicate-binary> <sidecar-root> <version>
  <x86_64|arm64> <output-directory>`. Set `MACOSX_DEPLOYMENT_TARGET=11.0` for
  both the Rust and FFmpeg builds. Default ad-hoc packages have `-adhoc.dmg`
  filenames and are not notarized releases.
- Set `MACOS_SIGNING_IDENTITY` to a Developer ID Application identity already
  in the keychain and `MACOS_NOTARY_PROFILE` to a `notarytool` credential profile
  for a distributable package. Signing enables hardened runtime and secure
  timestamps. Both app and DMG must receive an Accepted notarization response;
  tickets are stapled and validated. A signed build without notarization is
  explicitly named `-signed-unnotarized.dmg`.
- Only FFmpeg/ffprobe receive the disable-library-validation entitlement, so
  LGPL shared libraries remain replaceable without the publisher's signing key.
  The viewer keeps the default hardened-runtime library validation.
- CI accepts repository secrets `MACOS_CERTIFICATE_P12_BASE64`,
  `MACOS_CERTIFICATE_PASSWORD`, `MACOS_SIGNING_IDENTITY`, `MACOS_APPLE_ID`,
  `MACOS_APP_PASSWORD` (Apple app-specific password), and `MACOS_TEAM_ID`.
  It imports the certificate and stores the notarization profile in a temporary
  keychain removed in an always-run cleanup step. Partial configuration fails;
  no configured credentials produces only the clearly named ad-hoc package.
- Quick Look thumbnail/preview extensions remain a separate unfinished host
  integration. Current document declarations do not provide Finder previews.

## Next Milestone

1. Validate the provider through registered Explorer loading with a packaged
   app/provider layout.
2. Compile and qualify the Windows installer lifecycle on an Inno Setup host.
3. Reuse the shared QuickLook loader for in-app file-browser thumbnails.
4. Compile and qualify macOS document delivery and signing on native runners,
   then add Quick Look extension hosting through the shared thumbnail loader.
