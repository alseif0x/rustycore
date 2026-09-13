//! Composition boundary for the effective C++ skill catalog.

use anyhow::{Context, Result, bail};
use std::sync::Arc;
use tracing::info;
use wow_persistence::{
    SkillCatalogHotfixLoadOutcomeLikeCpp, SkillCatalogHotfixPersistencePortLikeCpp,
    SkillLineAbilityHotfixRowLikeCpp, SkillLineAbilityHotfixRowsLikeCpp, SkillLineHotfixRowLikeCpp,
    SkillLineXTraitTreeHotfixRowLikeCpp, SkillLineXTraitTreeHotfixRowsLikeCpp,
    SkillRaceClassInfoHotfixRowLikeCpp, SkillRaceClassInfoHotfixRowsLikeCpp,
};

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
    let trait_tree_skill_line_index =
        load_trait_index_like_cpp(data_dir, locale, persistence, skill_line_store).await?;
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
    Ok(SkillCatalogStagesLikeCpp {
        skill_store_outcome,
        trait_tree_skill_line_index,
    })
}

pub(crate) async fn load_trait_index_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
    skill_line_store: &wow_data::SkillLineStore,
) -> Result<Arc<wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp>> {
    let trait_tree_store = wow_data::trait_tree::TraitTreeStore::load(data_dir, locale)
        .context("Failed to load TraitTree.db2")?;
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
    let skill_line_x_trait_tree_store =
        skill_line_x_trait_tree_store.apply_hotfix_overlays_like_cpp(official_links, custom_links);
    let index = Arc::new(
        wow_data::trait_tree::TraitTreeSkillLineIndexLikeCpp::from_effective_stores_like_cpp(
            &skill_line_x_trait_tree_store,
            &trait_tree_store,
            |skill_line_id| skill_line_store.contains_effective_record_like_cpp(skill_line_id),
        ),
    );
    info!(
        trait_tree_links = index.len(),
        "Loaded C++ TraitMgr SkillLineXTraitTree profession index"
    );
    Ok(index)
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
    fn catalog_preserves_skill_line_ability_trait_tree_race_class_order() {
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
                && ability_hotfix < trait_tree
                && trait_tree < race_class
                && race_class < race_class_hotfix
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
        assert!(body.contains("apply_hotfix_overlays_like_cpp"));
    }
}
