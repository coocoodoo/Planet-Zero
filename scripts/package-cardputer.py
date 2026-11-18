#!/usr/bin/env python3
"""生成 Cardputer Zero 的 APPLaunch 安装包"""

import os
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent
version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
binary = ROOT / "dist/flight-club-cardputer"
icon = ROOT / "device/icon.png"
if not binary.is_file() or not icon.is_file():
    raise SystemExit("请先运行 bash scripts/build-cardputer.sh，并确认 device/icon.png 存在")

stage = ROOT / f"dist/debian-flight-club-{version}"
if stage.exists():
    shutil.rmtree(stage)
app = stage / "usr/share/APPLaunch"
for directory in [app / "bin", app / "applications", app / "share/images", stage / "DEBIAN", stage / "usr/share/doc/flight-club"]:
    directory.mkdir(parents=True, exist_ok=True)
shutil.copy2(binary, app / "bin/flight-club-cardputer")
(app / "bin/flight-club-cardputer").chmod(0o755)
shutil.copy2(icon, app / "share/images/flight-club.png")
shutil.copy2(ROOT / "device/flight-club.desktop", app / "applications/flight-club.desktop")
shutil.copy2(ROOT / "README.md", stage / "usr/share/doc/flight-club/README.md")
(stage / "DEBIAN/control").write_text(
    f"Package: flight-club\nVersion: {version}\nArchitecture: arm64\n"
    "Maintainer: journey-ad <journey-ad@users.noreply.github.com>\n"
    "Section: games\nPriority: optional\n"
    "Depends: pulseaudio-utils\n"
    "Recommends: pipewire-pulse | pulseaudio\n"
    f"Installed-Size: {(binary.stat().st_size + icon.stat().st_size + 1023) // 1024}\n"
    "Description: Pixel flight arcade for Cardputer Zero\n"
    " Race, coin rush and aerial combat with Rust rendering, RGB565 output and IMU controls\n"
)
package = ROOT / f"dist/flight-club_{version}_arm64.deb"
command = ["dpkg-deb", "--root-owner-group", "--build", str(stage), str(package)]
if not shutil.which("dpkg-deb"):
    command = ["docker", "run", "--rm", "--user", f"{os.getuid()}:{os.getgid()}", "--volume", f"{ROOT}:{ROOT}", "ubuntu:22.04", *command]
subprocess.run(command, check=True)
print(package)
