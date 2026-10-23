#!/usr/bin/env python3
"""Register a permanently extracted package for the current desktop user."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

APP_ID = "com.rizum.silicate"
OWNER_KEY = "X-RizumSilicate-Package"
DESKTOP = Path("applications") / (APP_ID + ".desktop")
RESOURCES = (
    Path("mime/packages") / (APP_ID + ".xml"),
    Path("icons/hicolor/256x256/apps") / (APP_ID + ".png"),
    Path("icons/hicolor/256x256/mimetypes") / (APP_ID + ".procreate.png"),
)


def desktop_string(value):
    return (value.replace("\\", "\\\\").replace("\n", "\\n")
            .replace("\r", "\\r").replace("\t", "\\t"))


def desktop_exec(path):
    value = str(path)
    # The desktop-entry specification forbids '=' in executable paths.
    if "=" in value or any(character in value for character in "\n\r\t"):
        raise ValueError("Move the package to a path without '=', tabs or newlines")
    for character in ("\\", '"', "`", "$"):
        value = value.replace(character, "\\" + character)
    value = value.replace("%", "%%")
    return desktop_string('"' + value + '"') + " -- %F"


def atomic_write(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as temporary:
        staging = Path(temporary.name)
        temporary.write(content)
    try:
        staging.chmod(0o644)
        os.replace(staging, path)
    finally:
        staging.unlink(missing_ok=True)


def refresh(data_home):
    subprocess.run(["update-mime-database", str(data_home / "mime")], check=True)
    if shutil.which("update-desktop-database"):
        subprocess.run(["update-desktop-database", str(data_home / "applications")], check=True)
    else:
        print("Install desktop-file-utils to refresh the Open With cache immediately.", file=sys.stderr)
    # User icon directories may lack an index.theme; themes inherit these icons.
    # Updating directory mtimes lets icon-theme implementations invalidate caches.
    theme = data_home / "icons/hicolor"
    if theme.is_dir():
        os.utime(theme, None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("install", "uninstall"))
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        raise ValueError("Desktop integration must be run on Linux")
    package = Path(__file__).resolve().parent
    data_home = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
    if not data_home.is_absolute():
        raise ValueError("XDG_DATA_HOME must be an absolute path")
    if not shutil.which("update-mime-database"):
        raise ValueError("Install shared-mime-info (update-mime-database) first")
    owner = OWNER_KEY + "=" + desktop_string(str(package))
    desktop = data_home / DESKTOP
    source = package / "share"
    if args.action == "install":
        binary = package / "silicate"
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise ValueError("The package must contain an executable silicate binary")
        # Read every input before changing registrations, so incomplete packages fail early.
        resources = {path: (source / path).read_bytes() for path in RESOURCES}
        template = (source / DESKTOP).read_text(encoding="utf-8")
        lines = ["Exec=" + desktop_exec(binary) if line.startswith("Exec=") else line
                 for line in template.splitlines()]
        content = ("\n".join(lines) + "\n" + owner + "\n").encode("utf-8")
        for path, data in resources.items():
            atomic_write(data_home / path, data)
        # Publish the launcher last so it never advertises a half-copied resource set.
        atomic_write(desktop, content)
        refresh(data_home)
        print("Installed desktop integration for " + str(package))
        print("Keep this package directory in place; run install again after moving/upgrading it.")
        print("Choose the default application in your desktop's Open With settings.")
    else:
        if not desktop.exists():
            print("No registered desktop entry; nothing removed.")
            return
        if desktop.is_symlink() or owner not in desktop.read_text(encoding="utf-8").splitlines():
            raise ValueError("Registration belongs to another package; nothing removed")
        # Preserve modified/shared resources. Never recursively remove an XDG directory.
        for path in RESOURCES:
            installed = data_home / path
            expected = source / path
            if installed.is_file() and not installed.is_symlink() and expected.is_file():
                if installed.read_bytes() == expected.read_bytes():
                    installed.unlink()
                else:
                    print("Preserved modified resource: " + str(installed), file=sys.stderr)
        desktop.unlink()
        refresh(data_home)
        print("Removed desktop integration. Artwork, settings and package files were preserved.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print("Desktop integration failed: " + str(error), file=sys.stderr)
        sys.exit(1)
