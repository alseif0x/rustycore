//! ObjectMgr.cpp:9005-9031 at 02245dcd; checked read, no invented defaults.
use crate::SqlResult;
use wow_persistence::forever::{LoadError, creation::SkillTierRow};

pub(super) const QUERY: &str = "SELECT ID, Value1, Value2, Value3, Value4, Value5, Value6, Value7, Value8, Value9, Value10, Value11, Value12, Value13, Value14, Value15, Value16 FROM skill_tiers";

pub(super) fn decode(row: &SqlResult) -> Result<SkillTierRow, LoadError> {
    let mut values = [0u32; 16];
    for (index, value) in values.iter_mut().enumerate() {
        *value = super::field(row, index + 1)?;
    }
    Ok(SkillTierRow {
        id: super::field(row, 0)?,
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_skill_tier_query_has_all_sixteen_columns_and_no_filter_or_limit() {
        let columns = std::iter::once("ID".to_owned())
            .chain((1..=16).map(|index| format!("Value{index}")))
            .collect::<Vec<_>>()
            .join(", ");
        assert_eq!(QUERY, format!("SELECT {columns} FROM skill_tiers"));
    }
}
