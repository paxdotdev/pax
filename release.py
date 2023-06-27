#!/usr/bin/env python3

# SETUP:
# `pip3 install tomlkit`
# `cargo login`

import os
import subprocess
import sys
import tomlkit
from collections import defaultdict

if len(sys.argv) != 2:
    print("Usage: {} new_version".format(sys.argv[0]))
    sys.exit(1)

NEW_VERSION = sys.argv[1]

PACKAGES = [
    "pax-cartridge",
    "pax-chassis-macos",
    "pax-chassis-web",
    "pax-cli",
    "pax-compiler",
    "pax-core",
    "pax-example",
    "pax-lang",
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

# Perform git commit
subprocess.run(["git", "commit", "-am", "Release " + NEW_VERSION], check=True)

# Set to keep track of already published packages
published = set()

# Second pass to publish the crates
for root in root_packages:
    order = topological_sort(root)

    for elem in order:
        # Only publish the package if it has not been published in this run
        if elem not in published:
            # Run `cargo publish` within the current package directory
            subprocess.run(["cargo", "publish", "--no-verify"], cwd=os.path.join(os.getcwd(), elem), check=True)

            # Mark this package as published
            published.add(elem)