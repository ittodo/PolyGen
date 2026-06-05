// Test Case 11: Relations and composite indexes

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

    static void TestCompositeIndexAndNavigation()
    {
        Console.WriteLine("  Testing composite index and navigation...");

        var container = new Schema.Container.SchemaDataContainer();
        var user = new examples.relations.User
        {
            id = 1,
            email = "alice@example.com",
            display_name = "Alice"
        };
        var draft = new examples.relations.Post
        {
            id = 10,
            author_id = 1,
            status = examples.relations.PostStatus.Draft,
            title = "Draft post"
        };
        var published = new examples.relations.Post
        {
            id = 11,
            author_id = 1,
            status = examples.relations.PostStatus.Published,
            title = "Published post"
        };

        container.Users.Add(user);
        container.Posts.Add(draft);
        container.Posts.Add(published);

        Assert(container.Posts.ByAuthorId[1].Count == 2, "author group index should include both posts");
        var publishedByComposite = container.Posts.ByAuthorIdStatus[(1u, examples.relations.PostStatus.Published)];
        Assert(publishedByComposite.Count == 1, "composite index should filter by author and status");
        Assert(publishedByComposite[0].id == 11, "composite index should return published post");
        Assert(user.Posts.Count == 2, "reverse navigation should resolve user posts");
        Assert(published.author?.email == "alice@example.com", "forward FK navigation should resolve author");
        container.ValidateOrThrow();

        var path = System.IO.Path.Combine(
            System.IO.Path.GetTempPath(),
            "polygen-csharp-relations-binary-ref-" + Guid.NewGuid().ToString("N") + ".bin");
        try
        {
            Schema.BinaryRefs.SchemaBinaryRefContext.SaveBinary(path, container);
            var ctx = Schema.BinaryRefs.SchemaBinaryRefContext.OpenBinary(path);

            var binaryPublished = ctx.Posts.FindByAuthorIdStatus((1u, examples.relations.PostStatus.Published));
            Assert(binaryPublished.Count == 1, "BinaryRef composite index should filter by author and status");
            Assert(binaryPublished[0].id == 11, "BinaryRef composite index should return published post");
            Assert(ctx.Posts.FindByAuthorIdStatus((1u, examples.relations.PostStatus.Archived)).Count == 0,
                "BinaryRef composite index should return empty list for missing tuple");
        }
        finally
        {
            if (System.IO.File.Exists(path))
            {
                System.IO.File.Delete(path);
            }
        }

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestMissingForeignKeyValidation()
    {
        Console.WriteLine("  Testing missing FK validation...");

        var container = new Schema.Container.SchemaDataContainer();
        container.Posts.Add(new examples.relations.Post
        {
            id = 20,
            author_id = 999,
            status = examples.relations.PostStatus.Archived,
            title = "Broken post"
        });

        var result = container.ValidateAll();
        Assert(!result.IsValid, "missing author should fail validation");
        Assert(result.Errors.Count == 1, "missing author should report one validation error");
        Assert(result.Errors[0].ConstraintType == "ForeignKey", "validation error should be ForeignKey");

        var threw = false;
        try
        {
            container.ValidateOrThrow();
        }
        catch (Polygen.Common.ValidationException)
        {
            threw = true;
        }
        Assert(threw, "ValidateOrThrow should reject missing FK");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 11: Relations and composite indexes ===");

        TestCompositeIndexAndNavigation();
        TestMissingForeignKeyValidation();

        if (failed > 0)
        {
            Console.WriteLine($"=== {failed} tests failed! ===");
            Environment.Exit(1);
        }

        Console.WriteLine("=== All tests passed! ===");
    }
}
