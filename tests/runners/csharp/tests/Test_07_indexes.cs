// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints with containers

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

    static void TestUserCreation()
    {
        Console.WriteLine("  Testing User table creation...");

        var user = new test.indexes.User
        {
            id = 1,
            username = "john",
            email = "john@example.com",
            display_name = "John Doe"
        };

        Assert(user.id == 1, "id");
        Assert(user.username == "john", "username");
        Assert(user.email == "john@example.com", "email");
        Assert(user.display_name == "John Doe", "display_name");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestCategoryCreation()
    {
        Console.WriteLine("  Testing Category creation...");

        var cat = new test.indexes.Category
        {
            id = 1,
            name = "Technology",
            description = "Tech stuff"
        };

        Assert(cat.id == 1, "id");
        Assert(cat.name == "Technology", "name");
        Assert(cat.description == "Tech stuff", "description");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestPostWithForeignKeys()
    {
        Console.WriteLine("  Testing Post with foreign keys...");

        var post = new test.indexes.Post
        {
            id = 1,
            title = "First Post",
            content = "Hello World",
            author_id = 1,
            category_id = 1
        };

        Assert(post.id == 1, "id");
        Assert(post.title == "First Post", "title");
        Assert(post.author_id == 1, "author_id (FK)");
        Assert(post.category_id == 1, "category_id (FK)");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestJunctionTable()
    {
        Console.WriteLine("  Testing PostTag junction table...");

        var tag = new test.indexes.Tag
        {
            id = 1,
            name = "cpp"
        };

        var postTag = new test.indexes.PostTag
        {
            post_id = 1,
            tag_id = 1
        };

        Assert(tag.id == 1, "tag.id");
        Assert(tag.name == "cpp", "tag.name");
        Assert(postTag.post_id == 1, "postTag.post_id");
        Assert(postTag.tag_id == 1, "postTag.tag_id");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinarySerialization()
    {
        Console.WriteLine("  Testing binary serialization...");

        var original = new test.indexes.User
        {
            id = 42,
            username = "testuser",
            email = "test@test.com",
            display_name = "Test User"
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        test.indexes.BinaryWriters.WriteUser(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = test.indexes.BinaryReaders.ReadUser(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.username == original.username, "username mismatch");
        Assert(loaded.email == original.email, "email mismatch");
        Assert(loaded.display_name == original.display_name, "display_name mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 07: Indexes ===");

        TestUserCreation();
        TestCategoryCreation();
        TestPostWithForeignKeys();
        TestJunctionTable();
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
