"""Publish a prepared Pax website release to the staging root, never the blog or production."""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import quote


WEBSITE = Path(__file__).resolve().parents[1]
BUCKET = "staging.pax.dev"
DISTRIBUTION = "E29ZMWF6F0HQ61"
MUTABLE = "public, max-age=0, must-revalidate"
IMMUTABLE = "public, max-age=31536000, immutable"
RUNTIME = {
    "pax-interface-web.js", "pax-interface-web.css", "pax-cartridge.js", "pax-cartridge_bg.wasm",
    "__wasm_split.js", "pax-split-loader.js", "split_application.wasm", "pax-fonts.json", "pax-web-config.json",
}
RESERVED = {"www", "build", "288", "_pax"}


def rewrite_entry(html, prefix, key):
    # Match the CLI-generated bootstrap only. Abort before uploading if its contract changes.
    if "pax-interface-web.css" in html.partition("</head>")[0]:
        raise ValueError(f"{key}: entry already references the runtime stylesheet; check its URL before publishing")
    replacements = {
        'src="pax-interface-web.js"': f'src="{prefix}pax-interface-web.js"',
        'new URL("pax-cartridge", document.baseURI)': f'new URL("{prefix}pax-cartridge", document.baseURI)',
        '</head>': f'<link rel="stylesheet" href="{prefix}pax-interface-web.css">\n    </head>',
    }
    for before, after in replacements.items():
        if html.count(before) != 1:
            raise ValueError(f"{key}: expected exactly one {before!r}; check the generated web entry")
        html = html.replace(before, after)
    return html


def prepare(web, work):
    """Snapshot the build into separate upload phases. Never edit the CLI's output."""
    missing = sorted(k for k in RUNTIME | {"index.html", "route-metadata.json"} if not (web / k).is_file())
    if missing:
        raise ValueError(f"Incomplete website release at {web}: missing {', '.join(missing)}")
    catalog = json.loads((web / "route-metadata.json").read_text())
    routes = {"index.html"}
    for route in catalog["routes"]:
        path = route["concrete_path"]
        if path:
            routes.add((Path(path.strip("/")) / "index.html").as_posix())

    entries = []
    public = []
    hashes = []
    for source in sorted(web.rglob("*")):
        key = source.relative_to(web)
        top = key.parts[0]
        if top == "blog" or any(part.startswith(".") for part in key.parts):
            continue  # May contain a draft preview of an independently published site.
        if source.is_symlink() or top in RESERVED or top == "__reloads__":
            raise ValueError(f"Unexpected path in website release: {key}")
        if not source.is_file() or source.name.endswith((".d.ts", ".map")) or source.name == "package.json":
            continue
        name = key.as_posix()
        if name in RUNTIME or top == "snippets":
            group = "runtime"
        elif name in routes:
            group = "entries"
            entries.append(name)
        else:
            if key.suffix == ".html" and 'Pax.init_web' in source.read_text():
                raise ValueError(f"Unrecognized Pax entry: {key}; refusing to publish an unversioned bootstrap")
            group = "public"
            public.append(name)
        dest = work / group / key
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, dest)
        if group == "runtime":
            hashes.append((name, hashlib.sha256(dest.read_bytes()).hexdigest()))

    if routes != set(entries):
        raise ValueError(f"Missing generated route entries: {sorted(routes - set(entries))}")
    # wasm-bindgen may add or rename snippets without changing their import style.
    for module in ("pax-cartridge.js", "__wasm_split.js"):
        for ref in re.findall(r"""\bfrom\s+['"](\./[^'"]+)['"]""", (work / "runtime" / module).read_text()):
            if not (work / "runtime" / ref).is_file():
                raise ValueError(f"{module}: missing runtime import {ref}")
    # Include filenames as well as bytes: a renamed nested Wasm import is a different release.
    release = hashlib.sha256(json.dumps(hashes, separators=(",", ":")).encode()).hexdigest()
    prefix = f"/_pax/releases/{release}/"
    for key in entries:
        entry = work / "entries" / key
        entry.write_text(rewrite_entry(entry.read_text(), prefix, key))
    return prefix, entries, public


def publish(work, prefix, entries, public):
    def aws(*args, capture=False):
        return subprocess.run(["aws", "--profile", "pax", "--region", "us-east-1", *map(str, args)],
                              check=True, text=True, stdout=subprocess.PIPE if capture else None).stdout

    cfg = json.loads(aws("cloudfront", "get-distribution-config", "--id", DISTRIBUTION,
                         "--output", "json", capture=True))["DistributionConfig"]
    origins = cfg["Origins"]["Items"]
    if (BUCKET not in cfg["Aliases"]["Items"] or len(origins) != 1 or origins[0]["OriginPath"] != ""
            or origins[0]["DomainName"] != f"{BUCKET}.s3-website-us-east-1.amazonaws.com"):
        raise ValueError("Staging alias or origin changed; refusing to publish")

    def upload(source, key, cache, *options):
        aws("s3", "cp", source, f"s3://{BUCKET}/{key}", "--acl", "public-read", "--cache-control", cache,
            "--no-follow-symlinks", "--only-show-errors", *options)

    # Old HTML must continue to work during and after a cut. Never sync --delete
    # at the bucket root, or overwrite the old unversioned runtime used by older tabs.
    release_key = prefix.strip("/") + "/"
    runtime = work / "runtime"
    upload(runtime, release_key, IMMUTABLE, "--recursive", "--exclude", "*.wasm")
    for wasm in sorted(runtime.rglob("*.wasm")):
        upload(wasm, release_key + wasm.relative_to(runtime).as_posix(), IMMUTABLE,
               "--content-type", "application/wasm")
    upload(work / "public", "", MUTABLE, "--recursive")
    for key in sorted(entries, key=lambda key: (key == "index.html", key)):
        upload(work / "entries" / key, key, MUTABLE, "--content-type", "text/html; charset=utf-8")

    # CloudFront can separately cache the S3 website directory alias and its index object.
    paths = {"/" + quote(key) for key in entries + public}
    for path in list(paths):
        if path.endswith("/index.html"):
            directory = path[:-len("index.html")]
            paths.update((directory, directory.rstrip("/") or "/"))
    invalidation = aws("cloudfront", "create-invalidation", "--distribution-id", DISTRIBUTION,
                       "--paths", *sorted(paths), "--query", "Invalidation.Id", "--output", "text", capture=True).strip()
    print(f"Waiting for staging invalidation {invalidation}…", flush=True)
    aws("cloudfront", "wait", "invalidation-completed", "--distribution-id", DISTRIBUTION, "--id", invalidation)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--web-dir", type=Path, default=WEBSITE / ".pax/build/release/web",
                        help="prepared release web output (default: this website's release build)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--dry-run", action="store_true", help="prepare and validate offline (default)")
    mode.add_argument("--publish", action="store_true", help="upload to staging and invalidate its mutable URLs")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="pax-staging-") as temp:
        work = Path(temp)
        prefix, entries, public = prepare(args.web_dir, work)
        print(f"Staging runtime: {prefix}\nPax entries: {', '.join(entries)}", flush=True)
        if args.publish:
            publish(work, prefix, entries, public)
        else:
            print("Dry run complete. No AWS calls were made.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error))
