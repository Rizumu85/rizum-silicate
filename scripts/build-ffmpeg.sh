#!/usr/bin/env bash
# Native Linux/macOS or Windows cross-build; no source tree is patched.
set -euo pipefail
if [[ $# != 2 ]]; then echo "usage: $0 <windows|linux|macos> <new-output-directory>" >&2; exit 2; fi
platform=$1
if [[ "$platform" == macos ]]; then
    export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
fi
case "$platform:$(uname -s)" in
    windows:Linux|linux:Linux|macos:Darwin) ;;
    *) echo "Unsupported build host/target: $platform on $(uname -s)" >&2; exit 2 ;;
esac
script=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
if [[ -e "$2" ]]; then echo "Output already exists: $2" >&2; exit 2; fi
mkdir -p "$2/tools/sources" "$2/tools/build"
output=$(cd "$2" && pwd)
sha256() { if command -v sha256sum >/dev/null; then sha256sum "$@"; else shasum -a 256 "$@"; fi; }
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
exec > >(tee "$output/tools/build/build.log") 2>&1
fetch() {
    local name=$1 url=$2 hash=$3
    curl --fail --location --retry 3 "$url" -o "$output/tools/sources/$name.tar.gz"
    echo "$hash  $output/tools/sources/$name.tar.gz" | sha256 --check -
    printf '%s %s %s\n' "$name" "$hash" "$url" >> "$output/tools/sources/manifest.txt"
    mkdir "$work/$name"
    tar -xzf "$output/tools/sources/$name.tar.gz" --strip-components=1 -C "$work/$name"
}
fetch ffmpeg https://codeload.github.com/FFmpeg/FFmpeg/tar.gz/7070fe638e9e492fb327d277cd0a524d916e7778 53c963a98447e4ff866d343c2b6f74a11857066f3adfc9e2f78b4d030454efc8
if [[ "$platform" != macos ]]; then
fetch nv-codec-headers https://codeload.github.com/FFmpeg/nv-codec-headers/tar.gz/e844e5b26f46bb77479f063029595293aa8f812d 42d72fbcca46e05a1c05a79f1cda1fedd68ba686b21adf917e05d908153645d9
fi
fetch zlib https://codeload.github.com/madler/zlib/tar.gz/51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf d9e270d46252734aa49770fbc544125391617956266f220bd63216c834f3a522
prefix="$work/prefix"
mkdir "$prefix"
export SOURCE_DATE_EPOCH=0
export PKG_CONFIG_LIBDIR="$prefix/lib/pkgconfig"
export PKG_CONFIG_PATH=
jobs=${JOBS:-4}
options=(--prefix="$prefix" --pkg-config=pkg-config --disable-autodetect
    --disable-gpl --disable-nonfree --disable-version3
    --enable-shared --disable-static --disable-debug --disable-doc --disable-ffplay
    --disable-network --enable-zlib --extra-cflags="-I$prefix/include"
    --extra-ldflags="-L$prefix/lib")
cc=cc
case "$platform" in
    windows)
        cc=x86_64-w64-mingw32-gcc
        options+=(--arch=x86_64 --target-os=mingw32 --cross-prefix=x86_64-w64-mingw32-
            --enable-mediafoundation --enable-w32threads --disable-pthreads
            --extra-ldflags=-static-libgcc) ;;
    macos)
        options+=(--enable-videotoolbox --enable-audiotoolbox --install-name-dir=@loader_path) ;;
esac
(
    cd "$work/zlib"
    if [[ "$platform" == windows ]]; then export CHOST=x86_64-w64-mingw32; fi
    CC="$cc" CFLAGS="-O3 -fPIC" ./configure --static --prefix="$prefix"
    make -j"$jobs"
    make install
)
if [[ "$platform" != macos ]]; then
    make -C "$work/nv-codec-headers" PREFIX="$prefix" install
    options+=(--enable-ffnvcodec --enable-nvenc)
fi
cd "$work/ffmpeg"
printf '%q ' ./configure "${options[@]}" > "$output/tools/build/configure-command.txt"
printf '\n' >> "$output/tools/build/configure-command.txt"
./configure "${options[@]}" || {
    cp ffbuild/config.log "$output/tools/build/config.log"
    exit 1
}
grep -q '^#define CONFIG_GPL 0$' config.h
grep -q '^#define CONFIG_NONFREE 0$' config.h
make -j"$jobs"
make install
case "$platform" in
    windows) cp "$prefix"/bin/*.dll "$prefix/bin/ffmpeg.exe" "$prefix/bin/ffprobe.exe" "$output/tools/" ;;
    linux)
        cp "$prefix/bin/ffmpeg" "$prefix/bin/ffprobe" "$output/tools/"
        for library in "$prefix"/lib/*.so.*; do
            if [[ -L "$library" ]]; then cp -L "$library" "$output/tools/"; fi
        done
        for binary in "$output/tools/ffmpeg" "$output/tools/ffprobe" "$output"/tools/*.so.*; do
            patchelf --set-rpath '$ORIGIN' "$binary"
            readelf -d "$binary" >> "$output/tools/build/dynamic-links.txt"
        done ;;
    macos)
        cp "$prefix/bin/ffmpeg" "$prefix/bin/ffprobe" "$output/tools/"
        for library in "$prefix"/lib/*.dylib; do
            # Mach-O install names use the ABI major alias, not the full build version.
            if [[ $(basename "$library") =~ ^lib[^.]+\.[0-9]+\.dylib$ ]]; then
                cp -L "$library" "$output/tools/"
            fi
        done
        for binary in "$output/tools/ffmpeg" "$output/tools/ffprobe" "$output"/tools/*.dylib; do
            codesign --force --sign - "$binary"
            otool -L "$binary" >> "$output/tools/build/dynamic-links.txt"
        done ;;
esac
cp COPYING.LGPLv2.1 "$output/tools/FFMPEG-LICENSE.txt"
cp LICENSE.md "$output/tools/FFMPEG-LICENSE.md"
cp "$work/zlib/LICENSE" "$output/tools/ZLIB-LICENSE.txt"
if [[ "$platform" != macos ]]; then
for header in "$work/nv-codec-headers"/include/ffnvcodec/*.h; do
    basename "$header"
    sed -n '1,/\*\//p' "$header"
done > "$output/tools/NVIDIA-LICENSES.txt"
fi
cp config.h ffbuild/config.mak ffbuild/config.log "$output/tools/build/"
cp "$script" "$output/tools/build/build-ffmpeg.sh"
: > "$output/tools/build/changes.diff"
{
    "$cc" --version
    uname -a
    if command -v nasm >/dev/null; then nasm -v; fi
    make --version
    pkg-config --version
    if [[ "$platform" == macos ]]; then
        xcodebuild -version
        xcrun --show-sdk-version
    else
        dpkg-query -W gcc libc6-dev binutils nasm
        if [[ "$platform" == windows ]]; then
            dpkg-query -W gcc-mingw-w64-x86-64 mingw-w64-common binutils-mingw-w64-x86-64
        else
            dpkg-query -W patchelf
        fi
    fi
} > "$output/tools/build/toolchain.txt"
printf 'bash build/build-ffmpeg.sh %s /absolute/new-output\n' "$platform" > "$output/tools/build/rebuild-command.txt"
cat > "$output/tools/NOTICE.txt" <<'EOF'
This distribution includes FFmpeg (https://ffmpeg.org), LGPL 2.1 or later,
using replaceable shared libraries, and zlib (zlib license). Windows/Linux
also include NVIDIA codec interface headers (MIT license).
Exact unmodified source archives and SHA-256 pins are in sources/.
Build recipe, configuration, toolchain versions and empty source diff are
in build/. See rebuild-command.txt. Linux/Windows builds need gcc, make,
pkg-config, nasm, curl and ca-certificates; Windows additionally needs
mingw-w64, Linux needs patchelf. macOS needs Xcode command line tools,
pkg-config and nasm (on Intel). Build on the target architecture for Unix.
No GPL/nonfree or automatically detected third-party libraries are enabled.
Hardware video encoding requires compatible OS codecs/drivers.
EOF
cd "$output/tools"
if [[ "$platform" == windows ]]; then
    sha256 ./*.exe ./*.dll > build/binary-sha256.txt
elif [[ "$platform" == linux ]]; then
    sha256 ffmpeg ffprobe ./*.so.* > build/binary-sha256.txt
else
    sha256 ffmpeg ffprobe ./*.dylib > build/binary-sha256.txt
fi
echo "Sidecar built: $output"
