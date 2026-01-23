// Test Case 06: Arrays and Optionals
// Tests array and optional field types

import { TestCollections } from '../generated/06_arrays_and_optionals/typescript/schema';

// Test ArrayTest with various array types
function testArrayTypes(): void {
    console.log("  Testing ArrayTest with various array types...");

    const arr: TestCollections.ArrayTest = {
        id: 1,
        intList: [1, 2, 3, 4, 5],
        stringList: ["a", "b", "c"],
        floatList: [1.1, 2.2, 3.3],
        boolList: [true, false, true],
        tags: [
            { name: "important", color: "red" },
            { name: "review", color: "yellow" },
        ],
    };

    console.assert(arr.id === 1, "id should be 1");
    console.assert(arr.intList.length === 5, "intList should have 5 elements");
    console.assert(arr.intList[0] === 1, "intList[0] should be 1");
    console.assert(arr.stringList.length === 3, "stringList should have 3 elements");
    console.assert(arr.tags.length === 2, "tags should have 2 elements");
    console.assert(arr.tags[0].name === "important", "tags[0].name should be important");

    console.log("    PASS");
}

// Test OptionalTest with optional fields
function testOptionalFields(): void {
    console.log("  Testing OptionalTest with optional fields...");

    // All optionals undefined
    const opt1: TestCollections.OptionalTest = {
        id: 1,
        requiredName: "Test",
        optInt: undefined,
        optString: undefined,
        optFloat: undefined,
        optBool: undefined,
        optTag: undefined,
    };

    console.assert(opt1.id === 1, "id should be 1");
    console.assert(opt1.requiredName === "Test", "requiredName should be Test");
    console.assert(opt1.optInt === undefined, "optInt should be undefined");

    // All optionals set
    const opt2: TestCollections.OptionalTest = {
        id: 2,
        requiredName: "Full",
        optInt: 42,
        optString: "hello",
        optFloat: 3.14,
        optBool: true,
        optTag: { name: "tag", color: "blue" },
    };

    console.assert(opt2.optInt === 42, "optInt should be 42");
    console.assert(opt2.optString === "hello", "optString should be hello");
    console.assert(opt2.optFloat === 3.14, "optFloat should be 3.14");
    console.assert(opt2.optBool === true, "optBool should be true");
    console.assert(opt2.optTag?.name === "tag", "optTag.name should be tag");

    console.log("    PASS");
}

// Test MixedTest with arrays and optionals
function testMixedArraysAndOptionals(): void {
    console.log("  Testing MixedTest with arrays and optionals...");

    const mixed: TestCollections.MixedTest = {
        id: 1,
        optTags: [
            { name: "tag1", color: "red" },
            { name: "tag2", color: "blue" },
        ],
        meta: {
            createdBy: "admin",
            updatedBy: undefined,
            version: 1,
        },
        history: [
            { createdBy: "user1", updatedBy: "user2", version: 1 },
            { createdBy: "user2", updatedBy: undefined, version: 2 },
        ],
    };

    console.assert(mixed.id === 1, "id should be 1");
    console.assert(mixed.optTags.length === 2, "optTags should have 2 elements");
    console.assert(mixed.meta?.createdBy === "admin", "meta.createdBy should be admin");
    console.assert(mixed.meta?.updatedBy === undefined, "meta.updatedBy should be undefined");
    console.assert(mixed.history.length === 2, "history should have 2 elements");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 06: Arrays and Optionals ===");
testArrayTypes();
testOptionalFields();
testMixedArraysAndOptionals();
console.log("=== All tests passed! ===");
