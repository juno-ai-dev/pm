#!/usr/bin/env python3
"""Behavior tests for the fail-closed oracle deployment verifier."""

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).parent / "fixtures"
VERIFIER = ROOT / "scripts" / "oracle" / "verify-deployment.py"


class VerifyDeploymentTests(unittest.TestCase):
    def run_verifier(self, evidence: str, manifest: Path | None = None):
        return subprocess.run(
            [
                "python3",
                str(VERIFIER),
                "--manifest",
                str(manifest or FIXTURES / "manifest.json"),
                "--evidence",
                str(FIXTURES / evidence),
                "--artifact",
                str(FIXTURES / "cw_reality.wasm"),
                "--repo-root",
                str(ROOT),
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_accepts_exact_frozen_deployment(self):
        result = self.run_verifier("evidence-ok.json")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["status"], "deployment_verified")
        self.assertEqual(report["audit_status"], "pending")
        self.assertIn("source_provenance", report["checks"])

    def test_rejects_wrong_checksum(self):
        result = self.run_verifier("evidence-wrong-checksum.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum", result.stderr)

    def test_rejects_chain_admin(self):
        result = self.run_verifier("evidence-wrong-admin.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("chain admin", result.stderr)

    def test_rejects_wrong_config(self):
        result = self.run_verifier("evidence-wrong-config.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("min_initial_bond_floor", result.stderr)

    def test_rejects_stored_admin(self):
        evidence = json.loads((FIXTURES / "evidence-ok.json").read_text())
        evidence["config"]["admin"] = "juno1storedadmin00000000000000000000000000"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.json"
            path.write_text(json.dumps(evidence))
            result = subprocess.run(
                ["python3", str(VERIFIER), "--manifest", str(FIXTURES / "manifest.json"),
                 "--evidence", str(path), "--artifact", str(FIXTURES / "cw_reality.wasm"),
                 "--repo-root", str(ROOT)], text=True, capture_output=True, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("stored admin", result.stderr)

    def test_rejects_non_reproducible_build_records(self):
        manifest = json.loads((FIXTURES / "manifest.json").read_text())
        manifest["build"]["builds"][1]["sha256"] = "b" * 64
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(json.dumps(manifest))
            result = self.run_verifier("evidence-ok.json", path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("two-build", result.stderr)


if __name__ == "__main__":
    unittest.main()
