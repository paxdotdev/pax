#!/usr/bin/env python3
"""Freeze an APT pilot transaction without changing the vendor graphics stack."""
import datetime
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def parse_plan(text):
    changes = []
    for line in text.splitlines():
        if line.startswith("Remv "):
            raise ValueError(f"Package removals are forbidden: {line}")
        if not line.startswith("Inst "):
            continue
        match = re.fullmatch(r"Inst (\S+) (?:\[([^\]]+)\] )?\((\S+) .+\)(?: \[[^\]]*\])?", line)
        if not match:
            raise ValueError(f"Unrecognized installation record: {line}")
        package, previous, version = match.groups()
        base = package.split(":")[0]
        if ("nvidia" in base or "cuda" in base or base == "dkms"
                or re.match(r"linux-(image|modules|headers|generic|aws|virtual|tools|firmware)", base)):
            raise ValueError(f"Vendor driver/kernel transaction forbidden: {line}")
        changes.append({"package": package, "previous_version": previous, "version": version})
    return changes


def archive_manifest(transaction, archives):
    expected = {(c["package"].split(":")[0], c["version"]) for c in transaction["changes"]}
    records = []
    for path in sorted(archives.glob("*.deb")):
        values = subprocess.check_output(["dpkg-deb", "-f", str(path), "Package", "Version", "Architecture"], text=True)
        fields = dict(line.split(": ", 1) for line in values.splitlines())
        records.append({"file": path.name, "package": fields["Package"], "version": fields["Version"],
                        "architecture": fields["Architecture"], "bytes": path.stat().st_size,
                        "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    actual = {(r["package"], r["version"]) for r in records}
    if actual != expected:
        raise ValueError(f"Downloaded transaction differs: missing={expected - actual}, extra={actual - expected}")
    return records


def main():
    mode, source, destination = sys.argv[1:]
    if mode == "plan":
        output = {"schema_version": 1, "source": "official-live-ubuntu-resolved-transaction",
                  "snapshot_reproducible": False,
                  "resolved_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  "changes": parse_plan(Path(source).read_text())}
        Path(destination).write_text(json.dumps(output, indent=2) + "\n")
        for change in output["changes"]:
            print(f'{change["package"]}={change["version"]}')
    elif mode == "archive":
        path = Path(source)
        output = json.loads(path.read_text())
        output["archives"] = archive_manifest(output, Path(destination))
        path.write_text(json.dumps(output, indent=2) + "\n")
    else:
        raise ValueError("Expected plan or archive")


if __name__ == "__main__":
    main()
