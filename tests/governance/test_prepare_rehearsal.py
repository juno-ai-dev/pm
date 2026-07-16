import base64
import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "prepare_rehearsal", ROOT / "scripts/governance/prepare_rehearsal.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

GOV = "juno10d07y265gmmuvt4z0w9aw880jnsr700jvss730"
MARKET = "juno1fl48vsnmsdzcv85q5d2q4z5ajdha8yu3rf257t"
ORACLE = "juno1jv65s3grqf6v6jl3dp4t6c9t9rk99cd83d88wr"
PAYEE = "juno17xpfvakm2amg962yls6f84z3kell8c5lxtqmvp"
QID = base64.b64encode(bytes(range(32))).decode()
ANSWER = base64.b64encode(b"\x00" * 31 + b"\x01").decode()


def valid_request():
    return {
        "chain_id": "juno-1",
        "observed_unix": 1_800_000_000,
        "governance_module": GOV,
        "market": MARKET,
        "oracle": ORACLE,
        "question_id": QID,
        "answer": ANSWER,
        "payee": PAYEE,
        "challenge_deadline_unix": 1_800_000_100,
        "market_state": {
            "status": "pending_arbitration",
            "governance": GOV,
            "oracle": ORACLE,
            "question_id": QID,
            "deadline_unix": 1_800_000_100,
        },
        "oracle_state": {
            "status": "pending_arbitration",
            "arbitrator": MARKET,
            "question_id": QID,
            "deadline_unix": 1_800_000_100,
        },
        "title": "Rehearsal-only PM arbitration verdict",
        "summary": "Authorized rehearsal evidence only; no production market.",
    }


class RehearsalPacketTests(unittest.TestCase):
    def assert_rejected(self, mutate):
        request = valid_request()
        mutate(request)
        with self.assertRaises(MODULE.PacketError):
            MODULE.build_packet(request)

    def test_builds_one_funds_free_governance_execute(self):
        packet = MODULE.build_packet(valid_request())
        MODULE.validate_packet(packet)
        message = packet["proposal"]["messages"][0]
        self.assertEqual(message["sender"], GOV)
        self.assertEqual(message["contract"], MARKET)
        self.assertEqual(message["funds"], [])
        decoded = json.loads(base64.b64decode(message["msg"], validate=True))
        self.assertEqual(
            decoded,
            {"governance_verdict": {"question_id": QID, "answer": ANSWER, "payee": PAYEE}},
        )

    def test_output_is_deterministic_and_round_trips(self):
        first = MODULE.build_packet(valid_request())
        second = MODULE.build_packet(valid_request())
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "packet.json"
            MODULE._write(str(path), first)
            self.assertEqual(MODULE.validate_packet(json.loads(path.read_text())), first)

    def test_documented_request_schema_is_valid_json(self):
        schema = json.loads(
            (ROOT / "docs/prediction-market/governance-rehearsal/request.schema.json").read_text()
        )
        self.assertEqual(schema["additionalProperties"], False)
        self.assertEqual(set(schema["required"]), set(valid_request()))

    def test_rejects_wrong_chain(self):
        self.assert_rejected(lambda request: request.__setitem__("chain_id", "uni-6"))

    def test_rejects_invalid_or_substituted_addresses(self):
        self.assert_rejected(lambda request: request.__setitem__("governance_module", "juno1notchecksummed"))
        self.assert_rejected(lambda request: request["market_state"].__setitem__("governance", PAYEE))
        self.assert_rejected(lambda request: request["market_state"].__setitem__("oracle", MARKET))
        self.assert_rejected(lambda request: request["oracle_state"].__setitem__("arbitrator", GOV))

    def test_rejects_wrong_or_noncanonical_question(self):
        self.assert_rejected(lambda request: request.__setitem__("question_id", base64.b64encode(b"short").decode()))
        self.assert_rejected(lambda request: request["market_state"].__setitem__("question_id", ANSWER))
        self.assert_rejected(lambda request: request["oracle_state"].__setitem__("question_id", ANSWER))

    def test_rejects_malformed_answer_and_payee(self):
        self.assert_rejected(lambda request: request.__setitem__("answer", "not-base64"))
        self.assert_rejected(lambda request: request.__setitem__("answer", base64.b64encode(b"arbitrary").decode()))
        self.assert_rejected(lambda request: request.__setitem__("payee", "juno1invalid"))

    def test_rejects_stale_or_nonpending_state(self):
        self.assert_rejected(lambda request: request.__setitem__("observed_unix", 1_800_000_100))
        self.assert_rejected(lambda request: request["market_state"].__setitem__("status", "resolved"))
        self.assert_rejected(lambda request: request["oracle_state"].__setitem__("status", "finalized"))
        self.assert_rejected(lambda request: request["oracle_state"].__setitem__("deadline_unix", 1_800_000_101))

    def test_packet_validator_rejects_funds_sender_contract_and_payload_drift(self):
        for mutate in (
            lambda packet: packet["proposal"]["messages"][0].__setitem__("funds", [{"denom": "ujuno", "amount": "1"}]),
            lambda packet: packet["proposal"]["messages"][0].__setitem__("sender", PAYEE),
            lambda packet: packet["proposal"]["messages"][0].__setitem__("contract", ORACLE),
            lambda packet: packet["proposal"]["messages"][0].__setitem__("msg", base64.b64encode(b"{}").decode()),
        ):
            packet = copy.deepcopy(MODULE.build_packet(valid_request()))
            mutate(packet)
            with self.assertRaises(MODULE.PacketError):
                MODULE.validate_packet(packet)

    def test_packet_validator_rejects_multiple_or_malformed_messages(self):
        packet = MODULE.build_packet(valid_request())
        packet["proposal"]["messages"].append(copy.deepcopy(packet["proposal"]["messages"][0]))
        with self.assertRaises(MODULE.PacketError):
            MODULE.validate_packet(packet)

    def test_rejects_duplicate_json_fields_and_noncanonical_payload_base64(self):
        packet = MODULE.build_packet(valid_request())
        preflight = packet["preflight"]
        duplicate = (
            '{"governance_verdict":{"question_id":'
            + json.dumps(preflight["question_id"])
            + ',"answer":"shadowed","answer":'
            + json.dumps(preflight["answer"])
            + ',"payee":'
            + json.dumps(preflight["payee"])
            + "}}"
        )
        packet["proposal"]["messages"][0]["msg"] = base64.b64encode(duplicate.encode()).decode()
        with self.assertRaises(MODULE.PacketError):
            MODULE.validate_packet(packet)

        packet = MODULE.build_packet(valid_request())
        encoded = packet["proposal"]["messages"][0]["msg"]
        alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        replacement = alphabet[(alphabet.index(encoded[-2]) + 1) % 64]
        packet["proposal"]["messages"][0]["msg"] = encoded[:-2] + replacement + "="
        with self.assertRaises(MODULE.PacketError):
            MODULE.validate_packet(packet)

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"chain_id":"juno-1","chain_id":"uni-6"}')
            with self.assertRaises(MODULE.PacketError):
                MODULE._load(str(path))


if __name__ == "__main__":
    unittest.main()
