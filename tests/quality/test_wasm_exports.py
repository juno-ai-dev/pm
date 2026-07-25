import importlib.util
import pathlib
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "release" / "verify-wasm-exports.py"
SPEC = importlib.util.spec_from_file_location("verify_wasm_exports", SCRIPT)
assert SPEC and SPEC.loader
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


def uleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        out.append(byte | (0x80 if value else 0))
        if not value:
            return bytes(out)


def section(identifier: int, payload: bytes) -> bytes:
    return bytes([identifier]) + uleb(len(payload)) + payload


def wasm_with_export_descriptors(*exports: tuple[str, int]) -> bytes:
    function_count = sum(kind == 0 for _, kind in exports)
    type_section = section(1, b"\x01\x60\x00\x00") if function_count else b""
    function_section = section(3, uleb(function_count) + b"\x00" * function_count) if function_count else b""
    memory_section = section(5, b"\x01\x00\x01") if any(kind == 2 for _, kind in exports) else b""
    payload = bytearray(uleb(len(exports)))
    function_index = 0
    for name, kind in exports:
        encoded = name.encode()
        index = function_index if kind == 0 else 0
        payload += uleb(len(encoded)) + encoded + bytes([kind]) + uleb(index)
        if kind == 0:
            function_index += 1
    export_section = section(7, bytes(payload))
    bodies = b"".join(b"\x02\x00\x0b" for _ in range(function_count))
    code_section = section(10, uleb(function_count) + bodies) if function_count else b""
    return module.MAGIC_AND_VERSION + type_section + function_section + memory_section + export_section + code_section


def wasm_with_exports(*names: str) -> bytes:
    return wasm_with_export_descriptors(*((name, 0) for name in names))


class WasmExportTests(unittest.TestCase):
    def test_reads_contract_exports(self):
        data = wasm_with_exports("memory", "instantiate", "execute", "query")
        self.assertEqual(
            module.exported_names(data),
            {"memory", "instantiate", "execute", "query"},
        )

    def test_rejects_missing_entry_point(self):
        with tempfile.TemporaryDirectory() as tmp:
            artifact = pathlib.Path(tmp) / "contract.wasm"
            artifact.write_bytes(wasm_with_exports("memory", "allocate"))
            with self.assertRaisesRegex(ValueError, "instantiate, query"):
                module.verify(artifact, {"instantiate", "query"})

    def test_rejects_cross_contract_checksum_collision(self):
        with tempfile.TemporaryDirectory() as tmp:
            first = pathlib.Path(tmp) / "first.wasm"
            second = pathlib.Path(tmp) / "second.wasm"
            first.write_bytes(wasm_with_exports("instantiate"))
            second.write_bytes(first.read_bytes())
            with self.assertRaisesRegex(ValueError, "checksum collision"):
                module.verify_distinct([first, second])

    def test_accepts_distinct_contract_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            first = pathlib.Path(tmp) / "first.wasm"
            second = pathlib.Path(tmp) / "second.wasm"
            first.write_bytes(wasm_with_exports("instantiate"))
            second.write_bytes(wasm_with_exports("instantiate", "query"))
            module.verify_distinct([first, second])

    def test_rejects_non_function_entry_point_name(self):
        data = wasm_with_export_descriptors(("instantiate", 2))
        self.assertEqual(module.exported_names(data), set())

    def test_rejects_semantically_malformed_module(self):
        malformed_type = (
            module.MAGIC_AND_VERSION
            + section(1, b"\x01\xff")
            + section(7, b"\x01\x0binstantiate\x00\x00")
        )
        out_of_bounds = module.MAGIC_AND_VERSION + section(7, b"\x01\x0binstantiate\x00\x00")
        for data in (malformed_type, out_of_bounds):
            with self.subTest(data=data):
                with tempfile.TemporaryDirectory() as tmp:
                    artifact = pathlib.Path(tmp) / "contract.wasm"
                    artifact.write_bytes(data)
                    with self.assertRaisesRegex(ValueError, "invalid Wasm"):
                        module.verify(artifact, {"instantiate"})

    def test_accepts_valid_data_count_before_code(self):
        type_section = section(1, b"\x01\x60\x00\x00")
        function_section = section(3, b"\x01\x00")
        export_section = section(7, b"\x01\x0binstantiate\x00\x00")
        data_count = section(12, b"\x00")
        code_section = section(10, b"\x01\x02\x00\x0b")
        data = module.MAGIC_AND_VERSION + type_section + function_section + export_section + data_count + code_section
        with tempfile.TemporaryDirectory() as tmp:
            artifact = pathlib.Path(tmp) / "contract.wasm"
            artifact.write_bytes(data)
            module.verify(artifact, {"instantiate"})

    def test_rejects_oversized_u32_leb(self):
        data = module.MAGIC_AND_VERSION + b"\x07\x80\x80\x80\x80\x80\x00"
        with self.assertRaisesRegex(ValueError, "oversized|exceeds"):
            module.exported_names(data)

    def test_rejects_unknown_section(self):
        with self.assertRaisesRegex(ValueError, "unknown Wasm section"):
            module.exported_names(module.MAGIC_AND_VERSION + b"\xff\x00")

    def test_rejects_duplicate_export_section(self):
        export = section(7, b"\x00")
        with self.assertRaisesRegex(ValueError, "duplicate Wasm section"):
            module.exported_names(module.MAGIC_AND_VERSION + export + export)

    def test_rejects_out_of_order_sections(self):
        with self.assertRaisesRegex(ValueError, "out-of-order"):
            module.exported_names(module.MAGIC_AND_VERSION + b"\x07\x01\x00\x01\x01\x00")

    def test_rejects_truncated_module(self):
        with self.assertRaisesRegex(ValueError, "truncated"):
            module.exported_names(module.MAGIC_AND_VERSION + b"\x07\x10\x00")


if __name__ == "__main__":
    unittest.main()
