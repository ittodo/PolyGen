// Test Case 05: Embedded Structs
// Tests embed definitions and nested embeds

use std::io::Cursor;

use polygen_test::schema::test_embed::{Address, ContactInfo, Company, Person, Details};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 05: Embedded Structs ===");

    test_address_embed();
    test_company_with_embeds();
    test_person_with_inline_embed();
    test_binary_with_embeds();

    println!("=== All tests passed! ===");
}

fn test_address_embed() {
    println!("  Testing Address embed...");

    let addr = Address {
        street: "123 Main St".to_string(),
        city: "Seoul".to_string(),
        country: "Korea".to_string(),
        postal_code: "12345".to_string(),
    };

    assert_eq!(addr.street, "123 Main St");
    assert_eq!(addr.city, "Seoul");
    assert_eq!(addr.country, "Korea");
    assert_eq!(addr.postal_code, "12345");

    println!("    PASS");
}

fn test_company_with_embeds() {
    println!("  Testing Company with embedded types...");

    let company = Company {
        id: 1,
        name: "Tech Corp".to_string(),
        address: Address {
            street: "456 Tech Blvd".to_string(),
            city: "San Francisco".to_string(),
            country: "USA".to_string(),
            postal_code: "94102".to_string(),
        },
        contact: ContactInfo {
            email: "info@techcorp.com".to_string(),
            phone: Some("555-1234".to_string()),
        },
    };

    assert_eq!(company.id, 1);
    assert_eq!(company.name, "Tech Corp");
    assert_eq!(company.address.city, "San Francisco");
    assert_eq!(company.contact.email, "info@techcorp.com");

    println!("    PASS");
}

fn test_person_with_inline_embed() {
    println!("  Testing Person with inline embed...");

    let person = Person {
        id: 1,
        name: "John Doe".to_string(),
        details: Details {
            birth_date: "1990-01-15".to_string(),
            nationality: "Korean".to_string(),
        },
        home_address: Address {
            street: "789 Home St".to_string(),
            city: "Busan".to_string(),
            country: "Korea".to_string(),
            postal_code: "67890".to_string(),
        },
        work_address: None,
    };

    assert_eq!(person.id, 1);
    assert_eq!(person.name, "John Doe");
    assert_eq!(person.details.birth_date, "1990-01-15");
    assert_eq!(person.details.nationality, "Korean");
    assert_eq!(person.home_address.city, "Busan");
    assert!(person.work_address.is_none());

    println!("    PASS");
}

fn test_binary_with_embeds() {
    println!("  Testing binary serialization with embeds...");

    let original = Company {
        id: 42,
        name: "Test Company".to_string(),
        address: Address {
            street: "Test Street".to_string(),
            city: "Test City".to_string(),
            country: "Test Country".to_string(),
            postal_code: "00000".to_string(),
        },
        contact: ContactInfo {
            email: "test@test.com".to_string(),
            phone: Some("000-0000".to_string()),
        },
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = Company::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.address.city, original.address.city);
    assert_eq!(loaded.contact.email, original.contact.email);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
