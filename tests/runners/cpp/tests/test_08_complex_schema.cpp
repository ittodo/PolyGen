// Test Case 08: Complex Schema
// Comprehensive test combining all features

#include <iostream>
#include <cassert>
#include <cmath>

#include "schema.hpp"
#include "schema_loaders.hpp"
#include "schema_container.hpp"

void test_common_types() {
    std::cout << "  Testing common types (Vec2, Vec3, Color, Element)..." << std::endl;

    game::common::Vec2 v2;
    v2.x = 1.0f;
    v2.y = 2.0f;
    assert(v2.x == 1.0f);
    assert(v2.y == 2.0f);

    game::common::Vec3 v3;
    v3.x = 1.0f;
    v3.y = 2.0f;
    v3.z = 3.0f;
    assert(v3.z == 3.0f);

    game::common::Color color;
    color.r = 255;
    color.g = 128;
    color.b = 64;
    color.a = 255;
    assert(color.r == 255);
    assert(color.g == 128);

    game::common::Element elem = game::common::Element::Fire;
    assert(static_cast<int32_t>(elem) == 1);

    std::cout << "    PASS" << std::endl;
}

void test_character_types() {
    std::cout << "  Testing character types (Stats, Player, NPC)..." << std::endl;

    // Stats
    game::character::Stats stats;
    stats.hp = 100;
    stats.max_hp = 100;
    stats.mp = 50;
    stats.max_mp = 50;
    stats.strength = 10;
    stats.agility = 8;
    stats.intelligence = 5;
    stats.vitality = 12;

    // Player
    game::character::Player player;
    player.id = 1;
    player.name = "Hero";
    player.level = 10;
    player.experience = 5000;
    player.stats = stats;
    player.position.x = 100.0f;
    player.position.y = 50.0f;
    player.position.z = 0.0f;
    player.status = game::character::Player::Status::Online;
    player.guild_id = 1;

    assert(player.name == "Hero");
    assert(player.level == 10);
    assert(player.stats.strength == 10);
    assert(player.guild_id.has_value());
    assert(*player.guild_id == 1);

    // NPC
    game::character::NPC npc;
    npc.id = 1;
    npc.name = "Merchant";
    npc.title = "Item Seller";
    npc.stats = stats;
    npc.spawn_position.x = 50.0f;
    npc.spawn_position.y = 50.0f;
    npc.spawn_position.z = 0.0f;
    npc.ai_type = game::character::NPC::AIType::Friendly;

    game::character::DialogOption opt1, opt2;
    opt1.text = "Hello!";
    opt1.next_dialog_id = 2;
    opt2.text = "Goodbye!";
    opt2.next_dialog_id = std::nullopt;
    npc.dialog_options = {opt1, opt2};

    assert(npc.title.has_value());
    assert(*npc.title == "Item Seller");
    assert(npc.dialog_options.size() == 2);
    assert(npc.dialog_options[0].next_dialog_id.has_value());
    assert(!npc.dialog_options[1].next_dialog_id.has_value());

    std::cout << "    PASS" << std::endl;
}

void test_item_types() {
    std::cout << "  Testing item types (Item, Weapon, Armor)..." << std::endl;

    // Item
    game::item::Item item;
    item.id = 1;
    item.name = "Iron Sword";
    item.description = "A basic sword";
    item.rarity = game::item::Rarity::Common;
    item.sell_price = 100;
    item.max_stack = 1;
    item.icon = "sword_01";
    item.item_type = game::item::Item::ItemType::Weapon;

    assert(item.name == "Iron Sword");
    assert(item.rarity == game::item::Rarity::Common);

    // Weapon
    game::item::Weapon weapon;
    weapon.item_id = 1;
    weapon.damage_min = 10;
    weapon.damage_max = 15;
    weapon.attack_speed = 1.2f;
    weapon.element = game::common::Element::Physical;
    weapon.equip_slot = game::character::EquipSlot::MainHand;

    game::item::BonusStat bonus;
    bonus.stat_name = "Strength";
    bonus.value = 5;
    weapon.bonus_stats = {bonus};

    assert(weapon.damage_min == 10);
    assert(weapon.bonus_stats.size() == 1);
    assert(weapon.bonus_stats[0].value == 5);

    // Armor
    game::item::Armor armor;
    armor.item_id = 2;
    armor.defense = 20;
    armor.magic_defense = 10;
    armor.equip_slot = game::character::EquipSlot::Body;

    game::item::Resistance res1, res2;
    res1.element = game::common::Element::Fire;
    res1.value = 10;
    res2.element = game::common::Element::Ice;
    res2.value = -5;  // weakness
    armor.resistances = {res1, res2};

    assert(armor.defense == 20);
    assert(armor.resistances.size() == 2);
    assert(armor.resistances[1].value == -5);

    std::cout << "    PASS" << std::endl;
}

void test_inventory_system() {
    std::cout << "  Testing inventory system..." << std::endl;

    game::inventory::InventorySlot slot;
    slot.id = 1;
    slot.player_id = 1;
    slot.item_id = 1;
    slot.slot_index = 0;
    slot.quantity = 5;

    game::inventory::Enhancement enh;
    enh.level = 3;
    enh.bonus_value = 15;
    slot.enhancement = enh;

    assert(slot.quantity == 5);
    assert(slot.enhancement.has_value());
    assert(slot.enhancement->level == 3);

    game::inventory::Equipment equip;
    equip.player_id = 1;
    equip.slot = game::character::EquipSlot::MainHand;
    equip.item_id = 1;

    assert(equip.slot == game::character::EquipSlot::MainHand);

    std::cout << "    PASS" << std::endl;
}

void test_social_system() {
    std::cout << "  Testing social system (Guild, GuildMember, Friendship)..." << std::endl;

    game::social::Guild guild;
    guild.id = 1;
    guild.name = "Heroes";
    guild.tag = "HRO";
    guild.leader_id = 1;
    guild.level = 5;
    guild.emblem_color.r = 255;
    guild.emblem_color.g = 215;
    guild.emblem_color.b = 0;
    guild.emblem_color.a = 255;
    guild.created_at = 1640000000;

    assert(guild.name == "Heroes");
    assert(guild.tag == "HRO");
    assert(guild.emblem_color.r == 255);

    game::social::GuildMember member;
    member.guild_id = 1;
    member.player_id = 1;
    member.rank = game::social::GuildMember::Rank::Leader;
    member.joined_at = 1640000000;

    assert(member.rank == game::social::GuildMember::Rank::Leader);

    game::social::Friendship friendship;
    friendship.player_a_id = 1;
    friendship.player_b_id = 2;
    friendship.since = 1641000000;

    assert(friendship.player_a_id == 1);

    std::cout << "    PASS" << std::endl;
}

void test_container_integration() {
    std::cout << "  Testing container integration..." << std::endl;

    schema_container::SchemaContainer container;

    // Add player
    game::character::Player player;
    player.id = 1;
    player.name = "TestPlayer";
    player.level = 50;
    player.experience = 100000;
    player.stats.hp = 500;
    player.stats.max_hp = 500;
    player.stats.mp = 200;
    player.stats.max_mp = 200;
    player.stats.strength = 50;
    player.stats.agility = 40;
    player.stats.intelligence = 30;
    player.stats.vitality = 60;
    player.position.x = 0;
    player.position.y = 0;
    player.position.z = 0;
    player.status = game::character::Player::Status::Online;
    player.guild_id = std::nullopt;

    container.players.add_row(player);

    // Add item
    game::item::Item item;
    item.id = 1;
    item.name = "Legendary Sword";
    item.description = "A sword of legends";
    item.rarity = game::item::Rarity::Legendary;
    item.sell_price = 10000;
    item.max_stack = 1;
    item.icon = "legendary_sword";
    item.item_type = game::item::Item::ItemType::Weapon;

    container.items.add_row(item);

    // Verify lookups
    auto* found_player = container.players.get_by_name("TestPlayer");
    assert(found_player != nullptr);
    assert(found_player->level == 50);

    auto* found_item = container.items.get_by_name("Legendary Sword");
    assert(found_item != nullptr);
    assert(found_item->rarity == game::item::Rarity::Legendary);

    std::cout << "    PASS" << std::endl;
}

void test_binary_complex() {
    std::cout << "  Testing binary serialization of complex types..." << std::endl;

    game::character::Player original;
    original.id = 999;
    original.name = "SerializationTest";
    original.level = 99;
    original.experience = 9999999;
    original.stats.hp = 9999;
    original.stats.max_hp = 9999;
    original.stats.mp = 4999;
    original.stats.max_mp = 4999;
    original.stats.strength = 255;
    original.stats.agility = 255;
    original.stats.intelligence = 255;
    original.stats.vitality = 255;
    original.position.x = 123.456f;
    original.position.y = 789.012f;
    original.position.z = 345.678f;
    original.status = game::character::Player::Status::InBattle;
    original.guild_id = 42;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_Player(writer, original);
    }

    // Deserialize
    game::character::Player loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_Player(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.name == original.name);
    assert(loaded.level == original.level);
    assert(loaded.experience == original.experience);
    assert(loaded.stats.hp == original.stats.hp);
    assert(loaded.stats.strength == original.stats.strength);
    assert(std::abs(loaded.position.x - original.position.x) < 0.001f);
    assert(loaded.status == original.status);
    assert(loaded.guild_id.has_value());
    assert(*loaded.guild_id == 42);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 08: Complex Schema ===" << std::endl;

    test_common_types();
    test_character_types();
    test_item_types();
    test_inventory_system();
    test_social_system();
    test_container_integration();
    test_binary_complex();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
