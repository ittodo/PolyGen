// Test Case 03: Nested Namespaces
// Tests deeply nested namespace structures

#include <iostream>
#include <cassert>

#include "schema.hpp"
#include "schema_loaders.hpp"
#include "schema_container.hpp"

void test_deeply_nested_table() {
    std::cout << "  Testing deeply nested table (app::data::models::User)..." << std::endl;

    app::data::models::User user;
    user.id = 1;
    user.username = "testuser";

    assert(user.id == 1);
    assert(user.username == "testuser");

    std::cout << "    PASS" << std::endl;
}

void test_nested_enum() {
    std::cout << "  Testing nested enum (app::data::enums::Permission)..." << std::endl;

    app::data::enums::Permission perm = app::data::enums::Permission::Admin;
    assert(static_cast<int32_t>(perm) == 3);

    perm = app::data::enums::Permission::Read;
    assert(static_cast<int32_t>(perm) == 1);

    std::cout << "    PASS" << std::endl;
}

void test_cross_namespace_reference() {
    std::cout << "  Testing cross-namespace reference (UserService)..." << std::endl;

    app::services::UserService service;
    service.id = 1;
    service.target_user_id = 42;
    service.permission = app::data::enums::Permission::Write;

    assert(service.id == 1);
    assert(service.target_user_id == 42);
    assert(service.permission == app::data::enums::Permission::Write);

    std::cout << "    PASS" << std::endl;
}

void test_separate_namespace() {
    std::cout << "  Testing separate namespace (util::Config)..." << std::endl;

    util::Config config;
    config.key = "debug_mode";
    config.value = "true";

    assert(config.key == "debug_mode");
    assert(config.value == "true");

    std::cout << "    PASS" << std::endl;
}

void test_container_deeply_nested_table() {
    std::cout << "  Testing container access for deeply nested table..." << std::endl;

    schema_container::SchemaContainer container;

    app::data::models::User user;
    user.id = 7;
    user.username = "deep_user";
    container.users.add_row(user);

    assert(container.users.count() == 1);

    auto* found = container.users.get_by_id(7);
    assert(found != nullptr);
    assert(found->username == "deep_user");

    std::cout << "    PASS" << std::endl;
}

void test_binary_nested() {
    std::cout << "  Testing binary serialization with nested namespaces..." << std::endl;

    // Test UserService binary serialization (loaders generated for this type)
    app::services::UserService original;
    original.id = 999;
    original.target_user_id = 42;
    original.permission = app::data::enums::Permission::Admin;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_UserService(writer, original);
    }

    // Deserialize
    app::services::UserService loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_UserService(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.target_user_id == original.target_user_id);
    assert(loaded.permission == original.permission);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

void test_binary_deeply_nested_table() {
    std::cout << "  Testing binary serialization on deeply nested table..." << std::endl;

    app::data::models::User original;
    original.id = 7;
    original.username = "deep_user";

    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_User(writer, original);
    }

    app::data::models::User loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_User(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.username == original.username);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 03: Nested Namespaces ===" << std::endl;

    test_deeply_nested_table();
    test_nested_enum();
    test_cross_namespace_reference();
    test_separate_namespace();
    test_container_deeply_nested_table();
    test_binary_nested();
    test_binary_deeply_nested_table();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
