from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class Check:
    label: str
    ok: bool
    detail: str


@dataclass
class TargetStatus:
    name: str
    ready: bool
    checks: list[Check]
    command: str
    env: dict[str, str] = field(default_factory=dict)


def which_from_env(env_name: str, *names: str) -> str | None:
    explicit = os.environ.get(env_name)
    if explicit:
        return explicit
    for name in names:
        found = shutil.which(name)
        if found:
            return found
    return None


def env_value(*names: str) -> tuple[str | None, str | None]:
    for name in names:
        value = os.environ.get(name)
        if value:
            return name, value
    return None, None


def classpath_status() -> Check:
    name, value = env_value("POLYGEN_KOTLIN_CLASSPATH", "KOTLIN_CLASSPATH")
    if not value:
        return Check(
            "Kotlin classpath",
            False,
            "set POLYGEN_KOTLIN_CLASSPATH or KOTLIN_CLASSPATH for kotlinx.serialization, kotlinx.datetime, and SQLite JDBC",
        )
    missing: list[str] = []
    for entry in value.split(os.pathsep):
        if not entry or "*" in entry:
            continue
        if not Path(entry).exists():
            missing.append(entry)
    if missing:
        return Check(name or "Kotlin classpath", False, "missing entries: " + ", ".join(missing))
    return Check(name or "Kotlin classpath", True, value)


def unreal_ubt_from_engine_root() -> str | None:
    _, engine_root = env_value("POLYGEN_UNREAL_ENGINE_ROOT", "UNREAL_ENGINE_ROOT")
    if not engine_root:
        return None
    dotnet_dir = Path(engine_root) / "Engine" / "Binaries" / "DotNET"
    for candidate in (
        dotnet_dir / "UnrealBuildTool" / "UnrealBuildTool.exe",
        dotnet_dir / "UnrealBuildTool.exe",
        dotnet_dir / "UnrealBuildTool" / "UnrealBuildTool",
    ):
        if candidate.is_file():
            return str(candidate)
    return None


def epic_manifest_engine_roots() -> list[Path]:
    program_data = os.environ.get("ProgramData") or r"C:\ProgramData"
    manifest_dir = Path(program_data) / "Epic" / "EpicGamesLauncher" / "Data" / "Manifests"
    if not manifest_dir.is_dir():
        return []

    roots: list[Path] = []
    seen: set[str] = set()
    for manifest in sorted(manifest_dir.glob("*.item")):
        try:
            data = json.loads(manifest.read_text(encoding="utf-8-sig"))
        except (OSError, json.JSONDecodeError):
            continue
        app_name = str(data.get("AppName", ""))
        display_name = str(data.get("DisplayName", ""))
        install_location = data.get("InstallLocation")
        if not install_location:
            continue
        if app_name.startswith("UE_") or display_name == "Unreal Engine":
            root = Path(str(install_location))
            key = str(root).lower()
            if key not in seen:
                roots.append(root)
                seen.add(key)
    return roots


def discover_ubt() -> str | None:
    explicit = os.environ.get("POLYGEN_UNREAL_UBT") or os.environ.get("UNREAL_BUILD_TOOL")
    if explicit:
        return explicit
    found = shutil.which("UnrealBuildTool") or shutil.which("UnrealBuildTool.exe")
    if found:
        return found
    engine_root_ubt = unreal_ubt_from_engine_root()
    if engine_root_ubt:
        return engine_root_ubt
    for engine_root in epic_manifest_engine_roots():
        dotnet_dir = engine_root / "Engine" / "Binaries" / "DotNET"
        for candidate in (
            dotnet_dir / "UnrealBuildTool" / "UnrealBuildTool.exe",
            dotnet_dir / "UnrealBuildTool.exe",
            dotnet_dir / "UnrealBuildTool" / "UnrealBuildTool",
        ):
            if candidate.is_file():
                return str(candidate)
    return None


def prepared_unreal_fixture_env() -> dict[str, str]:
    root_value = os.environ.get("POLYGEN_UNREAL_FIXTURE_ROOT")
    if not root_value:
        return {}
    root = Path(root_value)
    project_name = os.environ.get("POLYGEN_UNREAL_FIXTURE_PROJECT", "PolygenSmoke")
    module_name = os.environ.get("POLYGEN_UNREAL_FIXTURE_MODULE", "PolygenSmoke")
    project = root / f"{project_name}.uproject"
    header_dir = root / "Source" / module_name / "Public" / "Polygen"
    return {
        "POLYGEN_UNREAL_PROJECT": str(project),
        "POLYGEN_UNREAL_TARGET": f"{project_name}Editor",
        "POLYGEN_UNREAL_HEADER_DIR": str(header_dir),
    }


def swift_default_roots() -> list[Path]:
    roots: list[Path] = []
    for name in ("POLYGEN_SWIFT_ROOT", "SWIFT_ROOT"):
        value = os.environ.get(name)
        if value:
            roots.append(Path(value))
    local_app_data = os.environ.get("LOCALAPPDATA")
    if local_app_data:
        roots.append(Path(local_app_data) / "Programs" / "Swift")
    try:
        roots.append(Path.home() / "AppData" / "Local" / "Programs" / "Swift")
    except RuntimeError:
        pass
    return roots


def swift_sdk_for_root(root: Path, toolchain_dir: Path | None = None) -> Path | None:
    version = None
    if toolchain_dir is not None:
        version = toolchain_dir.name.split("+", 1)[0]
    candidates: list[Path] = []
    if version:
        candidates.append(root / "Platforms" / version / "Windows.platform" / "Developer" / "SDKs" / "Windows.sdk")
    candidates.extend(sorted((root / "Platforms").glob("*/Windows.platform/Developer/SDKs/Windows.sdk"), reverse=True))
    for candidate in candidates:
        if candidate.is_dir():
            return candidate
    return None


def discover_swift() -> tuple[str | None, str | None, list[str]]:
    explicit = os.environ.get("SWIFTC") or shutil.which("swiftc")
    if explicit:
        swiftc = Path(explicit)
        sdkroot = os.environ.get("SDKROOT")
        root: Path | None = None
        toolchain_dir: Path | None = None
        for parent in swiftc.parents:
            if parent.parent.name == "Toolchains":
                toolchain_dir = parent
                root = parent.parent.parent
                break
        if root is not None and not sdkroot:
            inferred_sdk = swift_sdk_for_root(root, toolchain_dir)
            if inferred_sdk is not None:
                sdkroot = str(inferred_sdk)
        path_entries = [str(swiftc.parent)]
        if root is not None and sdkroot:
            version = Path(sdkroot).parents[3].name
            for candidate in (
                root / "Runtimes" / version / "usr" / "bin",
                root / "Python-3.10.1" / "usr" / "bin",
            ):
                if candidate.is_dir():
                    path_entries.append(str(candidate))
        return explicit, sdkroot, path_entries

    for root in swift_default_roots():
        toolchains = sorted((root / "Toolchains").glob("*/usr/bin/swiftc.exe"), reverse=True)
        for swiftc in toolchains:
            toolchain_dir = swiftc.parents[2]
            sdkroot_path = swift_sdk_for_root(root, toolchain_dir)
            if sdkroot_path is None:
                continue
            version = sdkroot_path.parents[3].name
            path_entries = [str(swiftc.parent)]
            for candidate in (
                root / "Runtimes" / version / "usr" / "bin",
                root / "Python-3.10.1" / "usr" / "bin",
            ):
                if candidate.is_dir():
                    path_entries.append(str(candidate))
            return str(swiftc), str(sdkroot_path), path_entries
    return None, None, []


def check_kotlin() -> TargetStatus:
    kotlinc = which_from_env("KOTLINC", "kotlinc")
    java = which_from_env("JAVA", "java")
    classpath = classpath_status()
    checks = [
        Check("kotlinc", bool(kotlinc), kotlinc or "set KOTLINC or add kotlinc to PATH"),
        Check("java", bool(java), java or "set JAVA or add java to PATH"),
        classpath,
    ]
    ready = all(check.ok for check in checks)
    return TargetStatus(
        "kotlin",
        ready,
        checks,
        "set POLYGEN_KOTLIN_RUNTIME=1 and run tests\\runners\\kotlin\\run_tests.bat",
    )


def check_swift() -> TargetStatus:
    swiftc, sdkroot, path_entries = discover_swift()
    env = {}
    if swiftc:
        env["SWIFTC"] = swiftc
    if sdkroot:
        env["SDKROOT"] = sdkroot
    if path_entries:
        env["PATH"] = os.pathsep.join([*path_entries, os.environ.get("PATH", "")])
    checks = [
        Check("swiftc", bool(swiftc), swiftc or "set SWIFTC or add swiftc to PATH"),
        Check("SDKROOT", bool(sdkroot and Path(sdkroot).is_dir()), sdkroot or "set SDKROOT to Swift Windows.sdk"),
    ]
    ready = all(check.ok for check in checks)
    return TargetStatus(
        "swift",
        ready,
        checks,
        "set POLYGEN_SWIFT_RUNTIME=1 and run tests\\runners\\swift\\run_tests.bat",
        env,
    )


def check_unreal() -> TargetStatus:
    ubt = discover_ubt()
    fixture_env = prepared_unreal_fixture_env()
    project = os.environ.get("POLYGEN_UNREAL_PROJECT") or fixture_env.get("POLYGEN_UNREAL_PROJECT")
    target = os.environ.get("POLYGEN_UNREAL_TARGET") or fixture_env.get("POLYGEN_UNREAL_TARGET")
    header_dir = os.environ.get("POLYGEN_UNREAL_HEADER_DIR") or fixture_env.get("POLYGEN_UNREAL_HEADER_DIR")
    env = {}
    if project:
        env["POLYGEN_UNREAL_PROJECT"] = project
    if target:
        env["POLYGEN_UNREAL_TARGET"] = target
    if header_dir:
        env["POLYGEN_UNREAL_HEADER_DIR"] = header_dir
    if ubt:
        env["POLYGEN_UNREAL_UBT"] = ubt
    checks = [
        Check(
            "UnrealBuildTool",
            bool(ubt),
            ubt or "set POLYGEN_UNREAL_UBT, UNREAL_BUILD_TOOL, POLYGEN_UNREAL_ENGINE_ROOT, or add UnrealBuildTool to PATH",
        ),
        Check(
            "POLYGEN_UNREAL_PROJECT",
            bool(project and Path(project).is_file()),
            project or "set POLYGEN_UNREAL_PROJECT to a .uproject file or prepare POLYGEN_UNREAL_FIXTURE_ROOT",
        ),
        Check("POLYGEN_UNREAL_TARGET", bool(target), target or "set POLYGEN_UNREAL_TARGET"),
        Check(
            "POLYGEN_UNREAL_HEADER_DIR",
            bool(header_dir and Path(header_dir).is_dir()),
            header_dir or "set POLYGEN_UNREAL_HEADER_DIR to the target module header directory",
        ),
    ]
    ready = all(check.ok for check in checks)
    return TargetStatus(
        "unreal",
        ready,
        checks,
        "set POLYGEN_UNREAL_COMPILE=1 and run tests\\runners\\unreal\\run_tests.bat",
        env,
    )


def render(statuses: list[TargetStatus]) -> None:
    print("=== Optional Toolchain Readiness ===")
    for status in statuses:
        state = "READY" if status.ready else "MISSING"
        print(f"{status.name}: {state}")
        for check in status.checks:
            marker = "ok" if check.ok else "missing"
            print(f"  [{marker}] {check.label}: {check.detail}")
        print(f"  command: {status.command}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Check optional PolyGen runner toolchain readiness.")
    parser.add_argument(
        "--fail-on-missing",
        action="store_true",
        help="Exit 1 if any optional runtime/compile toolchain is not ready.",
    )
    args = parser.parse_args()

    statuses = [check_kotlin(), check_swift(), check_unreal()]
    render(statuses)
    if args.fail_on_missing and not all(status.ready for status in statuses):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
