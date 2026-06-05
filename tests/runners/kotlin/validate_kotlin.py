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
LOCAL_DECL_RE = re.compile(rf"^val\s+({IDENT_RE})\s*=")


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
            expected = f"fun fromValue(value: Int): {name} = entries.firstOrNull {{ it.value == value }}"
            require(line == expected, path, current_line_no, f"invalid fromValue helper for enum {name}")
            if idx + 1 >= len(lines):
                fail(path, current_line_no, f"missing invalid enum throw for enum {name}")
            throw_line_no, throw_line = lines[idx + 1]
            expected_throw = f'?: throw IllegalArgumentException("invalid enum discriminant for {name}: $value")'
            require(
                throw_line == expected_throw,
                path,
                throw_line_no,
                f"invalid enum throw for enum {name}",
            )
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
        if line.startswith("@Serializable"):
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


def validate_no_duplicate_simple_locals(path: Path) -> None:
    lines = split_lines(path)
    scopes: list[set[str]] = [set()]

    for line_no, line in lines:
        if not line:
            continue

        match = LOCAL_DECL_RE.match(line)
        if match is not None:
            name = match.group(1)
            require(name not in scopes[-1], path, line_no, f"duplicate local declaration {name!r} in same scope")
            scopes[-1].add(name)

        for _ in range(line.count("}")):
            if len(scopes) > 1:
                scopes.pop()
        for _ in range(line.count("{")):
            scopes.append(set())


def validate_container_file(path: Path) -> None:
    if not path.name.endswith("_container.kt"):
        return

    text = path.read_text(encoding="utf-8")
    validate_no_duplicate_simple_locals(path)
    require("class ValidationError(" in text, path, 0, "container missing ValidationError")
    require("class ValidationResult" in text, path, 0, "container missing ValidationResult")
    require("class ValidationException" in text, path, 0, "container missing ValidationException")
    require("fun validate(): ValidationResult" in text, path, 0, "container table missing validate")
    require("fun validateAll(): ValidationResult" in text, path, 0, "container missing validateAll")
    require("fun validateOrThrow()" in text, path, 0, "container missing validateOrThrow")
    require("fun clear()" in text, path, 0, "container missing clear")
    require("fun loadFromCsv(root: String)" in text, path, 0, "container missing sources CSV root load")
    require("fun loadFromJson(root: String)" in text, path, 0, "container missing sources JSON root load")
    require("private fun searchNormalize(value: String, policy: String): String" in text, path, 0, "container missing searchNormalize")
    require("private fun searchTokens(value: String, mode: String, n: Int, min: Int, normalize: String): List<String>" in text, path, 0, "container missing searchTokens")
    require("private fun <T> addGroup(index: MutableMap<Any, MutableList<T>>, key: Any, row: T)" in text, path, 0, "container missing search addGroup")
    require("private fun <T> intersectRows(postings: List<List<T>>): List<T>" in text, path, 0, "container missing search intersection")

    if "11_relations_indexes" in path.parts:
        required_fragments = {
            "class ExamplesRelationsUserTable": "user table wrapper",
            "class ExamplesRelationsPostTable": "post table wrapper",
            "class SchemaContainer": "root container",
            "val users: ExamplesRelationsUserTable": "user table member",
            "val posts: ExamplesRelationsPostTable": "post table member",
            "private val byAuthorIdStatus: MutableMap<List<Any>, MutableList<ExamplesRelationsPost>>": "composite index storage",
            "byAuthorIdStatus.getOrPut(listOf(row.author_id, row.status))": "composite index insertion",
            "fun findByAuthorIdStatus(key: List<Any>): List<ExamplesRelationsPost>": "composite index lookup",
            "fun getPostAuthor(row: ExamplesRelationsPost): ExamplesRelationsUser?": "post author navigation helper",
            "fun findUserPosts(row: ExamplesRelationsUser): List<ExamplesRelationsPost>": "reverse user posts helper",
            "return users.getById(key)": "author navigation lookup",
            "posts.findByAuthorId(row.id)": "reverse relation lookup",
            "val target = users.getById(row.author_id)": "author foreign key validation",
            'constraintType = "ForeignKey"': "foreign key error classification",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"11_relations_indexes container missing {description}")
        return

    if "08_complex_schema" in path.parts:
        required_fragments = {
            "class GameCharacterPlayerTable": "player table wrapper",
            "fun validate(): ValidationResult": "table validation method",
            'tableName = "Player"': "player validation table name",
            'fieldName = "name"': "name validation field",
            'constraintType = "MaxLength"': "max length error classification",
            'if (!Regex("^[A-Za-z0-9_ ]+$").containsMatchIn(value))': "name regex validation",
            'constraintType = "Regex"': "regex error classification",
            'message = "value ${value} does not match regex pattern"': "regex error message",
            'does not match pattern "': "bad nested regex pattern quote",
            "if (value < 1 || value > 100)": "level range validation",
            'constraintType = "Range"': "range error classification",
            "val seenId = mutableSetOf<Int>()": "primary key duplicate tracking",
            "val seenName = mutableSetOf<String>()": "unique field duplicate tracking",
            'constraintType = "Unique"': "unique error classification",
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
        "class TestIndexesUserTable": "user table wrapper",
        "class TestIndexesPostTable": "post table wrapper",
        "class SchemaContainer": "root container",
        "val users: TestIndexesUserTable": "user table member",
        "fun getByUsername(key: String): TestIndexesUser?": "unique username lookup",
        "fun getByEmail(key: String): TestIndexesUser?": "unique email lookup",
        "fun findByAuthorId(key: Int): List<TestIndexesPost>": "post author group lookup",
        "fun findByPostId(key: Int): List<TestIndexesComment>": "comment post group lookup",
        "fun loadUsersFromCsv(path: String)": "user CSV load wrapper",
        "fun loadUsersFromJson(path: String)": "user JSON load wrapper",
        "fun loadCategorysFromCsv(path: String)": "category CSV load wrapper",
        "fun loadCategorysFromJson(path: String)": "category JSON load wrapper",
        "fun loadPostTagsFromCsv(path: String)": "junction CSV load wrapper",
        "fun loadPostTagsFromJson(path: String)": "junction JSON load wrapper",
        'loadUsersFromCsv(java.io.File(root, "users.csv").path)': "sources user CSV root load",
        'loadUsersFromJson(java.io.File(root, "users.json").path)': "sources user JSON root load",
        'loadCategorysFromCsv(java.io.File(root, "categories.csv").path)': "sources category CSV root load",
        'loadCategorysFromJson(java.io.File(root, "categories.json").path)': "sources category JSON root load",
        'loadPostTagsFromCsv(java.io.File(root, "post_tags.csv").path)': "sources junction CSV root load",
        'loadPostTagsFromJson(java.io.File(root, "post_tags.json").path)': "sources junction JSON root load",
        "fun searchByName(key: String): List<TestIndexesCategory>": "category name exact search",
        "fun searchByDescription(query: String): List<TestIndexesCategory>": "category description token search",
        "fun searchByRank(key: Int): List<TestIndexesCategory>": "category rank exact search",
        "fun searchByKind(key: TestIndexesCategoryKind): List<TestIndexesCategory>": "category kind enum search",
        "fun searchByTitle(query: String): List<TestIndexesPost>": "post title token search",
        "addGroup(searchTitle, token, row)": "post title search postings",
        'searchTokens(query, "ngram"': "ngram query tokenization",
        'searchNormalize(key, "lower_trim")': "string exact query normalization",
        "fun getPostAuthor(row: TestIndexesPost): TestIndexesUser?": "post author navigation helper",
        "fun getPostCategory(row: TestIndexesPost): TestIndexesCategory?": "post category navigation helper",
        "fun getPostTagPost(row: TestIndexesPostTag): TestIndexesPost?": "post tag post navigation helper",
        "fun getPostTagTag(row: TestIndexesPostTag): TestIndexesTag?": "post tag tag navigation helper",
        "return users.getById(key)": "user navigation lookup",
        "return tags.getById(key)": "tag navigation lookup",
        "val target = users.getById(row.author_id)": "user foreign key validation",
        "val target = categorys.getById(row.category_id)": "category foreign key validation",
        'constraintType = "ForeignKey"': "foreign key error classification",
    }
    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"07_indexes container missing {description}")


def validate_pack_helpers(path: Path) -> None:
    if "10_pack_embed" not in path.parts:
        return
    if (
        path.name.endswith("_container.kt")
        or path.name.endswith("_redis_keys.kt")
        or path.name.endswith("_sqlite_accessor.kt")
        or path.name.endswith("_binary_refs.kt")
    ):
        return

    text = path.read_text(encoding="utf-8")
    required_fragments = {
        "fun TestPackEmbedPosition.pack(): String": "position pack extension",
        "fun unpackTestPackEmbedPosition(value: String): TestPackEmbedPosition": "position unpack helper",
        "fun tryUnpackTestPackEmbedPosition(value: String): TestPackEmbedPosition?": "position tryUnpack helper",
        "require(parts.size == 2) { \"expected 2 fields for TestPackEmbedPosition, got ${parts.size}\" }": "field count validation",
        "fun parsePackedFloat(raw: String, fieldName: String): Float": "float parser",
        "require(parsed.isFinite()) { \"invalid finite float for $fieldName: $raw\" }": "finite float validation",
        "fun TestPackEmbedColor.pack(): String": "color pack extension",
        "fun unpackTestPackEmbedColor(value: String): TestPackEmbedColor": "color unpack helper",
        "parsePackedInt(parts[0], \"r\", unsigned = true)": "unsigned int parser use",
        "require(!unsigned || parsed >= 0) { \"invalid unsigned integer for $fieldName: $raw\" }": "unsigned validation",
        "fun TestPackEmbedColorAlpha.pack(): String": "custom separator pack extension",
        "value.split(\"|\")": "pipe separator split",
        "fun TestPackEmbedRange.pack(): String": "signed range pack extension",
        "parsePackedInt(parts[0], \"min\")": "signed int parser use",
    }
    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"10_pack_embed missing {description}")

    require("fun TestPackEmbedStats.pack()" not in text, path, 0, "non-packed embed should not get pack API")
    require("fun unpackTestPackEmbedStats" not in text, path, 0, "non-packed embed should not get unpack API")


def validate_loader_helpers(path: Path, text: str) -> None:
    if path.name.endswith("_sqlite_accessor.kt") or path.name.endswith("_binary_refs.kt"):
        return

    require("import java.io.File" in text, path, 0, "loader missing File import")
    require("import java.io.ByteArrayOutputStream" in text, path, 0, "loader missing ByteArrayOutputStream import")
    require("import java.nio.ByteBuffer" in text, path, 0, "loader missing ByteBuffer import")
    require("import java.nio.ByteOrder" in text, path, 0, "loader missing ByteOrder import")
    require("import kotlinx.serialization.KSerializer" in text, path, 0, "loader missing KSerializer import")
    require("import kotlinx.serialization.SerializationException" in text, path, 0, "loader missing SerializationException import")
    require("import kotlinx.serialization.decodeFromString" in text, path, 0, "loader missing decodeFromString import")
    require("import kotlinx.serialization.json.Json" in text, path, 0, "loader missing Json import")
    require("import kotlinx.serialization.json.JsonDecoder" in text, path, 0, "loader missing JsonDecoder import")
    require("import kotlinx.serialization.json.JsonPrimitive" in text, path, 0, "loader missing JsonPrimitive import")
    require("import kotlinx.serialization.json.intOrNull" in text, path, 0, "loader missing intOrNull import")
    require("private val polygenJson = Json { ignoreUnknownKeys = true }" in text, path, 0, "loader missing Json instance")
    require("class PolygenBinaryReader(private val bytes: ByteArray)" in text, path, 0, "loader missing binary reader")
    require("class PolygenBinaryWriter" in text, path, 0, "loader missing binary writer")
    require("fun <T> readOptional(reader: (PolygenBinaryReader) -> T): T?" in text, path, 0, "loader missing optional binary reader")
    require("fun <T> readList(reader: (PolygenBinaryReader) -> T): List<T>" in text, path, 0, "loader missing list binary reader")
    require("fun <T> writeOptional(value: T?, writer: (PolygenBinaryWriter, T) -> Unit)" in text, path, 0, "loader missing optional binary writer")
    require("fun <T> writeList(value: List<T>, writer: (PolygenBinaryWriter, T) -> Unit)" in text, path, 0, "loader missing list binary writer")
    require("private fun polygenReadCsvRows(path: String): List<Map<String, String>>" in text, path, 0, "loader missing CSV row reader")
    require("private fun polygenSplitCsvLine(line: String): List<String>" in text, path, 0, "loader missing CSV splitter")
    require("private fun <T> polygenParseList(raw: String?, fieldName: String, parser: (String) -> T): List<T>" in text, path, 0, "loader missing list parser")
    require("private fun polygenParseBool(raw: String, fieldName: String): Boolean" in text, path, 0, "loader missing bool parser")
    require("private fun polygenParseInt(raw: String, fieldName: String, unsigned: Boolean = false): Int" in text, path, 0, "loader missing int parser")
    require("private fun polygenParseFloat(raw: String, fieldName: String): Float" in text, path, 0, "loader missing float parser")
    require("private inline fun <reified T : Enum<T>> polygenParseEnumValue" in text, path, 0, "loader missing enum parser")

    if "06_arrays_and_optionals" in path.parts:
        required_fragments = {
            "fun loadTestCollectionsArrayTestsFromJson(path: String): List<TestCollectionsArrayTest>": "array JSON loader",
            "fun loadTestCollectionsArrayTestsFromCsv(path: String): List<TestCollectionsArrayTest>": "array CSV loader",
            "int_list = polygenParseList(row[\"int_list\"], \"int_list\")": "primitive list parser use",
            "bool_list = polygenParseList(row[\"bool_list\"], \"bool_list\")": "bool list parser use",
            "tags = if (polygenIsEmpty(row[\"tags\"])) emptyList() else polygenJson.decodeFromString<List<TestCollectionsTag>>": "embed list JSON-cell parser use",
            "fun TestCollectionsArrayTest.toBinary(): ByteArray": "array binary writer entry",
            "fun TestCollectionsArrayTest.writeBinary(writer: PolygenBinaryWriter)": "array binary write extension",
            "writer.writeList(int_list) { w, v -> w.writeI32(v) }": "primitive list binary writer",
            "writer.writeList(tags) { w, v -> v.writeBinary(w) }": "embed list binary writer",
            "fun readTestCollectionsArrayTestFromBinary(bytes: ByteArray): TestCollectionsArrayTest": "array binary bytes reader",
            "int_list = reader.readList { it.readI32() }": "primitive list binary reader",
            "tags = reader.readList { readTestCollectionsTagBinary(it) }": "embed list binary reader",
            "fun loadTestCollectionsOptionalTestsFromCsv(path: String): List<TestCollectionsOptionalTest>": "optional CSV loader",
            "opt_float = if (polygenIsEmpty(row[\"opt_float\"])) null else polygenParseDouble": "optional double parser use",
            "opt_tag = if (polygenIsEmpty(row[\"opt_tag\"])) null else polygenJson.decodeFromString<TestCollectionsTag>": "optional embed parser use",
            "writer.writeOptional(opt_tag) { w, v -> v.writeBinary(w) }": "optional embed binary writer",
            "opt_tag = reader.readOptional { readTestCollectionsTagBinary(it) }": "optional embed binary reader",
            "history = if (polygenIsEmpty(row[\"history\"])) emptyList() else polygenJson.decodeFromString<List<TestCollectionsMixedTestMetadata>>": "nested embed list parser use",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"06_arrays_and_optionals missing {description}")

    if "07_indexes" in path.parts:
        required_fragments = {
            "fun loadTestIndexesCategorysFromJson(path: String): List<TestIndexesCategory>": "category JSON loader",
            "fun loadTestIndexesCategorysFromCsv(path: String): List<TestIndexesCategory>": "category CSV loader",
            "@Serializable(with = TestIndexesCategoryKindSerializer::class)": "custom enum serializer annotation",
            "object TestIndexesCategoryKindSerializer : KSerializer<TestIndexesCategoryKind>": "custom enum serializer",
            "override fun deserialize(decoder: Decoder): TestIndexesCategoryKind": "custom enum deserializer",
            "element.intOrNull?.let { return TestIndexesCategoryKind.fromValue(it) }": "numeric JSON enum parser",
            "return TestIndexesCategoryKind.fromNameOrValue(element.content)": "string JSON enum parser",
            "kind = TestIndexesCategoryKind.fromValue(polygenParseEnumValue<TestIndexesCategoryKind>": "enum name/numeric parser use",
            "rank = polygenParseInt(polygenRequired(row[\"rank\"], \"rank\"), \"rank\", unsigned = true)": "unsigned scalar parser use",
            "description = if (polygenIsEmpty(row[\"description\"])) null else polygenRequired": "optional string parser use",
            "fun TestIndexesCategory.toBinary(): ByteArray": "category binary writer entry",
            "fun TestIndexesCategory.writeBinary(writer: PolygenBinaryWriter)": "category binary write extension",
            "writer.writeOptional(description) { w, v -> w.writeString(v) }": "optional string binary writer",
            "writer.writeI32(kind.value)": "enum binary writer",
            "fun readTestIndexesCategoryFromBinary(bytes: ByteArray): TestIndexesCategory": "category binary bytes reader",
            "description = reader.readOptional { it.readString() }": "optional string binary reader",
            "kind = TestIndexesCategoryKind.fromValue(reader.readI32())": "enum binary reader",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"07_indexes missing {description}")


def validate_sqlite_accessor_file(path: Path) -> None:
    if not path.name.endswith("_sqlite_accessor.kt"):
        return

    text = path.read_text(encoding="utf-8")
    if "09_sqlite" not in path.parts:
        require("No @datasource(\"sqlite\") tables were found" in text, path, 0, "non-sqlite schema should emit empty sqlite accessor")
        return

    required_fragments = {
        "import java.sql.Connection": "Connection import",
        "import java.sql.DriverManager": "DriverManager import",
        "import java.sql.ResultSet": "ResultSet import",
        "class DbTable<T>": "DbTable wrapper",
        "class SqliteDb(private val connection: Connection)": "SqliteDb class",
        "fun connect(url: String): SqliteDb = SqliteDb(DriverManager.getConnection(url))": "connect helper",
        'val users: DbTable<TestSqliteUser> = DbTable("test_sqlite_User")': "user table member",
        'val posts: DbTable<TestSqlitePost> = DbTable("test_sqlite_Post")': "post table member",
        'val comments: DbTable<TestSqliteComment> = DbTable("test_sqlite_Comment")': "comment table member",
        'val loginEvents: DbTable<TestSqliteAuditLoginEvent> = DbTable("test_sqlite_audit_LoginEvent")': "nested login event table member",
        "fun loadAll()": "loadAll method",
        "fun loadUsers()": "user load method",
        "fun getUserById(key: Any): TestSqliteUser?": "user getById method",
        'SELECT id, name, email, created_at FROM test_sqlite_User': "user select SQL",
        "private fun mapTestSqliteUser(row: ResultSet): TestSqliteUser": "user mapper",
        "email = row.getString(\"email\")": "optional string mapping",
        "fun loadLoginEvents()": "nested load method",
        "fun getLoginEventById(key: Any): TestSqliteAuditLoginEvent?": "nested getById method",
        "private fun mapTestSqliteAuditLoginEvent(row: ResultSet): TestSqliteAuditLoginEvent": "nested mapper",
    }
    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"09_sqlite accessor missing {description}")


def validate_binary_refs_file(path: Path) -> None:
    if not path.name.endswith("_binary_refs.kt"):
        return

    text = path.read_text(encoding="utf-8")
    require("class BinaryRef<T>(" in text, path, 0, "BinaryRef file missing BinaryRef wrapper")
    require("open class BinaryRefTable<T>(" in text, path, 0, "BinaryRef file missing BinaryRefTable")
    require("class BinaryRefDocument(" in text, path, 0, "BinaryRef file missing document")
    require("fun get(): T" in text, path, 0, "BinaryRef file missing lazy get")
    require("fun count(): Int = refs.size" in text, path, 0, "BinaryRef table missing count")
    require("fun all(): List<T> = refs.map { it.get() }" in text, path, 0, "BinaryRef table missing all")
    require("fun <K> getBy(key: K, selector: (T) -> K): BinaryRef<T>?" in text, path, 0, "BinaryRef table missing unique lookup")
    require("fun <K> findBy(key: K, selector: (T) -> K): List<BinaryRef<T>>" in text, path, 0, "BinaryRef table missing group lookup")
    require("fun <K> searchExact(key: K, selector: (T) -> K?): List<BinaryRef<T>>" in text, path, 0, "BinaryRef table missing exact search")
    require("fun searchText(query: String, mode: String, n: Int, min: Int, normalize: String, selector: (T) -> String?): List<BinaryRef<T>>" in text, path, 0, "BinaryRef table missing text search")
    require("fun fromContainer(container: SchemaContainer): BinaryRefDocument" in text, path, 0, "BinaryRef document missing container export")
    require("fun fromByteArray(bytes: ByteArray): BinaryRefDocument" in text, path, 0, "BinaryRef document missing byte-array open")
    require("fun toByteArray(): ByteArray" in text, path, 0, "BinaryRef document missing byte-array save")
    require("private fun binaryRefSearchNormalize(value: String, policy: String): String" in text, path, 0, "BinaryRef file missing search normalize")
    require("private fun binaryRefSearchTokens(value: String, mode: String, n: Int, min: Int, normalize: String): List<String>" in text, path, 0, "BinaryRef file missing search tokenizer")
    require("private fun writeBinaryRefDocument(tables: List<Pair<String, List<ByteArray>>>): ByteArray" in text, path, 0, "BinaryRef file missing writer")
    require("private fun readBinaryRefDocument(bytes: ByteArray): Map<String, List<ByteArray>>" in text, path, 0, "BinaryRef file missing reader")

    if "07_indexes" in path.parts:
        required_fragments = {
            "class TestIndexesUserBinaryRefTable": "user binary ref table",
            'BinaryRefTable<TestIndexesUser>("test_indexes_User", payloads, ::readTestIndexesUserFromBinary)': "user table loader",
            "fun getById(key: Int): BinaryRef<TestIndexesUser>?": "user primary-key lookup",
            "fun getByUsername(key: String): BinaryRef<TestIndexesUser>?": "user unique lookup",
            "class TestIndexesPostBinaryRefTable": "post binary ref table",
            "fun findByAuthorId(key: Int): List<BinaryRef<TestIndexesPost>>": "post group lookup",
            "findBy(key) { row -> row.author_id }": "post group selector",
            "fun searchByName(key: String): List<BinaryRef<TestIndexesCategory>>": "category exact string search",
            'searchExact(binaryRefSearchNormalize(key, "lower_trim")) { row -> binaryRefSearchNormalize(row.name, "lower_trim") }': "category exact normalized search",
            "fun searchByDescription(query: String): List<BinaryRef<TestIndexesCategory>>": "category token search",
            'searchText(query, "ngram"': "ngram binary ref search",
            "fun searchByRank(key: Int): List<BinaryRef<TestIndexesCategory>>": "category numeric search",
            "fun searchByKind(key: TestIndexesCategoryKind): List<BinaryRef<TestIndexesCategory>>": "category enum search",
            "fun searchByTitle(query: String): List<BinaryRef<TestIndexesPost>>": "post title token search",
            "class TestIndexesPostTagBinaryRefTable": "junction binary ref table",
            "val users: TestIndexesUserBinaryRefTable": "document user member",
            "val postTags: TestIndexesPostTagBinaryRefTable": "document junction member",
            "users = TestIndexesUserBinaryRefTable.fromRows(container.users.all())": "container user export",
            "posts = TestIndexesPostBinaryRefTable(tables[\"test_indexes_Post\"] ?: emptyList())": "document post import",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"07_indexes binary refs missing {description}")

    if "11_relations_indexes" in path.parts:
        required_fragments = {
            "fun findByAuthorIdStatus(key: List<Any>): List<BinaryRef<ExamplesRelationsPost>>": "composite group lookup",
            "findBy(key) { row -> listOf(row.author_id, row.status) }": "composite key selector",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"11_relations_indexes binary refs missing {description}")


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
            validate_loader_helpers(path, text)
        validate_pack_helpers(path)
        validate_container_file(path)
        validate_sqlite_accessor_file(path)
        validate_binary_refs_file(path)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
