//! Constructors for generated character statements.
//!
//! Separated from the character.rs root under #652. Behaviour is preserved.

use super::*;

impl CharStatements {
    /// Build a generated C++ CharacterDatabase statement from its C++
    /// identifier and exact SQL.
    pub const fn cpp(name: &'static str, sql: &'static str) -> Self {
        Self::GENERATED_CPP { name, sql }
    }
}
