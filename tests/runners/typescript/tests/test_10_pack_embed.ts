// Test Case 10: Pack Embed
// Tests @pack annotation helpers generated in the Zod schema file.

import { TestPackEmbed } from '../generated/10_pack_embed/typescript/schema';
import { TestPackEmbed as TestPackEmbedSchema } from '../generated/10_pack_embed/typescript/schema.schema';

function assert(condition: boolean, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function testPositionPack(): void {
    console.log("  Testing Position pack/unpack (sep: ;)...");

    const position: TestPackEmbed.Position = { x: 100.5, y: 200.3 };
    const packed = TestPackEmbedSchema.packPosition(position);
    assert(packed === "100.5;200.3", `Position pack: expected '100.5;200.3' got '${packed}'`);

    const unpacked = TestPackEmbedSchema.unpackPosition(packed);
    assert(Math.abs(unpacked.x - 100.5) < 0.01, "Position unpack x");
    assert(Math.abs(unpacked.y - 200.3) < 0.01, "Position unpack y");
}

function testPosition3DPack(): void {
    console.log("  Testing Position3D pack/unpack (sep: ;)...");

    const position: TestPackEmbed.Position3D = { x: 10, y: 20, z: 30 };
    const packed = TestPackEmbedSchema.packPosition3D(position);
    assert(packed === "10;20;30", `Position3D pack: expected '10;20;30' got '${packed}'`);

    const unpacked = TestPackEmbedSchema.unpackPosition3D(packed);
    assert(Math.abs(unpacked.x - 10) < 0.01, "Position3D unpack x");
    assert(Math.abs(unpacked.y - 20) < 0.01, "Position3D unpack y");
    assert(Math.abs(unpacked.z - 30) < 0.01, "Position3D unpack z");
}

function testColorPack(): void {
    console.log("  Testing Color pack/unpack (sep: ,)...");

    const color: TestPackEmbed.Color = { r: 255, g: 128, b: 64 };
    const packed = TestPackEmbedSchema.packColor(color);
    assert(packed === "255,128,64", `Color pack: expected '255,128,64' got '${packed}'`);

    const unpacked = TestPackEmbedSchema.unpackColor(packed);
    assert(unpacked.r === 255, "Color unpack r");
    assert(unpacked.g === 128, "Color unpack g");
    assert(unpacked.b === 64, "Color unpack b");
}

function testColorAlphaPack(): void {
    console.log("  Testing ColorAlpha pack/unpack (sep: |)...");

    const color: TestPackEmbed.ColorAlpha = { r: 255, g: 255, b: 255, a: 128 };
    const packed = TestPackEmbedSchema.packColorAlpha(color);
    assert(packed === "255|255|255|128", `ColorAlpha pack: expected '255|255|255|128' got '${packed}'`);

    const unpacked = TestPackEmbedSchema.unpackColorAlpha(packed);
    assert(unpacked.r === 255, "ColorAlpha unpack r");
    assert(unpacked.g === 255, "ColorAlpha unpack g");
    assert(unpacked.b === 255, "ColorAlpha unpack b");
    assert(unpacked.a === 128, "ColorAlpha unpack a");
}

function testSizePack(): void {
    console.log("  Testing Size pack/unpack (sep: ;)...");

    const size: TestPackEmbed.Size = { width: 800, height: 600 };
    const packed = TestPackEmbedSchema.packSize(size);
    assert(packed === "800;600", `Size pack: expected '800;600' got '${packed}'`);

    const unpacked = TestPackEmbedSchema.unpackSize(packed);
    assert(unpacked.width === 800, "Size unpack width");
    assert(unpacked.height === 600, "Size unpack height");
}

function testRangePack(): void {
    console.log("  Testing Range pack/unpack (sep: ~)...");

    const range: TestPackEmbed.Range = { min: -100, max: 100 };
    const packed = TestPackEmbedSchema.packRange(range);
    assert(packed === "-100~100", `Range pack: expected '-100~100' got '${packed}'`);

    const unpacked = TestPackEmbedSchema.unpackRange(packed);
    assert(unpacked.min === -100, "Range unpack min");
    assert(unpacked.max === 100, "Range unpack max");
}

function testTryUnpack(): void {
    console.log("  Testing tryUnpack failure cases...");

    assert(TestPackEmbedSchema.tryUnpackPosition("invalid") === null, "tryUnpackPosition should fail on invalid input");
    assert(TestPackEmbedSchema.tryUnpackPosition("nan;2.0") === null, "tryUnpackPosition should reject invalid numeric input");
    assert(TestPackEmbedSchema.tryUnpackSize("-1;2") === null, "tryUnpackSize should reject negative unsigned integer input");

    const position = TestPackEmbedSchema.tryUnpackPosition("1.0;2.0");
    assert(position !== null, "tryUnpackPosition should succeed on valid input");
    assert(Math.abs(position.x - 1.0) < 0.01, "tryUnpackPosition x");
    assert(Math.abs(position.y - 2.0) < 0.01, "tryUnpackPosition y");
}

function testStatsNoPack(): void {
    console.log("  Testing Stats without @pack works as a normal embed...");

    const stats: TestPackEmbed.Stats = {
        hp: 100,
        mp: 50,
        attack: 25,
        defense: 10,
    };

    assert(stats.hp === 100, "Stats hp");
    assert(stats.mp === 50, "Stats mp");
    assert(stats.attack === 25, "Stats attack");
    assert(stats.defense === 10, "Stats defense");
}

console.log("=== Test Case 10: Pack Embed ===");
testPositionPack();
testPosition3DPack();
testColorPack();
testColorAlphaPack();
testSizePack();
testRangePack();
testTryUnpack();
testStatsNoPack();
console.log("=== All tests passed! ===");
