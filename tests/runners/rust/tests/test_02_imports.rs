// Test Case 02: Cross-namespace References
// Tests referencing types from different namespaces

use std::io::Cursor;

use polygen_test::schema::common::{Position, Status};
use polygen_test::schema::game::{Player, NPC};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 02: Cross-namespace References ===");

    test_common_embed();
    test_common_enum();
    test_player_cross_namespace();
    test_npc_cross_namespace();
    test_binary_cross_namespace();

    println!("=== All tests passed! ===");
}

fn test_common_embed() {
    println!("  Testing common embed (Position)...");

    let pos = Position {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    assert_eq!(pos.x, 1.0);
    assert_eq!(pos.y, 2.0);
    assert_eq!(pos.z, 3.0);

    println!("    PASS");
}

fn test_common_enum() {
    println!("  Testing common enum (Status)...");

    let status = Status::Active;
    assert_eq!(status, Status::Active);

    let status = Status::Inactive;
    assert_eq!(status as i32, 1);

    println!("    PASS");
}

fn test_player_cross_namespace() {
    println!("  Testing Player with cross-namespace types...");

    let player = Player {
        id: 1,
        name: "Hero".to_string(),
        position: Position { x: 100.0, y: 50.0, z: 0.0 },
        status: Status::Active,
    };

    assert_eq!(player.id, 1);
    assert_eq!(player.name, "Hero");
    assert_eq!(player.position.x, 100.0);
    assert_eq!(player.status, Status::Active);

    println!("    PASS");
}

fn test_npc_cross_namespace() {
    println!("  Testing NPC with cross-namespace types...");

    let npc = NPC {
        id: 100,
        display_name: "Merchant".to_string(),
        spawn_point: Position { x: 50.0, y: 50.0, z: 0.0 },
        ai_state: Status::Active,
    };

    assert_eq!(npc.id, 100);
    assert_eq!(npc.display_name, "Merchant");
    assert_eq!(npc.spawn_point.x, 50.0);

    println!("    PASS");
}

fn test_binary_cross_namespace() {
    println!("  Testing binary serialization with cross-namespace types...");

    let original = Player {
        id: 42,
        name: "Test Player".to_string(),
        position: Position { x: 123.456, y: 789.012, z: 345.678 },
        status: Status::Inactive,
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = Player::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert!((loaded.position.x - original.position.x).abs() < 0.001);
    assert_eq!(loaded.status, original.status);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
