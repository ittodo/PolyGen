// Test Case 11: Relations and composite indexes
// Tests reverse relation aliases, FK navigation helpers, and composite indexes.

#include <cassert>
#include <iostream>
#include <memory>
#include <optional>
#include <tuple>
#include <vector>

#include "schema.hpp"
#include "schema_binary_refs.hpp"
#include "schema_container.hpp"

using namespace examples::relations;

void append_i32(std::vector<uint8_t>& bytes, int32_t value) {
    auto raw = static_cast<uint32_t>(value);
    bytes.push_back(static_cast<uint8_t>(raw & 0xFF));
    bytes.push_back(static_cast<uint8_t>((raw >> 8) & 0xFF));
    bytes.push_back(static_cast<uint8_t>((raw >> 16) & 0xFF));
    bytes.push_back(static_cast<uint8_t>((raw >> 24) & 0xFF));
}

void append_u32(std::vector<uint8_t>& bytes, uint32_t value) {
    append_i32(bytes, static_cast<int32_t>(value));
}

void append_bool(std::vector<uint8_t>& bytes, bool value) {
    bytes.push_back(value ? 1 : 0);
}

void append_string(std::vector<uint8_t>& bytes, const std::string& value) {
    append_i32(bytes, static_cast<int32_t>(value.size()));
    bytes.insert(bytes.end(), value.begin(), value.end());
}

std::vector<uint8_t> field_u32(uint32_t value) {
    std::vector<uint8_t> bytes;
    append_u32(bytes, value);
    return bytes;
}

std::vector<uint8_t> field_i32(int32_t value) {
    std::vector<uint8_t> bytes;
    append_i32(bytes, value);
    return bytes;
}

std::vector<uint8_t> field_string(const std::string& value) {
    std::vector<uint8_t> bytes;
    append_string(bytes, value);
    return bytes;
}

std::vector<uint8_t> make_row(const std::vector<std::optional<std::vector<uint8_t>>>& fields) {
    std::vector<int32_t> offsets;
    std::vector<uint8_t> payload;
    int32_t cursor = static_cast<int32_t>(sizeof(int32_t) + fields.size() * sizeof(int32_t));

    for (const auto& field : fields) {
        if (!field.has_value()) {
            offsets.push_back(-1);
            continue;
        }
        offsets.push_back(cursor);
        payload.insert(payload.end(), field->begin(), field->end());
        cursor += static_cast<int32_t>(field->size());
    }

    std::vector<uint8_t> row;
    append_i32(row, static_cast<int32_t>(fields.size()));
    for (auto offset : offsets) append_i32(row, offset);
    row.insert(row.end(), payload.begin(), payload.end());
    return row;
}

void append_unique_u32_index(std::vector<uint8_t>& bytes, const std::string& name, uint32_t key) {
    append_string(bytes, name);
    append_bool(bytes, true);
    append_i32(bytes, 1);
    append_u32(bytes, key);
    append_i32(bytes, 0);
}

void append_group_string_index(std::vector<uint8_t>& bytes, const std::string& name, const std::string& key) {
    append_string(bytes, name);
    append_bool(bytes, false);
    append_i32(bytes, 1);
    append_string(bytes, key);
    append_i32(bytes, 1);
    append_i32(bytes, 0);
}

std::vector<uint8_t> make_post_binary_ref_section() {
    auto row = make_row({
        field_u32(10),
        field_u32(1),
        field_i32(static_cast<int32_t>(PostStatus::Published)),
        field_string("Relations"),
    });

    std::vector<uint8_t> bytes;
    append_string(bytes, "examples.relations.Post");
    append_i32(bytes, 1);
    append_i32(bytes, 0);

    append_i32(bytes, 2);
    append_unique_u32_index(bytes, "ById", 10);
    append_group_string_index(bytes, "ByAuthorIdStatus", "[1,2]");

    append_i32(bytes, 0);

    append_i32(bytes, static_cast<int32_t>(row.size()));
    bytes.insert(bytes.end(), row.begin(), row.end());
    return bytes;
}

void test_composite_index_and_navigation() {
    std::cout << "  Testing composite index and relation navigation..." << std::endl;

    schema_container::SchemaContainer container;

    User author;
    author.id = 1;
    author.email = "author@example.com";
    author.display_name = "Author";
    container.users.add_row(author);

    Post post;
    post.id = 10;
    post.author_id = 1;
    post.status = PostStatus::Published;
    post.title = "Relations";
    container.posts.add_row(post);

    auto* found_post = container.posts.get_by_id(10);
    assert(found_post != nullptr);

    auto composite = container.posts.get_ByAuthorIdStatus(
        std::make_tuple(static_cast<uint32_t>(1), PostStatus::Published));
    assert(composite.size() == 1);
    assert(composite[0]->id == 10);

    auto* found_author = container.get_post_author(*found_post);
    assert(found_author != nullptr);
    assert(found_author->email == "author@example.com");

    auto posts = container.find_user_posts(*found_author);
    assert(posts.size() == 1);
    assert(posts[0]->title == "Relations");

    auto post_bytes = make_post_binary_ref_section();
    auto post_doc = std::make_shared<polygen::BinaryDocument>(post_bytes);
    polygen::BinaryReader post_reader(post_bytes);
    auto post_refs = schema_binary_refs::examples_relations_PostRefTable::read(post_doc, post_reader);
    auto binary_composite = post_refs.get_by_author_id_status(
        std::make_tuple(static_cast<uint32_t>(1), PostStatus::Published));
    assert(binary_composite.size() == 1);
    assert(binary_composite[0].id() == 10);
    assert(binary_composite[0].title() == "Relations");
    assert(post_refs
               .get_by_author_id_status(std::make_tuple(static_cast<uint32_t>(2), PostStatus::Published))
               .empty());

    std::cout << "    PASS" << std::endl;
}

int main() {
    std::cout << "=== Test Case 11: Relations and Composite Indexes ===" << std::endl;

    test_composite_index_and_navigation();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
