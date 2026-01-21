// Test Case 03: Nested Namespaces
// Tests deeply nested namespace structures

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

    static void TestDeeplyNestedTable()
    {
        Console.WriteLine("  Testing deeply nested table (app.data.models.User)...");

        var user = new app.data.models.User
        {
            id = 1,
            username = "testuser"
        };

        Assert(user.id == 1, "id");
        Assert(user.username == "testuser", "username");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestNestedEnum()
    {
        Console.WriteLine("  Testing nested enum (app.data.enums.Permission)...");

        var perm = app.data.enums.Permission.Admin;
        Assert((int)perm == 3, "Admin value");

        perm = app.data.enums.Permission.Read;
        Assert((int)perm == 1, "Read value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestCrossNamespaceReference()
    {
        Console.WriteLine("  Testing cross-namespace reference (UserService)...");

        var service = new app.services.UserService
        {
            id = 1,
            target_user_id = 42,
            permission = app.data.enums.Permission.Write
        };

        Assert(service.id == 1, "id");
        Assert(service.target_user_id == 42, "target_user_id");
        Assert(service.permission == app.data.enums.Permission.Write, "permission");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestSeparateNamespace()
    {
        Console.WriteLine("  Testing separate namespace (util.Config)...");

        var config = new util.Config
        {
            key = "debug_mode",
            value = "true"
        };

        Assert(config.key == "debug_mode", "key");
        Assert(config.value == "true", "value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinaryNested()
    {
        Console.WriteLine("  Testing binary serialization with nested namespaces...");

        var original = new app.services.UserService
        {
            id = 999,
            target_user_id = 42,
            permission = app.data.enums.Permission.Admin
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        app.services.BinaryWriters.WriteUserService(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = app.services.BinaryReaders.ReadUserService(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.target_user_id == original.target_user_id, "target_user_id mismatch");
        Assert(loaded.permission == original.permission, "permission mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 03: Nested Namespaces ===");

        TestDeeplyNestedTable();
        TestNestedEnum();
        TestCrossNamespaceReference();
        TestSeparateNamespace();
        TestBinaryNested();

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
