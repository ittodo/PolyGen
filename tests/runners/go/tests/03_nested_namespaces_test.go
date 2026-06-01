package polygen

import "testing"

func TestNestedNamespaceContainerIncludesDeepTable(t *testing.T) {
	container := NewSchemaContainer()
	user := &User{Id: 7, Username: "deep_user"}

	container.Users.AddRow(user)

	if container.Users.Count() != 1 {
		t.Fatalf("expected one deep namespace user, got %d", container.Users.Count())
	}
	if got := container.Users.GetById(user.Id); got != user {
		t.Fatalf("deep namespace user index returned %#v", got)
	}
}

func TestNestedNamespaceContainerIncludesSiblingTables(t *testing.T) {
	container := NewSchemaContainer()
	service := &UserService{Id: 1, TargetUserId: 7, Permission: PermissionWrite}
	config := &Config{Key: "debug_mode", Value: "true"}

	container.UserServices.AddRow(service)
	container.Configs.AddRow(config)

	if container.UserServices.Count() != 1 {
		t.Fatalf("expected one user service, got %d", container.UserServices.Count())
	}
	if got := container.Configs.GetByKey(config.Key); got != config {
		t.Fatalf("config index returned %#v", got)
	}
}
