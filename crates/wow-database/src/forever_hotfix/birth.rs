//! Target numeric projections; locale strings/full wire serialization excluded.
//! HotfixDatabase.cpp:342-349,1532-1555 and DB2DatabaseLoader.cpp:27-174,02245dcd.
use super::{ForeverHotfixRepository, LoadError, PreparedStatement};
use crate::SqlResult;
use wow_persistence::forever::birth::{
    BirthOverlays, BirthRows, LoadoutItemRow, LoadoutRow, SkillAbilityRow, SkillLineRow,
    SkillRaceClassRow,
};

const QUERIES: [&str; 5] = [
    "SELECT ID,CategoryID,SpellIconFileID,CanLink,ParentSkillLineID,ParentTierIndex,Flags,SpellBookSpellID,ExpansionNameSharedStringID,HordeExpansionNameSharedStringID FROM skill_line WHERE (VerifiedBuild>0)=?",
    "SELECT ID,SkillID,ClassMask,Flags,Availability,MinLevel,SkillTierID,RaceMask1,RaceMask2 FROM skill_race_class_info WHERE (VerifiedBuild>0)=?",
    "SELECT ID,SkillLine,Spell,MinSkillLineRank,ClassMask,SupercedesSpell,AcquireMethod,TrivialSkillLineRankHigh,TrivialSkillLineRankLow,Flags,NumSkillUps,UniqueBit,TradeSkillCategoryID,SkillupSkillLineID,Field_5_5_4_67090_0141,Field_5_5_4_67090_0142,RaceMask1,RaceMask2 FROM skill_line_ability WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ChrClassID,Purpose,ItemContext,Field_1_60_1_69876_003,RaceMask1,RaceMask2 FROM character_loadout WHERE (VerifiedBuild>0)=?",
    "SELECT ID,CharacterLoadoutID,ItemID FROM character_loadout_item WHERE (VerifiedBuild>0)=?",
];

fn read<T: for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>>(
    row: &SqlResult,
    index: usize,
) -> Result<T, LoadError> {
    row.try_read(index).ok_or(LoadError::InvalidRow)
}

impl ForeverHotfixRepository {
    /// All ten reads complete before a batch is returned. Query/decode failure
    /// is fatal, not an empty-table fallback; no SQL snapshot is claimed.
    pub async fn load_birth_overlays(&self) -> Result<BirthOverlays, LoadError> {
        Ok(BirthOverlays {
            official: self.birth_batch(false).await?,
            custom: self.birth_batch(true).await?,
        })
    }

    async fn birth_batch(&self, custom: bool) -> Result<BirthRows, LoadError> {
        let mut rows = BirthRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut statement = PreparedStatement::new(*sql);
            statement.set_bool(0, !custom);
            let mut result = self
                .0
                .query(&statement)
                .await
                .map_err(|_| LoadError::Database)?;
            if result.is_empty() {
                continue;
            }
            loop {
                // SQL key is (ID,VerifiedBuild), not ID alone. Retain query
                // order and duplicate overlay IDs for the source overwrite.
                let id = read(&result, 0)?;
                match table {
                    0 => rows.skill_lines.push(SkillLineRow {
                        id,
                        category: read(&result, 1)?,
                        spell_icon_file: read(&result, 2)?,
                        can_link: read(&result, 3)?,
                        parent_skill: read(&result, 4)?,
                        parent_tier_index: read(&result, 5)?,
                        flags: read(&result, 6)?,
                        spell_book_spell: read(&result, 7)?,
                        expansion_name_shared_string: read(&result, 8)?,
                        horde_expansion_name_shared_string: read(&result, 9)?,
                    }),
                    1 => rows.race_class.push(SkillRaceClassRow {
                        id,
                        skill: read(&result, 1)?,
                        class_mask: read(&result, 2)?,
                        flags: read(&result, 3)?,
                        availability: read(&result, 4)?,
                        min_level: read(&result, 5)?,
                        tier: read(&result, 6)?,
                        race_mask: [read(&result, 7)?, read(&result, 8)?],
                    }),
                    2 => rows.abilities.push(SkillAbilityRow {
                        id,
                        skill_line: read(&result, 1)?,
                        spell: read(&result, 2)?,
                        min_skill_rank: read(&result, 3)?,
                        class_mask: read(&result, 4)?,
                        supercedes_spell: read(&result, 5)?,
                        acquire_method: read(&result, 6)?,
                        trivial_rank_high: read(&result, 7)?,
                        trivial_rank_low: read(&result, 8)?,
                        flags: read(&result, 9)?,
                        num_skill_ups: read(&result, 10)?,
                        unique_bit: read(&result, 11)?,
                        trade_skill_category: read(&result, 12)?,
                        skillup_skill_line: read(&result, 13)?,
                        field_5_5_4_67090_014: [read(&result, 14)?, read(&result, 15)?],
                        race_mask: [read(&result, 16)?, read(&result, 17)?],
                    }),
                    3 => rows.loadouts.push(LoadoutRow {
                        id,
                        class: read(&result, 1)?,
                        purpose: read(&result, 2)?,
                        item_context: read(&result, 3)?,
                        field_1_60_1_69876_003: read(&result, 4)?,
                        race_mask: [read(&result, 5)?, read(&result, 6)?],
                    }),
                    4 => rows.loadout_items.push(LoadoutItemRow {
                        id,
                        loadout: read(&result, 1)?,
                        item: read(&result, 2)?,
                    }),
                    _ => unreachable!(),
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_numeric_queries_keep_target_column_order_and_official_custom_predicate() {
        let expected = [
            (
                "skill_line",
                "ID,CategoryID,SpellIconFileID,CanLink,ParentSkillLineID,ParentTierIndex,Flags,SpellBookSpellID,ExpansionNameSharedStringID,HordeExpansionNameSharedStringID",
            ),
            (
                "skill_race_class_info",
                "ID,SkillID,ClassMask,Flags,Availability,MinLevel,SkillTierID,RaceMask1,RaceMask2",
            ),
            (
                "skill_line_ability",
                "ID,SkillLine,Spell,MinSkillLineRank,ClassMask,SupercedesSpell,AcquireMethod,TrivialSkillLineRankHigh,TrivialSkillLineRankLow,Flags,NumSkillUps,UniqueBit,TradeSkillCategoryID,SkillupSkillLineID,Field_5_5_4_67090_0141,Field_5_5_4_67090_0142,RaceMask1,RaceMask2",
            ),
            (
                "character_loadout",
                "ID,ChrClassID,Purpose,ItemContext,Field_1_60_1_69876_003,RaceMask1,RaceMask2",
            ),
            ("character_loadout_item", "ID,CharacterLoadoutID,ItemID"),
        ];
        for (query, (table, columns)) in QUERIES.iter().zip(expected) {
            assert_eq!(
                *query,
                format!("SELECT {columns} FROM {table} WHERE (VerifiedBuild>0)=?")
            );
            assert!(!query.contains("ORDER BY") && !query.contains("locale"));
        }
    }
}
