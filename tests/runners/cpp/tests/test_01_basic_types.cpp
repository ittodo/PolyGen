// Test Case 01: Basic Types
// Tests all primitive types and simple struct generation

#include <iostream>
#include <cassert>
#include <cmath>
#include <limits>

#include "schema.hpp"
#include "schema_loaders.hpp"

using namespace test::basic;

void test_all_types_creation() {
    std::cout << "  Testing AllTypes creation..." << std::endl;

    AllTypes obj;
    obj.val_u8 = 255;
    obj.val_u16 = 65535;
    obj.val_u32 = 4294967295;
    obj.val_u64 = 18446744073709551615ULL;
    obj.val_i8 = -128;
    obj.val_i16 = -32768;
    obj.val_i32 = -2147483648;
    obj.val_i64 = -9223372036854775807LL - 1;
    obj.val_f32 = 3.14159f;
    obj.val_f64 = 3.141592653589793;
    obj.val_bool = true;
    obj.val_string = "Hello, World!";
    obj.val_bytes = {0x00, 0x01, 0x02, 0xFF};

    assert(obj.val_u8 == 255);
    assert(obj.val_u16 == 65535);
    assert(obj.val_u32 == 4294967295);
    assert(obj.val_u64 == 18446744073709551615ULL);
    assert(obj.val_i8 == -128);
    assert(obj.val_i16 == -32768);
    assert(obj.val_i32 == -2147483648);
    assert(obj.val_bool == true);
    assert(obj.val_string == "Hello, World!");
    assert(obj.val_bytes.size() == 4);

    std::cout << "    PASS" << std::endl;
}

void test_simple_struct() {
    std::cout << "  Testing SimpleStruct..." << std::endl;

    SimpleStruct s;
    s.id = 1;
    s.name = "Test";
    s.value = -42;

    assert(s.id == 1);
    assert(s.name == "Test");
    assert(s.value == -42);

    std::cout << "    PASS" << std::endl;
}

void test_binary_serialization() {
    std::cout << "  Testing binary serialization..." << std::endl;

    SimpleStruct original;
    original.id = 12345;
    original.name = "Binary Test";
    original.value = -999;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_SimpleStruct(writer, original);
    }

    // Deserialize
    SimpleStruct loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_SimpleStruct(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.name == original.name);
    assert(loaded.value == original.value);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

void test_all_types_binary() {
    std::cout << "  Testing AllTypes binary serialization..." << std::endl;

    AllTypes original;
    original.val_u8 = 200;
    original.val_u16 = 50000;
    original.val_u32 = 3000000000;
    original.val_u64 = 10000000000000ULL;
    original.val_i8 = -100;
    original.val_i16 = -20000;
    original.val_i32 = -1000000000;
    original.val_i64 = -5000000000000LL;
    original.val_f32 = 1.5f;
    original.val_f64 = 2.718281828;
    original.val_bool = false;
    original.val_string = "Test string with special chars: !@#$%";
    original.val_bytes = {0xDE, 0xAD, 0xBE, 0xEF};

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_AllTypes(writer, original);
    }

    // Deserialize
    AllTypes loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_AllTypes(reader);
    }

    assert(loaded.val_u8 == original.val_u8);
    assert(loaded.val_u16 == original.val_u16);
    assert(loaded.val_u32 == original.val_u32);
    assert(loaded.val_u64 == original.val_u64);
    assert(loaded.val_i8 == original.val_i8);
    assert(loaded.val_i16 == original.val_i16);
    assert(loaded.val_i32 == original.val_i32);
    assert(loaded.val_i64 == original.val_i64);
    assert(std::abs(loaded.val_f32 - original.val_f32) < 0.0001f);
    assert(std::abs(loaded.val_f64 - original.val_f64) < 0.0000001);
    assert(loaded.val_bool == original.val_bool);
    assert(loaded.val_string == original.val_string);
    assert(loaded.val_bytes == original.val_bytes);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 01: Basic Types ===" << std::endl;

    test_all_types_creation();
    test_simple_struct();
    test_binary_serialization();
    test_all_types_binary();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
