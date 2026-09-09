//! Character database prepared statement definitions.
//!
//! These correspond to the `characters` database and the C++
//! `CharacterDatabaseStatements` enum
//! (`src/server/database/Database/Implementation/CharacterDatabase.h:23`).
//!
//! The header used to cite the C# `CharStatements` enum. Per AGENTS.md the C#
//! server is a historical reference and not an authority for this port, and a
//! module that names it as one invites the next reader to check the wrong
//! source -- which matters more here than elsewhere, because these identities
//! are what a persistence golden freezes.

use super::StatementDef;

mod builders;
mod identities;
mod statement_def;
#[allow(unused_imports)]
pub use builders::*;
#[allow(unused_imports)]
pub use identities::*;
#[allow(unused_imports)]
pub use statement_def::*;

#[cfg(test)]
#[path = "character_tests/mod.rs"]
mod tests;
