// Test Case 02: Cross-namespace References
// Tests referencing types from different namespaces

#include <iostream>
#include <cassert>
#include <cmath>

#include "schema.hpp"
#include "schema_loaders.hpp"

void test_common_embed() {
    std::cout << "  Testing common embed (Position)..." << std::endl;

    common::Position pos;
    pos.x = 1.0f;
    pos.y = 2.0f;
    pos.z = 3.0f;

    assert(pos.x == 1.0f);
    assert(pos.y == 2.0f);
    assert(pos.z == 3.0f);

    std::cout << "    PASS" << std::endl;
}

void test_common_enum() {
    std::cout << "  Testing common enum (Status)..." << std::endl;

    common::Status status = common::Status::Active;
    assert(status == common::Status::Active);

    status = common::Status::Inactive;
    assert(static_cast<int32_t>(status) == 1);

    std::cout << "    PASS" << std::endl;
}

void test_player_cross_namespace() {
    std::cout << "  Testing Player with cross-namespace types..." << std::endl;

    game::Player player;
    player.id = 1;
    player.name = "Hero";
    player.position.x = 100.0f;
    player.position.y = 50.0f;
    player.position.z = 0.0f;
    player.status = common::Status::Active;

    assert(player.id == 1);
    assert(player.name == "Hero");
    assert(player.position.x == 100.0f);
    assert(player.status == common::Status::Active);

    std::cout << "    PASS" << std::endl;
}

void test_npc_cross_namespace() {
    std::cout << "  Testing NPC with cross-namespace types..." << std::endl;

    game::NPC npc;
    npc.id = 100;
    npc.display_name = "Merchant";
    npc.spawn_point.x = 50.0f;
    npc.spawn_point.y = 50.0f;
    npc.spawn_point.z = 0.0f;
    npc.ai_state = common::Status::Active;

    assert(npc.id == 100);
    assert(npc.display_name == "Merchant");
    assert(npc.spawn_point.x == 50.0f);

    std::cout << "    PASS" << std::endl;
}

void test_binary_cross_namespace() {
    std::cout << "  Testing binary serialization with cross-namespace types..." << std::endl;

    game::Player original;
    original.id = 42;
    original.name = "Test Player";
    original.position.x = 123.456f;
    original.position.y = 789.012f;
    original.position.z = 345.678f;
    original.status = common::Status::Inactive;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_Player(writer, original);
    }

    // Deserialize
    game::Player loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_Player(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.name == original.name);
    assert(std::abs(loaded.position.x - original.position.x) < 0.001f);
    assert(std::abs(loaded.position.y - original.position.y) < 0.001f);
    assert(std::abs(loaded.position.z - original.position.z) < 0.001f);
    assert(loaded.status == original.status);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 02: Cross-namespace References ===" << std::endl;

    test_common_embed();
    test_common_enum();
    test_player_cross_namespace();
    test_npc_cross_namespace();
    test_binary_cross_namespace();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
