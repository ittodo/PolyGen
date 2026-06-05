// Test Case 06: Arrays and Optionals
// Tests array and optional field types

use std::io::Cursor;

use polygen_test::polygen_support::CsvRow;
use polygen_test::schema::test_collections::{ArrayTest, MixedTest, OptionalTest, Tag};
use polygen_test::schema_loaders::{BinaryIO, CsvLoadable};

fn main() {
    println!("=== Test Case 06: Arrays and Optionals ===");

    test_array_primitives();
    test_array_complex_types();
    test_optional_primitives();
    test_csv_array_parse_errors();
    test_csv_optional_parse_errors();
    test_csv_json_cell_embeds();
    test_binary_arrays_optionals();

    println!("=== All tests passed! ===");
}

fn test_array_primitives() {
    println!("  Testing ArrayTest with primitive arrays...");

    let arr = ArrayTest {
        id: 1,
        int_list: vec![1, 2, 3, 4, 5],
        string_list: vec!["one".to_string(), "two".to_string(), "three".to_string()],
        float_list: vec![1.1, 2.2, 3.3],
        bool_list: vec![true, false, true],
        tags: vec![],
    };

    assert_eq!(arr.int_list.len(), 5);
    assert_eq!(arr.int_list[0], 1);
    assert_eq!(arr.int_list[4], 5);
    assert_eq!(arr.string_list.len(), 3);
    assert_eq!(arr.string_list[1], "two");
    assert_eq!(arr.float_list.len(), 3);
    assert_eq!(arr.bool_list.len(), 3);
    assert!(arr.bool_list[0]);
    assert!(!arr.bool_list[1]);

    println!("    PASS");
}

fn test_array_complex_types() {
    println!("  Testing ArrayTest with complex type arrays...");

    let arr = ArrayTest {
        id: 2,
        int_list: vec![],
        string_list: vec![],
        float_list: vec![],
        bool_list: vec![],
        tags: vec![
            Tag { name: "Important".to_string(), color: "red".to_string() },
            Tag { name: "Review".to_string(), color: "yellow".to_string() },
        ],
    };

    assert_eq!(arr.tags.len(), 2);
    assert_eq!(arr.tags[0].name, "Important");
    assert_eq!(arr.tags[0].color, "red");
    assert_eq!(arr.tags[1].name, "Review");

    println!("    PASS");
}

fn test_optional_primitives() {
    println!("  Testing OptionalTest with optional primitives...");

    let mut opt = OptionalTest {
        id: 1,
        required_name: "Test".to_string(),
        opt_int: None,
        opt_string: None,
        opt_float: None,
        opt_bool: None,
        opt_tag: None,
    };

    assert_eq!(opt.required_name, "Test");
    assert!(opt.opt_int.is_none());
    assert!(opt.opt_string.is_none());
    assert!(opt.opt_float.is_none());
    assert!(opt.opt_bool.is_none());

    // Set values
    opt.opt_int = Some(42);
    opt.opt_string = Some("optional value".to_string());
    opt.opt_float = Some(3.14159);
    opt.opt_bool = Some(true);

    assert_eq!(opt.opt_int, Some(42));
    assert_eq!(opt.opt_string, Some("optional value".to_string()));
    assert!((opt.opt_float.unwrap() - 3.14159).abs() < 0.0001);
    assert_eq!(opt.opt_bool, Some(true));

    println!("    PASS");
}

fn test_csv_array_parse_errors() {
    println!("  Testing CSV array parse errors...");

    let bool_row = CsvRow::new(
        vec![
            "id".to_string(),
            "int_list".to_string(),
            "string_list".to_string(),
            "float_list".to_string(),
            "bool_list".to_string(),
            "tags".to_string(),
        ],
        vec![
            "1".to_string(),
            "10,20,30".to_string(),
            "one,two".to_string(),
            "1.5,2.5".to_string(),
            "yes,0,no,true".to_string(),
            "".to_string(),
        ],
    );

    let parsed = ArrayTest::from_csv_row(&bool_row).unwrap();
    assert_eq!(parsed.bool_list, vec![true, false, false, true]);

    let row = CsvRow::new(
        vec![
            "id".to_string(),
            "int_list".to_string(),
            "string_list".to_string(),
            "float_list".to_string(),
            "bool_list".to_string(),
            "tags".to_string(),
        ],
        vec![
            "1".to_string(),
            "10,not-an-int,30".to_string(),
            "one,two".to_string(),
            "1.5,2.5".to_string(),
            "true,false".to_string(),
            "".to_string(),
        ],
    );

    let err = ArrayTest::from_csv_row(&row).unwrap_err();
    assert!(format!("{}", err).contains("invalid value for int_list"));

    println!("    PASS");
}

fn test_csv_optional_parse_errors() {
    println!("  Testing CSV optional parse errors...");

    let invalid_row = CsvRow::new(
        vec![
            "id".to_string(),
            "required_name".to_string(),
            "opt_int".to_string(),
            "opt_string".to_string(),
            "opt_float".to_string(),
            "opt_bool".to_string(),
            "opt_tag".to_string(),
        ],
        vec![
            "1".to_string(),
            "Test".to_string(),
            "not-an-int".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ],
    );

    let err = OptionalTest::from_csv_row(&invalid_row).unwrap_err();
    assert!(format!("{}", err).contains("invalid value for opt_int"));

    let bool_row = CsvRow::new(
        vec![
            "id".to_string(),
            "required_name".to_string(),
            "opt_int".to_string(),
            "opt_string".to_string(),
            "opt_float".to_string(),
            "opt_bool".to_string(),
            "opt_tag".to_string(),
        ],
        vec![
            "2".to_string(),
            "BoolTest".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "yes".to_string(),
            "".to_string(),
        ],
    );

    let parsed = OptionalTest::from_csv_row(&bool_row).unwrap();
    assert_eq!(parsed.opt_bool, Some(true));

    println!("    PASS");
}

fn test_csv_json_cell_embeds() {
    println!("  Testing CSV JSON-cell embedded values...");

    let array_row = CsvRow::new(
        vec![
            "id".to_string(),
            "int_list".to_string(),
            "string_list".to_string(),
            "float_list".to_string(),
            "bool_list".to_string(),
            "tags".to_string(),
        ],
        vec![
            "3".to_string(),
            "1,2".to_string(),
            "one,two".to_string(),
            "1.5,2.5".to_string(),
            "true,false".to_string(),
            r#"[{"name":"Important","color":"red"},{"name":"Review","color":"yellow"}]"#.to_string(),
        ],
    );

    let parsed_array = ArrayTest::from_csv_row(&array_row).unwrap();
    assert_eq!(parsed_array.tags.len(), 2);
    assert_eq!(parsed_array.tags[0].name, "Important");
    assert_eq!(parsed_array.tags[1].color, "yellow");

    let optional_row = CsvRow::new(
        vec![
            "id".to_string(),
            "required_name".to_string(),
            "opt_int".to_string(),
            "opt_string".to_string(),
            "opt_float".to_string(),
            "opt_bool".to_string(),
            "opt_tag".to_string(),
        ],
        vec![
            "4".to_string(),
            "WithTag".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            r#"{"name":"Optional","color":"blue"}"#.to_string(),
        ],
    );

    let parsed_optional = OptionalTest::from_csv_row(&optional_row).unwrap();
    let opt_tag = parsed_optional.opt_tag.expect("optional tag should parse");
    assert_eq!(opt_tag.name, "Optional");
    assert_eq!(opt_tag.color, "blue");

    let mixed_row = CsvRow::new(
        vec![
            "id".to_string(),
            "opt_tags".to_string(),
            "meta".to_string(),
            "history".to_string(),
        ],
        vec![
            "5".to_string(),
            r#"[{"name":"One","color":"green"}]"#.to_string(),
            r#"{"created_by":"alice","updated_by":null,"version":3}"#.to_string(),
            r#"[{"created_by":null,"updated_by":"bob","version":1}]"#.to_string(),
        ],
    );

    let parsed_mixed = MixedTest::from_csv_row(&mixed_row).unwrap();
    assert_eq!(parsed_mixed.opt_tags.len(), 1);
    assert_eq!(parsed_mixed.opt_tags[0].name, "One");
    let meta = parsed_mixed.meta.expect("metadata should parse");
    assert_eq!(meta.created_by.as_deref(), Some("alice"));
    assert_eq!(meta.version, 3);
    assert_eq!(parsed_mixed.history.len(), 1);
    assert_eq!(parsed_mixed.history[0].updated_by.as_deref(), Some("bob"));

    let invalid_row = CsvRow::new(
        vec![
            "id".to_string(),
            "int_list".to_string(),
            "string_list".to_string(),
            "float_list".to_string(),
            "bool_list".to_string(),
            "tags".to_string(),
        ],
        vec![
            "6".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "not-json".to_string(),
        ],
    );
    let err = ArrayTest::from_csv_row(&invalid_row).unwrap_err();
    assert!(format!("{}", err).contains("invalid JSON for tags"));

    println!("    PASS");
}

fn test_binary_arrays_optionals() {
    println!("  Testing binary serialization with arrays and optionals...");

    let original = ArrayTest {
        id: 123,
        int_list: vec![10, 20, 30],
        string_list: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        float_list: vec![1.5, 2.5],
        bool_list: vec![true, false],
        tags: vec![
            Tag { name: "Test".to_string(), color: "white".to_string() },
        ],
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = ArrayTest::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.int_list, original.int_list);
    assert_eq!(loaded.string_list[0], "a");
    assert!(loaded.bool_list[0]);
    assert_eq!(loaded.tags.len(), 1);
    assert_eq!(loaded.tags[0].name, "Test");

    println!("    PASS (serialized {} bytes)", buffer.len());
}
