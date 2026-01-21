// TypeScript integration test
// This file validates that generated types compile correctly

import { Game } from '../generated/typescript/schema';
import { Game as GameSchemas } from '../generated/typescript/schema.schema';
import { z } from 'zod';

// Test basic type instantiation
function testCommonTypes(): void {
    console.log("Testing common types (Vec2, Vec3, Color, Element)...");

    const v2: Game.GameCommon.Vec2 = { x: 1.0, y: 2.0 };
    console.assert(v2.x === 1.0, "Vec2.x should be 1.0");
    console.assert(v2.y === 2.0, "Vec2.y should be 2.0");

    const v3: Game.GameCommon.Vec3 = { x: 1.0, y: 2.0, z: 3.0 };
    console.assert(v3.z === 3.0, "Vec3.z should be 3.0");

    const color: Game.GameCommon.Color = { r: 255, g: 128, b: 64, a: 255 };
    console.assert(color.r === 255, "Color.r should be 255");
    console.assert(color.g === 128, "Color.g should be 128");

    const elem = Game.GameCommon.Element.Fire;
    console.assert(elem === 1, "Element.Fire should be 1");

    console.log("    PASS");
}

// Test character types
function testCharacterTypes(): void {
    console.log("Testing character types (Stats, Player, NPC)...");

    const stats: Game.GameCharacter.Stats = {
        hp: 100, maxHp: 100,
        mp: 50, maxMp: 50,
        strength: 10, agility: 8, intelligence: 5, vitality: 12,
    };

    const player: Game.GameCharacter.Player = {
        id: 1,
        name: "Hero",
        level: 10,
        experience: 5000,
        stats: stats,
        position: { x: 100.0, y: 50.0, z: 0.0 },
        status: Game.GameCharacter.Status.Online,
        guildId: 1,
    };

    console.assert(player.name === "Hero", "Player name should be Hero");
    console.assert(player.level === 10, "Player level should be 10");
    console.assert(player.stats.strength === 10, "Player strength should be 10");
    console.assert(player.guildId === 1, "Player guildId should be 1");

    const npc: Game.GameCharacter.NPC = {
        id: 1,
        name: "Merchant",
        title: "Item Seller",
        stats: stats,
        spawnPosition: { x: 50.0, y: 50.0, z: 0.0 },
        aiType: Game.GameCharacter.AIType.Friendly,
        dialogOptions: [
            { text: "Hello!", nextDialogId: 2 },
            { text: "Goodbye!", nextDialogId: undefined },
        ],
    };

    console.assert(npc.title === "Item Seller", "NPC title should match");
    console.assert(npc.dialogOptions.length === 2, "NPC should have 2 dialog options");

    console.log("    PASS");
}

// Test item types
function testItemTypes(): void {
    console.log("Testing item types (Item, Weapon)...");

    const item: Game.GameItem.Item = {
        id: 1,
        name: "Iron Sword",
        description: "A basic sword",
        rarity: Game.GameItem.Rarity.Common,
        sellPrice: 100,
        maxStack: 1,
        icon: "sword_01",
        itemType: Game.GameItem.ItemType.Weapon,
    };

    console.assert(item.name === "Iron Sword", "Item name should match");
    console.assert(item.rarity === Game.GameItem.Rarity.Common, "Item rarity should be Common");

    const weapon: Game.GameItem.Weapon = {
        itemId: 1,
        damageMin: 10,
        damageMax: 15,
        attackSpeed: 1.2,
        element: Game.GameCommon.Element.Physical,
        equipSlot: Game.GameCharacter.EquipSlot.MainHand,
        bonusStats: [
            { statName: "Strength", value: 5 },
        ],
    };

    console.assert(weapon.damageMin === 10, "Weapon damageMin should be 10");
    console.assert(weapon.bonusStats.length === 1, "Weapon should have 1 bonus stat");

    console.log("    PASS");
}

// Test social system
function testSocialSystem(): void {
    console.log("Testing social system (Guild, GuildMember)...");

    const guild: Game.GameSocial.Guild = {
        id: 1,
        name: "Heroes",
        tag: "HRO",
        leaderId: 1,
        level: 5,
        emblemColor: { r: 255, g: 215, b: 0, a: 255 },
        createdAt: 1640000000,
    };

    console.assert(guild.name === "Heroes", "Guild name should match");
    console.assert(guild.tag === "HRO", "Guild tag should match");

    const member: Game.GameSocial.GuildMember = {
        guildId: 1,
        playerId: 1,
        rank: Game.GameSocial.Rank.Leader,
        joinedAt: 1640000000,
    };

    console.assert(member.rank === Game.GameSocial.Rank.Leader, "Member rank should be Leader");

    console.log("    PASS");
}

// Test Zod schema validation
function testZodSchemas(): void {
    console.log("Testing Zod schema validation...");

    // Valid data should pass
    const validVec2 = { x: 1.0, y: 2.0 };
    const result1 = GameSchemas.GameCommon.Vec2Schema.safeParse(validVec2);
    console.assert(result1.success, "Valid Vec2 should pass validation");

    // Invalid data should fail
    const invalidVec2 = { x: "not a number", y: 2.0 };
    const result2 = GameSchemas.GameCommon.Vec2Schema.safeParse(invalidVec2);
    console.assert(!result2.success, "Invalid Vec2 should fail validation");

    console.log("    PASS");
}

// Main
console.log("=== TypeScript Integration Tests ===");
testCommonTypes();
testCharacterTypes();
testItemTypes();
testSocialSystem();
testZodSchemas();
console.log("=== All tests passed! ===");
