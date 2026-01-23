// Test Case 01: Basic Types
// Tests all primitive types and simple struct generation

import { TestBasic } from '../generated/01_basic_types/typescript/schema';

// Test AllTypes creation
function testAllTypesCreation(): void {
    console.log("  Testing AllTypes creation...");

    const obj: TestBasic.AllTypes = {
        valU8: 255,
        valU16: 65535,
        valU32: 4294967295,
        valU64: 18446744073709551615,
        valI8: -128,
        valI16: -32768,
        valI32: -2147483648,
        valI64: -9223372036854775808,
        valF32: 3.14159,
        valF64: 3.141592653589793,
        valBool: true,
        valString: "Hello, World!",
        valBytes: new Uint8Array([0x00, 0x01, 0x02, 0xFF]),
    };

    console.assert(obj.valU8 === 255, "valU8 should be 255");
    console.assert(obj.valU16 === 65535, "valU16 should be 65535");
    console.assert(obj.valU32 === 4294967295, "valU32 should be 4294967295");
    console.assert(obj.valBool === true, "valBool should be true");
    console.assert(obj.valString === "Hello, World!", "valString should match");

    console.log("    PASS");
}

// Test SimpleStruct
function testSimpleStruct(): void {
    console.log("  Testing SimpleStruct...");

    const s: TestBasic.SimpleStruct = {
        id: 1,
        name: "Test",
        value: -42,
    };

    console.assert(s.id === 1, "id should be 1");
    console.assert(s.name === "Test", "name should be Test");
    console.assert(s.value === -42, "value should be -42");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 01: Basic Types ===");
testAllTypesCreation();
testSimpleStruct();
console.log("=== All tests passed! ===");
