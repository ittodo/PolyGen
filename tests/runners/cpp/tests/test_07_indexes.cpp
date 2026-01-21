// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints with containers

#include <iostream>
#include <cassert>

#include "schema.hpp"
#include "schema_loaders.hpp"
#include "schema_container.hpp"

using namespace test::indexes;

void test_user_unique_index() {
    std::cout << "  Testing User table with unique indexes..." << std::endl;

    schema_container::SchemaContainer container;

    // Add users
    User user1;
    user1.id = 1;
    user1.username = "john";
    user1.email = "john@example.com";
    user1.display_name = "John Doe";

    User user2;
    user2.id = 2;
    user2.username = "jane";
    user2.email = "jane@example.com";
    user2.display_name = "Jane Doe";

    container.users.add_row(user1);
    container.users.add_row(user2);

    assert(container.users.count() == 2);

    // Test unique index lookups
    auto* found_by_id = container.users.get_by_id(1);
    assert(found_by_id != nullptr);
    assert(found_by_id->username == "john");

    auto* found_by_username = container.users.get_by_username("jane");
    assert(found_by_username != nullptr);
    assert(found_by_username->id == 2);

    auto* found_by_email = container.users.get_by_email("john@example.com");
    assert(found_by_email != nullptr);
    assert(found_by_email->display_name == "John Doe");

    // Test not found
    auto* not_found = container.users.get_by_id(999);
    assert(not_found == nullptr);

    std::cout << "    PASS" << std::endl;
}

void test_category_simple_index() {
    std::cout << "  Testing Category with simple index..." << std::endl;

    schema_container::SchemaContainer container;

    Category cat1;
    cat1.id = 1;
    cat1.name = "Technology";
    cat1.description = "Tech stuff";

    Category cat2;
    cat2.id = 2;
    cat2.name = "Gaming";
    cat2.description = std::nullopt;

    container.categories.add_row(cat1);
    container.categories.add_row(cat2);

    auto* tech = container.categories.get_by_name("Technology");
    assert(tech != nullptr);
    assert(tech->id == 1);
    assert(tech->description.has_value());

    auto* gaming = container.categories.get_by_name("Gaming");
    assert(gaming != nullptr);
    assert(!gaming->description.has_value());

    std::cout << "    PASS" << std::endl;
}

void test_post_with_foreign_keys() {
    std::cout << "  Testing Post with foreign keys..." << std::endl;

    schema_container::SchemaContainer container;

    // Add user
    User user;
    user.id = 1;
    user.username = "author";
    user.email = "author@example.com";
    user.display_name = "Author";
    container.users.add_row(user);

    // Add category
    Category cat;
    cat.id = 1;
    cat.name = "Blog";
    cat.description = std::nullopt;
    container.categories.add_row(cat);

    // Add post
    Post post;
    post.id = 1;
    post.title = "First Post";
    post.content = "Hello World";
    post.author_id = 1;  // FK to User
    post.category_id = 1;  // FK to Category
    container.posts.add_row(post);

    auto* found_post = container.posts.get_by_id(1);
    assert(found_post != nullptr);
    assert(found_post->title == "First Post");
    assert(found_post->author_id == 1);
    assert(found_post->category_id == 1);

    // Verify FK targets exist
    auto* author = container.users.get_by_id(found_post->author_id);
    assert(author != nullptr);
    assert(author->username == "author");

    auto* category = container.categories.get_by_id(found_post->category_id);
    assert(category != nullptr);
    assert(category->name == "Blog");

    std::cout << "    PASS" << std::endl;
}

void test_junction_table() {
    std::cout << "  Testing PostTag junction table..." << std::endl;

    schema_container::SchemaContainer container;

    // Setup
    User user;
    user.id = 1;
    user.username = "author";
    user.email = "a@b.com";
    user.display_name = "A";
    container.users.add_row(user);

    Category cat;
    cat.id = 1;
    cat.name = "Blog";
    cat.description = std::nullopt;
    container.categories.add_row(cat);

    Post post;
    post.id = 1;
    post.title = "Post";
    post.content = "Content";
    post.author_id = 1;
    post.category_id = 1;
    container.posts.add_row(post);

    Tag tag1, tag2;
    tag1.id = 1;
    tag1.name = "cpp";
    tag2.id = 2;
    tag2.name = "polygen";
    container.tags.add_row(tag1);
    container.tags.add_row(tag2);

    // Add post-tag relationships
    PostTag pt1, pt2;
    pt1.post_id = 1;
    pt1.tag_id = 1;
    pt2.post_id = 1;
    pt2.tag_id = 2;
    container.post_tags.add_row(pt1);
    container.post_tags.add_row(pt2);

    assert(container.post_tags.count() == 2);

    // Lookup by post_id
    const auto& tags_for_post = container.post_tags.get_by_post_id(1);
    assert(tags_for_post.size() == 2);

    // Lookup by tag_id
    const auto& posts_for_tag = container.post_tags.get_by_tag_id(1);
    assert(posts_for_tag.size() == 1);
    assert(posts_for_tag[0]->post_id == 1);

    std::cout << "    PASS" << std::endl;
}

void test_iterator() {
    std::cout << "  Testing container iteration..." << std::endl;

    schema_container::SchemaContainer container;

    for (int i = 1; i <= 5; i++) {
        User u;
        u.id = i;
        u.username = "user" + std::to_string(i);
        u.email = u.username + "@test.com";
        u.display_name = "User " + std::to_string(i);
        container.users.add_row(u);
    }

    int count = 0;
    for (const auto& user : container.users) {
        count++;
        assert(user.id >= 1 && user.id <= 5);
    }
    assert(count == 5);

    std::cout << "    PASS" << std::endl;
}

void test_clear() {
    std::cout << "  Testing container clear..." << std::endl;

    schema_container::SchemaContainer container;

    User u;
    u.id = 1;
    u.username = "test";
    u.email = "test@test.com";
    u.display_name = "Test";
    container.users.add_row(u);

    assert(container.users.count() == 1);

    container.users.clear();
    assert(container.users.count() == 0);
    assert(container.users.get_by_id(1) == nullptr);

    std::cout << "    PASS" << std::endl;
}

int main() {
    std::cout << "=== Test Case 07: Indexes ===" << std::endl;

    test_user_unique_index();
    test_category_simple_index();
    test_post_with_foreign_keys();
    test_junction_table();
    test_iterator();
    test_clear();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
