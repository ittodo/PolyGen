#!/usr/bin/env python3
"""Validate generated PolyGen proto3 files without requiring protoc."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


SCALAR_TYPES = {
    "double",
    "float",
    "int32",
    "int64",
    "uint32",
    "uint64",
    "bool",
    "string",
    "bytes",
    "google.protobuf.Timestamp",
}

SYNTAX_RE = re.compile(r'^syntax\s*=\s*"proto3"\s*;\s*$')
PACKAGE_RE = re.compile(r"^package\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*;\s*$")
IMPORT_RE = re.compile(r'^import\s+"[^"]+"\s*;\s*$')
BLOCK_RE = re.compile(r"^(enum|message)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{\s*$")
ENUM_VALUE_RE = re.compile(r"^([A-Z][A-Z0-9_]*)\s*=\s*(-?\d+)\s*;\s*$")
FIELD_RE = re.compile(
    r"^(?:(optional|repeated)\s+)?([A-Za-z_][A-Za-z0-9_.]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(\d+)\s*;\s*$"
)


@dataclass
class ProtoField:
    label: str | None
    type_name: str
    name: str
    number: int


@dataclass
class ProtoBlock:
    kind: str
    name: str
    line: int
    fields: list[ProtoField] = field(default_factory=list)
    enum_values: list[tuple[str, int, int]] = field(default_factory=list)


def fail(path: Path, line: int, message: str) -> None:
    location = f"{path}:{line}" if line else str(path)
    raise ValueError(f"{location}: {message}")


def require(condition: bool, path: Path, line: int, message: str) -> None:
    if not condition:
        fail(path, line, message)


def strip_comment(line: str) -> str:
    return line.split("//", 1)[0].strip()


def parse_proto(path: Path) -> list[ProtoBlock]:
    lines = path.read_text(encoding="utf-8").splitlines()
    blocks: list[ProtoBlock] = []
    current: ProtoBlock | None = None
    saw_syntax = False
    saw_package = False
    type_names: set[str] = set()

    for line_no, raw_line in enumerate(lines, start=1):
        line = strip_comment(raw_line)
        if not line:
            continue

        if current is None:
            if SYNTAX_RE.match(line):
                require(not saw_syntax, path, line_no, "duplicate syntax declaration")
                saw_syntax = True
                continue
            if PACKAGE_RE.match(line):
                require(saw_syntax, path, line_no, "package must appear after syntax")
                require(not saw_package, path, line_no, "duplicate package declaration")
                saw_package = True
                continue
            if IMPORT_RE.match(line):
                require(saw_syntax and saw_package, path, line_no, "import must appear after package")
                continue

            block_match = BLOCK_RE.match(line)
            require(block_match is not None, path, line_no, f"unexpected top-level statement {line!r}")
            kind, name = block_match.groups()
            require(name not in type_names, path, line_no, f"duplicate type name {name!r}")
            type_names.add(name)
            current = ProtoBlock(kind=kind, name=name, line=line_no)
            continue

        if line == "}":
            blocks.append(current)
            current = None
            continue

        require("{" not in line and "}" not in line, path, line_no, "nested blocks are not supported in generated proto")
        if current.kind == "enum":
            match = ENUM_VALUE_RE.match(line)
            require(match is not None, path, line_no, f"invalid enum value {line!r}")
            name, value = match.groups()
            current.enum_values.append((name, int(value), line_no))
        else:
            match = FIELD_RE.match(line)
            require(match is not None, path, line_no, f"invalid field declaration {line!r}")
            label, type_name, name, number = match.groups()
            current.fields.append(ProtoField(label, type_name, name, int(number)))

    require(saw_syntax, path, 0, "missing proto3 syntax declaration")
    require(saw_package, path, 0, "missing package declaration")
    require(current is None, path, current.line if current else 0, "unclosed block")
    require(len(blocks) > 0, path, 0, "no enum or message blocks found")
    validate_blocks(path, blocks)
    return blocks


def validate_blocks(path: Path, blocks: list[ProtoBlock]) -> None:
    type_names = {block.name for block in blocks}
    for block in blocks:
        if block.kind == "enum":
            validate_enum(path, block)
        else:
            validate_message(path, block, type_names)


def validate_enum(path: Path, block: ProtoBlock) -> None:
    require(len(block.enum_values) > 0, path, block.line, f"enum {block.name} must contain at least one value")
    first_name, first_value, first_line = block.enum_values[0]
    require(first_value == 0, path, first_line, f"enum {block.name} first value must be 0")

    seen_names: set[str] = set()
    seen_values: set[int] = set()
    for name, value, line in block.enum_values:
        require(name not in seen_names, path, line, f"duplicate enum value name {name!r}")
        require(value not in seen_values, path, line, f"duplicate enum numeric value {value}")
        require(name.startswith(block.name.upper()[:1]) or "_" in name, path, line, f"suspicious enum value name {name!r}")
        seen_names.add(name)
        seen_values.add(value)

    require(first_name.endswith("UNSPECIFIED") or first_value == 0, path, first_line, "invalid first enum value")


def validate_message(path: Path, block: ProtoBlock, type_names: set[str]) -> None:
    seen_names: set[str] = set()
    seen_numbers: set[int] = set()
    for expected_number, field_def in enumerate(block.fields, start=1):
        require(field_def.number == expected_number, path, block.line, f"message {block.name} field numbers must be contiguous")
        require(field_def.name not in seen_names, path, block.line, f"duplicate field name {field_def.name!r}")
        require(field_def.number not in seen_numbers, path, block.line, f"duplicate field number {field_def.number}")
        require(field_def.label in (None, "optional", "repeated"), path, block.line, f"invalid field label {field_def.label!r}")
        require(
            field_def.type_name in SCALAR_TYPES or field_def.type_name in type_names,
            path,
            block.line,
            f"unknown field type {field_def.type_name!r}",
        )
        seen_names.add(field_def.name)
        seen_numbers.add(field_def.number)


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: validate_proto.py <proto> [...]", file=sys.stderr)
        return 2

    for arg in argv[1:]:
        parse_proto(Path(arg))

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
