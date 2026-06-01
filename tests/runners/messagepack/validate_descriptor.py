#!/usr/bin/env python3
"""Validate generated PolyGen MessagePack schema descriptors."""

from __future__ import annotations

import json
import sys
from pathlib import Path


VALID_TYPE_KINDS = {"table", "embed", "enum"}
VALID_WIRE_TYPES = {
    "array",
    "binary",
    "bool",
    "float32",
    "float64",
    "int",
    "object",
    "string",
    "timestamp",
    "uint",
}


def fail(path: Path, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def require(condition: bool, path: Path, message: str) -> None:
    if not condition:
        fail(path, message)


def validate_descriptor(path: Path) -> None:
    with path.open("r", encoding="utf-8") as handle:
        descriptor = json.load(handle)

    require(descriptor.get("format") == "polygen-messagepack-schema", path, "invalid format")
    require(descriptor.get("version") == 1, path, "invalid version")

    encoding = descriptor.get("encoding")
    require(isinstance(encoding, dict), path, "encoding must be an object")
    require(encoding.get("record") == "array", path, "record encoding must be array")
    require(encoding.get("fieldOrder") == "index", path, "field order must be index")
    require(encoding.get("optional") == "nil", path, "optional encoding must be nil")

    types = descriptor.get("types")
    require(isinstance(types, list), path, "types must be an array")
    require(len(types) > 0, path, "types must not be empty")

    seen_fqns: set[str] = set()
    for type_index, type_def in enumerate(types):
        require(isinstance(type_def, dict), path, f"types[{type_index}] must be an object")

        kind = type_def.get("kind")
        require(kind in VALID_TYPE_KINDS, path, f"types[{type_index}] has invalid kind {kind!r}")

        name = type_def.get("name")
        fqn = type_def.get("fqn")
        require(isinstance(name, str) and name, path, f"types[{type_index}] missing name")
        require(isinstance(fqn, str) and fqn, path, f"types[{type_index}] missing fqn")
        require(fqn not in seen_fqns, path, f"duplicate type fqn {fqn!r}")
        seen_fqns.add(fqn)

        if kind == "enum":
            validate_enum(path, type_index, type_def)
        else:
            validate_record(path, type_index, type_def)


def validate_enum(path: Path, type_index: int, type_def: dict) -> None:
    require(type_def.get("wireType") == "int", path, f"types[{type_index}] enum wireType must be int")
    members = type_def.get("members")
    require(isinstance(members, list), path, f"types[{type_index}] members must be an array")

    seen_names: set[str] = set()
    seen_values: set[int] = set()
    for member_index, member in enumerate(members):
        require(isinstance(member, dict), path, f"types[{type_index}].members[{member_index}] must be an object")
        name = member.get("name")
        value = member.get("value")
        require(isinstance(name, str) and name, path, f"types[{type_index}].members[{member_index}] missing name")
        require(isinstance(value, int), path, f"types[{type_index}].members[{member_index}] value must be int")
        require(name not in seen_names, path, f"duplicate enum member name {name!r}")
        require(value not in seen_values, path, f"duplicate enum member value {value!r}")
        seen_names.add(name)
        seen_values.add(value)


def validate_record(path: Path, type_index: int, type_def: dict) -> None:
    require(type_def.get("encoding") == "array", path, f"types[{type_index}] record encoding must be array")
    fields = type_def.get("fields")
    require(isinstance(fields, list), path, f"types[{type_index}] fields must be an array")

    for expected_index, field in enumerate(fields):
        require(isinstance(field, dict), path, f"types[{type_index}].fields[{expected_index}] must be an object")
        require(field.get("index") == expected_index, path, f"types[{type_index}] field indices must be contiguous")

        name = field.get("name")
        poly_type = field.get("polyType")
        wire_type = field.get("wireType")
        require(isinstance(name, str) and name, path, f"types[{type_index}].fields[{expected_index}] missing name")
        require(isinstance(poly_type, str) and poly_type, path, f"types[{type_index}].fields[{expected_index}] missing polyType")
        require(wire_type in VALID_WIRE_TYPES, path, f"types[{type_index}].fields[{expected_index}] invalid wireType {wire_type!r}")
        require(isinstance(field.get("optional"), bool), path, f"types[{type_index}].fields[{expected_index}] optional must be bool")
        require(isinstance(field.get("array"), bool), path, f"types[{type_index}].fields[{expected_index}] array must be bool")
        require(isinstance(field.get("primaryKey"), bool), path, f"types[{type_index}].fields[{expected_index}] primaryKey must be bool")
        require(isinstance(field.get("unique"), bool), path, f"types[{type_index}].fields[{expected_index}] unique must be bool")

        if "elementWireType" in field:
            require(
                field["elementWireType"] in VALID_WIRE_TYPES,
                path,
                f"types[{type_index}].fields[{expected_index}] invalid elementWireType",
            )
        if "foreignKey" in field:
            foreign_key = field["foreignKey"]
            require(isinstance(foreign_key, dict), path, f"types[{type_index}].fields[{expected_index}] foreignKey must be object")
            require(isinstance(foreign_key.get("table"), str) and foreign_key["table"], path, "foreignKey table missing")
            require(isinstance(foreign_key.get("field"), str) and foreign_key["field"], path, "foreignKey field missing")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: validate_descriptor.py <descriptor> [...]", file=sys.stderr)
        return 2

    for arg in argv[1:]:
        validate_descriptor(Path(arg))

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
