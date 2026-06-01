from __future__ import annotations

import re
import sys
from pathlib import Path


RUNNERS_DIR = Path(__file__).resolve().parent
RUNNER_NAME_PATTERN = re.compile(r"^[a-z0-9_-]+$")


def read_bat_runners() -> list[str]:
    text = (RUNNERS_DIR / "run_all.bat").read_text(encoding="utf-8")
    match = re.search(r'set "DEFAULT_RUNNERS=([^"]*)"', text)
    if not match:
        raise ValueError("run_all.bat DEFAULT_RUNNERS not found")
    return match.group(1).split()


def read_sh_runners() -> list[str]:
    text = (RUNNERS_DIR / "run_all.sh").read_text(encoding="utf-8")
    match = re.search(r"DEFAULT_RUNNERS=\(\n(?P<body>.*?)\n\)", text, re.DOTALL)
    if not match:
        raise ValueError("run_all.sh DEFAULT_RUNNERS block not found")
    runners = []
    for line in match.group("body").splitlines():
        name = line.strip()
        if name and not name.startswith("#"):
            runners.append(name)
    return runners


def discover_runner_dirs() -> list[str]:
    runners = []
    for path in sorted(RUNNERS_DIR.iterdir()):
        if path.is_dir() and has_runner_script(path):
            runners.append(path.name)
    return runners


def has_runner_script(path: Path) -> bool:
    return (path / "run_tests.bat").is_file() or (path / "run_tests.sh").is_file()


def missing_snippets(text: str, snippets: list[str]) -> list[str]:
    return [snippet for snippet in snippets if snippet not in text]


def missing_ordered_snippets(text: str, snippets: list[str]) -> list[str]:
    missing = []
    start = 0
    for snippet in snippets:
        index = text.find(snippet, start)
        if index == -1:
            missing.append(snippet)
        else:
            start = index + len(snippet)
    return missing


def report_missing_runtime_guards() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            ":validate_runner_arg",
            "POLYGEN_RUNNER_ARG",
            "^[a-z0-9_-]+$",
            "FAILED ^(invalid runner name^)",
        ],
        "run_all.sh": [
            "*[!abcdefghijklmnopqrstuvwxyz0123456789_-]*",
            "FAILED (invalid runner name)",
            "continue",
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = missing_snippets(text, snippets)
        if missing:
            if not failed:
                print("FAILED: run_all scripts must validate runtime runner arguments")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_missing_option_guards() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            'if /i "%~1"=="--list" goto :list',
            ":list",
            "echo %DEFAULT_RUNNERS%",
        ],
        "run_all.sh": [
            "--list)",
            "printf '%s\\n' \"${DEFAULT_RUNNERS[@]}\"",
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = missing_snippets(text, snippets)
        if missing:
            if not failed:
                print("FAILED: run_all scripts must expose DEFAULT_RUNNERS through --list")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_missing_help_guards() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            'if /i "%~1"=="--help" goto :usage',
            'if /i "%~1"=="-h" goto :usage',
            'if /i "%~1"=="/?" goto :usage',
            ":usage",
            "tests\\runners\\run_all.bat --list",
            "tests\\runners\\run_all.bat --verify",
            "tests\\runners\\run_all.bat --help",
        ],
        "run_all.sh": [
            "--help|-h)",
            "tests/runners/run_all.sh --list",
            "tests/runners/run_all.sh --verify",
            "tests/runners/run_all.sh --help",
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = missing_snippets(text, snippets)
        if missing:
            if not failed:
                print("FAILED: run_all scripts must expose --help usage")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_missing_verify_steps() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            'if /i "%~1"=="--verify" goto :verify',
            ":verify",
            "verify_runner_matrix.py",
            "test_verify_runner_matrix.py",
        ],
        "run_all.sh": [
            "--verify)",
            "verify_runner_matrix.py",
            "test_verify_runner_matrix.py",
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = missing_snippets(text, snippets)
        if missing:
            if not failed:
                print("FAILED: run_all --verify must run live matrix and regression checks")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_missing_verify_python_invocations() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            '%PYTHON_BIN% "%SCRIPT_DIR%verify_runner_matrix.py"',
            'set "VERIFY_EXIT=%ERRORLEVEL%"',
            "exit /b %VERIFY_EXIT%",
            '%PYTHON_BIN% "%SCRIPT_DIR%test_verify_runner_matrix.py"',
        ],
        "run_all.sh": [
            'PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/verify_runner_matrix.py"',
            'PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/test_verify_runner_matrix.py"',
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = missing_ordered_snippets(text, snippets)
        if missing:
            if not failed:
                print("FAILED: run_all --verify must use selected Python for live and regression checks")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_missing_python_guards() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            "where python >nul 2>nul",
            "where py >nul 2>nul",
            "goto :python_not_found",
            'set "PYTHON_BIN=py -3"',
            'set "PYTHON_BIN=python"',
            ":python_not_found",
            "FAILED ^(python not found^)",
        ],
        "run_all.sh": [
            "command -v python3",
            "command -v python",
            "FAILED (python not found)",
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = (
            missing_ordered_snippets(text, snippets)
            if filename == "run_all.bat"
            else missing_snippets(text, snippets)
        )
        if missing:
            if not failed:
                print("FAILED: run_all --verify must check Python availability in order")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_missing_no_bytecode_guards() -> bool:
    failed = False
    required_snippets = {
        "run_all.bat": [
            'set "PYTHONDONTWRITEBYTECODE=1"',
            '%PYTHON_BIN% "%SCRIPT_DIR%verify_runner_matrix.py"',
            '%PYTHON_BIN% "%SCRIPT_DIR%test_verify_runner_matrix.py"',
        ],
        "run_all.sh": [
            'PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/verify_runner_matrix.py"',
            'PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/test_verify_runner_matrix.py"',
        ],
    }

    for filename, snippets in required_snippets.items():
        text = (RUNNERS_DIR / filename).read_text(encoding="utf-8")
        missing = missing_ordered_snippets(text, snippets)
        if missing:
            if not failed:
                print("FAILED: run_all --verify must suppress Python bytecode caches before running verifiers")
            print(f"  {filename}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_duplicates(label: str, runners: list[str]) -> bool:
    duplicates = sorted({runner for runner in runners if runners.count(runner) > 1})
    if not duplicates:
        return False

    print(f"FAILED: {label} contains duplicate runner names")
    print(f"  duplicates: {' '.join(duplicates)}")
    return True


def report_invalid_names(label: str, runners: list[str]) -> bool:
    invalid = sorted({runner for runner in runners if not RUNNER_NAME_PATTERN.fullmatch(runner)})
    if not invalid:
        return False

    print(f"FAILED: {label} contains invalid runner names")
    print(f"  invalid: {' '.join(invalid)}")
    return True


def report_empty(label: str, runners: list[str]) -> bool:
    if runners:
        return False

    print(f"FAILED: {label} must contain at least one runner")
    return True


def report_incomplete_runner_dirs() -> bool:
    failed = False
    for path in sorted(RUNNERS_DIR.iterdir()):
        if not path.is_dir() or not has_runner_script(path):
            continue

        missing = []
        if not (path / "run_tests.bat").is_file():
            missing.append("run_tests.bat")
        if not (path / "run_tests.sh").is_file():
            missing.append("run_tests.sh")
        if missing:
            if not failed:
                print("FAILED: runner directories must include both Windows and POSIX scripts")
            print(f"  {path.name}: missing {', '.join(missing)}")
            failed = True

    return failed


def report_mismatch(label: str, expected: list[str], actual: list[str]) -> bool:
    if expected == actual:
        return False

    print(f"FAILED: {label} mismatch")
    print(f"  expected: {' '.join(expected)}")
    print(f"  actual:   {' '.join(actual)}")
    return True


def report_set_mismatch(label: str, expected: list[str], actual: list[str]) -> bool:
    expected_set = set(expected)
    actual_set = set(actual)
    if expected_set == actual_set:
        return False

    print(f"FAILED: {label} mismatch")
    missing = sorted(expected_set - actual_set)
    extra = sorted(actual_set - expected_set)
    if missing:
        print(f"  missing: {' '.join(missing)}")
    if extra:
        print(f"  extra:   {' '.join(extra)}")
    return True


def main() -> int:
    try:
        bat_runners = read_bat_runners()
        sh_runners = read_sh_runners()
        dir_runners = discover_runner_dirs()
    except (OSError, ValueError) as error:
        print(f"FAILED: {error}")
        return 1

    failed = False
    failed |= report_empty("run_all.bat", bat_runners)
    failed |= report_empty("run_all.sh", sh_runners)
    failed |= report_empty("runner directories", dir_runners)
    failed |= report_invalid_names("run_all.bat", bat_runners)
    failed |= report_invalid_names("run_all.sh", sh_runners)
    failed |= report_invalid_names("runner directories", dir_runners)
    failed |= report_duplicates("run_all.bat", bat_runners)
    failed |= report_duplicates("run_all.sh", sh_runners)
    failed |= report_missing_runtime_guards()
    failed |= report_missing_option_guards()
    failed |= report_missing_help_guards()
    failed |= report_missing_verify_steps()
    failed |= report_missing_verify_python_invocations()
    failed |= report_missing_python_guards()
    failed |= report_missing_no_bytecode_guards()
    failed |= report_incomplete_runner_dirs()
    failed |= report_mismatch("run_all.sh vs run_all.bat", bat_runners, sh_runners)
    failed |= report_set_mismatch("runner directories vs run_all.bat", bat_runners, dir_runners)

    if failed:
        return 1

    print(f"PASSED: {len(bat_runners)} runner entries are synchronized")
    return 0


if __name__ == "__main__":
    sys.exit(main())
