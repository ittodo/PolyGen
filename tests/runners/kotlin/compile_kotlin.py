"""Optional Kotlin compile gate for generated PolyGen Kotlin files."""

from __future__ import annotations

import argparse
import ctypes
import glob
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


def expand_inputs(patterns: list[str]) -> list[str]:
    files: list[str] = []
    for pattern in patterns:
        matches = glob.glob(pattern)
        if matches:
            files.extend(matches)
        else:
            files.append(pattern)
    return sorted({str(Path(path)) for path in files if Path(path).is_file()})


def split_env_args(value: str) -> list[str]:
    if not value.strip():
        return []
    if os.name != "nt":
        return shlex.split(value)

    argc = ctypes.c_int()
    ctypes.windll.shell32.CommandLineToArgvW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_int)]
    ctypes.windll.shell32.CommandLineToArgvW.restype = ctypes.POINTER(ctypes.c_wchar_p)
    ctypes.windll.kernel32.LocalFree.argtypes = [ctypes.c_void_p]
    argv = ctypes.windll.shell32.CommandLineToArgvW(value, ctypes.byref(argc))
    if not argv:
        raise OSError("CommandLineToArgvW failed")
    try:
        return [argv[index] for index in range(argc.value)]
    finally:
        ctypes.windll.kernel32.LocalFree(argv)


def should_use_argfile(args: list[str]) -> bool:
    return os.name == "nt" and any(os.pathsep in arg for arg in args)


def run_kotlinc(kotlinc: str, args: list[str]) -> int:
    if not should_use_argfile(args):
        cmd = [kotlinc, *args]
        print(" ".join(cmd))
        return subprocess.run(cmd).returncode

    with TemporaryDirectory() as temp:
        argfile = Path(temp) / "kotlinc.args"
        argfile.write_text("\n".join(args) + "\n", encoding="utf-8")
        cmd = [kotlinc, f"@{argfile}"]
        print(" ".join(cmd))
        return subprocess.run(cmd).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="+", help="Generated .kt files or glob patterns")
    parser.add_argument("--out", default=None, help="Output jar path")
    parser.add_argument("--run-main", default=None, help="Run the compiled Kotlin main class after compilation")
    args = parser.parse_args()

    kotlinc = os.environ.get("KOTLINC") or shutil.which("kotlinc")
    if not kotlinc:
        print("kotlinc was not found. Install Kotlin or set KOTLINC.", file=sys.stderr)
        return 1

    files = expand_inputs(args.files)
    if not files:
        print("no Kotlin files found to compile", file=sys.stderr)
        return 1

    output = args.out or str(Path(files[0]).parent / "polygen-generated.jar")
    compiler_args = split_env_args(os.environ.get("POLYGEN_KOTLIN_COMPILER_ARGS", ""))
    classpath = os.environ.get("POLYGEN_KOTLIN_CLASSPATH") or os.environ.get("KOTLIN_CLASSPATH")
    if classpath:
        compiler_args.extend(["-classpath", classpath])
    compiler_args.extend(files)
    compiler_args.extend(["-d", output])

    compile_exit = run_kotlinc(kotlinc, compiler_args)
    if compile_exit != 0 or not args.run_main:
        return compile_exit

    kotlin = os.environ.get("KOTLIN") or shutil.which("kotlin")
    if not kotlin:
        print("kotlin was not found. Install Kotlin or set KOTLIN.", file=sys.stderr)
        return 1

    runtime_classpath = [output]
    if classpath:
        runtime_classpath.append(classpath)
    run_cmd = [kotlin, "-classpath", os.pathsep.join(runtime_classpath), args.run_main]
    print(" ".join(run_cmd))
    return subprocess.run(run_cmd).returncode


if __name__ == "__main__":
    raise SystemExit(main())
