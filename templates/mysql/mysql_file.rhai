// MySQL Generator Entry Point

print("-- Polygen MySQL Schema Generation --");

let final_sql = "";

for file in schema.files {
    let file_sql = include("templates/mysql/mysql_namespace.rhai");
    final_sql += eval(`"""${file_sql}"""`);
}

// TODO: Figure out how to write to a .sql file.
// For now, it will be in the .cs file.
print(final_sql);
