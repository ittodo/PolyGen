#!/usr/bin/env python3
"""Validate generated PolyGen Swift files without requiring swiftc."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


SCALAR_TYPES = {
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "Float",
    "Double",
    "Bool",
    "String",
    "Data",
    "Date",
}

IDENT_RE = r"[A-Za-z_][A-Za-z0-9_]*"
STRUCT_RE = re.compile(rf"^struct\s+({IDENT_RE}):\s+Codable,\s+Hashable\s*\{{\s*$")
ENUM_RE = re.compile(rf"^enum\s+({IDENT_RE}):\s+Int,\s+Codable,\s+CaseIterable,\s+Hashable\s*\{{\s*$")
MODEL_RE = re.compile(rf"^final\s+class\s+({IDENT_RE})Model\s*\{{\s*$")
FIELD_RE = re.compile(rf"^(let|var)\s+({IDENT_RE}):\s+([^=]+?)(?:\s*=\s*(.+))?\s*$")
CASE_RE = re.compile(rf"^case\s+({IDENT_RE})\s*=\s*(-?\d+)\s*$")
INIT_RE = re.compile(r"^init\((.*)\)\s*\{\s*$")


@dataclass
class SwiftField:
    keyword: str
    name: str
    type_name: str
    default: str | None
    line: int


@dataclass
class SwiftStruct:
    name: str
    line: int
    fields: list[SwiftField] = field(default_factory=list)


@dataclass
class SwiftEnum:
    name: str
    line: int
    cases: list[tuple[str, int, int]] = field(default_factory=list)


@dataclass
class SwiftModel:
    name: str
    line: int
    fields: list[SwiftField] = field(default_factory=list)
    init_params: set[str] = field(default_factory=set)
    assigned_fields: set[str] = field(default_factory=set)


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


def collect_type_names(path: Path, lines: list[tuple[int, str]]) -> set[str]:
    type_names: set[str] = set()
    for line_no, line in lines:
        match = STRUCT_RE.match(line) or ENUM_RE.match(line)
        if match is not None:
            name = match.group(1)
            require(name not in type_names, path, line_no, f"duplicate type name {name!r}")
            type_names.add(name)
            continue

        model_match = MODEL_RE.match(line)
        if model_match is not None:
            name = f"{model_match.group(1)}Model"
            require(name not in type_names, path, line_no, f"duplicate type name {name!r}")
            type_names.add(name)
    return type_names


def parse_struct(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int) -> tuple[SwiftStruct, int]:
    struct = SwiftStruct(name=name, line=line_no)
    seen_fields: set[str] = set()
    idx = start + 1

    while idx < len(lines):
        field_line, line = lines[idx]
        if not line:
            idx += 1
            continue
        if line == "}":
            return struct, idx + 1

        match = FIELD_RE.match(line)
        require(match is not None, path, field_line, f"invalid struct field {line!r}")
        keyword, field_name, type_name, default = match.groups()
        require(keyword in ("let", "var"), path, field_line, f"invalid field keyword {keyword!r}")
        require(field_name not in seen_fields, path, field_line, f"duplicate field {field_name!r}")
        seen_fields.add(field_name)
        struct.fields.append(
            SwiftField(keyword, field_name, type_name.strip(), default.strip() if default else None, field_line)
        )
        idx += 1

    fail(path, line_no, f"unclosed struct {name}")


def parse_enum(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int) -> tuple[SwiftEnum, int]:
    enum_def = SwiftEnum(name=name, line=line_no)
    seen_names: set[str] = set()
    seen_values: set[int] = set()
    idx = start + 1

    while idx < len(lines):
        case_line, line = lines[idx]
        if not line:
            idx += 1
            continue
        if line == "}":
            return enum_def, idx + 1

        match = CASE_RE.match(line)
        require(match is not None, path, case_line, f"invalid enum case {line!r}")
        case_name, value = match.groups()
        numeric_value = int(value)
        require(case_name not in seen_names, path, case_line, f"duplicate enum case {case_name!r}")
        require(numeric_value not in seen_values, path, case_line, f"duplicate enum value {numeric_value}")
        seen_names.add(case_name)
        seen_values.add(numeric_value)
        enum_def.cases.append((case_name, numeric_value, case_line))
        idx += 1

    fail(path, line_no, f"unclosed enum {name}")


def parse_model(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int) -> tuple[SwiftModel, int]:
    model = SwiftModel(name=f"{name}Model", line=line_no)
    seen_fields: set[str] = set()
    idx = start + 1

    while idx < len(lines):
        current_line_no, line = lines[idx]
        if not line or line == "@Attribute(.transformable)":
            idx += 1
            continue
        if line == "}":
            return model, idx + 1

        init_match = INIT_RE.match(line)
        if init_match is not None:
            params = init_match.group(1)
            model.init_params = parse_init_params(params)
            idx += 1
            while idx < len(lines):
                assign_line_no, assign_line = lines[idx]
                if not assign_line:
                    idx += 1
                    continue
                if assign_line == "}":
                    idx += 1
                    break
                assign_match = re.match(rf"^self\.({IDENT_RE})\s*=\s*({IDENT_RE})\s*$", assign_line)
                require(assign_match is not None, path, assign_line_no, f"invalid model assignment {assign_line!r}")
                left, right = assign_match.groups()
                require(left == right, path, assign_line_no, "model initializer must assign matching parameter")
                model.assigned_fields.add(left)
                idx += 1
            continue

        match = FIELD_RE.match(line)
        require(match is not None, path, current_line_no, f"invalid model field {line!r}")
        keyword, field_name, type_name, default = match.groups()
        require(keyword == "var", path, current_line_no, "SwiftData model fields must use var")
        require(default is None, path, current_line_no, "SwiftData model stored fields should default in init only")
        require(field_name not in seen_fields, path, current_line_no, f"duplicate model field {field_name!r}")
        seen_fields.add(field_name)
        model.fields.append(SwiftField(keyword, field_name, type_name.strip(), None, current_line_no))
        idx += 1

    fail(path, line_no, f"unclosed model {model.name}")


def parse_init_params(params: str) -> set[str]:
    result: set[str] = set()
    for raw_param in [part.strip() for part in params.split(",") if part.strip()]:
        name = raw_param.split(":", 1)[0].strip()
        if name:
            result.add(name)
    return result


def parse_swift(path: Path) -> tuple[list[SwiftStruct], list[SwiftEnum], list[SwiftModel]]:
    lines = split_lines(path)
    type_names = collect_type_names(path, lines)
    structs: list[SwiftStruct] = []
    enums: list[SwiftEnum] = []
    models: list[SwiftModel] = []
    idx = 0
    model_pending = False

    while idx < len(lines):
        line_no, line = lines[idx]
        if not line or line.startswith("import "):
            idx += 1
            continue
        if line == "@Model":
            model_pending = True
            idx += 1
            continue
        if line.startswith("@"):
            fail(path, line_no, f"unexpected top-level attribute {line!r}")

        struct_match = STRUCT_RE.match(line)
        if struct_match is not None:
            struct, idx = parse_struct(path, lines, idx, struct_match.group(1), line_no)
            structs.append(struct)
            model_pending = False
            continue

        enum_match = ENUM_RE.match(line)
        if enum_match is not None:
            enum_def, idx = parse_enum(path, lines, idx, enum_match.group(1), line_no)
            enums.append(enum_def)
            model_pending = False
            continue

        model_match = MODEL_RE.match(line)
        if model_match is not None:
            require(model_pending, path, line_no, f"model {model_match.group(1)} must be preceded by @Model")
            model, idx = parse_model(path, lines, idx, model_match.group(1), line_no)
            models.append(model)
            model_pending = False
            continue

        if line.startswith("enum PolygenRedisKeys"):
            validate_redis_helper(path, lines, idx)
            return structs, enums, models

        fail(path, line_no, f"unexpected top-level statement {line!r}")

    validate_model(path, type_names, structs, enums, models)
    return structs, enums, models


def validate_model(
    path: Path,
    type_names: set[str],
    structs: list[SwiftStruct],
    enums: list[SwiftEnum],
    models: list[SwiftModel],
) -> None:
    for struct in structs:
        require(struct.fields, path, struct.line, f"struct {struct.name} must contain fields")
        for field_def in struct.fields:
            require_valid_type(path, field_def.line, field_def.type_name, type_names)
            validate_default(path, field_def)

    for enum_def in enums:
        require(enum_def.cases, path, enum_def.line, f"enum {enum_def.name} must contain cases")

    for model in models:
        field_names = {field_def.name for field_def in model.fields}
        require(field_names == model.init_params, path, model.line, f"model {model.name} init params must match fields")
        require(field_names == model.assigned_fields, path, model.line, f"model {model.name} init assignments must match fields")
        for field_def in model.fields:
            require_valid_type(path, field_def.line, field_def.type_name, type_names)


def validate_default(path: Path, field_def: SwiftField) -> None:
    if field_def.default is None:
        return
    if field_def.type_name.endswith("?"):
        require(field_def.default == "nil", path, field_def.line, "optional field default must be nil")
    if field_def.type_name.startswith("["):
        require(field_def.default == "[]", path, field_def.line, "array field default must be []")


def require_valid_type(path: Path, line: int, type_name: str, type_names: set[str]) -> None:
    if type_name.endswith("?"):
        require_valid_type(path, line, type_name[:-1], type_names)
        return

    if type_name.startswith("[") and type_name.endswith("]"):
        require_valid_type(path, line, type_name[1:-1], type_names)
        return

    require(
        type_name in SCALAR_TYPES or type_name in type_names,
        path,
        line,
        f"unknown Swift field type {type_name!r}",
    )


def validate_redis_helper(path: Path, lines: list[tuple[int, str]], start: int) -> None:
    text = path.read_text(encoding="utf-8")
    require("import Foundation" in text, path, 0, "Redis helper must import Foundation")
    require("static let keyNamespace = \"polygen\"" in text, path, 0, "Redis helper missing key namespace")
    require("static func segment(_ value: Any) -> String" in text, path, 0, "Redis helper missing segment function")

    brace_depth = 0
    for idx in range(start, len(lines)):
        line = lines[idx][1]
        brace_depth += line.count("{")
        brace_depth -= line.count("}")
        if brace_depth == 0:
            return

    fail(path, lines[start][0], "unclosed PolygenRedisKeys helper")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: validate_swift.py <swift> [...]", file=sys.stderr)
        return 2

    for arg in argv[1:]:
        path = Path(arg)
        structs, enums, models = parse_swift(path)
        text = path.read_text(encoding="utf-8")
        if structs or enums or models:
            require("import Foundation" in text, path, 0, "missing Foundation import")
        if models:
            require("import SwiftData" in text, path, 0, "SwiftData model file must import SwiftData")

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
