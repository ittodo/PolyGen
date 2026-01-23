// Test Case 04: Inline Enums
// Tests enum definitions inside tables

import { TestEnums } from '../generated/04_inline_enums/typescript/schema';

// Test Order with inline enums
function testOrderInlineEnums(): void {
    console.log("  Testing Order with inline enums (Status, Priority)...");

    const order: TestEnums.Order = {
        id: 1,
        customerName: "John Doe",
        status: TestEnums.Status.Pending,
        priority: TestEnums.Priority.Normal,
    };

    console.assert(order.id === 1, "id should be 1");
    console.assert(order.customerName === "John Doe", "customerName should match");
    console.assert(order.status === TestEnums.Status.Pending, "status should be Pending");
    console.assert(order.priority === TestEnums.Priority.Normal, "priority should be Normal");

    // Test all status values
    console.assert(TestEnums.Status.Pending === 0, "Pending should be 0");
    console.assert(TestEnums.Status.Paid === 1, "Paid should be 1");
    console.assert(TestEnums.Status.Shipped === 2, "Shipped should be 2");
    console.assert(TestEnums.Status.Delivered === 3, "Delivered should be 3");
    console.assert(TestEnums.Status.Cancelled === 4, "Cancelled should be 4");

    console.log("    PASS");
}

// Test Task with inline enum
function testTaskInlineEnum(): void {
    console.log("  Testing Task with inline enum (State)...");

    const task: TestEnums.Task = {
        id: 1,
        title: "Complete project",
        state: TestEnums.State.InProgress,
    };

    console.assert(task.id === 1, "id should be 1");
    console.assert(task.state === TestEnums.State.InProgress, "state should be InProgress");

    // Test all state values
    console.assert(TestEnums.State.Todo === 0, "Todo should be 0");
    console.assert(TestEnums.State.InProgress === 1, "InProgress should be 1");
    console.assert(TestEnums.State.Done === 2, "Done should be 2");

    console.log("    PASS");
}

// Test GlobalStatus (named enum)
function testGlobalStatus(): void {
    console.log("  Testing GlobalStatus (named enum)...");

    console.assert(TestEnums.GlobalStatus.Unknown === 0, "Unknown should be 0");
    console.assert(TestEnums.GlobalStatus.Active === 1, "Active should be 1");
    console.assert(TestEnums.GlobalStatus.Disabled === 2, "Disabled should be 2");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 04: Inline Enums ===");
testOrderInlineEnums();
testTaskInlineEnum();
testGlobalStatus();
console.log("=== All tests passed! ===");
