#!/usr/bin/env python3
"""Validate generated PolyGen Redis cache descriptors and Lua key helpers."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


ALLOWED_STRATEGIES = {"full_load", "on_demand", "write_through", "write_back"}


def fail(path: Path, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def require(condition: bool, path: Path, message: str) -> None:
    if not condition:
        fail(path, message)


def snake_case(value: str) -> str:
    value = value.replace(".", "_").replace("-", "_")
    out: list[str] = []
    previous_lower_or_digit = False
    for char in value:
        if char.isupper() and previous_lower_or_digit:
            out.append("_")
        out.append(char.lower())
        previous_lower_or_digit = char.islower() or char.isdigit()
    return "".join(out)


def index_name(value: str) -> str:
    return snake_case(value)


def key_prefix(fqn: str) -> str:
    parts = fqn.split(".")
    if len(parts) == 1:
        namespace = ""
        simple = parts[0]
    else:
        namespace = ".".join(parts[:-1])
        simple = parts[-1]
    return f"polygen:{namespace}:{simple}"


def read_descriptor(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(path, f"invalid JSON: {exc}")

    require(isinstance(data, dict), path, "descriptor must be a JSON object")
    require(data.get("format") == "polygen-redis-cache-schema", path, "invalid format")
    require(data.get("version") == 1, path, "invalid version")
    require(data.get("keyNamespace") == "polygen", path, "invalid keyNamespace")
    require(isinstance(data.get("tables"), list), path, "tables must be a list")
    validate_tables(path, data["tables"])
    return data


def validate_tables(path: Path, tables: list[Any]) -> None:
    seen_fqns: set[str] = set()
    seen_names: set[str] = set()
    for table in tables:
        require(isinstance(table, dict), path, "table entry must be an object")
        name = require_string(path, table, "name")
        fqn = require_string(path, table, "fqn")
        cache_strategy = require_string(path, table, "cacheStrategy")
        primary_key = require_string(path, table, "primaryKey")
        value_encoding = require_string(path, table, "valueEncoding")
        entity_pattern = require_string(path, table, "entityKeyPattern")
        all_set_key = require_string(path, table, "allSetKey")

        require(name not in seen_names, path, f"duplicate table name {name!r}")
        require(fqn not in seen_fqns, path, f"duplicate table fqn {fqn!r}")
        require(cache_strategy in ALLOWED_STRATEGIES, path, f"unknown cache strategy {cache_strategy!r}")
        require(value_encoding == "json", path, "valueEncoding must be json")

        ttl = table.get("ttlSeconds")
        require(ttl is None or (isinstance(ttl, int) and ttl >= 0), path, "ttlSeconds must be null or non-negative int")

        prefix = key_prefix(fqn)
        require(entity_pattern == f"{prefix}:{{{primary_key}}}", path, f"invalid entityKeyPattern for {fqn}")
        require(all_set_key == f"{prefix}:all", path, f"invalid allSetKey for {fqn}")
        require(isinstance(table.get("indexes"), list), path, f"indexes must be a list for {fqn}")
        validate_indexes(path, fqn, primary_key, table["indexes"])

        seen_names.add(name)
        seen_fqns.add(fqn)


def validate_indexes(path: Path, fqn: str, primary_key: str, indexes: list[Any]) -> None:
    seen_names: set[str] = set()
    seen_patterns: set[str] = set()
    has_primary_key_index = False
    prefix = key_prefix(fqn)

    for index in indexes:
        require(isinstance(index, dict), path, f"index entry for {fqn} must be an object")
        name = require_string(path, index, "name")
        unique = index.get("unique")
        fields = index.get("fields")
        pattern = require_string(path, index, "keyPattern")

        require(isinstance(unique, bool), path, f"index {name} unique must be bool")
        require(isinstance(fields, list) and fields, path, f"index {name} fields must be a non-empty list")
        require(all(isinstance(field, str) and field for field in fields), path, f"index {name} fields must be strings")
        require(name not in seen_names, path, f"duplicate index name {name!r}")
        require(pattern not in seen_patterns, path, f"duplicate index keyPattern {pattern!r}")
        require(pattern == f"{prefix}:idx:{index_name(name)}:{{value}}", path, f"invalid index keyPattern for {fqn}.{name}")

        if fields == [primary_key] and unique:
            has_primary_key_index = True
        seen_names.add(name)
        seen_patterns.add(pattern)

    require(has_primary_key_index, path, f"{fqn} must include a unique primary key index")


def require_string(path: Path, obj: dict[str, Any], key: str) -> str:
    value = obj.get(key)
    require(isinstance(value, str) and value != "", path, f"{key} must be a non-empty string")
    return value


def validate_lua(path: Path, descriptor: dict[str, Any]) -> None:
    text = path.read_text(encoding="utf-8")
    require("local redis_keys = {}" in text, path, "missing redis_keys table")
    require(text.rstrip().endswith("return redis_keys"), path, "Lua helper must return redis_keys")

    functions = set(re.findall(r"^function redis_keys\.([A-Za-z_][A-Za-z0-9_]*)\(", text, re.MULTILINE))
    expected: set[str] = set()
    for table in descriptor["tables"]:
        table_name = table["name"]
        base = snake_case(table_name)
        pk = table["primaryKey"]
        prefix = key_prefix(table["fqn"])

        expected.add(f"{base}_entity")
        require(
            f'function redis_keys.{base}_entity({snake_case(pk)})' in text,
            path,
            f"missing Lua entity helper for {table_name}",
        )
        require(f'return "{prefix}:" .. tostring({snake_case(pk)})' in text, path, f"invalid entity helper body for {table_name}")

        expected.add(f"{base}_all")
        require(f"function redis_keys.{base}_all()" in text, path, f"missing Lua all helper for {table_name}")
        require(f'return "{prefix}:all"' in text, path, f"invalid all helper body for {table_name}")

        for index in table["indexes"]:
            fn_name = f"{base}_idx_{snake_case(index_name(index['name']))}"
            expected.add(fn_name)
            expected_prefix = index["keyPattern"].replace("{value}", "")
            require(f"function redis_keys.{fn_name}(value)" in text, path, f"missing Lua index helper {fn_name}")
            require(f'return "{expected_prefix}" .. tostring(value)' in text, path, f"invalid Lua index helper body {fn_name}")

    require(functions == expected, path, f"Lua helper functions differ: expected {sorted(expected)}, got {sorted(functions)}")


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: validate_redis.py <schema.redis.json> <schema.redis.lua>", file=sys.stderr)
        return 2

    descriptor_path = Path(argv[1])
    lua_path = Path(argv[2])
    descriptor = read_descriptor(descriptor_path)
    validate_lua(lua_path, descriptor)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
