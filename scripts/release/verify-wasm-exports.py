#!/usr/bin/env python3
"""Fail closed when a deployable Wasm artifact lacks contract entry points."""
from __future__ import annotations

import argparse
import hashlib
import pathlib
import shutil
import subprocess
import sys

MAGIC_AND_VERSION = b"\x00asm\x01\x00\x00\x00"
SECTION_ORDER = {**{section: section for section in range(1, 10)}, 12: 10, 10: 11, 11: 12}


def read_uleb(data: bytes, offset: int) -> tuple[int, int]:
    """Read a canonical WebAssembly u32 LEB128 value."""
    value = 0
    start = offset
    for index in range(5):
        if offset >= len(data):
            raise ValueError("truncated unsigned LEB128")
        byte = data[offset]
        offset += 1
        if index == 4 and byte & 0xF0:
            raise ValueError("unsigned LEB128 exceeds u32")
        value |= (byte & 0x7F) << (index * 7)
        if byte & 0x80 == 0:
            encoded = data[start:offset]
            canonical = bytearray()
            remaining = value
            while True:
                part = remaining & 0x7F
                remaining >>= 7
                canonical.append(part | (0x80 if remaining else 0))
                if not remaining:
                    break
            if encoded != bytes(canonical):
                raise ValueError("non-canonical unsigned LEB128")
            return value, offset
    raise ValueError("oversized unsigned LEB128")


def exported_names(data: bytes) -> set[str]:
    """Return names exported specifically as functions from a valid section layout."""
    if not data.startswith(MAGIC_AND_VERSION):
        raise ValueError("invalid Wasm magic or version")
    offset = len(MAGIC_AND_VERSION)
    function_exports: set[str] = set()
    seen_sections: set[int] = set()
    last_section_rank = 0
    while offset < len(data):
        section_id = data[offset]
        offset += 1
        if section_id > 12:
            raise ValueError(f"unknown Wasm section id: {section_id}")
        if section_id != 0:
            if section_id in seen_sections:
                raise ValueError(f"duplicate Wasm section id: {section_id}")
            rank = SECTION_ORDER[section_id]
            if rank < last_section_rank:
                raise ValueError("out-of-order Wasm section")
            seen_sections.add(section_id)
            last_section_rank = rank
        size, offset = read_uleb(data, offset)
        end = offset + size
        if end > len(data):
            raise ValueError("truncated Wasm section")
        if section_id == 7:
            count, cursor = read_uleb(data, offset)
            for _ in range(count):
                name_len, cursor = read_uleb(data, cursor)
                name_end = cursor + name_len
                if name_end > end:
                    raise ValueError("truncated export name")
                try:
                    name = data[cursor:name_end].decode("utf-8")
                except UnicodeDecodeError as exc:
                    raise ValueError("invalid UTF-8 export name") from exc
                cursor = name_end
                if cursor >= end:
                    raise ValueError("truncated export descriptor")
                kind = data[cursor]
                cursor += 1
                if kind > 4:
                    raise ValueError(f"unknown export kind: {kind}")
                _, cursor = read_uleb(data, cursor)
                if cursor > end:
                    raise ValueError("truncated export descriptor")
                if kind == 0:
                    function_exports.add(name)
            if cursor != end:
                raise ValueError("malformed export section")
        offset = end
    return function_exports


def verify(path: pathlib.Path, required: set[str]) -> None:
    validator = shutil.which("wasm-tools")
    if validator is None:
        raise ValueError("wasm-tools is required for semantic Wasm validation")
    result = subprocess.run(
        [validator, "validate", str(path)],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown validation failure"
        raise ValueError(f"{path}: invalid Wasm: {detail}")
    exports = exported_names(path.read_bytes())
    missing = sorted(required - exports)
    if missing:
        raise ValueError(f"{path}: missing required exports: {', '.join(missing)}")


def verify_distinct(paths: list[pathlib.Path]) -> None:
    by_digest: dict[str, list[pathlib.Path]] = {}
    for path in paths:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        by_digest.setdefault(digest, []).append(path)
    collisions = [group for group in by_digest.values() if len(group) > 1]
    if collisions:
        names = "; ".join(", ".join(str(path) for path in group) for group in collisions)
        raise ValueError(f"cross-contract checksum collision: {names}")


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    exports_parser = subparsers.add_parser("exports")
    exports_parser.add_argument("artifact", type=pathlib.Path)
    exports_parser.add_argument("required", nargs="+")
    distinct_parser = subparsers.add_parser("distinct")
    distinct_parser.add_argument("artifacts", nargs="+", type=pathlib.Path)
    args = parser.parse_args()
    try:
        if args.command == "exports":
            verify(args.artifact, set(args.required))
        else:
            verify_distinct(args.artifacts)
    except (OSError, ValueError) as exc:
        print(exc, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
