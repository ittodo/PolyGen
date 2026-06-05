from __future__ import annotations

import os
import sys
import unittest
import json
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.dont_write_bytecode = True

import check_optional_toolchains


class CheckOptionalToolchainsTests(unittest.TestCase):
    def run_checker(self, env: dict[str, str], args: list[str] | None = None) -> tuple[int, str]:
        output = StringIO()
        with (
            patch.dict(os.environ, env, clear=True),
            patch.object(sys, "argv", ["check_optional_toolchains.py", *(args or [])]),
            patch.object(check_optional_toolchains.shutil, "which", return_value=None),
            redirect_stdout(output),
        ):
            exit_code = check_optional_toolchains.main()
        return exit_code, output.getvalue()

    def test_reports_missing_tools_without_failing_by_default(self) -> None:
        exit_code, output = self.run_checker({})

        self.assertEqual(exit_code, 0)
        self.assertIn("kotlin: MISSING", output)
        self.assertIn("swift: MISSING", output)
        self.assertIn("unreal: MISSING", output)
        self.assertIn("set KOTLINC", output)
        self.assertIn("set SWIFTC", output)
        self.assertIn("set POLYGEN_UNREAL_PROJECT", output)

    def test_fail_on_missing_returns_nonzero(self) -> None:
        exit_code, _ = self.run_checker({}, ["--fail-on-missing"])

        self.assertEqual(exit_code, 1)

    def test_ready_environment_passes_fail_on_missing(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            kotlinc = root / "kotlinc"
            java = root / "java"
            swiftc = root / "swiftc"
            sdkroot = root / "Windows.sdk"
            ubt = root / "UnrealBuildTool.exe"
            classpath = root / "deps.jar"
            project = root / "Game.uproject"
            header_dir = root / "Headers"
            sdkroot.mkdir()
            header_dir.mkdir()
            for path in (kotlinc, java, swiftc, ubt, classpath, project):
                path.write_text("", encoding="utf-8")
            env = {
                "KOTLINC": str(kotlinc),
                "JAVA": str(java),
                "POLYGEN_KOTLIN_CLASSPATH": str(classpath),
                "SWIFTC": str(swiftc),
                "SDKROOT": str(sdkroot),
                "POLYGEN_UNREAL_UBT": str(ubt),
                "POLYGEN_UNREAL_PROJECT": str(project),
                "POLYGEN_UNREAL_TARGET": "GameEditor",
                "POLYGEN_UNREAL_HEADER_DIR": str(header_dir),
            }

            exit_code, output = self.run_checker(env, ["--fail-on-missing"])

        self.assertEqual(exit_code, 0)
        self.assertIn("kotlin: READY", output)
        self.assertIn("swift: READY", output)
        self.assertIn("unreal: READY", output)

    def test_swift_default_install_layout_is_discovered(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp) / "Programs" / "Swift"
            swiftc = root / "Toolchains" / "6.3.2+Asserts" / "usr" / "bin" / "swiftc.exe"
            sdkroot = root / "Platforms" / "6.3.2" / "Windows.platform" / "Developer" / "SDKs" / "Windows.sdk"
            runtime = root / "Runtimes" / "6.3.2" / "usr" / "bin"
            python = root / "Python-3.10.1" / "usr" / "bin"
            for path in (swiftc.parent, sdkroot, runtime, python):
                path.mkdir(parents=True)
            swiftc.write_text("", encoding="utf-8")

            with patch.dict(os.environ, {"LOCALAPPDATA": str(Path(temp))}, clear=True):
                status = check_optional_toolchains.check_swift()

        self.assertTrue(status.ready)
        self.assertEqual(status.env["SWIFTC"], str(swiftc))
        self.assertEqual(status.env["SDKROOT"], str(sdkroot))
        self.assertIn(str(runtime), status.env["PATH"])

    def test_prepared_unreal_fixture_root_supplies_project_env(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            fixture = root / "Fixture"
            project = fixture / "PolygenSmoke.uproject"
            header_dir = fixture / "Source" / "PolygenSmoke" / "Public" / "Polygen"
            ubt = root / "Engine" / "Binaries" / "DotNET" / "UnrealBuildTool" / "UnrealBuildTool.exe"
            project.parent.mkdir(parents=True)
            header_dir.mkdir(parents=True)
            ubt.parent.mkdir(parents=True)
            project.write_text("{}", encoding="utf-8")
            ubt.write_text("", encoding="utf-8")
            env = {
                "POLYGEN_UNREAL_ENGINE_ROOT": str(root),
                "POLYGEN_UNREAL_FIXTURE_ROOT": str(fixture),
            }

            with patch.dict(os.environ, env, clear=True):
                status = check_optional_toolchains.check_unreal()

        self.assertTrue(status.ready)
        self.assertEqual(status.env["POLYGEN_UNREAL_PROJECT"], str(project))
        self.assertEqual(status.env["POLYGEN_UNREAL_TARGET"], "PolygenSmokeEditor")
        self.assertEqual(status.env["POLYGEN_UNREAL_HEADER_DIR"], str(header_dir))

    def test_unreal_engine_root_discovers_ubt(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
            ubt = root / "Engine" / "Binaries" / "DotNET" / "UnrealBuildTool" / "UnrealBuildTool.exe"
            ubt.parent.mkdir(parents=True)
            ubt.write_text("", encoding="utf-8")
            env = {"POLYGEN_UNREAL_ENGINE_ROOT": str(root)}

            with patch.dict(os.environ, env, clear=True):
                status = check_optional_toolchains.check_unreal()

        ubt_check = next(check for check in status.checks if check.label == "UnrealBuildTool")
        self.assertTrue(ubt_check.ok)
        self.assertEqual(ubt_check.detail, str(ubt))

    def test_unreal_epic_manifest_discovers_ubt(self) -> None:
        with TemporaryDirectory() as temp:
            root = Path(temp)
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

            with patch.dict(os.environ, {"ProgramData": str(root)}, clear=True):
                status = check_optional_toolchains.check_unreal()

        ubt_check = next(check for check in status.checks if check.label == "UnrealBuildTool")
        self.assertTrue(ubt_check.ok)
        self.assertEqual(ubt_check.detail, str(ubt))


if __name__ == "__main__":
    unittest.main()
