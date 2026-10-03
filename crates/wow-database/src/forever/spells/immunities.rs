//! 02245dcd SpellMgr.cpp:5373-5412, observed signed World SQL metadata.
use super::{LoadError, SqlResult, text};
use wow_persistence::forever::spells::CreatureImmunityRow;

pub(super) const QUERY: &str = "SELECT ID, SchoolMask, DispelTypeMask, MechanicsMask, Effects, Auras, ImmuneAoE, ImmuneChain FROM creature_immunities";

pub(super) fn decode(row: &SqlResult) -> Result<CreatureImmunityRow, LoadError> {
    if row.field_count() != 8 {
        return Err(LoadError::InvalidRow);
    }
    Ok(CreatureImmunityRow {
        id: row.try_read_typed::<i32>(0).ok_or(LoadError::InvalidRow)?,
        school: row.try_read_typed::<i8>(1).ok_or(LoadError::InvalidRow)?,
        dispel: row.try_read_typed::<i16>(2).ok_or(LoadError::InvalidRow)?,
        mechanics: row.try_read_typed::<i64>(3).ok_or(LoadError::InvalidRow)?,
        effects: text(row, 4)?,
        auras: text(row, 5)?,
        immune_aoe: row.try_read_typed::<i8>(6).ok_or(LoadError::InvalidRow)? != 0,
        immune_chain: row.try_read_typed::<i8>(7).ok_or(LoadError::InvalidRow)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_projection_and_empty_shape_never_synthesize_a_catalog_row() {
        assert_eq!(
            QUERY,
            "SELECT ID, SchoolMask, DispelTypeMask, MechanicsMask, Effects, Auras, ImmuneAoE, ImmuneChain FROM creature_immunities"
        );
        assert!(matches!(
            decode(&SqlResult::empty()),
            Err(LoadError::InvalidRow)
        ));
    }
}
