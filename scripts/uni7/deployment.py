#!/usr/bin/env python3
"""Fail-closed, read-only helpers for a restartable uni-7 deployment.

This program never signs or broadcasts. It validates reviewed artifacts, public
RPC observations and public evidence. It deliberately has no secret inputs.
"""
from __future__ import annotations

import argparse
import base64
import binascii
import datetime
import functools
import hashlib
import importlib.util
import ipaddress
import json
import pathlib
import re
import socket
import subprocess
import sys
import urllib.parse
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
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
TXHASH = re.compile(r"^[0-9A-F]{64}$")
ADDRESS = re.compile(r"^juno1[023456789acdefghjklmnpqrstuvwxyz]{20,90}$")
PROVIDER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 ._-]{0,63}$")
DNS_LABEL = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$")
SPECIAL_DNS_SUFFIXES = (
    "localhost", ".localhost", ".local", ".internal", ".home", ".lan",
    ".invalid", ".test", ".example", ".onion", ".arpa",
)
FORBIDDEN_KEYS = re.compile(
    r"(mnemonic|private.?key|seed.?phrase|keyring|pass(word|phrase)?|secret|"
    r"api.?key|access.?key|bearer|token)", re.I
)
FORBIDDEN_VALUES = re.compile(
    r"(-----BEGIN [A-Z ]*PRIVATE KEY-----|\b(mnemonic|seed[ _-]?phrase|private[ _-]?key|"
    r"password|passphrase|api[ _-]?key|access[ _-]?key|client[ _-]?secret|bearer[ +]|"
    r"auth(orization)?[ :=]|token[ :=]))", re.I
)
EXPECTED_ORACLE = {"admin": None, "min_initial_bond_floor": "10000000", "min_answer_timeout_secs": 86400}
EXPECTED_TIER = {
    "min_initial_liquidity": "100000000", "max_initial_liquidity": "200000000",
    "min_oracle_bounty": "1000000", "max_oracle_bounty": "1000000",
    "oracle_initial_bond": "10000000", "answer_timeout_secs": 86400,
    "arbitration_timeout_secs": 1814400, "fee_bps": 200, "min_trade": "10000",
    "max_trade_bps": 2500, "max_position_per_side": "20000000",
    "collateral_cap": "200000000", "challenge_bond": "10000000",
}
TOP_FIELDS = {"schema_version", "testnet_only", "authorization", "network", "release", "artifacts", "observations", "contracts"}
NETWORK_FIELDS = {"chain_id", "collateral_denom", "max_height_drift"}
RELEASE_FIELDS = {"source_commit", "approved_source_commit", "release_manifest_sha256", "optimizer_image"}
ARTIFACT_BASE_FIELDS = {"sha256", "size_bytes", "required_exports", "receipt_evidence", "chain_evidence"}
OBS_FIELDS = {"provider", "endpoint", "chain_id", "height", "time", "catching_up"}
COMMON_CHAIN_FIELDS = {"provider", "endpoint", "height", "code_id", "chain_sha256", "admin"}
FACTORY_CONFIG_FIELDS = {
    "contract_profile", "protocol_version", "market_code_id", "market_checksum", "tier_id", "tier",
    "oracle", "oracle_code_id", "oracle_checksum", "verdict_authority",
    "collateral_denom", "oracle_min_initial_bond_floor", "oracle_min_answer_timeout_secs",
}
MARKET_CONFIG_FIELDS = {
    "contract_profile", "protocol_version", "factory", "creator", "initial_lp", "oracle",
    "verdict_authority", "tier", "collateral_denom", "close_ts", "opening_ts",
    "initial_liquidity", "oracle_bounty", "oracle_initial_bond", "answer_timeout_secs",
    "arbitration_timeout_secs", "fee_bps", "min_trade", "max_trade_bps",
    "max_position_per_side", "collateral_cap", "challenge_bond",
}
IDENTITY_FIELDS = {
    "contract_profile", "collateral_denom", "protocol_version", "factory", "market", "nonce", "question_id"
}
QUESTION_FIELDS = {"question_id", "question_hash"}
REGISTRY_FIELDS = {
    "contract_profile", "collateral_denom", "nonce", "market", "creator", "tier_id", "question_id",
    "question_hash", "close_ts", "opening_ts", "initial_liquidity", "oracle_bounty", "initial_funding",
    "created_height", "created_time",
}


class ValidationError(ValueError):
    pass


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, child in pairs:
        if key in value:
            raise ValidationError(f"duplicate JSON key: {key}")
        value[key] = child
    return value


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text(), object_pairs_hook=_reject_duplicate_pairs)
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(f"cannot read JSON {path}: {exc}") from exc


def dump_json(path: pathlib.Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _exact(value: Any, fields: set[str], path: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        actual = set(value) if isinstance(value, dict) else set()
        raise ValidationError(f"{path} fields must be exactly {sorted(fields)}; got {sorted(actual)}")
    return value


def _uint(value: Any, path: str, *, positive: bool = False) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < (1 if positive else 0):
        raise ValidationError(f"{path} must be a{' positive' if positive else ' non-negative'} integer")
    return value


def _amount(value: Any, path: str, *, positive: bool = False) -> int:
    if not isinstance(value, str) or not value.isdigit() or (positive and int(value) < 1):
        raise ValidationError(f"{path} must be an unsigned decimal string")
    return int(value)


@functools.lru_cache(maxsize=32)
def _resolve_public_dns(host: str) -> None:
    try:
        answers = socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)
    except socket.gaierror as exc:
        raise ValidationError("RPC endpoint DNS host is not publicly resolvable") from exc
    addresses = {answer[4][0] for answer in answers}
    if not addresses:
        raise ValidationError("RPC endpoint DNS host has no address records")
    for address in addresses:
        try:
            parsed = ipaddress.ip_address(address)
        except ValueError as exc:
            raise ValidationError("RPC endpoint DNS returned a malformed address") from exc
        if not parsed.is_global:
            raise ValidationError("RPC endpoint DNS must resolve only to globally routable addresses")


def normalize_endpoint(endpoint: Any) -> tuple[str, str]:
    if not isinstance(endpoint, str) or endpoint != endpoint.strip() or FORBIDDEN_VALUES.search(endpoint):
        raise ValidationError("RPC endpoint is invalid or secret-bearing")
    try:
        parsed = urllib.parse.urlsplit(endpoint)
        port = parsed.port
    except ValueError as exc:
        raise ValidationError("RPC endpoint is malformed") from exc
    if parsed.scheme.lower() != "https" or not parsed.hostname or parsed.username is not None or parsed.password is not None:
        raise ValidationError("RPC endpoint must be HTTPS with a public host and no userinfo")
    if parsed.query or parsed.fragment:
        raise ValidationError("RPC endpoint must not contain a query or fragment")
    host = parsed.hostname.lower().rstrip(".")
    if not host or any(character.isspace() for character in host):
        raise ValidationError("RPC endpoint host is invalid")
    try:
        ipaddress.ip_address(host)
    except ValueError:
        labels = host.split(".")
        if (
            len(labels) < 2
            or len(host) > 253
            or any(not DNS_LABEL.fullmatch(label) for label in labels)
            or not labels[-1].isalpha()
            or len(labels[-1]) < 2
            or host == "localhost"
            or host.endswith(SPECIAL_DNS_SUFFIXES)
        ):
            raise ValidationError("RPC endpoint must use a globally routable DNS provider host")
    else:
        raise ValidationError("RPC endpoint must use a DNS provider host, not an IP literal")
    _resolve_public_dns(host)
    netloc = host if port in (None, 443) else f"{host}:{port}"
    path = parsed.path.rstrip("/")
    normalized = urllib.parse.urlunsplit(("https", netloc, path, "", ""))
    return normalized, host


def _provider(value: Any) -> str:
    if not isinstance(value, str) or not PROVIDER.fullmatch(value) or FORBIDDEN_VALUES.search(value):
        raise ValidationError("provider label is invalid or secret-bearing")
    return value


def _walk_public(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if FORBIDDEN_KEYS.search(str(key)):
                raise ValidationError(f"forbidden secret-bearing manifest field at {path}.{key}")
            _walk_public(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _walk_public(child, f"{path}[{index}]")
    elif isinstance(value, str):
        if FORBIDDEN_VALUES.search(value):
            raise ValidationError(f"forbidden secret-bearing manifest value at {path}")
        # URLs anywhere in evidence must be persistable public URLs, never credential/query URLs.
        if re.match(r"(?i)^https?://", value):
            parsed = urllib.parse.urlsplit(value)
            if parsed.username is not None or parsed.password is not None or parsed.query or parsed.fragment:
                raise ValidationError(f"unsafe URL value at {path}")


def parse_status(value: dict[str, Any]) -> dict[str, Any]:
    node = value.get("result", value)
    if not isinstance(node, dict):
        raise ValidationError("RPC status must be an object")
    info = node.get("node_info") or node.get("NodeInfo") or {}
    sync = node.get("sync_info") or node.get("SyncInfo") or {}
    chain_id = info.get("network") if isinstance(info, dict) else None
    catching = sync.get("catching_up") if isinstance(sync, dict) else None
    if isinstance(catching, str) and catching in ("true", "false"):
        catching = catching == "true"
    raw_height = sync.get("latest_block_height") if isinstance(sync, dict) else None
    if isinstance(raw_height, bool) or not isinstance(raw_height, (str, int)) or not str(raw_height).isdigit():
        raise ValidationError("RPC status has no positive latest block height")
    height = int(raw_height)
    block_time = sync.get("latest_block_time") if isinstance(sync, dict) else None
    if chain_id != CHAIN_ID:
        raise ValidationError(f"expected chain-id {CHAIN_ID}, got {chain_id!r}")
    if catching is not False:
        raise ValidationError("RPC is catching up or did not report catching_up=false")
    if height < 1 or not isinstance(block_time, str):
        raise ValidationError("RPC status has invalid height/time")
    try:
        parsed_time = datetime.datetime.fromisoformat(block_time.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValidationError("RPC status time is not RFC3339") from exc
    if parsed_time.tzinfo is None:
        raise ValidationError("RPC status time must include a timezone")
    return {"chain_id": chain_id, "height": height, "time": block_time, "catching_up": False}


def fetch_status(endpoint: str) -> dict[str, Any]:
    url = endpoint.rstrip("/") + "/status"
    request = urllib.request.Request(url, headers={"User-Agent": "pm-uni7-preflight/1"})
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            return parse_status(json.load(response, object_pairs_hook=_reject_duplicate_pairs))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError(f"RPC status request failed for {endpoint}: {exc}") from exc


def load_export_validator(root: pathlib.Path):
    path = root / "scripts/release/verify-wasm-exports.py"
    if not path.is_file():
        raise ValidationError("required reviewed Wasm entry-point validator is absent (issue #61 / PR #62)")
    spec = importlib.util.spec_from_file_location("verify_wasm_exports", path)
    if spec is None or spec.loader is None:
        raise ValidationError(f"cannot load entry-point validator {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def verify_local_commit(root: pathlib.Path, source: str) -> None:
    if not HEX40.fullmatch(source):
        raise ValidationError("approved source commit must be a full lowercase 40-hex object ID")
    result = subprocess.run(
        ["git", "cat-file", "-e", f"{source}^{{commit}}"], cwd=root,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False,
    )
    if result.returncode != 0:
        raise ValidationError("approved source commit does not exist locally as a commit object")


def validate_artifacts(root: pathlib.Path, release: pathlib.Path, artifact_dir: pathlib.Path, approved_source: str) -> dict[str, Any]:
    manifest = load_json(release)
    if not isinstance(manifest, dict) or manifest.get("schema_version") != 1:
        raise ValidationError("unsupported release manifest schema")
    image = f"{manifest.get('optimizer_image')}@{manifest.get('optimizer_image_digest')}"
    if image != PINNED_OPTIMIZER:
        raise ValidationError("release was not built by the pinned canonical optimizer")
    source = manifest.get("source_commit")
    if source != approved_source:
        raise ValidationError("explicitly approved source commit does not equal release manifest source_commit")
    verify_local_commit(root, approved_source)
    records = manifest.get("artifacts")
    if not isinstance(records, list) or len(records) != len(ARTIFACTS) or any(not isinstance(r, dict) for r in records):
        raise ValidationError("release must contain exactly three artifact records")
    names = [record.get("file") for record in records]
    if len(set(names)) != len(names) or set(names) != set(ARTIFACTS):
        raise ValidationError("release must name each deployable Wasm artifact exactly once")
    if not artifact_dir.is_dir() or artifact_dir.is_symlink():
        raise ValidationError("artifact root must be a real directory")
    entries = list(artifact_dir.iterdir())
    if {entry.name for entry in entries} != set(ARTIFACTS):
        raise ValidationError("artifact root must contain exactly the three reviewed Wasm files and no extras")
    if any(not entry.is_file() or entry.is_symlink() for entry in entries):
        raise ValidationError("every artifact-root entry must be an exact regular file")
    validator = load_export_validator(root)
    result: dict[str, Any] = {}
    hashes: set[str] = set()
    for record in records:
        name = record["file"]
        path = artifact_dir / name
        digest = sha256(path)
        size = path.stat().st_size
        if digest != record.get("sha256") or size != record.get("size_bytes"):
            raise ValidationError(f"checksum/size mismatch: {name}")
        if digest in hashes:
            raise ValidationError(f"colliding artifact checksum: {name}")
        hashes.add(digest)
        try:
            validator.verify(path, ARTIFACTS[name])
        except (OSError, ValueError) as exc:
            raise ValidationError(str(exc)) from exc
        result[name] = {"sha256": digest, "size_bytes": size,
                        "required_exports": sorted(ARTIFACTS[name]),
                        "receipt_evidence": [], "chain_evidence": []}
    return {"source_commit": source, "approved_source_commit": approved_source,
            "release_manifest_sha256": sha256(release), "optimizer_image": image,
            "artifacts": result}


EVENT_KEYS = {"code_id", "_contract_address", "contract_address", "market", "question_id"}


def _base64_text(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    try:
        raw = base64.b64decode(value, validate=True)
        text = raw.decode()
    except (ValueError, UnicodeDecodeError):
        return None
    return text if text and all(character.isprintable() for character in text) else None


def event_attributes(receipt: dict[str, Any]) -> dict[str, list[str]]:
    events: list[Any] = []
    tx_response = receipt.get("tx_response", receipt)
    if not isinstance(tx_response, dict):
        return {}
    raw_events = tx_response.get("events") or []
    if isinstance(raw_events, list):
        events.extend(raw_events)
    logs = tx_response.get("logs") or []
    if isinstance(logs, list):
        for log in logs:
            if isinstance(log, dict) and isinstance(log.get("events") or [], list):
                events.extend(log.get("events") or [])
    found: dict[str, list[str]] = {}
    for event in events:
        if not isinstance(event, dict) or not isinstance(event.get("attributes") or [], list):
            continue
        for attribute in event.get("attributes") or []:
            if not isinstance(attribute, dict):
                continue
            raw_key = attribute.get("key")
            decoded_key = _base64_text(raw_key)
            encoded = decoded_key in EVENT_KEYS
            key = decoded_key if encoded else str(raw_key)
            raw_value = attribute.get("value")
            decoded_value = _base64_text(raw_value) if encoded else None
            value = decoded_value if decoded_value is not None else str(raw_value)
            found.setdefault(key, []).append(value)
    return found


def _question_id_hex(value: str) -> str:
    if re.fullmatch(r"[0-9a-fA-F]{64}", value):
        return value.lower()
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as exc:
        raise ValidationError("question_id is neither 32-byte hex nor canonical base64") from exc
    if len(raw) != 32 or base64.b64encode(raw).decode("ascii") != value:
        raise ValidationError("question_id is neither 32-byte hex nor canonical base64")
    return raw.hex()


def parse_receipt(receipt: dict[str, Any], kind: str) -> dict[str, Any]:
    tx = receipt.get("tx_response", receipt)
    if not isinstance(tx, dict):
        raise ValidationError("transaction receipt must be an object")
    code = tx.get("code")
    if not isinstance(code, int) or isinstance(code, bool) or code != 0:
        raise ValidationError("transaction code must be explicitly present as numeric 0")
    txhash = tx.get("txhash")
    if not isinstance(txhash, str):
        raise ValidationError("transaction hash is missing or invalid")
    txhash = txhash.upper()
    if not TXHASH.fullmatch(txhash):
        raise ValidationError("transaction hash is missing or invalid")
    raw_height = tx.get("height")
    if isinstance(raw_height, bool) or not isinstance(raw_height, (str, int)) or not str(raw_height).isdigit():
        raise ValidationError("transaction height is missing or invalid")
    height = int(raw_height)
    if height < 1:
        raise ValidationError("transaction height must be positive")
    attrs = event_attributes(receipt)
    result: dict[str, Any] = {"txhash": txhash, "height": height}
    if kind == "store":
        values = attrs.get("code_id", [])
        if not values or len(set(values)) != 1 or not values[0].isdigit() or int(values[0]) < 1:
            raise ValidationError("receipt does not contain one unambiguous code_id")
        result["code_id"] = int(values[0])
    elif kind in {"instantiate", "market"}:
        address_values = attrs.get("market", []) if kind == "market" else \
            attrs.get("_contract_address", []) + attrs.get("contract_address", [])
        candidates = list(dict.fromkeys(address_values))
        if len(candidates) != 1 or not ADDRESS.fullmatch(candidates[0]):
            raise ValidationError(f"receipt does not contain one valid {kind} address")
        result["address"] = candidates[0]
        if kind == "market":
            raw_questions = attrs.get("question_id", [])
            if not raw_questions:
                raise ValidationError("market receipt does not contain a question_id")
            questions = {_question_id_hex(value) for value in raw_questions}
            if len(questions) != 1:
                raise ValidationError("market receipt contains conflicting question_id values")
            result["question_id"] = questions.pop()
    else:
        raise ValidationError("unknown receipt kind")
    return result


def _validate_provenance(item: dict[str, Any], path: str) -> tuple[str, str]:
    provider = _provider(item.get("provider"))
    endpoint, host = normalize_endpoint(item.get("endpoint"))
    if item["endpoint"] != endpoint:
        raise ValidationError(f"{path}.endpoint must be normalized as {endpoint}")
    return provider, host


def _validate_receipts(items: Any, kind: str, path: str) -> None:
    if not isinstance(items, list) or len(items) > 2:
        raise ValidationError(f"{path} must be an array of at most two receipt observations")
    fields = {"provider", "endpoint", "txhash", "height", "code_id"} if kind == "store" else \
             {"provider", "endpoint", "txhash", "height", "address"} | ({"question_id"} if kind == "market" else set())
    identities: set[tuple[str, str]] = set()
    for index, item in enumerate(items):
        item = _exact(item, fields, f"{path}[{index}]")
        provider, host = _validate_provenance(item, f"{path}[{index}]")
        if (provider.casefold(), host) in identities:
            raise ValidationError(f"duplicate provider receipt at {path}")
        identities.add((provider.casefold(), host))
        if not isinstance(item.get("txhash"), str) or not TXHASH.fullmatch(item["txhash"]):
            raise ValidationError(f"invalid txhash at {path}[{index}]")
        _uint(item.get("height"), f"{path}[{index}].height", positive=True)
        if kind == "store":
            _uint(item.get("code_id"), f"{path}[{index}].code_id", positive=True)
        else:
            if not isinstance(item.get("address"), str) or not ADDRESS.fullmatch(item["address"]):
                raise ValidationError(f"invalid address at {path}[{index}]")
            if kind == "market" and (not isinstance(item.get("question_id"), str) or not HEX64.fullmatch(item["question_id"])):
                raise ValidationError(f"invalid question_id at {path}[{index}]")
    if len(items) == 2:
        comparable = fields - {"provider", "endpoint"}
        if any(items[0][key] != items[1][key] for key in comparable):
            raise ValidationError(f"dual-provider receipts disagree at {path}")
        if len({normalize_endpoint(item["endpoint"])[1] for item in items}) != 2 or len({item["provider"].casefold() for item in items}) != 2:
            raise ValidationError(f"receipt evidence must use distinct providers and hosts at {path}")


def _validate_chain_base(item: dict[str, Any], path: str) -> None:
    _validate_provenance(item, path)
    _uint(item.get("height"), f"{path}.height", positive=True)
    _uint(item.get("code_id"), f"{path}.code_id", positive=True)
    if not isinstance(item.get("chain_sha256"), str) or not HEX64.fullmatch(item["chain_sha256"]):
        raise ValidationError(f"invalid chain checksum at {path}")
    if item.get("admin", "missing") is not None:
        raise ValidationError(f"independently queried chain admin must be null at {path}")


def _validate_artifact(name: str, record: Any) -> None:
    record = _exact(record, ARTIFACT_BASE_FIELDS, f"artifacts.{name}")
    digest = record["sha256"]
    if not isinstance(digest, str) or not HEX64.fullmatch(digest):
        raise ValidationError(f"invalid checksum for {name}")
    _uint(record["size_bytes"], f"artifacts.{name}.size_bytes", positive=True)
    if record["required_exports"] != sorted(ARTIFACTS[name]):
        raise ValidationError(f"required exports changed for {name}")
    _validate_receipts(record["receipt_evidence"], "store", f"artifacts.{name}.receipt_evidence")
    evidence = record["chain_evidence"]
    if not isinstance(evidence, list) or len(evidence) > 2:
        raise ValidationError(f"artifacts.{name}.chain_evidence must have at most two observations")
    for index, item in enumerate(evidence):
        item = _exact(item, COMMON_CHAIN_FIELDS, f"artifacts.{name}.chain_evidence[{index}]")
        _validate_chain_base(item, f"artifacts.{name}.chain_evidence[{index}]")
        if item["chain_sha256"] != digest:
            raise ValidationError(f"on-chain checksum mismatch for {name}")
        if record["receipt_evidence"] and item["code_id"] != record["receipt_evidence"][0]["code_id"]:
            raise ValidationError(f"on-chain code ID mismatch for {name}")


def _validate_oracle_config(config: Any, path: str) -> None:
    _exact(config, set(EXPECTED_ORACLE), path)
    if config != EXPECTED_ORACLE:
        raise ValidationError("oracle config does not match accepted canary floors")


def _validate_factory_config(
    config: Any,
    artifacts: dict[str, Any],
    oracle: str,
    authority_kind: Any,
    path: str,
) -> None:
    config = _exact(config, FACTORY_CONFIG_FIELDS, path)
    authority = config["verdict_authority"]
    if (
        not isinstance(authority, str)
        or not ADDRESS.fullmatch(authority)
        or authority_kind not in {"disclosed test-only authority", "verified uni-7 DAO core"}
    ):
        raise ValidationError("factory verdict authority is invalid or undisclosed")
    expected = {
        "contract_profile": "uni7", "protocol_version": "v1",
        "market_code_id": artifacts["binary_market.wasm"]["receipt_evidence"][0]["code_id"],
        "market_checksum": artifacts["binary_market.wasm"]["sha256"], "tier_id": 1,
        "tier": EXPECTED_TIER, "oracle": oracle,
        "oracle_code_id": artifacts["cw_reality.wasm"]["receipt_evidence"][0]["code_id"],
        "oracle_checksum": artifacts["cw_reality.wasm"]["sha256"],
        "verdict_authority": authority,
        "collateral_denom": DENOM, "oracle_min_initial_bond_floor": "10000000",
        "oracle_min_answer_timeout_secs": 86400,
    }
    if config != expected:
        raise ValidationError("factory config does not exactly match PR #66 uni7 schema/canary")


def _validate_market_readback(item: dict[str, Any], artifacts: dict[str, Any], factory: str, oracle: str, authority: str, address: str, path: str) -> None:
    config = _exact(item["config"], MARKET_CONFIG_FIELDS, f"{path}.config")
    identity = _exact(item["identity"], IDENTITY_FIELDS, f"{path}.identity")
    question = _exact(item["question"], QUESTION_FIELDS, f"{path}.question")
    registry = _exact(item["registry"], REGISTRY_FIELDS, f"{path}.registry")
    if config["contract_profile"] != "uni7" or config["protocol_version"] != "v1" or config["collateral_denom"] != DENOM:
        raise ValidationError("market profile/version/denom mismatch")
    creator = config["creator"]
    if not isinstance(creator, str) or not ADDRESS.fullmatch(creator) or config["initial_lp"] != creator:
        raise ValidationError("market creator/initial LP is invalid")
    if config["factory"] != factory or config["oracle"] != oracle or config["verdict_authority"] != authority or config["tier"] != 1:
        raise ValidationError("market topology/tier mismatch")
    market_tier = {key: value for key, value in EXPECTED_TIER.items() if key in MARKET_CONFIG_FIELDS}
    tier_links = {key: config[key] for key in market_tier}
    if tier_links != market_tier:
        raise ValidationError("market tier fields do not match factory tier")
    if not all(isinstance(config[key], int) and not isinstance(config[key], bool) for key in ("close_ts", "opening_ts")):
        raise ValidationError("market timestamps must be integers")
    if config["opening_ts"] < config["close_ts"]:
        raise ValidationError("market opening timestamp precedes close timestamp")
    _amount(config["initial_liquidity"], f"{path}.config.initial_liquidity", positive=True)
    _amount(config["oracle_bounty"], f"{path}.config.oracle_bounty", positive=True)
    if config["initial_liquidity"] != "100000000" or config["oracle_bounty"] != "1000000":
        raise ValidationError("seeded market funding does not match canary")
    if (
        identity["contract_profile"] != "uni7"
        or identity["collateral_denom"] != DENOM
        or identity["protocol_version"] != "v1"
        or identity["factory"] != factory
        or identity["market"] != address
    ):
        raise ValidationError("market identity profile/denom/topology mismatch")
    _uint(identity["nonce"], f"{path}.identity.nonce")
    for key in ("question_id", "question_hash"):
        if not isinstance(question[key], str) or not HEX64.fullmatch(question[key]):
            raise ValidationError(f"invalid market {key}")
    if identity["question_id"] != question["question_id"]:
        raise ValidationError("market identity/question query disagree")
    expected_registry = {
        "contract_profile": "uni7", "collateral_denom": DENOM,
        "nonce": identity["nonce"], "market": address, "creator": creator, "tier_id": 1,
        "question_id": question["question_id"], "question_hash": question["question_hash"],
        "close_ts": config["close_ts"], "opening_ts": config["opening_ts"],
        "initial_liquidity": config["initial_liquidity"], "oracle_bounty": config["oracle_bounty"],
        "initial_funding": str(int(config["initial_liquidity"]) + int(config["oracle_bounty"])),
        "created_height": registry["created_height"], "created_time": registry["created_time"],
    }
    _uint(registry["created_height"], f"{path}.registry.created_height", positive=True)
    _uint(registry["created_time"], f"{path}.registry.created_time", positive=True)
    if registry != expected_registry:
        raise ValidationError("factory registry and market identity/config/question do not exactly cross-link")
    if item["code_id"] != artifacts["binary_market.wasm"]["receipt_evidence"][0]["code_id"] or item["chain_sha256"] != artifacts["binary_market.wasm"]["sha256"]:
        raise ValidationError("market code ID/checksum is not bound to binary-market artifact")


def _validate_contracts(contracts: Any, artifacts: dict[str, Any], *, complete: bool) -> None:
    if not isinstance(contracts, dict) or not set(contracts).issubset({"oracle", "factory", "seeded_market"}):
        raise ValidationError("contracts may contain only oracle, factory, and seeded_market")
    if complete and set(contracts) != {"oracle", "factory", "seeded_market"}:
        raise ValidationError("complete manifest requires all three contract records")
    for name, record in contracts.items():
        record_fields = {"receipt_evidence", "readback_evidence"}
        if name == "factory":
            record_fields.add("authority_kind")
        record = _exact(record, record_fields, f"contracts.{name}")
        kind = "market" if name == "seeded_market" else "instantiate"
        _validate_receipts(record["receipt_evidence"], kind, f"contracts.{name}.receipt_evidence")
        readbacks = record["readback_evidence"]
        if not isinstance(readbacks, list) or len(readbacks) > 2:
            raise ValidationError(f"contracts.{name}.readback_evidence must have at most two observations")
        extra = {"config"} if name in ("oracle", "factory") else {"config", "identity", "question", "registry"}
        for index, item in enumerate(readbacks):
            item = _exact(item, COMMON_CHAIN_FIELDS | extra, f"contracts.{name}.readback_evidence[{index}]")
            _validate_chain_base(item, f"contracts.{name}.readback_evidence[{index}]")
    if not complete:
        return
    for name, record in contracts.items():
        if len(record["receipt_evidence"]) != 2 or len(record["readback_evidence"]) != 2:
            raise ValidationError(f"complete manifest requires dual receipt and readback evidence for {name}")
    oracle_address = contracts["oracle"]["receipt_evidence"][0]["address"]
    factory_address = contracts["factory"]["receipt_evidence"][0]["address"]
    market_address = contracts["seeded_market"]["receipt_evidence"][0]["address"]
    for item in contracts["oracle"]["readback_evidence"]:
        _validate_oracle_config(item["config"], "oracle.config")
        if item["code_id"] != artifacts["cw_reality.wasm"]["receipt_evidence"][0]["code_id"] or item["chain_sha256"] != artifacts["cw_reality.wasm"]["sha256"]:
            raise ValidationError("oracle code ID/checksum is not bound to artifact")
    factory_configs = []
    for item in contracts["factory"]["readback_evidence"]:
        _validate_factory_config(
            item["config"],
            artifacts,
            oracle_address,
            contracts["factory"]["authority_kind"],
            "factory.config",
        )
        factory_configs.append(item["config"])
        if item["code_id"] != artifacts["market_factory.wasm"]["receipt_evidence"][0]["code_id"] or item["chain_sha256"] != artifacts["market_factory.wasm"]["sha256"]:
            raise ValidationError("factory code ID/checksum is not bound to artifact")
    if factory_configs[0] != factory_configs[1]:
        raise ValidationError("dual-provider factory readbacks disagree")
    authority = factory_configs[0]["verdict_authority"]
    market_receipt_question = contracts["seeded_market"]["receipt_evidence"][0]["question_id"]
    for index, item in enumerate(contracts["seeded_market"]["readback_evidence"]):
        _validate_market_readback(item, artifacts, factory_address, oracle_address, authority, market_address, f"market[{index}]")
        if item["question"]["question_id"] != market_receipt_question:
            raise ValidationError("market receipt question ID disagrees with queried identity")
    left = {key: value for key, value in contracts["seeded_market"]["readback_evidence"][0].items()
            if key not in {"provider", "endpoint", "height"}}
    right = {key: value for key, value in contracts["seeded_market"]["readback_evidence"][1].items()
             if key not in {"provider", "endpoint", "height"}}
    if left != right:
        raise ValidationError("dual-provider market readbacks disagree")


def validate_manifest(value: dict[str, Any], *, complete: bool = False) -> None:
    _walk_public(value)
    value = _exact(value, TOP_FIELDS, "manifest")
    if value["schema_version"] != 1 or value["testnet_only"] is not True or value["authorization"] != "NOT launch authorization":
        raise ValidationError("manifest must be schema v1, testnet_only=true, and explicitly unauthorized")
    network = _exact(value["network"], NETWORK_FIELDS, "network")
    drift = _uint(network["max_height_drift"], "network.max_height_drift")
    if drift > 100:
        raise ValidationError("network.max_height_drift exceeds sane bound of 100")
    if network["chain_id"] != CHAIN_ID or network["collateral_denom"] != DENOM:
        raise ValidationError("manifest network must be uni-7/ujunox")
    release = _exact(value["release"], RELEASE_FIELDS, "release")
    if release["optimizer_image"] != PINNED_OPTIMIZER or release["source_commit"] != release["approved_source_commit"] or not HEX40.fullmatch(str(release["source_commit"])) or not HEX64.fullmatch(str(release["release_manifest_sha256"])):
        raise ValidationError("manifest release identity/explicit source approval is invalid")
    artifacts = value["artifacts"]
    if not isinstance(artifacts, dict) or set(artifacts) != set(ARTIFACTS):
        raise ValidationError("manifest artifact set must be exact")
    for name, record in artifacts.items():
        _validate_artifact(name, record)
    if len({record["sha256"] for record in artifacts.values()}) != 3:
        raise ValidationError("artifact checksums collide")
    observations = value["observations"]
    if not isinstance(observations, list) or len(observations) != 2:
        raise ValidationError("manifest requires exactly two RPC observations")
    providers: set[str] = set()
    hosts: set[str] = set()
    evidence_pairs: set[tuple[str, str]] = set()
    heights: list[int] = []
    for index, item in enumerate(observations):
        item = _exact(item, OBS_FIELDS, f"observations[{index}]")
        provider, host = _validate_provenance(item, f"observations[{index}]")
        parsed = parse_status({"node_info": {"network": item["chain_id"]}, "sync_info": {
            "catching_up": item["catching_up"], "latest_block_height": item["height"], "latest_block_time": item["time"]}})
        providers.add(provider.casefold()); hosts.add(host); evidence_pairs.add((provider.casefold(), item["endpoint"])); heights.append(parsed["height"])
    if len(providers) != 2 or len(hosts) != 2:
        raise ValidationError("RPC observations must use distinct normalized hosts and provider labels")
    if max(heights) - min(heights) > drift:
        raise ValidationError("RPC observation heights exceed recorded max_height_drift")
    _validate_contracts(value["contracts"], artifacts, complete=complete)
    if complete:
        all_evidence: list[dict[str, Any]] = []
        for record in artifacts.values():
            if len(record["receipt_evidence"]) != 2 or len(record["chain_evidence"]) != 2:
                raise ValidationError("complete manifest requires dual receipts and chain checksum evidence for every artifact")
            if {(item["provider"].casefold(), item["endpoint"]) for item in record["receipt_evidence"]} != evidence_pairs:
                raise ValidationError("each artifact receipt must come from both preflight providers")
            if {(item["provider"].casefold(), item["endpoint"]) for item in record["chain_evidence"]} != evidence_pairs:
                raise ValidationError("each artifact checksum must be independently read from both preflight providers")
            all_evidence.extend(record["chain_evidence"])
        for record in value["contracts"].values():
            if {(item["provider"].casefold(), item["endpoint"]) for item in record["receipt_evidence"]} != evidence_pairs:
                raise ValidationError("each contract receipt must come from both preflight providers")
            if {(item["provider"].casefold(), item["endpoint"]) for item in record["readback_evidence"]} != evidence_pairs:
                raise ValidationError("each contract must be independently read from both preflight providers")
            all_evidence.extend(record["readback_evidence"])
        if any((item["provider"].casefold(), item["endpoint"]) not in evidence_pairs for item in all_evidence):
            raise ValidationError("all complete evidence must come from the preflight providers/endpoints")
        common: dict[str, set[int]] = {provider: set() for provider in providers}
        for item in all_evidence:
            common[item["provider"].casefold()].add(item["height"])
        if any(len(provider_heights) != 1 for provider_heights in common.values()):
            raise ValidationError("all chain readbacks must use one common height per provider")
        query_heights = [next(iter(provider_heights)) for provider_heights in common.values()]
        if max(query_heights) - min(query_heights) > drift:
            raise ValidationError("dual-provider readback heights exceed recorded max_height_drift")


def command_preflight(args: argparse.Namespace) -> None:
    root = pathlib.Path(__file__).resolve().parents[2]
    reviewed = validate_artifacts(root, args.release_manifest.resolve(), args.artifacts.resolve(), args.source_commit)
    if not isinstance(args.max_height_drift, int) or args.max_height_drift < 0 or args.max_height_drift > 100:
        raise ValidationError("max-height-drift must be between 0 and 100")
    if len(args.rpc) != 2 or len(args.provider) != 2:
        raise ValidationError("provide exactly two --rpc and two corresponding --provider values")
    normalized = [normalize_endpoint(endpoint) for endpoint in args.rpc]
    providers = [_provider(provider) for provider in args.provider]
    if len({item[1] for item in normalized}) != 2 or len({provider.casefold() for provider in providers}) != 2:
        raise ValidationError("provide two independently named providers on distinct normalized hosts")
    observations = []
    for provider, (endpoint, _host) in zip(providers, normalized, strict=True):
        observations.append({"provider": provider, "endpoint": endpoint, **fetch_status(endpoint)})
    if abs(observations[0]["height"] - observations[1]["height"]) > args.max_height_drift:
        raise ValidationError("independent RPC heights exceed permitted drift")
    output = {
        "schema_version": 1, "testnet_only": True, "authorization": "NOT launch authorization",
        "network": {"chain_id": CHAIN_ID, "collateral_denom": DENOM, "max_height_drift": args.max_height_drift},
        "release": {key: reviewed[key] for key in RELEASE_FIELDS}, "artifacts": reviewed["artifacts"],
        "observations": observations,
        "contracts": {
            "oracle": {"receipt_evidence": [], "readback_evidence": []},
            "factory": {
                "authority_kind": "disclosed test-only authority",
                "receipt_evidence": [],
                "readback_evidence": [],
            },
            "seeded_market": {"receipt_evidence": [], "readback_evidence": []},
        },
    }
    validate_manifest(output)
    dump_json(args.output, output)
    print(f"wrote sanitized preflight manifest: {args.output}")


def _provenance(args: argparse.Namespace) -> dict[str, str]:
    endpoint, _host = normalize_endpoint(args.endpoint)
    return {"provider": _provider(args.provider), "endpoint": endpoint}


def command_parse(args: argparse.Namespace) -> None:
    result = {**_provenance(args), **parse_receipt(load_json(args.receipt), args.kind)}
    dump_json(args.output, result)
    print(f"wrote sanitized public receipt evidence: {args.output}")


def _record_evidence(items: list[dict[str, Any]], evidence: dict[str, Any]) -> None:
    identity = (evidence["provider"].casefold(), evidence["endpoint"])
    for current in items:
        if (current["provider"].casefold(), current["endpoint"]) == identity:
            if current != evidence:
                raise ValidationError("refusing to replace changed provider evidence")
            return
    if len(items) >= 2:
        raise ValidationError("refusing more than two provider observations")
    items.append(evidence)


def _record_once(record: dict[str, Any], key: str, value: Any) -> None:
    """Compatibility helper used by tests and external reviewers."""
    current = record.get(key)
    if current is not None and current != value:
        raise ValidationError(f"refusing to replace recorded {key}: {current!r} != {value!r}")
    record[key] = value


def command_record(args: argparse.Namespace) -> None:
    manifest = load_json(args.manifest)
    if not isinstance(manifest, dict):
        raise ValidationError("manifest must be an object")
    validate_manifest(manifest)
    provenance = _provenance(args)
    if (args.provider.casefold(), provenance["endpoint"]) not in {
        (item["provider"].casefold(), item["endpoint"]) for item in manifest["observations"]
    }:
        raise ValidationError("receipt provenance must exactly match a preflight provider/endpoint")
    if args.target in ARTIFACTS:
        parsed = parse_receipt(load_json(args.receipt), "store")
        record = manifest["artifacts"][args.target]
    else:
        kind = "market" if args.target == "seeded_market" else "instantiate"
        parsed = parse_receipt(load_json(args.receipt), kind)
        record = manifest["contracts"][args.target]
    _record_evidence(record["receipt_evidence"], {**provenance, **parsed})
    validate_manifest(manifest)
    dump_json(args.manifest, manifest)
    print(f"recorded sanitized {args.target} receipt evidence in {args.manifest}")


def command_validate(args: argparse.Namespace) -> None:
    value = load_json(args.manifest)
    if not isinstance(value, dict):
        raise ValidationError("manifest must be an object")
    validate_manifest(value, complete=args.complete)
    root = pathlib.Path(__file__).resolve().parents[2]
    verify_local_commit(root, value["release"]["approved_source_commit"])
    print("manifest valid")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(required=True)
    preflight = sub.add_parser("preflight", help="read RPC status and validate reviewed artifacts")
    preflight.add_argument("--release-manifest", type=pathlib.Path, required=True)
    preflight.add_argument("--artifacts", type=pathlib.Path, required=True)
    preflight.add_argument("--source-commit", required=True, help="full explicitly approved release source commit")
    preflight.add_argument("--rpc", action="append", required=True)
    preflight.add_argument("--provider", action="append", required=True)
    preflight.add_argument("--max-height-drift", type=int, default=20)
    preflight.add_argument("--output", type=pathlib.Path, required=True)
    preflight.set_defaults(func=command_preflight)
    receipt = sub.add_parser("parse-receipt", help="sanitize a public chain receipt with provenance")
    receipt.add_argument("--kind", choices=("store", "instantiate", "market"), required=True)
    receipt.add_argument("--receipt", type=pathlib.Path, required=True)
    receipt.add_argument("--provider", required=True)
    receipt.add_argument("--endpoint", required=True)
    receipt.add_argument("--output", type=pathlib.Path, required=True)
    receipt.set_defaults(func=command_parse)
    record = sub.add_parser("record-receipt", help="idempotently add one provider's public receipt evidence")
    record.add_argument("--target", choices=tuple(ARTIFACTS) + ("oracle", "factory", "seeded_market"), required=True)
    record.add_argument("--receipt", type=pathlib.Path, required=True)
    record.add_argument("--provider", required=True)
    record.add_argument("--endpoint", required=True)
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
