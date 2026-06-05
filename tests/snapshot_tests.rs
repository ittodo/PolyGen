use anyhow::Result;
use insta::{assert_debug_snapshot, assert_snapshot};
use polygen::{build_ir_from_asts, parse_and_merge_schemas};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn normalize_snapshot_paths(asts: &mut [polygen::AstRoot]) {
    for ast in asts {
        ast.path = PathBuf::from(ast.path.to_string_lossy().replace('\\', "/"));
    }
}

#[test]
fn test_ast_snapshots() -> Result<()> {
    for entry in WalkDir::new("tests/schemas")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "poly"))
    {
        let schema_path = entry.path();
        let schema_name = schema_path.file_stem().unwrap().to_str().unwrap();

        // Test AST generation
        let mut all_asts = parse_and_merge_schemas(schema_path, None)?; // Pass None for output_dir in tests
        normalize_snapshot_paths(&mut all_asts);
        assert_debug_snapshot!(format!("{}_ast", schema_name), all_asts);

        // Test IR generation
        let ir_context = build_ir_from_asts(&all_asts);
        assert_debug_snapshot!(format!("{}_ir", schema_name), ir_context);
    }
    Ok(())
}

#[test]
fn test_csv_mappers_snapshot() -> Result<()> {
    use polygen::{run, Cli};

    let schema_path = PathBuf::from("tests/test_data/csv_test_schema.poly");
    let temp_dir = tempfile::tempdir()?;
    let output_dir = temp_dir.path().to_path_buf();
    let templates_dir = PathBuf::from("templates");

    let cli = Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir,
        output_dir: output_dir.clone(),
        lang: Some("csharp".to_string()),
        baseline: None,
        sources: None,
    };

    run(cli)?;

    // Read generated file
    let generated_path = output_dir.join("csharp/csv_test_schema.CsvMappers.cs");
    let content = std::fs::read_to_string(generated_path)?;

    assert_debug_snapshot!("csv_mappers_csharp", content);
    Ok(())
}

#[test]
fn test_redis_key_helpers_snapshot() -> Result<()> {
    use polygen::{run, Cli};

    let schema_path = PathBuf::from("tests/test_data/redis_cache_schema.poly");
    let templates_dir = PathBuf::from("templates");
    let temp_dir = tempfile::tempdir()?;
    let cases = [
        (
            "redis",
            vec!["redis/schema.redis.json", "redis/schema.redis.lua"],
        ),
        ("csharp", vec!["csharp/redis_cache_schema.RedisKeys.cs"]),
        ("cpp", vec!["cpp/redis_cache_schema_redis_keys.hpp"]),
        ("rust", vec!["rust/redis_cache_schema_redis_keys.rs"]),
        (
            "typescript",
            vec!["typescript/redis_cache_schema_redis_keys.ts"],
        ),
        ("go", vec!["go/redis_cache_schema_redis_keys.go"]),
        ("python", vec!["python/redis_cache_schema_redis_keys.py"]),
        ("kotlin", vec!["kotlin/redis_cache_schema_redis_keys.kt"]),
        ("swift", vec!["swift/redis_cache_schema_redis_keys.swift"]),
        ("unreal", vec!["unreal/PolygenRedisCacheSchemaRedisKeys.h"]),
    ];

    let mut combined = String::new();
    for (lang, files) in cases {
        let output_dir = temp_dir.path().join(lang);
        let cli = Cli {
            command: None,
            schema_path: Some(schema_path.clone()),
            templates_dir: templates_dir.clone(),
            output_dir: output_dir.clone(),
            lang: Some(lang.to_string()),
            baseline: None,
            sources: None,
        };

        run(cli)?;

        for file in files {
            append_snapshot_file(&mut combined, &output_dir, file)?;
        }
    }

    assert_snapshot!("redis_key_helpers", combined);
    Ok(())
}

#[test]
fn test_additional_language_generation_smoke() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("multi_target_smoke.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace demo.catalog {
    enum ItemKind {
        Weapon = 1;
        Armor = 2;
    }

    embed Stats {
        power: i32;
        tags: string[];
    }

    table Item {
        id: u32 primary_key;
        name: string max_length(80);
        kind: ItemKind;
        price: u32 range(0, 9999);
        stats: Stats;
        notes: string?;
    }
}
"#,
    )?;

    let templates_dir = PathBuf::from("templates");
    for lang in ["python", "kotlin", "swift", "protobuf", "messagepack"] {
        run(Cli {
            command: None,
            schema_path: Some(schema_path.clone()),
            templates_dir: templates_dir.clone(),
            output_dir: temp_dir.path().join(lang),
            lang: Some(lang.to_string()),
            baseline: None,
            sources: None,
        })?;
    }

    let python =
        std::fs::read_to_string(temp_dir.path().join("python/python/multi_target_smoke.py"))?;
    assert!(python.contains("class DemoCatalogItemKind(IntEnum):"));
    assert!(python.contains("@dataclass(kw_only=True)"));
    assert!(python.contains("class DemoCatalogStats:"));
    assert!(python.contains("class DemoCatalogItem:"));
    assert!(python.contains("stats: DemoCatalogStats"));

    let pydantic = std::fs::read_to_string(
        temp_dir
            .path()
            .join("python/python/multi_target_smoke_pydantic.py"),
    )?;
    assert!(pydantic.contains("class DemoCatalogItem(BaseModel):"));
    assert!(pydantic.contains("name: str = Field(max_length=80)"));
    assert!(pydantic.contains("price: int = Field(ge=0, le=9999)"));

    let sqlalchemy = std::fs::read_to_string(
        temp_dir
            .path()
            .join("python/python/multi_target_smoke_sqlalchemy.py"),
    )?;
    assert!(sqlalchemy.contains("class DemoCatalogItem(Base):"));
    assert!(sqlalchemy.contains("__tablename__ = 'demo_catalog_Item'"));
    assert!(sqlalchemy.contains("id: Mapped[int] = mapped_column(Integer, primary_key=True)"));
    assert!(sqlalchemy.contains("stats: Mapped[dict] = mapped_column(JSON, nullable=False)"));

    let kotlin =
        std::fs::read_to_string(temp_dir.path().join("kotlin/kotlin/multi_target_smoke.kt"))?;
    assert!(kotlin.contains(
        "@Serializable(with = DemoCatalogItemKindSerializer::class)\nenum class DemoCatalogItemKind(val value: Int)"
    ));
    assert!(
        kotlin.contains("object DemoCatalogItemKindSerializer : KSerializer<DemoCatalogItemKind>")
    );
    assert!(kotlin.contains("fun fromNameOrValue(raw: String): DemoCatalogItemKind"));
    assert!(kotlin.contains("data class DemoCatalogStats("));
    assert!(kotlin.contains("val tags: List<String> = emptyList()"));
    assert!(kotlin.contains("data class DemoCatalogItem("));
    assert!(kotlin.contains("val stats: DemoCatalogStats"));

    let swift =
        std::fs::read_to_string(temp_dir.path().join("swift/swift/multi_target_smoke.swift"))?;
    assert!(swift.contains("enum DemoCatalogItemKind: Int, Codable, CaseIterable, Hashable"));
    assert!(swift.contains("init(from decoder: Decoder) throws"));
    assert!(swift.contains(
        "if let rawValue = try? container.decode(Int.self), let value = DemoCatalogItemKind(rawValue: rawValue)"
    ));
    assert!(swift.contains("struct DemoCatalogStats: Codable, Hashable"));
    assert!(swift.contains("var tags: [String] = []"));
    assert!(swift.contains("struct DemoCatalogItem: Codable, Hashable"));

    let swiftdata = std::fs::read_to_string(
        temp_dir
            .path()
            .join("swift/swift/multi_target_smoke_swiftdata.swift"),
    )?;
    assert!(swiftdata.contains("@Model"));
    assert!(swiftdata.contains("final class DemoCatalogItemModel"));
    assert!(swiftdata.contains("@Attribute(.transformable)\n    var stats: DemoCatalogStats"));

    let proto = std::fs::read_to_string(
        temp_dir
            .path()
            .join("protobuf/protobuf/multi_target_smoke.proto"),
    )?;
    assert!(proto.contains("syntax = \"proto3\";"));
    assert!(proto.contains("package demo.catalog;"));
    assert!(proto.contains("enum DemoCatalogItemKind"));
    assert!(proto.contains("message DemoCatalogStats"));
    assert!(proto.contains("repeated string tags = 2;"));
    assert!(proto.contains("message DemoCatalogItem"));
    assert!(proto.contains("optional string notes = 6;"));

    let messagepack = std::fs::read_to_string(
        temp_dir
            .path()
            .join("messagepack/messagepack/multi_target_smoke.messagepack.json"),
    )?;
    let descriptor: serde_json::Value = serde_json::from_str(&messagepack)?;
    assert_eq!(descriptor["format"], "polygen-messagepack-schema");
    assert_eq!(descriptor["encoding"]["record"], "array");
    assert!(messagepack.contains("\"fqn\": \"demo.catalog.Item\""));
    assert!(!messagepack.contains("\"foreignKey\""));
    assert!(messagepack.contains("\"elementWireType\": \"string\""));

    Ok(())
}

#[test]
fn test_db_auto_timestamp_ddl_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("auto_timestamp.poly");
    std::fs::write(
        &schema_path,
        r#"
@datasource("mysql")
namespace test.mysql {
    table AuditLog {
        id: u32 primary_key;
        created_at: timestamp auto_create;
        updated_at: timestamp auto_update;
    }
}

@datasource("postgresql")
namespace test.pg {
    table AuditLog {
        id: u32 primary_key;
        created_at: timestamp auto_create;
        updated_at: timestamp auto_update;
    }
}

@datasource("sqlite")
namespace test.sqlite {
    table AuditLog {
        id: u32 primary_key;
        created_at: timestamp auto_create;
        updated_at: timestamp auto_update;
    }
}
"#,
    )?;

    let templates_dir = PathBuf::from("templates");
    for lang in ["mysql", "postgresql", "sqlite"] {
        run(Cli {
            command: None,
            schema_path: Some(schema_path.clone()),
            templates_dir: templates_dir.clone(),
            output_dir: temp_dir.path().join(lang),
            lang: Some(lang.to_string()),
            baseline: None,
            sources: None,
        })?;
    }

    let mysql = std::fs::read_to_string(temp_dir.path().join("mysql/mysql/schema.sql"))?;
    assert!(mysql.contains("`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP"));
    assert!(mysql.contains(
        "`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP"
    ));

    let postgresql =
        std::fs::read_to_string(temp_dir.path().join("postgresql/postgresql/schema.sql"))?;
    assert!(postgresql.contains("\"created_at\" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP"));
    assert!(postgresql.contains("\"updated_at\" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP"));
    assert!(postgresql
        .contains("CREATE OR REPLACE FUNCTION \"polygen_set_test_pg_AuditLog_auto_update\"()"));
    assert!(postgresql.contains("NEW.\"updated_at\" = CURRENT_TIMESTAMP"));
    assert!(postgresql.contains("CREATE TRIGGER \"polygen_trg_test_pg_AuditLog_auto_update\""));

    let sqlite = std::fs::read_to_string(temp_dir.path().join("sqlite/sqlite/schema.sql"))?;
    assert!(sqlite.contains("created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP"));
    assert!(sqlite.contains("updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP"));
    assert!(sqlite
        .contains("CREATE TRIGGER IF NOT EXISTS polygen_trg_test_sqlite_AuditLog_auto_update"));
    assert!(sqlite.contains(
        "UPDATE test_sqlite_AuditLog SET updated_at = CURRENT_TIMESTAMP WHERE rowid = NEW.rowid;"
    ));
    let conn = rusqlite::Connection::open_in_memory()?;
    conn.execute_batch(&sqlite)?;
    conn.execute("INSERT INTO test_sqlite_AuditLog (id) VALUES (1)", [])?;
    conn.execute("UPDATE test_sqlite_AuditLog SET id = 1 WHERE id = 1", [])?;

    Ok(())
}

#[test]
fn test_csharp_auto_update_helpers_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("csharp_auto_update.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace test.audit {
    enum BinaryState {
        Unknown = 0;
        Active = 1;
    }

    table CreatedLog {
        id: u32 primary_key;
        created_at: timestamp auto_create(+5:30);
    }

    table NamedCreatedLog {
        id: u32 primary_key;
        created_at: timestamp auto_create("Korea Standard Time");
    }

    table BlobRecord {
        id: u32 primary_key;
        retry_count: i32?;
        state: BinaryState?;
        state_history: BinaryState[];
        payload: bytes;
        optional_payload: bytes?;
        chunks: bytes[];
    }

    table AuditLog {
        id: u32 primary_key;
        updated_at: timestamp auto_update;
        synced_at: timestamp auto_update(local);
    }
}
"#,
    )?;

    run(Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir: PathBuf::from("templates"),
        output_dir: temp_dir.path().join("out"),
        lang: Some("csharp".to_string()),
        baseline: None,
        sources: None,
    })?;

    let csharp = find_generated_file_containing(
        &temp_dir.path().join("out"),
        "Auto-update timestamp methods",
    )?;
    assert!(csharp.contains("public void OnUpdateUpdatedAt()"));
    assert!(csharp.contains("updated_at = DateTime.UtcNow;"));
    assert!(csharp.contains("public void OnUpdateSyncedAt()"));
    assert!(csharp.contains("synced_at = DateTime.Now;"));
    assert!(csharp.contains("public void OnUpdateAll()"));

    let container = find_generated_file_containing(
        &temp_dir.path().join("out"),
        "DateTimeOffset.UtcNow.ToOffset(new TimeSpan(5, 30, 0)).DateTime",
    )?;
    assert!(container.contains(
        "row.created_at = DateTimeOffset.UtcNow.ToOffset(new TimeSpan(5, 30, 0)).DateTime;"
    ));
    assert!(container.contains(
        "row.created_at = TimeZoneInfo.ConvertTimeFromUtc(DateTime.UtcNow, TimeZoneInfo.FindSystemTimeZoneById(\"Korea Standard Time\"));"
    ));

    let readers = find_generated_file_containing(&temp_dir.path().join("out"), "ReadBlobRecord")?;
    assert!(!readers.contains("TODO:"));
    assert!(readers.contains(
        "obj.optional_payload = BinaryUtils.ReadOption<byte[]>(br, BinaryUtils.ReadBytes);"
    ));
    assert!(
        readers.contains("obj.chunks = BinaryUtils.ReadList<byte[]>(br, BinaryUtils.ReadBytes);")
    );
    assert!(readers.contains("BinaryUtils.ReadOptionStruct<int>"));
    assert!(readers.contains("BinaryUtils.ReadOptionStruct<test.audit.BinaryState>"));
    assert!(readers.contains(
        "obj.state_history = BinaryUtils.ReadList<test.audit.BinaryState>(br, BinaryUtils.ReadEnumInt32<test.audit.BinaryState>);"
    ));
    let binary_utils =
        std::fs::read_to_string(temp_dir.path().join("out/csharp/Common/BinaryUtils.cs"))?;
    assert_eq!(
        binary_utils
            .matches("Enum.IsDefined(typeof(T), raw)")
            .count(),
        2
    );
    assert!(binary_utils.contains("throw new InvalidDataException"));

    let writers = find_generated_file_containing(&temp_dir.path().join("out"), "WriteBlobRecord")?;
    assert!(!writers.contains("TODO:"));
    assert!(writers.contains(
        "BinaryUtils.WriteOptionRef<byte[]>(bw, obj.optional_payload, BinaryUtils.WriteBytes);"
    ));
    assert!(
        writers.contains("BinaryUtils.WriteList<byte[]>(bw, obj.chunks, BinaryUtils.WriteBytes);")
    );
    assert!(writers.contains(
        "BinaryUtils.WriteOptionStruct<test.audit.BinaryState>(bw, obj.state, BinaryUtils.WriteEnumInt32<test.audit.BinaryState>);"
    ));
    assert!(writers.contains(
        "BinaryUtils.WriteList<test.audit.BinaryState>(bw, obj.state_history, BinaryUtils.WriteEnumInt32<test.audit.BinaryState>);"
    ));
    let binary_mapping =
        std::fs::read_to_string("templates/csharp/rhai_utils/binary_mapping.rhai")?;
    assert!(!binary_mapping.contains("TODO: Unsupported type for binary"));

    Ok(())
}

#[test]
fn test_cpp_binary_enum_cardinality_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("cpp_binary_enum.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace test.binary {
    enum BinaryState {
        Unknown = 0;
        Active = 1;
    }

    table BlobRecord {
        id: u32 primary_key;
        state: BinaryState;
        optional_state: BinaryState?;
        state_history: BinaryState[];
    }
}
"#,
    )?;

    run(Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir: PathBuf::from("templates"),
        output_dir: temp_dir.path().join("out"),
        lang: Some("cpp".to_string()),
        baseline: None,
        sources: None,
    })?;

    let loaders =
        find_generated_file_containing_ext(&temp_dir.path().join("out"), "read_BlobRecord", "hpp")?;
    assert!(loaders.contains("struct enum_validator<::test::binary::BinaryState>"));
    assert!(loaders.contains("E read_checked_enum(polygen::BinaryReader& reader)"));
    assert!(loaders.contains("void write_checked_enum(polygen::BinaryWriter& writer, E value)"));
    assert!(loaders.contains("obj.state = read_checked_enum<::test::binary::BinaryState>(reader);"));
    assert!(loaders.contains(
        "obj.optional_state = reader.read_optional<::test::binary::BinaryState>([](polygen::BinaryReader& r) { return read_checked_enum<::test::binary::BinaryState>(r); });"
    ));
    assert!(loaders.contains(
        "obj.state_history = reader.read_vector<::test::binary::BinaryState>([](polygen::BinaryReader& r) { return read_checked_enum<::test::binary::BinaryState>(r); });"
    ));
    assert!(loaders.contains("write_checked_enum(writer, obj.state);"));
    assert!(loaders.contains(
        "writer.write_optional(obj.optional_state, [](polygen::BinaryWriter& w, const ::test::binary::BinaryState& v) { write_checked_enum(w, v); });"
    ));
    assert!(loaders.contains(
        "writer.write_vector(obj.state_history, [](polygen::BinaryWriter& w, const ::test::binary::BinaryState& v) { write_checked_enum(w, v); });"
    ));
    assert!(!loaders.contains("read_BinaryState("));
    assert!(!loaders.contains("write_BinaryState("));

    Ok(())
}

#[test]
fn test_rust_enum_try_from_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("rust_enum.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace test.binary {
    enum BinaryState {
        Unknown = 0;
        Active = 1;
    }

    table BlobRecord {
        id: u32 primary_key;
        state: BinaryState;
        optional_state: BinaryState?;
        state_history: BinaryState[];
    }
}
"#,
    )?;

    run(Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir: PathBuf::from("templates"),
        output_dir: temp_dir.path().join("out"),
        lang: Some("rust".to_string()),
        baseline: None,
        sources: None,
    })?;

    let schema = find_generated_file_containing_ext(
        &temp_dir.path().join("out"),
        "impl std::convert::TryFrom<i32> for BinaryState",
        "rs",
    )?;
    assert!(schema.contains("0 => Ok(Self::Unknown),"));
    assert!(schema.contains("1 => Ok(Self::Active),"));
    assert!(schema.contains("other => Err(other),"));
    assert!(schema.contains("impl std::str::FromStr for BinaryState"));
    assert!(schema.contains("\"Unknown\" => Ok(Self::Unknown),"));
    assert!(schema.contains("\"Active\" => Ok(Self::Active),"));
    assert!(schema.contains("trimmed.parse::<i32>()"));

    let loaders = find_generated_file_containing_ext(
        &temp_dir.path().join("out"),
        "impl BinaryIO for crate::rust_enum::test_binary::BlobRecord",
        "rs",
    )?;
    assert!(!loaders.contains("transmute"));
    assert!(!loaders.contains("optional_state: row.get_i32(\"optional_state\").map"));
    assert!(!loaders.contains("state: row.get_i32(\"state\")"));
    assert!(loaders.contains(
        "state: row.get(\"state\").ok_or_else(|| LoadError::Parse(\"state\".into())).and_then(|s| s.parse::<crate::rust_enum::test_binary::BinaryState>()"
    ));
    assert!(loaders.contains(
        "optional_state: row.get_optional(\"optional_state\").map(|s| s.parse::<crate::rust_enum::test_binary::BinaryState>()"
    ));
    assert!(loaders.contains(
        "state_history: row.get(\"state_history\").map(|s| s.split(',').filter(|v| !v.trim().is_empty()).map(|v| v.trim().parse::<crate::rust_enum::test_binary::BinaryState>()"
    ));
    assert!(loaders.contains("crate::rust_enum::test_binary::BinaryState::try_from(v).map_err"));
    assert!(loaders.contains(
        "reader.read_i32().and_then(|v| crate::rust_enum::test_binary::BinaryState::try_from(v).map_err"
    ));
    assert!(loaders.contains(
        "reader.read_optional(|r| r.read_i32().and_then(|v| crate::rust_enum::test_binary::BinaryState::try_from(v).map_err"
    ));
    assert!(loaders.contains(
        "reader.read_vec(|r| r.read_i32().and_then(|v| crate::rust_enum::test_binary::BinaryState::try_from(v).map_err"
    ));
    assert!(loaders.contains("collect::<Result<Vec<_>, LoadError>>()"));

    Ok(())
}

#[test]
fn test_rust_csv_list_parse_errors_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("rust_csv_lists.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace test.csv {
    table ArrayRecord {
        id: u32 primary_key;
        numbers: i32[];
        flags: bool[];
        names: string[];
        retry_count: i32?;
        enabled: bool?;
    }
}
"#,
    )?;

    run(Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir: PathBuf::from("templates"),
        output_dir: temp_dir.path().join("out"),
        lang: Some("rust".to_string()),
        baseline: None,
        sources: None,
    })?;

    let loaders = find_generated_file_containing_ext(
        &temp_dir.path().join("out"),
        "impl CsvLoadable for crate::rust_csv_lists::test_csv::ArrayRecord",
        "rs",
    )?;
    assert!(!loaders.contains("filter_map(|v| v.trim().parse().ok())"));
    assert!(loaders.contains("parse::<i32>()"));
    assert!(!loaders.contains("parse::<bool>()"));
    assert!(loaders.contains("\"true\" | \"1\" | \"yes\" => Ok(true)"));
    assert!(loaders.contains("\"false\" | \"0\" | \"no\" => Ok(false)"));
    assert!(loaders.contains("collect::<Result<Vec<_>, LoadError>>()"));
    assert!(loaders.contains("invalid value for numbers"));
    assert!(loaders.contains("invalid value for flags"));
    assert!(loaders.contains("v.trim().to_string()"));
    assert!(
        loaders.contains("retry_count: row.get_optional(\"retry_count\").map(|s| s.parse::<i32>()")
    );
    assert!(loaders
        .contains("enabled: row.get_optional(\"enabled\").map(|_| row.get_bool(\"enabled\")"));

    Ok(())
}

#[test]
fn test_go_pack_methods_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("go_pack.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace test.pack {
    @pack
    embed PackedValues {
        ratio: f32;
        count: u32;
        delta: i32;
        enabled: bool;
    }
}
"#,
    )?;

    run(Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir: PathBuf::from("templates"),
        output_dir: temp_dir.path().join("out"),
        lang: Some("go".to_string()),
        baseline: None,
        sources: None,
    })?;

    let schema = find_generated_file_containing_ext(
        &temp_dir.path().join("out"),
        "func (v PackedValues) Pack() string",
        "go",
    )?;
    assert!(schema.contains("return packValues(\";\", v.Ratio, v.Count, v.Delta, v.Enabled)"));
    assert!(schema.contains("func UnpackPackedValues(value string) (PackedValues, error)"));
    assert!(schema.contains("v0, err := parsePackedFloat32(parts[0], \"PackedValues\", \"ratio\")"));
    assert!(schema.contains("v1, err := parsePackedUint32(parts[1], \"PackedValues\", \"count\")"));
    assert!(schema.contains("v2, err := parsePackedInt32(parts[2], \"PackedValues\", \"delta\")"));
    assert!(schema.contains("v3, err := parsePackedBool(parts[3], \"PackedValues\", \"enabled\")"));
    assert!(schema.contains("func TryUnpackPackedValues(value string) (PackedValues, bool)"));

    let support = find_generated_file_containing_ext(
        &temp_dir.path().join("out"),
        "func parsePackedFloat32",
        "go",
    )?;
    assert!(support.contains("math.IsNaN(parsed) || math.IsInf(parsed, 0)"));
    assert!(support.contains("strconv.ParseUint(value, 10, bitSize)"));

    Ok(())
}

#[test]
fn test_typescript_pack_unpack_validation_generation() -> Result<()> {
    use polygen::{run, Cli};

    let temp_dir = tempfile::tempdir()?;
    let schema_path = temp_dir.path().join("typescript_pack_validation.poly");
    std::fs::write(
        &schema_path,
        r#"
namespace test.pack {
    @pack
    embed PackedValues {
        ratio: f32;
        count: u32;
        delta: i32;
        enabled: bool;
    }
}
"#,
    )?;

    run(Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir: PathBuf::from("templates"),
        output_dir: temp_dir.path().join("out"),
        lang: Some("typescript".to_string()),
        baseline: None,
        sources: None,
    })?;

    let zod = find_generated_file_containing_ext(
        &temp_dir.path().join("out"),
        "export function unpackPackedValues",
        "ts",
    )?;
    assert!(zod.contains("function __parsePackedFloat(value: string, fieldName: string): number"));
    assert!(zod.contains(
        "function __parsePackedInteger(value: string, fieldName: string, min?: number): number"
    ));
    assert!(zod.contains("function __parsePackedBool(value: string, fieldName: string): boolean"));
    assert!(zod.contains("ratio: __parsePackedFloat(parts[0], 'ratio')"));
    assert!(zod.contains("count: __parsePackedInteger(parts[1], 'count', 0)"));
    assert!(zod.contains("delta: __parsePackedInteger(parts[2], 'delta')"));
    assert!(zod.contains("enabled: __parsePackedBool(parts[3], 'enabled')"));
    assert!(!zod.contains("parseFloat(parts["));
    assert!(!zod.contains("parseInt(parts["));
    assert!(!zod.contains("parts[3] === 'true'"));

    Ok(())
}

fn append_snapshot_file(
    snapshot: &mut String,
    output_dir: &Path,
    relative_path: &str,
) -> Result<()> {
    let path = output_dir.join(relative_path);
    let content = std::fs::read_to_string(path)?;

    snapshot.push_str("\n--- ");
    snapshot.push_str(relative_path);
    snapshot.push_str(" ---\n");
    snapshot.push_str(content.trim_end());
    snapshot.push('\n');

    Ok(())
}

fn find_generated_file_containing(root: &Path, needle: &str) -> Result<String> {
    find_generated_file_containing_ext(root, needle, "cs")
}

fn find_generated_file_containing_ext(
    root: &Path,
    needle: &str,
    extension: &str,
) -> Result<String> {
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some(extension)
        {
            continue;
        }

        let content = std::fs::read_to_string(entry.path())?;
        if content.contains(needle) {
            return Ok(content);
        }
    }

    anyhow::bail!("generated {extension} file containing `{needle}` was not found")
}
