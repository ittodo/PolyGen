"""Optional Swift typecheck gate for generated PolyGen Swift files."""

from __future__ import annotations

import argparse
import glob
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path


def expand_inputs(patterns: list[str]) -> list[str]:
    files: list[str] = []
    for pattern in patterns:
        matches = glob.glob(pattern)
        if matches:
            files.extend(matches)
        else:
            files.append(pattern)
    return sorted({str(Path(path)) for path in files if Path(path).is_file()})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="+", help="Generated .swift files or glob patterns")
    parser.add_argument(
        "--include-swiftdata",
        action="store_true",
        help="Include schema_swiftdata.swift in the typecheck set",
    )
    args = parser.parse_args()

    swiftc = os.environ.get("SWIFTC") or shutil.which("swiftc")
    if not swiftc:
        print("swiftc was not found. Install Swift or set SWIFTC.", file=sys.stderr)
        return 1

    include_swiftdata = args.include_swiftdata or os.environ.get("POLYGEN_SWIFT_INCLUDE_SWIFTDATA") == "1"
    files = expand_inputs(args.files)
    if not include_swiftdata:
        files = [path for path in files if not Path(path).name.endswith("_swiftdata.swift")]
    if not files:
        print("no Swift files found to typecheck", file=sys.stderr)
        return 1

    cmd = [
        swiftc,
        *shlex.split(os.environ.get("POLYGEN_SWIFT_COMPILER_ARGS", "")),
        "-parse-as-library",
        "-typecheck",
        *files,
    ]
    print(" ".join(cmd))
    result = subprocess.run(cmd)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
