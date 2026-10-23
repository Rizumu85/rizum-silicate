#!/usr/bin/env bash
set -euo pipefail
if [[ $# != 3 || $(uname -s) != Linux ]]; then
    echo "usage (Linux): $0 <silicate-binary> <sidecar-root> <output-directory>" >&2
    exit 2
fi
root=$(cd "$(dirname "$0")/.." && pwd)
binary=$1 sidecar=$2
for file in "$binary" "$sidecar/tools/ffmpeg" "$sidecar/tools/ffprobe" \
    "$sidecar/tools/FFMPEG-LICENSE.txt" "$sidecar/tools/build/binary-sha256.txt"; do
    [[ -f "$file" ]] || { echo "Missing package input: $file" >&2; exit 2; }
done
mkdir -p "$3"
output=$(cd "$3" && pwd)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
package="$work/Rizum-Silicate"
mkdir -p "$package/share/applications" "$package/share/mime/packages" \
    "$package/share/icons/hicolor/256x256/apps" "$package/share/icons/hicolor/256x256/mimetypes"
install -m 755 "$binary" "$package/silicate"
cp -R "$sidecar/tools" "$package/tools"
(cd "$package/tools" && sha256sum --check build/binary-sha256.txt)
install -m 755 "$root/packaging/linux/install-desktop.py" "$package/install-desktop.py"
cp "$root/packaging/linux/com.rizum.silicate.desktop" "$package/share/applications/"
cp "$root/packaging/linux/com.rizum.silicate.xml" "$package/share/mime/packages/"
cp "$root/assets/icon-256.png" "$package/share/icons/hicolor/256x256/apps/com.rizum.silicate.png"
cp "$root/assets/icon-256.png" "$package/share/icons/hicolor/256x256/mimetypes/com.rizum.silicate.procreate.png"
cp "$root/LICENSE" "$package/LICENSE.txt"
cat > "$package/INSTALL.txt" <<'EOF'
Rizum Silicate for Linux x86_64

Extract this directory to a permanent location, then run:
    python3 install-desktop.py install
This adds the application menu, Procreate file type and icons for your user.
Requires Python 3 and shared-mime-info; desktop-file-utils is recommended.
Select Rizum Silicate in your desktop's Open With settings to make it the default.
The app and tools must remain together; launching ./silicate is also supported.

Repeat install to repair registration or after moving/upgrading the package.
Before removing the package, run:
    python3 install-desktop.py uninstall
Uninstall removes owned desktop registration, not artwork or preferences.
The package directory can then be removed manually.
EOF
desktop-file-validate "$package/share/applications/com.rizum.silicate.desktop"
tar -czf "$output/Rizum-Silicate-linux-x86_64.tar.gz" -C "$work" Rizum-Silicate
echo "Package: $output/Rizum-Silicate-linux-x86_64.tar.gz"
