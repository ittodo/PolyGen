#!/usr/bin/env python3
"""Validate generated PolyGen Kotlin files without requiring kotlinc."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


SCALAR_TYPES = {
    "Int",
    "Long",
    "Float",
    "Double",
    "Boolean",
    "String",
    "ByteArray",
    "Instant",
}

IDENT_RE = r"[A-Za-z_][A-Za-z0-9_]*"
DATA_CLASS_RE = re.compile(rf"^data\s+class\s+({IDENT_RE})\s*\(\s*$")
ENUM_CLASS_RE = re.compile(rf"^enum\s+class\s+({IDENT_RE})\s*(?:\(\s*val\s+value:\s+Int\s*\))?\s*\{{\s*$")
FIELD_RE = re.compile(rf"^val\s+({IDENT_RE}):\s+([^=,]+?)(?:\s*=\s*(.*?))?,?\s*$")
ENUM_MEMBER_RE = re.compile(rf"^({IDENT_RE})\((-?\d+)\)([,;])\s*$")


@dataclass
class KtField:
    name: str
    type_name: str
    default: str | None
    line: int


@dataclass
class KtDataClass:
    name: str
    line: int
    fields: list[KtField] = field(default_factory=list)


@dataclass
class KtEnum:
    name: str
    line: int
    has_value_ctor: bool
    members: list[tuple[str, int, str, int]] = field(default_factory=list)
    has_from_value: bool = False


def fail(path: Path, line: int, message: str) -> None:
    location = f"{path}:{line}" if line else str(path)
    raise ValueError(f"{location}: {message}")


def require(condition: bool, path: Path, line: int, message: str) -> None:
    if not condition:
        fail(path, line, message)


def strip_comment(line: str) -> str:
    return line.split("//", 1)[0].strip()


def split_lines(path: Path) -> list[tuple[int, str]]:
    return [(line_no, strip_comment(line)) for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1)]


def parse_data_class(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int) -> tuple[KtDataClass, int]:
    data_class = KtDataClass(name=name, line=line_no)
    seen_fields: set[str] = set()
    idx = start + 1

    while idx < len(lines):
        field_line_no, line = lines[idx]
        if not line:
            idx += 1
            continue
        if line == ")":
            return data_class, idx + 1

        match = FIELD_RE.match(line)
        require(match is not None, path, field_line_no, f"invalid data class field {line!r}")
        field_name, type_name, default = match.groups()
        type_name = type_name.strip()
        default = default.strip() if default is not None else None
        require(field_name not in seen_fields, path, field_line_no, f"duplicate field {field_name!r}")
        seen_fields.add(field_name)
        data_class.fields.append(KtField(field_name, type_name, default, field_line_no))
        idx += 1

    fail(path, line_no, f"unclosed data class {name}")


def parse_enum(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int, has_value_ctor: bool) -> tuple[KtEnum, int]:
    enum_def = KtEnum(name=name, line=line_no, has_value_ctor=has_value_ctor)
    brace_depth = 1
    idx = start + 1
    in_companion = False

    while idx < len(lines):
        current_line_no, line = lines[idx]
        if not line:
            idx += 1
            continue

        if line.startswith("companion object"):
            in_companion = True
        if "fun fromValue(value: Int):" in line:
            expected = f"fun fromValue(value: Int): {name} = entries.first {{ it.value == value }}"
            require(line == expected, path, current_line_no, f"invalid fromValue helper for enum {name}")
            enum_def.has_from_value = True

        if not in_companion and line != "}":
            match = ENUM_MEMBER_RE.match(line)
            require(match is not None, path, current_line_no, f"invalid enum member {line!r}")
            member_name, value, suffix = match.groups()
            enum_def.members.append((member_name, int(value), suffix, current_line_no))

        brace_depth += line.count("{")
        brace_depth -= line.count("}")
        idx += 1
        if brace_depth == 0:
            return enum_def, idx

    fail(path, line_no, f"unclosed enum class {name}")


def parse_kotlin(path: Path) -> tuple[list[KtDataClass], list[KtEnum]]:
    lines = split_lines(path)
    data_classes: list[KtDataClass] = []
    enums: list[KtEnum] = []
    type_names: set[str] = set()
    idx = 0
    serializable_pending = False

    while idx < len(lines):
        line_no, line = lines[idx]
        if not line:
            idx += 1
            continue
        if line.startswith("import "):
            idx += 1
            continue
        if line == "@Serializable":
            serializable_pending = True
            idx += 1
            continue

        data_match = DATA_CLASS_RE.match(line)
        if data_match is not None:
            name = data_match.group(1)
            require(serializable_pending, path, line_no, f"data class {name} must be annotated with @Serializable")
            require(name not in type_names, path, line_no, f"duplicate type name {name!r}")
            type_names.add(name)
            data_class, idx = parse_data_class(path, lines, idx, name, line_no)
            data_classes.append(data_class)
            serializable_pending = False
            continue

        enum_match = ENUM_CLASS_RE.match(line)
        if enum_match is not None:
            name = enum_match.group(1)
            require(serializable_pending, path, line_no, f"enum class {name} must be annotated with @Serializable")
            require(name not in type_names, path, line_no, f"duplicate type name {name!r}")
            type_names.add(name)
            enum_def, idx = parse_enum(path, lines, idx, name, line_no, "(val value: Int)" in line)
            enums.append(enum_def)
            serializable_pending = False
            continue

        if serializable_pending:
            fail(path, line_no, "@Serializable must be followed by a data class or enum class")

        idx += 1

    validate_model(path, data_classes, enums)
    return data_classes, enums


def validate_model(path: Path, data_classes: list[KtDataClass], enums: list[KtEnum]) -> None:
    type_names = {item.name for item in data_classes} | {item.name for item in enums}
    for data_class in data_classes:
        for field_def in data_class.fields:
            require_valid_type(path, field_def.line, field_def.type_name, type_names)
            if field_def.type_name.endswith("?") and field_def.default is not None:
                require(field_def.default == "null", path, field_def.line, "nullable field default must be null")
            if field_def.type_name.startswith("List<") and field_def.default is not None:
                require(field_def.default == "emptyList()", path, field_def.line, "list field default must be emptyList()")

    for enum_def in enums:
        if not enum_def.members:
            require(not enum_def.has_value_ctor, path, enum_def.line, "empty enum must not declare a value constructor")
            continue

        require(enum_def.has_value_ctor, path, enum_def.line, f"enum {enum_def.name} must declare val value: Int")
        require(enum_def.has_from_value, path, enum_def.line, f"enum {enum_def.name} must include fromValue helper")
        seen_names: set[str] = set()
        seen_values: set[int] = set()
        for idx, (name, value, suffix, line) in enumerate(enum_def.members):
            require(name not in seen_names, path, line, f"duplicate enum member {name!r}")
            require(value not in seen_values, path, line, f"duplicate enum value {value}")
            expected_suffix = ";" if idx + 1 == len(enum_def.members) else ","
            require(suffix == expected_suffix, path, line, f"enum member suffix must be {expected_suffix!r}")
            seen_names.add(name)
            seen_values.add(value)


def require_valid_type(path: Path, line: int, type_name: str, type_names: set[str]) -> None:
    if type_name.endswith("?"):
        require_valid_type(path, line, type_name[:-1], type_names)
        return

    if type_name.startswith("List<") and type_name.endswith(">"):
        require_valid_type(path, line, type_name[len("List<") : -1], type_names)
        return

    require(
        type_name in SCALAR_TYPES or type_name in type_names,
        path,
        line,
        f"unknown Kotlin field type {type_name!r}",
    )


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: validate_kotlin.py <kt> [...]", file=sys.stderr)
        return 2

    for arg in argv[1:]:
        path = Path(arg)
        data_classes, enums = parse_kotlin(path)
        if data_classes or enums:
            text = path.read_text(encoding="utf-8")
            require("import kotlinx.serialization.Serializable" in text, path, 0, "missing kotlinx.serialization.Serializable import")

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
