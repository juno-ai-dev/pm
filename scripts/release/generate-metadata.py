#!/usr/bin/env python3
"""Generate deterministic release provenance and a dependency SBOM."""
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
OUT = pathlib.Path(sys.argv[1]).resolve()


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_hash(paths: list[pathlib.Path]) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths, key=lambda p: p.relative_to(ROOT).as_posix()):
        rel = path.relative_to(ROOT).as_posix().encode()
        digest.update(len(rel).to_bytes(4, "big"))
        digest.update(rel)
        digest.update(bytes.fromhex(sha256(path)))
    return digest.hexdigest()


commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
lock = ROOT / "contracts" / "Cargo.lock"
image = (OUT / "optimizer-image.txt").read_text().strip()
if "@sha256:" not in image:
    raise SystemExit("optimizer image must be pinned by digest")

artifacts = []
for path in sorted((OUT / "artifacts").glob("*.wasm")):
    artifacts.append({"file": path.name, "sha256": sha256(path), "size_bytes": path.stat().st_size})
if not artifacts:
    raise SystemExit("no wasm artifacts found")

schemas = list((ROOT / "contracts").glob("*/schema/**/*.json"))
manifest = {
    "schema_version": 1,
    "source_commit": commit,
    "cargo_lock_sha256": sha256(lock),
    "optimizer_image": image.split("@", 1)[0],
    "optimizer_image_digest": image.split("@", 1)[1],
    "schema_tree_sha256": tree_hash(schemas),
    "artifacts": artifacts,
}
(OUT / "release-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")

lock_data = tomllib.loads(lock.read_text())
components = []
for package in sorted(lock_data.get("package", []), key=lambda p: (p["name"], p["version"], p.get("source", ""))):
    item = {
        "type": "library",
        "name": package["name"],
        "version": package["version"],
        "bom-ref": f"pkg:cargo/{package['name']}@{package['version']}",
    }
    if checksum := package.get("checksum"):
        item["hashes"] = [{"alg": "SHA-256", "content": checksum}]
    components.append(item)
sbom = {
    "bomFormat": "CycloneDX",
    "specVersion": "1.5",
    "version": 1,
    "metadata": {"component": {"type": "application", "name": "juno-pm-contracts", "version": commit}},
    "components": components,
}
(OUT / "sbom.cdx.json").write_text(json.dumps(sbom, indent=2, sort_keys=True) + "\n")
