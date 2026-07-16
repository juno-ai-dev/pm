#!/usr/bin/env python3
"""Fail-closed verification of a cw-reality artifact and frozen deployment."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, NoReturn

TRUSTED_REPOSITORY = "https://github.com/juno-ai-dev/pm.git"
TRUSTED_COMMIT = "454f9777b0bafa71c43b427f7451e626d860269e"
TRUSTED_TREE = "e7cad35d114197fbae3cb0ff9e44ac05d0309bfa"
TRUSTED_CONTRACT_TREE = "cc53d2ea0aa9fcd04fe11ba910b02db11276a0b1"
TRUSTED_CONTRACT_PATH = "contracts/cw-reality"
TRUSTED_OPTIMIZER = "cosmwasm/optimizer:0.17.0@sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85"
TRUSTED_OPTIMIZER_DIGEST = "sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85"
TRUSTED_CHAIN_ID = "juno-1"
TRUSTED_MIN_BOND = "10000000"
TRUSTED_MIN_TIMEOUT = 86400


class VerificationError(Exception):
    pass


def fail(message: str) -> NoReturn:
    raise VerificationError(message)


def load_object(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    return value


def field(obj: dict[str, Any], name: str, expected_type: type, context: str) -> Any:
    if name not in obj or type(obj[name]) is not expected_type:
        fail(f"{context}.{name} must be {expected_type.__name__}")
    return obj[name]


def nullable(obj: dict[str, Any], name: str, context: str) -> None:
    if name not in obj:
        fail(f"{context}.{name} is required")
    if obj[name] is not None:
        fail(f"{context}.{name} must be null")


def require_sha256(value: str, label: str) -> str:
    if len(value) != 64 or any(char not in "0123456789abcdef" for char in value):
        fail(f"{label} must be a lowercase 64-character SHA-256")
    return value


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], text=True, capture_output=True, check=False
    )
    if result.returncode:
        fail(f"source provenance git command failed: {' '.join(args)}: {result.stderr.strip()}")
    return result.stdout.strip()


def verify(manifest: dict[str, Any], evidence: dict[str, Any], artifact: Path, repo: Path) -> list[str]:
    if manifest.get("schema_version") != 1 or evidence.get("schema_version") != 1:
        fail("unsupported schema_version (expected 1)")

    source = field(manifest, "source", dict, "manifest")
    repository = field(source, "repository", str, "manifest.source")
    commit = field(source, "commit", str, "manifest.source")
    tree = field(source, "tree", str, "manifest.source")
    contract_tree = field(source, "contract_tree", str, "manifest.source")
    contract_path = field(source, "contract_path", str, "manifest.source")
    if (repository, commit, tree, contract_tree, contract_path) != (
        TRUSTED_REPOSITORY, TRUSTED_COMMIT, TRUSTED_TREE,
        TRUSTED_CONTRACT_TREE, TRUSTED_CONTRACT_PATH,
    ):
        fail("source provenance does not match the selected immutable policy")
    if len(commit) != 40 or len(tree) != 40 or len(contract_tree) != 40:
        fail("source provenance pins must be full 40-character Git object IDs")
    git(repo, "cat-file", "-e", f"{commit}^{{commit}}")
    if git(repo, "rev-parse", f"{commit}^{{tree}}") != tree:
        fail("source provenance root tree mismatch")
    if git(repo, "rev-parse", f"{commit}:{contract_path}") != contract_tree:
        fail("source provenance contract tree mismatch")
    origin = git(repo, "config", "--get", "remote.origin.url")
    normalized = lambda value: value.removesuffix(".git").removesuffix("/")
    if normalized(origin) != normalized(repository):
        fail("source provenance repository does not match remote.origin.url")

    build = field(manifest, "build", dict, "manifest")
    image = field(build, "optimizer_image", str, "manifest.build")
    digest = field(build, "optimizer_digest", str, "manifest.build")
    if image != TRUSTED_OPTIMIZER or digest != TRUSTED_OPTIMIZER_DIGEST:
        fail("optimizer does not match the selected immutable policy")
    if not digest.startswith("sha256:"):
        fail("optimizer digest must be sha256-pinned")
    require_sha256(digest.removeprefix("sha256:"), "optimizer digest")
    if not image.endswith("@" + digest):
        fail("optimizer image must end with its immutable digest")
    if field(build, "platform", str, "manifest.build") != "linux/amd64":
        fail("optimizer platform must be the selected linux/amd64 production path")
    if build.get("locked") is not True:
        fail("build.locked must be true")
    builds = field(build, "builds", list, "manifest.build")
    if len(builds) != 2 or not all(isinstance(item, dict) for item in builds):
        fail("two-build record must contain exactly two builds")
    builders = [field(item, "builder", str, "manifest.build.builds[]") for item in builds]
    hashes = [require_sha256(field(item, "sha256", str, "manifest.build.builds[]"), "build sha256") for item in builds]
    if builders[0] == builders[1] or hashes[0] != hashes[1]:
        fail("two-build byte comparison requires distinct builders with identical SHA-256")

    artifact_info = field(manifest, "artifact", dict, "manifest")
    expected_hash = require_sha256(field(artifact_info, "sha256", str, "manifest.artifact"), "artifact sha256")
    try:
        artifact_bytes = artifact.read_bytes()
    except OSError as error:
        fail(f"cannot read artifact: {error}")
    actual_hash = hashlib.sha256(artifact_bytes).hexdigest()
    if actual_hash != expected_hash or hashes[0] != expected_hash:
        fail("artifact checksum does not match manifest and two-build records")
    if field(artifact_info, "size", int, "manifest.artifact") != len(artifact_bytes) or not artifact_bytes:
        fail("artifact size mismatch")

    deployment = field(manifest, "deployment", dict, "manifest")
    chain_id = field(deployment, "chain_id", str, "manifest.deployment")
    if chain_id != TRUSTED_CHAIN_ID:
        fail("deployment chain must be juno-1")
    code_id = field(deployment, "code_id", int, "manifest.deployment")
    address = field(deployment, "contract_address", str, "manifest.deployment")
    if code_id <= 0 or not address.startswith("juno1"):
        fail("deployment code ID and contract address must identify a Juno contract")
    nullable(deployment, "chain_admin", "manifest.deployment")
    nullable(deployment, "stored_admin", "manifest.deployment")
    expected_config = field(deployment, "config", dict, "manifest.deployment")
    if expected_config != {
        "min_initial_bond_floor": TRUSTED_MIN_BOND,
        "min_answer_timeout_secs": TRUSTED_MIN_TIMEOUT,
    }:
        fail("deployment config does not match the selected immutable policy")

    if field(evidence, "chain_id", str, "evidence") != chain_id:
        fail("chain ID mismatch")
    observed = field(evidence, "observed", dict, "evidence")
    if field(observed, "height", int, "evidence.observed") <= 0:
        fail("evidence height must be positive")
    field(observed, "endpoint", str, "evidence.observed")
    code_info = field(evidence, "code_info", dict, "evidence")
    if field(code_info, "code_id", int, "evidence.code_info") != code_id:
        fail("code ID mismatch in code info")
    observed_checksum = field(code_info, "checksum", str, "evidence.code_info").lower()
    if observed_checksum != expected_hash:
        fail("on-chain code checksum mismatch")
    contract_info = field(evidence, "contract_info", dict, "evidence")
    if field(contract_info, "address", str, "evidence.contract_info") != address:
        fail("contract address mismatch")
    if field(contract_info, "code_id", int, "evidence.contract_info") != code_id:
        fail("contract code ID mismatch")
    nullable(contract_info, "admin", "chain admin")
    config = field(evidence, "config", dict, "evidence")
    nullable(config, "admin", "stored admin")
    for key in ("min_initial_bond_floor", "min_answer_timeout_secs"):
        if key not in expected_config or config.get(key) != expected_config[key]:
            fail(f"config {key} mismatch")

    audit = field(manifest, "audit", dict, "manifest")
    if audit.get("status") != "accepted":
        fail("deployment verification requires an accepted independent audit")
    if audit.get("audited_commit") != commit or audit.get("audited_sha256") != expected_hash:
        fail("accepted audit must bind the exact source commit and artifact checksum")
    if not isinstance(audit.get("report"), str) or not audit["report"]:
        fail("accepted audit must link a report")

    return [
        "source_provenance", "optimizer_digest", "two_build_byte_identity",
        "artifact_checksum", "chain_id", "code_id", "on_chain_checksum",
        "chain_admin_empty", "stored_admin_empty", "config_exact",
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    try:
        manifest = load_object(args.manifest, "manifest")
        checks = verify(
            manifest,
            load_object(args.evidence, "evidence"),
            args.artifact,
            args.repo_root.resolve(),
        )
    except VerificationError as error:
        print(f"verification failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps({
        "status": "deployment_verified",
        "audit_status": manifest["audit"]["status"],
        "checks": checks,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
