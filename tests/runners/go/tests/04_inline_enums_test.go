package polygen

import (
	"bytes"
	"testing"
)

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

func TestBinaryInvalidEnumRejected(t *testing.T) {
	var buf bytes.Buffer
	writer := NewBinaryWriter(&buf)
	if err := writer.WriteUint32(1); err != nil {
		t.Fatalf("write id: %v", err)
	}
	if err := writer.WriteString("Invalid Customer"); err != nil {
		t.Fatalf("write customer: %v", err)
	}
	if err := writer.WriteInt32(999); err != nil {
		t.Fatalf("write invalid status: %v", err)
	}
	if err := writer.WriteInt32(int32(PriorityNormal)); err != nil {
		t.Fatalf("write priority: %v", err)
	}

	if _, err := ReadOrderBinary(NewBinaryReader(bytes.NewReader(buf.Bytes()))); err == nil {
		t.Fatalf("expected invalid binary enum discriminant to fail")
	}
}

func TestBinaryInvalidEnumWriteRejected(t *testing.T) {
	order := &Order{
		Id:           1,
		CustomerName: "Invalid Customer",
		Status:       Status(999),
		Priority:     PriorityNormal,
	}
	var buf bytes.Buffer
	if err := order.WriteBinary(NewBinaryWriter(&buf)); err == nil {
		t.Fatalf("expected invalid enum write to fail")
	}
}
