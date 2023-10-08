#!/usr/bin/env python3

# SETUP:
# `pip3 install tomlkit`
# `cargo login`

import os
import subprocess
import tomlkit
import time
import argparse
from collections import defaultdict

parser = argparse.ArgumentParser(description='My Script')
parser.add_argument('--turbo', action='store_true', help='Enable turbo mode')
parser.add_argument('new_version', help='The new version string')
args = parser.parse_args()

NEW_VERSION = args.new_version
# Use NEW_VERSION as needed
print('New version is:', NEW_VERSION)
print('Turbo mode is:', args.turbo)


PACKAGES = [
    "pax-cartridge",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-core",
    "pax-example",
    "pax-lang",
    "pax-language-server",
    "pax-macro",
    "pax-message",
    "pax-properties-coproduct",
    "pax-runtime-api",
    "pax-std",
    "pax-std/pax-std-primitives"
]

# Create a mapping from package name to path
PACKAGE_NAMES = {}
for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())
        PACKAGE_NAMES[doc['package']['name']] = elem

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
        order.append(node)

    dfs(source)
    return order[::-1]

for root in root_packages:
    order = topological_sort(root)
    print(order)

print(graph)

# The output
'''
['pax-chassis-macos', 'pax-cartridge', 'pax-std/pax-std-primitives', 'pax-std', 'pax-lang', 'pax-compiler', 'pax-macro', 'pax-core', 'pax-properties-coproduct', 'pax-runtime-api', 'pax-message']
['pax-chassis-web', 'pax-cartridge', 'pax-std/pax-std-primitives', 'pax-std', 'pax-lang', 'pax-compiler', 'pax-macro', 'pax-core', 'pax-properties-coproduct', 'pax-runtime-api', 'pax-message']
['pax-cli', 'pax-language-server', 'pax-compiler', 'pax-runtime-api', 'pax-message']
['pax-example', 'pax-std', 'pax-lang', 'pax-compiler', 'pax-runtime-api', 'pax-message', 'pax-macro']
'''

# First pass to update the versions
for root in root_packages:
    order = topological_sort(root)

    for elem in order:
        with open("{}/Cargo.toml".format(elem), 'r') as file:
            doc = tomlkit.parse(file.read())

        # If the current version is the same as NEW_VERSION, skip this package
        if doc['package']['version'] == NEW_VERSION:
            continue

        doc['package']['version'] = NEW_VERSION

        for dep in doc['dependencies']:
            if dep in PACKAGE_NAMES:
                dep_table = doc['dependencies'][dep]
                if isinstance(dep_table, tomlkit.items.InlineTable):
                    dep_table['version'] = NEW_VERSION

        with open("{}/Cargo.toml".format(elem), 'w') as file:
            file.write(tomlkit.dumps(doc))


# Set to keep track of already published packages
published = set()

# Perform git commit
subprocess.run(["git", "commit", "-am", "Release " + NEW_VERSION], check=True)

# Second pass to publish the crates
for root in root_packages:
    order = topological_sort(root).reverse()

    for elem in order:
        # Only publish the package if it has not been published in this run
        if elem not in published:
            # Run `cargo publish` within the current package directory
            subprocess.run(["cargo", "publish", "--no-verify"], cwd=os.path.join(os.getcwd(), elem), check=True)
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

# Perform git tag
# subprocess.run(["git", "tag", "-a", "v" + NEW_VERSION, "-m", "Release v" + NEW_VERSION], check=True)
