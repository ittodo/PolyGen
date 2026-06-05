// Test Case 11: Relations and composite indexes
// Tests reverse relation aliases, FK navigation helpers, and composite indexes.

use polygen_test::schema::examples_relations::{Post, PostStatus, User};
use polygen_test::schema_binary_refs::{read_binary_ref_document, write_binary_ref_document};
use polygen_test::schema_container::container::SchemaContainer;

fn main() {
    println!("=== Test Case 11: Relations and Composite Indexes ===");

    test_composite_index_and_navigation();

    println!("=== All tests passed! ===");
}

fn test_composite_index_and_navigation() {
    println!("  Testing composite index and relation navigation...");

    let mut container = SchemaContainer::new();

    container.users.add_row(User {
        id: 1,
        email: "author@example.com".to_string(),
        display_name: "Author".to_string(),
    });

    container.posts.add_row(Post {
        id: 10,
        author_id: 1,
        status: PostStatus::Published,
        title: "Relations".to_string(),
    });

    let post = container.posts.get_by_id(10).expect("post should exist");
    let composite = container
        .posts
        .get_by_author_id_status((1, PostStatus::Published));
    assert_eq!(composite.len(), 1);
    assert_eq!(composite[0].id, 10);

    let author = container
        .get_post_author(post)
        .expect("post author should resolve");
    assert_eq!(author.email, "author@example.com");

    let posts = container.find_user_posts(author);
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].title, "Relations");

    container.posts.add_row(Post {
        id: 11,
        author_id: 1,
        status: PostStatus::Draft,
        title: "Draft".to_string(),
    });

    let bytes = write_binary_ref_document(&container).unwrap();
    let document = read_binary_ref_document(bytes).unwrap();
    let binary_composite = document
        .posts
        .get_by_author_id_status((1, PostStatus::Published));
    assert_eq!(binary_composite.len(), 1);
    assert_eq!(binary_composite[0].title().unwrap(), "Relations");
    assert!(document
        .posts
        .get_by_author_id_status((2, PostStatus::Published))
        .is_empty());

    println!("    PASS");
}
