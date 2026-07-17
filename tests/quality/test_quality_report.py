import importlib.util
import json
import pathlib
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
VERIFY = ROOT / "scripts/quality/verify-report.py"
BASE = ROOT / "quality/gas-storage-report.json"


class QualityReportTest(unittest.TestCase):
    def run_report(self, mutate=None):
        report = json.loads(BASE.read_text())
        if mutate:
            mutate(report)
        with tempfile.NamedTemporaryFile("w", suffix=".json") as handle:
            json.dump(report, handle)
            handle.flush()
            return subprocess.run(["python3", str(VERIFY), handle.name], text=True, capture_output=True)

    def test_unmeasured_template_is_explicit_review_gate(self):
        result = self.run_report()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("explicit canary review gate", result.stdout)

    def test_missing_scenario_fails(self):
        result = self.run_report(lambda r: r["scenarios"].pop())
        self.assertNotEqual(result.returncode, 0)

    def test_fake_partial_measurement_fails(self):
        result = self.run_report(lambda r: r["scenarios"][0].update(gas_used=1))
        self.assertNotEqual(result.returncode, 0)

    def test_unmeasured_storage_and_provenance_fail(self):
        result = self.run_report(lambda r: r["scenarios"][0].update(storage_delta_bytes=1))
        self.assertNotEqual(result.returncode, 0)
        result = self.run_report(lambda r: r.update(source_commit="a" * 40))
        self.assertNotEqual(result.returncode, 0)

    def test_scenario_profile_drift_fails(self):
        result = self.run_report(lambda r: r["scenarios"][0].update(bound="maximum"))
        self.assertNotEqual(result.returncode, 0)

    def test_measured_report_enforces_headroom(self):
        def measured(report):
            report.update(status="measured", chain="juno-1@height", binary="junod vX", source_commit="a" * 40)
            for scenario in report["scenarios"]:
                scenario["gas_used"] = 1
                if scenario["class"] == "query":
                    scenario["response_bytes"] = 1
                else:
                    scenario["storage_delta_bytes"] = 1
            report["scenarios"][0]["gas_used"] = 5_000_000
            report["review"] = {"required": False}
        result = self.run_report(measured)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("named explicit review", result.stderr)

    def test_measured_report_must_match_release_commit(self):
        def measured(report):
            report.update(status="measured", chain="juno-1@height", binary="junod vX", source_commit="a" * 40)
            for scenario in report["scenarios"]:
                scenario["gas_used"] = 1
                if scenario["class"] == "query":
                    scenario["response_bytes"] = 1
                else:
                    scenario["storage_delta_bytes"] = 1
            report["review"] = {"required": False}

        report = json.loads(BASE.read_text())
        measured(report)
        manifest = {
            "source_commit": "b" * 40,
            "artifacts": [
                {"file": name, "size_bytes": 1}
                for name in ("binary_market.wasm", "cw_reality.wasm", "market_factory.wasm")
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            report_path = pathlib.Path(directory) / "report.json"
            manifest_path = pathlib.Path(directory) / "manifest.json"
            report_path.write_text(json.dumps(report))
            manifest_path.write_text(json.dumps(manifest))
            result = subprocess.run(
                ["python3", str(VERIFY), str(report_path), str(manifest_path)],
                text=True,
                capture_output=True,
            )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match", result.stderr)


if __name__ == "__main__":
    unittest.main()
