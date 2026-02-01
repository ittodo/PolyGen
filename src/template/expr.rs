//! Expression parser for `{{expr | filter}}` interpolation.
//!
//! Expressions are property access chains with optional pipe-separated filters:
//! - `{{struct.name}}` — property access
//! - `{{field.name | pascal_case}}` — with filter
//! - `{{field.field_type | lang_type}}` — type mapping filter

/// A parsed expression with optional filters.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Property access chain (e.g. `["struct", "name"]`).
    pub path: Vec<String>,
    /// Filters to apply in order (e.g. `[Filter::PascalCase]`).
    pub filters: Vec<Filter>,
}

/// A filter applied to an expression value.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// Convert to PascalCase.
    PascalCase,
    /// Convert to snake_case.
    SnakeCase,
    /// Convert to camelCase.
    CamelCase,
    /// Convert to UPPER_CASE.
    Upper,
    /// Convert to lower case.
    Lower,
    /// Map poly type to target language type using TOML type_map.
    LangType,
    /// Format a default value for the target language.
    Format,
    /// Wrap in quotes.
    Quote,
    /// Count items in a collection.
    Count,
    /// Join a collection with a separator.
    Join(String),
    /// Binary read expression from TOML binary_read mapping.
    BinaryRead,
    /// Binary read for Option<T> from TOML binary_read.option.
    BinaryReadOption,
    /// Binary read for List<T> from TOML binary_read.list.
    BinaryReadList,
    /// Binary read for struct type from TOML binary_read.struct.
    BinaryReadStruct,
    /// CSV read expression from TOML csv_read mapping.
    CsvRead,
    /// Check if struct FQN is embedded anywhere in schema.
    IsEmbedded,
    /// Append a suffix string.
    Suffix(String),
    /// Prepend a prefix string.
    Prefix(String),
    /// Remove all dots from a string (e.g., "game.character" → "gamecharacter").
    RemoveDots,
}

/// A collection expression for `%for var in collection`.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionExpr {
    /// Property access chain to the collection.
    pub path: Vec<String>,
    /// Optional `| where condition` filter (raw Rhai expression string).
    pub where_filter: Option<String>,
}

/// Parse a `{{...}}` interpolation expression.
///
/// Input: the content between `{{` and `}}`, trimmed.
/// Example: `"field.name | pascal_case"` → `Expr { path: ["field", "name"], filters: [PascalCase] }`
pub fn parse_expr(input: &str) -> Result<Expr, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty expression".to_string());
    }

    // Split on `|` for filters
    let parts: Vec<&str> = input.split('|').collect();
    let path_str = parts[0].trim();

    let path: Vec<String> = path_str.split('.').map(|s| s.trim().to_string()).collect();
    if path.iter().any(|p| p.is_empty()) {
        return Err(format!("Invalid property path: '{}'", path_str));
    }

    let mut filters = Vec::new();
    for filter_str in &parts[1..] {
        let filter = parse_filter(filter_str.trim())?;
        filters.push(filter);
    }

    Ok(Expr { path, filters })
}

/// Parse a collection expression for `%for`.
///
/// Supports optional `| where condition` filter:
/// - `"namespace.items"` → path only
/// - `"struct.fields | where field.is_primary_key"` → path + where filter
pub fn parse_collection(input: &str) -> Result<CollectionExpr, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty collection expression".to_string());
    }

    // Check for `| where` filter
    let (path_str, where_filter) = if let Some(pos) = input.find("| where ") {
        let path_part = input[..pos].trim();
        let cond_part = input[pos + 8..].trim(); // Skip "| where "
        (path_part, Some(cond_part.to_string()))
    } else {
        (input, None)
    };

    let path: Vec<String> = path_str.split('.').map(|s| s.trim().to_string()).collect();
    if path.iter().any(|p| p.is_empty()) {
        return Err(format!("Invalid collection path: '{}'", path_str));
    }

    Ok(CollectionExpr { path, where_filter })
}

/// Parse a single filter name.
fn parse_filter(name: &str) -> Result<Filter, String> {
    // Handle join("sep") syntax
    if name.starts_with("join(") && name.ends_with(')') {
        let sep = name[5..name.len() - 1]
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        return Ok(Filter::Join(sep.to_string()));
    }

    // Handle suffix("str") syntax
    if name.starts_with("suffix(") && name.ends_with(')') {
        let s = name[7..name.len() - 1]
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        return Ok(Filter::Suffix(s.to_string()));
    }

    // Handle prefix("str") syntax
    if name.starts_with("prefix(") && name.ends_with(')') {
        let s = name[7..name.len() - 1]
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        return Ok(Filter::Prefix(s.to_string()));
    }

    match name {
        "pascal_case" => Ok(Filter::PascalCase),
        "snake_case" => Ok(Filter::SnakeCase),
        "camel_case" => Ok(Filter::CamelCase),
        "upper" => Ok(Filter::Upper),
        "lower" => Ok(Filter::Lower),
        "lang_type" => Ok(Filter::LangType),
        "format" => Ok(Filter::Format),
        "quote" => Ok(Filter::Quote),
        "count" => Ok(Filter::Count),
        "binary_read" => Ok(Filter::BinaryRead),
        "binary_read_option" => Ok(Filter::BinaryReadOption),
        "binary_read_list" => Ok(Filter::BinaryReadList),
        "binary_read_struct" => Ok(Filter::BinaryReadStruct),
        "csv_read" => Ok(Filter::CsvRead),
        "is_embedded" => Ok(Filter::IsEmbedded),
        "remove_dots" => Ok(Filter::RemoveDots),
        _ => Err(format!("Unknown filter: '{}'", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_expr() {
        let expr = parse_expr("struct.name").unwrap();
        assert_eq!(expr.path, vec!["struct", "name"]);
        assert!(expr.filters.is_empty());
    }

    #[test]
    fn test_parse_expr_with_filter() {
        let expr = parse_expr("field.name | pascal_case").unwrap();
        assert_eq!(expr.path, vec!["field", "name"]);
        assert_eq!(expr.filters, vec![Filter::PascalCase]);
    }

    #[test]
    fn test_parse_expr_with_multiple_filters() {
        let expr = parse_expr("field.type | lang_type | upper").unwrap();
        assert_eq!(expr.path, vec!["field", "type"]);
        assert_eq!(expr.filters, vec![Filter::LangType, Filter::Upper]);
    }

    #[test]
    fn test_parse_expr_join_filter() {
        let expr = parse_expr("items | join(\", \")").unwrap();
        assert_eq!(expr.filters, vec![Filter::Join(", ".to_string())]);
    }

    #[test]
    fn test_parse_collection() {
        let coll = parse_collection("namespace.items").unwrap();
        assert_eq!(coll.path, vec!["namespace", "items"]);
        assert!(coll.where_filter.is_none());
    }

    #[test]
    fn test_parse_collection_with_where() {
        let coll = parse_collection("struct.fields | where field.is_primary_key").unwrap();
        assert_eq!(coll.path, vec!["struct", "fields"]);
        assert_eq!(
            coll.where_filter,
            Some("field.is_primary_key".to_string())
        );
    }

    #[test]
    fn test_parse_collection_where_with_not() {
        let coll = parse_collection("struct.fields | where !field.is_optional").unwrap();
        assert_eq!(coll.path, vec!["struct", "fields"]);
        assert_eq!(
            coll.where_filter,
            Some("!field.is_optional".to_string())
        );
    }

    #[test]
    fn test_parse_expr_error() {
        assert!(parse_expr("").is_err());
        assert!(parse_expr(".name").is_err());
    }
}
