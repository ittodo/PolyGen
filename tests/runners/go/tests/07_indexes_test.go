package polygen

import "testing"

func TestIndexesAndForeignKeyValidation(t *testing.T) {
	container := NewSchemaContainer()
	user := &User{Id: 1, Username: "alice", Email: "alice@example.com", DisplayName: "Alice"}
	category := &Category{Id: 10, Name: "news"}
	post := &Post{Id: 100, Title: "title", Content: "body", AuthorId: user.Id, CategoryId: category.Id}
	tag := &Tag{Id: 200, Name: "featured"}
	postTag := &PostTag{PostId: post.Id, TagId: tag.Id}

	container.Users.AddRow(user)
	container.Categorys.AddRow(category)
	container.Posts.AddRow(post)
	container.Tags.AddRow(tag)
	container.PostTags.AddRow(postTag)

	if container.Users.GetByUsername("alice") != user {
		t.Fatalf("unique username index did not return inserted row")
	}
	if got := container.Posts.GetByAuthorId(user.Id); len(got) != 1 || got[0] != post {
		t.Fatalf("group author index returned %#v", got)
	}
	if got := container.PostTags.GetByTagId(tag.Id); len(got) != 1 || got[0] != postTag {
		t.Fatalf("junction tag index returned %#v", got)
	}
	if err := container.ValidateOrError(); err != nil {
		t.Fatalf("expected valid foreign keys: %v", err)
	}
}

func TestForeignKeyValidationRejectsMissingReference(t *testing.T) {
	container := NewSchemaContainer()
	container.Users.AddRow(&User{Id: 1, Username: "alice", Email: "alice@example.com", DisplayName: "Alice"})
	container.Posts.AddRow(&Post{Id: 100, Title: "title", Content: "body", AuthorId: 1, CategoryId: 999})

	result := container.ValidateAll()
	if result.IsValid() {
		t.Fatalf("expected missing category foreign key to be rejected")
	}
	if result.ErrorCount() != 1 {
		t.Fatalf("expected one validation error, got %d: %s", result.ErrorCount(), result.String())
	}
}
