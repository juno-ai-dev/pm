#!/usr/bin/env python3
"""Fail unless two isolated release outputs are byte-for-byte equivalent."""
from __future__ import annotations
import hashlib
import json
import pathlib
import sys

one, two = map(pathlib.Path, sys.argv[1:3])
for relative in ("release-manifest.json", "sbom.cdx.json", "optimizer-image.txt"):
    if (one / relative).read_bytes() != (two / relative).read_bytes():
        raise SystemExit(f"release metadata mismatch: {relative}")
manifest = json.loads((one / "release-manifest.json").read_text())
expected = {"binary_market.wasm", "cw_reality.wasm", "market_factory.wasm"}
artifacts = manifest.get("artifacts", [])
names = [artifact.get("file") for artifact in artifacts]
if len(names) != len(set(names)) or set(names) != expected:
    raise SystemExit("release manifest must list every required artifact exactly once")
for artifact in manifest["artifacts"]:
    name = artifact["file"]
    one_bytes = (one / "artifacts" / name).read_bytes()
    two_bytes = (two / "artifacts" / name).read_bytes()
    if one_bytes != two_bytes:
        raise SystemExit(f"artifact mismatch: {name}")
    if artifact.get("size_bytes") != len(one_bytes):
        raise SystemExit(f"manifest size mismatch: {name}")
    if artifact.get("sha256") != hashlib.sha256(one_bytes).hexdigest():
        raise SystemExit(f"manifest checksum mismatch: {name}")
print(f"matched {len(manifest['artifacts'])} artifacts at {manifest['source_commit']}")
