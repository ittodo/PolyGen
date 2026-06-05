"""Optional Kotlin runtime assertions for generated PolyGen Kotlin files."""

from __future__ import annotations

import argparse
import ctypes
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
private fun assertThat(condition: Boolean, message: String) {
    if (!condition) {
        throw IllegalStateException(message)
    }
}

fun main() {
    val tag = TestCollectionsTag("red", "#f00")
    val arrayRow = TestCollectionsArrayTest(
        id = 1,
        int_list = listOf(1, -2),
        string_list = listOf("alpha", "beta"),
        float_list = listOf(1.5f, 2.5f),
        bool_list = listOf(true, false),
        tags = listOf(tag)
    )
    assertThat(readTestCollectionsArrayTestFromBinary(arrayRow.toBinary()) == arrayRow, "array binary roundtrip")

    val optionalRow = TestCollectionsOptionalTest(
        id = 2,
        required_name = "optional",
        opt_int = 7,
        opt_string = "present",
        opt_float = 3.5,
        opt_bool = true,
        opt_tag = tag
    )
    assertThat(readTestCollectionsOptionalTestFromBinary(optionalRow.toBinary()) == optionalRow, "optional binary roundtrip")

    val mixedRow = TestCollectionsMixedTest(
        id = 3,
        opt_tags = listOf(tag),
        meta = TestCollectionsMixedTestMetadata(created_by = "me", updated_by = null, version = 1),
        history = listOf(TestCollectionsMixedTestMetadata(created_by = null, updated_by = "you", version = 2))
    )
    assertThat(readTestCollectionsMixedTestFromBinary(mixedRow.toBinary()) == mixedRow, "mixed binary roundtrip")

    val jsonFile = java.io.File.createTempFile("polygen-kotlin-collections", ".json")
    jsonFile.writeText("""[
        {"id":1,"int_list":[1,-2],"string_list":["alpha","beta"],"float_list":[1.5,2.5],"bool_list":[true,false],"tags":[{"name":"red","color":"#f00"}]}
    ]""".trimIndent())
    assertThat(loadTestCollectionsArrayTestsFromJson(jsonFile.path).single() == arrayRow, "array JSON loader")

    val csvFile = java.io.File.createTempFile("polygen-kotlin-collections", ".csv")
    csvFile.writeText("id,int_list,string_list,float_list,bool_list,tags\n1,\"1,-2\",\"alpha,beta\",\"1.5,2.5\",\"true,false\",\"[{\"\"name\"\":\"\"red\"\",\"\"color\"\":\"\"#f00\"\"}]\"\n")
    assertThat(loadTestCollectionsArrayTestsFromCsv(csvFile.path).single() == arrayRow, "array CSV loader")
}
''',
    "07_indexes": r'''
private fun assertThat(condition: Boolean, message: String) {
    if (!condition) {
        throw IllegalStateException(message)
    }
}

fun main() {
    val container = SchemaContainer()
    val alice = TestIndexesUser(1, "alice", "alice@example.com", "Alice")
    val bob = TestIndexesUser(2, "bob", "bob@example.com", "Bob")
    val category = TestIndexesCategory(10, "Guides", "PolyGen runtime guide category", 5, TestIndexesCategoryKind.Public)
    val post = TestIndexesPost(100, "PolyGen runtime guide", "body", 1, 10)
    val comment = TestIndexesComment(1000, 100, 2, "nice", null)
    val tag = TestIndexesTag(20, "runtime")
    val postTag = TestIndexesPostTag(100, 20)

    container.users.loadAll(listOf(alice, bob))
    container.categorys.addRow(category)
    container.posts.addRow(post)
    container.comments.addRow(comment)
    container.tags.addRow(tag)
    container.postTags.addRow(postTag)

    assertThat(container.users.count == 2, "user table count")
    assertThat(container.users.getByUsername("alice") == alice, "unique username lookup")
    assertThat(container.categorys.searchByName("guides").single().id == 10, "exact string search")
    assertThat(container.categorys.searchByDescription("runtime guide").single().id == 10, "token search")
    assertThat(container.categorys.searchByRank(5).single().id == 10, "numeric search")
    assertThat(container.categorys.searchByKind(TestIndexesCategoryKind.Public).single().id == 10, "enum search")
    assertThat(container.posts.findByAuthorId(1).single() == post, "group index")
    assertThat(container.posts.searchByTitle("runtime guide").single() == post, "post title search")
    assertThat(container.getPostAuthor(post) == alice, "post author navigation")
    assertThat(container.getPostCategory(post) == category, "post category navigation")
    assertThat(container.getPostTagTag(postTag) == tag, "junction tag navigation")
    assertThat(container.validateAll().isValid(), "container validation")

    val invalid = SchemaContainer()
    invalid.users.addRow(alice)
    invalid.users.addRow(alice.copy(email = "alice2@example.com"))
    assertThat(!invalid.validateAll().isValid(), "duplicate primary key validation")

    val document = BinaryRefDocument.fromContainer(container)
    val reopened = BinaryRefDocument.fromByteArray(document.toByteArray())
    assertThat(reopened.users.getByEmail("alice@example.com")!!.get().username == "alice", "binary ref unique lookup")
    assertThat(reopened.categorys.searchByDescription("runtime guide").single().get().id == 10, "binary ref text search")
    assertThat(reopened.posts.searchByTitle("runtime guide").single().get().id == 100, "binary ref post search")
}
''',
    "08_complex_schema": r'''
private fun assertThat(condition: Boolean, message: String) {
    if (!condition) {
        throw IllegalStateException(message)
    }
}

private fun player(id: Int, name: String, level: Int): GameCharacterPlayer =
    GameCharacterPlayer(
        id = id,
        name = name,
        level = level,
        experience = 0,
        stats = GameCharacterStats(10, 10, 5, 5, 1, 1, 1, 1),
        position = GameCommonVec3(1.0f, 2.0f, 3.0f),
        status = GameCharacterPlayerStatus.Online,
        guild_id = null
    )

fun main() {
    val valid = SchemaContainer()
    valid.players.addRow(player(1, "Valid_Name 1", 1))
    assertThat(valid.validateAll().isValid(), "valid player validation")

    val invalid = SchemaContainer()
    invalid.players.addRow(player(1, "Invalid!", 0))
    invalid.players.addRow(player(1, "ThisNameIsFarLongerThanThirtyTwoCharacters", 101))
    val errors = invalid.validateAll().errors
    assertThat(errors.any { it.constraintType == "Regex" && it.fieldName == "name" }, "regex validation")
    assertThat(errors.any { it.constraintType == "Range" && it.fieldName == "level" }, "range validation")
    assertThat(errors.any { it.constraintType == "MaxLength" && it.fieldName == "name" }, "max length validation")
    assertThat(errors.any { it.constraintType == "Unique" && it.fieldName == "id" }, "primary key validation")
}
''',
    "09_sqlite": r'''
import java.sql.Connection
import java.sql.DriverManager

private fun assertThat(condition: Boolean, message: String) {
    if (!condition) {
        throw IllegalStateException(message)
    }
}

private fun execute(connection: Connection, sql: String) {
    val statement = connection.createStatement()
    try {
        statement.executeUpdate(sql)
    } finally {
        statement.close()
    }
}

fun main() {
    Class.forName("org.sqlite.JDBC")
    val connection = DriverManager.getConnection("jdbc:sqlite::memory:")
    try {
        execute(connection, "CREATE TABLE test_sqlite_User (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT, created_at INTEGER NOT NULL)")
        execute(connection, "CREATE TABLE test_sqlite_Post (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, title TEXT NOT NULL, content TEXT)")
        execute(connection, "CREATE TABLE test_sqlite_Comment (id INTEGER PRIMARY KEY, post_id INTEGER NOT NULL, user_id INTEGER NOT NULL, content TEXT NOT NULL)")
        execute(connection, "CREATE TABLE test_sqlite_audit_LoginEvent (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, ip_address TEXT NOT NULL)")
        execute(connection, "INSERT INTO test_sqlite_User VALUES (1, 'Alice', 'alice@example.com', 123456789)")
        execute(connection, "INSERT INTO test_sqlite_Post VALUES (10, 1, 'Hello', 'Body')")
        execute(connection, "INSERT INTO test_sqlite_Comment VALUES (100, 10, 1, 'Comment')")
        execute(connection, "INSERT INTO test_sqlite_audit_LoginEvent VALUES (1000, 1, '127.0.0.1')")

        val db = SqliteDb(connection)
        db.loadAll()
        assertThat(db.users.count() == 1, "user count")
        assertThat(db.posts.count() == 1, "post count")
        assertThat(db.comments.count() == 1, "comment count")
        assertThat(db.loginEvents.count() == 1, "nested login event count")
        assertThat(db.getUserById(1)!!.email == "alice@example.com", "user lookup")
        assertThat(db.getPostById(10)!!.title == "Hello", "post lookup")
        assertThat(db.getCommentById(100)!!.content == "Comment", "comment lookup")
        assertThat(db.getLoginEventById(1000)!!.ip_address == "127.0.0.1", "nested lookup")
    } finally {
        connection.close()
    }
}
''',
    "10_pack_embed": r'''
private fun assertThat(condition: Boolean, message: String) {
    if (!condition) {
        throw IllegalStateException(message)
    }
}

fun main() {
    val position = TestPackEmbedPosition(100.5f, 200.25f)
    assertThat(unpackTestPackEmbedPosition(position.pack()) == position, "position pack roundtrip")
    assertThat(tryUnpackTestPackEmbedPosition("bad") == null, "position invalid tryUnpack")
    assertThat(readTestPackEmbedPositionFromBinary(position.toBinary()) == position, "position binary roundtrip")

    val color = TestPackEmbedColor(255, 128, 64)
    assertThat(color.pack() == "255,128,64", "color custom separator pack")
    assertThat(unpackTestPackEmbedColor(color.pack()) == color, "color pack roundtrip")
    assertThat(tryUnpackTestPackEmbedColor("-1,2,3") == null, "color unsigned guard")

    val alpha = TestPackEmbedColorAlpha(255, 255, 255, 128)
    assertThat(alpha.pack() == "255|255|255|128", "alpha custom separator pack")
    assertThat(unpackTestPackEmbedColorAlpha(alpha.pack()) == alpha, "alpha pack roundtrip")

    val size = TestPackEmbedSize(800, 600)
    assertThat(unpackTestPackEmbedSize(size.pack()) == size, "size pack roundtrip")
    assertThat(tryUnpackTestPackEmbedSize("-1;2") == null, "size unsigned guard")

    val range = TestPackEmbedRange(-100, 100)
    assertThat(range.pack() == "-100~100", "range custom separator pack")
    assertThat(readTestPackEmbedRangeFromBinary(range.toBinary()) == range, "range binary roundtrip")
}
''',
    "11_relations_indexes": r'''
private fun assertThat(condition: Boolean, message: String) {
    if (!condition) {
        throw IllegalStateException(message)
    }
}

fun main() {
    val container = SchemaContainer()
    val user = ExamplesRelationsUser(1, "author@example.com", "Author")
    val draft = ExamplesRelationsPost(10, 1, ExamplesRelationsPostStatus.Draft, "Draft")
    val published = ExamplesRelationsPost(11, 1, ExamplesRelationsPostStatus.Published, "Published")

    container.users.addRow(user)
    container.posts.loadAll(listOf(draft, published))

    assertThat(container.posts.findByAuthorId(1).map { it.id } == listOf(10, 11), "author group index")
    assertThat(
        container.posts.findByAuthorIdStatus(listOf(1, ExamplesRelationsPostStatus.Published)).single() == published,
        "composite group index"
    )
    assertThat(container.getPostAuthor(published) == user, "forward navigation")
    assertThat(container.findUserPosts(user).map { it.id } == listOf(10, 11), "reverse navigation")
    assertThat(container.validateAll().isValid(), "container validation")

    val invalid = SchemaContainer()
    invalid.users.addRow(user)
    invalid.users.addRow(user.copy(id = 2))
    assertThat(!invalid.validateAll().isValid(), "duplicate unique validation")

    val document = BinaryRefDocument.fromContainer(container)
    val reopened = BinaryRefDocument.fromByteArray(document.toByteArray())
    assertThat(reopened.users.getByEmail("author@example.com")!!.get().id == 1, "binary ref unique lookup")
    assertThat(
        reopened.posts.findByAuthorIdStatus(listOf(1, ExamplesRelationsPostStatus.Published)).single().get().id == 11,
        "binary ref composite lookup"
    )
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


def build_classpath() -> str:
    return os.environ.get("POLYGEN_KOTLIN_CLASSPATH") or os.environ.get("KOTLIN_CLASSPATH") or ""


def split_env_args(value: str) -> list[str]:
    if not value.strip():
        return []
    if os.name != "nt":
        return shlex.split(value)

    argc = ctypes.c_int()
    ctypes.windll.shell32.CommandLineToArgvW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_int)]
    ctypes.windll.shell32.CommandLineToArgvW.restype = ctypes.POINTER(ctypes.c_wchar_p)
    ctypes.windll.kernel32.LocalFree.argtypes = [ctypes.c_void_p]
    argv = ctypes.windll.shell32.CommandLineToArgvW(value, ctypes.byref(argc))
    if not argv:
        raise OSError("CommandLineToArgvW failed")
    try:
        return [argv[index] for index in range(argc.value)]
    finally:
        ctypes.windll.kernel32.LocalFree(argv)


def should_use_argfile(args: list[str]) -> bool:
    return os.name == "nt" and any(os.pathsep in arg for arg in args)


def run_kotlinc(kotlinc: str, args: list[str], temp_dir: Path) -> int:
    if not should_use_argfile(args):
        cmd = [kotlinc, *args]
        print(" ".join(cmd))
        return subprocess.run(cmd).returncode

    argfile = temp_dir / "kotlinc.args"
    argfile.write_text("\n".join(args) + "\n", encoding="utf-8")
    cmd = [kotlinc, f"@{argfile}"]
    print(" ".join(cmd))
    return subprocess.run(cmd).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case_name", help="Integration case name, e.g. 07_indexes")
    parser.add_argument("files", nargs="+", help="Generated .kt files or glob patterns")
    args = parser.parse_args()

    test_source = RUNTIME_TESTS.get(args.case_name)
    if test_source is None:
        print(f"no Kotlin runtime assertions for {args.case_name}")
        return 0

    kotlinc = os.environ.get("KOTLINC") or shutil.which("kotlinc")
    if not kotlinc:
        print("kotlinc was not found. Install Kotlin or set KOTLINC.", file=sys.stderr)
        return 1

    java = os.environ.get("JAVA") or shutil.which("java")
    if not java:
        print("java was not found. Install a JRE/JDK or set JAVA.", file=sys.stderr)
        return 1

    files = expand_inputs(args.files)
    if not files:
        print("no Kotlin files found to run", file=sys.stderr)
        return 1

    classpath = build_classpath()
    extra_args = split_env_args(os.environ.get("POLYGEN_KOTLIN_COMPILER_ARGS", ""))

    with TemporaryDirectory() as temp:
        temp_dir = Path(temp)
        harness = temp_dir / "PolygenKotlinRuntimeTest.kt"
        output_jar = temp_dir / "polygen-kotlin-runtime-test.jar"
        harness.write_text(test_source.strip() + "\n", encoding="utf-8")

        compile_args = [*extra_args]
        if classpath:
            compile_args.extend(["-classpath", classpath])
        compile_args.extend(files)
        compile_args.append(str(harness))
        compile_args.extend(["-include-runtime", "-d", str(output_jar)])

        compile_exit = run_kotlinc(kotlinc, compile_args, temp_dir)
        if compile_exit != 0:
            return compile_exit

        runtime_classpath = str(output_jar)
        if classpath:
            runtime_classpath = os.pathsep.join([runtime_classpath, classpath])
        run_cmd = [java, "-cp", runtime_classpath, "PolygenKotlinRuntimeTestKt"]
        print(" ".join(run_cmd))
        return subprocess.run(run_cmd).returncode


if __name__ == "__main__":
    raise SystemExit(main())
