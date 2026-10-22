"""Smoke a packaged sidecar in isolation; no PATH-based ffmpeg is used."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("tools", type=Path)
    parser.add_argument("--require-video", action="store_true")
    args = parser.parse_args()
    tools = args.tools.resolve()
    ffmpeg = tools / "ffmpeg.exe"
    ffprobe = tools / "ffprobe.exe"
    env = dict(os.environ)
    env["PATH"] = str(Path(env.get("SystemRoot", "C:/Windows")) / "System32")

    def run(exe, *arguments):
        return subprocess.run([str(exe), *map(str, arguments)], check=True,
                              capture_output=True, text=True, timeout=60, env=env)

    sources = (tools / "sources/manifest.txt").read_text().splitlines()
    assert {row.split()[0] for row in sources} == {"ffmpeg", "zlib", "nv-codec-headers"}
    for row in sources:
        name, digest, _ = row.split()
        actual = hashlib.sha256((tools / f"sources/{name}.tar.gz").read_bytes()).hexdigest()
        assert actual == digest, f"Source checksum mismatch: {name}"
    for row in (tools / "build/binary-sha256.txt").read_text().splitlines():
        digest, filename = row.split()
        assert hashlib.sha256((tools / filename).read_bytes()).hexdigest() == digest
    version = run(ffmpeg, "-hide_banner", "-version").stdout
    assert "--disable-autodetect" in version
    assert "--enable-gpl" not in version and "--enable-nonfree" not in version
    license_text = run(ffmpeg, "-hide_banner", "-L").stdout
    assert "Lesser General Public License" in license_text
    for name in ["FFMPEG-LICENSE.txt", "FFMPEG-LICENSE.md", "ZLIB-LICENSE.txt", "NOTICE.txt",
                 "NVIDIA-LICENSES.txt",
                 "build/configure-command.txt", "build/config.h", "build/config.mak",
                 "build/config.log", "build/toolchain.txt", "build/build-ffmpeg.sh"]:
        assert (tools / name).stat().st_size > 0, name
    assert (tools / "build/changes.diff").read_bytes() == b""
    config = (tools / "build/config.h").read_text()
    for option in ["GPL", "NONFREE", "VERSION3"]:
        assert f"#define CONFIG_{option} 0" in config

    with tempfile.TemporaryDirectory(prefix="silicate-codec-smoke-") as work:
        root = Path(work)
        run(ffmpeg, "-v", "error", "-f", "lavfi", "-i",
            "testsrc=size=256x256:rate=24", "-frames:v", 12, root / "frame-%06d.png")
        for codec, muxer, suffix in [("gif", "gif", "gif"), ("apng", "apng", "png")]:
            target = root / f"output.{suffix}"
            run(ffmpeg, "-v", "error", "-framerate", 24, "-i", root / "frame-%06d.png",
                "-frames:v", 12, "-c:v", codec, "-f", muxer, target)
            packets = json.loads(run(ffprobe, "-v", "error", "-show_entries",
                                    "packet=duration_time", "-of", "json", target).stdout)["packets"]
            assert len(packets) == 12
            assert abs(sum(float(p["duration_time"]) for p in packets) - 0.5) < 0.011
            run(ffmpeg, "-v", "error", "-i", target, "-f", "null", "-")
            print(f"{codec}: 12 frames, 0.5 seconds, decoded")
        # CI may have no hardware encoder. Availability and driver execution are separate gates.
        encoders = run(ffmpeg, "-hide_banner", "-encoders").stdout
        for family in ["h264", "hevc"]:
            candidates = [f"{family}_mf", f"{family}_nvenc"]
            assert all(codec in encoders for codec in candidates), candidates
            errors = []
            for codec in candidates:
                target = root / f"{codec}.mp4"
                try:
                    run(ffmpeg, "-v", "error", "-f", "lavfi", "-i",
                        "color=size=256x256:rate=24", "-frames:v", 12,
                        "-c:v", codec, "-b:v", "12M", target)
                    run(ffmpeg, "-v", "error", "-i", target, "-f", "null", "-")
                    print(f"{codec}: encoded and decoded")
                    break
                except subprocess.CalledProcessError as error:
                    errors.append(error.stderr)
            else:
                if args.require_video:
                    raise RuntimeError("\n".join(errors))
                print(f"{family}: encoder compiled; OS/driver unavailable on this runner")
        # Time-lapse uses concat/remux; exercise that path without hardware requirements.
        segment = root / "segment.mp4"
        run(ffmpeg, "-v", "error", "-f", "lavfi", "-i", "color=size=64x64:rate=24",
            "-frames:v", 12, "-c:v", "mpeg4", segment)
        (root / "concat.txt").write_text("file 'segment.mp4'\nfile 'segment.mp4'\n")
        run(ffmpeg, "-v", "error", "-f", "concat", "-safe", 0, "-i", root / "concat.txt",
            "-c", "copy", root / "merged.mp4")
        run(ffmpeg, "-v", "error", "-i", root / "merged.mp4", "-f", "null", "-")
    print("verification=ffmpeg_sidecar_v1 isolated_path=true sources_verified=true")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        print(error.stderr)
        raise
