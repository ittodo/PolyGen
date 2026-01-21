// Test Case 05: Embedded Structs
// Tests embed definitions and nested embeds

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

    static void TestAddressEmbed()
    {
        Console.WriteLine("  Testing Address embed...");

        var addr = new test.embed.Address
        {
            street = "123 Main St",
            city = "Seoul",
            country = "Korea",
            postal_code = "12345"
        };

        Assert(addr.street == "123 Main St", "street");
        Assert(addr.city == "Seoul", "city");
        Assert(addr.country == "Korea", "country");
        Assert(addr.postal_code == "12345", "postal_code");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestCompanyWithEmbeds()
    {
        Console.WriteLine("  Testing Company with embedded types...");

        var company = new test.embed.Company
        {
            id = 1,
            name = "Tech Corp",
            address = new test.embed.Address
            {
                street = "456 Tech Blvd",
                city = "San Francisco",
                country = "USA",
                postal_code = "94102"
            },
            contact = new test.embed.ContactInfo
            {
                email = "info@techcorp.com",
                phone = "555-1234"
            }
        };

        Assert(company.id == 1, "id");
        Assert(company.name == "Tech Corp", "name");
        Assert(company.address.city == "San Francisco", "address.city");
        Assert(company.contact.email == "info@techcorp.com", "contact.email");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestPersonWithInlineEmbed()
    {
        Console.WriteLine("  Testing Person with inline embed...");

        var person = new test.embed.Person
        {
            id = 1,
            name = "John Doe",
            details = new test.embed.Details
            {
                birth_date = "1990-01-15",
                nationality = "Korean"
            },
            home_address = new test.embed.Address
            {
                street = "789 Home St",
                city = "Busan",
                country = "Korea",
                postal_code = "67890"
            }
        };

        Assert(person.id == 1, "id");
        Assert(person.name == "John Doe", "name");
        Assert(person.details.birth_date == "1990-01-15", "details.birth_date");
        Assert(person.details.nationality == "Korean", "details.nationality");
        Assert(person.home_address.city == "Busan", "home_address.city");

        Console.WriteLine("    PASS");
        passed++;
    }

    static void TestBinaryWithEmbeds()
    {
        Console.WriteLine("  Testing binary serialization with embeds...");

        var original = new test.embed.Company
        {
            id = 42,
            name = "Test Company",
            address = new test.embed.Address
            {
                street = "Test Street",
                city = "Test City",
                country = "Test Country",
                postal_code = "00000"
            },
            contact = new test.embed.ContactInfo
            {
                email = "test@test.com",
                phone = "000-0000"
            }
        };

        // Serialize
        using var ms = new System.IO.MemoryStream();
        using var writer = new System.IO.BinaryWriter(ms);
        test.embed.BinaryWriters.WriteCompany(writer, original);

        // Deserialize
        ms.Position = 0;
        using var reader = new System.IO.BinaryReader(ms);
        var loaded = test.embed.BinaryReaders.ReadCompany(reader);

        Assert(loaded.id == original.id, "id mismatch");
        Assert(loaded.name == original.name, "name mismatch");
        Assert(loaded.address.city == original.address.city, "address.city mismatch");
        Assert(loaded.contact.email == original.contact.email, "contact.email mismatch");

        Console.WriteLine($"    PASS (serialized {ms.Length} bytes)");
        passed++;
    }

    static void Main()
    {
        Console.WriteLine("=== Test Case 05: Embedded Structs ===");

        TestAddressEmbed();
        TestCompanyWithEmbeds();
        TestPersonWithInlineEmbed();
        TestBinaryWithEmbeds();

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
