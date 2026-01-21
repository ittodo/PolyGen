// Test Case 08: Complex Schema
// Comprehensive test combining all features
// Note: This test may fail if the Rust code generator has issues with
// cross-namespace type references (e.g., crate::common_types::... paths)

use std::io::Cursor;

use polygen_test::schema::game::game_common::{Vec2, Vec3, Color, Element};
use polygen_test::schema::game::game_character::{Stats, Player, NPC, EquipSlot, DialogOption, Status, AIType};
use polygen_test::schema::game::game_item::{Item, Weapon, Rarity, BonusStat, ItemType};
use polygen_test::schema::game::game_social::{Guild, GuildMember, Rank};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 08: Complex Schema ===");

    test_common_types();
    test_character_types();
    test_item_types();
    test_social_system();
    test_binary_complex();

    println!("=== All tests passed! ===");
}

fn test_common_types() {
    println!("  Testing common types (Vec2, Vec3, Color, Element)...");

    let v2 = Vec2 { x: 1.0, y: 2.0 };
    assert_eq!(v2.x, 1.0);
    assert_eq!(v2.y, 2.0);

    let v3 = Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    assert_eq!(v3.z, 3.0);

    let color = Color { r: 255, g: 128, b: 64, a: 255 };
    assert_eq!(color.r, 255);
    assert_eq!(color.g, 128);

    let elem = Element::Fire;
    assert_eq!(elem as i32, 1);

    println!("    PASS");
}

fn test_character_types() {
    println!("  Testing character types (Stats, Player, NPC)...");

    let stats = Stats {
        hp: 100, max_hp: 100,
        mp: 50, max_mp: 50,
        strength: 10, agility: 8, intelligence: 5, vitality: 12,
    };

    let player = Player {
        id: 1,
        name: "Hero".to_string(),
        level: 10,
        experience: 5000,
        stats: stats.clone(),
        position: Vec3 { x: 100.0, y: 50.0, z: 0.0 },
        status: Status::Online,
        guild_id: Some(1),
    };

    assert_eq!(player.name, "Hero");
    assert_eq!(player.level, 10);
    assert_eq!(player.stats.strength, 10);
    assert_eq!(player.guild_id, Some(1));

    let npc = NPC {
        id: 1,
        name: "Merchant".to_string(),
        title: Some("Item Seller".to_string()),
        stats: stats,
        spawn_position: Vec3 { x: 50.0, y: 50.0, z: 0.0 },
        ai_type: AIType::Friendly,
        dialog_options: vec![
            DialogOption { text: "Hello!".to_string(), next_dialog_id: Some(2) },
            DialogOption { text: "Goodbye!".to_string(), next_dialog_id: None },
        ],
    };

    assert_eq!(npc.title, Some("Item Seller".to_string()));
    assert_eq!(npc.dialog_options.len(), 2);

    println!("    PASS");
}

fn test_item_types() {
    println!("  Testing item types (Item, Weapon)...");

    let item = Item {
        id: 1,
        name: "Iron Sword".to_string(),
        description: Some("A basic sword".to_string()),
        rarity: Rarity::Common,
        sell_price: 100,
        max_stack: 1,
        icon: "sword_01".to_string(),
        item_type: ItemType::Weapon,
    };

    assert_eq!(item.name, "Iron Sword");
    assert_eq!(item.rarity, Rarity::Common);

    let weapon = Weapon {
        item_id: 1,
        damage_min: 10,
        damage_max: 15,
        attack_speed: 1.2,
        element: Element::Physical,
        equip_slot: EquipSlot::MainHand,
        bonus_stats: vec![
            BonusStat { stat_name: "Strength".to_string(), value: 5 },
        ],
    };

    assert_eq!(weapon.damage_min, 10);
    assert_eq!(weapon.bonus_stats.len(), 1);

    println!("    PASS");
}

fn test_social_system() {
    println!("  Testing social system (Guild, GuildMember)...");

    let guild = Guild {
        id: 1,
        name: "Heroes".to_string(),
        tag: "HRO".to_string(),
        leader_id: 1,
        level: 5,
        emblem_color: Color { r: 255, g: 215, b: 0, a: 255 },
        created_at: 1640000000,
    };

    assert_eq!(guild.name, "Heroes");
    assert_eq!(guild.tag, "HRO");

    let member = GuildMember {
        guild_id: 1,
        player_id: 1,
        rank: Rank::Leader,
        joined_at: 1640000000,
    };

    assert_eq!(member.rank, Rank::Leader);

    println!("    PASS");
}

fn test_binary_complex() {
    println!("  Testing binary serialization of complex types...");

    let original = Player {
        id: 999,
        name: "SerializationTest".to_string(),
        level: 99,
        experience: 9999999,
        stats: Stats {
            hp: 9999, max_hp: 9999,
            mp: 4999, max_mp: 4999,
            strength: 255, agility: 255, intelligence: 255, vitality: 255,
        },
        position: Vec3 { x: 123.456, y: 789.012, z: 345.678 },
        status: Status::InBattle,
        guild_id: Some(42),
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = Player::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.level, original.level);
    assert_eq!(loaded.stats.hp, original.stats.hp);
    assert!((loaded.position.x - original.position.x).abs() < 0.001);
    assert_eq!(loaded.status, original.status);
    assert_eq!(loaded.guild_id, original.guild_id);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
