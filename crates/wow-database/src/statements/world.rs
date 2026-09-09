//! World database prepared statement definitions.
//!
//! These correspond to TrinityCore's `world` database prepared statements.

use super::StatementDef;

mod identities;
mod statement_def;
#[allow(unused_imports)]
pub use identities::*;
#[allow(unused_imports)]
pub use statement_def::*;

#[cfg(test)]
#[path = "world/tests.rs"]
mod tests;
