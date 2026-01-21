// Test Case 04: Inline Enums
// Tests enum definitions inside tables

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

    static void TestOrderInlineEnums()
    {
        Console.WriteLine("  Testing Order inline enums...");

        var order = new test.enums.Order
        {
            id = 1,
            customer_name = "John Doe",
            status = test.enums.Order.Status.Pending,
            priority = test.enums.Order.Priority.High
        };

        Assert(order.id == 1, "id");
        Assert(order.customer_name == "John Doe", "customer_name");
        Assert(order.status == test.enums.Order.Status.Pending, "status");
        Assert(order.priority == test.enums.Order.Priority.High, "priority");

        // Test all status values
        order.status = test.enums.Order.Status.Paid;
        Assert((int)order.status == 1, "Paid value");

        order.status = test.enums.Order.Status.Shipped;
        Assert((int)order.status == 2, "Shipped value");

        order.status = test.enums.Order.Status.Delivered;
        Assert((int)order.status == 3, "Delivered value");

        order.status = test.enums.Order.Status.Cancelled;
        Assert((int)order.status == 4, "Cancelled value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestTaskInlineEnum()
    {
        Console.WriteLine("  Testing Task inline enum...");

        var task = new test.enums.Task
        {
            id = 1,
            title = "Complete tests",
            state = test.enums.Task.State.Todo
        };

        Assert(task.state == test.enums.Task.State.Todo, "Todo");

        task.state = test.enums.Task.State.InProgress;
        Assert((int)task.state == 1, "InProgress value");

        task.state = test.enums.Task.State.Done;
        Assert((int)task.state == 2, "Done value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestGlobalEnum()
    {
        Console.WriteLine("  Testing global enum (GlobalStatus)...");

        var status = test.enums.GlobalStatus.Unknown;
        Assert((int)status == 0, "Unknown value");

        status = test.enums.GlobalStatus.Active;
        Assert((int)status == 1, "Active value");

        status = test.enums.GlobalStatus.Disabled;
        Assert((int)status == 2, "Disabled value");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinaryInlineEnums()
    {
        Console.WriteLine("  Testing binary serialization with inline enums...");

        var original = new test.enums.Order
        {
            id = 12345,
            customer_name = "Test Customer",
            status = test.enums.Order.Status.Shipped,
            priority = test.enums.Order.Priority.Urgent
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        test.enums.BinaryWriters.WriteOrder(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = test.enums.BinaryReaders.ReadOrder(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.customer_name == original.customer_name, "customer_name mismatch");
        Assert(loaded.status == original.status, "status mismatch");
        Assert(loaded.priority == original.priority, "priority mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 04: Inline Enums ===");

        TestOrderInlineEnums();
        TestTaskInlineEnum();
        TestGlobalEnum();
        TestBinaryInlineEnums();

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
