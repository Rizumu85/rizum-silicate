#!/usr/bin/env bash
# Cross-build on Ubuntu 24.04 (also usable in WSL); no source tree is patched.
set -euo pipefail
if [[ $# != 1 ]]; then echo "usage: $0 <new-output-directory>" >&2; exit 2; fi
script=$(realpath "$0")
output=$(realpath -m "$1")
if [[ -e "$output" ]]; then echo "Output already exists: $output" >&2; exit 2; fi
mkdir -p "$output/tools/sources" "$output/tools/build"
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
exec > >(tee "$output/tools/build/build.log") 2>&1
fetch() {
    local name=$1 url=$2 hash=$3
    curl --fail --location --retry 3 "$url" -o "$output/tools/sources/$name.tar.gz"
    echo "$hash  $output/tools/sources/$name.tar.gz" | sha256sum --check -
    printf '%s %s %s\n' "$name" "$hash" "$url" >> "$output/tools/sources/manifest.txt"
    mkdir "$work/$name"
    tar -xzf "$output/tools/sources/$name.tar.gz" --strip-components=1 -C "$work/$name"
}
fetch ffmpeg https://codeload.github.com/FFmpeg/FFmpeg/tar.gz/7070fe638e9e492fb327d277cd0a524d916e7778 53c963a98447e4ff866d343c2b6f74a11857066f3adfc9e2f78b4d030454efc8
fetch nv-codec-headers https://codeload.github.com/FFmpeg/nv-codec-headers/tar.gz/e844e5b26f46bb77479f063029595293aa8f812d 42d72fbcca46e05a1c05a79f1cda1fedd68ba686b21adf917e05d908153645d9
fetch zlib https://codeload.github.com/madler/zlib/tar.gz/51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf d9e270d46252734aa49770fbc544125391617956266f220bd63216c834f3a522
prefix="$work/prefix"
mkdir "$prefix"
export SOURCE_DATE_EPOCH=0
export PKG_CONFIG_LIBDIR="$prefix/lib/pkgconfig"
export PKG_CONFIG_PATH=
jobs=${JOBS:-4}
(
    cd "$work/zlib"
    CHOST=x86_64-w64-mingw32 CC=x86_64-w64-mingw32-gcc ./configure --static --prefix="$prefix"
    make -j"$jobs"
    make install
)
make -C "$work/nv-codec-headers" PREFIX="$prefix" install
cd "$work/ffmpeg"
options=(--prefix="$prefix" --arch=x86_64 --target-os=mingw32
    --cross-prefix=x86_64-w64-mingw32- --pkg-config=pkg-config --disable-autodetect
    --disable-gpl --disable-nonfree --disable-version3
    --enable-shared --disable-static --disable-debug --disable-doc --disable-ffplay
    --disable-network --enable-zlib --enable-ffnvcodec --enable-nvenc
    --enable-mediafoundation --enable-w32threads --disable-pthreads
    --extra-cflags="-I$prefix/include" --extra-ldflags="-L$prefix/lib -static-libgcc")
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
cp "$prefix"/bin/*.dll "$prefix/bin/ffmpeg.exe" "$prefix/bin/ffprobe.exe" "$output/tools/"
cp COPYING.LGPLv2.1 "$output/tools/FFMPEG-LICENSE.txt"
cp LICENSE.md "$output/tools/FFMPEG-LICENSE.md"
cp "$work/zlib/LICENSE" "$output/tools/ZLIB-LICENSE.txt"
for header in "$work/nv-codec-headers"/include/ffnvcodec/*.h; do
    basename "$header"
    sed -n '1,/\*\//p' "$header"
done > "$output/tools/NVIDIA-LICENSES.txt"
cp config.h ffbuild/config.mak ffbuild/config.log "$output/tools/build/"
cp "$script" "$output/tools/build/build-ffmpeg-windows.sh"
: > "$output/tools/build/changes.diff"
{
    x86_64-w64-mingw32-gcc --version
    x86_64-w64-mingw32-ld --version
    nasm -v
    make --version
    pkg-config --version
    dpkg-query -W gcc-mingw-w64-x86-64 mingw-w64-common binutils-mingw-w64-x86-64 nasm
} > "$output/tools/build/toolchain.txt"
cat > "$output/tools/NOTICE.txt" <<'EOF'
This distribution includes FFmpeg (https://ffmpeg.org), LGPL 2.1 or later.
FFmpeg is a separate executable using replaceable FFmpeg DLLs. It includes
zlib (zlib license) and NVIDIA codec interface headers (MIT license).
Exact unmodified source archives and SHA-256 pins are in sources/.
Build instructions, configuration, toolchain versions and empty source diff
are in build/. Rebuild on Ubuntu 24.04 with gcc-mingw-w64-x86-64, nasm,
make, pkg-config, curl and ca-certificates installed:
  bash build/build-ffmpeg-windows.sh /absolute/new-output
No GPL/nonfree components or automatically detected third-party libraries
are enabled. Hardware video encoding requires compatible OS codecs/drivers.
EOF
cd "$output/tools"
sha256sum ./*.exe ./*.dll > build/binary-sha256.txt
echo "Sidecar built: $output"
