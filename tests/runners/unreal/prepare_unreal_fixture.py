"""Create a minimal Unreal project fixture for PolyGen UBT smoke gates."""

from __future__ import annotations

import argparse
import os
from pathlib import Path


def write_fixture_file(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def prepare_fixture(root: Path, project_name: str = "PolygenSmoke", module_name: str = "PolygenSmoke") -> dict[str, str]:
    root = root.resolve()
    source_dir = root / "Source"
    module_dir = source_dir / module_name
    public_polygen_dir = module_dir / "Public" / "Polygen"
    private_dir = module_dir / "Private"

    write_fixture_file(
        root / f"{project_name}.uproject",
        f"""{{
  "FileVersion": 3,
  "EngineAssociation": "",
  "Category": "PolyGen",
  "Description": "Minimal PolyGen Unreal smoke-test fixture",
  "Modules": [
    {{
      "Name": "{module_name}",
      "Type": "Runtime",
      "LoadingPhase": "Default"
    }}
  ]
}}
""",
    )
    write_fixture_file(
        source_dir / f"{project_name}.Target.cs",
        f"""using UnrealBuildTool;
using System.Collections.Generic;

public class {project_name}Target : TargetRules
{{
    public {project_name}Target(TargetInfo Target) : base(Target)
    {{
        Type = TargetType.Game;
        DefaultBuildSettings = BuildSettingsVersion.Latest;
        IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
        ExtraModuleNames.Add("{module_name}");
    }}
}}
""",
    )
    write_fixture_file(
        source_dir / f"{project_name}Editor.Target.cs",
        f"""using UnrealBuildTool;
using System.Collections.Generic;

public class {project_name}EditorTarget : TargetRules
{{
    public {project_name}EditorTarget(TargetInfo Target) : base(Target)
    {{
        Type = TargetType.Editor;
        DefaultBuildSettings = BuildSettingsVersion.Latest;
        IncludeOrderVersion = EngineIncludeOrderVersion.Latest;
        ExtraModuleNames.Add("{module_name}");
    }}
}}
""",
    )
    write_fixture_file(
        module_dir / f"{module_name}.Build.cs",
        f"""using UnrealBuildTool;

public class {module_name} : ModuleRules
{{
    public {module_name}(ReadOnlyTargetRules Target) : base(Target)
    {{
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicDependencyModuleNames.AddRange(new[] {{ "Core", "CoreUObject", "Engine", "Json", "JsonUtilities" }});
    }}
}}
""",
    )
    write_fixture_file(
        public_polygen_dir / ".gitkeep",
        "",
    )
    write_fixture_file(
        private_dir / f"{module_name}.cpp",
        f"""#include "Modules/ModuleManager.h"

IMPLEMENT_PRIMARY_GAME_MODULE(FDefaultGameModuleImpl, {module_name}, "{module_name}");
""",
    )

    return {
        "POLYGEN_UNREAL_PROJECT": str(root / f"{project_name}.uproject"),
        "POLYGEN_UNREAL_TARGET": f"{project_name}Editor",
        "POLYGEN_UNREAL_HEADER_DIR": str(public_polygen_dir),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        default=os.environ.get("POLYGEN_UNREAL_FIXTURE_ROOT"),
        help="Fixture project root. Defaults to POLYGEN_UNREAL_FIXTURE_ROOT.",
    )
    parser.add_argument("--project-name", default=os.environ.get("POLYGEN_UNREAL_FIXTURE_PROJECT", "PolygenSmoke"))
    parser.add_argument("--module-name", default=os.environ.get("POLYGEN_UNREAL_FIXTURE_MODULE", "PolygenSmoke"))
    args = parser.parse_args()

    if not args.root:
        parser.error("--root or POLYGEN_UNREAL_FIXTURE_ROOT is required")

    env = prepare_fixture(Path(args.root), args.project_name, args.module_name)
    for key, value in env.items():
        print(f"{key}={value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
