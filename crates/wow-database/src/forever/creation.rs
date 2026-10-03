//! Exact target World startup projection, independent of 3.4.3 statements.
//! 02245dcd ObjectMgr.cpp::LoadPlayerInfo:3855-4458. Read-only, no Player save.

use super::field;
use crate::{SqlResult, WorldDatabase};
use std::sync::Arc;
use wow_persistence::PersistenceFutureLikeCpp;
use wow_persistence::forever::{
    LoadError,
    creation::{
        CastSpell, ClassLevelStats, CreationWorldRepository, CreationWorldRows, CustomSpell,
        ItemOverride, RaceStats, StartAction, StartDefinition, XpOverride,
    },
};
mod item_addons;
mod skill_tiers;
mod templates;

// Source order of the SQL families (DB2 loadouts/skills are separate inputs).
const QUERIES: [&str; 8] = [
    "SELECT race, class, map, position_x, position_y, position_z, orientation, npe_map, npe_position_x, npe_position_y, npe_position_z, npe_orientation, npe_transport_guid, intro_movie_id, intro_scene_id, npe_intro_scene_id FROM playercreateinfo",
    "SELECT race, class, itemid, amount FROM playercreateinfo_item",
    "SELECT racemask, classmask, Spell FROM playercreateinfo_spell_custom",
    "SELECT raceMask, classMask, spell, createMode FROM playercreateinfo_cast_spell",
    "SELECT race, class, button, action, type FROM playercreateinfo_action",
    "SELECT race, str, agi, sta, inte, spi FROM player_racestats",
    "SELECT class, level, str, agi, sta, inte, spi FROM player_classlevelstats",
    "SELECT Level, Experience FROM player_xp_for_level",
];

pub struct ForeverCreationWorldRepository(Arc<WorldDatabase>);

impl ForeverCreationWorldRepository {
    pub fn new(world: Arc<WorldDatabase>) -> Self {
        Self(world)
    }
}

async fn rows<T>(
    world: &WorldDatabase,
    sql: &str,
    decode: fn(&SqlResult) -> Result<T, LoadError>,
) -> Result<Vec<T>, LoadError> {
    let mut result = world
        .direct_query(sql)
        .await
        .map_err(|_| LoadError::Database)?;
    let mut rows = Vec::new();
    if !result.is_empty() {
        loop {
            rows.push(decode(&result)?);
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

fn definition(row: &SqlResult) -> Result<StartDefinition, LoadError> {
    Ok(StartDefinition {
        race: field(row, 0)?,
        class: field(row, 1)?,
        map: field(row, 2)?,
        position: [
            field(row, 3)?,
            field(row, 4)?,
            field(row, 5)?,
            field(row, 6)?,
        ],
        npe_map: field(row, 7)?,
        npe_position: [
            field(row, 8)?,
            field(row, 9)?,
            field(row, 10)?,
            field(row, 11)?,
        ],
        npe_transport: field(row, 12)?,
        intro_movie: field(row, 13)?,
        intro_scene: field(row, 14)?,
        npe_intro_scene: field(row, 15)?,
    })
}

fn item(row: &SqlResult) -> Result<ItemOverride, LoadError> {
    Ok(ItemOverride {
        race: field(row, 0)?,
        class: field(row, 1)?,
        item: field(row, 2)?,
        amount: field(row, 3)?,
    })
}

fn spell(row: &SqlResult) -> Result<CustomSpell, LoadError> {
    Ok(CustomSpell {
        race_mask: field(row, 0)?,
        class_mask: field(row, 1)?,
        spell: field(row, 2)?,
    })
}

fn cast_spell(row: &SqlResult) -> Result<CastSpell, LoadError> {
    Ok(CastSpell {
        spell: spell(row)?,
        create_mode: field(row, 3)?,
    })
}

fn action(row: &SqlResult) -> Result<StartAction, LoadError> {
    Ok(StartAction {
        race: field(row, 0)?,
        class: field(row, 1)?,
        button: field(row, 2)?,
        action: field(row, 3)?,
        kind: field(row, 4)?,
    })
}

fn race_stats(row: &SqlResult) -> Result<RaceStats, LoadError> {
    Ok(RaceStats {
        race: field(row, 0)?,
        modifiers: [
            field(row, 1)?,
            field(row, 2)?,
            field(row, 3)?,
            field(row, 4)?,
            field(row, 5)?,
        ],
    })
}

fn class_stats(row: &SqlResult) -> Result<ClassLevelStats, LoadError> {
    // Spirit is SMALLINT in target SQL, but the source reads all five as
    // int32. SQLx's checked field decoder accepts compatible integer widths.
    Ok(ClassLevelStats {
        class: field(row, 0)?,
        level: field(row, 1)?,
        stats: [
            field(row, 2)?,
            field(row, 3)?,
            field(row, 4)?,
            field(row, 5)?,
            field(row, 6)?,
        ],
    })
}

fn xp(row: &SqlResult) -> Result<XpOverride, LoadError> {
    Ok(XpOverride {
        level: field(row, 0)?,
        experience: field(row, 1)?,
    })
}

impl CreationWorldRepository for ForeverCreationWorldRepository {
    fn load_item_addons(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        Result<Vec<wow_persistence::forever::item_specs::ItemAddonRow>, LoadError>,
    > {
        Box::pin(item_addons::load(&self.0))
    }
    fn load_character_templates(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        Result<wow_persistence::forever::creation::templates::TemplateRows, LoadError>,
    > {
        Box::pin(templates::load(&self.0))
    }
    fn load_creation_world(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, Result<CreationWorldRows, LoadError>> {
        Box::pin(async move {
            let definitions = rows(&self.0, QUERIES[0], definition).await?;
            if definitions.is_empty() {
                return Err(LoadError::InvalidRow);
            }
            let item_overrides = rows(&self.0, QUERIES[1], item).await?;
            let custom_spells = rows(&self.0, QUERIES[2], spell).await?;
            let cast_spells = rows(&self.0, QUERIES[3], cast_spell).await?;
            let actions = rows(&self.0, QUERIES[4], action).await?;
            let race_stats = rows(&self.0, QUERIES[5], race_stats).await?;
            if race_stats.is_empty() {
                return Err(LoadError::InvalidRow);
            }
            let class_stats = rows(&self.0, QUERIES[6], class_stats).await?;
            if class_stats.is_empty() {
                return Err(LoadError::InvalidRow);
            }
            let xp_overrides = rows(&self.0, QUERIES[7], xp).await?;
            // World.cpp:1702,1776 loads PlayerInfo before SkillTiers. The
            // immutable birth-source batch publishes only after both succeed.
            let skill_tiers = rows(&self.0, skill_tiers::QUERY, skill_tiers::decode).await?;
            Ok(CreationWorldRows {
                definitions,
                item_overrides,
                custom_spells,
                cast_spells,
                actions,
                race_stats,
                class_stats,
                xp_overrides,
                skill_tiers,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_queries_preserve_optional_columns_signed_stats_and_order() {
        assert_eq!(QUERIES.len(), 8);
        assert!(
            QUERIES[0].ends_with(
                "intro_movie_id, intro_scene_id, npe_intro_scene_id FROM playercreateinfo"
            )
        );
        assert!(!QUERIES[0].contains("JOIN")); // no 3.4.3 transport-entry join
        assert_eq!(
            QUERIES[6],
            "SELECT class, level, str, agi, sta, inte, spi FROM player_classlevelstats"
        );
        assert_eq!(
            QUERIES[7],
            "SELECT Level, Experience FROM player_xp_for_level"
        );
        assert!(QUERIES.iter().all(|query| query.starts_with("SELECT ")));
    }
}
