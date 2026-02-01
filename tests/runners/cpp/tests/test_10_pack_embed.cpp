// Test Case 10: Pack Embed
// Tests @pack annotation on embed types for serializing fields to a single string

#include <iostream>
#include <cassert>
#include <cmath>
#include <string>

#include "schema.hpp"

using namespace test::pack_embed;

void test_position_pack() {
    std::cout << "  Testing Position pack/unpack (sep: ;)..." << std::endl;

    Position pos;
    pos.x = 100.5f;
    pos.y = 200.3f;

    std::string packed = pos.pack();
    assert(packed == "100.5;200.3");

    auto unpacked = Position::unpack(packed);
    assert(std::abs(unpacked.x - 100.5f) < 0.01f);
    assert(std::abs(unpacked.y - 200.3f) < 0.01f);

    std::cout << "    PASS" << std::endl;
}

void test_position3d_pack() {
    std::cout << "  Testing Position3D pack/unpack (sep: ;)..." << std::endl;

    Position3D pos;
    pos.x = 10.0f;
    pos.y = 20.0f;
    pos.z = 30.0f;

    std::string packed = pos.pack();
    assert(packed == "10;20;30");

    auto unpacked = Position3D::unpack(packed);
    assert(std::abs(unpacked.x - 10.0f) < 0.01f);
    assert(std::abs(unpacked.y - 20.0f) < 0.01f);
    assert(std::abs(unpacked.z - 30.0f) < 0.01f);

    std::cout << "    PASS" << std::endl;
}

void test_color_pack() {
    std::cout << "  Testing Color pack/unpack (sep: ,)..." << std::endl;

    Color c;
    c.r = 255;
    c.g = 128;
    c.b = 64;

    std::string packed = c.pack();
    assert(packed == "255,128,64");

    auto unpacked = Color::unpack(packed);
    assert(unpacked.r == 255);
    assert(unpacked.g == 128);
    assert(unpacked.b == 64);

    std::cout << "    PASS" << std::endl;
}

void test_color_alpha_pack() {
    std::cout << "  Testing ColorAlpha pack/unpack (sep: |)..." << std::endl;

    ColorAlpha ca;
    ca.r = 255;
    ca.g = 255;
    ca.b = 255;
    ca.a = 128;

    std::string packed = ca.pack();
    assert(packed == "255|255|255|128");

    auto unpacked = ColorAlpha::unpack(packed);
    assert(unpacked.r == 255);
    assert(unpacked.g == 255);
    assert(unpacked.b == 255);
    assert(unpacked.a == 128);

    std::cout << "    PASS" << std::endl;
}

void test_size_pack() {
    std::cout << "  Testing Size pack/unpack (sep: ;)..." << std::endl;

    Size s;
    s.width = 800;
    s.height = 600;

    std::string packed = s.pack();
    assert(packed == "800;600");

    auto unpacked = Size::unpack(packed);
    assert(unpacked.width == 800);
    assert(unpacked.height == 600);

    std::cout << "    PASS" << std::endl;
}

void test_range_pack() {
    std::cout << "  Testing Range pack/unpack (sep: ~)..." << std::endl;

    Range r;
    r.min = -100;
    r.max = 100;

    std::string packed = r.pack();
    assert(packed == "-100~100");

    auto unpacked = Range::unpack(packed);
    assert(unpacked.min == -100);
    assert(unpacked.max == 100);

    std::cout << "    PASS" << std::endl;
}

void test_try_unpack() {
    std::cout << "  Testing try_unpack failure cases..." << std::endl;

    Position out;
    bool ok = Position::try_unpack("invalid", out);
    assert(!ok);

    ok = Position::try_unpack("1.0;2.0", out);
    assert(ok);
    assert(std::abs(out.x - 1.0f) < 0.01f);
    assert(std::abs(out.y - 2.0f) < 0.01f);

    std::cout << "    PASS" << std::endl;
}

void test_stats_no_pack() {
    std::cout << "  Testing Stats (no @pack) has no pack methods..." << std::endl;

    // Stats should still work as a normal embed
    Stats stats;
    stats.hp = 100;
    stats.mp = 50;
    stats.attack = 25;
    stats.defense = 10;

    assert(stats.hp == 100);
    assert(stats.mp == 50);

    std::cout << "    PASS" << std::endl;
}

int main() {
    std::cout << "=== Test Case 10: Pack Embed ===" << std::endl;

    test_position_pack();
    test_position3d_pack();
    test_color_pack();
    test_color_alpha_pack();
    test_size_pack();
    test_range_pack();
    test_try_unpack();
    test_stats_no_pack();

    std::cout << "=== All tests passed! ===" << std::endl;
    return 0;
}
