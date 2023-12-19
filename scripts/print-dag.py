### Prints the Pax monorepo's dependency DAG to stdout
### Usage, from monorepo root: `python3 scripts/print-dag.py`

import tomlkit
from collections import defaultdict

PACKAGES = [
    "pax-cartridge",
    "pax-chassis-common",
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
    "pax-runtime-api",
    "pax-std",
    "pax-std/pax-std-primitives"
    "pax-manifest",
]

# Create a mapping from package name to path
PACKAGE_NAMES = {}
for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())
        PACKAGE_NAMES[doc['package']['name']] = elem

# Create a dependency graph
graph = defaultdict(list)
for elem in PACKAGES:
    with open("{}/Cargo.toml".format(elem), 'r') as file:
        doc = tomlkit.parse(file.read())
        for dep in doc['dependencies']:
            if dep in PACKAGE_NAMES:
                graph[elem].append(PACKAGE_NAMES[dep])

# Recursive function to print dependencies
def print_dependencies(pkg, indent=0):
    print("  " * indent + pkg)
    for dep in graph[pkg]:
        print_dependencies(dep, indent+1)

# Print the dependency DAG
for root in PACKAGES:
    print_dependencies(root)
    print()  # newline for better readability
