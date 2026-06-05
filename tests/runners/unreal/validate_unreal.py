#!/usr/bin/env python3
"""Validate generated PolyGen Unreal headers without requiring UnrealBuildTool."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


IDENT_RE = r"[A-Za-z_][A-Za-z0-9_]*"
ENUM_RE = re.compile(rf"^enum\s+class\s+(E{IDENT_RE})\s*:\s*uint8\s*$")
ENUM_CASE_RE = re.compile(rf"^({IDENT_RE})(?:\s*=\s*(-?\d+))?\s+UMETA\(DisplayName\s*=\s*\"[^\"]+\"\),\s*$")
STRUCT_RE = re.compile(rf"^struct\s+(F{IDENT_RE})\s*$")
FIELD_RE = re.compile(rf"^(.+?)\s+({IDENT_RE})(?:\s*=\s*.+)?;\s*$")
JSON_LOADER_RE = re.compile(rf"^inline\s+bool\s+LoadFromJson\(const\s+FString&\s+JsonString,\s+(F{IDENT_RE})&\s+OutStruct\)\s*$")
JSON_ARRAY_RE = re.compile(rf"^inline\s+bool\s+LoadArrayFromJson\(const\s+FString&\s+JsonString,\s+TArray<(F{IDENT_RE})>&\s+OutArray\)\s*$")
CSV_LOADER_RE = re.compile(rf"^inline\s+bool\s+LoadFromCsv\(const\s+FString&\s+CsvPath,\s+TArray<(F{IDENT_RE})>&\s+OutArray\)\s*$")
CSV_WRITER_RE = re.compile(rf"^inline\s+bool\s+SaveToCsv\(const\s+FString&\s+CsvPath,\s+const\s+TArray<(F{IDENT_RE})>&\s+InArray\)\s*$")
DELEGATE_RE = re.compile(rf"^DECLARE_MULTICAST_DELEGATE_OneParam\(FOn({IDENT_RE})DataReloaded,\s+const\s+TArray<(F{IDENT_RE})>&\);\s*$")
LOCAL_INCLUDE_RE = re.compile(r'^#include\s+"(Polygen[^"]+\.h)"\s*$')


@dataclass
class UnrealEnum:
    name: str
    line: int
    cases: list[tuple[str, int | None, int]] = field(default_factory=list)


@dataclass
class UnrealField:
    type_name: str
    name: str
    line: int
    has_uproperty: bool


@dataclass
class UnrealStruct:
    name: str
    line: int
    fields: list[UnrealField] = field(default_factory=list)
    has_generated_body: bool = False
    has_constructor: bool = False


def fail(path: Path, line: int, message: str) -> None:
    location = f"{path}:{line}" if line else str(path)
    raise ValueError(f"{location}: {message}")


def require(condition: bool, path: Path, line: int, message: str) -> None:
    if not condition:
        fail(path, line, message)


def strip(line: str) -> str:
    return line.strip()


def split_lines(path: Path) -> list[tuple[int, str]]:
    return [(line_no, strip(line)) for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1)]


def validate_generated_include_last(path: Path, text: str, generated_header: str) -> None:
    include_lines = [line.strip() for line in text.splitlines() if line.strip().startswith("#include ")]
    require(include_lines, path, 0, "header contains no includes")
    require(
        include_lines[-1] == f'#include "{generated_header}"',
        path,
        0,
        f"{generated_header} must be the last #include",
    )


def validate_local_includes_exist(path: Path, text: str) -> None:
    for line_no, line in split_lines(path):
        match = LOCAL_INCLUDE_RE.match(line)
        if match is None:
            continue
        include_name = match.group(1)
        if include_name.endswith(".generated.h"):
            continue
        require(
            (path.parent / include_name).is_file(),
            path,
            line_no,
            f"local include {include_name} does not exist next to generated Unreal headers",
        )


def validate_header(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    require("#pragma once" in text, path, 0, "missing #pragma once")
    validate_local_includes_exist(path, text)

    if path.name.endswith("Loaders.h"):
        validate_loaders(path)
    elif path.name.endswith("Registry.h"):
        validate_registry(path)
    elif path.name == "PolygenHotReload.h":
        validate_hot_reload(path)
    elif path.name.endswith("RedisKeys.h"):
        validate_redis_keys(path)
    else:
        validate_model_header(path)


def validate_model_header(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    require('#include "CoreMinimal.h"' in text, path, 0, "missing CoreMinimal include")
    expected_generated = path.stem + ".generated.h"
    require(f'#include "{expected_generated}"' in text, path, 0, f"missing generated include {expected_generated}")
    validate_generated_include_last(path, text, expected_generated)

    lines = split_lines(path)
    enums: list[UnrealEnum] = []
    structs: list[UnrealStruct] = []
    seen_types: set[str] = set()
    idx = 0

    while idx < len(lines):
        line_no, line = lines[idx]
        if line == "UENUM(BlueprintType)":
            require(idx + 1 < len(lines), path, line_no, "UENUM missing enum declaration")
            enum_line_no, enum_line = lines[idx + 1]
            match = ENUM_RE.match(enum_line)
            require(match is not None, path, enum_line_no, f"invalid enum declaration {enum_line!r}")
            enum_def, idx = parse_enum(path, lines, idx + 1, match.group(1), enum_line_no)
            require(enum_def.name not in seen_types, path, enum_def.line, f"duplicate type {enum_def.name}")
            seen_types.add(enum_def.name)
            enums.append(enum_def)
            continue

        if line == "USTRUCT(BlueprintType)":
            require(idx + 1 < len(lines), path, line_no, "USTRUCT missing struct declaration")
            struct_line_no, struct_line = lines[idx + 1]
            match = STRUCT_RE.match(struct_line)
            require(match is not None, path, struct_line_no, f"invalid struct declaration {struct_line!r}")
            struct_def, idx = parse_struct(path, lines, idx + 1, match.group(1), struct_line_no)
            require(struct_def.name not in seen_types, path, struct_def.line, f"duplicate type {struct_def.name}")
            seen_types.add(struct_def.name)
            structs.append(struct_def)
            continue

        idx += 1

    require(enums or structs, path, 0, "model header contains no UENUM or USTRUCT blocks")
    for enum_def in enums:
        require(enum_def.cases, path, enum_def.line, f"enum {enum_def.name} contains no cases")
        seen_names: set[str] = set()
        seen_values: set[int] = set()
        for case_name, value, case_line in enum_def.cases:
            require(case_name not in seen_names, path, case_line, f"duplicate enum case {case_name}")
            if value is not None:
                require(value not in seen_values, path, case_line, f"duplicate enum value {value}")
                seen_values.add(value)
            seen_names.add(case_name)

    for struct_def in structs:
        require(struct_def.has_generated_body, path, struct_def.line, f"struct {struct_def.name} missing GENERATED_BODY")
        require(struct_def.has_constructor, path, struct_def.line, f"struct {struct_def.name} missing default constructor")
        require(struct_def.fields, path, struct_def.line, f"struct {struct_def.name} contains no fields")
        seen_fields: set[str] = set()
        for field_def in struct_def.fields:
            require(field_def.has_uproperty, path, field_def.line, f"field {field_def.name} missing UPROPERTY")
            require(field_def.name not in seen_fields, path, field_def.line, f"duplicate field {field_def.name}")
            require_valid_unreal_type(path, field_def.line, field_def.type_name)
            seen_fields.add(field_def.name)

    if "10_pack_embed" in path.parts:
        required_fragments = {
            "FString Pack() const": "pack method",
            "static bool TryUnpack(const FString& Str, FPolygenPosition& OutValue)": "position try unpack method",
            "static FPolygenPosition Unpack(const FString& Str)": "position unpack method",
            "if (Parts.Num() != 2)": "field count validation",
            "if (!LexTryParseString(Parsed.x, *Parts[0]) || !FMath::IsFinite(Parsed.x))": "finite float parse validation",
            "TryUnpack(Str, Result);": "unpack delegates to try unpack",
            "static bool TryUnpack(const FString& Str, FPolygenColorAlpha& OutValue)": "custom separator try unpack method",
            "Str.ParseIntoArray(Parts, TEXT(\"|\"), false);": "pipe separator parser",
            "if (Parsed.width < 0)": "unsigned dimension negative guard",
            "if (!LexTryParseString(Parsed.min, *Parts[0]))": "signed range parse validation",
        }
        for fragment, description in required_fragments.items():
            require(fragment in text, path, 0, f"10_pack_embed missing {description}")

        stats_start = text.find("struct FPolygenStats")
        game_object_start = text.find("struct FPolygenGameObject")
        if stats_start != -1 and game_object_start != -1:
            stats_body = text[stats_start:game_object_start]
            require("FString Pack() const" not in stats_body, path, 0, "non-packed embed should not get pack method")
            require("TryUnpack" not in stats_body, path, 0, "non-packed embed should not get try unpack method")


def parse_enum(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int) -> tuple[UnrealEnum, int]:
    enum_def = UnrealEnum(name=name, line=line_no)
    idx = start + 1
    require(idx < len(lines) and lines[idx][1] == "{", path, line_no, f"enum {name} missing opening brace")
    idx += 1

    while idx < len(lines):
        current_line_no, line = lines[idx]
        if not line or line.startswith("//"):
            idx += 1
            continue
        if line == "};":
            return enum_def, idx + 1
        match = ENUM_CASE_RE.match(line)
        require(match is not None, path, current_line_no, f"invalid enum case {line!r}")
        case_name, value = match.groups()
        enum_def.cases.append((case_name, int(value) if value is not None else None, current_line_no))
        idx += 1

    fail(path, line_no, f"unclosed enum {name}")


def parse_struct(path: Path, lines: list[tuple[int, str]], start: int, name: str, line_no: int) -> tuple[UnrealStruct, int]:
    struct_def = UnrealStruct(name=name, line=line_no)
    idx = start + 1
    require(idx < len(lines) and lines[idx][1] == "{", path, line_no, f"struct {name} missing opening brace")
    idx += 1
    pending_uproperty = False

    while idx < len(lines):
        current_line_no, line = lines[idx]
        if not line or line.startswith("//") or line.startswith("/**"):
            idx += 1
            continue
        if line == "GENERATED_BODY()":
            struct_def.has_generated_body = True
            idx += 1
            continue
        if line.startswith("UPROPERTY("):
            require("Category = \"Data\"" in line, path, current_line_no, "UPROPERTY missing Data category")
            pending_uproperty = True
            idx += 1
            continue
        if line == f"{name}() = default;":
            struct_def.has_constructor = True
            idx += 1
            continue
        if line in (
            "FString Pack() const",
            f"static bool TryUnpack(const FString& Str, {name}& OutValue)",
            f"static {name} Unpack(const FString& Str)",
        ):
            idx = skip_function(path, lines, idx)
            continue
        if line == "};":
            return struct_def, idx + 1

        match = FIELD_RE.match(line)
        require(match is not None, path, current_line_no, f"unexpected struct statement {line!r}")
        type_name, field_name = match.groups()
        struct_def.fields.append(UnrealField(type_name.strip(), field_name, current_line_no, pending_uproperty))
        pending_uproperty = False
        idx += 1

    fail(path, line_no, f"unclosed struct {name}")


def skip_function(path: Path, lines: list[tuple[int, str]], start: int) -> int:
    idx = start + 1
    require(idx < len(lines) and lines[idx][1] == "{", path, lines[start][0], "function missing opening brace")
    depth = 0
    while idx < len(lines):
        line = lines[idx][1]
        depth += line.count("{")
        depth -= line.count("}")
        idx += 1
        if depth == 0:
            return idx
    fail(path, lines[start][0], "unclosed function")


def require_valid_unreal_type(path: Path, line: int, type_name: str) -> None:
    normalized = type_name.replace(" ", "")
    primitive = {"FString", "bool", "uint8", "int32", "int64", "float", "double", "TArray<uint8>"}
    if normalized.startswith("TArray<") and normalized.endswith(">"):
        inner = normalized[len("TArray<") : -1]
        require_valid_unreal_type(path, line, inner)
        return
    require(
        normalized in primitive or re.match(r"^[FE][A-Za-z_][A-Za-z0-9_]*$", normalized) is not None,
        path,
        line,
        f"unknown Unreal type {type_name!r}",
    )


def validate_loaders(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    require("namespace PolygenLoaders" in text, path, 0, "missing PolygenLoaders namespace")
    require("#include \"JsonObjectConverter.h\"" in text, path, 0, "missing JsonObjectConverter include")
    require("LINE_TERMINATOR" in text, path, 0, "missing LINE_TERMINATOR guard")

    lines = split_lines(path)
    json_single: set[str] = set()
    json_array: set[str] = set()
    csv_loaders: set[str] = set()
    csv_writers: set[str] = set()
    for line_no, line in lines:
        for regex, target in (
            (JSON_LOADER_RE, json_single),
            (JSON_ARRAY_RE, json_array),
            (CSV_LOADER_RE, csv_loaders),
            (CSV_WRITER_RE, csv_writers),
        ):
            match = regex.match(line)
            if match is not None:
                struct_name = match.group(1)
                require(struct_name not in target, path, line_no, f"duplicate loader for {struct_name}")
                target.add(struct_name)

    require(json_single, path, 0, "no JSON loaders generated")
    require(json_single == json_array, path, 0, "JSON loader and array loader structs differ")
    require(csv_loaders, path, 0, "no CSV loaders generated")
    require(csv_loaders == csv_writers, path, 0, "CSV loader and writer structs differ")


def validate_registry(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    require('#include "CoreMinimal.h"' in text, path, 0, "registry missing CoreMinimal include")
    require(
        '#include "UObject/Object.h"' in text or '#include "UObject/NoExportTypes.h"' in text,
        path,
        0,
        "registry missing UObject include",
    )
    require(path.stem + ".generated.h" in text, path, 0, "registry missing generated include")
    validate_generated_include_last(path, text, path.stem + ".generated.h")
    require("USTRUCT(BlueprintType)" in text, path, 0, "registry missing validation USTRUCT")
    require("UCLASS(BlueprintType)" in text, path, 0, "registry missing BlueprintType UCLASS")
    require("GENERATED_BODY()" in text, path, 0, "registry missing GENERATED_BODY")
    require("void Clear()" in text, path, 0, "registry missing Clear")
    require("void RebuildIndexes()" in text, path, 0, "registry missing RebuildIndexes")
    require("ValidateFieldConstraints" in text, path, 0, "registry missing field constraint validation")
    require("ValidateUniqueKeys" in text, path, 0, "registry missing unique key validation")
    require("ValidateForeignKeys" in text, path, 0, "registry missing foreign key validation")
    require("ValidateAll" in text, path, 0, "registry missing combined validation")
    require("TMap<" in text, path, 0, "registry missing TMap index storage")
    require("BlueprintCallable" in text, path, 0, "registry missing BlueprintCallable API")
    require("Add(," not in text, path, 0, "registry generated empty unique index key")
    require("FindOrAdd()" not in text, path, 0, "registry generated empty group index key")
    require("for (const &" not in text, path, 0, "registry generated missing row type")
    require(".Add(Index)" not in text, path, 0, "registry generated stale index variable")

    if "07_indexes" not in path.parts:
        required_fragments = {}
    else:
        required_fragments = {
            "class UPolygenSchemaRegistry : public UObject": "schema registry class",
            "TArray<FPolygenUser> Users;": "user row storage",
            "TArray<FPolygenPost> Posts;": "post row storage",
            "TMap<int32, int32> UserById;": "primary key index storage",
            "TMap<FString, int32> UserByUsername;": "unique string index storage",
            "TMap<int32, TArray<int32>> PostByAuthorId;": "group index storage",
            "TMap<FString, TArray<int32>> CategorySearchByName;": "string exact search storage",
            "TMap<uint8, TArray<int32>> CategorySearchByRank;": "numeric exact search storage",
            "TMap<EPolygenCategoryKind, TArray<int32>> CategorySearchByKind;": "enum exact search storage",
            "TMap<FString, TArray<int32>> PostSearchByTitle;": "search postings storage",
            "void SetUsers(const TArray<FPolygenUser>& InRows)": "Blueprint set user rows API",
            "const TArray<FPolygenUser>& GetUsers() const": "Blueprint get user rows API",
            "int32 GetUsersCount() const": "Blueprint user count API",
            "bool GetUserByUsername(FString Key, FPolygenUser& OutRow) const": "unique lookup API",
            "void GetPostsByAuthorId(int32 Key, TArray<FPolygenPost>& OutRows) const": "group lookup API",
            "TArray<FPolygenCategory> SearchCategoryByName(FString Query) const": "string exact search API",
            "TArray<FPolygenCategory> SearchCategoryByRank(uint8 Query) const": "numeric exact search API",
            "TArray<FPolygenCategory> SearchCategoryByKind(EPolygenCategoryKind Query) const": "enum exact search API",
            "TArray<FPolygenPost> SearchPostByTitle(const FString& Query) const": "search query API",
            "UserByUsername.Add(Row.username, RowIndex);": "unique index population",
            "PostByAuthorId.FindOrAdd(Row.author_id).Add(RowIndex);": "group index population",
            "CategorySearchByName.FindOrAdd(PolygenRegistryDetail::NormalizeString(Row.name, TEXT(\"lower_trim\"))).Add(RowIndex);": "string exact search population",
            "CategorySearchByRank.FindOrAdd(Row.rank).Add(RowIndex);": "numeric exact search population",
            "CategorySearchByKind.FindOrAdd(Row.kind).Add(RowIndex);": "enum exact search population",
            "PostSearchByTitle.FindOrAdd(Token).Add(RowIndex);": "search postings population",
            "bool GetPostAuthor(const FPolygenPost& Row, FPolygenUser& OutRow) const": "post author navigation API",
            "return GetUserById(Row.author_id, OutRow);": "post author navigation lookup",
            "bool GetPostCategory(const FPolygenPost& Row, FPolygenCategory& OutRow) const": "post category navigation API",
            "return GetCategoryById(Row.category_id, OutRow);": "post category navigation lookup",
            "bool ValidateUniqueKeys(TArray<FPolygenRegistryValidationError>& OutErrors) const": "unique validation API",
            "TSet<int32> UserByIdSeenKeys;": "primary key duplicate tracking",
            "const int32 UserByIdKey = Row.id;": "primary key duplicate key capture",
            "TSet<FString> UserByUsernameSeenKeys;": "string unique duplicate tracking",
            "Error.ConstraintType = TEXT(\"Unique\");": "unique diagnostic classification",
            "bool ValidateAll(TArray<FPolygenRegistryValidationError>& OutErrors) const": "combined validation API",
            "const bool bFieldsValid = ValidateFieldConstraints(FieldErrors);": "combined validation field constraint call",
            "const bool bUniqueValid = ValidateUniqueKeys(UniqueErrors);": "combined validation unique call",
            "const bool bForeignKeysValid = ValidateForeignKeys(ForeignKeyErrors);": "combined validation foreign key call",
            "if (!UserById.Contains(Row.author_id))": "foreign key validation",
            'Error.ConstraintType = TEXT("ForeignKey");': "foreign key diagnostic classification",
        }

    if "08_complex_schema" in path.parts:
        required_fragments.update(
            {
                "bool ValidateFieldConstraints(TArray<FPolygenRegistryValidationError>& OutErrors) const": "field constraint validation API",
                '#include "Internationalization/Regex.h"': "regex support include",
                "if (Row.name.Len() > 32)": "player name max length validation",
                "Error.ConstraintType = TEXT(\"MaxLength\");": "max length diagnostic classification",
                "if (Row.level < 1 || Row.level > 100)": "player level range validation",
                "Error.ConstraintType = TEXT(\"Range\");": "range diagnostic classification",
                'const FRegexPattern PlayerNameRegexPattern(TEXT("^[A-Za-z0-9_ ]+$"));': "player name regex pattern",
                "FRegexMatcher PlayerNameRegexMatcher(PlayerNameRegexPattern, Row.name);": "player name regex matcher",
                "if (!PlayerNameRegexMatcher.FindNext())": "player name regex validation",
                "Error.ConstraintType = TEXT(\"Regex\");": "regex diagnostic classification",
                "OutErrors.Append(FieldErrors);": "combined validation appends field diagnostics",
                "return bFieldsValid && bUniqueValid && bForeignKeysValid;": "combined validation return state",
            }
        )

    if "11_relations_indexes" in path.parts:
        required_fragments.update(
            {
                "TArray<FPolygenUser> Users;": "user row storage",
                "TArray<FPolygenPost> Posts;": "post row storage",
                "TMap<int32, int32> UserById;": "primary key index storage",
                "TMap<int32, TArray<int32>> PostByAuthorId;": "foreign key group index storage",
                "bool GetPostAuthor(const FPolygenPost& Row, FPolygenUser& OutRow) const": "forward FK navigation API",
                "return GetUserById(Row.author_id, OutRow);": "forward FK navigation lookup",
                "void FindUserPosts(const FPolygenUser& Row, TArray<FPolygenPost>& OutRows) const": "reverse relation navigation API",
                "GetPostsByAuthorId(Row.id, OutRows);": "reverse relation lookup",
                "bool ValidateUniqueKeys(TArray<FPolygenRegistryValidationError>& OutErrors) const": "unique validation API",
                "TSet<int32> UserByIdSeenKeys;": "primary key duplicate tracking",
                "const bool bFieldsValid = ValidateFieldConstraints(FieldErrors);": "combined validation field constraint call",
                "const bool bUniqueValid = ValidateUniqueKeys(UniqueErrors);": "combined validation unique call",
                "if (!UserById.Contains(Row.author_id))": "foreign key validation",
            }
        )

    for fragment, description in required_fragments.items():
        require(fragment in text, path, 0, f"registry missing {description}")


def validate_hot_reload(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    require('#include "CoreMinimal.h"' in text, path, 0, "missing CoreMinimal include")
    require("class FPolygenHotReloadManager" in text, path, 0, "missing hot reload manager")
    require("POLYGEN_HOTRELOAD_INIT" in text, path, 0, "missing init macro")

    has_loader_include = "Loaders.h\"" in text
    delegates = {}
    for line_no, line in split_lines(path):
        match = DELEGATE_RE.match(line)
        if match is not None:
            table_name, struct_name = match.groups()
            require(table_name not in delegates, path, line_no, f"duplicate delegate {table_name}")
            delegates[table_name] = struct_name

    if has_loader_include:
        require(delegates, path, 0, "loadable hot reload header missing delegate declarations")
        for table_name, struct_name in delegates.items():
            require(f"TArray<{struct_name}> {table_name}Data;" in text, path, 0, f"missing storage for {table_name}")
            require(f"bool Load{table_name}Data()" in text, path, 0, f"missing load function for {table_name}")
            require(f"On{table_name}Reloaded.Broadcast({table_name}Data);" in text, path, 0, f"missing broadcast for {table_name}")


def validate_redis_keys(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    require('#include "CoreMinimal.h"' in text, path, 0, "missing CoreMinimal include")
    require("namespace PolygenRedisKeys" in text, path, 0, "missing Redis key namespace")
    require("KeyNamespace = TEXT(\"polygen\")" in text, path, 0, "missing key namespace constant")
    require("FORCEINLINE FString Segment" in text, path, 0, "missing Segment helper")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: validate_unreal.py <header> [...]", file=sys.stderr)
        return 2

    for arg in argv[1:]:
        validate_header(Path(arg))

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
