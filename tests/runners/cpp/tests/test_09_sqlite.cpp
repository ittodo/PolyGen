// Test Case 09: SQLite Schema Types
// Tests the generated schema types for SQLite tables
// Note: This test does not use actual SQLite - it only validates types

#include <iostream>
#include <cassert>
#include <optional>

#include "schema.hpp"
#include "schema_loaders.hpp"

using namespace test::sqlite;

void test_user_creation() {
    std::cout << "  Testing User creation..." << std::endl;

    User user;
    user.id = 1;
    user.name = "TestUser";
    user.email = "test@example.com";
    user.created_at = 1700000000;

    assert(user.id == 1);
    assert(user.name == "TestUser");
    assert(user.email.has_value());
    assert(user.email.value() == "test@example.com");
    assert(user.created_at == 1700000000);

    std::cout << "    PASS" << std::endl;
}

void test_post_creation() {
    std::cout << "  Testing Post creation..." << std::endl;

    Post post;
    post.id = 1;
    post.user_id = 1;
    post.title = "Test Title";
    post.content = "Test Content";

    assert(post.id == 1);
    assert(post.user_id == 1);
    assert(post.title == "Test Title");
    assert(post.content.has_value());
    assert(post.content.value() == "Test Content");

    std::cout << "    PASS" << std::endl;
}

void test_comment_creation() {
    std::cout << "  Testing Comment creation..." << std::endl;

    Comment comment;
    comment.id = 1;
    comment.post_id = 1;
    comment.user_id = 2;
    comment.content = "Test Comment";

    assert(comment.id == 1);
    assert(comment.post_id == 1);
    assert(comment.user_id == 2);
    assert(comment.content == "Test Comment");

    std::cout << "    PASS" << std::endl;
}

void test_optional_fields() {
    std::cout << "  Testing optional fields..." << std::endl;

    // User with no email
    User user;
    user.id = 2;
    user.name = "Bob";
    user.created_at = 1700000001;

    assert(!user.email.has_value());

    // Post with no content
    Post post;
    post.id = 2;
    post.user_id = 1;
    post.title = "No Content Post";

    assert(!post.content.has_value());

    std::cout << "    PASS" << std::endl;
}

void test_post_status_enum() {
    std::cout << "  Testing PostStatus enum..." << std::endl;

    assert(static_cast<int32_t>(PostStatus::Draft) == 0);
    assert(static_cast<int32_t>(PostStatus::Published) == 1);
    assert(static_cast<int32_t>(PostStatus::Archived) == 2);

    PostStatus status = PostStatus::Published;
    assert(status == PostStatus::Published);

    std::cout << "    PASS" << std::endl;
}

void test_binary_serialization() {
    std::cout << "  Testing binary serialization..." << std::endl;

    User original;
    original.id = 12345;
    original.name = "Binary Test User";
    original.email = "binary@test.com";
    original.created_at = 1700000123;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_User(writer, original);
    }

    // Deserialize
    User loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_User(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.name == original.name);
    assert(loaded.email.has_value());
    assert(loaded.email.value() == original.email.value());
    assert(loaded.created_at == original.created_at);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

void test_binary_optional_null() {
    std::cout << "  Testing binary serialization with null optional..." << std::endl;

    User original;
    original.id = 99;
    original.name = "No Email";
    // email is std::nullopt by default
    original.created_at = 1700099999;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_User(writer, original);
    }

    // Deserialize
    User loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_User(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.name == original.name);
    assert(!loaded.email.has_value());
    assert(loaded.created_at == original.created_at);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 09: SQLite Schema Types ===" << std::endl;

    test_user_creation();
    test_post_creation();
    test_comment_creation();
    test_optional_fields();
    test_post_status_enum();
    test_binary_serialization();
    test_binary_optional_null();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
