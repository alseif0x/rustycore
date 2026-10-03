//! 02245dcd SpellMgr.cpp:905 / :1009. Source columns observed read-only.
use super::{LoadError, SqlResult};
use wow_persistence::forever::spells::{SpellLearnRow, SpellRequiredRow};
pub(super) const REQUIRED: &str = "SELECT spell_id, req_spell from spell_required";
pub(super) const LEARN: &str = "SELECT entry, SpellID, Active FROM spell_learn_spell";
pub(super) fn required(row: &SqlResult) -> Result<SpellRequiredRow, LoadError> {
    if row.field_count() != 2 {
        return Err(LoadError::InvalidRow);
    }
    // Actual SQL metadata is signed INT; source GetUInt32 round-trips the
    // signed bit pattern (FieldValueConverters.h:58-70), including negative IDs.
    Ok(SpellRequiredRow {
        spell: row.try_read_typed::<i32>(0).ok_or(LoadError::InvalidRow)? as u32,
        required: row.try_read_typed::<i32>(1).ok_or(LoadError::InvalidRow)? as u32,
    })
}
pub(super) fn learn(row: &SqlResult) -> Result<SpellLearnRow, LoadError> {
    if row.field_count() != 3 {
        return Err(LoadError::InvalidRow);
    }
    Ok(SpellLearnRow {
        source: row.try_read_typed::<u32>(0).ok_or(LoadError::InvalidRow)?,
        learned: row.try_read_typed::<u32>(1).ok_or(LoadError::InvalidRow)?,
        active: row.try_read_typed::<u8>(2).ok_or(LoadError::InvalidRow)? != 0,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_queries_and_incomplete_rows_do_not_synthesize_relationships() {
        assert_eq!(REQUIRED, "SELECT spell_id, req_spell from spell_required");
        assert_eq!(
            LEARN,
            "SELECT entry, SpellID, Active FROM spell_learn_spell"
        );
        assert_eq!(required(&SqlResult::empty()), Err(LoadError::InvalidRow));
        assert_eq!(learn(&SqlResult::empty()), Err(LoadError::InvalidRow));
    }
}
