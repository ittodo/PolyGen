use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum AstBuildError {
    #[error("Invalid value '{value}' for {element} at {line}:{col}")]
    InvalidValue {
        element: String,
        value: String,
        line: usize,
        col: usize,
    },
    #[error("Unexpected rule '{found:?}' at {line}:{col}, expected {expected}")]
    UnexpectedRule {
        expected: String,
        found: crate::Rule,
        line: usize,
        col: usize,
    },
    #[error("Missing element '{element}' for rule '{rule:?}' at {line}:{col}")]
    MissingElement {
        rule: crate::Rule,
        element: String,
        line: usize,
        col: usize,
    },
}

#[derive(Error, Debug, PartialEq)]
pub enum ValidationError {
    #[error("Type not found: '{0}'")]
    TypeNotFound(String),

    #[error("Duplicate definition for type: '{0}'")]
    DuplicateDefinition(String),
}

#[derive(Error, Debug)]
pub enum CodeGenError {
    #[error("Template file not found: {path}")]
    TemplateNotFound { path: String },

    #[error("Failed to read template file '{path}': {source}")]
    TemplateReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Rhai script execution failed: {message}")]
    RhaiExecutionError { message: String },

    #[error("Failed to write output file '{path}': {source}")]
    OutputWriteError {
        path: String,
        #[source]
        source: std::io::Error,
    },
}
