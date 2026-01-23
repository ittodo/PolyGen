// Test Case 08: Complex Schema
// Comprehensive test combining all features

using System;
using System.Collections.Generic;

class Program
{
    static int passed = 0;
    static int failed = 0;

    static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            Console.WriteLine($"    FAILED: {message}");
            failed++;
        }
    }

    static void TestCommonTypes()
    {
        Console.WriteLine("  Testing common types (Vec2, Vec3, Color, Element)...");

        var v2 = new game.common.Vec2 { x = 1.0f, y = 2.0f };
        Assert(v2.x == 1.0f, "v2.x");
        Assert(v2.y == 2.0f, "v2.y");

        var v3 = new game.common.Vec3 { x = 1.0f, y = 2.0f, z = 3.0f };
        Assert(v3.z == 3.0f, "v3.z");

        var color = new game.common.Color { r = 255, g = 128, b = 64, a = 255 };
        Assert(color.r == 255, "color.r");
        Assert(color.g == 128, "color.g");

        var elem = game.common.Element.Fire;
        Assert((int)elem == 1, "Element.Fire");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestCharacterTypes()
    {
        Console.WriteLine("  Testing character types (Stats, Player, NPC)...");

        var stats = new game.character.Stats
        {
            hp = 100, max_hp = 100,
            mp = 50, max_mp = 50,
            strength = 10, agility = 8, intelligence = 5, vitality = 12
        };

        var player = new game.character.Player
        {
            id = 1,
            name = "Hero",
            level = 10,
            experience = 5000,
            stats = stats,
            position = new game.common.Vec3 { x = 100.0f, y = 50.0f, z = 0.0f },
            status = game.character.Player.Status.Online,
            guild_id = 1
        };

        Assert(player.name == "Hero", "player.name");
        Assert(player.level == 10, "player.level");
        Assert(player.stats.strength == 10, "player.stats.strength");
        Assert(player.guild_id == 1, "player.guild_id");

        var npc = new game.character.NPC
        {
            id = 1,
            name = "Merchant",
            title = "Item Seller",
            stats = stats,
            spawn_position = new game.common.Vec3 { x = 50.0f, y = 50.0f, z = 0.0f },
            ai_type = game.character.NPC.AIType.Friendly,
            dialog_options = new List<game.character.NPC.DialogOption>
            {
                new game.character.NPC.DialogOption { text = "Hello!", next_dialog_id = 2 },
                new game.character.NPC.DialogOption { text = "Goodbye!", next_dialog_id = null }
            }
        };

        Assert(npc.title == "Item Seller", "npc.title");
        Assert(npc.dialog_options.Count == 2, "npc.dialog_options.Count");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestItemTypes()
    {
        Console.WriteLine("  Testing item types (Item, Weapon, Armor)...");

        var item = new game.item.Item
        {
            id = 1,
            name = "Iron Sword",
            description = "A basic sword",
            rarity = game.item.Rarity.Common,
            sell_price = 100,
            max_stack = 1,
            icon = "sword_01",
            item_type = game.item.Item.ItemType.Weapon
        };

        Assert(item.name == "Iron Sword", "item.name");
        Assert(item.rarity == game.item.Rarity.Common, "item.rarity");

        var weapon = new game.item.Weapon
        {
            item_id = 1,
            damage_min = 10,
            damage_max = 15,
            attack_speed = 1.2f,
            element = game.common.Element.Physical,
            equip_slot = game.character.EquipSlot.MainHand,
            bonus_stats = new List<game.item.Weapon.BonusStat>
            {
                new game.item.Weapon.BonusStat { stat_name = "Strength", value = 5 }
            }
        };

        Assert(weapon.damage_min == 10, "weapon.damage_min");
        Assert(weapon.bonus_stats.Count == 1, "weapon.bonus_stats.Count");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestSocialSystem()
    {
        Console.WriteLine("  Testing social system (Guild, GuildMember, Friendship)...");

        var guild = new game.social.Guild
        {
            id = 1,
            name = "Heroes",
            tag = "HRO",
            leader_id = 1,
            level = 5,
            emblem_color = new game.common.Color { r = 255, g = 215, b = 0, a = 255 },
            created_at = 1640000000
        };

        Assert(guild.name == "Heroes", "guild.name");
        Assert(guild.tag == "HRO", "guild.tag");

        var member = new game.social.GuildMember
        {
            guild_id = 1,
            player_id = 1,
            rank = game.social.GuildMember.Rank.Leader,
            joined_at = 1640000000
        };

        Assert(member.rank == game.social.GuildMember.Rank.Leader, "member.rank");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinaryComplex()
    {
        Console.WriteLine("  Testing binary serialization of complex types...");

        var original = new game.character.Player
        {
            id = 999,
            name = "SerializationTest",
            level = 99,
            experience = 9999999,
            stats = new game.character.Stats
            {
                hp = 9999, max_hp = 9999,
                mp = 4999, max_mp = 4999,
                strength = 255, agility = 255, intelligence = 255, vitality = 255
            },
            position = new game.common.Vec3 { x = 123.456f, y = 789.012f, z = 345.678f },
            status = game.character.Player.Status.InBattle,
            guild_id = 42
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        game.character.BinaryWriters.WritePlayer(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = game.character.BinaryReaders.ReadPlayer(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.name == original.name, "name mismatch");
        Assert(loaded.level == original.level, "level mismatch");
        Assert(loaded.stats.hp == original.stats.hp, "stats.hp mismatch");
        Assert(Math.Abs(loaded.position.x - original.position.x) < 0.001f, "position.x mismatch");
        Assert(loaded.status == original.status, "status mismatch");
        Assert(loaded.guild_id == original.guild_id, "guild_id mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 08: Complex Schema ===");

        TestCommonTypes();
        TestCharacterTypes();
        TestItemTypes();
        TestSocialSystem();
        TestBinaryComplex();

        if (failed > 0)
        {
            Console.WriteLine($"=== {failed} tests failed! ===");
            Environment.Exit(1);
        }
        else
        {
            Console.WriteLine("=== All tests passed! ===");
        }
    }
}
