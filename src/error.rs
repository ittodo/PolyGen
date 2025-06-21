use crate::Rule;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Pest parsing error")]
    Pest(#[from] Box<pest::error::Error<Rule>>),

    #[error("L{line}:{col} - Unexpected rule: expected {expected}, but found {found:?}")]
    UnexpectedRule {
        expected: String,
        found: Rule,
        line: usize,
        col: usize,
    },

    #[error("L{line}:{col} - Missing element in {rule:?}: {element}")]
    MissingElement {
        rule: Rule,
        element: String,
        line: usize,
        col: usize,
    },

    #[error("L{line}:{col} - Invalid value for {element}: '{value}'")]
    InvalidValue {
        element: String,
        value: String,
        line: usize,
        col: usize,
    },
}
