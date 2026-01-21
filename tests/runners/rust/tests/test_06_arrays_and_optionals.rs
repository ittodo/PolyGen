// Test Case 06: Arrays and Optionals
// Tests array and optional field types

use std::io::Cursor;

use polygen_test::schema::test_collections::{ArrayTest, OptionalTest, Tag};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 06: Arrays and Optionals ===");

    test_array_primitives();
    test_array_complex_types();
    test_optional_primitives();
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
