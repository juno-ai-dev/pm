#!/usr/bin/env python3
"""Fail unless two isolated release outputs are byte-for-byte equivalent."""
from __future__ import annotations
import json
import pathlib
import sys

one, two = map(pathlib.Path, sys.argv[1:3])
for relative in ("release-manifest.json", "sbom.cdx.json", "optimizer-image.txt"):
    if (one / relative).read_bytes() != (two / relative).read_bytes():
        raise SystemExit(f"release metadata mismatch: {relative}")
manifest = json.loads((one / "release-manifest.json").read_text())
for artifact in manifest["artifacts"]:
    name = artifact["file"]
    if (one / "artifacts" / name).read_bytes() != (two / "artifacts" / name).read_bytes():
        raise SystemExit(f"artifact mismatch: {name}")
print(f"matched {len(manifest['artifacts'])} artifacts at {manifest['source_commit']}")
