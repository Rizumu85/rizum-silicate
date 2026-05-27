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

It does not currently provide the ProcreateViewer system integration layer:

- no Windows `.procreate` file association installer/repair UI
- no Windows Explorer thumbnail provider
- no macOS `.procreate` document type declaration
- no macOS Finder thumbnail or Quick Look preview extension
- no Linux MIME/desktop registration

## Windows

### File Association

Goal: `.procreate` files open in Rizum Silicate on double click and show a
friendly file type/icon.

Implementation shape:

- Register `.procreate` under `HKCU\Software\Classes` for per-user installs.
- Register machine-wide only from an elevated installer.
- Set a ProgID such as `RizumSilicate.procreate`.
- Set `Content Type` to `application/x-procreate` and `PerceivedType` to
  `image`.
- Add `shell\open\command` pointing at the installed executable with `%1`.
- Add an icon under `DefaultIcon`.
- Add an "Open with Rizum Silicate" shell verb if useful.
- Notify Explorer with `SHChangeNotify` after install/uninstall.

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

Current progress:

- Done: read-only registration status model for the `.procreate` ShellEx
  thumbnail handler, provider CLSID registration, and provider DLL file
  presence.
- Done: pure registration write plan and Settings install/repair/uninstall
  actions for ShellEx/provider DLL registry keys.
- Done: shared, egui-free platform thumbnail loader that reads QuickLook
  Preview/Thumbnail PNG bytes and decoded RGBA pixels from `.procreate`
  paths.
- Not done: the actual Shell thumbnail provider DLL/COM bitmap bridge.

Reference projects and docs:

- Microsoft Explorer thumbnail provider sample:
  https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/recipethumbnailprovider/
- Rust Windows thumbnail handler example:
  https://github.com/ThioJoe/win-svg-thumbs-rust

Important notes from the Microsoft sample:

- Explorer hosts thumbnail providers in an isolated process.
- Explorer caches thumbnails; testing may require cache invalidation or file
  modification-time changes.
- Do not ship with `DisableProcessIsolation`; it is only a debugging aid.

### Settings UI

Settings should report:

- File Association: installed/missing
- Explorer Thumbnails: installed/missing
- Thumbnail DLL: present/missing
- Video Tools: bundled/system/missing

Current progress:

- Done: Windows integration status summary rows for file association and
  Explorer thumbnails.
- Done: a read-only detection entry point that combines registry checks and
  thumbnail DLL file presence into those summary rows.
- Done: current-install detection derives the expected app executable and
  co-located thumbnail DLL paths for the Settings UI.
- Done: read-only egui Settings panel for integration status.
- Done: pure install/repair registry write plan for per-user file association
  and thumbnail registration.
- Done: Windows registry writer and explicit Settings UI install/repair
  action.
- Done: uninstall action and Explorer association-change notification after
  registration changes.
- Done: explicit Restart Explorer action.
- Not done: deeper thumbnail cache invalidation.

Actions:

- Install / Repair Everything
- Restart Explorer
- Uninstall All Registrations
- Open install log

Use read-only detection first, then add write actions. System writes should be
explicit user actions, not startup side effects.

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
- Prefer a pure Rust or `windows-rs` thumbnail DLL long term.
- Signing is strongly preferred because shell extensions run in Explorer's
  trust boundary.

macOS:

- App bundle must include `Info.plist`, icons, document type declarations, and
  Quick Look extensions.
- Signing and notarization are required for a low-friction user install.
- The app should degrade gracefully if extensions are not enabled.

## First Milestone

1. Keep inherited CLI file loading working.
2. Done: add read-only Windows registry status checks.
   - Done: pure status model for expected registry values.
   - Done: read those values from `HKCU\Software\Classes` without writes.
3. Add Settings rows for file association and thumbnail status.
   - Done: read-only thumbnail registration status model.
   - Done: combined detection entry point for Settings rows.
   - Done: read-only egui Settings panel.
4. Done: add a pure QuickLook PNG extraction function.
5. Use that function from in-app thumbnails and future extension prototypes.
   - Done: egui-free platform thumbnail loader for future extension hosts.
   - Not done: in-app file browser thumbnails.
6. Only then add install/repair/uninstall write actions.
   - Done: pure install/repair registry write plan.
   - Done: Windows registry writer and Settings install/repair action wiring.
   - Done: uninstall action and Explorer association-change notification.
   - Done: explicit Restart Explorer action.
   - Not done: deeper thumbnail cache invalidation.
