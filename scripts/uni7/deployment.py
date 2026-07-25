#!/usr/bin/env python3
"""Fail-closed, read-only helpers for a restartable uni-7 deployment.

This program never signs or broadcasts.  It validates reviewed release artifacts,
RPC status responses, public transaction receipts, and the sanitized deployment
manifest used to resume an operator-led deployment.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import importlib.util
import json
import pathlib
import re
import sys
import urllib.request
from typing import Any

CHAIN_ID = "uni-7"
DENOM = "ujunox"
ARTIFACTS = {
    "binary_market.wasm": {"instantiate", "execute", "reply", "query"},
    "cw_reality.wasm": {"instantiate", "execute", "query", "migrate"},
    "market_factory.wasm": {"instantiate", "execute", "reply", "query"},
}
PINNED_OPTIMIZER = (
    "cosmwasm/optimizer:0.17.0@sha256:"
    "7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85"
)
HEX64 = re.compile(r"^[0-9a-f]{64}$")
TXHASH = re.compile(r"^[0-9A-F]{64}$")
ADDRESS = re.compile(r"^juno1[023456789acdefghjklmnpqrstuvwxyz]{20,90}$")
FORBIDDEN_KEYS = re.compile(r"(mnemonic|private.?key|seed.?phrase|keyring|password|secret|token)", re.I)
EXPECTED_ORACLE = {"admin": None, "min_initial_bond_floor": "10000000", "min_answer_timeout_secs": 86400}
EXPECTED_TIER = {
    "min_initial_liquidity": "100000000", "max_initial_liquidity": "200000000",
    "min_oracle_bounty": "1000000", "max_oracle_bounty": "1000000",
    "oracle_initial_bond": "10000000", "answer_timeout_secs": 86400,
    "arbitration_timeout_secs": 1814400, "fee_bps": 200, "min_trade": "10000",
    "max_trade_bps": 2500, "max_position_per_side": "20000000",
    "collateral_cap": "200000000", "challenge_bond": "10000000",
}


class ValidationError(ValueError):
    pass


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(f"cannot read JSON {path}: {exc}") from exc


def dump_json(path: pathlib.Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_status(value: dict[str, Any]) -> dict[str, Any]:
    node = value.get("result", value)
    info = node.get("node_info") or node.get("NodeInfo") or {}
    sync = node.get("sync_info") or node.get("SyncInfo") or {}
    chain_id = info.get("network")
    catching = sync.get("catching_up")
    if isinstance(catching, str):
        catching = catching.lower() == "true"
    try:
        height = int(sync.get("latest_block_height"))
    except (TypeError, ValueError) as exc:
        raise ValidationError("RPC status has no positive latest block height") from exc
    block_time = sync.get("latest_block_time")
    if chain_id != CHAIN_ID:
        raise ValidationError(f"expected chain-id {CHAIN_ID}, got {chain_id!r}")
    if catching is not False:
        raise ValidationError("RPC is catching up or did not report catching_up=false")
    if height < 1 or not isinstance(block_time, str) or not block_time:
        raise ValidationError("RPC status has invalid height/time")
    return {"chain_id": chain_id, "height": height, "time": block_time, "catching_up": False}


def fetch_status(endpoint: str) -> dict[str, Any]:
    url = endpoint.rstrip("/") + "/status"
    request = urllib.request.Request(url, headers={"User-Agent": "pm-uni7-preflight/1"})
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            return parse_status(json.load(response))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(f"RPC status request failed for {endpoint}: {exc}") from exc


def load_export_validator(root: pathlib.Path):
    # Issue #61 / PR #62 owns this release gate. Deployment deliberately does
    # not carry a weaker parser or silently substitute wasm-opt for that gate.
    path = root / "scripts/release/verify-wasm-exports.py"
    if not path.is_file():
        raise ValidationError(
            "required Wasm entry-point validator is absent; issue #61 / PR #62 "
            "must be reviewed and merged before deployment"
        )
    spec = importlib.util.spec_from_file_location("verify_wasm_exports", path)
    if spec is None or spec.loader is None:
        raise ValidationError(f"cannot load entry-point validator {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_artifacts(root: pathlib.Path, release: pathlib.Path, artifact_dir: pathlib.Path) -> dict[str, Any]:
    manifest = load_json(release)
    if manifest.get("schema_version") != 1:
        raise ValidationError("unsupported release manifest schema")
    image = f"{manifest.get('optimizer_image')}@{manifest.get('optimizer_image_digest')}"
    if image != PINNED_OPTIMIZER:
        raise ValidationError("release was not built by the pinned canonical optimizer")
    source = manifest.get("source_commit")
    if not isinstance(source, str) or not re.fullmatch(r"[0-9a-f]{40}", source):
        raise ValidationError("release source_commit is invalid")
    records = manifest.get("artifacts")
    if not isinstance(records, list) or {r.get("file") for r in records if isinstance(r, dict)} != set(ARTIFACTS):
        raise ValidationError("release must contain exactly the three deployable Wasm artifacts")
    validator = load_export_validator(root)
    result: dict[str, Any] = {}
    hashes: set[str] = set()
    for record in records:
        name = record["file"]
        path = artifact_dir / name
        if not path.is_file() or path.is_symlink():
            raise ValidationError(f"missing or symlinked artifact: {name}")
        digest = sha256(path)
        if digest != record.get("sha256") or path.stat().st_size != record.get("size_bytes"):
            raise ValidationError(f"checksum/size mismatch: {name}")
        if not HEX64.fullmatch(digest) or digest in hashes:
            raise ValidationError(f"invalid or colliding artifact checksum: {name}")
        hashes.add(digest)
        try:
            validator.verify(path, ARTIFACTS[name])
        except (OSError, ValueError) as exc:
            raise ValidationError(str(exc)) from exc
        result[name] = {
            "sha256": digest,
            "size_bytes": path.stat().st_size,
            "required_exports": sorted(ARTIFACTS[name]),
        }
    return {
        "source_commit": source,
        "release_manifest_sha256": sha256(release),
        "optimizer_image": image,
        "artifacts": result,
    }


def _decoded(value: Any) -> str:
    if not isinstance(value, str):
        return str(value)
    # Tendermint's older JSON API base64-encodes event attributes. Decode only
    # printable UTF-8 ending in a known identifier character to avoid changing
    # ordinary modern attributes that happen to be valid base64.
    try:
        raw = base64.b64decode(value, validate=True)
        text = raw.decode()
        if text and all(c.isprintable() for c in text) and any(c in text for c in ("_", "-")):
            return text
    except (ValueError, UnicodeDecodeError):
        pass
    return value


def event_attributes(receipt: dict[str, Any]) -> dict[str, list[str]]:
    events: list[Any] = []
    tx_response = receipt.get("tx_response", receipt)
    events.extend(tx_response.get("events") or [])
    for log in tx_response.get("logs") or []:
        if isinstance(log, dict):
            events.extend(log.get("events") or [])
    found: dict[str, list[str]] = {}
    for event in events:
        if not isinstance(event, dict):
            continue
        for attribute in event.get("attributes") or []:
            if isinstance(attribute, dict):
                key = _decoded(attribute.get("key"))
                value = _decoded(attribute.get("value"))
                found.setdefault(key, []).append(value)
    return found


def parse_receipt(receipt: dict[str, Any], kind: str) -> dict[str, Any]:
    tx = receipt.get("tx_response", receipt)
    try:
        code = int(tx.get("code", 0))
    except (TypeError, ValueError) as exc:
        raise ValidationError("transaction code is invalid") from exc
    if code != 0:
        raise ValidationError(f"transaction failed with code {code}")
    txhash = str(tx.get("txhash", "")).upper()
    if not TXHASH.fullmatch(txhash):
        raise ValidationError("transaction hash is missing or invalid")
    try:
        height = int(tx["height"])
    except (KeyError, TypeError, ValueError) as exc:
        raise ValidationError("transaction height is missing or invalid") from exc
    if height < 1:
        raise ValidationError("transaction height must be positive")
    attrs = event_attributes(receipt)
    result: dict[str, Any] = {"txhash": txhash, "height": height}
    if kind == "store":
        values = attrs.get("code_id", [])
        if len(set(values)) != 1 or not values[0].isdigit() or int(values[0]) < 1:
            raise ValidationError("receipt does not contain one unambiguous code_id")
        result["code_id"] = int(values[0])
    elif kind in {"instantiate", "market"}:
        candidates = attrs.get("_contract_address", []) + attrs.get("contract_address", [])
        candidates = list(dict.fromkeys(candidates))
        if len(candidates) != 1 or not ADDRESS.fullmatch(candidates[0]):
            raise ValidationError("receipt does not contain one valid contract address")
        result["contract_address"] = candidates[0]
        if kind == "market":
            question = list(dict.fromkeys(attrs.get("question_id", [])))
            if len(question) != 1 or not re.fullmatch(r"[0-9a-fA-F]{64}", question[0]):
                raise ValidationError("market receipt does not contain one question_id")
            result["question_id"] = question[0].lower()
    return result


def _walk_public(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if FORBIDDEN_KEYS.search(str(key)):
                raise ValidationError(f"forbidden secret-bearing manifest field at {path}.{key}")
            _walk_public(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _walk_public(child, f"{path}[{index}]")


def validate_manifest(value: dict[str, Any], *, complete: bool = False) -> None:
    _walk_public(value)
    if value.get("schema_version") != 1 or value.get("testnet_only") is not True:
        raise ValidationError("manifest must be schema v1 and testnet_only=true")
    network = value.get("network", {})
    if network.get("chain_id") != CHAIN_ID or network.get("collateral_denom") != DENOM:
        raise ValidationError("manifest network must be uni-7/ujunox")
    if value.get("authorization") != "NOT launch authorization":
        raise ValidationError("manifest authorization warning is missing")
    release = value.get("release", {})
    if release.get("optimizer_image") != PINNED_OPTIMIZER:
        raise ValidationError("manifest canonical optimizer identity is invalid")
    artifacts = value.get("artifacts", {})
    if set(artifacts) != set(ARTIFACTS):
        raise ValidationError("manifest artifact set is incomplete")
    seen: set[str] = set()
    for name, record in artifacts.items():
        digest = record.get("sha256")
        if not isinstance(digest, str) or not HEX64.fullmatch(digest) or digest in seen:
            raise ValidationError(f"invalid/colliding checksum for {name}")
        seen.add(digest)
        if record.get("required_exports") != sorted(ARTIFACTS[name]):
            raise ValidationError(f"required exports changed for {name}")
        code_id = record.get("code_id")
        if code_id is not None and (not isinstance(code_id, int) or isinstance(code_id, bool) or code_id < 1):
            raise ValidationError(f"invalid code_id for {name}")
    if complete:
        observations = value.get("observations", [])
        if len(observations) < 2 or len({item.get("endpoint") for item in observations}) < 2:
            raise ValidationError("complete manifest requires two independent RPC observations")
        for item in observations:
            parse_status({"node_info": {"network": item.get("chain_id")}, "sync_info": {
                "catching_up": item.get("catching_up"), "latest_block_height": item.get("height"),
                "latest_block_time": item.get("time")}})
        for name, record in artifacts.items():
            if record.get("code_id") is None or not TXHASH.fullmatch(str(record.get("store_txhash", ""))):
                raise ValidationError("complete manifest requires every store receipt")
            if record.get("chain_sha256") != record.get("sha256"):
                raise ValidationError(f"on-chain checksum mismatch for {name}")
        contracts = value.get("contracts", {})
        for key in ("oracle", "factory", "seeded_market"):
            record = contracts.get(key, {})
            if not ADDRESS.fullmatch(str(record.get("address", ""))) or not TXHASH.fullmatch(str(record.get("txhash", ""))):
                raise ValidationError(f"complete manifest requires valid {key} readback")
        oracle = contracts["oracle"]
        factory = contracts["factory"]
        market = contracts["seeded_market"]
        if oracle.get("admin", "missing") is not None or factory.get("admin", "missing") is not None:
            raise ValidationError("oracle and factory must have no chain admin")
        if oracle.get("config") != EXPECTED_ORACLE:
            raise ValidationError("oracle config readback does not match accepted canary floors")
        authority = factory.get("verdict_authority")
        if not ADDRESS.fullmatch(str(authority)) or factory.get("verdict_authority_kind") not in {
            "disclosed test-only authority", "verified uni-7 DAO core"}:
            raise ValidationError("factory verdict authority is invalid or not explicitly disclosed")
        if factory.get("tier") != EXPECTED_TIER or factory.get("collateral_denom") != DENOM:
            raise ValidationError("factory tier/denom readback does not match the accepted canary")
        links = (
            (factory.get("code_id"), artifacts["market_factory.wasm"].get("code_id")),
            (factory.get("market_code_id"), artifacts["binary_market.wasm"].get("code_id")),
            (factory.get("oracle_code_id"), artifacts["cw_reality.wasm"].get("code_id")),
            (factory.get("oracle"), oracle.get("address")),
            (market.get("code_id"), artifacts["binary_market.wasm"].get("code_id")),
            (market.get("factory"), factory.get("address")),
            (market.get("oracle"), oracle.get("address")),
            (market.get("verdict_authority"), authority),
            (market.get("collateral_denom"), DENOM),
        )
        if any(actual != expected for actual, expected in links):
            raise ValidationError("contract readback does not match artifact/topology manifest")
        if not re.fullmatch(r"[0-9a-f]{64}", str(market.get("question_id", ""))):
            raise ValidationError("seeded market question identity is invalid")


def command_preflight(args: argparse.Namespace) -> None:
    root = pathlib.Path(__file__).resolve().parents[2]
    reviewed = validate_artifacts(root, args.release_manifest.resolve(), args.artifacts.resolve())
    endpoints = list(dict.fromkeys(args.rpc))
    if len(endpoints) != 2:
        raise ValidationError("provide exactly two distinct RPC endpoints")
    observations = []
    for endpoint in endpoints:
        status = fetch_status(endpoint)
        observations.append({"endpoint": endpoint, **status})
    if abs(observations[0]["height"] - observations[1]["height"]) > args.max_height_drift:
        raise ValidationError("independent RPC heights exceed permitted drift")
    output = {
        "schema_version": 1,
        "testnet_only": True,
        "authorization": "NOT launch authorization",
        "network": {"chain_id": CHAIN_ID, "collateral_denom": DENOM},
        "release": {k: reviewed[k] for k in ("source_commit", "release_manifest_sha256", "optimizer_image")},
        "artifacts": reviewed["artifacts"],
        "observations": observations,
        "contracts": {},
    }
    validate_manifest(output)
    dump_json(args.output, output)
    print(f"wrote sanitized preflight manifest: {args.output}")


def command_parse(args: argparse.Namespace) -> None:
    result = parse_receipt(load_json(args.receipt), args.kind)
    dump_json(args.output, result)
    print(f"wrote sanitized public receipt: {args.output}")


def _record_once(record: dict[str, Any], key: str, value: Any) -> None:
    current = record.get(key)
    if current is not None and current != value:
        raise ValidationError(f"refusing to replace recorded {key}: {current!r} != {value!r}")
    record[key] = value


def command_record(args: argparse.Namespace) -> None:
    manifest = load_json(args.manifest)
    if not isinstance(manifest, dict):
        raise ValidationError("manifest must be an object")
    validate_manifest(manifest)
    if args.target in ARTIFACTS:
        parsed = parse_receipt(load_json(args.receipt), "store")
        record = manifest["artifacts"][args.target]
        _record_once(record, "code_id", parsed["code_id"])
        _record_once(record, "store_txhash", parsed["txhash"])
        _record_once(record, "store_height", parsed["height"])
    else:
        kind = "market" if args.target == "seeded_market" else "instantiate"
        parsed = parse_receipt(load_json(args.receipt), kind)
        contracts = manifest.setdefault("contracts", {})
        record = contracts.setdefault(args.target, {})
        _record_once(record, "address", parsed["contract_address"])
        _record_once(record, "txhash", parsed["txhash"])
        _record_once(record, "height", parsed["height"])
        if kind == "market":
            _record_once(record, "question_id", parsed["question_id"])
        else:
            _record_once(record, "admin", None)
    validate_manifest(manifest)
    dump_json(args.manifest, manifest)
    print(f"recorded sanitized {args.target} receipt in {args.manifest}")


def command_validate(args: argparse.Namespace) -> None:
    value = load_json(args.manifest)
    if not isinstance(value, dict):
        raise ValidationError("manifest must be an object")
    validate_manifest(value, complete=args.complete)
    print("manifest valid")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(required=True)
    preflight = sub.add_parser("preflight", help="read RPC status and validate reviewed artifacts")
    preflight.add_argument("--release-manifest", type=pathlib.Path, required=True)
    preflight.add_argument("--artifacts", type=pathlib.Path, required=True)
    preflight.add_argument("--rpc", action="append", required=True)
    preflight.add_argument("--max-height-drift", type=int, default=20)
    preflight.add_argument("--output", type=pathlib.Path, required=True)
    preflight.set_defaults(func=command_preflight)
    receipt = sub.add_parser("parse-receipt", help="sanitize and parse a public chain receipt")
    receipt.add_argument("--kind", choices=("store", "instantiate", "market"), required=True)
    receipt.add_argument("--receipt", type=pathlib.Path, required=True)
    receipt.add_argument("--output", type=pathlib.Path, required=True)
    receipt.set_defaults(func=command_parse)
    record = sub.add_parser("record-receipt", help="idempotently add a public receipt to a manifest")
    record.add_argument("--target", choices=tuple(ARTIFACTS) + ("oracle", "factory", "seeded_market"), required=True)
    record.add_argument("--receipt", type=pathlib.Path, required=True)
    record.add_argument("--manifest", type=pathlib.Path, required=True)
    record.set_defaults(func=command_record)
    validate = sub.add_parser("validate-manifest")
    validate.add_argument("--manifest", type=pathlib.Path, required=True)
    validate.add_argument("--complete", action="store_true")
    validate.set_defaults(func=command_validate)
    args = parser.parse_args()
    try:
        args.func(args)
    except ValidationError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
