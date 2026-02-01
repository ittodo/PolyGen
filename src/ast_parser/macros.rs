/// Macro to require the next element from an iterator, or return a MissingElement error.
///
/// # Usage
/// ```ignore
/// let element = require_next!(iter, Rule::some_rule, "element description", line, col)?;
/// ```
macro_rules! require_next {
    ($iter:expr, $rule:expr, $element:expr, $line:expr, $col:expr) => {
        $iter
            .next()
            .ok_or(crate::error::AstBuildError::MissingElement {
                rule: $rule,
                element: $element.to_string(),
                line: $line,
                col: $col,
            })
    };
}

/// Macro to return an UnexpectedRule error for match default cases.
///
/// # Usage
/// ```ignore
/// match pair.as_rule() {
///     Rule::expected1 => { ... },
///     Rule::expected2 => { ... },
///     found => unexpected_rule!(found, "expected1 or expected2", line, col),
/// }
/// ```
macro_rules! unexpected_rule {
    ($found:expr, $expected:expr, $line:expr, $col:expr) => {
        return Err(crate::error::AstBuildError::UnexpectedRule {
            expected: $expected.to_string(),
            found: $found,
            line: $line,
            col: $col,
        })
    };
}
