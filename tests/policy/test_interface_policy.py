import json
import re
import unittest
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY_DIR = ROOT / "docs" / "prediction-market" / "interface-policy"
STATUSES = {"Listed", "Unlisted", "Warning", "Duplicate", "Unsafe"}
HUMAN_LISTING_ROLES = {"reviewer", "appeal_reviewer"}
SECRET_PATTERNS = (
    re.compile(r"(?i)seed\s+phrase"),
    re.compile(r"(?i)private\s+key"),
    re.compile(r"(?i)(api|auth)[_-]?token\s*[:=]"),
    re.compile(r"(?i)password\s*[:=]"),
    re.compile(r"\b(?:[a-z]+\s+){11,23}(?:about|abandon|zoo)\b", re.I),
)


def load(name):
    return json.loads((POLICY_DIR / name).read_text())


def validate_fixture(data):
    required = {
        "policy_id",
        "policy_version",
        "promoted_discovery_enabled",
        "report_intake_enabled",
        "retention",
        "transitions",
        "reports",
        "appeals",
    }
    if set(data) != required:
        raise ValueError("fixture has missing or unknown top-level fields")
    if data["policy_id"] != "juno-pm-reference-discovery":
        raise ValueError("wrong policy id")
    if data["retention"]["status"] != "counsel_approved":
        if data["promoted_discovery_enabled"] or data["report_intake_enabled"]:
            raise ValueError("draft retention must keep launch features disabled")

    events = {}
    market_events = {}
    for event in data["transitions"]:
        required_event = {
            "event_id",
            "market",
            "from",
            "to",
            "visibility",
            "quarantined",
            "actor_id",
            "actor_role",
            "occurred_at",
            "reason_code",
            "reason",
            "policy_version",
            "evidence_refs",
        }
        if not required_event.issubset(event):
            raise ValueError("transition audit fields are incomplete")
        if event["event_id"] in events:
            raise ValueError("duplicate event id")
        if event["from"] not in STATUSES or event["to"] not in STATUSES:
            raise ValueError("unknown status")
        datetime.fromisoformat(event["occurred_at"].replace("Z", "+00:00"))
        if not event["reason"].strip() or not event["evidence_refs"]:
            raise ValueError("transition reason and evidence are required")
        if event["policy_version"] != data["policy_version"]:
            raise ValueError("transition policy version mismatch")
        if event["to"] == "Listed":
            if event["actor_role"] not in HUMAN_LISTING_ROLES:
                raise ValueError("automation cannot list")
            if event["visibility"] != "promoted" or event["quarantined"]:
                raise ValueError("listed visibility is inconsistent")
        if event["to"] in {"Unlisted", "Duplicate", "Unsafe"}:
            if event["visibility"] != "exact_address_only" or not event["quarantined"]:
                raise ValueError("non-promoted status must remain quarantined")
        key = (event["market"]["chain_id"], event["market"]["address"])
        previous = market_events.get(key)
        if previous is None:
            if not (
                event["from"] == event["to"] == "Unlisted"
                and event["visibility"] == "exact_address_only"
                and event["quarantined"]
                and event["reason_code"] == "pending_review"
            ):
                raise ValueError("first observation must be default quarantine")
        elif event["from"] != previous["to"]:
            raise ValueError("transition chain is discontinuous")
        market_events[key] = event
        events[event["event_id"]] = event

    for report in data["reports"]:
        allowed = {
            "report_id", "market", "category", "description", "evidence_refs",
            "submitted_at", "status", "follow_up_consent",
        }
        if set(report) != allowed:
            raise ValueError("report contains personal or unknown fields")
        serialized = json.dumps(report, sort_keys=True)
        if any(pattern.search(serialized) for pattern in SECRET_PATTERNS):
            raise ValueError("report contains credential material")

    for appeal in data["appeals"]:
        challenged = events.get(appeal["challenged_event_id"])
        if challenged is None:
            raise ValueError("appeal references unknown transition")
        if appeal["status"] != "pending":
            reviewer = appeal.get("reviewer_actor_id")
            if not reviewer or reviewer == challenged["actor_id"]:
                raise ValueError("appeal reviewer is not independent")
            if appeal["status"] in {"modified", "reversed"}:
                disposition = events.get(appeal.get("disposition_transition_id"))
                if disposition is None or disposition["actor_id"] != reviewer:
                    raise ValueError("appeal disposition transition is missing")


class InterfacePolicyTest(unittest.TestCase):
    def test_schema_is_parseable_and_closed(self):
        schema = load("interface-policy.schema.json")
        self.assertEqual(schema["$schema"], "https://json-schema.org/draft/2020-12/schema")
        self.assertFalse(schema["additionalProperties"])
        self.assertFalse(schema["$defs"]["transition"]["additionalProperties"])
        self.assertFalse(schema["$defs"]["report"]["additionalProperties"])

    def test_valid_fixture_is_fail_closed_and_consistent(self):
        fixture = load("fixtures/valid-policy.json")
        validate_fixture(fixture)
        self.assertFalse(fixture["promoted_discovery_enabled"])
        self.assertFalse(fixture["report_intake_enabled"])

    def test_secret_report_fixture_is_rejected(self):
        fixture = load("fixtures/invalid-secret-report.json")
        with self.assertRaisesRegex(ValueError, "credential material"):
            validate_fixture(fixture)

    def test_automation_cannot_list(self):
        fixture = load("fixtures/valid-policy.json")
        fixture["transitions"][1]["actor_role"] = "automation"
        with self.assertRaisesRegex(ValueError, "automation cannot list"):
            validate_fixture(fixture)

    def test_appeal_reviewer_must_be_independent(self):
        fixture = load("fixtures/valid-policy.json")
        fixture["appeals"][0]["reviewer_actor_id"] = "actor-reviewer-alpha"
        with self.assertRaisesRegex(ValueError, "not independent"):
            validate_fixture(fixture)

    def test_unlisting_cannot_be_promoted(self):
        fixture = load("fixtures/valid-policy.json")
        fixture["transitions"][0]["visibility"] = "promoted"
        with self.assertRaisesRegex(ValueError, "remain quarantined"):
            validate_fixture(fixture)


if __name__ == "__main__":
    unittest.main()
