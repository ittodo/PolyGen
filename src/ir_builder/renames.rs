use crate::ast_model;
use crate::ir_model::{RenameInfo, RenameKind};

/// Converts an AST RenameRule to an IR RenameInfo.
pub(super) fn convert_rename(rename: &ast_model::RenameRule) -> RenameInfo {
    // Determine the kind based on path length:
    // - Length 2: Table rename (e.g., game.Player -> User)
    // - Length 3+: Field rename (e.g., game.User.hp -> health)
    let kind = if rename.from_path.len() <= 2 {
        RenameKind::Table
    } else {
        RenameKind::Field
    };

    RenameInfo {
        kind,
        from_path: rename.from_path.clone(),
        to_name: rename.to_name.clone(),
    }
}
