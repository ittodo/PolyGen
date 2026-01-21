// Test Case 03: Nested Namespaces
// Tests deeply nested namespace structures

use std::io::Cursor;

use polygen_test::schema::app::app_data::app_data_models::User;
use polygen_test::schema::app::app_data::app_data_enums::Permission;
use polygen_test::schema::app::app_services::UserService;
use polygen_test::schema::util::Config;
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 03: Nested Namespaces ===");

    test_deeply_nested_table();
    test_nested_enum();
    test_cross_namespace_reference();
    test_separate_namespace();
    test_binary_nested();

    println!("=== All tests passed! ===");
}

fn test_deeply_nested_table() {
    println!("  Testing deeply nested table (app.data.models.User)...");

    let user = User {
        id: 1,
        username: "testuser".to_string(),
    };

    assert_eq!(user.id, 1);
    assert_eq!(user.username, "testuser");

    println!("    PASS");
}

fn test_nested_enum() {
    println!("  Testing nested enum (app.data.enums.Permission)...");

    let perm = Permission::Admin;
    assert_eq!(perm as i32, 3);

    let perm = Permission::Read;
    assert_eq!(perm as i32, 1);

    println!("    PASS");
}

fn test_cross_namespace_reference() {
    println!("  Testing cross-namespace reference (UserService)...");

    let service = UserService {
        id: 1,
        target_user_id: 42,
        permission: Permission::Write,
    };

    assert_eq!(service.id, 1);
    assert_eq!(service.target_user_id, 42);
    assert_eq!(service.permission, Permission::Write);

    println!("    PASS");
}

fn test_separate_namespace() {
    println!("  Testing separate namespace (util.Config)...");

    let config = Config {
        key: "debug_mode".to_string(),
        value: "true".to_string(),
    };

    assert_eq!(config.key, "debug_mode");
    assert_eq!(config.value, "true");

    println!("    PASS");
}

fn test_binary_nested() {
    println!("  Testing binary serialization with nested namespaces...");

    let original = UserService {
        id: 999,
        target_user_id: 42,
        permission: Permission::Admin,
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = UserService::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.target_user_id, original.target_user_id);
    assert_eq!(loaded.permission, original.permission);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
