//! World-facing facade for lifecycle-owned character rename operations.

pub(crate) use wow_world_lifecycle::{
    prepare_rename, PreparedRename, RenameFailure, RenameOutcome, RenamePreparation, RenameRequest,
};

#[cfg(test)]
#[path = "../unit_tests/character_administration/tests.rs"]
mod tests;
