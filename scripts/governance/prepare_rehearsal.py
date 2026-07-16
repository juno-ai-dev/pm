#!/usr/bin/env python3
"""Build and validate an unsigned Juno x/gov arbitration rehearsal packet.

This tool is deliberately offline. It never invokes junod, signs, or broadcasts.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import json
from pathlib import Path
from typing import Any

FORMAT = "juno-pm-gov-rehearsal/1"
CHAIN_ID = "juno-1"
MSG_EXECUTE = "/cosmwasm.wasm.v1.MsgExecuteContract"
WARNING = "UNSIGNED_UNAUTHORIZED_DO_NOT_BROADCAST"


class PacketError(ValueError):
    pass


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise PacketError(message)


def _polymod(values: list[int]) -> int:
    generators = (0x3B6A57B2, 0x26508E6D, 0x1EA119FA, 0x3D4233DD, 0x2A1462B3)
    chk = 1
    for value in values:
        top = chk >> 25
        chk = ((chk & 0x1FFFFFF) << 5) ^ value
        for index, generator in enumerate(generators):
            if (top >> index) & 1:
                chk ^= generator
    return chk


def _hrp_expand(hrp: str) -> list[int]:
    return [ord(char) >> 5 for char in hrp] + [0] + [ord(char) & 31 for char in hrp]


def valid_juno_address(value: Any) -> bool:
    if not isinstance(value, str) or value.lower() != value or not value.startswith("juno1"):
        return False
    separator = value.rfind("1")
    if separator < 1 or len(value) - separator - 1 < 6 or len(value) > 90:
        return False
    alphabet = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
    try:
        data = [alphabet.index(char) for char in value[separator + 1 :]]
    except ValueError:
        return False
    return _polymod(_hrp_expand(value[:separator]) + data) == 1


def decode_exact_32(value: Any, field: str) -> bytes:
    _require(isinstance(value, str), f"{field} must be base64 text")
    try:
        raw = base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError) as error:
        raise PacketError(f"{field} must be canonical base64") from error
    _require(len(raw) == 32, f"{field} must decode to exactly 32 bytes")
    _require(base64.b64encode(raw).decode() == value, f"{field} must use canonical base64")
    return raw


def validate_request(request: Any) -> dict[str, Any]:
    _require(isinstance(request, dict), "request must be an object")
    expected = {
        "chain_id",
        "observed_unix",
        "governance_module",
        "market",
        "oracle",
        "question_id",
        "answer",
        "payee",
        "challenge_deadline_unix",
        "market_state",
        "oracle_state",
        "title",
        "summary",
    }
    _require(set(request) == expected, "request fields do not match the versioned contract")
    _require(request["chain_id"] == CHAIN_ID, "chain_id must be juno-1")
    for field in ("governance_module", "market", "oracle", "payee"):
        _require(valid_juno_address(request[field]), f"{field} must be a checksummed juno address")
    _require(len({request["governance_module"], request["market"], request["oracle"]}) == 3,
             "governance, market, and oracle identities must be distinct")
    decode_exact_32(request["question_id"], "question_id")
    decode_exact_32(request["answer"], "answer")
    observed = request["observed_unix"]
    deadline = request["challenge_deadline_unix"]
    _require(isinstance(observed, int) and not isinstance(observed, bool) and observed >= 0,
             "observed_unix must be a nonnegative integer")
    _require(isinstance(deadline, int) and not isinstance(deadline, bool),
             "challenge_deadline_unix must be an integer")
    _require(observed < deadline, "verdict preparation must be strictly before the challenge deadline")
    _require(isinstance(request["title"], str) and request["title"].strip(), "title is required")
    _require(isinstance(request["summary"], str) and request["summary"].strip(), "summary is required")

    market = request["market_state"]
    _require(isinstance(market, dict) and set(market) == {"status", "governance", "oracle", "question_id", "deadline_unix"},
             "market_state fields do not match the rehearsal preflight contract")
    _require(market["status"] == "pending_arbitration", "market must be pending arbitration")
    _require(market["governance"] == request["governance_module"], "market governance identity mismatch")
    _require(market["oracle"] == request["oracle"], "market oracle identity mismatch")
    _require(market["question_id"] == request["question_id"], "market question identity mismatch")
    _require(market["deadline_unix"] == deadline, "market challenge deadline mismatch")

    oracle = request["oracle_state"]
    _require(isinstance(oracle, dict) and set(oracle) == {"status", "arbitrator", "question_id", "deadline_unix"},
             "oracle_state fields do not match the rehearsal preflight contract")
    _require(oracle["status"] == "pending_arbitration", "oracle must be pending arbitration")
    _require(oracle["arbitrator"] == request["market"], "oracle arbitrator must be the exact market")
    _require(oracle["question_id"] == request["question_id"], "oracle question identity mismatch")
    _require(oracle["deadline_unix"] == deadline, "oracle deadline mismatch")
    return request


def build_packet(request: dict[str, Any]) -> dict[str, Any]:
    request = validate_request(request)
    execute = {
        "governance_verdict": {
            "question_id": request["question_id"],
            "answer": request["answer"],
            "payee": request["payee"],
        }
    }
    encoded = base64.b64encode(
        json.dumps(execute, sort_keys=True, separators=(",", ":")).encode()
    ).decode()
    return {
        "format": FORMAT,
        "warning": WARNING,
        "preflight": {
            key: request[key]
            for key in (
                "chain_id",
                "observed_unix",
                "governance_module",
                "market",
                "oracle",
                "question_id",
                "answer",
                "payee",
                "challenge_deadline_unix",
                "market_state",
                "oracle_state",
            )
        },
        "proposal": {
            "title": request["title"],
            "summary": request["summary"],
            "metadata": "",
            "messages": [
                {
                    "@type": MSG_EXECUTE,
                    "sender": request["governance_module"],
                    "contract": request["market"],
                    "msg": encoded,
                    "funds": [],
                }
            ],
        },
    }


def validate_packet(packet: Any) -> dict[str, Any]:
    _require(isinstance(packet, dict) and set(packet) == {"format", "warning", "preflight", "proposal"},
             "packet fields do not match the versioned contract")
    _require(packet["format"] == FORMAT and packet["warning"] == WARNING,
             "packet format or safety warning mismatch")
    preflight = packet["preflight"]
    _require(isinstance(preflight, dict), "preflight must be an object")
    proposal = packet["proposal"]
    _require(isinstance(proposal, dict) and set(proposal) == {"title", "summary", "metadata", "messages"},
             "proposal fields do not match the expected gov v1 shape")
    messages = proposal["messages"]
    _require(isinstance(messages, list) and len(messages) == 1, "proposal must contain exactly one message")
    message = messages[0]
    _require(isinstance(message, dict) and set(message) == {"@type", "sender", "contract", "msg", "funds"},
             "inner message fields do not match MsgExecuteContract")
    _require(message["@type"] == MSG_EXECUTE, "inner message must be MsgExecuteContract")
    _require(message["sender"] == preflight.get("governance_module"), "inner sender mismatch")
    _require(message["contract"] == preflight.get("market"), "inner market mismatch")
    _require(message["funds"] == [], "governance verdict must attach no funds")
    try:
        execute = json.loads(base64.b64decode(message["msg"], validate=True))
    except (binascii.Error, json.JSONDecodeError, TypeError) as error:
        raise PacketError("inner msg must be base64-encoded JSON") from error
    expected_execute = {
        "governance_verdict": {
            "question_id": preflight.get("question_id"),
            "answer": preflight.get("answer"),
            "payee": preflight.get("payee"),
        }
    }
    _require(execute == expected_execute, "decoded verdict differs from preflight identities or answer/payee")
    request = dict(preflight)
    request["title"] = proposal["title"]
    request["summary"] = proposal["summary"]
    validate_request(request)
    return packet


def _load(path: str) -> Any:
    return json.loads(Path(path).read_text())


def _write(path: str, value: Any) -> None:
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    build = subparsers.add_parser("build", help="build an unsigned packet")
    build.add_argument("request")
    build.add_argument("output")
    validate = subparsers.add_parser("validate", help="fail closed on a prepared packet")
    validate.add_argument("packet")
    args = parser.parse_args()
    try:
        if args.command == "build":
            _write(args.output, build_packet(_load(args.request)))
            print(f"wrote {WARNING} packet: {args.output}")
        else:
            validate_packet(_load(args.packet))
            print(f"valid {WARNING} packet: {args.packet}")
    except (OSError, json.JSONDecodeError, PacketError) as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
