// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints

use std::io::Cursor;

use polygen_test::schema::test_indexes::{User, Category, Post, Tag, PostTag};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 07: Indexes ===");

    test_user_creation();
    test_category_creation();
    test_post_with_foreign_keys();
    test_junction_table();
    test_binary_serialization();

    println!("=== All tests passed! ===");
}

fn test_user_creation() {
    println!("  Testing User table creation...");

    let user = User {
        id: 1,
        username: "john".to_string(),
        email: "john@example.com".to_string(),
        display_name: "John Doe".to_string(),
    };

    assert_eq!(user.id, 1);
    assert_eq!(user.username, "john");
    assert_eq!(user.email, "john@example.com");
    assert_eq!(user.display_name, "John Doe");

    println!("    PASS");
}

fn test_category_creation() {
    println!("  Testing Category creation...");

    let cat = Category {
        id: 1,
        name: "Technology".to_string(),
        description: Some("Tech stuff".to_string()),
    };

    assert_eq!(cat.id, 1);
    assert_eq!(cat.name, "Technology");
    assert_eq!(cat.description, Some("Tech stuff".to_string()));

    println!("    PASS");
}

fn test_post_with_foreign_keys() {
    println!("  Testing Post with foreign keys...");

    let post = Post {
        id: 1,
        title: "First Post".to_string(),
        content: "Hello World".to_string(),
        author_id: 1,
        category_id: 1,
    };

    assert_eq!(post.id, 1);
    assert_eq!(post.title, "First Post");
    assert_eq!(post.author_id, 1);
    assert_eq!(post.category_id, 1);

    println!("    PASS");
}

fn test_junction_table() {
    println!("  Testing PostTag junction table...");

    let tag = Tag {
        id: 1,
        name: "cpp".to_string(),
    };

    let post_tag = PostTag {
        post_id: 1,
        tag_id: 1,
    };

    assert_eq!(tag.id, 1);
    assert_eq!(tag.name, "cpp");
    assert_eq!(post_tag.post_id, 1);
    assert_eq!(post_tag.tag_id, 1);

    println!("    PASS");
}

fn test_binary_serialization() {
    println!("  Testing binary serialization...");

    let original = User {
        id: 42,
        username: "testuser".to_string(),
        email: "test@test.com".to_string(),
        display_name: "Test User".to_string(),
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = User::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.username, original.username);
    assert_eq!(loaded.email, original.email);
    assert_eq!(loaded.display_name, original.display_name);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
