// Test Case 10: Pack Embed
// Tests @pack annotation on embed types for serializing fields to a single string

use polygen_test::schema::test_pack_embed::*;

fn main() {
    println!("=== Test Case 10: Pack Embed ===");

    test_position_pack();
    test_position3d_pack();
    test_color_pack();
    test_color_alpha_pack();
    test_size_pack();
    test_range_pack();
    test_unpack_error();
    test_stats_no_pack();

    println!("=== All tests passed! ===");
}

fn test_position_pack() {
    println!("  Testing Position pack/unpack (sep: ;)...");

    let pos = Position { x: 100.5, y: 200.3 };
    let packed = pos.pack();
    assert_eq!(packed, "100.5;200.3");

    let unpacked = Position::unpack(&packed).unwrap();
    assert!((unpacked.x - 100.5).abs() < 0.01);
    assert!((unpacked.y - 200.3).abs() < 0.01);

    println!("    PASS");
}

fn test_position3d_pack() {
    println!("  Testing Position3D pack/unpack (sep: ;)...");

    let pos = Position3D { x: 10.0, y: 20.0, z: 30.0 };
    let packed = pos.pack();
    assert_eq!(packed, "10;20;30");

    let unpacked = Position3D::unpack(&packed).unwrap();
    assert!((unpacked.x - 10.0).abs() < 0.01);
    assert!((unpacked.y - 20.0).abs() < 0.01);
    assert!((unpacked.z - 30.0).abs() < 0.01);

    println!("    PASS");
}

fn test_color_pack() {
    println!("  Testing Color pack/unpack (sep: ,)...");

    let c = Color { r: 255, g: 128, b: 64 };
    let packed = c.pack();
    assert_eq!(packed, "255,128,64");

    let unpacked = Color::unpack(&packed).unwrap();
    assert_eq!(unpacked.r, 255);
    assert_eq!(unpacked.g, 128);
    assert_eq!(unpacked.b, 64);

    println!("    PASS");
}

fn test_color_alpha_pack() {
    println!("  Testing ColorAlpha pack/unpack (sep: |)...");

    let ca = ColorAlpha { r: 255, g: 255, b: 255, a: 128 };
    let packed = ca.pack();
    assert_eq!(packed, "255|255|255|128");

    let unpacked = ColorAlpha::unpack(&packed).unwrap();
    assert_eq!(unpacked.r, 255);
    assert_eq!(unpacked.g, 255);
    assert_eq!(unpacked.b, 255);
    assert_eq!(unpacked.a, 128);

    println!("    PASS");
}

fn test_size_pack() {
    println!("  Testing Size pack/unpack (sep: ;)...");

    let s = Size { width: 800, height: 600 };
    let packed = s.pack();
    assert_eq!(packed, "800;600");

    let unpacked = Size::unpack(&packed).unwrap();
    assert_eq!(unpacked.width, 800);
    assert_eq!(unpacked.height, 600);

    println!("    PASS");
}

fn test_range_pack() {
    println!("  Testing Range pack/unpack (sep: ~)...");

    let r = Range { min: -100, max: 100 };
    let packed = r.pack();
    assert_eq!(packed, "-100~100");

    let unpacked = Range::unpack(&packed).unwrap();
    assert_eq!(unpacked.min, -100);
    assert_eq!(unpacked.max, 100);

    println!("    PASS");
}

fn test_unpack_error() {
    println!("  Testing unpack error cases...");

    let result = Position::unpack("invalid");
    assert!(result.is_err());

    let result = Position::unpack("1;2;3");
    assert!(result.is_err());

    println!("    PASS");
}

fn test_stats_no_pack() {
    println!("  Testing Stats (no @pack) works as normal embed...");

    let stats = Stats {
        hp: 100,
        mp: 50,
        attack: 25,
        defense: 10,
    };

    assert_eq!(stats.hp, 100);
    assert_eq!(stats.mp, 50);

    println!("    PASS");
}
