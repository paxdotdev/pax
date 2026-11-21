"""Assemble a production website directory locally. Never uploads or includes the blog."""
import argparse
from pathlib import Path
import shutil
import tempfile

from deploy_staging import WEBSITE, prepare


def assemble(web, destination):
    if destination.exists() or destination.is_symlink():
        raise ValueError(f"Choose a new output directory: {destination}")
    if destination.resolve().is_relative_to(web.resolve()):
        raise ValueError("Output must be outside the input build")
    with tempfile.TemporaryDirectory(dir=destination.parent, prefix=".pax-package-") as temp:
        work = Path(temp)
        prefix, _, _ = prepare(web, work)
        bundle = work / "public"
        shutil.copytree(work / "entries", bundle, dirs_exist_ok=True)
        runtime = bundle / prefix.strip("/")
        runtime.parent.mkdir(parents=True, exist_ok=True)
        (work / "runtime").rename(runtime)
        bundle.rename(destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="a new directory outside the web build")
    parser.add_argument("--web-dir", type=Path, default=WEBSITE / ".pax/build/release/web")
    args = parser.parse_args()
    try:
        assemble(args.web_dir, args.output)
    except (OSError, ValueError, KeyError) as error:
        parser.exit(1, f"{error}\n")
