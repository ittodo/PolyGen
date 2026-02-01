// Test Case 10: Pack Embed
// Tests @pack annotation on embed types for serializing fields to a single string

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

    static void TestPositionPack()
    {
        Console.WriteLine("  Testing Position pack/unpack (sep: ;)...");

        var pos = new test.pack_embed.Position { x = 100.5f, y = 200.3f };
        var packed = pos.Pack();
        Assert(packed == "100.5;200.3", $"Position pack: expected '100.5;200.3' got '{packed}'");

        var unpacked = test.pack_embed.Position.Unpack(packed);
        Assert(Math.Abs(unpacked.x - 100.5f) < 0.01f, "Position unpack x");
        Assert(Math.Abs(unpacked.y - 200.3f) < 0.01f, "Position unpack y");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestPosition3DPack()
    {
        Console.WriteLine("  Testing Position3D pack/unpack (sep: ;)...");

        var pos = new test.pack_embed.Position3D { x = 10f, y = 20f, z = 30f };
        var packed = pos.Pack();
        Assert(packed == "10;20;30", $"Position3D pack: expected '10;20;30' got '{packed}'");

        var unpacked = test.pack_embed.Position3D.Unpack(packed);
        Assert(Math.Abs(unpacked.x - 10f) < 0.01f, "Position3D unpack x");
        Assert(Math.Abs(unpacked.y - 20f) < 0.01f, "Position3D unpack y");
        Assert(Math.Abs(unpacked.z - 30f) < 0.01f, "Position3D unpack z");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestColorPack()
    {
        Console.WriteLine("  Testing Color pack/unpack (sep: ,)...");

        var c = new test.pack_embed.Color { r = 255, g = 128, b = 64 };
        var packed = c.Pack();
        Assert(packed == "255,128,64", $"Color pack: expected '255,128,64' got '{packed}'");

        var unpacked = test.pack_embed.Color.Unpack(packed);
        Assert(unpacked.r == 255, "Color unpack r");
        Assert(unpacked.g == 128, "Color unpack g");
        Assert(unpacked.b == 64, "Color unpack b");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestColorAlphaPack()
    {
        Console.WriteLine("  Testing ColorAlpha pack/unpack (sep: |)...");

        var ca = new test.pack_embed.ColorAlpha { r = 255, g = 255, b = 255, a = 128 };
        var packed = ca.Pack();
        Assert(packed == "255|255|255|128", $"ColorAlpha pack: expected '255|255|255|128' got '{packed}'");

        var unpacked = test.pack_embed.ColorAlpha.Unpack(packed);
        Assert(unpacked.r == 255, "ColorAlpha unpack r");
        Assert(unpacked.g == 255, "ColorAlpha unpack g");
        Assert(unpacked.b == 255, "ColorAlpha unpack b");
        Assert(unpacked.a == 128, "ColorAlpha unpack a");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestSizePack()
    {
        Console.WriteLine("  Testing Size pack/unpack (sep: ;)...");

        var s = new test.pack_embed.Size { width = 800, height = 600 };
        var packed = s.Pack();
        Assert(packed == "800;600", $"Size pack: expected '800;600' got '{packed}'");

        var unpacked = test.pack_embed.Size.Unpack(packed);
        Assert(unpacked.width == 800, "Size unpack width");
        Assert(unpacked.height == 600, "Size unpack height");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestRangePack()
    {
        Console.WriteLine("  Testing Range pack/unpack (sep: ~)...");

        var r = new test.pack_embed.Range { min = -100, max = 100 };
        var packed = r.Pack();
        Assert(packed == "-100~100", $"Range pack: expected '-100~100' got '{packed}'");

        var unpacked = test.pack_embed.Range.Unpack(packed);
        Assert(unpacked.min == -100, "Range unpack min");
        Assert(unpacked.max == 100, "Range unpack max");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestTryUnpack()
    {
        Console.WriteLine("  Testing TryUnpack failure cases...");

        bool ok = test.pack_embed.Position.TryUnpack("invalid", out var result);
        Assert(!ok, "TryUnpack should fail on invalid input");

        ok = test.pack_embed.Position.TryUnpack("1.0;2.0", out result);
        Assert(ok, "TryUnpack should succeed on valid input");
        Assert(Math.Abs(result.x - 1.0f) < 0.01f, "TryUnpack x");
        Assert(Math.Abs(result.y - 2.0f) < 0.01f, "TryUnpack y");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestStatsNoPack()
    {
        Console.WriteLine("  Testing Stats (no @pack) works as normal embed...");

        var stats = new test.pack_embed.Stats { hp = 100, mp = 50, attack = 25, defense = 10 };
        Assert(stats.hp == 100, "Stats hp");
        Assert(stats.mp == 50, "Stats mp");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void Main(string[] args)
    {
        Console.WriteLine("=== Test Case 10: Pack Embed ===");

        TestPositionPack();
        TestPosition3DPack();
        TestColorPack();
        TestColorAlphaPack();
        TestSizePack();
        TestRangePack();
        TestTryUnpack();
        TestStatsNoPack();

        Console.WriteLine($"\nResults: {passed} passed, {failed} failed");
        if (failed > 0)
        {
            Environment.Exit(1);
        }
        Console.WriteLine("=== All tests passed! ===");
    }
}
