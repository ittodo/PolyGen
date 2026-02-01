use anyhow::Result;
use insta::assert_debug_snapshot;
use polygen::{build_ir_from_asts, parse_and_merge_schemas};
use walkdir::WalkDir;

#[test]
fn test_ast_snapshots() -> Result<()> {
    for entry in WalkDir::new("tests/schemas")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "poly"))
    {
        let schema_path = entry.path();
        let schema_name = schema_path.file_stem().unwrap().to_str().unwrap();

        // Test AST generation
        let all_asts = parse_and_merge_schemas(schema_path, None)?; // Pass None for output_dir in tests
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
    use std::path::PathBuf;

    let schema_path = PathBuf::from("tests/test_data/csv_test_schema.poly");
    let output_dir = PathBuf::from("tests/output/snapshot_test");
    let templates_dir = PathBuf::from("templates");

    let cli = Cli {
        command: None,
        schema_path: Some(schema_path),
        templates_dir,
        output_dir: output_dir.clone(),
        lang: Some("csharp".to_string()),
        baseline: None,
    };

    run(cli)?;

    // Read generated file
    let generated_path = output_dir.join("csharp/csv_test_schema.CsvMappers.cs");
    let content = std::fs::read_to_string(generated_path)?;

    assert_debug_snapshot!("csv_mappers_csharp", content);
    Ok(())
}
