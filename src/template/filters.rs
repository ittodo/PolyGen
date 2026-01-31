//! Built-in filter implementations for template expressions.
//!
//! Filters transform string values through pipe syntax: `{{field.name | pascal_case}}`.
//! The [`apply_filter`] function dispatches to the appropriate implementation.

use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};

use crate::template::expr::Filter;

/// Apply a simple string filter (non-context-dependent filters).
///
/// Context-dependent filters like `lang_type`, `binary_read`, etc. are handled
/// by the renderer which has access to the TOML config and IR context.
///
/// Returns `None` if the filter requires context (handled externally).
pub fn apply_string_filter(value: &str, filter: &Filter) -> Option<String> {
    match filter {
        Filter::PascalCase => Some(value.to_pascal_case()),
        Filter::SnakeCase => Some(value.to_snake_case()),
        Filter::CamelCase => Some(value.to_lower_camel_case()),
        Filter::Upper => Some(value.to_uppercase()),
        Filter::Lower => Some(value.to_lowercase()),
        Filter::Quote => Some(format!("\"{}\"", value)),
        Filter::Suffix(s) => Some(format!("{}{}", value, s)),
        Filter::Prefix(s) => Some(format!("{}{}", s, value)),
        // These require context — handled by the renderer
        Filter::LangType
        | Filter::Format
        | Filter::Count
        | Filter::Join(_)
        | Filter::BinaryRead
        | Filter::BinaryReadOption
        | Filter::BinaryReadList
        | Filter::BinaryReadStruct
        | Filter::CsvRead
        | Filter::IsEmbedded => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case() {
        assert_eq!(
            apply_string_filter("hello_world", &Filter::PascalCase),
            Some("HelloWorld".to_string())
        );
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(
            apply_string_filter("HelloWorld", &Filter::SnakeCase),
            Some("hello_world".to_string())
        );
    }

    #[test]
    fn test_camel_case() {
        assert_eq!(
            apply_string_filter("hello_world", &Filter::CamelCase),
            Some("helloWorld".to_string())
        );
    }

    #[test]
    fn test_upper() {
        assert_eq!(
            apply_string_filter("hello", &Filter::Upper),
            Some("HELLO".to_string())
        );
    }

    #[test]
    fn test_lower() {
        assert_eq!(
            apply_string_filter("HELLO", &Filter::Lower),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_quote() {
        assert_eq!(
            apply_string_filter("value", &Filter::Quote),
            Some("\"value\"".to_string())
        );
    }

    #[test]
    fn test_context_dependent_returns_none() {
        assert_eq!(apply_string_filter("u32", &Filter::LangType), None);
        assert_eq!(apply_string_filter("val", &Filter::Format), None);
        assert_eq!(apply_string_filter("u32", &Filter::BinaryRead), None);
    }
}
