#!/usr/bin/env python3
"""Adversarial tests for the fail-closed uni-7 evidence gates."""
from __future__ import annotations

import base64
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "uni7" / "deployment.py"
SPEC = importlib.util.spec_from_file_location("uni7_deployment", SCRIPT)
assert SPEC and SPEC.loader
DEPLOY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DEPLOY)

TX = "A" * 64
ADDR = "juno1" + "q" * 38
FACTORY = "juno1" + "r" * 38
MARKET = "juno1" + "s" * 38
CREATOR = "juno1" + "t" * 38
AUTHORITY = "juno1" + "p" * 38
ENDPOINTS = ("https://rpc-a.example", "https://rpc-b.example")
PROVIDERS = ("provider-a", "provider-b")
QID = "d" * 64
QHASH = "e" * 64


def receipt_evidence(kind: str, index: int, *, code_id: int = 1, address: str = ADDR) -> dict:
    value = {"provider": PROVIDERS[index], "endpoint": ENDPOINTS[index], "txhash": TX, "height": 90}
    if kind == "store":
        value["code_id"] = code_id
    else:
        value["address"] = address
        if kind == "market":
            value["question_id"] = QID
    return value


def artifact(name: str, char: str, code_id: int) -> dict:
    return {
        "sha256": char * 64, "size_bytes": 123,
        "required_exports": sorted(DEPLOY.ARTIFACTS[name]),
        "receipt_evidence": [receipt_evidence("store", i, code_id=code_id) for i in range(2)],
        "chain_evidence": [
            {"provider": PROVIDERS[i], "endpoint": ENDPOINTS[i], "height": 100 + i,
             "code_id": code_id, "chain_sha256": char * 64, "admin": None}
            for i in range(2)
        ],
    }


def factory_config() -> dict:
    return {
        "contract_profile": "uni7", "protocol_version": "v1", "market_code_id": 1,
        "market_checksum": "a" * 64, "tier_id": 1, "tier": dict(DEPLOY.EXPECTED_TIER),
        "oracle": ADDR, "oracle_code_id": 2, "oracle_checksum": "b" * 64,
        "verdict_authority": AUTHORITY,
        "collateral_denom": "ujunox", "oracle_min_initial_bond_floor": "10000000",
        "oracle_min_answer_timeout_secs": 86400,
    }


def market_config() -> dict:
    return {
        "contract_profile": "uni7", "protocol_version": "v1", "factory": FACTORY,
        "creator": CREATOR, "initial_lp": CREATOR, "oracle": ADDR,
        "verdict_authority": AUTHORITY, "tier": 1, "collateral_denom": "ujunox",
        "close_ts": 2_000_000_000, "opening_ts": 2_000_086_400,
        "initial_liquidity": "100000000", "oracle_bounty": "1000000",
        **{key: value for key, value in DEPLOY.EXPECTED_TIER.items()
           if key not in {"min_initial_liquidity", "max_initial_liquidity", "min_oracle_bounty", "max_oracle_bounty"}},
    }


def complete_manifest() -> dict:
    artifacts = {
        "binary_market.wasm": artifact("binary_market.wasm", "a", 1),
        "cw_reality.wasm": artifact("cw_reality.wasm", "b", 2),
        "market_factory.wasm": artifact("market_factory.wasm", "c", 3),
    }
    registry = {
        "contract_profile": "uni7", "collateral_denom": "ujunox",
        "nonce": 7, "market": MARKET, "creator": CREATOR, "tier_id": 1,
        "question_id": QID, "question_hash": QHASH, "close_ts": 2_000_000_000,
        "opening_ts": 2_000_086_400, "initial_liquidity": "100000000",
        "oracle_bounty": "1000000", "initial_funding": "101000000",
        "created_height": 90, "created_time": 1_999_999_000,
    }
    contracts = {
        "oracle": {
            "receipt_evidence": [receipt_evidence("instantiate", i, address=ADDR) for i in range(2)],
            "readback_evidence": [
                {"provider": PROVIDERS[i], "endpoint": ENDPOINTS[i], "height": 100 + i,
                 "code_id": 2, "chain_sha256": "b" * 64, "admin": None,
                 "config": dict(DEPLOY.EXPECTED_ORACLE)} for i in range(2)
            ],
        },
        "factory": {
            "authority_kind": "disclosed test-only authority",
            "receipt_evidence": [receipt_evidence("instantiate", i, address=FACTORY) for i in range(2)],
            "readback_evidence": [
                {"provider": PROVIDERS[i], "endpoint": ENDPOINTS[i], "height": 100 + i,
                 "code_id": 3, "chain_sha256": "c" * 64, "admin": None,
                 "config": factory_config()} for i in range(2)
            ],
        },
        "seeded_market": {
            "receipt_evidence": [receipt_evidence("market", i, address=MARKET) for i in range(2)],
            "readback_evidence": [
                {"provider": PROVIDERS[i], "endpoint": ENDPOINTS[i], "height": 100 + i,
                 "code_id": 1, "chain_sha256": "a" * 64, "admin": None,
                 "config": market_config(),
                 "identity": {"contract_profile": "uni7", "collateral_denom": "ujunox",
                              "protocol_version": "v1", "factory": FACTORY, "market": MARKET,
                              "nonce": 7, "question_id": QID},
                 "question": {"question_id": QID, "question_hash": QHASH},
                 "registry": dict(registry)} for i in range(2)
            ],
        },
    }
    return {
        "schema_version": 1, "testnet_only": True, "authorization": "NOT launch authorization",
        "network": {"chain_id": "uni-7", "collateral_denom": "ujunox", "max_height_drift": 20},
        "release": {"source_commit": "1" * 40, "approved_source_commit": "1" * 40,
                    "release_manifest_sha256": "2" * 64, "optimizer_image": DEPLOY.PINNED_OPTIMIZER},
        "artifacts": artifacts,
        "observations": [
            {"provider": PROVIDERS[i], "endpoint": ENDPOINTS[i], "chain_id": "uni-7",
             "height": 95 + i, "time": f"2026-07-25T00:00:0{i}Z", "catching_up": False}
            for i in range(2)
        ],
        "contracts": contracts,
    }


class StatusAndEndpointTests(unittest.TestCase):
    def test_status_requires_sane_noncatching_observation(self):
        valid = {"node_info": {"network": "uni-7"}, "sync_info": {
            "catching_up": False, "latest_block_height": "91", "latest_block_time": "2026-07-25T00:00:00Z"}}
        self.assertEqual(DEPLOY.parse_status(valid)["height"], 91)
        for key, bad in (("catching_up", "False"), ("catching_up", True),
                         ("latest_block_height", True), ("latest_block_height", "0"),
                         ("latest_block_time", "now")):
            value = copy.deepcopy(valid); value["sync_info"][key] = bad
            with self.subTest(key=key, bad=bad), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.parse_status(value)

    def test_endpoint_is_https_persistable_and_host_distinct(self):
        self.assertEqual(DEPLOY.normalize_endpoint("https://RPC.EXAMPLE.:443/base/")[0], "https://rpc.example/base")
        for endpoint in ("http://rpc.example", "https://user@rpc.example", "https://rpc.example?q=token",
                         "https://rpc.example/#fragment", "https://rpc.example?password=x"):
            with self.subTest(endpoint=endpoint), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.normalize_endpoint(endpoint)
        manifest = complete_manifest()
        manifest["observations"][1]["endpoint"] = "https://rpc-a.example/other"
        manifest["observations"][1]["provider"] = "provider-b"
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest)


class ReceiptTests(unittest.TestCase):
    @staticmethod
    def store(code: Any = 0) -> dict:
        return {"code": code, "txhash": TX, "height": "34", "events": [{"attributes": [
            {"key": "code_id", "value": "42"}]}]}

    def test_code_must_be_explicit_numeric_zero_bool_rejected(self):
        for code in (None, False, True, "0", 1):
            receipt = self.store(code)
            if code is None:
                receipt.pop("code")
            with self.subTest(code=code), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.parse_receipt(receipt, "store")
        self.assertEqual(DEPLOY.parse_receipt(self.store(), "store")["code_id"], 42)

    def test_base64_decoding_and_ambiguity(self):
        enc = lambda value: base64.b64encode(value.encode()).decode()
        receipt = {"code": 0, "txhash": TX, "height": 34, "events": [{"attributes": [
            {"key": enc("_contract_address"), "value": enc(ADDR)}]}]}
        self.assertEqual(DEPLOY.parse_receipt(receipt, "instantiate")["address"], ADDR)
        receipt = self.store(); receipt["events"][0]["attributes"].append({"key": "code_id", "value": "43"})
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.parse_receipt(receipt, "store")

    def test_dual_receipts_must_match_and_be_independent(self):
        manifest = complete_manifest()
        manifest["artifacts"]["binary_market.wasm"]["receipt_evidence"][1]["txhash"] = "B" * 64
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest)
        manifest = complete_manifest()
        manifest["contracts"]["oracle"]["receipt_evidence"][1]["endpoint"] = ENDPOINTS[0]
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest)


class ManifestTests(unittest.TestCase):
    def test_accepts_exact_complete_manifest(self):
        DEPLOY.validate_manifest(complete_manifest(), complete=True)

    def test_exact_schema_rejects_arbitrary_fields_everywhere(self):
        mutations = [
            lambda m: m.update(extra="x"),
            lambda m: m["network"].update(extra="x"),
            lambda m: m["artifacts"]["binary_market.wasm"].update(extra="x"),
            lambda m: m["contracts"]["factory"]["readback_evidence"][0]["config"].update(extra="x"),
            lambda m: m["contracts"]["seeded_market"]["readback_evidence"][0]["registry"].update(extra="x"),
        ]
        for mutate in mutations:
            manifest = complete_manifest(); mutate(manifest)
            with self.subTest(mutate=mutate), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest, complete=True)

    def test_secret_values_and_keys_are_rejected(self):
        for key, value in (("note", "bearer abcdef"), ("note", "https://source.example/x?token=abc"),
                           ("api_key", "abc"), ("note", "-----BEGIN PRIVATE KEY-----")):
            manifest = complete_manifest(); manifest[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest)

    def test_max_drift_rechecked_for_status_and_common_readback_height(self):
        manifest = complete_manifest(); manifest["observations"][1]["height"] = 200
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest)
        manifest = complete_manifest()
        manifest["contracts"]["oracle"]["readback_evidence"][0]["height"] = 99
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)
        manifest = complete_manifest()
        for record in manifest["artifacts"].values(): record["chain_evidence"][1]["height"] = 130
        for record in manifest["contracts"].values(): record["readback_evidence"][1]["height"] = 130
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)

    def test_every_admin_is_independently_queried_null(self):
        for contract in ("oracle", "factory", "seeded_market"):
            manifest = complete_manifest(); manifest["contracts"][contract]["readback_evidence"][1]["admin"] = ""
            with self.subTest(contract=contract), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest, complete=True)

    def test_all_code_ids_and_checksums_bound_to_artifacts(self):
        mutations = [
            lambda m: m["artifacts"]["cw_reality.wasm"]["chain_evidence"][0].update(chain_sha256="f" * 64),
            lambda m: m["contracts"]["oracle"]["readback_evidence"][0].update(code_id=3),
            lambda m: m["contracts"]["factory"]["readback_evidence"][0].update(chain_sha256="a" * 64),
            lambda m: m["contracts"]["seeded_market"]["readback_evidence"][0].update(code_id=2),
        ]
        for mutate in mutations:
            manifest = complete_manifest(); mutate(manifest)
            with self.subTest(mutate=mutate), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest, complete=True)

    def test_factory_pr66_fields_are_exact(self):
        for field, bad in (("contract_profile", "juno1"), ("protocol_version", "v2"),
                           ("tier_id", 2), ("market_checksum", "f" * 64),
                           ("oracle_min_answer_timeout_secs", 1)):
            manifest = complete_manifest()
            for evidence in manifest["contracts"]["factory"]["readback_evidence"]:
                evidence["config"][field] = bad
            with self.subTest(field=field), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest, complete=True)
        manifest = complete_manifest()
        manifest["contracts"]["factory"]["authority_kind"] = "production authority"
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)

    def test_market_registry_identity_and_funding_cross_links(self):
        for section, field, bad in (
            ("config", "creator", ADDR),
            ("identity", "nonce", 8),
            ("identity", "contract_profile", "juno1"),
            ("question", "question_hash", "f" * 64),
            ("registry", "collateral_denom", "ujuno"),
            ("registry", "initial_funding", "100000000"),
            ("registry", "opening_ts", 1),
        ):
            manifest = complete_manifest()
            for evidence in manifest["contracts"]["seeded_market"]["readback_evidence"]:
                evidence[section][field] = bad
            with self.subTest(section=section, field=field), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest, complete=True)

    def test_completion_requires_dual_public_evidence(self):
        removals = [
            lambda m: m["artifacts"]["binary_market.wasm"]["receipt_evidence"].pop(),
            lambda m: m["artifacts"]["binary_market.wasm"]["chain_evidence"].pop(),
            lambda m: m["contracts"]["oracle"]["receipt_evidence"].pop(),
            lambda m: m["contracts"]["factory"]["readback_evidence"].pop(),
        ]
        for remove in removals:
            manifest = complete_manifest(); remove(manifest)
            with self.subTest(remove=remove), self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(manifest, complete=True)

    def test_dual_chain_evidence_cannot_repeat_or_replace_preflight_provider(self):
        manifest = complete_manifest()
        manifest["artifacts"]["binary_market.wasm"]["chain_evidence"][1].update(
            provider=PROVIDERS[0], endpoint=ENDPOINTS[0], height=100)
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)
        manifest = complete_manifest()
        manifest["contracts"]["oracle"]["readback_evidence"][1].update(
            provider=PROVIDERS[0], endpoint=ENDPOINTS[0], height=100)
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)
        manifest = complete_manifest()
        manifest["artifacts"]["binary_market.wasm"]["receipt_evidence"][1].update(
            provider="provider-c", endpoint="https://rpc-c.example")
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)
        manifest = complete_manifest()
        manifest["contracts"]["factory"]["receipt_evidence"][1].update(
            provider="provider-c", endpoint="https://rpc-c.example")
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)

    def test_record_is_idempotent_changed_evidence_rejected_and_no_admin_inferred(self):
        evidence = receipt_evidence("instantiate", 0)
        items = []
        DEPLOY._record_evidence(items, evidence); DEPLOY._record_evidence(items, dict(evidence))
        self.assertEqual(items, [evidence]); self.assertNotIn("admin", items[0])
        changed = dict(evidence); changed["height"] = 91
        with self.assertRaises(DEPLOY.ValidationError): DEPLOY._record_evidence(items, changed)


class ArtifactTests(unittest.TestCase):
    def test_source_must_be_explicit_equal_and_local_commit(self):
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        DEPLOY.verify_local_commit(ROOT, head)
        with self.assertRaises(DEPLOY.ValidationError): DEPLOY.verify_local_commit(ROOT, "0" * 40)

    def test_artifact_root_exact_regular_file_set(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            root = base / "artifacts"
            root.mkdir()
            for name in DEPLOY.ARTIFACTS: (root / name).write_bytes(name.encode())
            release = base / "release.json"
            release.write_text("{}")
            with mock.patch.object(DEPLOY, "load_json", return_value={
                "schema_version": 1, "optimizer_image": "cosmwasm/optimizer:0.17.0",
                "optimizer_image_digest": "sha256:" + DEPLOY.PINNED_OPTIMIZER.rsplit("sha256:", 1)[1],
                "source_commit": "1" * 40,
                "artifacts": [{"file": name, "sha256": DEPLOY.sha256(root / name),
                               "size_bytes": (root / name).stat().st_size} for name in DEPLOY.ARTIFACTS],
            }), mock.patch.object(DEPLOY, "verify_local_commit"), mock.patch.object(DEPLOY, "load_export_validator") as loader:
                loader.return_value.verify.return_value = None
                DEPLOY.validate_artifacts(ROOT, release, root, "1" * 40)
                (root / "extra.txt").write_text("x")
                with self.assertRaises(DEPLOY.ValidationError):
                    DEPLOY.validate_artifacts(ROOT, release, root, "1" * 40)


class JsonAndShellSafetyTests(unittest.TestCase):
    def test_duplicate_json_keys_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "x.json"; path.write_text('{"x":1,"x":2}')
            with self.assertRaises(DEPLOY.ValidationError): DEPLOY.load_json(path)

    def test_unsigned_script_uses_shell_arrays_and_has_no_broadcast(self):
        text = (ROOT / "scripts/uni7/prepare-unsigned.sh").read_text()
        self.assertIn('junod "${args[@]}"', text)
        self.assertNotIn("--broadcast", text)
        self.assertNotIn(" tx broadcast ", text.lower())


if __name__ == "__main__":
    unittest.main()
