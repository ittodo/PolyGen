package polygen

import "testing"

func TestInlineEnumConstantsAndContainer(t *testing.T) {
	order := &Order{
		Id:           1,
		CustomerName: "customer",
		Status:       StatusPaid,
		Priority:     PriorityHigh,
	}
	if order.Status != StatusPaid {
		t.Fatalf("expected StatusPaid, got %v", order.Status)
	}
	if order.Priority != PriorityHigh {
		t.Fatalf("expected PriorityHigh, got %v", order.Priority)
	}

	task := &Task{Id: 7, Title: "ship", State: StateInProgress}
	container := NewSchemaContainer()
	container.Orders.AddRow(order)
	container.Tasks.AddRow(task)

	if container.Orders.GetById(1) != order {
		t.Fatalf("order unique index did not return inserted row")
	}
	if container.Tasks.GetById(7) != task {
		t.Fatalf("task unique index did not return inserted row")
	}
	if err := container.ValidateOrError(); err != nil {
		t.Fatalf("expected inline enum container to validate: %v", err)
	}
}
