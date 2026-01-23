// Test Case 02: Cross-namespace References
// Tests referencing types from different namespaces

import { Common, Game } from '../generated/02_imports/typescript/schema';

// Test common embed (Position)
function testCommonEmbed(): void {
    console.log("  Testing common embed (Position)...");

    const pos: Common.Position = {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    console.assert(pos.x === 1.0, "x should be 1.0");
    console.assert(pos.y === 2.0, "y should be 2.0");
    console.assert(pos.z === 3.0, "z should be 3.0");

    console.log("    PASS");
}

// Test common enum (Status)
function testCommonEnum(): void {
    console.log("  Testing common enum (Status)...");

    const status = Common.Status.Active;
    console.assert(status === 0, "Active should be 0");

    const inactive = Common.Status.Inactive;
    console.assert(inactive === 1, "Inactive should be 1");

    console.log("    PASS");
}

// Test Player with cross-namespace types
function testPlayerCrossNamespace(): void {
    console.log("  Testing Player with cross-namespace types...");

    const player: Game.Player = {
        id: 1,
        name: "Hero",
        position: { x: 100.0, y: 50.0, z: 0.0 },
        status: Common.Status.Active,
    };

    console.assert(player.id === 1, "id should be 1");
    console.assert(player.name === "Hero", "name should be Hero");
    console.assert(player.position.x === 100.0, "position.x should be 100.0");
    console.assert(player.status === Common.Status.Active, "status should be Active");

    console.log("    PASS");
}

// Test NPC with cross-namespace types
function testNpcCrossNamespace(): void {
    console.log("  Testing NPC with cross-namespace types...");

    const npc: Game.NPC = {
        id: 100,
        displayName: "Merchant",
        spawnPoint: { x: 50.0, y: 50.0, z: 0.0 },
        aiState: Common.Status.Active,
    };

    console.assert(npc.id === 100, "id should be 100");
    console.assert(npc.displayName === "Merchant", "displayName should be Merchant");
    console.assert(npc.spawnPoint.x === 50.0, "spawnPoint.x should be 50.0");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 02: Cross-namespace References ===");
testCommonEmbed();
testCommonEnum();
testPlayerCrossNamespace();
testNpcCrossNamespace();
console.log("=== All tests passed! ===");
