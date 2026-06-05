from __future__ import annotations

import os
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import run_swift_runtime


class Completed:
    returncode = 0


class RunSwiftRuntimeTests(unittest.TestCase):
    def run_helper(self, args: list[str], env: dict[str, str]) -> int:
        with (
            patch.dict(os.environ, env, clear=False),
            patch.object(sys, "argv", ["run_swift_runtime.py", *args]),
            redirect_stdout(StringIO()),
            redirect_stderr(StringIO()),
        ):
            return run_swift_runtime.main()

    def test_unknown_case_is_noop_before_toolchain_lookup(self) -> None:
        exit_code = self.run_helper(["01_basic_types", "missing.swift"], {})

        self.assertEqual(exit_code, 0)

    def test_missing_swiftc_fails_clearly(self) -> None:
        with patch.object(run_swift_runtime.shutil, "which", return_value=None):
            exit_code = self.run_helper(["07_indexes", "missing.swift"], {})

        self.assertEqual(exit_code, 1)

    def test_compiles_harness_and_excludes_swiftdata_by_default(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            generated = root / "schema.swift"
            swiftdata = root / "schema_swiftdata.swift"
            generated.write_text("struct SchemaContainer {}\n", encoding="utf-8")
            swiftdata.write_text("import SwiftData\n", encoding="utf-8")
            calls: list[list[str]] = []
            harness_text: dict[str, str] = {}

            def fake_run(cmd: list[str]) -> Completed:
                calls.append(cmd)
                if cmd and cmd[0] == "FakeSwiftc":
                    harness_path = next(path for path in cmd if str(path).endswith("PolygenSwiftRuntimeTest.swift"))
                    harness_text["value"] = Path(harness_path).read_text(encoding="utf-8")
                return Completed()

            env = {
                "SWIFTC": "FakeSwiftc",
                "POLYGEN_SWIFT_COMPILER_ARGS": "-warnings-as-errors",
            }
            with patch.object(run_swift_runtime.subprocess, "run", fake_run):
                exit_code = self.run_helper(["11_relations_indexes", str(root / "*.swift")], env)

        self.assertEqual(exit_code, 0)
        self.assertEqual(calls[0][0], "FakeSwiftc")
        self.assertIn("-warnings-as-errors", calls[0])
        self.assertIn(str(generated), calls[0])
        self.assertNotIn(str(swiftdata), calls[0])
        self.assertIn("-o", calls[0])
        self.assertEqual(calls[1][0], calls[0][calls[0].index("-o") + 1])
        self.assertIn("BinaryRefDocument.fromContainer(container)", harness_text["value"])
        self.assertIn("findByAuthorIdStatus", harness_text["value"])

    def test_runtime_assertion_set_covers_kotlin_parity_cases(self) -> None:
        self.assertTrue({"06_arrays_and_optionals", "08_complex_schema", "10_pack_embed"}.issubset(run_swift_runtime.RUNTIME_TESTS))
        self.assertIn("loadTestCollectionsArrayTestsFromCsv", run_swift_runtime.RUNTIME_TESTS["06_arrays_and_optionals"])
        self.assertIn('constraintType == "Regex"', run_swift_runtime.RUNTIME_TESTS["08_complex_schema"])
        self.assertIn("TestPackEmbedPosition.unpack", run_swift_runtime.RUNTIME_TESTS["10_pack_embed"])

    def test_can_include_swiftdata_when_requested(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            generated = root / "schema.swift"
            swiftdata = root / "schema_swiftdata.swift"
            generated.write_text("struct SchemaContainer {}\n", encoding="utf-8")
            swiftdata.write_text("import SwiftData\n", encoding="utf-8")
            calls: list[list[str]] = []

            def fake_run(cmd: list[str]) -> Completed:
                calls.append(cmd)
                return Completed()

            env = {
                "SWIFTC": "FakeSwiftc",
                "POLYGEN_SWIFT_INCLUDE_SWIFTDATA": "1",
            }
            with patch.object(run_swift_runtime.subprocess, "run", fake_run):
                exit_code = self.run_helper(["07_indexes", str(root / "*.swift")], env)

        self.assertEqual(exit_code, 0)
        self.assertIn(str(generated), calls[0])
        self.assertIn(str(swiftdata), calls[0])


if __name__ == "__main__":
    unittest.main()
