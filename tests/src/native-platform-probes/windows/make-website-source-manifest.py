#!/usr/bin/env python3
"""Freeze the canonical website's authored source tabs for native UI readback.

This recognizes literal pairs, language tuples, and ExampleSource structs in
home_page.rs; it is a fixture catalogue, not a Rust parser. Regenerate when that canonical source list changes.
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
    # Preserve authored order, including the optional language in source tuples.
    pattern = (r'"(?P<pair>[^"\n]+)",\s*(?:"(?:rust|pax|wgsl)",\s*)?'
               r'include_str!\("(?P<pair_path>[^"\n]+)"\)'
               r'|label:\s*"(?P<struct>[^"\n]+)"[^}]*?'
               r'code:\s*include_str!\("(?P<struct_path>[^"\n]+)"\)')
    matches = list(re.finditer(pattern, fragment))
    if len(matches) != fragment.count("include_str!("):
        raise ValueError("Unrecognized authored source include; update the fixture catalogue")
    return [(m["pair"] or m["struct"], m["pair_path"] or m["struct_path"])
            for m in matches]


pairs = source_pairs(text)
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
if len(rows) != 92:
    raise ValueError(f"Canonical website source catalogue changed: expected 92 tabs, found {len(rows)}")
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
     "let mut hero_sources =", "let mut open_source_sources =", 25),
    ("runtime", "runtime_sources", "A PORTABLE RUNTIME", "runtime_section.pax",
     "let mut runtime_sources =", "self.feature_gallery_sources.set", 20),
    ("framework", "framework_sources", "BUILT FOR CREATIVE FREEDOM", "framework_section.pax",
     "self.framework_sources.set", "let mut runtime_sources =", 16),
    ("gallery", "feature_gallery_sources", "A love poem to the pixel", "feature_gallery/gallery.pax",
     "self.feature_gallery_sources.set", "\nfn section_sources(", 8),
    ("open_source", "open_source_sources", "More room for making.", "open_source_section.pax",
     "let mut open_source_sources =", "let mut resource_sources =", 7),
    ("resource", "resource_sources", "PAX IS READY FOR BUILDERS", "resource_section.pax",
     "let mut resource_sources =", "self.framework_sources.set", 16),
]

template = source.with_suffix(".pax")
authored_order = re.findall(r"<ExampleHost\b[^>]*?sources=\{self\.(\w+)\}", template.read_text())
if authored_order != [section[1] for section in sections]:
    raise ValueError(f"Canonical authored section order changed: {authored_order}")
inventory = []
provenance = {source, template, root / "pax-std/src/reference/example_host.pax",
              root / "pax-std/src/reference/example_source_viewer.pax"}
for key, field, anchor, anchor_source, start, end, count in sections:
    section_pairs = source_pairs(fragment(start, end))
    labels = [label for label, _ in section_pairs]
    if len(labels) != count or len(set(labels)) != count:
        raise ValueError(f"Canonical {key} drawer changed: expected {count} distinct tabs")
    anchor_path = source.parent / anchor_source
    if anchor is not None and anchor_path.read_text().count(f'>{anchor}</Text>') != 1:
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
