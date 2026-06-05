// Test Case 06: Arrays and Optionals
// Tests array and optional field types

#include <iostream>
#include <cassert>
#include <cmath>
#include <sstream>
#include <stdexcept>
#include <string>

#include "schema.hpp"
#include "schema_loaders.hpp"

using namespace test::collections;

std::string csv_quote(const std::string& value) {
    std::string out = "\"";
    for (char ch : value) {
        if (ch == '"') {
            out += "\"\"";
        } else {
            out += ch;
        }
    }
    out += "\"";
    return out;
}

void test_array_primitives() {
    std::cout << "  Testing ArrayTest with primitive arrays..." << std::endl;

    ArrayTest arr;
    arr.id = 1;
    arr.int_list = {1, 2, 3, 4, 5};
    arr.string_list = {"one", "two", "three"};
    arr.float_list = {1.1f, 2.2f, 3.3f};
    arr.bool_list = {true, false, true};
    arr.tags = {};  // empty array

    assert(arr.int_list.size() == 5);
    assert(arr.int_list[0] == 1);
    assert(arr.int_list[4] == 5);
    assert(arr.string_list.size() == 3);
    assert(arr.string_list[1] == "two");
    assert(arr.float_list.size() == 3);
    assert(std::abs(arr.float_list[0] - 1.1f) < 0.001f);
    assert(arr.bool_list.size() == 3);
    assert(arr.bool_list[0] == true);
    assert(arr.bool_list[1] == false);

    std::cout << "    PASS" << std::endl;
}

void test_array_complex_types() {
    std::cout << "  Testing ArrayTest with complex type arrays..." << std::endl;

    ArrayTest arr;
    arr.id = 2;
    arr.int_list = {};
    arr.string_list = {};
    arr.float_list = {};
    arr.bool_list = {};

    // Add tags
    Tag tag1;
    tag1.name = "Important";
    tag1.color = "red";

    Tag tag2;
    tag2.name = "Review";
    tag2.color = "yellow";

    arr.tags = {tag1, tag2};

    assert(arr.tags.size() == 2);
    assert(arr.tags[0].name == "Important");
    assert(arr.tags[0].color == "red");
    assert(arr.tags[1].name == "Review");

    std::cout << "    PASS" << std::endl;
}

void test_optional_primitives() {
    std::cout << "  Testing OptionalTest with optional primitives..." << std::endl;

    OptionalTest opt;
    opt.id = 1;
    opt.required_name = "Test";
    opt.opt_int = std::nullopt;
    opt.opt_string = std::nullopt;
    opt.opt_float = std::nullopt;
    opt.opt_bool = std::nullopt;
    opt.opt_tag = std::nullopt;

    assert(opt.required_name == "Test");
    assert(!opt.opt_int.has_value());
    assert(!opt.opt_string.has_value());
    assert(!opt.opt_float.has_value());
    assert(!opt.opt_bool.has_value());
    assert(!opt.opt_tag.has_value());

    // Set values
    opt.opt_int = 42;
    opt.opt_string = "optional value";
    opt.opt_float = 3.14159;
    opt.opt_bool = true;

    assert(opt.opt_int.has_value());
    assert(*opt.opt_int == 42);
    assert(opt.opt_string.has_value());
    assert(*opt.opt_string == "optional value");
    assert(opt.opt_float.has_value());
    assert(std::abs(*opt.opt_float - 3.14159) < 0.0001);
    assert(opt.opt_bool.has_value());
    assert(*opt.opt_bool == true);

    std::cout << "    PASS" << std::endl;
}

void test_optional_complex_type() {
    std::cout << "  Testing OptionalTest with optional complex type..." << std::endl;

    OptionalTest opt;
    opt.id = 2;
    opt.required_name = "Complex Test";
    opt.opt_tag = std::nullopt;

    assert(!opt.opt_tag.has_value());

    Tag tag;
    tag.name = "Optional Tag";
    tag.color = "blue";
    opt.opt_tag = tag;

    assert(opt.opt_tag.has_value());
    assert(opt.opt_tag->name == "Optional Tag");
    assert(opt.opt_tag->color == "blue");

    std::cout << "    PASS" << std::endl;
}

void test_mixed_arrays_optionals() {
    std::cout << "  Testing MixedTest with mixed types..." << std::endl;

    MixedTest mixed;
    mixed.id = 1;
    mixed.opt_tags = {};  // regular array, starts empty
    mixed.meta = std::nullopt;
    mixed.history = {};

    assert(mixed.opt_tags.empty());
    assert(!mixed.meta.has_value());
    assert(mixed.history.empty());

    // Set array of tags
    Tag t1, t2;
    t1.name = "Tag1";
    t1.color = "green";
    t2.name = "Tag2";
    t2.color = "purple";
    mixed.opt_tags = {t1, t2};

    assert(mixed.opt_tags.size() == 2);

    // Set optional metadata
    Metadata meta;
    meta.created_by = "user1";
    meta.updated_by = std::nullopt;
    meta.version = 1;
    mixed.meta = meta;

    assert(mixed.meta.has_value());
    assert(mixed.meta->created_by.has_value());
    assert(*mixed.meta->created_by == "user1");
    assert(!mixed.meta->updated_by.has_value());

    // Add to history array
    Metadata hist1, hist2;
    hist1.created_by = "admin";
    hist1.updated_by = "admin";
    hist1.version = 0;
    hist2.created_by = "user1";
    hist2.updated_by = std::nullopt;
    hist2.version = 1;
    mixed.history = {hist1, hist2};

    assert(mixed.history.size() == 2);
    assert(mixed.history[0].version == 0);
    assert(mixed.history[1].version == 1);

    std::cout << "    PASS" << std::endl;
}

void test_csv_loaders() {
    std::cout << "  Testing generated CSV loaders..." << std::endl;

    {
        std::stringstream csv;
        csv << "id,int_list,string_list,float_list,bool_list,tags\n";
        csv << "3,\"1,2\",\"one,two\",\"1.5,2.5\",\"yes,no,1,0\","
            << csv_quote("[{\"name\":\"Important\",\"color\":\"red\"},{\"name\":\"Review\",\"color\":\"yellow\"}]")
            << "\n";

        auto rows = polygen_loaders::load_ArrayTest_from_csv(csv);
        assert(rows.size() == 1);
        assert(rows[0].int_list.size() == 2);
        assert(rows[0].int_list[1] == 2);
        assert(rows[0].string_list[0] == "one");
        assert(std::abs(rows[0].float_list[1] - 2.5f) < 0.001f);
        assert(rows[0].bool_list.size() == 4);
        assert(rows[0].bool_list[0] == true);
        assert(rows[0].bool_list[1] == false);
        assert(rows[0].tags.size() == 2);
        assert(rows[0].tags[1].color == "yellow");
    }

    {
        std::stringstream csv;
        csv << "id,required_name,opt_int,opt_string,opt_float,opt_bool,opt_tag\n";
        csv << "4,WithTag,,,3.25,yes,"
            << csv_quote("{\"name\":\"Optional\",\"color\":\"blue\"}")
            << "\n";

        auto rows = polygen_loaders::load_OptionalTest_from_csv(csv);
        assert(rows.size() == 1);
        assert(!rows[0].opt_int.has_value());
        assert(rows[0].opt_float.has_value());
        assert(std::abs(*rows[0].opt_float - 3.25) < 0.0001);
        assert(rows[0].opt_bool.has_value());
        assert(*rows[0].opt_bool == true);
        assert(rows[0].opt_tag.has_value());
        assert(rows[0].opt_tag->name == "Optional");
    }

    {
        std::stringstream csv;
        csv << "id,opt_tags,meta,history\n";
        csv << "5,"
            << csv_quote("[{\"name\":\"One\",\"color\":\"green\"}]")
            << ","
            << csv_quote("{\"created_by\":\"alice\",\"updated_by\":null,\"version\":3}")
            << ","
            << csv_quote("[{\"created_by\":null,\"updated_by\":\"bob\",\"version\":1}]")
            << "\n";

        auto rows = polygen_loaders::load_MixedTest_from_csv(csv);
        assert(rows.size() == 1);
        assert(rows[0].opt_tags.size() == 1);
        assert(rows[0].meta.has_value());
        assert(rows[0].meta->created_by.has_value());
        assert(*rows[0].meta->created_by == "alice");
        assert(rows[0].meta->version == 3);
        assert(rows[0].history.size() == 1);
        assert(rows[0].history[0].updated_by.has_value());
        assert(*rows[0].history[0].updated_by == "bob");
    }

    {
        std::stringstream csv;
        csv << "id,int_list,string_list,float_list,bool_list,tags\n";
        csv << "6,,,,,not-json\n";
        bool failed = false;
        try {
            (void)polygen_loaders::load_ArrayTest_from_csv(csv);
        } catch (const std::exception& ex) {
            failed = std::string(ex.what()).find("invalid JSON array for tags") != std::string::npos;
        }
        assert(failed);
    }

    std::cout << "    PASS" << std::endl;
}

void test_binary_arrays_optionals() {
    std::cout << "  Testing binary serialization with arrays and optionals..." << std::endl;

    ArrayTest original;
    original.id = 123;
    original.int_list = {10, 20, 30};
    original.string_list = {"a", "b", "c"};
    original.float_list = {1.5f, 2.5f};
    original.bool_list = {true, false};

    Tag t;
    t.name = "Test";
    t.color = "white";
    original.tags = {t};

    // Serialize
    std::vector<uint8_t> buffer;
    {
        polygen::BinaryWriter writer(buffer);
        polygen_loaders::write_ArrayTest(writer, original);
    }

    // Deserialize
    ArrayTest loaded;
    {
        polygen::BinaryReader reader(buffer);
        loaded = polygen_loaders::read_ArrayTest(reader);
    }

    assert(loaded.id == original.id);
    assert(loaded.int_list == original.int_list);
    assert(loaded.string_list == original.string_list);
    assert(loaded.bool_list == original.bool_list);
    assert(loaded.tags.size() == 1);
    assert(loaded.tags[0].name == "Test");

    std::cout << "    PASS (serialized " << buffer.size() << " bytes)" << std::endl;
}

int main() {
    std::cout << "=== Test Case 06: Arrays and Optionals ===" << std::endl;

    test_array_primitives();
    test_array_complex_types();
    test_optional_primitives();
    test_optional_complex_type();
    test_mixed_arrays_optionals();
    test_csv_loaders();
    test_binary_arrays_optionals();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
