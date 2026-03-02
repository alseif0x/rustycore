//! Hotfix database prepared statement definitions.
//!
//! These correspond to the `hotfixes` database and the C# `HotfixStatements` enum.
//! SQL strings will be populated incrementally as data loading is implemented.

use super::StatementDef;

/// Prepared statements for the hotfix database.
///
/// This enum currently contains a placeholder. The full 419+ statements from
/// the C# `HotfixDatabase` will be added as data loading is ported (Phase 4+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum HotfixStatements {
    /// Placeholder — real statements added as data loading is ported.
    _PLACEHOLDER,
}

impl StatementDef for HotfixStatements {
    fn sql(self) -> &'static str {
        match self {
            Self::_PLACEHOLDER => "",
        }
    }
}
