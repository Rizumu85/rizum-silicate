#!/usr/bin/env bash
set -euo pipefail
if [[ $# != 5 || $(uname -s) != Darwin ]]; then
    echo "usage (macOS): $0 <binary> <sidecar-root> <version> <x86_64|arm64> <output-directory>" >&2
    exit 2
fi
root=$(cd "$(dirname "$0")/.." && pwd)
binary=$1 sidecar=$2 version=$3 arch=$4
case "$arch" in x86_64|arm64) ;; *) echo 'Unsupported architecture' >&2; exit 2 ;; esac
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Invalid version' >&2; exit 2; }
minimum=${MACOSX_DEPLOYMENT_TARGET:-11.0}
identity=${MACOS_SIGNING_IDENTITY:--}
profile=${MACOS_NOTARY_PROFILE:-}
if [[ "$identity" == - && -n "$profile" ]]; then
    echo 'Notarization requires a Developer ID Application identity' >&2; exit 2
fi
for file in "$binary" "$sidecar/tools/ffmpeg" "$sidecar/tools/ffprobe" \
    "$sidecar/tools/FFMPEG-LICENSE.txt" "$sidecar/tools/build/binary-sha256.txt"; do
    [[ -f "$file" ]] || { echo "Missing package input: $file" >&2; exit 2; }
done
mkdir -p "$5"
output=$(cd "$5" && pwd)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
app="$work/dmg/Rizum Silicate.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$work/AppIcon.iconset"
cp "$binary" "$app/Contents/MacOS/silicate"
cp -R "$sidecar/tools" "$app/Contents/MacOS/tools"
cp "$root/LICENSE" "$app/Contents/Resources/LICENSE.txt"
python3 "$root/packaging/macos/bundle_info.py" "$app/Contents/Info.plist" "$version" "$minimum"
plutil -lint "$app/Contents/Info.plist"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$root/assets/icon-1024.png" --out "$work/AppIcon.iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$root/assets/icon-1024.png" --out "$work/AppIcon.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$work/AppIcon.iconset" -o "$app/Contents/Resources/AppIcon.icns"

# Verify the downloaded bytes before signing changes their Mach-O signatures.
(cd "$app/Contents/MacOS/tools" && shasum -a 256 --check build/binary-sha256.txt)
sign_options=(--force --sign "$identity")
if [[ "$identity" != - ]]; then sign_options+=(--options runtime --timestamp); fi
notary_options=(--keychain-profile "$profile" --wait --output-format json)
dmg_sign_options=(--force --sign "$identity" --timestamp)
if [[ -n "${MACOS_SIGNING_KEYCHAIN:-}" ]]; then
    sign_options+=(--keychain "$MACOS_SIGNING_KEYCHAIN")
    notary_options+=(--keychain "$MACOS_SIGNING_KEYCHAIN")
    dmg_sign_options+=(--keychain "$MACOS_SIGNING_KEYCHAIN")
fi
notarize() {
    xcrun notarytool submit "$1" "${notary_options[@]}" > "$work/notary-result.json"
    python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); print("Notarization:", r.get("status"), r.get("id")); sys.exit(0 if r.get("status") == "Accepted" else 1)' "$work/notary-result.json"
}
for code in "$app/Contents/MacOS/tools/"*.dylib \
    "$app/Contents/MacOS/tools/ffmpeg" "$app/Contents/MacOS/tools/ffprobe" \
    "$app/Contents/MacOS/silicate"; do
    lipo -verify_arch "$arch" "$code"
    # Absolute build-machine dependencies cannot survive relocation into /Applications.
    while IFS= read -r dependency; do
        case "$dependency" in
            /System/Library/*|/usr/lib/*|@loader_path/*) ;;
            *) echo "Non-relocatable dependency in $code: $dependency" >&2; exit 1 ;;
        esac
    done < <(otool -L "$code" | tail -n +2 | sed -E 's/^[[:space:]]+//; s/ \(compatibility version.*//')
    code_options=("${sign_options[@]}")
    case "$code" in
        */tools/ffmpeg|*/tools/ffprobe)
            code_options+=(--entitlements "$root/packaging/macos/ffmpeg.entitlements.plist") ;;
    esac
    codesign "${code_options[@]}" "$code"
done
(
    cd "$app/Contents/MacOS/tools"
    mv build/binary-sha256.txt build/source-build-binary-sha256.txt
    shasum -a 256 ffmpeg ffprobe ./*.dylib > build/binary-sha256.txt
)
# Sign nested code first, then seal the bundle resources. Never re-sign with --deep.
codesign "${sign_options[@]}" "$app"
codesign --verify --deep --strict --verbose=2 "$app"
if [[ -n "$profile" ]]; then
    ditto -c -k --keepParent "$app" "$work/notarize.zip"
    notarize "$work/notarize.zip"
    xcrun stapler staple "$app"
    xcrun stapler validate "$app"
    spctl --assess --type execute --verbose=2 "$app"
fi
ln -s /Applications "$work/dmg/Applications"
if [[ "$identity" == - ]]; then
    suffix=-adhoc
elif [[ -z "$profile" ]]; then
    suffix=-signed-unnotarized
else
    suffix=
fi
dmg="$output/Rizum-Silicate-macos-${arch}${suffix}.dmg"
hdiutil create "$dmg" -volname 'Rizum Silicate' -srcfolder "$work/dmg" -format UDZO -ov
if [[ "$identity" != - ]]; then
    codesign "${dmg_sign_options[@]}" "$dmg"
fi
if [[ -n "$profile" ]]; then
    notarize "$dmg"
    xcrun stapler staple "$dmg"
    xcrun stapler validate "$dmg"
fi
echo "Package: $dmg"
