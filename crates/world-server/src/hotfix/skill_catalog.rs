//! Composition boundary for the effective C++ skill catalog.

use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;
use wow_data::trait_tree::TraitCatalogOverlayValueLikeCpp;
use wow_persistence::{
    SkillCatalogHotfixLoadOutcomeLikeCpp, SkillCatalogHotfixPersistencePortLikeCpp,
    SkillLineAbilityHotfixRowLikeCpp, SkillLineAbilityHotfixRowsLikeCpp, SkillLineHotfixRowLikeCpp,
    SkillLineXTraitTreeHotfixRowLikeCpp, SkillRaceClassInfoHotfixRowLikeCpp,
    SkillRaceClassInfoHotfixRowsLikeCpp, TraitCatalogHotfixRowLikeCpp,
    TraitCatalogHotfixRowsLikeCpp, TraitCatalogHotfixTableLikeCpp, TraitCatalogHotfixValueLikeCpp,
};

type TraitOverlayBatchesLikeCpp = HashMap<
    wow_data::trait_tree::TraitCatalogOverlayTableLikeCpp,
    (
        Vec<wow_data::trait_tree::TraitCatalogOverlayRowLikeCpp>,
        Vec<wow_data::trait_tree::TraitCatalogOverlayRowLikeCpp>,
    ),
>;

fn trait_overlay_table_like_cpp(
    table: TraitCatalogHotfixTableLikeCpp,
) -> wow_data::trait_tree::TraitCatalogOverlayTableLikeCpp {
    use wow_data::trait_tree::TraitCatalogOverlayTableLikeCpp as T;
    match table {
        TraitCatalogHotfixTableLikeCpp::SpecSetMember => T::SpecSetMember,
        TraitCatalogHotfixTableLikeCpp::TraitCond => T::TraitCond,
        TraitCatalogHotfixTableLikeCpp::TraitCost => T::TraitCost,
        TraitCatalogHotfixTableLikeCpp::TraitCurrency => T::TraitCurrency,
        TraitCatalogHotfixTableLikeCpp::TraitCurrencySource => T::TraitCurrencySource,
        TraitCatalogHotfixTableLikeCpp::TraitDefinition => T::TraitDefinition,
        TraitCatalogHotfixTableLikeCpp::TraitDefinitionEffectPoints => {
            T::TraitDefinitionEffectPoints
        }
        TraitCatalogHotfixTableLikeCpp::TraitEdge => T::TraitEdge,
        TraitCatalogHotfixTableLikeCpp::TraitNode => T::TraitNode,
        TraitCatalogHotfixTableLikeCpp::TraitNodeEntry => T::TraitNodeEntry,
        TraitCatalogHotfixTableLikeCpp::TraitNodeEntryXTraitCond => T::TraitNodeEntryXTraitCond,
        TraitCatalogHotfixTableLikeCpp::TraitNodeEntryXTraitCost => T::TraitNodeEntryXTraitCost,
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroup => T::TraitNodeGroup,
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroupXTraitCond => T::TraitNodeGroupXTraitCond,
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroupXTraitCost => T::TraitNodeGroupXTraitCost,
        TraitCatalogHotfixTableLikeCpp::TraitNodeGroupXTraitNode => T::TraitNodeGroupXTraitNode,
        TraitCatalogHotfixTableLikeCpp::TraitNodeXTraitCond => T::TraitNodeXTraitCond,
        TraitCatalogHotfixTableLikeCpp::TraitNodeXTraitCost => T::TraitNodeXTraitCost,
        TraitCatalogHotfixTableLikeCpp::TraitNodeXTraitNodeEntry => T::TraitNodeXTraitNodeEntry,
        TraitCatalogHotfixTableLikeCpp::TraitTree => T::TraitTree,
        TraitCatalogHotfixTableLikeCpp::TraitTreeLoadout => T::TraitTreeLoadout,
        TraitCatalogHotfixTableLikeCpp::TraitTreeLoadoutEntry => T::TraitTreeLoadoutEntry,
        TraitCatalogHotfixTableLikeCpp::TraitTreeXTraitCost => T::TraitTreeXTraitCost,
        TraitCatalogHotfixTableLikeCpp::TraitTreeXTraitCurrency => T::TraitTreeXTraitCurrency,
    }
}

fn trait_overlay_value_like_cpp(
    value: TraitCatalogHotfixValueLikeCpp,
) -> TraitCatalogOverlayValueLikeCpp {
    match value {
        TraitCatalogHotfixValueLikeCpp::Integer(value) => {
            TraitCatalogOverlayValueLikeCpp::Integer(value)
        }
        TraitCatalogHotfixValueLikeCpp::Real(value) => TraitCatalogOverlayValueLikeCpp::Real(value),
        TraitCatalogHotfixValueLikeCpp::Text(value) => TraitCatalogOverlayValueLikeCpp::Text(value),
        TraitCatalogHotfixValueLikeCpp::Null => TraitCatalogOverlayValueLikeCpp::Null,
    }
}

fn trait_overlay_row_like_cpp(
    row: TraitCatalogHotfixRowLikeCpp,
) -> wow_data::trait_tree::TraitCatalogOverlayRowLikeCpp {
    wow_data::trait_tree::TraitCatalogOverlayRowLikeCpp {
        table: trait_overlay_table_like_cpp(row.table),
        values: row
            .values
            .into_iter()
            .map(trait_overlay_value_like_cpp)
            .collect(),
    }
}

fn trait_overlay_batches_like_cpp(
    rows: TraitCatalogHotfixRowsLikeCpp,
) -> TraitOverlayBatchesLikeCpp {
    let mut batches = TraitOverlayBatchesLikeCpp::new();
    for (index, source) in [rows.official, rows.custom].into_iter().enumerate() {
        for row in source {
            let row = trait_overlay_row_like_cpp(row);
            let batch = batches.entry(row.table).or_default();
            match index {
                0 => batch.0.push(row),
                1 => batch.1.push(row),
                _ => unreachable!("Trait hotfix source index is fixed to official/custom"),
            }
        }
    }
    batches
}

fn skill_line_overlay_like_cpp(
    row: SkillLineHotfixRowLikeCpp,
) -> wow_data::SkillLineHotfixOverlayLikeCpp {
    wow_data::SkillLineHotfixOverlayLikeCpp {
        id: row.id,
        category_id: row.category_id,
        parent_skill_line_id: row.parent_skill_line_id,
        parent_tier_index: row.parent_tier_index,
    }
}

fn skill_line_ability_source_like_cpp(
    row: SkillLineAbilityHotfixRowLikeCpp,
    source: wow_data::SkillStoreLoadSourceLikeCpp,
) -> wow_data::SkillLineAbilitySourceRecordLikeCpp {
    wow_data::SkillLineAbilitySourceRecordLikeCpp {
        source,
        id: row.id,
        race_mask: row.race_mask,
        skill_line: row.skill_line,
        spell: row.spell,
        min_skill_line_rank: row.min_skill_line_rank,
        class_mask: row.class_mask,
        supercedes_spell: row.supercedes_spell,
        acquire_method: row.acquire_method,
        trivial_rank_high: row.trivial_rank_high,
        trivial_rank_low: row.trivial_rank_low,
        flags: row.flags,
        num_skill_ups: row.num_skill_ups,
        skillup_skill_line_id: row.skillup_skill_line_id,
    }
}

fn skill_race_class_info_source_like_cpp(
    row: SkillRaceClassInfoHotfixRowLikeCpp,
    source: wow_data::SkillStoreLoadSourceLikeCpp,
) -> wow_data::SkillRaceClassInfoSourceRecordLikeCpp {
    wow_data::SkillRaceClassInfoSourceRecordLikeCpp {
        source,
        id: row.id,
        race_mask: row.race_mask,
        skill_id: row.skill_id,
        class_mask: row.class_mask,
        flags: row.flags,
        availability: row.availability,
        min_level: row.min_level,
        skill_tier_id: row.skill_tier_id,
    }
}

fn skill_line_x_trait_tree_entry_like_cpp(
    row: SkillLineXTraitTreeHotfixRowLikeCpp,
) -> Result<wow_data::SkillLineXTraitTreeEntry> {
    Ok(wow_data::SkillLineXTraitTreeEntry {
        id: row.id,
        skill_line_id: row.skill_line_id,
        trait_tree_id: i32::try_from(row.trait_tree_id)
            .context("SkillLineXTraitTree.TraitTreeID is not i32")?,
        order_index: i32::try_from(row.order_index)
            .context("SkillLineXTraitTree.OrderIndex is not i32")?,
    })
}

fn apply_skill_line_hotfix_outcome_like_cpp(
    base: wow_data::SkillLineStore,
    outcome: SkillCatalogHotfixLoadOutcomeLikeCpp<wow_persistence::SkillLineHotfixRowsLikeCpp>,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<wow_data::SkillLineStore> {
    let rows = match outcome {
        SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SkillCatalogHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    base.apply_hotfix_overlays_like_cpp(
        rows.official.into_iter().map(skill_line_overlay_like_cpp),
        rows.custom.into_iter().map(skill_line_overlay_like_cpp),
        removals,
    )
}

pub(crate) async fn load_skill_line_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
) -> Result<wow_data::SkillLineStore> {
    let base = wow_data::SkillLineStore::load(data_dir, locale)?;
    apply_skill_line_hotfix_outcome_like_cpp(
        base,
        persistence.load_skill_line_hotfix_rows_like_cpp().await,
        removals,
    )
}

pub(crate) async fn load_skill_line_ability_hotfix_rows_like_cpp(
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
) -> Result<SkillLineAbilityHotfixRowsLikeCpp> {
    let rows = match persistence
        .load_skill_line_ability_hotfix_rows_like_cpp()
        .await
    {
        SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SkillCatalogHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    Ok(rows)
}

pub(crate) async fn load_skill_race_class_info_hotfix_rows_like_cpp(
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
) -> Result<SkillRaceClassInfoHotfixRowsLikeCpp> {
    let rows = match persistence
        .load_skill_race_class_info_hotfix_rows_like_cpp()
        .await
    {
        SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SkillCatalogHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    Ok(rows)
}

pub(crate) fn compose_skill_store_like_cpp(
    ability_base: wow_data::SkillStoreWdc4AbilityBaseLikeCpp,
    race_class_info_base: wow_data::SkillStoreWdc4RaceClassInfoBaseLikeCpp,
    ability_rows: SkillLineAbilityHotfixRowsLikeCpp,
    race_class_info_rows: SkillRaceClassInfoHotfixRowsLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    skill_line_store: &wow_data::SkillLineStore,
) -> wow_data::SkillStoreEffectiveLoadOutcomeLikeCpp {
    use wow_data::SkillStoreLoadSourceLikeCpp::{CustomSql, OfficialSql};
    wow_data::SkillStore::compose_effective_from_hotfix_overlays_like_cpp(
        ability_base,
        race_class_info_base,
        ability_rows
            .official
            .into_iter()
            .map(|row| skill_line_ability_source_like_cpp(row, OfficialSql)),
        ability_rows
            .custom
            .into_iter()
            .map(|row| skill_line_ability_source_like_cpp(row, CustomSql)),
        race_class_info_rows
            .official
            .into_iter()
            .map(|row| skill_race_class_info_source_like_cpp(row, OfficialSql)),
        race_class_info_rows
            .custom
            .into_iter()
            .map(|row| skill_race_class_info_source_like_cpp(row, CustomSql)),
        removals,
        skill_line_store,
    )
}

pub(crate) struct SkillCatalogStagesLikeCpp {
    pub skill_store_outcome: wow_data::SkillStoreEffectiveLoadOutcomeLikeCpp,
    pub trait_tree_skill_line_index: Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>,
    pub trait_node_entry_store: Arc<wow_data::trait_tree::TraitNodeEntryStore>,
    pub trait_definition_store: Arc<wow_data::trait_tree::TraitDefinitionStore>,
}

struct TraitMgrCatalogLikeCpp {
    index: Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>,
    trait_node_entry_store: Arc<wow_data::trait_tree::TraitNodeEntryStore>,
    trait_definition_store: Arc<wow_data::trait_tree::TraitDefinitionStore>,
}

/// Load the relation catalog in the exact C++ table order. Keeping this
/// sequence beside the catalog owner makes it difficult for a later consumer
/// to reintroduce a combined relation read in the composition root.
pub(crate) async fn load_skill_catalog_stages_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    skill_line_store: &wow_data::SkillLineStore,
) -> Result<SkillCatalogStagesLikeCpp> {
    let ability_base =
        wow_data::SkillStore::load_wdc4_skill_line_ability_base_like_cpp(data_dir, locale)
            .context("Failed to load SkillLineAbility.db2")?;
    let ability_rows = load_skill_line_ability_hotfix_rows_like_cpp(persistence)
        .await
        .context("Failed to load SkillLineAbility hotfix rows")?;
    let race_class_info_base =
        wow_data::SkillStore::load_wdc4_skill_race_class_info_base_like_cpp(data_dir, locale)
            .context("Failed to load SkillRaceClassInfo.db2")?;
    let race_class_info_rows = load_skill_race_class_info_hotfix_rows_like_cpp(persistence)
        .await
        .context("Failed to load SkillRaceClassInfo hotfix rows")?;
    let skill_store_outcome = compose_skill_store_like_cpp(
        ability_base,
        race_class_info_base,
        ability_rows,
        race_class_info_rows,
        removals,
        skill_line_store,
    );
    // C++ calls TraitMgr::Load after all DB2 stores are loaded and effective;
    // build the combat class projection only once SkillRaceClassInfo is ready.
    let trait_mgr_catalog = load_trait_index_like_cpp(
        data_dir,
        locale,
        persistence,
        removals,
        skill_line_store,
        &skill_store_outcome.store,
    )
    .await?;
    Ok(SkillCatalogStagesLikeCpp {
        skill_store_outcome,
        trait_tree_skill_line_index: trait_mgr_catalog.index,
        trait_node_entry_store: trait_mgr_catalog.trait_node_entry_store,
        trait_definition_store: trait_mgr_catalog.trait_definition_store,
    })
}

async fn load_trait_index_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    skill_line_store: &wow_data::SkillLineStore,
    skill_store: &wow_data::SkillStore,
) -> Result<TraitMgrCatalogLikeCpp> {
    let hotfix_rows = match persistence.load_trait_catalog_hotfix_rows_like_cpp().await {
        SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SkillCatalogHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    let mut hotfix_batches = trait_overlay_batches_like_cpp(hotfix_rows);
    let take_hotfix =
        |batches: &mut TraitOverlayBatchesLikeCpp,
         table: wow_data::trait_tree::TraitCatalogOverlayTableLikeCpp| {
            batches.remove(&table).unwrap_or_default()
        };

    macro_rules! trait_store {
        ($store:ty, $file:literal, $table:ident) => {{
            let store = <$store>::load(data_dir, locale)
                .with_context(|| format!("Failed to load {}", $file))?;
            let (official, custom) = take_hotfix(
                &mut hotfix_batches,
                wow_data::trait_tree::TraitCatalogOverlayTableLikeCpp::$table,
            );
            store
                .apply_trait_catalog_hotfix_like_cpp(official, custom, removals)
                .with_context(|| format!("Failed to apply {} hotfix overlays", $file))?
        }};
    }

    let trait_tree_store = trait_store!(
        wow_data::trait_tree::TraitTreeStore,
        "TraitTree.db2",
        TraitTree
    );
    let trait_node_store = trait_store!(
        wow_data::trait_tree::TraitNodeStore,
        "TraitNode.db2",
        TraitNode
    );
    let trait_node_entry_store = trait_store!(
        wow_data::trait_tree::TraitNodeEntryStore,
        "TraitNodeEntry.db2",
        TraitNodeEntry
    );
    let trait_node_entry_x_trait_cond_store = trait_store!(
        wow_data::trait_tree::TraitNodeEntryXTraitCondStore,
        "TraitNodeEntryXTraitCond.db2",
        TraitNodeEntryXTraitCond
    );
    let trait_node_entry_x_trait_cost_store = trait_store!(
        wow_data::trait_tree::TraitNodeEntryXTraitCostStore,
        "TraitNodeEntryXTraitCost.db2",
        TraitNodeEntryXTraitCost
    );
    let trait_node_group_store = trait_store!(
        wow_data::trait_tree::TraitNodeGroupStore,
        "TraitNodeGroup.db2",
        TraitNodeGroup
    );
    let trait_node_group_x_trait_cond_store = trait_store!(
        wow_data::trait_tree::TraitNodeGroupXTraitCondStore,
        "TraitNodeGroupXTraitCond.db2",
        TraitNodeGroupXTraitCond
    );
    let trait_node_group_x_trait_cost_store = trait_store!(
        wow_data::trait_tree::TraitNodeGroupXTraitCostStore,
        "TraitNodeGroupXTraitCost.db2",
        TraitNodeGroupXTraitCost
    );
    let trait_node_group_x_trait_node_store = trait_store!(
        wow_data::trait_tree::TraitNodeGroupXTraitNodeStore,
        "TraitNodeGroupXTraitNode.db2",
        TraitNodeGroupXTraitNode
    );
    let trait_node_x_trait_cond_store = trait_store!(
        wow_data::trait_tree::TraitNodeXTraitCondStore,
        "TraitNodeXTraitCond.db2",
        TraitNodeXTraitCond
    );
    let trait_node_x_trait_cost_store = trait_store!(
        wow_data::trait_tree::TraitNodeXTraitCostStore,
        "TraitNodeXTraitCost.db2",
        TraitNodeXTraitCost
    );
    let trait_node_x_trait_node_entry_store = trait_store!(
        wow_data::trait_tree::TraitNodeXTraitNodeEntryStore,
        "TraitNodeXTraitNodeEntry.db2",
        TraitNodeXTraitNodeEntry
    );
    let trait_edge_store = trait_store!(
        wow_data::trait_tree::TraitEdgeStore,
        "TraitEdge.db2",
        TraitEdge
    );
    let trait_cost_store = trait_store!(
        wow_data::trait_tree::TraitCostStore,
        "TraitCost.db2",
        TraitCost
    );
    let trait_currency_store = trait_store!(
        wow_data::trait_tree::TraitCurrencyStore,
        "TraitCurrency.db2",
        TraitCurrency
    );
    let trait_currency_source_store = trait_store!(
        wow_data::trait_tree::TraitCurrencySourceStore,
        "TraitCurrencySource.db2",
        TraitCurrencySource
    );
    let trait_cond_store = trait_store!(
        wow_data::trait_tree::TraitCondStore,
        "TraitCond.db2",
        TraitCond
    );
    let trait_definition_store = trait_store!(
        wow_data::trait_tree::TraitDefinitionStore,
        "TraitDefinition.db2",
        TraitDefinition
    );
    let _trait_definition_effect_points_store = trait_store!(
        wow_data::trait_tree::TraitDefinitionEffectPointsStore,
        "TraitDefinitionEffectPoints.db2",
        TraitDefinitionEffectPoints
    );
    let trait_tree_loadout_store = trait_store!(
        wow_data::trait_tree::TraitTreeLoadoutStore,
        "TraitTreeLoadout.db2",
        TraitTreeLoadout
    );
    let trait_tree_loadout_entry_store = trait_store!(
        wow_data::trait_tree::TraitTreeLoadoutEntryStore,
        "TraitTreeLoadoutEntry.db2",
        TraitTreeLoadoutEntry
    );
    let trait_tree_x_trait_cost_store = trait_store!(
        wow_data::trait_tree::TraitTreeXTraitCostStore,
        "TraitTreeXTraitCost.db2",
        TraitTreeXTraitCost
    );
    let trait_tree_x_trait_currency_store = trait_store!(
        wow_data::trait_tree::TraitTreeXTraitCurrencyStore,
        "TraitTreeXTraitCurrency.db2",
        TraitTreeXTraitCurrency
    );
    let spec_set_member_store = {
        let store = wow_data::SpecSetMemberStore::load(data_dir, locale)
            .context("Failed to load SpecSetMember.db2")?;
        let (official, custom) = take_hotfix(
            &mut hotfix_batches,
            wow_data::trait_tree::TraitCatalogOverlayTableLikeCpp::SpecSetMember,
        );
        store
            .apply_trait_catalog_hotfix_like_cpp(official, custom, removals)
            .context("Failed to apply SpecSetMember hotfix overlays")?
    };
    let skill_line_x_trait_tree_store = wow_data::SkillLineXTraitTreeStore::load(data_dir, locale)
        .context("Failed to load SkillLineXTraitTree.db2")?;
    let hotfix_rows = match persistence
        .load_skill_line_x_trait_tree_hotfix_rows_like_cpp()
        .await
    {
        SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SkillCatalogHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    let official_links = hotfix_rows
        .official
        .into_iter()
        .map(skill_line_x_trait_tree_entry_like_cpp)
        .collect::<Result<Vec<_>>>()?;
    let custom_links = hotfix_rows
        .custom
        .into_iter()
        .map(skill_line_x_trait_tree_entry_like_cpp)
        .collect::<Result<Vec<_>>>()?;
    let skill_line_x_trait_tree_store = skill_line_x_trait_tree_store
        .apply_hotfix_overlays_and_removals_like_cpp(official_links, custom_links, removals)
        .context("Failed to apply SkillLineXTraitTree hotfix removals")?;
    let index = Arc::new(
        wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &skill_line_x_trait_tree_store,
            &trait_tree_store,
            |skill_line_id| skill_line_store.contains_effective_record_like_cpp(skill_line_id),
            |skill_line_id| {
                let Some(skill_line) = skill_line_store.get(skill_line_id) else {
                    return Vec::new();
                };
                if skill_line.category_id != wow_data::SKILL_CATEGORY_CLASS_LIKE_CPP {
                    return Vec::new();
                }
                skill_store.class_ids_for_skill_line_like_cpp(skill_line_id)
            },
        )
        .with_trait_graph_like_cpp(
            &trait_tree_store,
            &trait_node_store,
            &trait_node_entry_store,
            &trait_node_entry_x_trait_cond_store,
            &trait_node_entry_x_trait_cost_store,
            &trait_node_group_store,
            &trait_node_group_x_trait_cond_store,
            &trait_node_group_x_trait_cost_store,
            &trait_node_group_x_trait_node_store,
            &trait_node_x_trait_cond_store,
            &trait_node_x_trait_cost_store,
            &trait_node_x_trait_node_entry_store,
            &trait_edge_store,
            &trait_cost_store,
            &trait_cond_store,
            &trait_tree_loadout_store,
            &trait_tree_loadout_entry_store,
            &trait_tree_x_trait_cost_store,
        )
        .with_trait_currency_data_like_cpp(
            &trait_currency_store,
            &trait_currency_source_store,
            &trait_tree_x_trait_currency_store,
            &spec_set_member_store,
        ),
    );
    info!(
        trait_tree_links = index.len(),
        trait_graph_nodes = trait_node_store.len(),
        trait_graph_groups = trait_node_group_store.len(),
        trait_graph_loadouts = trait_tree_loadout_store.len(),
        "Loaded C++ TraitMgr skill-line/combat and node graph index"
    );
    Ok(TraitMgrCatalogLikeCpp {
        index,
        trait_node_entry_store: Arc::new(trait_node_entry_store),
        trait_definition_store: Arc::new(trait_definition_store),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_rows_preserve_raw_domains_and_source_classification() {
        let source = skill_line_ability_source_like_cpp(
            SkillLineAbilityHotfixRowLikeCpp {
                id: 1,
                race_mask: -2,
                skill_line: -3,
                spell: 4,
                min_skill_line_rank: -5,
                class_mask: 6,
                supercedes_spell: -7,
                acquire_method: 8,
                trivial_rank_high: -9,
                trivial_rank_low: 10,
                flags: -11,
                num_skill_ups: 12,
                skillup_skill_line_id: -13,
            },
            wow_data::SkillStoreLoadSourceLikeCpp::CustomSql,
        );
        assert_eq!(source.id, 1);
        assert_eq!(source.race_mask, -2);
        assert_eq!(source.skillup_skill_line_id, -13);
        assert_eq!(
            source.source,
            wow_data::SkillStoreLoadSourceLikeCpp::CustomSql
        );
        let trait_link =
            skill_line_x_trait_tree_entry_like_cpp(SkillLineXTraitTreeHotfixRowLikeCpp {
                id: 2,
                skill_line_id: 171,
                trait_tree_id: -3,
                order_index: 4,
            })
            .unwrap();
        assert_eq!(trait_link.trait_tree_id, -3);
        assert_eq!(trait_link.order_index, 4);
    }

    #[test]
    fn failed_stage_stops_before_domain_application() {
        let result = apply_skill_line_hotfix_outcome_like_cpp(
            wow_data::SkillLineStore::from_entries([]),
            SkillCatalogHotfixLoadOutcomeLikeCpp::Failed {
                reason: "skill Hotfix read failed".into(),
            },
            &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
        );
        let error = match result {
            Ok(_) => panic!("failed Hotfix stage must not publish a SkillLine store"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "skill Hotfix read failed");
    }

    #[test]
    fn catalog_loads_all_skill_inputs_before_trait_mgr_projection() {
        let source = include_str!("skill_catalog.rs");
        let loader = source
            .find("fn load_skill_catalog_stages_like_cpp")
            .expect("catalog stage loader must remain explicit");
        let body = &source[loader..];
        let ability = body
            .find("load_wdc4_skill_line_ability_base_like_cpp")
            .expect("SkillLineAbility WDC4 stage must remain explicit");
        let ability_hotfix = body
            .find("load_skill_line_ability_hotfix_rows_like_cpp")
            .expect("SkillLineAbility hotfix stage must remain explicit");
        let trait_tree = body
            .find("load_trait_index_like_cpp")
            .expect("TraitMgr SkillLineXTraitTree stage must remain composed");
        let race_class = body
            .find("load_wdc4_skill_race_class_info_base_like_cpp")
            .expect("SkillRaceClassInfo WDC4 stage must remain explicit");
        let race_class_hotfix = body
            .find("load_skill_race_class_info_hotfix_rows_like_cpp")
            .expect("SkillRaceClassInfo hotfix stage must remain explicit");
        assert!(
            ability < ability_hotfix
                && ability_hotfix < race_class
                && race_class < race_class_hotfix
                && race_class_hotfix < trait_tree
        );
    }

    #[test]
    fn trait_index_loader_owns_both_wdc4_stages() {
        let source = include_str!("skill_catalog.rs");
        let loader = source
            .find("fn load_trait_index_like_cpp")
            .expect("TraitMgr index loader must remain explicit");
        let body = &source[loader..];
        assert!(body.contains("TraitTreeStore::load"));
        assert!(body.contains("SkillLineXTraitTreeStore::load"));
        assert!(body.contains("load_skill_line_x_trait_tree_hotfix_rows_like_cpp"));
        assert!(body.contains("apply_hotfix_overlays_and_removals_like_cpp"));
        assert!(body.contains("class_ids_for_skill_line_like_cpp"));
        assert!(body.contains("removals"));
    }
}
