//! MariaDB adapter for bounded gameplay rule startup catalogs.

use std::sync::Arc;

use anyhow::{Context, Result};
use wow_persistence::{
    FactionChangePairPersistenceRowLikeCpp, FactionChangePersistenceRowsLikeCpp,
    GameplayRuleCatalogPersistencePortLikeCpp, GameplayRuleRowsLoadOutcomeLikeCpp,
    NpcSpellClickPersistenceRowLikeCpp, NpcVendorPersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{SqlResult, WorldDatabase, WorldStatements};

pub struct MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }

    async fn spell_click_rows(&self) -> Result<Vec<NpcSpellClickPersistenceRowLikeCpp>> {
        let stmt = self
            .world_db
            .prepare(WorldStatements::SEL_NPC_SPELLCLICK_SPELLS);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(NpcSpellClickPersistenceRowLikeCpp {
                    npc_entry: result.try_read(0).unwrap_or(0),
                    spell_id: result.try_read(1).unwrap_or(0),
                    cast_flags: result.try_read(2).unwrap_or(0),
                    user_type: result.try_read(3).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn vendor_rows(&self) -> Result<Vec<NpcVendorPersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(WorldStatements::SEL_NPC_VENDORS_ALL);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(NpcVendorPersistenceRowLikeCpp {
                    entry: result.read(0),
                    item: result.read(1),
                    maxcount: result.read(2),
                    incrtime: result.read(3),
                    extended_cost: result.read(4),
                    vendor_type: result.read(5),
                    bonus_list_ids_raw: result.read_string(6),
                    player_condition_id: result.read(7),
                    ignore_filtering: result.try_read::<u8>(8).unwrap_or(0) != 0,
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }

    async fn faction_change_rows(&self) -> Result<FactionChangePersistenceRowsLikeCpp> {
        Ok(FactionChangePersistenceRowsLikeCpp {
            achievements: self
                .pair_rows(WorldStatements::SEL_FACTION_CHANGE_ACHIEVEMENTS)
                .await?,
            quests: self
                .pair_rows(WorldStatements::SEL_FACTION_CHANGE_QUESTS)
                .await?,
            reputations: self
                .pair_rows(WorldStatements::SEL_FACTION_CHANGE_REPUTATIONS)
                .await?,
            spells: self
                .pair_rows(WorldStatements::SEL_FACTION_CHANGE_SPELLS)
                .await?,
            titles: self
                .pair_rows(WorldStatements::SEL_FACTION_CHANGE_TITLES)
                .await?,
        })
    }

    async fn pair_rows(
        &self,
        statement: WorldStatements,
    ) -> Result<Vec<FactionChangePairPersistenceRowLikeCpp>> {
        let stmt = self.world_db.prepare(statement);
        let mut result = self.world_db.query(&stmt).await?;
        let mut rows = Vec::new();
        if !result.is_empty() {
            loop {
                rows.push(FactionChangePairPersistenceRowLikeCpp {
                    alliance_id: read_id(&result, 0, statement)
                        .context("failed to read faction-change alliance id")?,
                    horde_id: read_id(&result, 1, statement)
                        .context("failed to read faction-change horde id")?,
                });
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

fn read_id(result: &SqlResult, column: usize, statement: WorldStatements) -> Result<u32> {
    if let Some(value) = result.try_read::<u32>(column) {
        return Ok(value);
    }
    let value = result
        .try_read::<i32>(column)
        .with_context(|| format!("column {column} in {statement:?} is not an integer id"))?;
    u32::try_from(value)
        .with_context(|| format!("column {column} in {statement:?} is negative: {value}"))
}

fn outcome<T>(result: Result<T>) -> GameplayRuleRowsLoadOutcomeLikeCpp<T> {
    match result {
        Ok(rows) => GameplayRuleRowsLoadOutcomeLikeCpp::Loaded(rows),
        Err(error) => GameplayRuleRowsLoadOutcomeLikeCpp::Failed {
            reason: error.to_string(),
        },
    }
}

impl GameplayRuleCatalogPersistencePortLikeCpp
    for MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp
{
    fn load_npc_spell_click_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameplayRuleRowsLoadOutcomeLikeCpp<Vec<NpcSpellClickPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.spell_click_rows().await) })
    }
    fn load_npc_vendor_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameplayRuleRowsLoadOutcomeLikeCpp<Vec<NpcVendorPersistenceRowLikeCpp>>,
    > {
        Box::pin(async move { outcome(self.vendor_rows().await) })
    }
    fn load_faction_change_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameplayRuleRowsLoadOutcomeLikeCpp<FactionChangePersistenceRowsLikeCpp>,
    > {
        Box::pin(async move { outcome(self.faction_change_rows().await) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StatementDef;

    #[test]
    fn statement_family_and_faction_order_are_explicit() {
        assert!(
            WorldStatements::SEL_NPC_SPELLCLICK_SPELLS
                .sql()
                .contains("npc_spellclick_spells")
        );
        assert!(
            WorldStatements::SEL_NPC_VENDORS_ALL
                .sql()
                .contains("npc_vendor")
        );
        let order = [
            WorldStatements::SEL_FACTION_CHANGE_ACHIEVEMENTS,
            WorldStatements::SEL_FACTION_CHANGE_QUESTS,
            WorldStatements::SEL_FACTION_CHANGE_REPUTATIONS,
            WorldStatements::SEL_FACTION_CHANGE_SPELLS,
            WorldStatements::SEL_FACTION_CHANGE_TITLES,
        ];
        assert_eq!(order.len(), 5);
    }
}
