"""Optional Swift runtime assertions for generated PolyGen Swift files."""

from __future__ import annotations

import argparse
import glob
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory


RUNTIME_TESTS = {
    "06_arrays_and_optionals": r'''
import Foundation

func assertThat(_ condition: Bool, _ message: String) {
    if !condition {
        fatalError(message)
    }
}

@main
struct PolygenSwiftRuntimeTest {
    static func main() throws {
        let tag = TestCollectionsTag(name: "red", color: "#f00")
        let arrayRow = TestCollectionsArrayTest(
            id: 1,
            int_list: [1, -2],
            string_list: ["alpha", "beta"],
            float_list: [1.5, 2.5],
            bool_list: [true, false],
            tags: [tag]
        )
        assertThat(try TestCollectionsArrayTest.fromBinary(arrayRow.toBinary()) == arrayRow, "array binary roundtrip")

        let optionalRow = TestCollectionsOptionalTest(
            id: 2,
            required_name: "optional",
            opt_int: 7,
            opt_string: "present",
            opt_float: 3.5,
            opt_bool: true,
            opt_tag: tag
        )
        assertThat(try TestCollectionsOptionalTest.fromBinary(optionalRow.toBinary()) == optionalRow, "optional binary roundtrip")

        let mixedRow = TestCollectionsMixedTest(
            id: 3,
            opt_tags: [tag],
            meta: TestCollectionsMixedTestMetadata(created_by: "me", updated_by: nil, version: 1),
            history: [TestCollectionsMixedTestMetadata(created_by: nil, updated_by: "you", version: 2)]
        )
        assertThat(try TestCollectionsMixedTest.fromBinary(mixedRow.toBinary()) == mixedRow, "mixed binary roundtrip")

        let root = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let jsonPath = root.appendingPathComponent("array_tests.json").path
        let csvPath = root.appendingPathComponent("array_tests.csv").path

        let jsonText = """
        [
          {"id":1,"int_list":[1,-2],"string_list":["alpha","beta"],"float_list":[1.5,2.5],"bool_list":[true,false],"tags":[{"name":"red","color":"#f00"}]}
        ]
        """
        try jsonText.write(toFile: jsonPath, atomically: true, encoding: .utf8)
        assertThat(try loadTestCollectionsArrayTestsFromJson(jsonPath).single == arrayRow, "array JSON loader")

        let csvText = "id,int_list,string_list,float_list,bool_list,tags\n1,\"1,-2\",\"alpha,beta\",\"1.5,2.5\",\"true,false\",\"[{\"\"name\"\":\"\"red\"\",\"\"color\"\":\"\"#f00\"\"}]\"\n"
        try csvText.write(toFile: csvPath, atomically: true, encoding: .utf8)
        assertThat(try loadTestCollectionsArrayTestsFromCsv(csvPath).single == arrayRow, "array CSV loader")
    }
}

extension Array {
    var single: Element? { count == 1 ? self[0] : nil }
}
''',
    "07_indexes": r'''
import Foundation

func assertThat(_ condition: Bool, _ message: String) {
    if !condition {
        fatalError(message)
    }
}

@main
struct PolygenSwiftRuntimeTest {
    static func main() throws {
        let container = SchemaContainer()
        let alice = TestIndexesUser(id: 1, username: "alice", email: "alice@example.com", display_name: "Alice")
        let bob = TestIndexesUser(id: 2, username: "bob", email: "bob@example.com", display_name: "Bob")
        let category = TestIndexesCategory(
            id: 10,
            name: " Guides ",
            description: "PolyGen runtime guide category",
            rank: 5,
            kind: TestIndexesCategoryKind(rawValue: 1)!
        )
        let post = TestIndexesPost(id: 100, title: "PolyGen runtime guide", content: "body", author_id: 1, category_id: 10)
        let comment = TestIndexesComment(id: 1000, post_id: 100, author_id: 2, content: "nice", parent_id: nil)
        let tag = TestIndexesTag(id: 20, name: "runtime")
        let postTag = TestIndexesPostTag(post_id: 100, tag_id: 20)

        container.users.loadAll([alice, bob])
        container.categorys.addRow(category)
        container.posts.addRow(post)
        container.comments.addRow(comment)
        container.tags.addRow(tag)
        container.postTags.addRow(postTag)

        assertThat(container.users.count == 2, "user table count")
        assertThat(container.users.getByUsername("alice") == alice, "unique username lookup")
        assertThat(container.categorys.searchByName("guides").single?.id == 10, "exact string search")
        assertThat(container.categorys.searchByDescription("runtime guide").single?.id == 10, "token search")
        assertThat(container.categorys.searchByRank(5).single?.id == 10, "numeric search")
        assertThat(container.categorys.searchByKind(TestIndexesCategoryKind(rawValue: 1)!).single?.id == 10, "enum search")
        assertThat(container.posts.findByAuthorId(1).single == post, "group index")
        assertThat(container.posts.searchByTitle("runtime guide").single == post, "post title search")
        assertThat(container.getPostAuthor(post) == alice, "post author navigation")
        assertThat(container.getPostCategory(post) == category, "post category navigation")
        assertThat(container.getPostTagTag(postTag) == tag, "junction tag navigation")
        assertThat(container.validateAll().isValid, "container validation")

        let invalid = SchemaContainer()
        invalid.users.addRow(alice)
        invalid.users.addRow(TestIndexesUser(id: 1, username: "alice2", email: "alice2@example.com", display_name: "Alice 2"))
        assertThat(!invalid.validateAll().isValid, "duplicate primary key validation")

        let document = try BinaryRefDocument.fromContainer(container)
        let reopened = try BinaryRefDocument.fromData(document.toData())
        assertThat(try reopened.users.getByEmail("alice@example.com")?.get().username == "alice", "binary ref unique lookup")
        assertThat(try reopened.categorys.searchByDescription("runtime guide").single?.get().id == 10, "binary ref text search")
        assertThat(try reopened.posts.searchByTitle("runtime guide").single?.get().id == 100, "binary ref post search")
    }
}

extension Array {
    var single: Element? { count == 1 ? self[0] : nil }
}
''',
    "08_complex_schema": r'''
import Foundation

func assertThat(_ condition: Bool, _ message: String) {
    if !condition {
        fatalError(message)
    }
}

func player(_ id: UInt32, _ name: String, _ level: UInt16) -> GameCharacterPlayer {
    GameCharacterPlayer(
        id: id,
        name: name,
        level: level,
        experience: 0,
        stats: GameCharacterStats(hp: 10, max_hp: 10, mp: 5, max_mp: 5, strength: 1, agility: 1, intelligence: 1, vitality: 1),
        position: GameCommonVec3(x: 1.0, y: 2.0, z: 3.0),
        status: .online,
        guild_id: nil
    )
}

@main
struct PolygenSwiftRuntimeTest {
    static func main() throws {
        let valid = SchemaContainer()
        valid.players.addRow(player(1, "Valid_Name 1", 1))
        assertThat(valid.validateAll().isValid, "valid player validation")

        let invalid = SchemaContainer()
        invalid.players.addRow(player(1, "Invalid!", 0))
        invalid.players.addRow(player(1, "ThisNameIsFarLongerThanThirtyTwoCharacters", 101))

        let errors = invalid.validateAll().errors
        assertThat(errors.contains { $0.constraintType == "Regex" && $0.fieldName == "name" }, "regex validation")
        assertThat(errors.contains { $0.constraintType == "Range" && $0.fieldName == "level" }, "range validation")
        assertThat(errors.contains { $0.constraintType == "MaxLength" && $0.fieldName == "name" }, "max length validation")
        assertThat(errors.contains { $0.constraintType == "Unique" && $0.fieldName == "id" }, "primary key validation")

        let guilds = SchemaContainer()
        guilds.guilds.addRow(GameSocialGuild(
            id: 1,
            name: "GuildNameThatIsLongerThanTwentyFourChars",
            tag: "TOOLONG",
            leader_id: 1,
            level: 1,
            emblem_color: GameCommonColor(r: 1, g: 2, b: 3, a: 255),
            created_at: 123
        ))
        let guildErrors = guilds.validateAll().errors
        assertThat(guildErrors.contains { $0.constraintType == "MaxLength" && $0.fieldName == "tag" }, "guild tag max length validation")
    }
}
''',
    "09_sqlite": r'''
import Foundation

func assertThat(_ condition: Bool, _ message: String) {
    if !condition {
        fatalError(message)
    }
}

struct FakeRow: PolygenSQLiteRow {
    let values: [String: Any?]

    func value(_ column: String) -> Any? {
        values[column] ?? nil
    }
}

final class FakeConnection: PolygenSQLiteConnection {
    let tables: [String: [FakeRow]]

    init(tables: [String: [FakeRow]]) {
        self.tables = tables
    }

    func query(_ sql: String, parameters: [Any]) throws -> [any PolygenSQLiteRow] {
        let tableName: String
        if sql.contains("test_sqlite_audit_LoginEvent") {
            tableName = "test_sqlite_audit_LoginEvent"
        } else if sql.contains("test_sqlite_Comment") {
            tableName = "test_sqlite_Comment"
        } else if sql.contains("test_sqlite_Post") {
            tableName = "test_sqlite_Post"
        } else {
            tableName = "test_sqlite_User"
        }
        let rows = tables[tableName] ?? []
        guard sql.contains("WHERE id = ?"), let key = parameters.first as? UInt32 else {
            return rows
        }
        return rows.filter { ($0.values["id"] as? UInt32) == key }
    }

    func close() throws {}
}

@main
struct PolygenSwiftRuntimeTest {
    static func main() throws {
        let connection = FakeConnection(tables: [
            "test_sqlite_User": [FakeRow(values: ["id": UInt32(1), "name": "Alice", "email": "alice@example.com", "created_at": Int64(1234)])],
            "test_sqlite_Post": [FakeRow(values: ["id": UInt32(10), "user_id": UInt32(1), "title": "Hello", "content": Optional<String>.none as Any])],
            "test_sqlite_Comment": [FakeRow(values: ["id": UInt32(20), "post_id": UInt32(10), "user_id": UInt32(1), "content": "Nice"])],
            "test_sqlite_audit_LoginEvent": [FakeRow(values: ["id": UInt32(30), "user_id": UInt32(1), "ip_address": "127.0.0.1"])],
        ])
        let db = SqliteDb(connection: connection)
        try db.loadAll()
        assertThat(db.users.count() == 1, "user count")
        assertThat(db.posts.count() == 1, "post count")
        assertThat(db.comments.count() == 1, "comment count")
        assertThat(db.loginEvents.count() == 1, "nested login event count")
        assertThat(try db.getUserById(UInt32(1))?.email == "alice@example.com", "user lookup")
        assertThat(try db.getPostById(UInt32(10))?.title == "Hello", "post lookup")
        assertThat(try db.getCommentById(UInt32(20))?.content == "Nice", "comment lookup")
        assertThat(try db.getLoginEventById(UInt32(30))?.ip_address == "127.0.0.1", "nested lookup")
    }
}
''',
    "10_pack_embed": r'''
import Foundation

func assertThat(_ condition: Bool, _ message: String) {
    if !condition {
        fatalError(message)
    }
}

@main
struct PolygenSwiftRuntimeTest {
    static func main() throws {
        let position = TestPackEmbedPosition(x: 100.5, y: 200.25)
        assertThat(try TestPackEmbedPosition.unpack(position.pack()) == position, "position pack roundtrip")
        assertThat(TestPackEmbedPosition.tryUnpack("bad") == nil, "position invalid tryUnpack")
        assertThat(try TestPackEmbedPosition.fromBinary(position.toBinary()) == position, "position binary roundtrip")

        let color = TestPackEmbedColor(r: 255, g: 128, b: 64)
        assertThat(color.pack() == "255,128,64", "color custom separator pack")
        assertThat(try TestPackEmbedColor.unpack(color.pack()) == color, "color pack roundtrip")
        assertThat(TestPackEmbedColor.tryUnpack("-1,2,3") == nil, "color unsigned guard")

        let alpha = TestPackEmbedColorAlpha(r: 255, g: 255, b: 255, a: 128)
        assertThat(alpha.pack() == "255|255|255|128", "alpha custom separator pack")
        assertThat(try TestPackEmbedColorAlpha.unpack(alpha.pack()) == alpha, "alpha pack roundtrip")

        let size = TestPackEmbedSize(width: 800, height: 600)
        assertThat(try TestPackEmbedSize.unpack(size.pack()) == size, "size pack roundtrip")
        assertThat(TestPackEmbedSize.tryUnpack("-1;2") == nil, "size unsigned guard")

        let range = TestPackEmbedRange(min: -100, max: 100)
        assertThat(range.pack() == "-100~100", "range custom separator pack")
        assertThat(try TestPackEmbedRange.fromBinary(range.toBinary()) == range, "range binary roundtrip")

        let object = TestPackEmbedGameObject(
            id: 1,
            name: "box",
            position: position,
            position3d: TestPackEmbedPosition3D(x: 1, y: 2, z: 3),
            color: color,
            size: size,
            stats: TestPackEmbedStats(hp: 10, mp: 5, attack: 3, defense: 2)
        )
        assertThat(try TestPackEmbedGameObject.fromBinary(object.toBinary()) == object, "embedded object binary roundtrip")
    }
}
''',
    "11_relations_indexes": r'''
import Foundation

func assertThat(_ condition: Bool, _ message: String) {
    if !condition {
        fatalError(message)
    }
}

@main
struct PolygenSwiftRuntimeTest {
    static func main() throws {
        let container = SchemaContainer()
        let user = ExamplesRelationsUser(id: 1, email: "author@example.com", display_name: "Author")
        let draft = ExamplesRelationsPost(id: 10, author_id: 1, status: .draft, title: "Draft")
        let published = ExamplesRelationsPost(id: 11, author_id: 1, status: .published, title: "Published")

        container.users.addRow(user)
        container.posts.loadAll([draft, published])

        assertThat(container.posts.findByAuthorId(1).map(\.id) == [10, 11], "author group index")
        assertThat(container.posts.findByAuthorIdStatus([AnyHashable(UInt32(1)), AnyHashable(ExamplesRelationsPostStatus.published)]).single == published, "composite group index")
        assertThat(container.getPostAuthor(published) == user, "forward navigation")
        assertThat(container.findUserPosts(user).map(\.id) == [10, 11], "reverse navigation")
        assertThat(container.validateAll().isValid, "container validation")

        let invalid = SchemaContainer()
        invalid.users.addRow(user)
        invalid.users.addRow(ExamplesRelationsUser(id: 2, email: "author@example.com", display_name: "Duplicate"))
        assertThat(!invalid.validateAll().isValid, "duplicate unique validation")

        let document = try BinaryRefDocument.fromContainer(container)
        let reopened = try BinaryRefDocument.fromData(document.toData())
        assertThat(try reopened.users.getByEmail("author@example.com")?.get().id == 1, "binary ref unique lookup")
        assertThat(try reopened.posts.findByAuthorIdStatus([AnyHashable(UInt32(1)), AnyHashable(ExamplesRelationsPostStatus.published)]).single?.get().id == 11, "binary ref composite lookup")
    }
}

extension Array {
    var single: Element? { count == 1 ? self[0] : nil }
}
''',
}


def expand_inputs(patterns: list[str]) -> list[str]:
    files: list[str] = []
    for pattern in patterns:
        matches = glob.glob(pattern)
        if matches:
            files.extend(matches)
        else:
            files.append(pattern)
    return sorted({str(Path(path)) for path in files if Path(path).is_file()})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case_name", help="Integration case name, e.g. 07_indexes")
    parser.add_argument("files", nargs="+", help="Generated .swift files or glob patterns")
    args = parser.parse_args()

    test_source = RUNTIME_TESTS.get(args.case_name)
    if test_source is None:
        print(f"no Swift runtime assertions for {args.case_name}")
        return 0

    swiftc = os.environ.get("SWIFTC") or shutil.which("swiftc")
    if not swiftc:
        print("swiftc was not found. Install Swift or set SWIFTC.", file=sys.stderr)
        return 1

    include_swiftdata = os.environ.get("POLYGEN_SWIFT_INCLUDE_SWIFTDATA") == "1"
    files = expand_inputs(args.files)
    if not include_swiftdata:
        files = [path for path in files if not Path(path).name.endswith("_swiftdata.swift")]
    if not files:
        print("no Swift files found to run", file=sys.stderr)
        return 1

    extra_args = shlex.split(os.environ.get("POLYGEN_SWIFT_COMPILER_ARGS", ""))

    with TemporaryDirectory() as temp:
        temp_dir = Path(temp)
        harness = temp_dir / "PolygenSwiftRuntimeTest.swift"
        executable = temp_dir / ("polygen-swift-runtime-test.exe" if os.name == "nt" else "polygen-swift-runtime-test")
        harness.write_text(test_source.strip() + "\n", encoding="utf-8")

        compile_cmd = [swiftc, *extra_args, *files, str(harness), "-o", str(executable)]
        print(" ".join(compile_cmd))
        compile_result = subprocess.run(compile_cmd)
        if compile_result.returncode != 0:
            return compile_result.returncode

        print(str(executable))
        return subprocess.run([str(executable)]).returncode


if __name__ == "__main__":
    raise SystemExit(main())
