// Test Case 09: SQLite Accessor
// Tests SQLite database access with generated accessor code

use rusqlite::{Connection, Result};
use polygen_test::schema::test_sqlite::{User, Post, Comment, PostStatus};
use polygen_test::schema_sqlite_accessor::SqliteDb;

fn main() {
    println!("=== Test Case 09: SQLite Accessor ===");

    test_entity_creation();
    test_enum();
    test_sqlite_operations().expect("SQLite operations failed");
    test_sqlite_db_accessor().expect("SqliteDb accessor tests failed");

    println!("=== All tests passed! ===");
}

fn test_entity_creation() {
    println!("  Testing entity creation...");

    let user = User {
        id: 1,
        name: "TestUser".to_string(),
        email: Some("test@example.com".to_string()),
        created_at: 1700000000,
    };

    assert_eq!(user.id, 1);
    assert_eq!(user.name, "TestUser");
    assert_eq!(user.email, Some("test@example.com".to_string()));
    assert_eq!(user.created_at, 1700000000);

    let post = Post {
        id: 1,
        user_id: 1,
        title: "Test Title".to_string(),
        content: Some("Test Content".to_string()),
    };

    assert_eq!(post.id, 1);
    assert_eq!(post.user_id, 1);
    assert_eq!(post.title, "Test Title");

    let comment = Comment {
        id: 1,
        post_id: 1,
        user_id: 2,
        content: "Test Comment".to_string(),
    };

    assert_eq!(comment.id, 1);
    assert_eq!(comment.post_id, 1);
    assert_eq!(comment.user_id, 2);

    println!("    PASS");
}

fn test_enum() {
    println!("  Testing PostStatus enum...");

    assert_eq!(PostStatus::Draft as i32, 0);
    assert_eq!(PostStatus::Published as i32, 1);
    assert_eq!(PostStatus::Archived as i32, 2);

    println!("    PASS");
}

fn setup_database(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS test_sqlite_User (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS test_sqlite_Post (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT,
            FOREIGN KEY (user_id) REFERENCES test_sqlite_User(id)
        );

        CREATE TABLE IF NOT EXISTS test_sqlite_Comment (
            id INTEGER PRIMARY KEY,
            post_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            FOREIGN KEY (post_id) REFERENCES test_sqlite_Post(id),
            FOREIGN KEY (user_id) REFERENCES test_sqlite_User(id)
        );

        INSERT INTO test_sqlite_User (id, name, email, created_at) VALUES
            (1, 'Alice', 'alice@example.com', 1700000000),
            (2, 'Bob', NULL, 1700000001),
            (3, 'Charlie', 'charlie@example.com', 1700000002);

        INSERT INTO test_sqlite_Post (id, user_id, title, content) VALUES
            (1, 1, 'First Post', 'Hello World'),
            (2, 1, 'Second Post', NULL),
            (3, 2, 'Bob Post', 'Content here');

        INSERT INTO test_sqlite_Comment (id, post_id, user_id, content) VALUES
            (1, 1, 2, 'Nice post!'),
            (2, 1, 3, 'Great work'),
            (3, 3, 1, 'Thanks for sharing');
        "
    )?;
    Ok(())
}

fn test_sqlite_operations() -> Result<()> {
    println!("  Testing direct SQLite operations...");

    let conn = Connection::open_in_memory()?;
    setup_database(&conn)?;

    // Verify data
    let user_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM test_sqlite_User",
        [],
        |row| row.get(0)
    )?;
    assert_eq!(user_count, 3, "Expected 3 users");

    let post_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM test_sqlite_Post",
        [],
        |row| row.get(0)
    )?;
    assert_eq!(post_count, 3, "Expected 3 posts");

    let comment_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM test_sqlite_Comment",
        [],
        |row| row.get(0)
    )?;
    assert_eq!(comment_count, 3, "Expected 3 comments");

    // Test reading a specific user
    let user_name: String = conn.query_row(
        "SELECT name FROM test_sqlite_User WHERE id = 1",
        [],
        |row| row.get(0)
    )?;
    assert_eq!(user_name, "Alice");

    println!("    PASS");
    Ok(())
}

fn test_sqlite_db_accessor() -> Result<()> {
    println!("  Testing SqliteDb accessor...");

    // Create in-memory database with test data
    let temp_file = std::env::temp_dir().join("polygen_test_09.db");

    // Clean up if exists
    let _ = std::fs::remove_file(&temp_file);

    {
        let conn = Connection::open(&temp_file)?;
        setup_database(&conn)?;
    }

    // Test SqliteDb
    let mut db = SqliteDb::open(&temp_file)?;

    // Load all tables
    db.load_all()?;

    // Verify users
    assert_eq!(db.users.len(), 3, "Expected 3 users loaded");
    assert_eq!(db.users.all()[0].name, "Alice");
    assert_eq!(db.users.all()[1].name, "Bob");
    assert!(db.users.all()[1].email.is_none(), "Bob's email should be None");

    // Verify posts
    assert_eq!(db.posts.len(), 3, "Expected 3 posts loaded");

    // Verify comments
    assert_eq!(db.comments.len(), 3, "Expected 3 comments loaded");

    // Test get_by_id
    let user = db.get_user_by_id(2)?;
    assert!(user.is_some(), "User with id=2 should exist");
    assert_eq!(user.unwrap().name, "Bob");

    let non_existent = db.get_user_by_id(999)?;
    assert!(non_existent.is_none(), "User with id=999 should not exist");

    // Clean up
    let _ = std::fs::remove_file(&temp_file);

    println!("    PASS");
    Ok(())
}
