// Test Case 05: Embedded Structs
// Tests embed definitions and nested embeds

#include <iostream>
#include <cassert>
#include <cmath>

#include "schema.hpp"
#include "schema_loaders.hpp"

using namespace test::embed;

void test_address_embed() {
    std::cout << "  Testing Address embed..." << std::endl;

    Address addr;
    addr.street = "123 Main St";
    addr.city = "Seoul";
    addr.country = "South Korea";
    addr.postal_code = "12345";

    assert(addr.street == "123 Main St");
    assert(addr.city == "Seoul");
    assert(addr.country == "South Korea");
    assert(addr.postal_code == "12345");

    std::cout << "    PASS" << std::endl;
}

void test_company_with_embeds() {
    std::cout << "  Testing Company with embedded types..." << std::endl;

    Company company;
    company.id = 1;
    company.name = "Test Corp";
    company.address.street = "456 Business Ave";
    company.address.city = "Tokyo";
    company.address.country = "Japan";
    company.address.postal_code = "100-0001";
    company.contact.email = "contact@test.com";
    company.contact.phone = "+81-3-1234-5678";

    assert(company.id == 1);
    assert(company.name == "Test Corp");
    assert(company.address.city == "Tokyo");
    assert(company.contact.email == "contact@test.com");
    assert(company.contact.phone.has_value());
    assert(*company.contact.phone == "+81-3-1234-5678");

    std::cout << "    PASS" << std::endl;
}

void test_person_inline_embed() {
    std::cout << "  Testing Person with inline embed..." << std::endl;

    Person person;
    person.id = 1;
    person.name = "John Doe";
    person.details.birth_date = "1990-01-01";
    person.details.nationality = "Korean";
    person.home_address.street = "789 Home St";
    person.home_address.city = "Busan";
    person.home_address.country = "South Korea";
    person.home_address.postal_code = "48000";
    // work_address is optional, leave it empty

    assert(person.id == 1);
    assert(person.details.birth_date == "1990-01-01");
    assert(person.home_address.city == "Busan");
    assert(!person.work_address.has_value());

    // Set optional work address
    person.work_address = Address{};
    person.work_address->street = "Work St";
    person.work_address->city = "Seoul";
    person.work_address->country = "South Korea";
    person.work_address->postal_code = "06000";

    assert(person.work_address.has_value());
    assert(person.work_address->city == "Seoul");

    std::cout << "    PASS" << std::endl;
}

void test_nested_embed() {
    std::cout << "  Testing Product with nested embeds..." << std::endl;

    Product product;
    product.id = 1;
    product.name = "Box";
    product.size.width.value = 10.0f;
    product.size.width.unit = "cm";
    product.size.height.value = 20.0f;
    product.size.height.unit = "cm";
    product.size.depth.value = 5.0f;
    product.size.depth.unit = "cm";

    assert(product.id == 1);
    assert(product.name == "Box");
    assert(std::abs(product.size.width.value - 10.0f) < 0.001f);
    assert(product.size.width.unit == "cm");
    assert(std::abs(product.size.height.value - 20.0f) < 0.001f);
    assert(std::abs(product.size.depth.value - 5.0f) < 0.001f);

    std::cout << "    PASS" << std::endl;
}

void test_binary_embeds() {
    std::cout << "  Testing binary serialization with embeds..." << std::endl;

    Company original;
    original.id = 999;
    original.name = "Serialization Test Corp";
    original.address.street = "Binary St";
    original.address.city = "Test City";
    original.address.country = "Test Country";
    original.address.postal_code = "00000";
    original.contact.email = "test@binary.com";
    original.contact.phone = std::nullopt;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_Company(writer, original);
    }

    // Deserialize
    Company loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_Company(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.name == original.name);
    assert(loaded.address.street == original.address.street);
    assert(loaded.address.city == original.address.city);
    assert(loaded.contact.email == original.contact.email);
    assert(!loaded.contact.phone.has_value());

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 05: Embedded Structs ===" << std::endl;

    test_address_embed();
    test_company_with_embeds();
    test_person_inline_embed();
    test_nested_embed();
    test_binary_embeds();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
