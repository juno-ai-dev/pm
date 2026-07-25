#!/usr/bin/env python3
"""Deterministic tests for uni-7 status/receipt parsing and manifest gates."""
from __future__ import annotations

import base64
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "uni7" / "deployment.py"
SPEC = importlib.util.spec_from_file_location("uni7_deployment", SCRIPT)
assert SPEC and SPEC.loader
DEPLOY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DEPLOY)

TX = "A" * 64
ADDR = "juno1" + "q" * 38


def artifact(name: str, char: str) -> dict:
    return {
        "sha256": char * 64,
        "size_bytes": 123,
        "required_exports": sorted(DEPLOY.ARTIFACTS[name]),
        "code_id": None,
    }


def partial_manifest() -> dict:
    return {
        "schema_version": 1,
        "testnet_only": True,
        "authorization": "NOT launch authorization",
        "network": {"chain_id": "uni-7", "collateral_denom": "ujunox"},
        "release": {
            "source_commit": "1" * 40,
            "release_manifest_sha256": "2" * 64,
            "optimizer_image": DEPLOY.PINNED_OPTIMIZER,
        },
        "artifacts": {
            "binary_market.wasm": artifact("binary_market.wasm", "a"),
            "cw_reality.wasm": artifact("cw_reality.wasm", "b"),
            "market_factory.wasm": artifact("market_factory.wasm", "c"),
        },
        "observations": [],
        "contracts": {},
    }


class StatusParsingTests(unittest.TestCase):
    def test_accepts_lowercase_status(self):
        status = DEPLOY.parse_status({
            "node_info": {"network": "uni-7"},
            "sync_info": {"catching_up": False, "latest_block_height": "91", "latest_block_time": "2026-07-25T00:00:00Z"},
        })
        self.assertEqual(status["height"], 91)

    def test_accepts_wrapped_legacy_status(self):
        status = DEPLOY.parse_status({"result": {
            "NodeInfo": {"network": "uni-7"},
            "SyncInfo": {"catching_up": "false", "latest_block_height": "92", "latest_block_time": "2026-07-25T00:00:01Z"},
        }})
        self.assertFalse(status["catching_up"])

    def test_rejects_wrong_chain_and_syncing(self):
        base = {"node_info": {"network": "uni-7"}, "sync_info": {
            "catching_up": False, "latest_block_height": "1", "latest_block_time": "now"}}
        for mutation in (lambda x: x["node_info"].update(network="uni-8"),
                         lambda x: x["sync_info"].update(catching_up=True)):
            value = json.loads(json.dumps(base))
            mutation(value)
            with self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.parse_status(value)


class JsonParsingTests(unittest.TestCase):
    def test_rejects_duplicate_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "duplicate.json"
            source.write_text('{"chain_id":"uni-7","chain_id":"juno-1"}')
            with self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.load_json(source)


class ReceiptParsingTests(unittest.TestCase):
    def test_store_from_logs(self):
        receipt = {"tx_response": {"code": 0, "txhash": TX, "height": "33", "logs": [{"events": [
            {"type": "store_code", "attributes": [{"key": "code_id", "value": "42"}]}
        ]}]}}
        self.assertEqual(DEPLOY.parse_receipt(receipt, "store")["code_id"], 42)

    def test_base64_tendermint_attributes(self):
        enc = lambda value: base64.b64encode(value.encode()).decode()
        receipt = {"code": 0, "txhash": TX, "height": "34", "events": [{"attributes": [
            {"key": enc("_contract_address"), "value": enc(ADDR)}
        ]}]}
        self.assertEqual(DEPLOY.parse_receipt(receipt, "instantiate")["contract_address"], ADDR)

        store = {"code": 0, "txhash": TX, "height": "34", "events": [{"attributes": [
            {"key": enc("code_id"), "value": enc("42")}
        ]}]}
        self.assertEqual(DEPLOY.parse_receipt(store, "store")["code_id"], 42)

    def test_market_identity_and_failed_receipt(self):
        receipt = {"code": 0, "txhash": TX, "height": "35", "events": [{"attributes": [
            {"key": "_contract_address", "value": ADDR}, {"key": "question_id", "value": "d" * 64}
        ]}]}
        self.assertEqual(DEPLOY.parse_receipt(receipt, "market")["question_id"], "d" * 64)
        receipt["code"] = 5
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.parse_receipt(receipt, "market")

    def test_rejects_ambiguous_code_ids(self):
        receipt = {"code": 0, "txhash": TX, "height": "1", "events": [{"attributes": [
            {"key": "code_id", "value": "4"}, {"key": "code_id", "value": "5"}
        ]}]}
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.parse_receipt(receipt, "store")


class ManifestTests(unittest.TestCase):
    def test_partial_manifest_is_restartable(self):
        DEPLOY.validate_manifest(partial_manifest())

    def test_rejects_secret_field_wrong_denom_and_collision(self):
        mutations = [
            lambda m: m.update(mnemonic="never"),
            lambda m: m["network"].update(collateral_denom="ujuno"),
            lambda m: m["artifacts"]["cw_reality.wasm"].update(sha256="a" * 64),
            lambda m: m["release"].update(optimizer_image="cosmwasm/optimizer:latest"),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                manifest = partial_manifest()
                mutate(manifest)
                with self.assertRaises(DEPLOY.ValidationError):
                    DEPLOY.validate_manifest(manifest)

    def test_complete_manifest_requires_readback(self):
        manifest = partial_manifest()
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)

    def test_accepts_exact_complete_manifest_and_rejects_tier_drift(self):
        manifest = partial_manifest()
        manifest["observations"] = [
            {"endpoint": "https://a", "chain_id": "uni-7", "height": 10,
             "time": "2026-07-25T00:00:00Z", "catching_up": False},
            {"endpoint": "https://b", "chain_id": "uni-7", "height": 11,
             "time": "2026-07-25T00:00:01Z", "catching_up": False},
        ]
        for index, record in enumerate(manifest["artifacts"].values(), 1):
            record.update(code_id=index, store_txhash=TX, chain_sha256=record["sha256"])
        authority = "juno1" + "p" * 38
        manifest["contracts"] = {
            "oracle": {"address": ADDR, "txhash": TX, "admin": None,
                       "config": dict(DEPLOY.EXPECTED_ORACLE)},
            "factory": {"address": ADDR, "txhash": TX, "admin": None, "code_id": 3,
                        "contract_profile": "uni7", "market_code_id": 1,
                        "oracle_code_id": 2, "oracle": ADDR,
                        "tier": dict(DEPLOY.EXPECTED_TIER), "collateral_denom": "ujunox",
                        "verdict_authority": authority,
                        "verdict_authority_kind": "disclosed test-only authority"},
            "seeded_market": {"address": ADDR, "txhash": TX, "code_id": 1,
                              "contract_profile": "uni7", "factory": ADDR,
                              "oracle": ADDR, "collateral_denom": "ujunox",
                              "verdict_authority": authority, "question_id": "d" * 64},
        }
        DEPLOY.validate_manifest(manifest, complete=True)
        for contract in ("factory", "seeded_market"):
            wrong_profile = json.loads(json.dumps(manifest))
            wrong_profile["contracts"][contract]["contract_profile"] = "juno1"
            with self.assertRaises(DEPLOY.ValidationError):
                DEPLOY.validate_manifest(wrong_profile, complete=True)
        manifest["contracts"]["factory"]["tier"]["fee_bps"] = 201
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY.validate_manifest(manifest, complete=True)

    def test_record_is_idempotent_and_rejects_changed_checkpoint(self):
        record = {}
        DEPLOY._record_once(record, "code_id", 7)
        DEPLOY._record_once(record, "code_id", 7)
        with self.assertRaises(DEPLOY.ValidationError):
            DEPLOY._record_once(record, "code_id", 8)

    def test_cli_writes_only_sanitized_receipt(self):
        receipt = {"tx_response": {"code": 0, "txhash": TX, "height": "33", "events": [
            {"attributes": [{"key": "code_id", "value": "42"}]}
        ]}, "irrelevant": {"password": "must not copy"}}
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "receipt.json"
            output = Path(directory) / "public.json"
            source.write_text(json.dumps(receipt))
            result = DEPLOY.parse_receipt(DEPLOY.load_json(source), "store")
            DEPLOY.dump_json(output, result)
            self.assertEqual(json.loads(output.read_text()), {"code_id": 42, "height": 33, "txhash": TX})


if __name__ == "__main__":
    unittest.main()
