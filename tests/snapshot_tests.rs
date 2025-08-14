use anyhow::Result;
use insta::assert_debug_snapshot;
use PolyGen::{parse_and_merge_schemas, build_ir_from_asts};
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