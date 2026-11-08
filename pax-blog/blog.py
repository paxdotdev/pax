#!/usr/bin/env python3
"""Pinned local Zola/Tabi setup and preview. Contains no AWS or publishing commands."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent
LOCK = json.loads((ROOT / "dependencies.json").read_text())
# Explicit deployment closure for SiteHeader. Keep the font license with the
# bytes; other website assets are not used by this embedded instance.
HEADER_ASSETS = ("fonts/Manrope.ttf", "fonts/OFL-Manrope.txt", "fonts/README.md")


def unpack(dependency, destination, strip_root=False):
    with tempfile.TemporaryDirectory(prefix="pax-blog-") as temporary:
        archive = Path(temporary) / "download.tar.gz"
        subprocess.run(["curl", "--fail", "--location", "--silent", "--show-error",
                        dependency["url"], "--output", str(archive)], check=True)
        actual = hashlib.sha256(archive.read_bytes()).hexdigest()
        if actual != dependency["sha256"]:
            raise SystemExit(f"Archive checksum mismatch: expected {dependency['sha256']}, got {actual}")
        staging = Path(temporary) / "unpacked"
        staging.mkdir()
        with tarfile.open(archive) as tar:
            for member in tar.getmembers():
                parts = Path(member.name).parts
                if not parts or member.name.startswith("/") or ".." in parts:
                    raise SystemExit("Unexpected archive path")
                relative = Path(*parts[1:]) if strip_root else Path(*parts)
                output = staging / relative
                if member.isdir():
                    output.mkdir(parents=True, exist_ok=True)
                elif member.isfile():
                    output.parent.mkdir(parents=True, exist_ok=True)
                    with tar.extractfile(member) as source, output.open("wb") as target:
                        shutil.copyfileobj(source, target)
                    output.chmod(member.mode & 0o755)
                else:
                    raise SystemExit("Archive contains a link or special file")
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            raise SystemExit(f"Remove {destination} explicitly before replacing installed dependencies")
        shutil.copytree(staging, destination)
        (destination / ".archive-sha256").write_text(actual + "\n")


def zola():
    candidates = [os.environ.get("ZOLA_BIN"), str(ROOT / ".tools/zola/zola"), shutil.which("zola")]
    for candidate in filter(None, candidates):
        if Path(candidate).is_file():
            version = subprocess.check_output([candidate, "--version"], text=True).strip()
            if version == f"zola {LOCK['zola_version']}":
                return candidate
            if os.environ.get("ZOLA_BIN") == candidate:
                raise SystemExit(f"ZOLA_BIN reports {version}; expected Zola {LOCK['zola_version']}")
    raise SystemExit(f"Install Zola {LOCK['zola_version']} and set ZOLA_BIN, or run `python3 pax-blog/blog.py setup` on Apple Silicon macOS.")


def stage_header(profile):
    """Package the header without changing the caller's build or website assets."""
    source = ROOT / "header/.pax/build" / profile / "web"
    for name in ("pax-interface-web.js", "pax-interface-web.css", "pax-cartridge.js", "pax-cartridge_bg.wasm"):
        if not (source / name).is_file():
            raise SystemExit(f"Missing header bundle file: {source / name}. Build pax-blog/header first.")
    for name in HEADER_ASSETS:
        if not (source / "assets" / name).is_file():
            raise SystemExit(f"Missing header asset: {source / 'assets' / name}. Rebuild pax-blog/header first.")
    destination = ROOT / "static/_pax/header"
    marker = destination / ".pax-blog-header.json"
    if destination.exists():
        if destination.is_symlink() or not marker.is_file():
            raise SystemExit(f"Refusing to replace an unmarked header bundle: {destination}")
        saved = json.loads(marker.read_text())
        if saved.get("generator") != "pax-blog/blog.py":
            raise SystemExit(f"Unexpected header bundle marker: {marker}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    # Validate and optimize in isolation, so a failed minifier or missing asset
    # cannot destroy the previous working preview.
    with tempfile.TemporaryDirectory(prefix=".header-stage-", dir=destination.parent) as temporary:
        staging = Path(temporary) / "header"
        shutil.copytree(source, staging, ignore=lambda directory, names:
                        ["assets"] if Path(directory) == source else [])
        for name in HEADER_ASSETS:
            target = staging / "assets" / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source / "assets" / name, target)
        # Shared settings describe website-root URLs, while this host uses a
        # nested deployment prefix. Preload the same bytes as its Text nodes.
        fonts_path = staging / "pax-fonts.json"
        if fonts_path.is_file():
            fonts = json.loads(fonts_path.read_text())
            for font in fonts:
                url = font.get("url", "")
                if url.startswith("/assets/"):
                    asset = url[len("/assets/"):]
                    if asset not in HEADER_ASSETS:
                        raise SystemExit(f"Header font is not in HEADER_ASSETS: {asset}")
                    font["url"] = "/blog/_pax/header/assets/" + asset
            fonts_path.write_text(json.dumps(fonts) + "\n")
        if profile == "release":
            subprocess.run(["node", str(ROOT / "minify-header.mjs"), str(staging)], check=True)
        (staging / marker.name).write_text(json.dumps({
            "generator": "pax-blog/blog.py", "profile": profile,
            "assets": list(HEADER_ASSETS), "minified": profile == "release",
        }) + "\n")
        if destination.exists():
            shutil.rmtree(destination)
        staging.rename(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["setup", "serve", "build", "preview-build", "stage", "stage-header", "unstage", "check"])
    parser.add_argument("--port", type=int, default=8095)
    parser.add_argument("--profile", choices=["debug", "release"], default="debug", help="Header bundle profile for stage-header")
    args = parser.parse_args()
    if args.command == "stage-header":
        stage_header(args.profile)
        return
    staged = ROOT.parent / "pax-website/public/blog"
    marker = staged / ".pax-blog-preview"
    if args.command == "unstage":
        if staged.exists():
            if staged.is_symlink() or not marker.is_file() or marker.read_text() != "Generated by pax-blog/blog.py stage\n":
                raise SystemExit(f"Refusing to remove an unmarked directory: {staged}")
            shutil.rmtree(staged)
        return
    if args.command == "setup":
        theme = ROOT / "themes/tabi"
        marker = theme / ".archive-sha256"
        if not marker.exists() or marker.read_text().strip() != LOCK["tabi"]["sha256"]:
            unpack(LOCK["tabi"], theme, strip_root=True)
        if not os.environ.get("ZOLA_BIN") and platform.system() == "Darwin" and platform.machine() == "arm64":
            binary = ROOT / ".tools/zola"
            if not binary.exists():
                unpack(LOCK["zola_darwin_arm64"], binary)
        print(f"Ready: {zola()}, Tabi {LOCK['tabi']['version']}")
        return
    if not (ROOT / "themes/tabi/theme.toml").is_file():
        raise SystemExit("Run `python3 pax-blog/blog.py setup` first.")
    if args.command == "build":
        header_marker = ROOT / "static/_pax/header/.pax-blog-header.json"
        if header_marker.is_file() and json.loads(header_marker.read_text()).get("profile") != "release":
            raise SystemExit("Production blog builds require a release header. Rebuild it with --release, then stage-header --profile release.")
    command = [zola(), "--root", str(ROOT)]
    if args.command == "serve":
        command += ["serve", "--drafts", "--interface", "127.0.0.1", "--port", str(args.port),
                    "--base-url", f"http://127.0.0.1:{args.port}/blog", "--no-port-append",
                    "--output-dir", str(ROOT / "preview"), "--force"]
    elif args.command in ("preview-build", "stage"):
        output = ROOT / "preview" if args.command == "preview-build" else staged
        if args.command == "stage" and staged.exists() and (staged.is_symlink() or not marker.is_file()):
            raise SystemExit(f"Refusing to overwrite an unmarked directory: {staged}")
        command += ["build", "--drafts", "--base-url", f"http://127.0.0.1:{args.port}/blog",
                    "--output-dir", str(output), "--force"]
    else:
        command += [args.command]
    subprocess.run(command, check=True)
    if args.command == "stage":
        marker.write_text("Generated by pax-blog/blog.py stage\n")


if __name__ == "__main__":
    main()
