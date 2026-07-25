#!/usr/bin/env python3

# SETUP:
# `pip3 install tomlkit`
# `cargo login`

import os
import glob
import subprocess
import tomlkit
import time
import argparse
from collections import defaultdict

parser = argparse.ArgumentParser(description='Release Pax crates and release-bound docs')
parser.add_argument('--turbo', action='store_true', help='Enable turbo mode')
parser.add_argument(
    '--skip-docs',
    action='store_true',
    help='Skip docs version manifest updates and docs publishing',
)
parser.add_argument(
    '--skip-docs-publish',
    action='store_true',
    help='Update the docs version manifest, but skip uploading docs to S3/CloudFront',
)
parser.add_argument(
    '--skip-docs-examples',
    action='store_true',
    help='Skip rebuilding runnable example bundles while publishing docs',
)
parser.add_argument(
    '--docs-bucket',
    help='S3 bucket for docs output (default: publish_versioned_docs.py default)',
)
parser.add_argument(
    '--docs-distribution-id',
    help='CloudFront distribution id for docs invalidation',
)
parser.add_argument(
    '--docs-no-latest',
    action='store_true',
    help='Publish versioned docs only, leaving the mutable latest docs at the bucket root untouched',
)
parser.add_argument('new_version', help='The new version string')
args = parser.parse_args()

NEW_VERSION = args.new_version
WORKSPACE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
DOCS_PUBLISHER = os.path.join(
    WORKSPACE_DIR, "pax-docs", "scripts", "publish_versioned_docs.py"
)
# Use NEW_VERSION as needed
print('New version is:', NEW_VERSION)
print('Turbo mode is:', args.turbo)
print('Docs publishing skipped:', args.skip_docs or args.skip_docs_publish)

os.chdir(WORKSPACE_DIR)


PACKAGES = [
    "pax-gpu",
    "pax-chassis-common",
    "pax-chassis-ios",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-designtime",
    "pax-generation",
    "pax-kit",
    "pax-runtime",
    "pax-runtime-api",
    "pax-engine",
    "pax-language-server",
    "pax-macro",
    "pax-manifest",
    "pax-message",
    "pax-std",
    "pax-language",
]


def docs_publish_command(new_version, manifest_only=False):
    cmd_args = [
        DOCS_PUBLISHER,
        new_version,
        "--workspace",
        WORKSPACE_DIR,
    ]

    if manifest_only:
        cmd_args.extend(["--skip-build", "--no-upload"])
        return cmd_args

    if args.skip_docs_examples:
        cmd_args.append("--skip-examples")
    if args.docs_bucket:
        cmd_args.extend(["--bucket", args.docs_bucket])
    if args.docs_distribution_id:
        cmd_args.extend(["--distribution-id", args.docs_distribution_id])
    if args.docs_no_latest:
        cmd_args.append("--no-latest")

    return cmd_args


def update_docs_version_manifest(new_version):
    if args.skip_docs:
        print("Skipping docs version manifest update.")
        return

    subprocess.run(docs_publish_command(new_version, manifest_only=True), check=True)


def publish_docs(new_version):
    if args.skip_docs:
        print("Skipping docs publish.")
        return
    if args.skip_docs_publish:
        print("Skipping docs publish; docs version manifest was still updated.")
        return

    subprocess.run(docs_publish_command(new_version), check=True)


def verify_web_interface_release_artifacts():
    required_paths = {
        "files/interfaces/web/public/pax-interface-web.js",
        "files/interfaces/web/public/pax-interface-web.css",
    }
    package_root = os.path.join(WORKSPACE_DIR, "pax-compiler")
    missing_on_disk = [
        path for path in required_paths
        if not os.path.isfile(os.path.join(package_root, path))
    ]
    if missing_on_disk:
        print("ERROR: missing built web interface artifacts:")
        for path in missing_on_disk:
            print("  " + os.path.join("pax-compiler", path))
        exit(1)

    package_list = subprocess.run(
        ["cargo", "package", "--list", "--allow-dirty"],
        cwd=package_root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.splitlines()
    package_files = set(package_list)
    missing_from_package = sorted(required_paths - package_files)
    if missing_from_package:
        print("ERROR: built web interface artifacts are missing from pax-compiler package:")
        for path in missing_from_package:
            print("  " + path)
        exit(1)


# Compile ts to js and css for the web chassis
original_dir = WORKSPACE_DIR
try:
    target_dir = os.path.join(original_dir, 'pax-compiler', 'files', 'interfaces', 'web')
    os.chdir(target_dir)
    subprocess.run(['./build-interface.sh'], check=True)
except: 
    print("ERROR: failed to build ts files")
    exit(1)

os.chdir(original_dir)
verify_web_interface_release_artifacts()

# Create a mapping from package name to path
PACKAGE_NAMES = {}
for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())
        PACKAGE_NAMES[doc['package']['name']] = elem



def update_crate_versions_in_examples(new_version, package_names, examples_dir):
    """
    Update the crate versions in all Cargo.toml files within the examples directory.
    :param new_version: The new version to set for the crates.
    :param package_names: A set of package names whose versions need to be updated.
    :param examples_dir: Path to the examples directory.
    """
    # Find all Cargo.toml files in the examples/src/**/ directories
    cargo_toml_paths = glob.glob(os.path.join(examples_dir, '**', 'Cargo.toml'), recursive=True)

    for cargo_toml_path in cargo_toml_paths:
        # Read the Cargo.toml file
        with open(cargo_toml_path, 'r') as file:
            doc = tomlkit.parse(file.read())

        doc['package']['version'] = new_version

        # Check and update dependencies
        if 'dependencies' in doc:
            for dep in doc['dependencies']:
                if dep in package_names:
                    dep_table = doc['dependencies'][dep]
                    if isinstance(dep_table, tomlkit.items.InlineTable):
                        dep_table['version'] = new_version

        # Write the updated document back to the file
        with open(cargo_toml_path, 'w') as file:
            file.write(tomlkit.dumps(doc))

# Create a dependency graph
graph = defaultdict(list)
dependency_set = set()
for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())
        for dep in doc['dependencies']:
            if dep in PACKAGE_NAMES:
                graph[elem].append(PACKAGE_NAMES[dep])
                dependency_set.add(PACKAGE_NAMES[dep])

# The root packages are those in the graph keys but not in the dependency set
root_packages = [package for package in graph if package not in dependency_set]

def topological_sort(source):
    visited = set()
    order = []

    def dfs(node):
        visited.add(node)
        for neighbor in graph[node]:
            if neighbor not in visited:
                dfs(neighbor)
        order.insert(0,node)

    dfs(source)
    return order[::-1]


# First pass to update the versions
for root in root_packages:
    order = topological_sort(root)

    for elem in ["."] + order:
        with open("{}/Cargo.toml".format(elem), 'r') as file:
            doc = tomlkit.parse(file.read())

        # If the current version is the same as NEW_VERSION, skip this package
        if doc['package']['version'] == NEW_VERSION:
            continue

        doc['package']['version'] = NEW_VERSION

        if 'dependencies' in doc:
            for dep in doc['dependencies']:
                if dep in PACKAGE_NAMES:
                    dep_table = doc['dependencies'][dep]
                    if isinstance(dep_table, tomlkit.items.InlineTable):
                        dep_table['version'] = NEW_VERSION

        with open("{}/Cargo.toml".format(elem), 'w') as file:
            file.write(tomlkit.dumps(doc))

# Also update the versions in the examples directory
EXAMPLES_DIR = "examples/src"
update_crate_versions_in_examples(NEW_VERSION, PACKAGE_NAMES, EXAMPLES_DIR)

# Also update the docs version manifest, so the release commit records the
# newly published docs version before any S3/CloudFront upload happens.
update_docs_version_manifest(NEW_VERSION)


# Set to keep track of already published packages
published = set()

# Perform git commit
subprocess.run(["git", "commit", "-am", "Release " + NEW_VERSION], check=True)

# Second pass to publish the crates
for root in root_packages:
    order = topological_sort(root)

    for elem in order:
        # Only publish the package if it has not been published in this run
        if elem not in published:
            cmd_args = ["cargo", "publish", "--no-verify", "--allow-dirty"]

            # Run `cargo publish` within the current package directory
            subprocess.run(cmd_args, cwd=os.path.join(os.getcwd(), elem), check=True)
            # Mark this package as published
            published.add(elem)
            # Wait one minute, to satisfy crates.io's throttling mechanism.
            # This can be overridden with the --turbo flag, as we have some burst
            # allowance with crates.io.  Once the burst allowance is used, the publish
            # script may fail with some crates left unpublished, which breaks the entire
            # publish (all crates must be published together.) Thus, turbo is off by default.
            if not args.turbo:
                time.sleep(60)


# Build for macos in order to update Cargo.lock
subprocess.run(['cargo', 'build'])

# Fixup git commit, to include updates to Cargo.lock
subprocess.run(["git", "commit", "-a", "--amend", "--no-edit"], check=True)

# Publish the mutable latest docs plus the immutable /<version>/ snapshot after
# the release commit has been amended into its final state.
publish_docs(NEW_VERSION)

# Perform git tag
# subprocess.run(["git", "tag", "-a", "v" + NEW_VERSION, "-m", "Release v" + NEW_VERSION], check=True)
