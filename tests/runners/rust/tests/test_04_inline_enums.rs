// Test Case 04: Inline Enums
// Tests enum definitions inside tables

use std::io::Cursor;

use polygen_test::schema::test_enums::{Order, Task, GlobalStatus, Status, Priority, State};
use polygen_test::schema_loaders::BinaryIO;

fn main() {
    println!("=== Test Case 04: Inline Enums ===");

    test_order_inline_enums();
    test_task_inline_enum();
    test_global_enum();
    test_binary_inline_enums();

    println!("=== All tests passed! ===");
}

fn test_order_inline_enums() {
    println!("  Testing Order inline enums...");

    let mut order = Order {
        id: 1,
        customer_name: "John Doe".to_string(),
        status: Status::Pending,
        priority: Priority::High,
    };

    assert_eq!(order.id, 1);
    assert_eq!(order.customer_name, "John Doe");
    assert_eq!(order.status, Status::Pending);
    assert_eq!(order.priority, Priority::High);

    // Test all status values
    order.status = Status::Paid;
    assert_eq!(order.status as i32, 1);

    order.status = Status::Shipped;
    assert_eq!(order.status as i32, 2);

    order.status = Status::Delivered;
    assert_eq!(order.status as i32, 3);

    order.status = Status::Cancelled;
    assert_eq!(order.status as i32, 4);

    println!("    PASS");
}

fn test_task_inline_enum() {
    println!("  Testing Task inline enum...");

    let mut task = Task {
        id: 1,
        title: "Complete tests".to_string(),
        state: State::Todo,
    };

    assert_eq!(task.state, State::Todo);

    task.state = State::InProgress;
    assert_eq!(task.state as i32, 1);

    task.state = State::Done;
    assert_eq!(task.state as i32, 2);

    println!("    PASS");
}

fn test_global_enum() {
    println!("  Testing global enum (GlobalStatus)...");

    let status = GlobalStatus::Unknown;
    assert_eq!(status as i32, 0);

    let status = GlobalStatus::Active;
    assert_eq!(status as i32, 1);

    let status = GlobalStatus::Disabled;
    assert_eq!(status as i32, 2);

    println!("    PASS");
}

fn test_binary_inline_enums() {
    println!("  Testing binary serialization with inline enums...");

    let original = Order {
        id: 12345,
        customer_name: "Test Customer".to_string(),
        status: Status::Shipped,
        priority: Priority::Urgent,
    };

    // Serialize
    let mut buffer = Vec::new();
    original.write_binary(&mut buffer).unwrap();

    // Deserialize
    let mut cursor = Cursor::new(&buffer);
    let loaded = Order::read_binary(&mut cursor).unwrap();

    assert_eq!(loaded.id, original.id);
    assert_eq!(loaded.customer_name, original.customer_name);
    assert_eq!(loaded.status, original.status);
    assert_eq!(loaded.priority, original.priority);

    println!("    PASS (serialized {} bytes)", buffer.len());
}
