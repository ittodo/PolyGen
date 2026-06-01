#!/usr/bin/env python3
"""Validate generated PolyGen Mermaid ER diagrams."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ENTITY_START_RE = re.compile(r"^    (?P<name>[A-Za-z_][A-Za-z0-9_]*) \{$")
FIELD_RE = re.compile(r"^        (?P<type>[A-Za-z_][A-Za-z0-9_]*(?:\[\])?\??) (?P<name>[A-Za-z_][A-Za-z0-9_]*)(?: \"(?P<marker>[A-Z,]+)\")?$")
RELATION_RE = re.compile(
    r"^    (?P<from>[A-Za-z_][A-Za-z0-9_]*) \|\|--o\{ "
    r"(?P<to>[A-Za-z_][A-Za-z0-9_]*) : \"(?P<label>[A-Za-z_][A-Za-z0-9_]*)\"$"
)


def fail(path: Path, message: str) -> None:
    raise ValueError(f"{path}: {message}")


def require(condition: bool, path: Path, message: str) -> None:
    if not condition:
        fail(path, message)


def parse_mermaid(path: Path) -> tuple[dict[str, list[dict[str, str]]], list[dict[str, str]]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    require(lines and lines[0] == "erDiagram", path, "diagram must start with erDiagram")

    entities: dict[str, list[dict[str, str]]] = {}
    relations: list[dict[str, str]] = []
    current_entity: str | None = None

    for line_number, line in enumerate(lines[1:], start=2):
        if line == "":
            continue

        if current_entity is None:
            entity = ENTITY_START_RE.match(line)
            if entity:
                name = entity.group("name")
                require(name not in entities, path, f"duplicate entity {name!r} at line {line_number}")
                entities[name] = []
                current_entity = name
                continue

            relation = RELATION_RE.match(line)
            if relation:
                relations.append(relation.groupdict())
                continue

            fail(path, f"unrecognized top-level line {line_number}: {line!r}")

        if line == "    }":
            current_entity = None
            continue

        field = FIELD_RE.match(line)
        require(field is not None, path, f"unrecognized field line {line_number}: {line!r}")
        entities[current_entity].append(field.groupdict(default=""))

    require(current_entity is None, path, f"entity {current_entity!r} was not closed")
    require(entities, path, "diagram must contain at least one entity")

    for entity, fields in entities.items():
        require(fields, path, f"entity {entity!r} must contain at least one field")
        field_names = [field["name"] for field in fields]
        require(len(field_names) == len(set(field_names)), path, f"entity {entity!r} has duplicate fields")

    for relation in relations:
        require(relation["from"] in entities, path, f"relation references unknown source {relation['from']!r}")
        require(relation["to"] in entities, path, f"relation references unknown target {relation['to']!r}")
        target_fields = {field["name"] for field in entities[relation["to"]]}
        require(relation["label"] in target_fields, path, f"relation label {relation['label']!r} is not a field on {relation['to']}")

    return entities, relations


def validate_mermaid(path: Path) -> None:
    entities, relations = parse_mermaid(path)

    if "08_complex_schema" in path.parts:
        expected = {
            "game_character_Player",
            "game_character_NPC",
            "game_item_Item",
            "game_inventory_InventorySlot",
            "game_social_Guild",
        }
        require(expected.issubset(entities.keys()), path, "complex schema missing nested entities")
        require("game_common_Vec3" not in entities, path, "embed Vec3 should not be emitted as an entity")
        require("game_character_Stats" not in entities, path, "embed Stats should not be emitted as an entity")
        require(len(relations) >= 8, path, "complex schema should contain multiple foreign-key relations")

    if "07_indexes" in path.parts:
        require(relations, path, "index fixture should contain FK relations")


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: validate_mermaid.py <schema.mmd>", file=sys.stderr)
        return 2

    validate_mermaid(Path(argv[1]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
