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

import run_kotlin_runtime


class Completed:
    returncode = 0


class RunKotlinRuntimeTests(unittest.TestCase):
    def run_helper(self, args: list[str], env: dict[str, str]) -> int:
        with (
            patch.dict(os.environ, env, clear=False),
            patch.object(sys, "argv", ["run_kotlin_runtime.py", *args]),
            redirect_stdout(StringIO()),
            redirect_stderr(StringIO()),
        ):
            return run_kotlin_runtime.main()

    def test_unknown_case_is_noop_before_toolchain_lookup(self) -> None:
        exit_code = self.run_helper(["01_basic_types", "missing.kt"], {})

        self.assertEqual(exit_code, 0)

    def test_missing_kotlinc_fails_clearly(self) -> None:
        with patch.object(run_kotlin_runtime.shutil, "which", return_value=None):
            exit_code = self.run_helper(["07_indexes", "missing.kt"], {})

        self.assertEqual(exit_code, 1)

    def test_compiles_harness_and_runs_main_with_configured_tools(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            generated = root / "schema.kt"
            generated.write_text("class SchemaContainer\n", encoding="utf-8")
            calls: list[list[str]] = []
            harness_text: dict[str, str] = {}

            def fake_run(cmd: list[str]) -> Completed:
                calls.append(cmd)
                if cmd and cmd[0] == "FakeKotlinc":
                    harness_path = next(path for path in cmd if str(path).endswith("PolygenKotlinRuntimeTest.kt"))
                    harness_text["value"] = Path(harness_path).read_text(encoding="utf-8")
                return Completed()

            env = {
                "KOTLINC": "FakeKotlinc",
                "JAVA": "FakeJava",
                "POLYGEN_KOTLIN_CLASSPATH": "deps.jar",
                "POLYGEN_KOTLIN_COMPILER_ARGS": "-Xcontext-receivers -Werror",
            }
            with patch.object(run_kotlin_runtime.subprocess, "run", fake_run):
                exit_code = self.run_helper(["11_relations_indexes", str(generated)], env)

        self.assertEqual(exit_code, 0)
        self.assertEqual(calls[0][0], "FakeKotlinc")
        self.assertIn("-Xcontext-receivers", calls[0])
        self.assertIn("-Werror", calls[0])
        self.assertIn("-classpath", calls[0])
        self.assertIn("deps.jar", calls[0])
        self.assertIn(str(generated), calls[0])
        self.assertIn("-include-runtime", calls[0])
        self.assertEqual(calls[1][0:3], ["FakeJava", "-cp", calls[1][2]])
        self.assertIn("deps.jar", calls[1][2])
        self.assertEqual(calls[1][-1], "PolygenKotlinRuntimeTestKt")
        self.assertIn("BinaryRefDocument.fromContainer(container)", harness_text["value"])
        self.assertIn("findByAuthorIdStatus", harness_text["value"])
        self.assertIn("10_pack_embed", run_kotlin_runtime.RUNTIME_TESTS)
        self.assertIn("06_arrays_and_optionals", run_kotlin_runtime.RUNTIME_TESTS)

    def test_windows_compiler_args_preserve_backslash_paths(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows command-line parsing")

        args = run_kotlin_runtime.split_env_args(r'-Xplugin=C:\Users\sksk1\AppData\Local\Temp\plugin.jar -Werror')

        self.assertEqual(
            args,
            [r"-Xplugin=C:\Users\sksk1\AppData\Local\Temp\plugin.jar", "-Werror"],
        )

    def test_windows_multi_jar_classpath_uses_argfile(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows argfile path")

        with TemporaryDirectory() as temp:
            temp_dir = Path(temp)
            calls: list[list[str]] = []
            argfile = temp_dir / "kotlinc.args"

            def fake_run(cmd: list[str]) -> Completed:
                calls.append(cmd)
                return Completed()

            args = ["-classpath", r"C:\deps\a.jar;C:\deps\b.jar", "schema.kt"]
            with (
                patch.object(run_kotlin_runtime.subprocess, "run", fake_run),
                redirect_stdout(StringIO()),
            ):
                exit_code = run_kotlin_runtime.run_kotlinc("FakeKotlinc", args, temp_dir)
            argfile_text = argfile.read_text(encoding="utf-8")

        self.assertEqual(exit_code, 0)
        self.assertEqual(calls, [["FakeKotlinc", f"@{argfile}"]])
        self.assertIn(r"C:\deps\a.jar;C:\deps\b.jar", argfile_text)


if __name__ == "__main__":
    unittest.main()
