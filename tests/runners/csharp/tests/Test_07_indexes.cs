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
            description = "Tech stuff",
            rank = 7,
            kind = test.indexes.CategoryKind.Public
        };

        Assert(cat.id == 1, "id");
        Assert(cat.name == "Technology", "name");
        Assert(cat.description == "Tech stuff", "description");
        Assert(cat.rank == 7, "rank");
        Assert(cat.kind == test.indexes.CategoryKind.Public, "kind");

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

    static Schema.BinaryRefs.test_indexes_UserRef OpenUserRefAfterContextScope(string path)
    {
        var ctx = Schema.BinaryRefs.SchemaBinaryRefContext.OpenBinary(path);
        var userRef = ctx.Users.GetById(1);
        if (userRef == null)
        {
            throw new InvalidOperationException("User ref was not found after OpenBinary.");
        }
        return userRef;
    }

    static void TestBinaryRefLoad()
    {
        Console.WriteLine("  Testing indexed binary ref load...");

        var container = new Schema.Container.SchemaDataContainer();
        container.Users.Add(new test.indexes.User
        {
            id = 1,
            username = "alice",
            email = "alice@example.com",
            display_name = "Alice"
        });
        container.Users.Add(new test.indexes.User
        {
            id = 2,
            username = "bob",
            email = "bob@example.com",
            display_name = "Bob"
        });
        container.Categorys.Add(new test.indexes.Category
        {
            id = 10,
            name = "Tech",
            description = "Technology",
            rank = 7,
            kind = test.indexes.CategoryKind.Public
        });
        container.Posts.Add(new test.indexes.Post
        {
            id = 100,
            title = "Binary refs",
            content = "Lazy row access",
            author_id = 1,
            category_id = 10
        });
        container.Posts.Add(new test.indexes.Post
        {
            id = 101,
            title = "Indexes",
            content = "Lookup by author",
            author_id = 1,
            category_id = 10
        });
        container.Comments.Add(new test.indexes.Comment
        {
            id = 1000,
            post_id = 100,
            author_id = 2,
            content = "Looks good",
            parent_id = null
        });
        container.Tags.Add(new test.indexes.Tag
        {
            id = 50,
            name = "csharp"
        });
        container.PostTags.Add(new test.indexes.PostTag
        {
            post_id = 100,
            tag_id = 50
        });

        var containerTitleMatches = container.Posts.SearchByTitle("binary");
        Assert(containerTitleMatches.Count == 1, "container SearchByTitle should find one binary post");
        Assert(containerTitleMatches[0].id == 100, "container SearchByTitle post id");

        var containerNameMatches = container.Categorys.SearchByName("tech");
        Assert(containerNameMatches.Count == 1, "container SearchByName should find category");
        Assert(containerNameMatches[0].id == 10, "container SearchByName category id");

        var containerDescriptionMatches = container.Categorys.SearchByDescription("tech");
        Assert(containerDescriptionMatches.Count == 1, "container SearchByDescription should find category");
        Assert(containerDescriptionMatches[0].name == "Tech", "container SearchByDescription category name");

        var containerRankMatches = container.Categorys.SearchByRank(7);
        Assert(containerRankMatches.Count == 1, "container SearchByRank should find category");
        Assert(containerRankMatches[0].id == 10, "container SearchByRank category id");

        var containerKindMatches = container.Categorys.SearchByKind(test.indexes.CategoryKind.Public);
        Assert(containerKindMatches.Count == 1, "container SearchByKind should find category");
        Assert(containerKindMatches[0].id == 10, "container SearchByKind category id");

        var path = System.IO.Path.Combine(
            System.IO.Path.GetTempPath(),
            "polygen-csharp-binary-ref-" + Guid.NewGuid().ToString("N") + ".bin");

        try
        {
            Schema.BinaryRefs.SchemaBinaryRefContext.SaveBinary(path, container);
            var ctx = Schema.BinaryRefs.SchemaBinaryRefContext.OpenBinary(path);

            var aliceById = ctx.Users.GetById(1);
            Assert(aliceById != null, "GetById should find alice");
            Assert(aliceById!.username == "alice", "alice username");
            Assert(aliceById.email == "alice@example.com", "alice email");

            var aliceByUsername = ctx.Users.GetByUsername("alice");
            Assert(aliceByUsername != null, "GetByUsername should find alice");
            Assert(aliceByUsername!.id == 1, "alice id by username");

            var ownedAlice = aliceById.ToOwned();
            Assert(ownedAlice.display_name == "Alice", "ToOwned display_name");

            var tech = ctx.Categorys.GetByName("Tech");
            Assert(tech != null, "GetByName should find category");
            Assert(tech!.description == "Technology", "optional category description");

            var techBySearchName = ctx.Categorys.SearchByName("tech");
            Assert(techBySearchName.Count == 1, "SearchByName should find category");
            Assert(techBySearchName[0].id == 10, "SearchByName category id");

            var postsByAlice = ctx.Posts.FindByAuthorId(1);
            Assert(postsByAlice.Count == 2, "FindByAuthorId should return two posts");
            Assert(postsByAlice[0].title == "Binary refs", "first post title");

            var binaryTitleMatches = ctx.Posts.SearchByTitle("binary");
            Assert(binaryTitleMatches.Count == 1, "SearchByTitle should find one binary post");
            Assert(binaryTitleMatches[0].id == 100, "SearchByTitle post id");

            var techDescriptionMatches = ctx.Categorys.SearchByDescription("tech");
            Assert(techDescriptionMatches.Count == 1, "SearchByDescription should find category");
            Assert(techDescriptionMatches[0].name == "Tech", "SearchByDescription category name");

            var rankMatches = ctx.Categorys.SearchByRank(7);
            Assert(rankMatches.Count == 1, "SearchByRank should find category");
            Assert(rankMatches[0].id == 10, "SearchByRank category id");

            var kindMatches = ctx.Categorys.SearchByKind(test.indexes.CategoryKind.Public);
            Assert(kindMatches.Count == 1, "SearchByKind should find category");
            Assert(kindMatches[0].id == 10, "SearchByKind category id");

            var postTags = ctx.PostTags.FindByPostId(100);
            Assert(postTags.Count == 1, "FindByPostId should return one post tag");
            Assert(postTags[0].tag_id == 50, "post tag id");

            Assert(ctx.Users.GetById(999) == null, "missing unique index should return null");
            Assert(ctx.Posts.FindByAuthorId(999).Count == 0, "missing group index should return empty list");

            var detachedRef = OpenUserRefAfterContextScope(path);
            Assert(detachedRef.username == "alice", "ref should keep binary owner alive after context scope");
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

    static void Main()
    {
        Console.WriteLine("=== Test Case 07: Indexes ===");

        TestUserCreation();
        TestCategoryCreation();
        TestPostWithForeignKeys();
        TestJunctionTable();
        TestBinarySerialization();
        TestBinaryRefLoad();

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
