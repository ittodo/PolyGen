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
}

/// A parsed condition expression for `%if`.
#[derive(Debug, Clone, PartialEq)]
pub enum CondExpr {
    /// Property access that evaluates to bool (e.g. `field.is_primary_key`).
    Property(Vec<String>),
    /// Property access with filter (e.g. `struct.fqn | is_embedded`).
    PropertyWithFilter(Vec<String>, Filter),
    /// Negation: `!expr`.
    Not(Box<CondExpr>),
    /// Logical AND: `expr && expr`.
    And(Box<CondExpr>, Box<CondExpr>),
    /// Logical OR: `expr || expr`.
    Or(Box<CondExpr>, Box<CondExpr>),
}

/// A collection expression for `%for var in collection`.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionExpr {
    /// Property access chain to the collection.
    pub path: Vec<String>,
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

/// Parse a condition expression for `%if`.
///
/// Supports:
/// - Simple property: `field.is_primary_key`
/// - Property with filter: `struct.fqn | is_embedded`
/// - Negation: `!field.is_optional`
/// - AND: `field.is_primary_key && field.is_unique`
/// - OR: `field.is_primary_key || field.is_unique`
pub fn parse_condition(input: &str) -> Result<CondExpr, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty condition".to_string());
    }

    // Handle OR (lowest precedence)
    if let Some(pos) = find_logical_op(input, "||") {
        let left = parse_condition(&input[..pos])?;
        let right = parse_condition(&input[pos + 2..])?;
        return Ok(CondExpr::Or(Box::new(left), Box::new(right)));
    }

    // Handle AND
    if let Some(pos) = find_logical_op(input, "&&") {
        let left = parse_condition(&input[..pos])?;
        let right = parse_condition(&input[pos + 2..])?;
        return Ok(CondExpr::And(Box::new(left), Box::new(right)));
    }

    // Handle NOT
    if let Some(rest) = input.strip_prefix('!') {
        let inner = parse_condition(rest)?;
        return Ok(CondExpr::Not(Box::new(inner)));
    }

    // Handle property with filter: `struct.fqn | is_embedded`
    if input.contains('|') {
        let parts: Vec<&str> = input.splitn(2, '|').collect();
        let path: Vec<String> = parts[0].trim().split('.').map(|s| s.trim().to_string()).collect();
        let filter = parse_filter(parts[1].trim())?;
        return Ok(CondExpr::PropertyWithFilter(path, filter));
    }

    // Simple property access
    let path: Vec<String> = input.split('.').map(|s| s.trim().to_string()).collect();
    if path.iter().any(|p| p.is_empty()) {
        return Err(format!("Invalid condition path: '{}'", input));
    }

    Ok(CondExpr::Property(path))
}

/// Parse a collection expression for `%for`.
///
/// Input: `"namespace.items"` → `CollectionExpr { path: ["namespace", "items"] }`
pub fn parse_collection(input: &str) -> Result<CollectionExpr, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty collection expression".to_string());
    }

    let path: Vec<String> = input.split('.').map(|s| s.trim().to_string()).collect();
    if path.iter().any(|p| p.is_empty()) {
        return Err(format!("Invalid collection path: '{}'", input));
    }

    Ok(CollectionExpr { path })
}

/// Parse a single filter name.
fn parse_filter(name: &str) -> Result<Filter, String> {
    // Handle join("sep") syntax
    if name.starts_with("join(") && name.ends_with(')') {
        let sep = name[5..name.len() - 1].trim().trim_matches('"').trim_matches('\'');
        return Ok(Filter::Join(sep.to_string()));
    }

    // Handle suffix("str") syntax
    if name.starts_with("suffix(") && name.ends_with(')') {
        let s = name[7..name.len() - 1].trim().trim_matches('"').trim_matches('\'');
        return Ok(Filter::Suffix(s.to_string()));
    }

    // Handle prefix("str") syntax
    if name.starts_with("prefix(") && name.ends_with(')') {
        let s = name[7..name.len() - 1].trim().trim_matches('"').trim_matches('\'');
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
        _ => Err(format!("Unknown filter: '{}'", name)),
    }
}

/// Find a logical operator (`&&` or `||`) at the top level (not inside parentheses).
fn find_logical_op(input: &str, op: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    let op_bytes = op.as_bytes();
    if bytes.len() < op_bytes.len() {
        return None;
    }
    bytes
        .windows(op_bytes.len())
        .position(|w| w == op_bytes)
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
    fn test_parse_condition_simple() {
        let cond = parse_condition("field.is_primary_key").unwrap();
        assert_eq!(cond, CondExpr::Property(vec!["field".into(), "is_primary_key".into()]));
    }

    #[test]
    fn test_parse_condition_not() {
        let cond = parse_condition("!field.is_optional").unwrap();
        match cond {
            CondExpr::Not(inner) => {
                assert_eq!(*inner, CondExpr::Property(vec!["field".into(), "is_optional".into()]));
            }
            _ => panic!("Expected Not"),
        }
    }

    #[test]
    fn test_parse_condition_and() {
        let cond = parse_condition("field.is_primary_key && field.is_unique").unwrap();
        match cond {
            CondExpr::And(left, right) => {
                assert_eq!(*left, CondExpr::Property(vec!["field".into(), "is_primary_key".into()]));
                assert_eq!(*right, CondExpr::Property(vec!["field".into(), "is_unique".into()]));
            }
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_parse_condition_with_filter() {
        let cond = parse_condition("struct.fqn | is_embedded").unwrap();
        match cond {
            CondExpr::PropertyWithFilter(path, filter) => {
                assert_eq!(path, vec!["struct".to_string(), "fqn".to_string()]);
                assert_eq!(filter, Filter::IsEmbedded);
            }
            _ => panic!("Expected PropertyWithFilter"),
        }
    }

    #[test]
    fn test_parse_collection() {
        let coll = parse_collection("namespace.items").unwrap();
        assert_eq!(coll.path, vec!["namespace", "items"]);
    }

    #[test]
    fn test_parse_expr_error() {
        assert!(parse_expr("").is_err());
        assert!(parse_expr(".name").is_err());
    }
}
