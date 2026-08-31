//! MariaDB adapter for the represented `LFGDungeons.db2` Hotfix overlay.
//!
//! This preserves Rust's existing single `VerifiedBuild > 0` batch. C++'s
//! generic DB2 loader can also stage custom rows; reconciling that pre-existing
//! parity gap is behavior work, not part of this ownership-only cut.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    LfgDungeonsHotfixLoadOutcomeLikeCpp, LfgDungeonsHotfixPersistencePortLikeCpp,
    LfgDungeonsHotfixRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, SqlResult};

fn read_integer_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i128> {
    result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))
}

fn integer_field_like_cpp<T>(value: i128, field: &'static str) -> Result<T>
where
    T: TryFrom<i128>,
{
    T::try_from(value).map_err(|_| anyhow::anyhow!("{field} SQL value {value} is out of range"))
}

fn string_field_like_cpp(result: &SqlResult, column: usize, field: &'static str) -> Result<String> {
    result
        .try_read::<String>(column)
        .with_context(|| format!("missing or non-string {field} SQL column {column}"))
}

fn float_field_like_cpp(result: &SqlResult, column: usize, field: &'static str) -> Result<f32> {
    result
        .try_read::<f32>(column)
        .with_context(|| format!("missing or non-float {field} SQL column {column}"))
}

fn lfg_dungeons_row_like_cpp(result: &SqlResult) -> Result<LfgDungeonsHotfixRowLikeCpp> {
    Ok(LfgDungeonsHotfixRowLikeCpp {
        id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 0, "LFGDungeons.ID")?,
            "LFGDungeons.ID",
        )?,
        name: string_field_like_cpp(result, 1, "LFGDungeons.Name")?,
        description: string_field_like_cpp(result, 2, "LFGDungeons.Description")?,
        min_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 3, "LFGDungeons.MinLevel")?,
            "LFGDungeons.MinLevel",
        )?,
        max_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 4, "LFGDungeons.MaxLevel")?,
            "LFGDungeons.MaxLevel",
        )?,
        type_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 5, "LFGDungeons.TypeID")?,
            "LFGDungeons.TypeID",
        )?,
        subtype: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 6, "LFGDungeons.Subtype")?,
            "LFGDungeons.Subtype",
        )?,
        faction: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 7, "LFGDungeons.Faction")?,
            "LFGDungeons.Faction",
        )?,
        icon_texture_file_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 8, "LFGDungeons.IconTextureFileID")?,
            "LFGDungeons.IconTextureFileID",
        )?,
        rewards_bg_texture_file_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 9, "LFGDungeons.RewardsBgTextureFileID")?,
            "LFGDungeons.RewardsBgTextureFileID",
        )?,
        popup_bg_texture_file_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 10, "LFGDungeons.PopupBgTextureFileID")?,
            "LFGDungeons.PopupBgTextureFileID",
        )?,
        expansion_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 11, "LFGDungeons.ExpansionLevel")?,
            "LFGDungeons.ExpansionLevel",
        )?,
        map_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 12, "LFGDungeons.MapID")?,
            "LFGDungeons.MapID",
        )?,
        difficulty_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 13, "LFGDungeons.DifficultyID")?,
            "LFGDungeons.DifficultyID",
        )?,
        min_gear: float_field_like_cpp(result, 14, "LFGDungeons.MinGear")?,
        group_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 15, "LFGDungeons.GroupID")?,
            "LFGDungeons.GroupID",
        )?,
        order_index: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 16, "LFGDungeons.OrderIndex")?,
            "LFGDungeons.OrderIndex",
        )?,
        required_player_condition_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 17, "LFGDungeons.RequiredPlayerConditionId")?,
            "LFGDungeons.RequiredPlayerConditionId",
        )?,
        target_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 18, "LFGDungeons.TargetLevel")?,
            "LFGDungeons.TargetLevel",
        )?,
        target_level_min: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 19, "LFGDungeons.TargetLevelMin")?,
            "LFGDungeons.TargetLevelMin",
        )?,
        target_level_max: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 20, "LFGDungeons.TargetLevelMax")?,
            "LFGDungeons.TargetLevelMax",
        )?,
        random_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 21, "LFGDungeons.RandomID")?,
            "LFGDungeons.RandomID",
        )?,
        scenario_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 22, "LFGDungeons.ScenarioID")?,
            "LFGDungeons.ScenarioID",
        )?,
        final_encounter_id: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 23, "LFGDungeons.FinalEncounterID")?,
            "LFGDungeons.FinalEncounterID",
        )?,
        count_tank: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 24, "LFGDungeons.CountTank")?,
            "LFGDungeons.CountTank",
        )?,
        count_healer: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 25, "LFGDungeons.CountHealer")?,
            "LFGDungeons.CountHealer",
        )?,
        count_damage: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 26, "LFGDungeons.CountDamage")?,
            "LFGDungeons.CountDamage",
        )?,
        min_count_tank: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 27, "LFGDungeons.MinCountTank")?,
            "LFGDungeons.MinCountTank",
        )?,
        min_count_healer: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 28, "LFGDungeons.MinCountHealer")?,
            "LFGDungeons.MinCountHealer",
        )?,
        min_count_damage: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 29, "LFGDungeons.MinCountDamage")?,
            "LFGDungeons.MinCountDamage",
        )?,
        bonus_reputation_amount: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 30, "LFGDungeons.BonusReputationAmount")?,
            "LFGDungeons.BonusReputationAmount",
        )?,
        mentor_item_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 31, "LFGDungeons.MentorItemLevel")?,
            "LFGDungeons.MentorItemLevel",
        )?,
        mentor_char_level: integer_field_like_cpp(
            read_integer_checked_like_cpp(result, 32, "LFGDungeons.MentorCharLevel")?,
            "LFGDungeons.MentorCharLevel",
        )?,
        flags: [
            integer_field_like_cpp(
                read_integer_checked_like_cpp(result, 33, "LFGDungeons.Flags1")?,
                "LFGDungeons.Flags1",
            )?,
            integer_field_like_cpp(
                read_integer_checked_like_cpp(result, 34, "LFGDungeons.Flags2")?,
                "LFGDungeons.Flags2",
            )?,
        ],
    })
}

pub struct MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl LfgDungeonsHotfixPersistencePortLikeCpp for MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp {
    fn load_lfg_dungeons_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, LfgDungeonsHotfixLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let result = async {
                let mut rows = self
                    .hotfix_db
                    .query(&self.hotfix_db.prepare(HotfixStatements::SEL_LFG_DUNGEONS))
                    .await?;
                let mut decoded = Vec::with_capacity(rows.count());
                if rows.is_empty() {
                    return Ok::<_, anyhow::Error>(decoded);
                }
                loop {
                    decoded.push(lfg_dungeons_row_like_cpp(&rows)?);
                    if !rows.next_row() {
                        break;
                    }
                }
                Ok(decoded)
            }
            .await;

            match result {
                Ok(rows) => LfgDungeonsHotfixLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => LfgDungeonsHotfixLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn statement_order_matches_cpp_lfg_dungeons_load_info() {
        assert_eq!(
            HotfixStatements::SEL_LFG_DUNGEONS.sql(),
            concat!(
                "SELECT ID, Name, Description, MinLevel, MaxLevel, TypeID, Subtype, Faction, IconTextureFileID, ",
                "RewardsBgTextureFileID, PopupBgTextureFileID, ExpansionLevel, MapID, DifficultyID, MinGear, GroupID, OrderIndex, RequiredPlayerConditionId, ",
                "TargetLevel, TargetLevelMin, TargetLevelMax, RandomID, ScenarioID, FinalEncounterID, CountTank, CountHealer, CountDamage, MinCountTank, ",
                "MinCountHealer, MinCountDamage, BonusReputationAmount, MentorItemLevel, MentorCharLevel, Flags1, Flags2 FROM lfg_dungeons WHERE VerifiedBuild > 0"
            )
        );
    }

    #[test]
    fn checked_field_widths_preserve_cpp_signedness_and_reject_defaults() {
        assert_eq!(integer_field_like_cpp::<u32>(42, "ID").unwrap(), 42);
        assert_eq!(integer_field_like_cpp::<i8>(-1, "Faction").unwrap(), -1);
        assert_eq!(integer_field_like_cpp::<i16>(-2, "MapID").unwrap(), -2);
        assert_eq!(integer_field_like_cpp::<i32>(-3, "Flags").unwrap(), -3);
        assert!(integer_field_like_cpp::<u8>(256, "MinLevel").is_err());
        assert!(integer_field_like_cpp::<u16>(-1, "MaxLevel").is_err());
        assert!(integer_field_like_cpp::<i8>(128, "Faction").is_err());
        assert!(integer_field_like_cpp::<i16>(32_768, "MapID").is_err());
        assert!(integer_field_like_cpp::<i32>(i128::from(i64::MAX), "Flags").is_err());
    }
}
