package polygen

import "testing"

func makeComplexStats() Stats {
	return Stats{
		Hp:           100,
		MaxHp:        100,
		Mp:           50,
		MaxMp:        50,
		Strength:     10,
		Agility:      8,
		Intelligence: 5,
		Vitality:     12,
	}
}

func makeComplexPlayer(id uint32, name string, level uint16) *Player {
	return &Player{
		Id:         id,
		Name:       name,
		Level:      level,
		Experience: 0,
		Stats:      makeComplexStats(),
		Position:   Vec3{X: 0, Y: 0, Z: 0},
		Status:     StatusOnline,
	}
}

func TestComplexContainerFieldAndUniqueValidation(t *testing.T) {
	invalidFields := NewSchemaContainer()
	invalidFields.Players.AddRow(makeComplexPlayer(
		1,
		"A name that is definitely longer than thirty two chars",
		101,
	))

	result := invalidFields.ValidateAll()
	if result.IsValid() {
		t.Fatalf("expected max_length/range violations")
	}
	if result.ErrorCount() != 2 {
		t.Fatalf("expected two field validation errors, got %d: %s", result.ErrorCount(), result.String())
	}
	seenMaxLength := false
	seenRange := false
	for _, err := range result.Errors {
		if err.ConstraintType == "MaxLength" && err.FieldName == "Name" {
			seenMaxLength = true
		}
		if err.ConstraintType == "Range" && err.FieldName == "Level" {
			seenRange = true
		}
	}
	if !seenMaxLength || !seenRange {
		t.Fatalf("expected max_length and range errors, got %#v", result.Errors)
	}

	invalidRegex := NewSchemaContainer()
	invalidRegex.Players.AddRow(makeComplexPlayer(2, "Invalid!", 10))

	regexResult := invalidRegex.ValidateAll()
	if regexResult.IsValid() {
		t.Fatalf("expected regex violation")
	}
	seenRegex := false
	for _, err := range regexResult.Errors {
		if err.ConstraintType == "Regex" && err.FieldName == "Name" {
			seenRegex = true
			break
		}
	}
	if !seenRegex {
		t.Fatalf("expected regex validation error, got %#v", regexResult.Errors)
	}

	duplicate := NewSchemaContainer()
	duplicate.Players.AddRow(makeComplexPlayer(1, "Hero A", 10))
	duplicate.Players.AddRow(makeComplexPlayer(1, "Hero B", 10))

	duplicateResult := duplicate.ValidateAll()
	if duplicateResult.IsValid() {
		t.Fatalf("expected duplicate primary key to fail validation")
	}
	seenUnique := false
	for _, err := range duplicateResult.Errors {
		if err.ConstraintType == "Unique" {
			seenUnique = true
			break
		}
	}
	if !seenUnique {
		t.Fatalf("expected unique validation error, got %#v", duplicateResult.Errors)
	}
}
