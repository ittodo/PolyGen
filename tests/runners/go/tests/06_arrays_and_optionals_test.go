package polygen

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestGeneratedBinaryIORoundTripArraysAndOptionals(t *testing.T) {
	createdBy := "system"
	updatedBy := "editor"
	row := MixedTest{
		Id: 7,
		OptTags: []Tag{
			{Name: "alpha", Color: "red"},
			{Name: "beta", Color: "blue"},
		},
		Meta: &Metadata{
			CreatedBy: &createdBy,
			Version:   3,
		},
		History: []Metadata{
			{CreatedBy: &createdBy, UpdatedBy: &updatedBy, Version: 1},
			{Version: 2},
		},
	}

	var buf bytes.Buffer
	if err := row.WriteBinary(NewBinaryWriter(&buf)); err != nil {
		t.Fatalf("write mixed binary: %v", err)
	}
	loaded, err := ReadMixedTestBinary(NewBinaryReader(bytes.NewReader(buf.Bytes())))
	if err != nil {
		t.Fatalf("read mixed binary: %v", err)
	}
	if loaded.Id != row.Id || len(loaded.OptTags) != 2 || loaded.OptTags[1].Color != "blue" {
		t.Fatalf("mixed binary tags mismatch: %#v", loaded)
	}
	if loaded.Meta == nil || loaded.Meta.CreatedBy == nil || *loaded.Meta.CreatedBy != "system" || loaded.Meta.Version != 3 {
		t.Fatalf("mixed binary optional metadata mismatch: %#v", loaded.Meta)
	}
	if len(loaded.History) != 2 || loaded.History[0].UpdatedBy == nil || *loaded.History[0].UpdatedBy != "editor" {
		t.Fatalf("mixed binary history mismatch: %#v", loaded.History)
	}
}

func TestGeneratedBinaryTableLoaderRoundTrip(t *testing.T) {
	rows := []*ArrayTest{
		{
			Id:         1,
			IntList:    []int32{1, 2, 3},
			StringList: []string{"a", "b"},
			FloatList:  []float32{1.5, 2.5},
			BoolList:   []bool{true, false, true},
			Tags:       []Tag{{Name: "tag", Color: "green"}},
		},
	}
	path := filepath.Join(t.TempDir(), "array_tests.bin")
	if err := SaveArrayTestsToBinary(path, rows); err != nil {
		t.Fatalf("save array tests: %v", err)
	}
	loaded, err := LoadArrayTestsFromBinary(path)
	if err != nil {
		t.Fatalf("load array tests: %v", err)
	}
	if len(loaded) != 1 || loaded[0].Id != 1 || len(loaded[0].IntList) != 3 || loaded[0].Tags[0].Name != "tag" {
		t.Fatalf("binary table loader returned %#v", loaded)
	}
}

func TestGeneratedCsvLoaderParsesListsAndEmbedJSON(t *testing.T) {
	path := filepath.Join(t.TempDir(), "array_tests.csv")
	content := "id,int_list,string_list,float_list,bool_list,tags\n" +
		"1,\"1, 2,3\",\"alpha,beta\",\"1.25,2.5\",\"yes,no,true\",\"[{\"\"name\"\":\"\"tag-a\"\",\"\"color\"\":\"\"red\"\"},{\"\"name\"\":\"\"tag-b\"\",\"\"color\"\":\"\"blue\"\"}]\"\n"
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write csv: %v", err)
	}

	rows, err := LoadArrayTestsFromCsv(path)
	if err != nil {
		t.Fatalf("load array CSV: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected one row, got %d", len(rows))
	}
	row := rows[0]
	if row.Id != 1 || len(row.IntList) != 3 || row.IntList[2] != 3 {
		t.Fatalf("primitive int list mismatch: %#v", row.IntList)
	}
	if len(row.BoolList) != 3 || !row.BoolList[0] || row.BoolList[1] || !row.BoolList[2] {
		t.Fatalf("bool list mismatch: %#v", row.BoolList)
	}
	if len(row.Tags) != 2 || row.Tags[1].Name != "tag-b" || row.Tags[1].Color != "blue" {
		t.Fatalf("embed JSON list mismatch: %#v", row.Tags)
	}

	badListPath := filepath.Join(t.TempDir(), "bad_array_tests.csv")
	badList := "id,int_list,string_list,float_list,bool_list,tags\n" +
		"1,\"1,nope\",alpha,1.25,true,\"[]\"\n"
	if err := os.WriteFile(badListPath, []byte(badList), 0644); err != nil {
		t.Fatalf("write bad primitive csv: %v", err)
	}
	if _, err := LoadArrayTestsFromCsv(badListPath); err == nil {
		t.Fatalf("expected invalid primitive list item to fail")
	}

	badJSONPath := filepath.Join(t.TempDir(), "bad_tags.csv")
	badJSON := "id,int_list,string_list,float_list,bool_list,tags\n" +
		"1,1,alpha,1.25,true,\"[{bad json}]\"\n"
	if err := os.WriteFile(badJSONPath, []byte(badJSON), 0644); err != nil {
		t.Fatalf("write bad JSON csv: %v", err)
	}
	if _, err := LoadArrayTestsFromCsv(badJSONPath); err == nil {
		t.Fatalf("expected invalid embed JSON list to fail")
	}
}

func TestGeneratedCsvLoaderParsesOptionalAndHistoryEmbedJSON(t *testing.T) {
	path := filepath.Join(t.TempDir(), "mixed_tests.csv")
	content := "id,opt_tags,meta,history\n" +
		"7,\"[{\"\"name\"\":\"\"alpha\"\",\"\"color\"\":\"\"red\"\"}]\",\"{\"\"created_by\"\":\"\"system\"\",\"\"version\"\":3}\",\"[{\"\"updated_by\"\":\"\"editor\"\",\"\"version\"\":1},{\"\"version\"\":2}]\"\n"
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write mixed csv: %v", err)
	}

	rows, err := LoadMixedTestsFromCsv(path)
	if err != nil {
		t.Fatalf("load mixed CSV: %v", err)
	}
	if len(rows) != 1 {
		t.Fatalf("expected one mixed row, got %d", len(rows))
	}
	row := rows[0]
	if len(row.OptTags) != 1 || row.OptTags[0].Name != "alpha" {
		t.Fatalf("optional tag list mismatch: %#v", row.OptTags)
	}
	if row.Meta == nil || row.Meta.CreatedBy == nil || *row.Meta.CreatedBy != "system" || row.Meta.Version != 3 {
		t.Fatalf("optional embed JSON mismatch: %#v", row.Meta)
	}
	if len(row.History) != 2 || row.History[0].UpdatedBy == nil || *row.History[0].UpdatedBy != "editor" {
		t.Fatalf("history embed JSON list mismatch: %#v", row.History)
	}
}
