// Test Case 06: Arrays and Optionals
// Tests array and optional field types

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

    static void TestArrayPrimitives()
    {
        Console.WriteLine("  Testing ArrayTest with primitive arrays...");

        var arr = new test.collections.ArrayTest
        {
            id = 1,
            int_list = new List<int> { 1, 2, 3, 4, 5 },
            string_list = new List<string> { "one", "two", "three" },
            float_list = new List<float> { 1.1f, 2.2f, 3.3f },
            bool_list = new List<bool> { true, false, true },
            tags = new List<test.collections.Tag>()
        };

        Assert(arr.int_list.Count == 5, "int_list count");
        Assert(arr.int_list[0] == 1, "int_list[0]");
        Assert(arr.int_list[4] == 5, "int_list[4]");
        Assert(arr.string_list.Count == 3, "string_list count");
        Assert(arr.string_list[1] == "two", "string_list[1]");
        Assert(arr.float_list.Count == 3, "float_list count");
        Assert(arr.bool_list.Count == 3, "bool_list count");
        Assert(arr.bool_list[0] == true, "bool_list[0]");
        Assert(arr.bool_list[1] == false, "bool_list[1]");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestArrayComplexTypes()
    {
        Console.WriteLine("  Testing ArrayTest with complex type arrays...");

        var arr = new test.collections.ArrayTest
        {
            id = 2,
            int_list = new List<int>(),
            string_list = new List<string>(),
            float_list = new List<float>(),
            bool_list = new List<bool>(),
            tags = new List<test.collections.Tag>
            {
                new test.collections.Tag { name = "Important", color = "red" },
                new test.collections.Tag { name = "Review", color = "yellow" }
            }
        };

        Assert(arr.tags.Count == 2, "tags count");
        Assert(arr.tags[0].name == "Important", "tags[0].name");
        Assert(arr.tags[0].color == "red", "tags[0].color");
        Assert(arr.tags[1].name == "Review", "tags[1].name");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestOptionalPrimitives()
    {
        Console.WriteLine("  Testing OptionalTest with optional primitives...");

        var opt = new test.collections.OptionalTest
        {
            id = 1,
            required_name = "Test"
        };

        Assert(opt.required_name == "Test", "required_name");
        // Note: In C#, nullable types default to null

        // Set values
        opt.opt_int = 42;
        opt.opt_string = "optional value";
        opt.opt_float = 3.14159;
        opt.opt_bool = true;

        Assert(opt.opt_int == 42, "opt_int");
        Assert(opt.opt_string == "optional value", "opt_string");
        Assert(Math.Abs(opt.opt_float.Value - 3.14159) < 0.0001, "opt_float");
        Assert(opt.opt_bool == true, "opt_bool");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinaryArraysOptionals()
    {
        Console.WriteLine("  Testing binary serialization with arrays and optionals...");

        var original = new test.collections.ArrayTest
        {
            id = 123,
            int_list = new List<int> { 10, 20, 30 },
            string_list = new List<string> { "a", "b", "c" },
            float_list = new List<float> { 1.5f, 2.5f },
            bool_list = new List<bool> { true, false },
            tags = new List<test.collections.Tag>
            {
                new test.collections.Tag { name = "Test", color = "white" }
            }
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        test.collections.BinaryWriters.WriteArrayTest(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = test.collections.BinaryReaders.ReadArrayTest(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.int_list.Count == original.int_list.Count, "int_list count mismatch");
        Assert(loaded.string_list[0] == "a", "string_list[0] mismatch");
        Assert(loaded.bool_list[0] == true, "bool_list[0] mismatch");
        Assert(loaded.tags.Count == 1, "tags count mismatch");
        Assert(loaded.tags[0].name == "Test", "tags[0].name mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 06: Arrays and Optionals ===");

        TestArrayPrimitives();
        TestArrayComplexTypes();
        TestOptionalPrimitives();
        TestBinaryArraysOptionals();

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
