#!/usr/bin/env python3
"""Validate release size and empirical gas/storage evidence fail-closed."""
from __future__ import annotations
import json
import pathlib
import re
import sys

root = pathlib.Path(__file__).resolve().parents[2]
thresholds = json.loads((root / "quality/thresholds.json").read_text())
report_path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else root / "quality/gas-storage-report.json"
report = json.loads(report_path.read_text())
expected = {
    "create_activate_realistic": ("transaction", "realistic"),
    "create_activate_max_question_metadata": ("transaction", "maximum"),
    "buy_sell_realistic": ("transaction", "realistic"),
    "buy_sell_max_reserve_arithmetic": ("transaction", "maximum"),
    "challenge_verdict": ("transaction", "maximum"),
    "resolve": ("transaction", "maximum"),
    "redeem_positions_fragmented": ("transaction", "maximum"),
    "redeem_lp_with_accrual": ("transaction", "maximum"),
    "factory_pagination_max": ("query", "maximum"),
}
scenarios = report.get("scenarios", [])
ids = [s.get("id") for s in scenarios]
if len(ids) != len(set(ids)) or set(ids) != set(expected):
    raise SystemExit("report must contain each required scenario exactly once")
for item in scenarios:
    if (item.get("class"), item.get("bound")) != expected[item["id"]]:
        raise SystemExit(f"{item['id']}: scenario class/bound does not match the required profile")

status = report.get("status")
if status == "measurement_required":
    review = report.get("review", {})
    if review.get("required") is not True or not review.get("reason"):
        raise SystemExit("unmeasured report must fail closed with a review reason")
    measurement_fields = ("gas_used", "storage_delta_bytes", "response_bytes")
    if any(s.get(field) is not None for s in scenarios for field in measurement_fields):
        raise SystemExit("measurement_required report cannot mix unverified measurements")
    if any(report.get(field) is not None for field in ("chain", "binary", "source_commit")):
        raise SystemExit("measurement_required report cannot claim measurement provenance")
    print("gas/storage measurement remains an explicit canary review gate")
elif status == "measured":
    if not report.get("chain") or not report.get("binary") or not report.get("source_commit"):
        raise SystemExit("measured report requires chain, binary, and source commit provenance")
    if re.fullmatch(r"[0-9a-f]{40}", report["source_commit"]) is None:
        raise SystemExit("measured report source_commit must be a lowercase 40-character Git SHA")
    breaches = []
    for item in scenarios:
        gas = item.get("gas_used")
        if not isinstance(gas, int) or gas <= 0:
            raise SystemExit(f"{item['id']}: positive empirical gas_used required")
        gas_limit = thresholds["gas"][f"{item['class']}_max"]
        allowed = gas_limit * (10000 - thresholds["gas"]["required_headroom_bps"]) // 10000
        if gas > allowed:
            breaches.append(f"{item['id']} gas {gas}>{allowed}")
        size_key = "response_bytes" if item["class"] == "query" else "storage_delta_bytes"
        value = item.get(size_key)
        if not isinstance(value, int) or value < 0:
            raise SystemExit(f"{item['id']}: non-negative {size_key} required")
        size_limit = thresholds["storage_bytes"][
            "query_response_max" if item["class"] == "query" else "transaction_delta_max"
        ]
        if value > size_limit:
            breaches.append(f"{item['id']} {size_key} {value}>{size_limit}")
    if breaches:
        review = report.get("review", {})
        if review.get("required") is not True or not review.get("approver") or not review.get("reason"):
            raise SystemExit("threshold breach requires named explicit review: " + "; ".join(breaches))
    print(f"validated {len(scenarios)} empirical scenarios")
else:
    raise SystemExit("status must be measurement_required or measured")

if len(sys.argv) > 2:
    manifest = json.loads(pathlib.Path(sys.argv[2]).read_text())
    sizes = thresholds["wasm_size_bytes"]
    artifacts = manifest.get("artifacts", [])
    names = [artifact.get("file") for artifact in artifacts]
    if len(names) != len(set(names)) or set(names) != set(sizes):
        raise SystemExit("manifest must contain every required wasm artifact exactly once")
    if status == "measured" and report["source_commit"] != manifest.get("source_commit"):
        raise SystemExit("measured report source_commit does not match the release manifest")
    for artifact in artifacts:
        limit = sizes.get(artifact["file"])
        size = artifact.get("size_bytes")
        if limit is None or not isinstance(size, int) or size <= 0 or size > limit:
            raise SystemExit(f"wasm size threshold failed: {artifact['file']}")
    print(f"validated {len(artifacts)} wasm sizes")
