//! Immutable calculation inputs, not an admitted CalcValue/custom phase.
use super::{SpellDefinitionError, SpellDefinitionSeeds};
use std::sync::Arc;
use wow_data::forever_game_tables::SpellValueGameTables;
#[cfg(test)]
mod tests;

impl SpellDefinitionSeeds {
    /// Complete parsed three-table input batch, one shared immutable owner.
    /// No corrections, gameplay writes, initialized values or ready marker.
    pub fn with_value_game_tables(
        mut self,
        tables: Arc<SpellValueGameTables>,
    ) -> Result<Self, SpellDefinitionError> {
        if self.value_game_tables.is_some() {
            return Err(SpellDefinitionError::SpellValueTablesAlreadyLoaded);
        }
        self.value_game_tables = Some(tables);
        Ok(self)
    }
    pub fn value_game_tables(&self) -> Option<&SpellValueGameTables> {
        self.value_game_tables.as_deref()
    }
}
