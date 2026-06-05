from __future__ import annotations

import os
import sys
import unittest
import json
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import compile_unreal
import prepare_unreal_fixture
import validate_unreal


class Completed:
    returncode = 0


class CompileUnrealTests(unittest.TestCase):
    def run_helper(self, args: list[str], env: dict[str, str]) -> int:
        with (
            patch.dict(os.environ, env, clear=False),
            patch.object(sys, "argv", ["compile_unreal.py", *args]),
            redirect_stdout(StringIO()),
            redirect_stderr(StringIO()),
        ):
            return compile_unreal.main()

    def test_copies_headers_and_invokes_configured_ubt(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            generated = root / "generated"
            generated.mkdir()
            first_header = generated / "PolygenSchema.h"
            second_header = generated / "PolygenRegistry.h"
            first_header.write_text("#pragma once\n", encoding="utf-8")
            second_header.write_text("#pragma once\n", encoding="utf-8")
            project = root / "Game.uproject"
            project.write_text("{}", encoding="utf-8")
            header_dir = root / "Game" / "Source" / "Game" / "Public" / "Polygen"

            captured: dict[str, list[str]] = {}

            def fake_run(cmd: list[str]) -> Completed:
                captured["cmd"] = cmd
                return Completed()

            env = {
                "POLYGEN_UNREAL_PROJECT": str(project),
                "POLYGEN_UNREAL_TARGET": "GameEditor",
                "POLYGEN_UNREAL_HEADER_DIR": str(header_dir),
                "POLYGEN_UNREAL_PLATFORM": "Linux",
                "POLYGEN_UNREAL_CONFIG": "DebugGame",
                "POLYGEN_UNREAL_UBT": "FakeUnrealBuildTool",
                "POLYGEN_UNREAL_UBT_ARGS": "-Mode=Test -Verbose",
            }
            with patch.object(compile_unreal.subprocess, "run", fake_run):
                exit_code = self.run_helper([str(generated / "*.h")], env)

            self.assertEqual(exit_code, 0)
            self.assertEqual((header_dir / first_header.name).read_text(encoding="utf-8"), "#pragma once\n")
            self.assertEqual((header_dir / second_header.name).read_text(encoding="utf-8"), "#pragma once\n")
            self.assertEqual(captured["cmd"][0], "FakeUnrealBuildTool")
            self.assertEqual(captured["cmd"][1:4], ["GameEditor", "Linux", "DebugGame"])
            self.assertIn(f"-Project={project.resolve()}", captured["cmd"])
            self.assertIn("-NoHotReloadFromIDE", captured["cmd"])
            self.assertIn("-NoEngineChanges", captured["cmd"])
            self.assertIn("-NoMutex", captured["cmd"])
            self.assertEqual(captured["cmd"][-2:], ["-Mode=Test", "-Verbose"])

    def test_discovers_ubt_from_configured_engine_root(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            header = root / "PolygenSchema.h"
            header.write_text("#pragma once\n", encoding="utf-8")
            project = root / "Game.uproject"
            project.write_text("{}", encoding="utf-8")
            header_dir = root / "headers"
            engine_root = root / "UE_5.4"
            ubt = engine_root / "Engine" / "Binaries" / "DotNET" / "UnrealBuildTool" / "UnrealBuildTool.exe"
            ubt.parent.mkdir(parents=True)
            ubt.write_text("", encoding="utf-8")
            captured: dict[str, list[str]] = {}

            def fake_run(cmd: list[str]) -> Completed:
                captured["cmd"] = cmd
                return Completed()

            env = {
                "POLYGEN_UNREAL_PROJECT": str(project),
                "POLYGEN_UNREAL_TARGET": "GameEditor",
                "POLYGEN_UNREAL_HEADER_DIR": str(header_dir),
                "POLYGEN_UNREAL_UBT": "",
                "UNREAL_BUILD_TOOL": "",
                "POLYGEN_UNREAL_ENGINE_ROOT": str(engine_root),
            }
            with (
                patch.object(compile_unreal.shutil, "which", return_value=None),
                patch.object(compile_unreal.subprocess, "run", fake_run),
            ):
                exit_code = self.run_helper([str(header)], env)

        self.assertEqual(exit_code, 0)
        self.assertEqual(captured["cmd"][0], str(ubt))

    def test_discovers_ubt_from_epic_manifest(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            header = root / "PolygenSchema.h"
            header.write_text("#pragma once\n", encoding="utf-8")
            project = root / "Game.uproject"
            project.write_text("{}", encoding="utf-8")
            header_dir = root / "headers"
            engine_root = root / "EpicGames" / "UE_5.7"
            ubt = engine_root / "Engine" / "Binaries" / "DotNET" / "UnrealBuildTool" / "UnrealBuildTool.exe"
            manifest_dir = root / "Epic" / "EpicGamesLauncher" / "Data" / "Manifests"
            ubt.parent.mkdir(parents=True)
            manifest_dir.mkdir(parents=True)
            ubt.write_text("", encoding="utf-8")
            (manifest_dir / "engine.item").write_text(
                json.dumps(
                    {
                        "AppName": "UE_5.7",
                        "DisplayName": "Unreal Engine",
                        "InstallLocation": str(engine_root),
                    }
                ),
                encoding="utf-8",
            )
            captured: dict[str, list[str]] = {}

            def fake_run(cmd: list[str]) -> Completed:
                captured["cmd"] = cmd
                return Completed()

            env = {
                "POLYGEN_UNREAL_PROJECT": str(project),
                "POLYGEN_UNREAL_TARGET": "GameEditor",
                "POLYGEN_UNREAL_HEADER_DIR": str(header_dir),
                "POLYGEN_UNREAL_UBT": "",
                "UNREAL_BUILD_TOOL": "",
                "ProgramData": str(root),
            }
            with (
                patch.object(compile_unreal.shutil, "which", return_value=None),
                patch.object(compile_unreal.subprocess, "run", fake_run),
            ):
                exit_code = self.run_helper([str(header)], env)

        self.assertEqual(exit_code, 0)
        self.assertEqual(captured["cmd"][0], str(ubt))

    def test_compile_uses_prepared_fixture_root_env(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            header = root / "PolygenSchema.h"
            header.write_text("#pragma once\n", encoding="utf-8")
            fixture = root / "Fixture"
            env_from_fixture = prepare_unreal_fixture.prepare_fixture(fixture)
            captured: dict[str, list[str]] = {}

            def fake_run(cmd: list[str]) -> Completed:
                captured["cmd"] = cmd
                return Completed()

            env = {
                "POLYGEN_UNREAL_FIXTURE_ROOT": str(fixture),
                "POLYGEN_UNREAL_UBT": "FakeUnrealBuildTool",
            }
            with patch.object(compile_unreal.subprocess, "run", fake_run):
                exit_code = self.run_helper([str(header)], env)
            copied_header_exists = (Path(env_from_fixture["POLYGEN_UNREAL_HEADER_DIR"]) / header.name).is_file()

            self.assertEqual(exit_code, 0)
            self.assertEqual(captured["cmd"][0], "FakeUnrealBuildTool")
            self.assertIn(f"-Project={Path(env_from_fixture['POLYGEN_UNREAL_PROJECT']).resolve()}", captured["cmd"])
            self.assertTrue(copied_header_exists)

    def test_missing_required_env_fails_before_copy(self) -> None:
        with TemporaryDirectory() as temp:
            header = Path(temp) / "PolygenSchema.h"
            header.write_text("#pragma once\n", encoding="utf-8")
            with self.assertRaises(SystemExit) as raised:
                self.run_helper([str(header)], {"POLYGEN_UNREAL_TARGET": "GameEditor"})

        self.assertEqual(raised.exception.code, 1)

    def test_missing_project_fails_before_ubt(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            header = root / "PolygenSchema.h"
            header.write_text("#pragma once\n", encoding="utf-8")
            env = {
                "POLYGEN_UNREAL_PROJECT": str(root / "Missing.uproject"),
                "POLYGEN_UNREAL_TARGET": "GameEditor",
                "POLYGEN_UNREAL_HEADER_DIR": str(root / "headers"),
                "POLYGEN_UNREAL_UBT": "FakeUnrealBuildTool",
            }
            with patch.object(compile_unreal.subprocess, "run") as run:
                exit_code = self.run_helper([str(header)], env)

        self.assertEqual(exit_code, 1)
        run.assert_not_called()

    def test_no_headers_fails_before_env_requirements(self) -> None:
        with TemporaryDirectory() as temp:
            exit_code = self.run_helper([str(Path(temp) / "*.h")], {})

        self.assertEqual(exit_code, 1)

    def test_prepare_fixture_creates_minimal_project_layout(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp) / "Fixture"
            env = prepare_unreal_fixture.prepare_fixture(root)

            project = root / "PolygenSmoke.uproject"
            build_cs = root / "Source" / "PolygenSmoke" / "PolygenSmoke.Build.cs"
            target_cs = root / "Source" / "PolygenSmokeEditor.Target.cs"
            module_cpp = root / "Source" / "PolygenSmoke" / "Private" / "PolygenSmoke.cpp"
            header_dir = root / "Source" / "PolygenSmoke" / "Public" / "Polygen"

            self.assertTrue(project.is_file())
            self.assertTrue(build_cs.is_file())
            self.assertTrue(target_cs.is_file())
            self.assertTrue(module_cpp.is_file())
            self.assertTrue(header_dir.is_dir())
            self.assertEqual(env["POLYGEN_UNREAL_PROJECT"], str(project))
            self.assertEqual(env["POLYGEN_UNREAL_TARGET"], "PolygenSmokeEditor")
            self.assertEqual(env["POLYGEN_UNREAL_HEADER_DIR"], str(header_dir))

    def test_validator_rejects_missing_local_polygen_include(self) -> None:
        with TemporaryDirectory() as temp:
            header = Path(temp) / "PolygenSchema.h"
            header.write_text(
                "\n".join(
                    [
                        "#pragma once",
                        '#include "CoreMinimal.h"',
                        '#include "PolygenCommon.h"',
                        '#include "PolygenSchema.generated.h"',
                        "",
                        "USTRUCT(BlueprintType)",
                        "struct FUser",
                        "{",
                        "    GENERATED_BODY()",
                        "    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = \"Data\")",
                        "    int32 id = 0;",
                        "    FUser() = default;",
                        "};",
                    ]
                ),
                encoding="utf-8",
            )

            with self.assertRaises(ValueError) as raised:
                validate_unreal.validate_header(header)

        self.assertIn("local include PolygenCommon.h does not exist", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
