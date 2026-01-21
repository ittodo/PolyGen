// Test Case 01: Basic Types
// Tests all primitive types and simple struct generation

use std::io::Cursor;

use polygen_test::schema::test_basic::{AllTypes, SimpleStruct};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 01: Basic Types ===");

    test_all_types();
    test_simple_struct();
    test_binary_serialization();

    println!("=== All tests passed! ===");
}

fn test_all_types() {
    println!("  Testing AllTypes creation...");

    let all = AllTypes {
        val_u8: 255,
        val_u16: 65535,
        val_u32: 4294967295,
        val_u64: 18446744073709551615,
        val_i8: -128,
        val_i16: -32768,
        val_i32: -2147483648,
        val_i64: -9223372036854775808,
        val_f32: 3.14159,
        val_f64: 2.718281828459045,
        val_bool: true,
        val_string: "Hello, World!".to_string(),
        val_bytes: vec![1, 2, 3],
    };

    assert_eq!(all.val_u8, 255);
    assert_eq!(all.val_u16, 65535);
    assert_eq!(all.val_u32, 4294967295);
    assert_eq!(all.val_u64, 18446744073709551615);
    assert_eq!(all.val_i8, -128);
    assert_eq!(all.val_i16, -32768);
    assert_eq!(all.val_i32, -2147483648);
    assert_eq!(all.val_i64, -9223372036854775808);
    assert!((all.val_f32 - 3.14159).abs() < 0.0001);
    assert!((all.val_f64 - 2.718281828459045).abs() < 0.0000001);
    assert!(all.val_bool);
    assert_eq!(all.val_string, "Hello, World!");

    println!("    PASS");
}

fn test_simple_struct() {
    println!("  Testing SimpleStruct...");

    let simple = SimpleStruct {
        id: 42,
        name: "Test".to_string(),
        value: 100,
    };

    assert_eq!(simple.id, 42);
    assert_eq!(simple.name, "Test");
    assert_eq!(simple.value, 100);

    println!("    PASS");
}

fn test_binary_serialization() {
    println!("  Testing binary serialization...");

    let original = SimpleStruct {
        id: 123,
        name: "Serialization Test".to_string(),
        value: 456,
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = SimpleStruct::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.value, original.value);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
