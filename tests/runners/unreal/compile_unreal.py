"""Optional UnrealBuildTool smoke gate for generated PolyGen headers.

This helper is intentionally opt-in. It copies generated headers into an
explicit Unreal module header directory and invokes UnrealBuildTool for the
configured project target. It never tries to discover or mutate an Unreal
project implicitly.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path


def expand_headers(patterns: list[str]) -> list[Path]:
    headers: list[Path] = []
    for pattern in patterns:
        matches = glob.glob(pattern)
        if matches:
            headers.extend(Path(match) for match in matches)
        else:
            path = Path(pattern)
            if path.is_file():
                headers.append(path)
    return sorted({path.resolve() for path in headers})


def prepared_fixture_env() -> dict[str, str]:
    root_value = os.environ.get("POLYGEN_UNREAL_FIXTURE_ROOT")
    if not root_value:
        return {}
    root = Path(root_value)
    project_name = os.environ.get("POLYGEN_UNREAL_FIXTURE_PROJECT", "PolygenSmoke")
    module_name = os.environ.get("POLYGEN_UNREAL_FIXTURE_MODULE", "PolygenSmoke")
    return {
        "POLYGEN_UNREAL_PROJECT": str(root / f"{project_name}.uproject"),
        "POLYGEN_UNREAL_TARGET": f"{project_name}Editor",
        "POLYGEN_UNREAL_HEADER_DIR": str(root / "Source" / module_name / "Public" / "Polygen"),
    }


def require_unreal_setting(name: str, fixture_env: dict[str, str]) -> str:
    value = os.environ.get(name) or fixture_env.get(name)
    if not value:
        print(
            f"{name} is required when POLYGEN_UNREAL_COMPILE=1. "
            "Set it directly or set POLYGEN_UNREAL_FIXTURE_ROOT.",
            file=sys.stderr,
        )
        raise SystemExit(1)
    return value


def ubt_candidates_for_engine_root(engine_root: Path) -> list[Path]:
    engine_dir = engine_root if engine_root.name.lower() == "engine" else engine_root / "Engine"
    dotnet_dir = engine_dir / "Binaries" / "DotNET"
    return [
        dotnet_dir / "UnrealBuildTool" / "UnrealBuildTool.exe",
        dotnet_dir / "UnrealBuildTool.exe",
        dotnet_dir / "UnrealBuildTool" / "UnrealBuildTool",
    ]


def discover_engine_roots() -> list[Path]:
    roots: list[Path] = []
    seen: set[str] = set()

    def add_root(root: Path) -> None:
        key = str(root).lower()
        if key not in seen:
            roots.append(root)
            seen.add(key)

    for env_name in ("POLYGEN_UNREAL_ENGINE_ROOT", "UNREAL_ENGINE_ROOT"):
        value = os.environ.get(env_name)
        if value:
            add_root(Path(value))

    for env_name in ("ProgramFiles", "ProgramFiles(x86)"):
        value = os.environ.get(env_name)
        if not value:
            continue
        epic_root = Path(value) / "Epic Games"
        if epic_root.is_dir():
            for path in epic_root.iterdir():
                if path.is_dir() and path.name.startswith("UE_"):
                    add_root(path)

    program_data = os.environ.get("ProgramData") or r"C:\ProgramData"
    manifest_dir = Path(program_data) / "Epic" / "EpicGamesLauncher" / "Data" / "Manifests"
    if manifest_dir.is_dir():
        for manifest in sorted(manifest_dir.glob("*.item")):
            try:
                data = json.loads(manifest.read_text(encoding="utf-8-sig"))
            except (OSError, json.JSONDecodeError):
                continue
            app_name = str(data.get("AppName", ""))
            display_name = str(data.get("DisplayName", ""))
            install_location = data.get("InstallLocation")
            if install_location and (app_name.startswith("UE_") or display_name == "Unreal Engine"):
                add_root(Path(str(install_location)))

    return roots


def find_ubt() -> str:
    explicit = os.environ.get("POLYGEN_UNREAL_UBT") or os.environ.get("UNREAL_BUILD_TOOL")
    if explicit:
        return explicit
    found = shutil.which("UnrealBuildTool") or shutil.which("UnrealBuildTool.exe")
    if found:
        return found
    for engine_root in discover_engine_roots():
        for candidate in ubt_candidates_for_engine_root(engine_root):
            if candidate.is_file():
                return str(candidate)
    print(
        "UnrealBuildTool was not found. Set POLYGEN_UNREAL_UBT, UNREAL_BUILD_TOOL, or POLYGEN_UNREAL_ENGINE_ROOT.",
        file=sys.stderr,
    )
    raise SystemExit(1)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("headers", nargs="+", help="Generated Unreal .h files or glob patterns")
    args = parser.parse_args()

    headers = expand_headers(args.headers)
    if not headers:
        print("no Unreal headers found to compile", file=sys.stderr)
        return 1

    fixture_env = prepared_fixture_env()
    project = Path(require_unreal_setting("POLYGEN_UNREAL_PROJECT", fixture_env)).resolve()
    target = require_unreal_setting("POLYGEN_UNREAL_TARGET", fixture_env)
    header_dir = Path(require_unreal_setting("POLYGEN_UNREAL_HEADER_DIR", fixture_env)).resolve()
    platform = os.environ.get("POLYGEN_UNREAL_PLATFORM", "Win64")
    config = os.environ.get("POLYGEN_UNREAL_CONFIG", "Development")
    ubt = find_ubt()

    if not project.is_file():
        print(f"POLYGEN_UNREAL_PROJECT does not exist: {project}", file=sys.stderr)
        return 1

    header_dir.mkdir(parents=True, exist_ok=True)
    for header in headers:
        shutil.copy2(header, header_dir / header.name)

    extra_args = shlex.split(os.environ.get("POLYGEN_UNREAL_UBT_ARGS", ""))
    cmd = [
        ubt,
        target,
        platform,
        config,
        f"-Project={project}",
        "-NoHotReloadFromIDE",
        "-NoEngineChanges",
        "-NoMutex",
        *extra_args,
    ]

    print("Copied generated headers to:", header_dir)
    print(" ".join(str(part) for part in cmd))
    result = subprocess.run(cmd)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
