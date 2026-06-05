// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints with containers

#include <iostream>
#include <cassert>
#include <filesystem>
#include <fstream>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <stdexcept>
#include <unordered_map>
#include <vector>

#include "schema.hpp"
#include "schema_loaders.hpp"
#include "schema_container.hpp"
#include "schema_binary_refs.hpp"

using namespace test::indexes;

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

void append_u8(std::vector<uint8_t>& bytes, uint8_t value) {
    bytes.push_back(value);
}

void append_bool(std::vector<uint8_t>& bytes, bool value) {
    append_u8(bytes, value ? 1 : 0);
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

std::vector<uint8_t> field_u8(uint8_t value) {
    std::vector<uint8_t> bytes;
    append_u8(bytes, value);
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

void append_unique_string_index(std::vector<uint8_t>& bytes, const std::string& name, const std::string& key) {
    append_string(bytes, name);
    append_bool(bytes, true);
    append_i32(bytes, 1);
    append_string(bytes, key);
    append_i32(bytes, 0);
}

void append_group_u32_index(std::vector<uint8_t>& bytes, const std::string& name, uint32_t key) {
    append_string(bytes, name);
    append_bool(bytes, false);
    append_i32(bytes, 1);
    append_u32(bytes, key);
    append_i32(bytes, 1);
    append_i32(bytes, 0);
}

void append_search_header(std::vector<uint8_t>& bytes, const std::string& name, const std::string& mode, int32_t entry_count) {
    append_string(bytes, name);
    append_string(bytes, mode);
    append_i32(bytes, entry_count);
}

void append_search_string_hit(std::vector<uint8_t>& bytes, const std::string& key) {
    append_string(bytes, key);
    append_i32(bytes, 1);
    append_i32(bytes, 0);
}

void append_search_u8_hit(std::vector<uint8_t>& bytes, uint8_t key) {
    append_u8(bytes, key);
    append_i32(bytes, 1);
    append_i32(bytes, 0);
}

void append_search_i32_hit(std::vector<uint8_t>& bytes, int32_t key) {
    append_i32(bytes, key);
    append_i32(bytes, 1);
    append_i32(bytes, 0);
}

std::vector<uint8_t> make_category_binary_ref_section(
    int32_t row_kind = static_cast<int32_t>(CategoryKind::Public),
    int32_t search_kind = static_cast<int32_t>(CategoryKind::Public)) {
    auto row = make_row({
        field_u32(10),
        field_string("Tech"),
        field_string("Technology"),
        field_u8(7),
        field_i32(row_kind),
    });

    std::vector<uint8_t> bytes;
    append_string(bytes, "test.indexes.Category");
    append_i32(bytes, 1);
    append_i32(bytes, 0);

    append_i32(bytes, 2);
    append_unique_u32_index(bytes, "ById", 10);
    append_unique_string_index(bytes, "ByName", "Tech");

    append_i32(bytes, 4);
    append_search_header(bytes, "Name", "exact", 1);
    append_search_string_hit(bytes, "tech");
    append_search_header(bytes, "Description", "ngram", 3);
    append_search_string_hit(bytes, "te");
    append_search_string_hit(bytes, "ec");
    append_search_string_hit(bytes, "ch");
    append_search_header(bytes, "Rank", "exact", 1);
    append_search_u8_hit(bytes, 7);
    append_search_header(bytes, "Kind", "exact", 1);
    append_search_i32_hit(bytes, search_kind);

    append_i32(bytes, static_cast<int32_t>(row.size()));
    bytes.insert(bytes.end(), row.begin(), row.end());
    return bytes;
}

std::vector<uint8_t> make_post_binary_ref_section() {
    auto row = make_row({
        field_u32(100),
        field_string("Binary refs"),
        field_string("Lazy row access"),
        field_u32(1),
        field_u32(10),
    });

    std::vector<uint8_t> bytes;
    append_string(bytes, "test.indexes.Post");
    append_i32(bytes, 1);
    append_i32(bytes, 0);

    append_i32(bytes, 3);
    append_unique_u32_index(bytes, "ById", 100);
    append_group_u32_index(bytes, "ByAuthorId", 1);
    append_group_u32_index(bytes, "ByCategoryId", 10);

    append_i32(bytes, 1);
    append_search_header(bytes, "Title", "ngram", 5);
    append_search_string_hit(bytes, "bi");
    append_search_string_hit(bytes, "in");
    append_search_string_hit(bytes, "na");
    append_search_string_hit(bytes, "ar");
    append_search_string_hit(bytes, "ry");

    append_i32(bytes, static_cast<int32_t>(row.size()));
    bytes.insert(bytes.end(), row.begin(), row.end());
    return bytes;
}

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
    cat1.rank = 7;
    cat1.kind = CategoryKind::Public;

    Category cat2;
    cat2.id = 2;
    cat2.name = "Gaming";
    cat2.description = std::nullopt;
    cat2.rank = 3;
    cat2.kind = CategoryKind::Internal;

    container.categories.add_row(cat1);
    container.categories.add_row(cat2);

    auto* tech = container.categories.get_by_name("Technology");
    assert(tech != nullptr);
    assert(tech->id == 1);
    assert(tech->description.has_value());
    assert(tech->rank == 7);
    assert(tech->kind == CategoryKind::Public);

    auto* gaming = container.categories.get_by_name("Gaming");
    assert(gaming != nullptr);
    assert(!gaming->description.has_value());
    assert(gaming->rank == 3);
    assert(gaming->kind == CategoryKind::Internal);

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
    cat.rank = 1;
    cat.kind = CategoryKind::Public;
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

    // Verify FK navigation helpers resolve targets through the container
    auto* author = container.get_post_author(*found_post);
    assert(author != nullptr);
    assert(author->username == "author");

    auto* category = container.get_post_category(*found_post);
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
    cat.rank = 1;
    cat.kind = CategoryKind::Public;
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

    auto* first_link = posts_for_tag[0];
    auto* linked_post = container.get_post_tag_post(*first_link);
    assert(linked_post != nullptr);
    assert(linked_post->title == "Post");

    auto* linked_tag = container.get_post_tag_tag(*first_link);
    assert(linked_tag != nullptr);
    assert(linked_tag->name == "cpp");

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

void test_container_search() {
    std::cout << "  Testing container search indexes..." << std::endl;

    schema_container::SchemaContainer container;

    Category category;
    category.id = 10;
    category.name = "Tech";
    category.description = "Technology";
    category.rank = 7;
    category.kind = CategoryKind::Public;
    container.categories.add_row(category);

    Post post1;
    post1.id = 100;
    post1.title = "Binary refs";
    post1.content = "Lazy row access";
    post1.author_id = 1;
    post1.category_id = 10;
    container.posts.add_row(post1);

    Post post2;
    post2.id = 101;
    post2.title = "Indexes";
    post2.content = "Lookup by author";
    post2.author_id = 1;
    post2.category_id = 10;
    container.posts.add_row(post2);

    auto title_matches = container.posts.search_by_title("binary");
    assert(title_matches.size() == 1);
    assert(title_matches[0]->id == 100);

    auto name_matches = container.categories.search_by_name("tech");
    assert(name_matches.size() == 1);
    assert(name_matches[0]->id == 10);

    auto description_matches = container.categories.search_by_description("tech");
    assert(description_matches.size() == 1);
    assert(description_matches[0]->name == "Tech");

    auto rank_matches = container.categories.search_by_rank(7);
    assert(rank_matches.size() == 1);
    assert(rank_matches[0]->id == 10);

    auto kind_matches = container.categories.search_by_kind(CategoryKind::Public);
    assert(kind_matches.size() == 1);
    assert(kind_matches[0]->id == 10);

    auto* found_post = container.posts.get_by_id(100);
    assert(found_post != nullptr);
    assert(container.get_post_category(*found_post)->name == "Tech");

    std::cout << "    PASS" << std::endl;
}

void write_text_file(const std::filesystem::path& path, const std::string& body) {
    std::ofstream file(path);
    file << body;
}

void test_container_load_from_csv_sources() {
    std::cout << "  Testing container sources-config CSV load..." << std::endl;

    auto root = std::filesystem::temp_directory_path() / "polygen_cpp_sources_load";
    std::filesystem::remove_all(root);
    std::filesystem::create_directories(root);

    write_text_file(root / "users.csv",
        "id,username,email,display_name\n"
        "1,author,author@example.com,Author\n");
    write_text_file(root / "categories.csv",
        "id,name,description,rank,kind\n"
        "10,Tech,Technology,7,Public\n");
    write_text_file(root / "posts.csv",
        "id,title,content,author_id,category_id\n"
        "100,Binary refs,Lazy row access,1,10\n");
    write_text_file(root / "comments.csv",
        "id,post_id,author_id,content,parent_id\n"
        "200,100,1,Comment,\n");
    write_text_file(root / "tags.csv",
        "id,name\n"
        "300,cpp\n");
    write_text_file(root / "post_tags.csv",
        "post_id,tag_id\n"
        "100,300\n");

    schema_container::SchemaContainer container;
    container.load_from_csv(root);

    assert(container.users.count() == 1);
    assert(container.categories.count() == 1);
    assert(container.posts.count() == 1);
    assert(container.comments.count() == 1);
    assert(container.tags.count() == 1);
    assert(container.post_tags.count() == 1);

    auto* user = container.users.get_by_id(1);
    assert(user != nullptr);
    assert(user->username == "author");

    auto* category = container.categories.get_by_name("Tech");
    assert(category != nullptr);
    assert(category->rank == 7);
    assert(category->kind == CategoryKind::Public);

    auto title_matches = container.posts.search_by_title("binary");
    assert(title_matches.size() == 1);
    assert(title_matches[0]->id == 100);

    auto description_matches = container.categories.search_by_description("tech");
    assert(description_matches.size() == 1);

    auto kind_matches = container.categories.search_by_kind(CategoryKind::Public);
    assert(kind_matches.size() == 1);

    auto validation = container.validate_all();
    assert(validation.is_valid());

    std::filesystem::remove_all(root);
    std::cout << "    PASS" << std::endl;
}

void test_container_load_from_json_sources() {
    std::cout << "  Testing container sources-config JSON load..." << std::endl;

    auto root = std::filesystem::temp_directory_path() / "polygen_cpp_json_sources_load";
    std::filesystem::remove_all(root);
    std::filesystem::create_directories(root);

    write_text_file(root / "users.json",
        R"([{"id":1,"username":"author","email":"author@example.com","display_name":"Author"}])");
    write_text_file(root / "categories.json",
        R"([{"id":10,"name":"Tech","description":"Technology","rank":7,"kind":"Public"}])");
    write_text_file(root / "posts.json",
        R"([{"id":100,"title":"Binary refs","content":"Lazy row access","author_id":1,"category_id":10}])");
    write_text_file(root / "comments.json",
        R"([{"id":200,"post_id":100,"author_id":1,"content":"Comment","parent_id":null}])");
    write_text_file(root / "tags.json",
        R"([{"id":300,"name":"cpp"}])");
    write_text_file(root / "post_tags.json",
        R"([{"post_id":100,"tag_id":300}])");

    schema_container::SchemaContainer container;
    container.load_from_json(root);

    assert(container.users.count() == 1);
    assert(container.categories.count() == 1);
    assert(container.posts.count() == 1);
    assert(container.comments.count() == 1);
    assert(container.tags.count() == 1);
    assert(container.post_tags.count() == 1);

    auto* user = container.users.get_by_id(1);
    assert(user != nullptr);
    assert(user->username == "author");

    auto* category = container.categories.get_by_name("Tech");
    assert(category != nullptr);
    assert(category->rank == 7);
    assert(category->kind == CategoryKind::Public);

    auto title_matches = container.posts.search_by_title("binary");
    assert(title_matches.size() == 1);
    assert(title_matches[0]->id == 100);

    auto description_matches = container.categories.search_by_description("tech");
    assert(description_matches.size() == 1);

    auto kind_matches = container.categories.search_by_kind(CategoryKind::Public);
    assert(kind_matches.size() == 1);

    auto validation = container.validate_all();
    assert(validation.is_valid());

    std::filesystem::remove_all(root);
    std::cout << "    PASS" << std::endl;
}

void test_enum_csv_json_loaders() {
    std::cout << "  Testing enum CSV/JSON loader parsing..." << std::endl;

    std::stringstream csv(
        "id,name,description,rank,kind\n"
        "10,Tech,Technology,7,Public\n"
        "11,Ops,Operations,8,2\n");
    auto rows = polygen_loaders::load_Category_from_csv(csv);
    assert(rows.size() == 2);
    assert(rows[0].kind == CategoryKind::Public);
    assert(rows[1].kind == CategoryKind::Internal);

    bool invalid_csv_rejected = false;
    try {
        std::stringstream invalid_csv(
            "id,name,description,rank,kind\n"
            "12,Bad,Bad value,9,999\n");
        (void)polygen_loaders::load_Category_from_csv(invalid_csv);
    } catch (const std::runtime_error&) {
        invalid_csv_rejected = true;
    }
    assert(invalid_csv_rejected);

    auto json_name = polygen_loaders::from_json_Category(polygen::JsonParser::parse(
        R"({"id":20,"name":"Docs","description":"Public docs","rank":3,"kind":"Public"})"));
    assert(json_name.kind == CategoryKind::Public);

    auto json_numeric = polygen_loaders::from_json_Category(polygen::JsonParser::parse(
        R"({"id":21,"name":"Ops","description":"Internal ops","rank":4,"kind":2})"));
    assert(json_numeric.kind == CategoryKind::Internal);

    bool invalid_json_rejected = false;
    try {
        (void)polygen_loaders::from_json_Category(polygen::JsonParser::parse(
            R"({"id":22,"name":"Bad","description":"Bad value","rank":5,"kind":"Archived"})"));
    } catch (const std::runtime_error&) {
        invalid_json_rejected = true;
    }
    assert(invalid_json_rejected);

    std::cout << "    PASS" << std::endl;
}

void test_binary_ref_search() {
    std::cout << "  Testing binary ref search indexes..." << std::endl;

    auto category_bytes = make_category_binary_ref_section();
    auto category_doc = std::make_shared<polygen::BinaryDocument>(category_bytes);
    polygen::BinaryReader category_reader(category_bytes);
    auto categories = schema_binary_refs::test_indexes_CategoryRefTable::read(category_doc, category_reader);

    auto name_matches = categories.search_by_name("tech");
    assert(name_matches.size() == 1);
    assert(name_matches[0].id() == 10);

    auto description_matches = categories.search_by_description("tech");
    assert(description_matches.size() == 1);
    assert(description_matches[0].name() == "Tech");

    auto rank_matches = categories.search_by_rank(7);
    assert(rank_matches.size() == 1);
    assert(rank_matches[0].id() == 10);

    auto kind_matches = categories.search_by_kind(CategoryKind::Public);
    assert(kind_matches.size() == 1);
    assert(kind_matches[0].id() == 10);

    auto post_bytes = make_post_binary_ref_section();
    auto post_doc = std::make_shared<polygen::BinaryDocument>(post_bytes);
    polygen::BinaryReader post_reader(post_bytes);
    auto posts = schema_binary_refs::test_indexes_PostRefTable::read(post_doc, post_reader);

    auto title_matches = posts.search_by_title("binary");
    assert(title_matches.size() == 1);
    assert(title_matches[0].id() == 100);
    assert(title_matches[0].title() == "Binary refs");

    std::cout << "    PASS" << std::endl;
}

void test_binary_ref_save_roundtrip() {
    std::cout << "  Testing binary ref save/open roundtrip..." << std::endl;

    schema_container::SchemaContainer container;

    User user;
    user.id = 1;
    user.username = "alice";
    user.email = "alice@example.com";
    user.display_name = "Alice";
    container.users.add_row(user);

    Category category;
    category.id = 10;
    category.name = "Tech";
    category.description = std::string("Technology");
    category.rank = 7;
    category.kind = CategoryKind::Public;
    container.categories.add_row(category);

    Post post;
    post.id = 100;
    post.title = "Binary Reference Guide";
    post.content = "body";
    post.author_id = 1;
    post.category_id = 10;
    container.posts.add_row(post);

    Tag tag;
    tag.id = 200;
    tag.name = "featured";
    container.tags.add_row(tag);

    PostTag post_tag;
    post_tag.post_id = 100;
    post_tag.tag_id = 200;
    container.post_tags.add_row(post_tag);

    auto bytes = schema_binary_refs::BinaryRefContext::save_binary(container);
    auto refs = schema_binary_refs::BinaryRefContext::open_bytes(bytes);

    auto user_ref = refs.users.get_by_username("alice");
    assert(user_ref.has_value());
    assert(user_ref->email() == "alice@example.com");

    auto category_ref = refs.categories.get_by_name("Tech");
    assert(category_ref.has_value());
    assert(category_ref->kind() == CategoryKind::Public);
    assert(refs.categories.search_by_description("technology").size() == 1);
    assert(refs.categories.search_by_rank(7).size() == 1);
    assert(refs.categories.search_by_kind(CategoryKind::Public).size() == 1);

    auto posts = refs.posts.find_by_author_id(1);
    assert(posts.size() == 1);
    assert(posts[0].title() == "Binary Reference Guide");
    assert(refs.posts.search_by_title("binary").size() == 1);
    assert(refs.post_tags.count() == 1);

    std::cout << "    PASS" << std::endl;
}

void test_binary_ref_rejects_invalid_enum() {
    std::cout << "  Testing binary ref invalid enum rejection..." << std::endl;

    auto row_invalid_bytes = make_category_binary_ref_section(999);
    auto row_invalid_doc = std::make_shared<polygen::BinaryDocument>(row_invalid_bytes);
    polygen::BinaryReader row_invalid_reader(row_invalid_bytes);
    auto row_invalid_categories =
        schema_binary_refs::test_indexes_CategoryRefTable::read(row_invalid_doc, row_invalid_reader);

    auto kind_matches = row_invalid_categories.search_by_kind(CategoryKind::Public);
    assert(kind_matches.size() == 1);

    bool lazy_getter_rejected = false;
    try {
        (void)kind_matches[0].kind();
    } catch (const std::runtime_error&) {
        lazy_getter_rejected = true;
    }
    assert(lazy_getter_rejected);

    bool search_key_rejected = false;
    try {
        auto search_invalid_bytes = make_category_binary_ref_section(
            static_cast<int32_t>(CategoryKind::Public),
            999);
        auto search_invalid_doc = std::make_shared<polygen::BinaryDocument>(search_invalid_bytes);
        polygen::BinaryReader search_invalid_reader(search_invalid_bytes);
        (void)schema_binary_refs::test_indexes_CategoryRefTable::read(
            search_invalid_doc,
            search_invalid_reader);
    } catch (const std::runtime_error&) {
        search_key_rejected = true;
    }
    assert(search_key_rejected);

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
    test_container_search();
    test_container_load_from_csv_sources();
    test_container_load_from_json_sources();
    test_enum_csv_json_loaders();
    test_binary_ref_search();
    test_binary_ref_save_roundtrip();
    test_binary_ref_rejects_invalid_enum();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
