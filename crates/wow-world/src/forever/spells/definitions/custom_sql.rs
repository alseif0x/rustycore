//! SQL-only prefix of SpellMgr::LoadSpellInfoCustomAttributes:2995-3039.
//! 02245dcd. Derived custom attributes/positivity and subsequent passes remain
//! separate required phases; this prefix never admits executable SpellInfo.
#[cfg(test)]
mod tests;
use super::{SpellDefinitionError, SpellDefinitionSeeds};
use wow_persistence::forever::spells::SpellCustomAttributeRow;

// SpellInfo.h:148 / SharedDefines.h:1348 of the target, not legacy enums.
const SHARE_DAMAGE: u32 = 0x0000_0008;
const SCHOOL_DAMAGE: u32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SqlCustomAttributeCounts {
    pub input_rows: usize,
    /// Source count: every row with any existing difficulty, even if all
    /// difficulties reject SHARE_DAMAGE. Not a count of changed definitions.
    pub counted_rows: usize,
    pub missing_spell_rows: usize,
    pub definition_assignments: usize,
    pub share_damage_rejected_definitions: usize,
}

impl SpellDefinitionSeeds {
    /// Consume observed SQL order after the source skill map. Duplicate/zero
    /// rows are retained, and every existing difficulty is considered without
    /// manufacture or fallback. No SQL handle/row mirror remains afterwards.
    pub fn with_sql_custom_attributes(
        mut self,
        rows: Vec<SpellCustomAttributeRow>,
    ) -> Result<Self, SpellDefinitionError> {
        if self.skill_line_abilities.is_none() || self.source_order.is_none() {
            return Err(SpellDefinitionError::SqlCustomAttributesRequireSkillLineAbilities);
        }
        if self.sql_custom_attributes.is_some() {
            return Err(SpellDefinitionError::SqlCustomAttributesAlreadyApplied);
        }
        let order = self.source_order.as_ref().expect("admitted source order");
        let mut counts = SqlCustomAttributeCounts {
            input_rows: rows.len(),
            ..Default::default()
        };
        for row in rows {
            let Some(keys) = order.by_spell.get(&row.spell) else {
                counts.missing_spell_rows += 1;
                continue;
            };
            for key in keys {
                let definition = self
                    .definitions
                    .get_mut(key)
                    .expect("admitted immutable key index");
                if row.attributes & SHARE_DAMAGE != 0 && !definition.has_effect(SCHOOL_DAMAGE) {
                    // Source skips the ENTIRE word for this difficulty, not
                    // only SHARE_DAMAGE and not other difficulties of the ID.
                    counts.share_damage_rejected_definitions += 1;
                    continue;
                }
                definition.custom_attributes |= row.attributes;
                counts.definition_assignments += 1;
            }
            counts.counted_rows += 1;
        }
        self.sql_custom_attributes = Some(counts);
        Ok(self)
    }

    /// None before the SQL prefix, Some(zero) after a valid empty batch.
    /// This does not mean complete custom attributes are initialized.
    pub fn sql_custom_attribute_counts(&self) -> Option<SqlCustomAttributeCounts> {
        self.sql_custom_attributes
    }
}
