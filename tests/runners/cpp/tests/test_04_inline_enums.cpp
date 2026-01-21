// Test Case 04: Inline Enums
// Tests enum definitions inside tables

#include <iostream>
#include <cassert>

#include "schema.hpp"
#include "schema_loaders.hpp"

using namespace test::enums;

void test_order_inline_enums() {
    std::cout << "  Testing Order inline enums..." << std::endl;

    Order order;
    order.id = 1;
    order.customer_name = "John Doe";
    order.status = Order::Status::Pending;
    order.priority = Order::Priority::High;

    assert(order.id == 1);
    assert(order.customer_name == "John Doe");
    assert(order.status == Order::Status::Pending);
    assert(order.priority == Order::Priority::High);

    // Test all status values
    order.status = Order::Status::Paid;
    assert(static_cast<int32_t>(order.status) == 1);

    order.status = Order::Status::Shipped;
    assert(static_cast<int32_t>(order.status) == 2);

    order.status = Order::Status::Delivered;
    assert(static_cast<int32_t>(order.status) == 3);

    order.status = Order::Status::Cancelled;
    assert(static_cast<int32_t>(order.status) == 4);

    std::cout << "    PASS" << std::endl;
}

void test_task_inline_enum() {
    std::cout << "  Testing Task inline enum..." << std::endl;

    Task task;
    task.id = 1;
    task.title = "Complete tests";
    task.state = Task::State::Todo;

    assert(task.state == Task::State::Todo);

    task.state = Task::State::InProgress;
    assert(static_cast<int32_t>(task.state) == 1);

    task.state = Task::State::Done;
    assert(static_cast<int32_t>(task.state) == 2);

    std::cout << "    PASS" << std::endl;
}

void test_global_enum() {
    std::cout << "  Testing global enum (GlobalStatus)..." << std::endl;

    GlobalStatus status = GlobalStatus::Unknown;
    assert(static_cast<int32_t>(status) == 0);

    status = GlobalStatus::Active;
    assert(static_cast<int32_t>(status) == 1);

    status = GlobalStatus::Disabled;
    assert(static_cast<int32_t>(status) == 2);

    std::cout << "    PASS" << std::endl;
}

void test_binary_inline_enums() {
    std::cout << "  Testing binary serialization with inline enums..." << std::endl;

    Order original;
    original.id = 12345;
    original.customer_name = "Test Customer";
    original.status = Order::Status::Shipped;
    original.priority = Order::Priority::Urgent;

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_Order(writer, original);
    }

    // Deserialize
    Order loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_Order(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.customer_name == original.customer_name);
    assert(loaded.status == original.status);
    assert(loaded.priority == original.priority);

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 04: Inline Enums ===" << std::endl;

    test_order_inline_enums();
    test_task_inline_enum();
    test_global_enum();
    test_binary_inline_enums();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
