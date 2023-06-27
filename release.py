#!/usr/bin/env python3

# SETUP:
# `pip3 install tomlkit`
# `cargo login`

import os
import subprocess
import sys
import tomlkit

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

# Parse the package name from the Cargo.toml of each package
PACKAGE_NAMES = {}
for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())
        PACKAGE_NAMES[doc['package']['name']] = elem

for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())

    doc['package']['version'] = NEW_VERSION

    for dep in doc['dependencies']:
        if dep in PACKAGE_NAMES:
            dep_table = doc['dependencies'][dep]
            if isinstance(dep_table, tomlkit.items.InlineTable):
                dep_table['version'] = NEW_VERSION

    with open("{}/Cargo.toml".format(elem), 'w') as file:
        file.write(tomlkit.dumps(doc))


# Perform git commit
subprocess.run(["git", "commit", "-am", "Release " + NEW_VERSION], cwd=os.path.join(os.getcwd(), elem), check=True)

# Publish all packages
for elem in PACKAGES:
    # Run `cargo publish` within the current package directory
    subprocess.run(["cargo", "publish", "--no-verify"], cwd=os.path.join(os.getcwd(), elem), check=True)
