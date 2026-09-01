//! MariaDB adapter for the bounded DB2/rule overlay startup capability.

use std::sync::Arc;

use anyhow::Result;
use wow_persistence::{
    AreaTableHotfixRowLikeCpp, PersistenceFutureLikeCpp, PowerTypeHotfixRowLikeCpp,
    SpellEnchantProcPersistenceRowLikeCpp, StaticDataOverlayPersistencePortLikeCpp,
    StaticDataRowsLoadOutcomeLikeCpp, UiMapXMapArtHotfixRowLikeCpp,
};

use crate::{HotfixDatabase, HotfixStatements, WorldDatabase, WorldStatements};

const OFFICIAL_THEN_CUSTOM_LIKE_CPP: [bool; 2] = [true, false];

pub struct MariaDbStaticDataOverlayPersistenceAdapterLikeCpp {
    hotfix_db: Arc<HotfixDatabase>,
    world_db: Arc<WorldDatabase>,
}

impl MariaDbStaticDataOverlayPersistenceAdapterLikeCpp {
    pub fn new(hotfix_db: Arc<HotfixDatabase>, world_db: Arc<WorldDatabase>) -> Self {
        Self {
            hotfix_db,
            world_db,
        }
    }

    async fn area_rows(&self) -> Result<Vec<AreaTableHotfixRowLikeCpp>> {
        let stmt = self.hotfix_db.prepare(HotfixStatements::SEL_AREA_TABLE);
        let mut result = self.hotfix_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(AreaTableHotfixRowLikeCpp {
                    id: result.read(0),
                    continent_id: result.read(3),
                    parent_area_id: result.read(4),
                    area_bit: result.read(5),
                    exploration_level: result.read(12),
                    faction_group_mask: result.read(15),
                    mount_flags: result.read(17),
                    flags: result.read(22),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn power_rows(
        &self,
    ) -> Result<(
        Vec<PowerTypeHotfixRowLikeCpp>,
        Vec<PowerTypeHotfixRowLikeCpp>,
    )> {
        let mut batches = [Vec::new(), Vec::new()];
        for (index, official) in OFFICIAL_THEN_CUSTOM_LIKE_CPP.into_iter().enumerate() {
            let mut stmt = self.hotfix_db.prepare(HotfixStatements::SEL_POWER_TYPE);
            stmt.set_bool(0, official);
            let mut result = self.hotfix_db.query(&stmt).await?;
            if result.is_empty() {
                continue;
            }
            loop {
                if let Some(id) = result.try_read::<u32>(0) {
                    batches[index].push(PowerTypeHotfixRowLikeCpp {
                        id,
                        name_global_string_tag: result.try_read(1).unwrap_or_default(),
                        cost_global_string_tag: result.try_read(2).unwrap_or_default(),
                        power_type_enum: result.try_read(3).unwrap_or_default(),
                        min_power: result.try_read(4).unwrap_or_default(),
                        max_base_power: result.try_read(5).unwrap_or_default(),
                        center_power: result.try_read(6).unwrap_or_default(),
                        default_power: result.try_read(7).unwrap_or_default(),
                        display_modifier: result.try_read(8).unwrap_or_default(),
                        regen_interrupt_time_ms: result.try_read(9).unwrap_or_default(),
                        regen_peace: result.try_read(10).unwrap_or_default(),
                        regen_combat: result.try_read(11).unwrap_or_default(),
                        flags: result.try_read(12).unwrap_or_default(),
                    });
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        let [official, custom] = batches;
        Ok((official, custom))
    }

    async fn ui_map_rows(&self) -> Result<Vec<UiMapXMapArtHotfixRowLikeCpp>> {
        let stmt = self
            .hotfix_db
            .prepare(HotfixStatements::SEL_UI_MAP_X_MAP_ART);
        let mut result = self.hotfix_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(UiMapXMapArtHotfixRowLikeCpp {
                    id: result.read(0),
                    phase_id: result.read(1),
                    ui_map_art_id: result.read(2),
                    ui_map_id: result.read(3),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn enchant_rows(&self) -> Result<Vec<SpellEnchantProcPersistenceRowLikeCpp>> {
        let stmt = self
            .world_db
            .prepare(WorldStatements::SEL_SPELL_ENCHANT_PROC_DATA);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(SpellEnchantProcPersistenceRowLikeCpp {
                    enchant_id: result.try_read(0).unwrap_or(0),
                    chance: result.try_read(1).unwrap_or(0.0),
                    procs_per_minute: result.try_read(2).unwrap_or(0.0),
                    hit_mask: result.try_read(3).unwrap_or(0),
                    attributes_mask: result.try_read(4).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

macro_rules! load_outcome {
    ($future:expr) => {{
        match $future.await {
            Ok(rows) => StaticDataRowsLoadOutcomeLikeCpp::Loaded(rows),
            Err(error) => StaticDataRowsLoadOutcomeLikeCpp::Failed {
                reason: error.to_string(),
            },
        }
    }};
}

impl StaticDataOverlayPersistencePortLikeCpp for MariaDbStaticDataOverlayPersistenceAdapterLikeCpp {
    fn load_area_table_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<Vec<AreaTableHotfixRowLikeCpp>>,
    > {
        Box::pin(async move { load_outcome!(self.area_rows()) })
    }

    fn load_power_type_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<(
            Vec<PowerTypeHotfixRowLikeCpp>,
            Vec<PowerTypeHotfixRowLikeCpp>,
        )>,
    > {
        Box::pin(async move { load_outcome!(self.power_rows()) })
    }

    fn load_ui_map_x_map_art_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<Vec<UiMapXMapArtHotfixRowLikeCpp>>,
    > {
        Box::pin(async move { load_outcome!(self.ui_map_rows()) })
    }

    fn load_spell_enchant_proc_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<Vec<SpellEnchantProcPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { load_outcome!(self.enchant_rows()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn bounded_sources_keep_exact_statement_identity_and_overlay_order() {
        assert!(
            HotfixStatements::SEL_AREA_TABLE
                .sql()
                .contains("FROM area_table")
        );
        assert!(
            HotfixStatements::SEL_POWER_TYPE
                .sql()
                .contains("FROM power_type")
        );
        assert!(
            HotfixStatements::SEL_UI_MAP_X_MAP_ART
                .sql()
                .contains("FROM ui_map_x_map_art")
        );
        assert_eq!(OFFICIAL_THEN_CUSTOM_LIKE_CPP, [true, false]);
        assert!(
            WorldStatements::SEL_SPELL_ENCHANT_PROC_DATA
                .sql()
                .contains("FROM spell_enchant_proc_data")
        );
    }
}
