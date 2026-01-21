// Test Case 01: Basic Types
// Tests all primitive types and simple struct generation

using System;
using System.Diagnostics;
using test.basic;

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

    static void TestAllTypes()
    {
        Console.WriteLine("  Testing AllTypes creation...");

        var all = new AllTypes
        {
            val_u8 = 255,
            val_u16 = 65535,
            val_u32 = 4294967295,
            val_u64 = 18446744073709551615,
            val_i8 = -128,
            val_i16 = -32768,
            val_i32 = -2147483648,
            val_i64 = -9223372036854775808,
            val_f32 = 3.14159f,
            val_f64 = 2.718281828459045,
            val_bool = true,
            val_string = "Hello, World!"
        };

        Assert(all.val_u8 == 255, "val_u8");
        Assert(all.val_u16 == 65535, "val_u16");
        Assert(all.val_u32 == 4294967295, "val_u32");
        Assert(all.val_u64 == 18446744073709551615, "val_u64");
        Assert(all.val_i8 == -128, "val_i8");
        Assert(all.val_i16 == -32768, "val_i16");
        Assert(all.val_i32 == -2147483648, "val_i32");
        Assert(all.val_i64 == -9223372036854775808, "val_i64");
        Assert(Math.Abs(all.val_f32 - 3.14159f) < 0.0001f, "val_f32");
        Assert(Math.Abs(all.val_f64 - 2.718281828459045) < 0.0000001, "val_f64");
        Assert(all.val_bool == true, "val_bool");
        Assert(all.val_string == "Hello, World!", "val_string");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestSimpleStruct()
    {
        Console.WriteLine("  Testing SimpleStruct...");

        var simple = new SimpleStruct
        {
            id = 42,
            name = "Test",
            value = 100
        };

        Assert(simple.id == 42, "id");
        Assert(simple.name == "Test", "name");
        Assert(simple.value == 100, "value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinarySerialization()
    {
        Console.WriteLine("  Testing binary serialization...");

        var original = new SimpleStruct
        {
            id = 123,
            name = "Serialization Test",
            value = 456
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        test.basic.BinaryWriters.WriteSimpleStruct(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = test.basic.BinaryReaders.ReadSimpleStruct(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.name == original.name, "name mismatch");
        Assert(loaded.value == original.value, "value mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 01: Basic Types ===");

        TestAllTypes();
        TestSimpleStruct();
        TestBinarySerialization();

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
