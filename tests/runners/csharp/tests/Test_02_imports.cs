// Test Case 02: Cross-namespace References
// Tests referencing types from different namespaces

using System;

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

    static void TestCommonEmbed()
    {
        Console.WriteLine("  Testing common embed (Position)...");

        var pos = new common.Position
        {
            x = 1.0f,
            y = 2.0f,
            z = 3.0f
        };

        Assert(pos.x == 1.0f, "x");
        Assert(pos.y == 2.0f, "y");
        Assert(pos.z == 3.0f, "z");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestCommonEnum()
    {
        Console.WriteLine("  Testing common enum (Status)...");

        var status = common.Status.Active;
        Assert(status == common.Status.Active, "Active");

        status = common.Status.Inactive;
        Assert((int)status == 1, "Inactive value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestPlayerCrossNamespace()
    {
        Console.WriteLine("  Testing Player with cross-namespace types...");

        var player = new game.Player
        {
            id = 1,
            name = "Hero",
            position = new common.Position { x = 100.0f, y = 50.0f, z = 0.0f },
            status = common.Status.Active
        };

        Assert(player.id == 1, "id");
        Assert(player.name == "Hero", "name");
        Assert(player.position.x == 100.0f, "position.x");
        Assert(player.status == common.Status.Active, "status");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestNpcCrossNamespace()
    {
        Console.WriteLine("  Testing NPC with cross-namespace types...");

        var npc = new game.NPC
        {
            id = 100,
            display_name = "Merchant",
            spawn_point = new common.Position { x = 50.0f, y = 50.0f, z = 0.0f },
            ai_state = common.Status.Active
        };

        Assert(npc.id == 100, "id");
        Assert(npc.display_name == "Merchant", "display_name");
        Assert(npc.spawn_point.x == 50.0f, "spawn_point.x");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinaryCrossNamespace()
    {
        Console.WriteLine("  Testing binary serialization with cross-namespace types...");

        var original = new game.Player
        {
            id = 42,
            name = "Test Player",
            position = new common.Position { x = 123.456f, y = 789.012f, z = 345.678f },
            status = common.Status.Inactive
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        game.BinaryWriters.WritePlayer(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = game.BinaryReaders.ReadPlayer(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.name == original.name, "name mismatch");
        Assert(Math.Abs(loaded.position.x - original.position.x) < 0.001f, "position.x mismatch");
        Assert(loaded.status == original.status, "status mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 02: Cross-namespace References ===");

        TestCommonEmbed();
        TestCommonEnum();
        TestPlayerCrossNamespace();
        TestNpcCrossNamespace();
        TestBinaryCrossNamespace();

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
