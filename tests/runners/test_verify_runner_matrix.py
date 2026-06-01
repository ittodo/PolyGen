from __future__ import annotations

import os
import subprocess
import sys
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

sys.dont_write_bytecode = True

import verify_runner_matrix


SCRIPT_DIR = Path(__file__).resolve().parent


def write_run_all_bat(
    root: Path,
    runners: list[str],
    *,
    include_guard: bool = True,
    include_options: bool = True,
    include_help: bool = True,
    include_verify: bool = True,
    include_python_guard: bool = True,
    include_windows_py_fallback: bool = True,
    python_guard_out_of_order: bool = False,
    regression_uses_selected_python: bool = True,
    include_no_bytecode_guard: bool = True,
    no_bytecode_after_invocation: bool = False,
) -> None:
    options = (
        'if /i "%~1"=="--list" goto :list\n'
        ":list\n"
        "echo %DEFAULT_RUNNERS%\n"
        if include_options
        else ""
    )
    help_guard = (
        'if /i "%~1"=="--help" goto :usage\n'
        'if /i "%~1"=="-h" goto :usage\n'
        'if /i "%~1"=="/?" goto :usage\n'
        ":usage\n"
        "echo   tests\\runners\\run_all.bat --list\n"
        "echo   tests\\runners\\run_all.bat --verify\n"
        "echo   tests\\runners\\run_all.bat --help\n"
        if include_help
        else ""
    )
    fallback_guard = (
        "where py >nul 2>nul\n"
        "goto :python_not_found\n"
        'set "PYTHON_BIN=py -3"\n'
        if include_windows_py_fallback
        else ""
    )
    if python_guard_out_of_order:
        python_guard = (
            "where python >nul 2>nul\n"
            "where py >nul 2>nul\n"
            "goto :python_not_found\n"
            'set "PYTHON_BIN=python"\n'
            ":python_not_found\n"
            'set "PYTHON_BIN=py -3"\n'
            "echo FAILED ^(python not found^)\n"
            if include_python_guard
            else ""
        )
    else:
        python_guard = (
            "where python >nul 2>nul\n"
            f"{fallback_guard}"
            'set "PYTHON_BIN=python"\n'
            ":python_not_found\n"
            "echo FAILED ^(python not found^)\n"
            if include_python_guard
            else ""
        )
    no_bytecode_guard = (
        'set "PYTHONDONTWRITEBYTECODE=1"\n'
        if include_no_bytecode_guard
        else ""
    )
    if no_bytecode_after_invocation:
        before_invocations_no_bytecode = ""
        after_invocations_no_bytecode = no_bytecode_guard
    else:
        before_invocations_no_bytecode = no_bytecode_guard
        after_invocations_no_bytecode = ""
    verify = (
        'if /i "%~1"=="--verify" goto :verify\n'
        ":verify\n"
        if include_verify
        else ""
    )
    verify_invocations = (
        '%PYTHON_BIN% "%SCRIPT_DIR%verify_runner_matrix.py"\n'
        'set "VERIFY_EXIT=%ERRORLEVEL%"\n'
        "if %VERIFY_EXIT% neq 0 (\n"
        "    exit /b %VERIFY_EXIT%\n"
        ")\n"
        f'{"%PYTHON_BIN%" if regression_uses_selected_python else "python"} "%SCRIPT_DIR%test_verify_runner_matrix.py"\n'
        if include_verify
        else ""
    )
    guard = (
        ":validate_runner_arg\n"
        "set \"POLYGEN_RUNNER_ARG=%RUNNER_ARG%\"\n"
        "powershell -Command \"if ($env:POLYGEN_RUNNER_ARG -match '^[a-z0-9_-]+$') { exit 0 } exit 1\"\n"
        "echo   FAILED ^(invalid runner name^)\n"
        if include_guard
        else ""
    )
    root.joinpath("run_all.bat").write_text(
        f'@echo off\nset "DEFAULT_RUNNERS={" ".join(runners)}"\n{options}{help_guard}{verify}{python_guard}{before_invocations_no_bytecode}{verify_invocations}{after_invocations_no_bytecode}{guard}',
        encoding="utf-8",
    )


def write_actual_run_all_bat(root: Path, runners: list[str]) -> None:
    text = SCRIPT_DIR.joinpath("run_all.bat").read_text(encoding="utf-8")
    default_line = next(
        line for line in text.splitlines() if line.startswith('set "DEFAULT_RUNNERS=')
    )
    text = text.replace(
        default_line,
        f'set "DEFAULT_RUNNERS={" ".join(runners)}"',
    )
    root.joinpath("run_all.bat").write_text(text, encoding="utf-8")


def write_run_all_sh(
    root: Path,
    runners: list[str],
    *,
    include_guard: bool = True,
    include_options: bool = True,
    include_help: bool = True,
    include_verify: bool = True,
    include_python_guard: bool = True,
    include_no_bytecode_guard: bool = True,
) -> None:
    body = "\n".join(f"    {runner}" for runner in runners)
    options = (
        "--list)\n"
        "    printf '%s\\n' \"${DEFAULT_RUNNERS[@]}\"\n"
        "    ;;\n"
        if include_options
        else ""
    )
    help_guard = (
        "--help|-h)\n"
        "    echo 'tests/runners/run_all.sh --list'\n"
        "    echo 'tests/runners/run_all.sh --verify'\n"
        "    echo 'tests/runners/run_all.sh --help'\n"
        "    ;;\n"
        if include_help
        else ""
    )
    python_guard = (
        "if command -v python3 >/dev/null 2>&1; then\n"
        "    PYTHON_BIN=python3\n"
        "elif command -v python >/dev/null 2>&1; then\n"
        "    PYTHON_BIN=python\n"
        "else\n"
        "    echo \"FAILED (python not found)\"\n"
        "fi\n"
        if include_python_guard
        else ""
    )
    no_bytecode_guard = (
        'PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/verify_runner_matrix.py"\n'
        'PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/test_verify_runner_matrix.py"\n'
        if include_no_bytecode_guard
        else ""
    )
    verify = (
        "--verify)\n"
        "    python verify_runner_matrix.py\n"
        "    python test_verify_runner_matrix.py\n"
        "    ;;\n"
        if include_verify
        else ""
    )
    guard = (
        "case \"$RUNNER\" in\n"
        "    ''|*[!abcdefghijklmnopqrstuvwxyz0123456789_-]*)\n"
        "        echo \"  FAILED (invalid runner name)\"\n"
        "        continue\n"
        "        ;;\n"
        "esac\n"
        if include_guard
        else ""
    )
    root.joinpath("run_all.sh").write_text(
        f"#!/usr/bin/env bash\nDEFAULT_RUNNERS=(\n{body}\n)\n{options}{help_guard}{verify}{python_guard}{no_bytecode_guard}{guard}",
        encoding="utf-8",
    )


def write_runner_dir(root: Path, name: str, *, bat: bool = True, sh: bool = True) -> None:
    runner_dir = root / name
    runner_dir.mkdir()
    if bat:
        runner_dir.joinpath("run_tests.bat").write_text("@echo off\n", encoding="utf-8")
    if sh:
        runner_dir.joinpath("run_tests.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")


def write_tracking_runner(root: Path, name: str, *, exit_code: int = 0) -> None:
    runner_dir = root / name
    runner_dir.mkdir()
    runner_dir.joinpath("run_tests.bat").write_text(
        f'@echo off\necho {name}>>"%~dp0..\\selected.txt"\nexit /b {exit_code}\n',
        encoding="utf-8",
    )
    runner_dir.joinpath("run_tests.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")


def write_escape_runner(root: Path, name: str) -> None:
    runner_dir = root / name
    runner_dir.mkdir()
    runner_dir.joinpath("run_tests.bat").write_text(
        '@echo off\necho escaped>"%~dp0..\\escaped.txt"\nexit /b 0\n',
        encoding="utf-8",
    )


class VerifyRunnerMatrixTests(unittest.TestCase):
    def run_matrix(self, root: Path) -> tuple[int, str]:
        old_runners_dir = verify_runner_matrix.RUNNERS_DIR
        verify_runner_matrix.RUNNERS_DIR = root
        try:
            output = StringIO()
            with redirect_stdout(output):
                exit_code = verify_runner_matrix.main()
            return exit_code, output.getvalue()
        finally:
            verify_runner_matrix.RUNNERS_DIR = old_runners_dir

    def test_valid_matrix_passes(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            runners = ["csharp", "rust", "sqlite"]
            write_run_all_bat(root, runners)
            write_run_all_sh(root, runners)
            for runner in runners:
                write_runner_dir(root, runner)

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 0)
        self.assertIn("PASSED: 3 runner entries are synchronized", output)

    def test_windows_list_option_matches_bat_runner_matrix(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch option test")

        result = subprocess.run(
            ["cmd", "/c", str(SCRIPT_DIR / "run_all.bat"), "--list"],
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip().split(), verify_runner_matrix.read_bat_runners())

    def test_windows_help_options_mention_supported_options(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch option test")

        for help_arg in ("--help", "-h", "/?"):
            with self.subTest(help_arg=help_arg):
                result = subprocess.run(
                    ["cmd", "/c", str(SCRIPT_DIR / "run_all.bat"), help_arg],
                    check=False,
                    capture_output=True,
                    text=True,
                )

                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("tests\\runners\\run_all.bat --list", result.stdout)
                self.assertIn("tests\\runners\\run_all.bat --verify", result.stdout)
                self.assertIn("tests\\runners\\run_all.bat --help", result.stdout)

    def test_windows_verify_uses_py_launcher_when_python_is_missing(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["rust"])
            root.joinpath("verify_runner_matrix.py").write_text(
                "from pathlib import Path\n"
                "Path(__file__).with_name('live_marker.txt').write_text('ok', encoding='utf-8')\n"
                "print('live verifier ok')\n",
                encoding="utf-8",
            )
            root.joinpath("test_verify_runner_matrix.py").write_text(
                "from pathlib import Path\n"
                "Path(__file__).with_name('regression_marker.txt').write_text('ok', encoding='utf-8')\n"
                "print('regression verifier ok')\n",
                encoding="utf-8",
            )
            env = os.environ.copy()
            system_root_value = env.get("SystemRoot") or env.get("WINDIR")
            self.assertIsNotNone(system_root_value)
            system_root = Path(system_root_value)
            if not system_root.joinpath("py.exe").is_file():
                self.skipTest("Windows py launcher not installed")
            env["PATH"] = os.pathsep.join(
                [
                    str(system_root / "System32"),
                    str(system_root),
                ]
            )

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "--verify"],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("live verifier ok", result.stdout)
        self.assertIn("regression verifier ok", result.stdout)

    def test_windows_verify_fails_when_python_and_py_are_missing(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["rust"])
            root.joinpath("verify_runner_matrix.py").write_text(
                "print('live verifier should not run')\n",
                encoding="utf-8",
            )
            root.joinpath("test_verify_runner_matrix.py").write_text(
                "print('regression verifier should not run')\n",
                encoding="utf-8",
            )
            env = os.environ.copy()
            system_root_value = env.get("SystemRoot") or env.get("WINDIR")
            self.assertIsNotNone(system_root_value)
            system_root = Path(system_root_value)
            env["PATH"] = str(system_root / "System32")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "--verify"],
                check=False,
                capture_output=True,
                text=True,
                env=env,
            )

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("FAILED (python not found)", result.stdout)
        self.assertNotIn("live verifier should not run", result.stdout)
        self.assertNotIn("regression verifier should not run", result.stdout)

    def test_windows_verify_stops_when_live_matrix_check_fails(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["rust"])
            root.joinpath("verify_runner_matrix.py").write_text(
                "import sys\n"
                "print('live verifier failed')\n"
                "sys.exit(7)\n",
                encoding="utf-8",
            )
            root.joinpath("test_verify_runner_matrix.py").write_text(
                "from pathlib import Path\n"
                "Path(__file__).with_name('regression_marker.txt').write_text('ran', encoding='utf-8')\n"
                "print('regression verifier should not run')\n",
                encoding="utf-8",
            )

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "--verify"],
                check=False,
                capture_output=True,
                text=True,
            )
            regression_marker_exists = root.joinpath("regression_marker.txt").exists()

        self.assertEqual(result.returncode, 7, result.stdout + result.stderr)
        self.assertIn("live verifier failed", result.stdout)
        self.assertNotIn("regression verifier should not run", result.stdout)
        self.assertFalse(regression_marker_exists)

    def test_windows_subset_runs_only_requested_runners(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["csharp", "rust"])
            write_tracking_runner(root, "csharp")
            write_tracking_runner(root, "rust")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "rust"],
                check=False,
                capture_output=True,
                text=True,
            )
            selected = root.joinpath("selected.txt").read_text(encoding="utf-8").splitlines()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(selected, ["rust"])
        self.assertIn("Passed:  1", result.stdout)
        self.assertIn("Failed:  0", result.stdout)

    def test_windows_default_runs_all_default_runners_in_order(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["csharp", "rust"])
            write_tracking_runner(root, "csharp")
            write_tracking_runner(root, "rust")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat")],
                check=False,
                capture_output=True,
                text=True,
            )
            selected = root.joinpath("selected.txt").read_text(encoding="utf-8").splitlines()

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(selected, ["csharp", "rust"])
        self.assertIn("Passed:  2", result.stdout)
        self.assertIn("Failed:  0", result.stdout)

    def test_windows_default_runner_list_uses_runtime_name_validation(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["csharp", "../outside"])
            write_tracking_runner(root, "csharp")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat")],
                check=False,
                capture_output=True,
                text=True,
            )
            selected = root.joinpath("selected.txt").read_text(encoding="utf-8").splitlines()

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(selected, ["csharp"])
        self.assertIn("FAILED (invalid runner name)", result.stdout)
        self.assertIn("Passed:  1", result.stdout)
        self.assertIn("Failed:  1", result.stdout)

    def test_windows_runner_failure_is_counted_and_does_not_stop_next_runner(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["csharp", "rust"])
            write_tracking_runner(root, "csharp", exit_code=7)
            write_tracking_runner(root, "rust")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat")],
                check=False,
                capture_output=True,
                text=True,
            )
            selected = root.joinpath("selected.txt").read_text(encoding="utf-8").splitlines()

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(selected, ["csharp", "rust"])
        self.assertIn("FAILED (runner csharp failed)", result.stdout)
        self.assertIn("Passed:  1", result.stdout)
        self.assertIn("Failed:  1", result.stdout)

    def test_windows_unknown_runner_fails(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["rust"])
            write_tracking_runner(root, "rust")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "missing"],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("FAILED (runner script not found)", result.stdout)
        self.assertIn("Failed:  1", result.stdout)

    def test_windows_invalid_runner_name_fails_before_path_resolution(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            temp_root = Path(temp)
            root = temp_root / "runners"
            root.mkdir()
            write_actual_run_all_bat(root, ["rust"])
            write_escape_runner(temp_root, "outside")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "..\\outside"],
                check=False,
                capture_output=True,
                text=True,
            )

            escaped = temp_root.joinpath("escaped.txt").exists()

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertFalse(escaped)
        self.assertIn("FAILED (invalid runner name)", result.stdout)
        self.assertIn("Failed:  1", result.stdout)

    def test_windows_metachar_runner_name_is_not_interpreted_by_cmd(self) -> None:
        if os.name != "nt":
            self.skipTest("Windows batch execution test")

        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_actual_run_all_bat(root, ["rust"])
            write_tracking_runner(root, "rust")

            result = subprocess.run(
                ["cmd", "/c", str(root / "run_all.bat"), "bad&echo injected"],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("FAILED (invalid runner name)", result.stdout)
        self.assertIn("Failed:  1", result.stdout)
        self.assertNotIn("injected", result.stdout + result.stderr)

    def test_duplicate_runner_name_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp", "rust", "rust"])
            write_run_all_sh(root, ["csharp", "rust", "rust"])
            write_runner_dir(root, "csharp")
            write_runner_dir(root, "rust")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("duplicate runner names", output)
        self.assertIn("rust", output)

    def test_empty_runner_list_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, [])
            write_run_all_sh(root, [])

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all.bat must contain at least one runner", output)
        self.assertIn("run_all.sh must contain at least one runner", output)
        self.assertIn("runner directories must contain at least one runner", output)

    def test_missing_bat_runner_block_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            root.joinpath("run_all.bat").write_text("@echo off\n", encoding="utf-8")
            write_run_all_sh(root, ["csharp"])
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("FAILED: run_all.bat DEFAULT_RUNNERS not found", output)

    def test_missing_sh_runner_block_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"])
            root.joinpath("run_all.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("FAILED: run_all.sh DEFAULT_RUNNERS block not found", output)

    def test_missing_runtime_guard_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_guard=False)
            write_run_all_sh(root, ["csharp"], include_guard=False)
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all scripts must validate runtime runner arguments", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("run_all.sh: missing", output)

    def test_missing_list_option_guard_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_options=False)
            write_run_all_sh(root, ["csharp"], include_options=False)
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all scripts must expose DEFAULT_RUNNERS through --list", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("run_all.sh: missing", output)

    def test_missing_help_guard_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_help=False)
            write_run_all_sh(root, ["csharp"], include_help=False)
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all scripts must expose --help usage", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("run_all.sh: missing", output)

    def test_missing_verify_steps_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_verify=False)
            write_run_all_sh(root, ["csharp"], include_verify=False)
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must run live matrix and regression checks", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("run_all.sh: missing", output)

    def test_verify_regression_must_use_selected_python_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], regression_uses_selected_python=False)
            write_run_all_sh(root, ["csharp"])
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must use selected Python", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn('%PYTHON_BIN% "%SCRIPT_DIR%test_verify_runner_matrix.py"', output)

    def test_missing_python_guard_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_python_guard=False)
            write_run_all_sh(root, ["csharp"], include_python_guard=False)
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must check Python availability", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("run_all.sh: missing", output)

    def test_windows_python_guard_order_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], python_guard_out_of_order=True)
            write_run_all_sh(root, ["csharp"])
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must check Python availability in order", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn('set "PYTHON_BIN=python"', output)
        self.assertIn(":python_not_found", output)

    def test_missing_windows_py_fallback_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_windows_py_fallback=False)
            write_run_all_sh(root, ["csharp"])
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must check Python availability", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("where py >nul 2>nul", output)
        self.assertIn('set "PYTHON_BIN=py -3"', output)

    def test_missing_no_bytecode_guard_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], include_no_bytecode_guard=False)
            write_run_all_sh(root, ["csharp"], include_no_bytecode_guard=False)
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must suppress Python bytecode caches", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn("run_all.sh: missing", output)

    def test_no_bytecode_guard_order_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"], no_bytecode_after_invocation=True)
            write_run_all_sh(root, ["csharp"])
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all --verify must suppress Python bytecode caches", output)
        self.assertIn("run_all.bat: missing", output)
        self.assertIn('%PYTHON_BIN% "%SCRIPT_DIR%verify_runner_matrix.py"', output)

    def test_invalid_runner_name_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp", "../rust"])
            write_run_all_sh(root, ["csharp", "../rust"])
            write_runner_dir(root, "csharp")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("invalid runner names", output)
        self.assertIn("../rust", output)

    def test_invalid_runner_directory_name_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp"])
            write_run_all_sh(root, ["csharp"])
            write_runner_dir(root, "csharp")
            write_runner_dir(root, "Rust")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("runner directories contains invalid runner names", output)
        self.assertIn("Rust", output)

    def test_one_sided_runner_script_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            runners = ["csharp", "rust"]
            write_run_all_bat(root, runners)
            write_run_all_sh(root, runners)
            write_runner_dir(root, "csharp")
            write_runner_dir(root, "rust", sh=False)

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("must include both Windows and POSIX scripts", output)
        self.assertIn("rust: missing run_tests.sh", output)

    def test_posix_runner_order_mismatch_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            write_run_all_bat(root, ["csharp", "rust", "sqlite"])
            write_run_all_sh(root, ["csharp", "sqlite", "rust"])
            write_runner_dir(root, "csharp")
            write_runner_dir(root, "rust")
            write_runner_dir(root, "sqlite")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("run_all.sh vs run_all.bat mismatch", output)

    def test_missing_runner_directory_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            runners = ["csharp", "rust", "sqlite"]
            write_run_all_bat(root, runners)
            write_run_all_sh(root, runners)
            write_runner_dir(root, "csharp")
            write_runner_dir(root, "rust")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("runner directories vs run_all.bat mismatch", output)
        self.assertIn("missing: sqlite", output)

    def test_extra_runner_directory_fails(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            runners = ["csharp", "rust"]
            write_run_all_bat(root, runners)
            write_run_all_sh(root, runners)
            write_runner_dir(root, "csharp")
            write_runner_dir(root, "rust")
            write_runner_dir(root, "sqlite")

            exit_code, output = self.run_matrix(root)

        self.assertEqual(exit_code, 1)
        self.assertIn("runner directories vs run_all.bat mismatch", output)
        self.assertIn("extra:   sqlite", output)


if __name__ == "__main__":
    unittest.main()
