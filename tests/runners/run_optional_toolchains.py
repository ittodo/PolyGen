from __future__ import annotations

import argparse
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import check_optional_toolchains


RUNNERS_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = RUNNERS_DIR.parent.parent


@dataclass(frozen=True)
class OptionalTarget:
    name: str
    env_name: str
    status_fn: str


TARGETS = {
    "kotlin": OptionalTarget("kotlin", "POLYGEN_KOTLIN_RUNTIME", "check_kotlin"),
    "swift": OptionalTarget("swift", "POLYGEN_SWIFT_RUNTIME", "check_swift"),
    "unreal": OptionalTarget("unreal", "POLYGEN_UNREAL_COMPILE", "check_unreal"),
}


def runner_command(target: str) -> list[str]:
    if os.name == "nt":
        return ["cmd", "/d", "/c", "call", str(RUNNERS_DIR / target / "run_tests.bat")]
    return ["bash", str(RUNNERS_DIR / target / "run_tests.sh")]


def target_status(target: OptionalTarget) -> check_optional_toolchains.TargetStatus:
    return getattr(check_optional_toolchains, target.status_fn)()


def selected_targets(names: list[str] | None) -> list[OptionalTarget]:
    if not names:
        return list(TARGETS.values())
    return [TARGETS[name] for name in names]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run ready optional PolyGen toolchain gates for Kotlin, Swift, and Unreal.",
    )
    parser.add_argument(
        "targets",
        nargs="*",
        choices=sorted(TARGETS),
        help="Optional targets to consider. Defaults to all.",
    )
    parser.add_argument(
        "--fail-on-missing",
        action="store_true",
        help="Exit 1 if any selected target is not ready.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print commands for ready targets without running them.",
    )
    args = parser.parse_args()

    failed = False
    ran = 0
    ready = 0
    for target in selected_targets(args.targets):
        status = target_status(target)
        if not status.ready:
            print(f"SKIP {target.name}: optional toolchain is not ready")
            for check in status.checks:
                if not check.ok:
                    print(f"  missing {check.label}: {check.detail}")
            if args.fail_on_missing:
                failed = True
            continue

        cmd = runner_command(target.name)
        env = os.environ.copy()
        env.update(status.env)
        env[target.env_name] = "1"
        ready += 1
        print(f"RUN {target.name}: {' '.join(cmd)} with {target.env_name}=1")
        if args.dry_run:
            continue

        result = subprocess.run(cmd, cwd=PROJECT_ROOT, env=env)
        ran += 1
        if result.returncode != 0:
            print(f"FAILED {target.name}: exit {result.returncode}")
            failed = True
        else:
            print(f"PASSED {target.name}")

    if ready == 0 and not failed:
        print("No optional toolchain gates were ready to run.")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
