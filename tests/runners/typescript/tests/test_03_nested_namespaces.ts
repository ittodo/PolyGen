// Test Case 03: Nested Namespaces
// Tests deeply nested namespace structures

import { App, Util } from '../generated/03_nested_namespaces/typescript/schema';

// Test deeply nested table
function testDeeplyNestedTable(): void {
    console.log("  Testing deeply nested table (App.AppData.AppDataModels.User)...");

    const user: App.AppData.AppDataModels.User = {
        id: 1,
        username: "testuser",
    };

    console.assert(user.id === 1, "id should be 1");
    console.assert(user.username === "testuser", "username should be testuser");

    console.log("    PASS");
}

// Test nested enum
function testNestedEnum(): void {
    console.log("  Testing nested enum (App.AppData.AppDataEnums.Permission)...");

    const read = App.AppData.AppDataEnums.Permission.Read;
    console.assert(read === 1, "Read should be 1");

    const write = App.AppData.AppDataEnums.Permission.Write;
    console.assert(write === 2, "Write should be 2");

    const admin = App.AppData.AppDataEnums.Permission.Admin;
    console.assert(admin === 3, "Admin should be 3");

    console.log("    PASS");
}

// Test cross-namespace reference (UserService)
function testCrossNamespaceReference(): void {
    console.log("  Testing cross-namespace reference (UserService)...");

    const service: App.AppServices.UserService = {
        id: 1,
        targetUserId: 100,
        permission: App.AppData.AppDataEnums.Permission.Admin,
    };

    console.assert(service.id === 1, "id should be 1");
    console.assert(service.targetUserId === 100, "targetUserId should be 100");
    console.assert(service.permission === App.AppData.AppDataEnums.Permission.Admin, "permission should be Admin");

    console.log("    PASS");
}

// Test separate namespace (Util.Config)
function testSeparateNamespace(): void {
    console.log("  Testing separate namespace (Util.Config)...");

    const config: Util.Config = {
        key: "app.setting",
        value: "enabled",
    };

    console.assert(config.key === "app.setting", "key should match");
    console.assert(config.value === "enabled", "value should match");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 03: Nested Namespaces ===");
testDeeplyNestedTable();
testNestedEnum();
testCrossNamespaceReference();
testSeparateNamespace();
console.log("=== All tests passed! ===");
