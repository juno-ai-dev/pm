#!/usr/bin/env python3
"""Validate release size and empirical gas/storage evidence fail-closed."""
from __future__ import annotations
import json
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[2]
thresholds = json.loads((root / "quality/thresholds.json").read_text())
report_path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else root / "quality/gas-storage-report.json"
report = json.loads(report_path.read_text())
expected = {
    "create_activate_realistic", "create_activate_max_question_metadata",
    "buy_sell_realistic", "buy_sell_max_reserve_arithmetic",
    "challenge_verdict", "resolve", "redeem_positions_fragmented",
    "redeem_lp_with_accrual", "factory_pagination_max",
}
scenarios = report.get("scenarios", [])
ids = [s.get("id") for s in scenarios]
if len(ids) != len(set(ids)) or set(ids) != expected:
    raise SystemExit("report must contain each required scenario exactly once")

status = report.get("status")
if status == "measurement_required":
    review = report.get("review", {})
    if review.get("required") is not True or not review.get("reason"):
        raise SystemExit("unmeasured report must fail closed with a review reason")
    if any(s.get("gas_used") is not None for s in scenarios):
        raise SystemExit("measurement_required report cannot mix unverified measurements")
    print("gas/storage measurement remains an explicit canary review gate")
elif status == "measured":
    if not report.get("chain") or not report.get("binary") or not report.get("source_commit"):
        raise SystemExit("measured report requires chain, binary, and source commit provenance")
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
    for artifact in manifest.get("artifacts", []):
        limit = sizes.get(artifact["file"])
        if limit is None or artifact["size_bytes"] > limit:
            raise SystemExit(f"wasm size threshold failed: {artifact['file']}")
    print(f"validated {len(manifest.get('artifacts', []))} wasm sizes")
