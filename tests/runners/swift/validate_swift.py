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
CASE_RE = re.compile(rf"^case\s+`?({IDENT_RE})`?\s*=\s*(-?\d+)\s*$")
INIT_RE = re.compile(r"^init\((.*)\)\s*\{\s*$")
LOCAL_DECL_RE = re.compile(rf"^(let|var)\s+({IDENT_RE})\s*=")


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
    has_custom_decoder: bool = False
    has_custom_encoder: bool = False


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
        if line.startswith("func ") or line.startswith("static func "):
            idx = skip_braced_block(path, lines, idx)
            continue

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


def skip_braced_block(path: Path, lines: list[tuple[int, str]], start: int) -> int:
    depth = 0
    idx = start
    while idx < len(lines):
        line_no, line = lines[idx]
        depth += line.count("{")
        depth -= line.count("}")
        idx += 1
        if depth == 0:
            return idx
    fail(path, lines[start][0], "unclosed braced block")


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
        if line.startswith("init(from decoder: Decoder) throws"):
            enum_def.has_custom_decoder = True
            idx = skip_braced_block(path, lines, idx)
            continue
        if line.startswith("func encode(to encoder: Encoder) throws"):
            enum_def.has_custom_encoder = True
            idx = skip_braced_block(path, lines, idx)
            continue

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
        if line.startswith("func ") or line.startswith("private func "):
            idx = skip_braced_block(path, lines, idx)
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

        if line.startswith("enum PolygenPackError"):
            idx = skip_braced_block(path, lines, idx)
            continue

        if line.startswith("enum PolygenLoadError"):
            idx = skip_braced_block(path, lines, idx)
            continue

        if line.startswith("enum PolygenBinaryError"):
            idx = skip_braced_block(path, lines, idx)
            continue

        if line.startswith("struct PolygenBinaryReader") or line.startswith("struct PolygenBinaryWriter"):
            idx = skip_braced_block(path, lines, idx)
            continue

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
        if not path.name.endswith("_swiftdata.swift"):
            require(enum_def.has_custom_decoder, path, enum_def.line, f"enum {enum_def.name} must include checked JSON decoder")
            require(enum_def.has_custom_encoder, path, enum_def.line, f"enum {enum_def.name} must include JSON encoder")

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


def validate_no_duplicate_simple_locals(path: Path) -> None:
    lines = split_lines(path)
    scopes: list[set[str]] = [set()]

    for line_no, line in lines:
        if not line:
            continue

        match = LOCAL_DECL_RE.match(line)
        if match is not None:
            name = match.group(2)
            require(name not in scopes[-1], path, line_no, f"duplicate local declaration {name!r} in same scope")
            scopes[-1].add(name)

        for _ in range(line.count("}")):
            if len(scopes) > 1:
                scopes.pop()
        for _ in range(line.count("{")):
            scopes.append(set())


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


def validate_container_file(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    validate_no_duplicate_simple_locals(path)
    require("import Foundation" in text, path, 0, "container must import Foundation")
    require("struct ValidationError: Hashable" in text, path, 0, "container missing ValidationError")
    require("struct ValidationResult" in text, path, 0, "container missing ValidationResult")
    require("struct ValidationException: Error" in text, path, 0, "container missing ValidationException")
    require("var count: Int" in text, path, 0, "container table missing count property")
    require("func all() -> [" in text, path, 0, "container table missing all accessor")
    require("@discardableResult" in text, path, 0, "container table missing discardable addRow")
    require("func addRow(_ row:" in text, path, 0, "container table missing addRow")
    require("func loadAll<S: Sequence>(_ sourceRows: S)" in text, path, 0, "container table missing loadAll")
    require("func validate() -> ValidationResult" in text, path, 0, "container table missing validate")
    require("func validateAll() -> ValidationResult" in text, path, 0, "container missing validateAll")
    require("func validateOrThrow() throws" in text, path, 0, "container missing validateOrThrow")
    require("func clear()" in text, path, 0, "container missing clear")
    require("func loadFromCsv(_ root: String) throws" in text, path, 0, "container missing sources config CSV root loader")
    require("func loadFromJson(_ root: String) throws" in text, path, 0, "container missing sources config JSON root loader")
    require("private func searchNormalize(_ value: String, _ policy: String) -> String" in text, path, 0, "container missing searchNormalize")
    require("private func searchTokens(_ value: String, mode: String, n: Int, min: Int, normalize: String) -> [String]" in text, path, 0, "container missing searchTokens")
    require("private func addGroup<Row>(_ index: inout [AnyHashable: [Row]], key: AnyHashable, row: Row)" in text, path, 0, "container missing search addGroup")
    require("private func intersectRows<Row: Hashable>(_ postings: [[Row]]) -> [Row]" in text, path, 0, "container missing search intersection")

    if "11_relations_indexes" in path.parts:
        required_fragments = {
            "final class ExamplesRelationsUserTable": "user table wrapper",
            "final class ExamplesRelationsPostTable": "post table wrapper",
            "final class SchemaContainer": "root container",
            "let users = ExamplesRelationsUserTable()": "user table member",
            "let posts = ExamplesRelationsPostTable()": "post table member",
            "private var byAuthorIdStatus: [[AnyHashable]: [ExamplesRelationsPost]]": "composite index storage",
            "byAuthorIdStatus[[AnyHashable(row.author_id), AnyHashable(row.status)], default: []].append(row)": "composite index insertion",
            "func findByAuthorIdStatus(_ key: [AnyHashable]) -> [ExamplesRelationsPost]": "composite index lookup",
            "func getPostAuthor(_ row: ExamplesRelationsPost) -> ExamplesRelationsUser?": "post author navigation helper",
            "func findUserPosts(_ row: ExamplesRelationsUser) -> [ExamplesRelationsPost]": "reverse user posts helper",
            "return users.getById(key)": "author navigation lookup",
            "posts.findByAuthorId(row.id)": "reverse relation lookup",
            "let target = users.getById(row.author_id)": "author foreign key validation",
            'constraintType: "ForeignKey"': "foreign key error classification",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"11_relations_indexes container missing {description}")
        return

    if "08_complex_schema" in path.parts:
        required_fragments = {
            "final class GameCharacterPlayerTable": "player table wrapper",
            "func validate() -> ValidationResult": "table validation method",
            'tableName: "Player"': "player validation table name",
            'fieldName: "name"': "name validation field",
            'constraintType: "MaxLength"': "max length error classification",
            'if value.range(of: "^[A-Za-z0-9_ ]+$", options: .regularExpression) == nil': "name regex validation",
            'constraintType: "Regex"': "regex error classification",
            'message: "value \\(value) does not match regex pattern"': "regex error message",
            'does not match pattern "': "bad nested regex pattern quote",
            "if value < 1 || value > 100": "level range validation",
            'constraintType: "Range"': "range error classification",
            "var seenGetById = Set<UInt32>()": "primary key duplicate tracking",
            "var seenGetByName = Set<String>()": "unique field duplicate tracking",
            'constraintType: "Unique"': "unique error classification",
            "result.merge(players.validate())": "root container table validation merge",
        }
        for fragment, description in required_fragments.items():
            if description == "bad nested regex pattern quote":
                require(fragment not in text, path, 0, f"08_complex_schema container contains {description}")
            else:
                require(fragment in text, path, 0, f"08_complex_schema container missing {description}")
        return

    if "07_indexes" not in path.parts:
        return

    required_fragments = {
        "final class TestIndexesUserTable": "user table wrapper",
        "final class TestIndexesPostTable": "post table wrapper",
        "final class TestIndexesCommentTable": "comment table wrapper",
        "final class TestIndexesPostTagTable": "junction table wrapper",
        "final class SchemaContainer": "root container",
        "let users = TestIndexesUserTable()": "user table member",
        "let posts = TestIndexesPostTable()": "post table member",
        "let postTags = TestIndexesPostTagTable()": "junction table member",
        "func loadUsersFromCsv(_ path: String) throws": "user CSV table loader wrapper",
        "func loadUsersFromJson(_ path: String) throws": "user JSON table loader wrapper",
        "func loadCategorysFromCsv(_ path: String) throws": "category CSV table loader wrapper",
        "func loadCategorysFromJson(_ path: String) throws": "category JSON table loader wrapper",
        "func loadPostTagsFromCsv(_ path: String) throws": "junction CSV table loader wrapper",
        "func loadPostTagsFromJson(_ path: String) throws": "junction JSON table loader wrapper",
        "try loadUsersFromCsv(URL(fileURLWithPath: root).appendingPathComponent(\"users.csv\").path)": "user sources config CSV path",
        "try loadUsersFromJson(URL(fileURLWithPath: root).appendingPathComponent(\"users.json\").path)": "user sources config JSON path",
        "try loadCategorysFromCsv(URL(fileURLWithPath: root).appendingPathComponent(\"categories.csv\").path)": "category sources config CSV path",
        "try loadCategorysFromJson(URL(fileURLWithPath: root).appendingPathComponent(\"categories.json\").path)": "category sources config JSON path",
        "try loadPostTagsFromCsv(URL(fileURLWithPath: root).appendingPathComponent(\"post_tags.csv\").path)": "junction sources config CSV path",
        "try loadPostTagsFromJson(URL(fileURLWithPath: root).appendingPathComponent(\"post_tags.json\").path)": "junction sources config JSON path",
        "func loadAll<S: Sequence>(_ sourceRows: S) where S.Element == TestIndexesUser": "user loadAll",
        "func addRow(_ row: TestIndexesPostTag) -> TestIndexesPostTag": "junction addRow",
        "func getByUsername(_ key: String) -> TestIndexesUser?": "unique username lookup",
        "func getByEmail(_ key: String) -> TestIndexesUser?": "unique email lookup",
        "func findByAuthorId(_ key: UInt32) -> [TestIndexesPost]": "post author group lookup",
        "func findByPostId(_ key: UInt32) -> [TestIndexesComment]": "comment post group lookup",
        "func findByTagId(_ key: UInt32) -> [TestIndexesPostTag]": "junction tag group lookup",
        "func searchByName(_ key: String) -> [TestIndexesCategory]": "category name exact search",
        "func searchByDescription(_ query: String) -> [TestIndexesCategory]": "category description token search",
        "func searchByRank(_ key: UInt8) -> [TestIndexesCategory]": "category rank exact search",
        "func searchByKind(_ key: TestIndexesCategoryKind) -> [TestIndexesCategory]": "category kind enum search",
        "func searchByTitle(_ query: String) -> [TestIndexesPost]": "post title token search",
        "addGroup(&searchTitle, key: AnyHashable(token), row: row)": "post title search postings",
        'searchTokens(query, mode: "ngram"': "ngram query tokenization",
        'searchNormalize(key, "lower_trim")': "string exact query normalization",
        "func getPostAuthor(_ row: TestIndexesPost) -> TestIndexesUser?": "post author navigation helper",
        "func getPostCategory(_ row: TestIndexesPost) -> TestIndexesCategory?": "post category navigation helper",
        "func getPostTagPost(_ row: TestIndexesPostTag) -> TestIndexesPost?": "post tag post navigation helper",
        "func getPostTagTag(_ row: TestIndexesPostTag) -> TestIndexesTag?": "post tag tag navigation helper",
        "return users.getById(key)": "user navigation lookup",
        "return tags.getById(key)": "tag navigation lookup",
        "let target = users.getById(row.author_id)": "user foreign key validation",
        "let target = categorys.getById(row.category_id)": "category foreign key validation",
        "let target = posts.getById(row.post_id)": "post foreign key validation",
        "let target = tags.getById(row.tag_id)": "tag foreign key validation",
        'constraintType: "ForeignKey"': "foreign key error classification",
    }
    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"07_indexes container missing {description}")


def validate_pack_helpers(path: Path) -> None:
    if "10_pack_embed" not in path.parts:
        return
    if (
        path.name.endswith("_container.swift")
        or path.name.endswith("_redis_keys.swift")
        or path.name.endswith("_swiftdata.swift")
        or path.name.endswith("_sqlite_accessor.swift")
        or path.name.endswith("_binary_refs.swift")
    ):
        return

    text = path.read_text(encoding="utf-8")
    required_fragments = {
        "enum PolygenPackError: Error, Equatable": "pack error type",
        "case invalidFieldCount(type: String, expected: Int, actual: Int)": "field count error",
        "case invalidValue(field: String, value: String)": "invalid value error",
        "func pack() -> String": "pack method",
        "static func unpack(_ value: String) throws -> TestPackEmbedPosition": "position unpack method",
        "static func tryUnpack(_ value: String) -> TestPackEmbedPosition?": "position tryUnpack method",
        "throw PolygenPackError.invalidFieldCount(type: \"TestPackEmbedPosition\", expected: 2, actual: parts.count)": "field count validation",
        "func parsePackedFloating<T: BinaryFloatingPoint & LosslessStringConvertible>": "floating parser",
        "parsed.isFinite": "finite float validation",
        "static func unpack(_ value: String) throws -> TestPackEmbedColor": "color unpack method",
        "try parsePackedInteger(parts[0], fieldName: \"r\", as: UInt8.self)": "unsigned int parser use",
        "static func unpack(_ value: String) throws -> TestPackEmbedColorAlpha": "custom separator unpack method",
        "Character(\"|\")": "pipe separator split",
        "static func unpack(_ value: String) throws -> TestPackEmbedRange": "signed range unpack method",
        "try parsePackedInteger(parts[0], fieldName: \"min\", as: Int32.self)": "signed int parser use",
        "try? unpack(value)": "tryUnpack implementation",
    }
    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"10_pack_embed missing {description}")

    require("extension TestPackEmbedStats" not in text, path, 0, "non-packed embed should not get extension pack API")
    stats_start = text.find("struct TestPackEmbedStats: Codable, Hashable")
    game_object_start = text.find("struct TestPackEmbedGameObject: Codable, Hashable")
    if stats_start != -1 and game_object_start != -1:
        stats_body = text[stats_start:game_object_start]
        require("func pack() -> String" not in stats_body, path, 0, "non-packed embed should not get pack method")
        require("static func unpack" not in stats_body, path, 0, "non-packed embed should not get unpack method")


def validate_loader_helpers(path: Path, text: str) -> None:
    if (
        path.name.endswith("_container.swift")
        or path.name.endswith("_redis_keys.swift")
        or path.name.endswith("_swiftdata.swift")
        or path.name.endswith("_sqlite_accessor.swift")
        or path.name.endswith("_binary_refs.swift")
    ):
        return

    required_helpers = {
        "enum PolygenLoadError: Error, Equatable": "load error type",
        "case missingRequiredField(String)": "missing required field error",
        "case invalidValue(field: String, value: String)": "invalid value error",
        "enum PolygenBinaryError: Error, Equatable": "binary error type",
        "case invalidEnumDiscriminant(Int)": "invalid enum discriminant error",
        "struct PolygenBinaryReader": "binary reader",
        "struct PolygenBinaryWriter": "binary writer",
        "mutating func readEnum<T>(_ type: T.Type) throws -> T where T: RawRepresentable, T.RawValue == Int": "checked enum binary reader",
        "throw PolygenBinaryError.invalidEnumDiscriminant(rawValue)": "invalid enum discriminant throw",
        "mutating func readOptional<T>(_ reader: (inout PolygenBinaryReader) throws -> T) throws -> T?": "optional binary reader",
        "mutating func readList<T>(_ reader: (inout PolygenBinaryReader) throws -> T) throws -> [T]": "list binary reader",
        "mutating func writeOptional<T>(_ value: T?, writer: (inout PolygenBinaryWriter, T) -> Void)": "optional binary writer",
        "mutating func writeList<T>(_ value: [T], writer: (inout PolygenBinaryWriter, T) -> Void)": "list binary writer",
        "private func polygenJsonDecoder() -> JSONDecoder": "JSON decoder helper",
        "private func polygenReadCsvRows(_ path: String) throws -> [[String: String]]": "CSV row reader",
        "private func polygenSplitCsvLine(_ line: String) -> [String]": "CSV splitter",
        "private func polygenParseList<T>(_ raw: String?, fieldName: String, parser: (String) throws -> T) throws -> [T]": "list parser",
        "private func polygenParseBool(_ raw: String, fieldName: String) throws -> Bool": "bool parser",
        "private func polygenParseInteger<T: FixedWidthInteger>(_ raw: String, fieldName: String, as type: T.Type) throws -> T": "integer parser",
        "private func polygenParseFloating<T: BinaryFloatingPoint & LosslessStringConvertible>(_ raw: String, fieldName: String, as type: T.Type) throws -> T": "floating parser",
        "private func polygenParseEnum<T: RawRepresentable & CaseIterable>(_ raw: String, fieldName: String, as type: T.Type) throws -> T where T.RawValue == Int": "enum parser",
    }
    for fragment, description in required_helpers.items():
        require(fragment in text, path, 0, f"loader missing {description}")

    if "06_arrays_and_optionals" in path.parts:
        required_fragments = {
            "func loadTestCollectionsArrayTestsFromJson(_ path: String) throws -> [TestCollectionsArrayTest]": "array JSON loader",
            "func loadTestCollectionsArrayTestsFromCsv(_ path: String) throws -> [TestCollectionsArrayTest]": "array CSV loader",
            'int_list: try polygenParseList(row["int_list"], fieldName: "int_list")': "primitive list parser use",
            'bool_list: try polygenParseList(row["bool_list"], fieldName: "bool_list")': "bool list parser use",
            'tags: polygenIsEmpty(row["tags"]) ? [] : try polygenJsonDecoder().decode([TestCollectionsTag].self': "embed list JSON-cell parser use",
            "func toBinary() -> Data": "binary writer entry",
            "func writeBinary(to writer: inout PolygenBinaryWriter)": "binary write method",
            "writer.writeList(int_list) { writer, value in writer.writeInt32(value) }": "primitive list binary writer",
            "writer.writeList(tags) { writer, value in value.writeBinary(to: &writer) }": "embed list binary writer",
            "static func fromBinary(_ data: Data) throws -> TestCollectionsArrayTest": "array binary bytes reader",
            "int_list: try reader.readList { reader in try reader.readInt32() }": "primitive list binary reader",
            "tags: try reader.readList { reader in try TestCollectionsTag.readBinary(from: &reader) }": "embed list binary reader",
            "func loadTestCollectionsOptionalTestsFromCsv(_ path: String) throws -> [TestCollectionsOptionalTest]": "optional CSV loader",
            'opt_float: polygenIsEmpty(row["opt_float"]) ? nil : try polygenParseFloating': "optional double parser use",
            'opt_tag: polygenIsEmpty(row["opt_tag"]) ? nil : try polygenJsonDecoder().decode(TestCollectionsTag.self': "optional embed parser use",
            "writer.writeOptional(opt_tag) { writer, value in value.writeBinary(to: &writer) }": "optional embed binary writer",
            "opt_tag: try reader.readOptional { reader in try TestCollectionsTag.readBinary(from: &reader) }": "optional embed binary reader",
            'history: polygenIsEmpty(row["history"]) ? [] : try polygenJsonDecoder().decode([TestCollectionsMixedTestMetadata].self': "nested embed list parser use",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"06_arrays_and_optionals missing {description}")

    if "07_indexes" in path.parts:
        required_fragments = {
            "func loadTestIndexesCategorysFromJson(_ path: String) throws -> [TestIndexesCategory]": "category JSON loader",
            "func loadTestIndexesCategorysFromCsv(_ path: String) throws -> [TestIndexesCategory]": "category CSV loader",
            "init(from decoder: Decoder) throws": "custom enum JSON decoder",
            "if let rawValue = try? container.decode(Int.self), let value = TestIndexesCategoryKind(rawValue: rawValue)": "numeric JSON enum parser",
            "if let rawValue = Int(raw), let value = TestIndexesCategoryKind(rawValue: rawValue)": "string numeric JSON enum parser",
            "if let value = TestIndexesCategoryKind.allCases.first(where: { String(describing: $0) == raw })": "string name JSON enum parser",
            "func encode(to encoder: Encoder) throws": "custom enum JSON encoder",
            'kind: try polygenParseEnum(try polygenRequired(row["kind"], fieldName: "kind"), fieldName: "kind", as: TestIndexesCategoryKind.self)': "enum name/numeric parser use",
            'rank: try polygenParseInteger(try polygenRequired(row["rank"], fieldName: "rank"), fieldName: "rank", as: UInt8.self)': "unsigned scalar parser use",
            'description: polygenIsEmpty(row["description"]) ? nil : try polygenRequired': "optional string parser use",
            "static func fromBinary(_ data: Data) throws -> TestIndexesCategory": "category binary bytes reader",
            "writer.writeOptional(description) { writer, value in writer.writeString(value) }": "optional string binary writer",
            "writer.writeInt32(Int32(kind.rawValue))": "enum binary writer",
            "description: try reader.readOptional { reader in try reader.readString() }": "optional string binary reader",
            "kind: try reader.readEnum(TestIndexesCategoryKind.self)": "enum binary reader",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"07_indexes missing {description}")


def validate_sqlite_accessor_file(path: Path) -> None:
    if not path.name.endswith("_sqlite_accessor.swift"):
        return

    text = path.read_text(encoding="utf-8")
    require("import Foundation" in text, path, 0, "SQLite accessor must import Foundation")

    if "09_sqlite" not in path.parts:
        require(
            'No @datasource("sqlite") tables were found' in text,
            path,
            0,
            "non-SQLite schemas should generate an empty SQLite accessor marker",
        )
        return

    required_fragments = {
        "protocol PolygenSQLiteRow": "row protocol",
        "protocol PolygenSQLiteConnection": "connection protocol",
        "func query(_ sql: String, parameters: [Any]) throws -> [any PolygenSQLiteRow]": "query API",
        "enum PolygenSqliteError: Error, Equatable": "SQLite error type",
        "final class DbTable<T>": "table wrapper",
        "final class SqliteDb": "database wrapper",
        "static func connect(_ connection: any PolygenSQLiteConnection) -> SqliteDb": "connect helper",
        "func close() throws": "close helper",
        "let users: DbTable<TestSqliteUser> = DbTable(\"test_sqlite_User\")": "user table member",
        "let posts: DbTable<TestSqlitePost> = DbTable(\"test_sqlite_Post\")": "post table member",
        "let comments: DbTable<TestSqliteComment> = DbTable(\"test_sqlite_Comment\")": "comment table member",
        "let loginEvents: DbTable<TestSqliteAuditLoginEvent> = DbTable(\"test_sqlite_audit_LoginEvent\")": "nested table member",
        "func loadAll() throws": "load all method",
        "func loadUsers() throws": "user load method",
        'connection.query("SELECT id, name, email, created_at FROM test_sqlite_User", parameters: [])': "user SELECT query",
        "users.loadAll(try rows.map(mapTestSqliteUser))": "user table load",
        "func getUserById(_ key: Any) throws -> TestSqliteUser?": "primary-key getter",
        'connection.query("SELECT id, name, email, created_at FROM test_sqlite_User WHERE id = ?", parameters: [key])': "primary-key SELECT query",
        "private func polygenSqliteOptional<T>(_ value: Any?, _ mapper: (Any?) throws -> T) throws -> T?": "optional mapper",
        "private func polygenSqliteInteger<T: FixedWidthInteger>": "integer mapper",
        "private func polygenSqliteString(_ value: Any?, column: String) throws -> String": "string mapper",
        "private func mapTestSqliteUser(_ row: any PolygenSQLiteRow) throws -> TestSqliteUser": "user row mapper",
        'email: try polygenSqliteOptional(row.value("email")) { try polygenSqliteString($0, column: "email") }': "optional string mapping",
        "private func mapTestSqliteAuditLoginEvent(_ row: any PolygenSQLiteRow) throws -> TestSqliteAuditLoginEvent": "nested row mapper",
        "func loadLoginEvents() throws": "nested table load method",
        "func getLoginEventById(_ key: Any) throws -> TestSqliteAuditLoginEvent?": "nested primary-key getter",
    }
    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"09_sqlite SQLite accessor missing {description}")


def validate_binary_refs_file(path: Path) -> None:
    if not path.name.endswith("_binary_refs.swift"):
        return

    text = path.read_text(encoding="utf-8")
    require("import Foundation" in text, path, 0, "BinaryRef file must import Foundation")
    require("final class BinaryRef<T>" in text, path, 0, "BinaryRef file missing ref wrapper")
    require("class BinaryRefTable<T>" in text, path, 0, "BinaryRef file missing table wrapper")
    require("final class BinaryRefDocument" in text, path, 0, "BinaryRef file missing document")
    require("func get() throws -> T" in text, path, 0, "BinaryRef file missing lazy get")
    require("func count() -> Int" in text, path, 0, "BinaryRef table missing count")
    require("func all() throws -> [T]" in text, path, 0, "BinaryRef table missing all")
    require("func getBy<K: Equatable>(_ key: K, selector: (T) -> K) throws -> BinaryRef<T>?" in text, path, 0, "BinaryRef table missing unique lookup")
    require("func findBy<K: Equatable>(_ key: K, selector: (T) -> K) throws -> [BinaryRef<T>]" in text, path, 0, "BinaryRef table missing group lookup")
    require("func searchExact<K: Equatable>(_ key: K, selector: (T) -> K?) throws -> [BinaryRef<T>]" in text, path, 0, "BinaryRef table missing exact search lookup")
    require("func searchText(_ query: String, mode: String, n: Int, min: Int, normalize: String, selector: (T) -> String?) throws -> [BinaryRef<T>]" in text, path, 0, "BinaryRef table missing text search lookup")
    require("static func fromContainer(_ container: SchemaContainer) throws -> BinaryRefDocument" in text, path, 0, "BinaryRef document missing container export")
    require("static func fromData(_ data: Data) throws -> BinaryRefDocument" in text, path, 0, "BinaryRef document missing data import")
    require("func toData() -> Data" in text, path, 0, "BinaryRef document missing data export")
    require("private func binaryRefSearchNormalize(_ value: String, _ policy: String) -> String" in text, path, 0, "BinaryRef file missing search normalizer")
    require("private func binaryRefSearchTokens(_ value: String, mode: String, n: Int, min: Int, normalize: String) -> [String]" in text, path, 0, "BinaryRef file missing search tokenizer")
    require("private func writeBinaryRefDocument(_ tables: [(String, [Data])]) -> Data" in text, path, 0, "BinaryRef file missing writer")
    require("private func readBinaryRefDocument(_ data: Data) throws -> [String: [Data]]" in text, path, 0, "BinaryRef file missing reader")

    if "07_indexes" in path.parts:
        required_fragments = {
            "final class TestIndexesUserBinaryRefTable": "user binary ref table",
            'super.init(tableName: "test_indexes_User", payloads: payloads, loader: TestIndexesUser.fromBinary)': "user table loader",
            "func getById(_ key: UInt32) throws -> BinaryRef<TestIndexesUser>?": "user primary-key lookup",
            "func getByUsername(_ key: String) throws -> BinaryRef<TestIndexesUser>?": "user unique lookup",
            "final class TestIndexesPostBinaryRefTable": "post binary ref table",
            "func findByAuthorId(_ key: UInt32) throws -> [BinaryRef<TestIndexesPost>]": "post group lookup",
            "try findBy(key) { row in row.author_id }": "post group selector",
            "func searchByName(_ key: String) throws -> [BinaryRef<TestIndexesCategory>]": "category name exact search",
            'try searchExact(binaryRefSearchNormalize(key, "lower_trim")) { row in binaryRefSearchNormalize(row.name, "lower_trim") }': "category name normalized search selector",
            "func searchByDescription(_ query: String) throws -> [BinaryRef<TestIndexesCategory>]": "category description token search",
            'try searchText(query, mode: "ngram"': "category description ngram search",
            "func searchByRank(_ key: UInt8) throws -> [BinaryRef<TestIndexesCategory>]": "category rank exact search",
            "func searchByKind(_ key: TestIndexesCategoryKind) throws -> [BinaryRef<TestIndexesCategory>]": "category enum exact search",
            "func searchByTitle(_ query: String) throws -> [BinaryRef<TestIndexesPost>]": "post title token search",
            "final class TestIndexesPostTagBinaryRefTable": "junction binary ref table",
            "let users: TestIndexesUserBinaryRefTable": "document user member",
            "let postTags: TestIndexesPostTagBinaryRefTable": "document junction member",
            "users: TestIndexesUserBinaryRefTable.fromRows(container.users.all())": "container user export",
            'posts: TestIndexesPostBinaryRefTable(payloads: tables["test_indexes_Post"] ?? [])': "document post import",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"07_indexes binary refs missing {description}")

    if "11_relations_indexes" in path.parts:
        required_fragments = {
            "func findByAuthorIdStatus(_ key: [AnyHashable]) throws -> [BinaryRef<ExamplesRelationsPost>]": "composite group lookup",
            "try findBy(key) { row in [AnyHashable(row.author_id), AnyHashable(row.status)] }": "composite key selector",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"11_relations_indexes binary refs missing {description}")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: validate_swift.py <swift> [...]", file=sys.stderr)
        return 2

    for arg in argv[1:]:
        path = Path(arg)
        if path.name.endswith("_container.swift"):
            validate_container_file(path)
            continue
        if path.name.endswith("_sqlite_accessor.swift"):
            validate_sqlite_accessor_file(path)
            continue
        if path.name.endswith("_binary_refs.swift"):
            validate_binary_refs_file(path)
            continue

        structs, enums, models = parse_swift(path)
        text = path.read_text(encoding="utf-8")
        if structs or enums or models:
            require("import Foundation" in text, path, 0, "missing Foundation import")
        if models:
            require("import SwiftData" in text, path, 0, "SwiftData model file must import SwiftData")
        validate_loader_helpers(path, text)
        validate_pack_helpers(path)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
