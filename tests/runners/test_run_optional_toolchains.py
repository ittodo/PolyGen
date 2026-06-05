from __future__ import annotations

import os
import sys
import unittest
from contextlib import redirect_stdout
from io import StringIO
from unittest.mock import patch

sys.dont_write_bytecode = True

import check_optional_toolchains
import run_optional_toolchains


class Completed:
    def __init__(self, returncode: int = 0) -> None:
        self.returncode = returncode


def status(name: str, ready: bool) -> check_optional_toolchains.TargetStatus:
    checks = [check_optional_toolchains.Check("tool", ready, "ok" if ready else "missing")]
    return check_optional_toolchains.TargetStatus(name, ready, checks, f"run {name}")


class RunOptionalToolchainsTests(unittest.TestCase):
    def run_script(self, args: list[str], statuses: dict[str, bool]) -> tuple[int, str, list[tuple[list[str], str]]]:
        calls: list[tuple[list[str], str]] = []

        def fake_run(cmd: list[str], cwd: object, env: dict[str, str]) -> Completed:
            active_env = next((key for key, value in env.items() if key.startswith("POLYGEN_") and value == "1"), "")
            calls.append((cmd, active_env))
            return Completed(0)

        output = StringIO()
        with (
            patch.object(sys, "argv", ["run_optional_toolchains.py", *args]),
            patch.object(run_optional_toolchains, "target_status", lambda target: status(target.name, statuses[target.name])),
            patch.object(run_optional_toolchains.subprocess, "run", fake_run),
            redirect_stdout(output),
        ):
            exit_code = run_optional_toolchains.main()
        return exit_code, output.getvalue(), calls

    def test_skips_missing_targets_by_default(self) -> None:
        exit_code, output, calls = self.run_script([], {"kotlin": False, "swift": False, "unreal": False})

        self.assertEqual(exit_code, 0)
        self.assertEqual(calls, [])
        self.assertIn("SKIP kotlin", output)
        self.assertIn("No optional toolchain gates were ready to run.", output)

    def test_fail_on_missing_returns_nonzero(self) -> None:
        exit_code, output, calls = self.run_script(["--fail-on-missing"], {"kotlin": False, "swift": False, "unreal": False})

        self.assertEqual(exit_code, 1)
        self.assertEqual(calls, [])
        self.assertIn("SKIP unreal", output)

    def test_runs_ready_targets_with_opt_in_env(self) -> None:
        exit_code, output, calls = self.run_script([], {"kotlin": True, "swift": False, "unreal": True})

        self.assertEqual(exit_code, 0)
        self.assertEqual([env_name for _, env_name in calls], ["POLYGEN_KOTLIN_RUNTIME", "POLYGEN_UNREAL_COMPILE"])
        self.assertIn("RUN kotlin", output)
        self.assertIn("PASSED unreal", output)

    def test_passes_discovered_status_env_to_ready_runner(self) -> None:
        captured_env: dict[str, str] = {}

        def fake_status(target: run_optional_toolchains.OptionalTarget) -> check_optional_toolchains.TargetStatus:
            return check_optional_toolchains.TargetStatus(
                target.name,
                True,
                [check_optional_toolchains.Check("tool", True, "ok")],
                f"run {target.name}",
                {"SWIFTC": "discovered-swiftc", "SDKROOT": "discovered-sdk", "PATH": "discovered-path"},
            )

        def fake_run(cmd: list[str], cwd: object, env: dict[str, str]) -> Completed:
            captured_env.update(env)
            return Completed(0)

        output = StringIO()
        with (
            patch.object(sys, "argv", ["run_optional_toolchains.py", "swift"]),
            patch.object(run_optional_toolchains, "target_status", fake_status),
            patch.object(run_optional_toolchains.subprocess, "run", fake_run),
            redirect_stdout(output),
        ):
            exit_code = run_optional_toolchains.main()

        self.assertEqual(exit_code, 0)
        self.assertEqual(captured_env["SWIFTC"], "discovered-swiftc")
        self.assertEqual(captured_env["SDKROOT"], "discovered-sdk")
        self.assertEqual(captured_env["PATH"], "discovered-path")
        self.assertEqual(captured_env["POLYGEN_SWIFT_RUNTIME"], "1")

    def test_dry_run_does_not_spawn_ready_runner(self) -> None:
        exit_code, output, calls = self.run_script(["--dry-run", "swift"], {"kotlin": False, "swift": True, "unreal": False})

        self.assertEqual(exit_code, 0)
        self.assertEqual(calls, [])
        self.assertIn("RUN swift", output)
        self.assertNotIn("No optional toolchain gates were ready to run.", output)

    def test_windows_runner_command_uses_call_for_batch_isolation(self) -> None:
        with patch.object(run_optional_toolchains.os, "name", "nt"):
            cmd = run_optional_toolchains.runner_command("kotlin")

        self.assertEqual(cmd[:4], ["cmd", "/d", "/c", "call"])
        self.assertTrue(cmd[-1].endswith(r"kotlin\run_tests.bat"))

    def test_posix_runner_command_uses_bash(self) -> None:
        with patch.object(run_optional_toolchains.os, "name", "posix"):
            cmd = run_optional_toolchains.runner_command("swift")

        self.assertEqual(cmd[0], "bash")
        self.assertTrue(cmd[-1].endswith("swift/run_tests.sh") or cmd[-1].endswith(r"swift\run_tests.sh"))


if __name__ == "__main__":
    unittest.main()
