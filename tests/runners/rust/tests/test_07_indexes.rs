// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints

use std::io::{Cursor, ErrorKind};

use polygen_test::schema::test_indexes::{Category, CategoryKind, Post, PostTag, Tag, User};
use polygen_test::schema_binary_refs::{read_binary_ref_document, write_binary_ref_document};
use polygen_test::schema_container::container::SchemaContainer;
use polygen_test::schema_loaders::{BinaryIO, CsvLoadable};

fn main() {
    println!("=== Test Case 07: Indexes ===");

    test_user_creation();
    test_category_creation();
    test_post_with_foreign_keys();
    test_junction_table();
    test_binary_serialization();
    test_container_search();
    test_container_load_from_csv_sources();
    test_container_load_from_json_sources();
    test_enum_csv_loader();
    test_binary_refs();
    test_binary_ref_rejects_invalid_enum_read();

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
        rank: 7,
        kind: CategoryKind::Public,
    };

    assert_eq!(cat.id, 1);
    assert_eq!(cat.name, "Technology");
    assert_eq!(cat.description, Some("Tech stuff".to_string()));
    assert_eq!(cat.rank, 7);
    assert_eq!(cat.kind, CategoryKind::Public);

    println!("    PASS");
}

fn test_binary_refs() {
    println!("  Testing BinaryRef document indexes and search...");

    let mut container = SchemaContainer::new();
    container.users.add_row(User {
        id: 1,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        display_name: "Alice".to_string(),
    });
    container.categorys.add_row(Category {
        id: 10,
        name: "Tech".to_string(),
        description: Some("Technology".to_string()),
        rank: 7,
        kind: CategoryKind::Public,
    });
    container.posts.add_row(Post {
        id: 100,
        title: "Binary refs".to_string(),
        content: "Lazy row access".to_string(),
        author_id: 1,
        category_id: 10,
    });
    container.posts.add_row(Post {
        id: 101,
        title: "Indexes".to_string(),
        content: "Lookup by author".to_string(),
        author_id: 1,
        category_id: 10,
    });

    let bytes = write_binary_ref_document(&container).unwrap();
    let document = read_binary_ref_document(bytes).unwrap();

    assert_eq!(document.users.count(), 1);
    let user_ref = document.users.get_by_id(1).expect("user ref should exist");
    assert_eq!(user_ref.username().unwrap(), "alice");

    let posts_by_author = document.posts.find_by_author_id(1);
    assert_eq!(posts_by_author.len(), 2);
    assert_eq!(posts_by_author[0].author_id().unwrap(), 1);

    let title_matches = document.posts.search_by_title("binary");
    assert_eq!(title_matches.len(), 1);
    assert_eq!(title_matches[0].id().unwrap(), 100);

    let name_matches = document.categorys.search_by_name("tech");
    assert_eq!(name_matches.len(), 1);
    assert_eq!(name_matches[0].name().unwrap(), "Tech");

    let rank_matches = document.categorys.search_by_rank(7);
    assert_eq!(rank_matches.len(), 1);
    assert_eq!(rank_matches[0].id().unwrap(), 10);

    let kind_matches = document.categorys.search_by_kind(CategoryKind::Public);
    assert_eq!(kind_matches.len(), 1);
    assert_eq!(kind_matches[0].id().unwrap(), 10);

    println!("    PASS (document {} bytes)", document.bytes().len());
}

fn read_u32_at(bytes: &[u8], pos: &mut usize) -> u32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[*pos..*pos + 4]);
    *pos += 4;
    u32::from_le_bytes(raw)
}

fn read_string_at(bytes: &[u8], pos: &mut usize) -> String {
    let len = read_u32_at(bytes, pos) as usize;
    let value = std::str::from_utf8(&bytes[*pos..*pos + len])
        .unwrap()
        .to_string();
    *pos += len;
    value
}

fn skip_string_at(bytes: &[u8], pos: &mut usize) {
    let len = read_u32_at(bytes, pos) as usize;
    *pos += len;
}

fn patch_first_category_kind(bytes: &mut [u8], value: i32) {
    let mut pos = 8; // BINARY_REF_MAGIC
    let table_count = read_u32_at(bytes, &mut pos);

    for _ in 0..table_count {
        let table_name = read_string_at(bytes, &mut pos);
        let row_count = read_u32_at(bytes, &mut pos);
        for row_index in 0..row_count {
            let row_len = read_u32_at(bytes, &mut pos) as usize;
            let row_start = pos;
            let row_end = row_start + row_len;

            if table_name == "test.indexes.Category" && row_index == 0 {
                let mut field_pos = row_start;
                field_pos += 4; // id: u32
                skip_string_at(bytes, &mut field_pos); // name
                let has_description = bytes[field_pos] != 0;
                field_pos += 1;
                if has_description {
                    skip_string_at(bytes, &mut field_pos);
                }
                field_pos += 1; // rank: u8
                bytes[field_pos..field_pos + 4].copy_from_slice(&value.to_le_bytes());
                return;
            }

            pos = row_end;
        }
    }

    panic!("Category row not found in BinaryRef document");
}

fn test_binary_ref_rejects_invalid_enum_read() {
    println!("  Testing BinaryRef invalid enum read rejection...");

    let mut container = SchemaContainer::new();
    container.categorys.add_row(Category {
        id: 10,
        name: "Tech".to_string(),
        description: Some("Technology".to_string()),
        rank: 7,
        kind: CategoryKind::Public,
    });

    let mut bytes = write_binary_ref_document(&container).unwrap();
    patch_first_category_kind(&mut bytes, 999);

    let err = read_binary_ref_document(bytes).expect_err("invalid enum should reject document");
    assert_eq!(err.kind(), ErrorKind::InvalidData);

    println!("    PASS");
}

fn test_container_search() {
    println!("  Testing container search indexes...");

    let mut container = SchemaContainer::new();
    container.users.add_row(User {
        id: 1,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        display_name: "Alice".to_string(),
    });
    container.categorys.add_row(Category {
        id: 10,
        name: "Tech".to_string(),
        description: Some("Technology".to_string()),
        rank: 7,
        kind: CategoryKind::Public,
    });
    container.posts.add_row(Post {
        id: 100,
        title: "Binary refs".to_string(),
        content: "Lazy row access".to_string(),
        author_id: 1,
        category_id: 10,
    });
    container.posts.add_row(Post {
        id: 101,
        title: "Indexes".to_string(),
        content: "Lookup by author".to_string(),
        author_id: 1,
        category_id: 10,
    });

    let post = container.posts.get_by_id(100).expect("post should exist");
    let author = container.get_post_author(post).expect("post author should resolve");
    assert_eq!(author.username, "alice");
    let category = container
        .get_post_category(post)
        .expect("post category should resolve");
    assert_eq!(category.name, "Tech");

    let title_matches = container.posts.search_by_title("binary");
    assert_eq!(title_matches.len(), 1);
    assert_eq!(title_matches[0].id, 100);

    let name_matches = container.categorys.search_by_name("tech");
    assert_eq!(name_matches.len(), 1);
    assert_eq!(name_matches[0].id, 10);

    let description_matches = container.categorys.search_by_description("tech");
    assert_eq!(description_matches.len(), 1);
    assert_eq!(description_matches[0].name, "Tech");

    let rank_matches = container.categorys.search_by_rank(7);
    assert_eq!(rank_matches.len(), 1);
    assert_eq!(rank_matches[0].id, 10);

    let kind_matches = container.categorys.search_by_kind(CategoryKind::Public);
    assert_eq!(kind_matches.len(), 1);
    assert_eq!(kind_matches[0].id, 10);

    println!("    PASS");
}

fn test_container_load_from_csv_sources() {
    println!("  Testing container sources config CSV load...");

    let root = std::env::temp_dir().join(format!(
        "polygen_rust_sources_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();

    std::fs::write(
        root.join("users.csv"),
        "id,username,email,display_name\n1,alice,alice@example.com,Alice\n",
    )
    .unwrap();
    std::fs::write(
        root.join("categories.csv"),
        "id,name,description,rank,kind\n10,Tech,Technology posts,7,Public\n",
    )
    .unwrap();
    std::fs::write(
        root.join("posts.csv"),
        "id,title,content,author_id,category_id\n100,Binary refs,Lazy row access,1,10\n",
    )
    .unwrap();
    std::fs::write(
        root.join("comments.csv"),
        "id,post_id,author_id,content,parent_id\n1000,100,1,First comment,\n",
    )
    .unwrap();
    std::fs::write(root.join("tags.csv"), "id,name\n50,rust\n").unwrap();
    std::fs::write(root.join("post_tags.csv"), "post_id,tag_id\n100,50\n").unwrap();

    let mut container = SchemaContainer::new();
    container.load_from_csv(&root).unwrap();

    assert_eq!(container.users.count(), 1);
    assert_eq!(container.posts.count(), 1);
    assert_eq!(container.post_tags.count(), 1);
    assert_eq!(
        container.users.get_by_username("alice".to_string()).unwrap().id,
        1
    );
    assert_eq!(container.posts.get_by_author_id(1).len(), 1);
    assert_eq!(container.posts.search_by_title("binary").len(), 1);
    assert_eq!(container.categorys.search_by_description("tech").len(), 1);
    assert_eq!(container.categorys.search_by_kind(CategoryKind::Public).len(), 1);
    assert!(container.validate_all().is_valid());

    std::fs::remove_dir_all(&root).unwrap();

    println!("    PASS");
}

fn test_container_load_from_json_sources() {
    println!("  Testing container sources config JSON load...");

    let root = std::env::temp_dir().join(format!(
        "polygen_rust_json_sources_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();

    std::fs::write(
        root.join("users.json"),
        r#"[{"id":1,"username":"alice","email":"alice@example.com","display_name":"Alice"}]"#,
    )
    .unwrap();
    std::fs::write(
        root.join("categories.json"),
        r#"[{"id":10,"name":"Tech","description":"Technology posts","rank":7,"kind":"Public"}]"#,
    )
    .unwrap();
    std::fs::write(
        root.join("posts.json"),
        r#"[{"id":100,"title":"Binary refs","content":"Lazy row access","author_id":1,"category_id":10}]"#,
    )
    .unwrap();
    std::fs::write(
        root.join("comments.json"),
        r#"[{"id":1000,"post_id":100,"author_id":1,"content":"First comment","parent_id":null}]"#,
    )
    .unwrap();
    std::fs::write(root.join("tags.json"), r#"[{"id":50,"name":"rust"}]"#).unwrap();
    std::fs::write(
        root.join("post_tags.json"),
        r#"[{"post_id":100,"tag_id":50}]"#,
    )
    .unwrap();

    let mut container = SchemaContainer::new();
    container.load_from_json(&root).unwrap();

    assert_eq!(container.users.count(), 1);
    assert_eq!(container.posts.count(), 1);
    assert_eq!(container.post_tags.count(), 1);
    assert_eq!(
        container.users.get_by_username("alice".to_string()).unwrap().id,
        1
    );
    assert_eq!(container.posts.get_by_author_id(1).len(), 1);
    assert_eq!(container.posts.search_by_title("binary").len(), 1);
    assert_eq!(container.categorys.search_by_description("tech").len(), 1);
    assert_eq!(container.categorys.search_by_kind(CategoryKind::Public).len(), 1);
    assert!(container.validate_all().is_valid());

    std::fs::remove_dir_all(&root).unwrap();

    println!("    PASS");
}

fn test_enum_csv_loader() {
    println!("  Testing CSV enum name/numeric parsing...");

    let root = std::env::temp_dir().join(format!(
        "polygen_rust_enum_csv_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();

    let valid_path = root.join("categories_valid.csv");
    std::fs::write(
        &valid_path,
        "id,name,description,rank,kind\n10,Tech,Technology posts,7,Public\n11,Ops,Operations,8,2\n",
    )
    .unwrap();

    let rows = Category::load_csv(valid_path.to_str().unwrap()).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].kind, CategoryKind::Public);
    assert_eq!(rows[1].kind, CategoryKind::Internal);

    let invalid_path = root.join("categories_invalid.csv");
    std::fs::write(
        &invalid_path,
        "id,name,description,rank,kind\n12,Bad,Bad enum,9,Archived\n",
    )
    .unwrap();

    assert!(Category::load_csv(invalid_path.to_str().unwrap()).is_err());

    std::fs::remove_dir_all(&root).unwrap();

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
