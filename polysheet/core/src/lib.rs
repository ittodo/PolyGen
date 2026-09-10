//! Core document, validation, diff, merge, and Git services for PolySheet.

pub mod diff;
pub mod formula;
pub mod git;
pub mod merge;
pub mod model;
pub mod project;
pub mod transaction;
pub mod validation;

pub use diff::{diff_snapshots, ChangeKind, DiffEntry, DiffReport, ProjectSnapshot};
pub use merge::{merge_snapshots, MergeConflict, MergeReport};
pub use model::*;
pub use project::{OpenProjectOptions, PolySheetProject};
pub use transaction::{
    recover_transaction, recover_transaction_with_allowed_targets, write_transaction, FileUpdate,
};
pub use validation::{Diagnostic, DiagnosticSeverity};
