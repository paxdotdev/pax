#!/usr/bin/env python3
"""Freeze the canonical website's authored source tabs for native UI readback.

This recognizes the two include_str forms used by home_page.rs; it is a fixture
catalogue, not a Rust parser. Regenerate when that canonical source list changes.
"""
import argparse
from collections import Counter
import hashlib
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("repository", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
root = args.repository.resolve()
source = root / "pax-website/src/home_page.rs"
text = source.read_text()
def source_pairs(fragment):
    return (re.findall(r'"([^"\n]+)",\s*include_str!\("([^"\n]+)"\)', fragment)
            + re.findall(r'label:\s*"([^"\n]+)"[^}]*?code:\s*include_str!\("([^"\n]+)"\)', fragment))

pairs = source_pairs(text)
# These six sources are appended to both authored drawers. The fixture catalogue
# must count rendered tabs, not just textual include_str occurrences.
shared = text.split("for sources in [&self.framework_sources, &self.authoring_sources] {", 1)
if len(shared) != 2:
    raise ValueError("Canonical shared source-drawer loop changed")
shared_pairs = source_pairs(shared[1].split("fn section_sources(", 1)[0])
if len(shared_pairs) != 6:
    raise ValueError("Canonical shared source-drawer list changed")
pairs += shared_pairs
rows = []
seen = {}
for label, relative in pairs:
    path = (source.parent / relative).resolve()
    repository_path = str(path.relative_to(root))
    data = path.read_bytes()
    # Match Rust str::lines followed by the highlighter's LF per source line.
    lines = data.decode("utf-8").split("\n")
    if lines[-1] == "":
        lines.pop()
    display = "".join(line.removesuffix("\r") + "\n" for line in lines)
    row = {"label": label, "repositoryPath": repository_path,
           "sha256": hashlib.sha256(data).hexdigest(),
           "displayCharacters": len(display),
           "displaySha256Utf8": hashlib.sha256(display.encode()).hexdigest()}
    if label in seen and seen[label] != row:
        raise ValueError(f"Conflicting authored source label {label}")
    seen[label] = row
    rows.append(row)
if len(rows) != 72:
    raise ValueError(f"Canonical website source catalogue changed: expected 72 tabs, found {len(rows)}")
# Section membership is part of the acceptance oracle: repeated labels must be
# clicked in each authored drawer, not merely counted anywhere on the page.
# These boundaries intentionally recognize this fixture's source construction;
# fail for changed structure rather than pretending to parse arbitrary Rust.
def fragment(start, end):
    if text.count(start) != 1 or text.count(end) != 1:
        raise ValueError(f"Canonical source-drawer boundary changed: {start}")
    return text.split(start, 1)[1].split(end, 1)[0]


sections = [
    ("hero", "hero_sources", None, "hero_section.pax",
     "let mut hero_sources =", "let mut resource_sources =", 14),
    ("runtime", "runtime_sources", "A PORTABLE RUNTIME", "runtime_section.pax",
     "let mut runtime_sources =", "self.feature_gallery_sources.set", 20),
    ("framework", "framework_sources", "BUILT FOR CREATIVE FREEDOM", "framework_section.pax",
     "self.framework_sources.set", "let mut runtime_sources =", 8),
    ("authoring", "authoring_sources", "THE LANGUAGE / THE LOGIC", "authoring_section.pax",
     "self.authoring_sources.set", "self.framework_sources.set", 8),
    ("gallery", "feature_gallery_sources", "EXPLORE / CURRENT CAPABILITIES", "feature_gallery/gallery.pax",
     "self.feature_gallery_sources.set", "for sources in [&self.framework_sources", 8),
    ("resource", "resource_sources", "PAX IS READY FOR BUILDERS", "resource_section.pax",
     "let mut resource_sources =", "self.authoring_sources.set", 14),
]
template = source.with_suffix(".pax")
authored_order = re.findall(r"<ExampleHost\b[^>]*?sources=\{self\.(\w+)\}", template.read_text())
if authored_order != [section[1] for section in sections]:
    raise ValueError(f"Canonical authored section order changed: {authored_order}")
inventory = []
provenance = {source, template, root / "pax-std/src/reference/example_host.pax"}
for key, field, anchor, anchor_source, start, end, count in sections:
    section_pairs = source_pairs(fragment(start, end))
    if key in ("framework", "authoring"):
        section_pairs += shared_pairs
    labels = [label for label, _ in section_pairs]
    if len(labels) != count or len(set(labels)) != count:
        raise ValueError(f"Canonical {key} drawer changed: expected {count} distinct tabs")
    anchor_path = source.parent / anchor_source
    if anchor is not None and anchor_path.read_text().count(f'text="{anchor}"') != 1:
        raise ValueError(f"Canonical {key} section anchor changed")
    provenance.add(anchor_path)
    inventory.append({"id": key, "anchor": anchor, "sources": [
        {"label": label, "displaySha256Utf8": seen[label]["displaySha256Utf8"]}
        for label in labels
    ]})
if Counter(label for section in inventory for label in (row["label"] for row in section["sources"])) != Counter(row["label"] for row in rows):
    raise ValueError("Section inventory does not preserve the full source occurrence multiset")

# Keep the existing source-array format stable for existing source-readback
# consumers. Bind its additive companion to the exact bytes written here.
source_bytes = (json.dumps(rows, indent=2) + "\n").encode()
section_manifest = {
    "schemaVersion": 1,
    "sourceManifestSha256": hashlib.sha256(source_bytes).hexdigest(),
    "sourceProvenance": [
        {"path": str(path.relative_to(root)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        for path in sorted(provenance)
    ],
    "sections": inventory,
}
args.output.write_bytes(source_bytes)
section_output = args.output.with_suffix(".sections.json")
section_output.write_text(json.dumps(section_manifest, indent=2) + "\n")
print(f"Recorded {len(rows)} source identities and {len(inventory)} sections without copying source contents")
