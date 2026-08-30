//! Composition boundary for the effective C++ skill catalog.

use anyhow::{Result, bail};
use wow_persistence::{
    SkillCatalogHotfixLoadOutcomeLikeCpp, SkillCatalogHotfixPersistencePortLikeCpp,
    SkillLineAbilityHotfixRowLikeCpp, SkillLineHotfixRowLikeCpp,
    SkillRaceClassInfoHotfixRowLikeCpp,
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

pub(super) async fn load_skill_line_store_like_cpp(
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

pub(super) async fn load_skill_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SkillCatalogHotfixPersistencePortLikeCpp,
    removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    skill_line_store: &wow_data::SkillLineStore,
) -> Result<wow_data::SkillStoreEffectiveLoadOutcomeLikeCpp> {
    let base = wow_data::SkillStore::load_wdc4_base_like_cpp(data_dir, locale)?;
    let rows = match persistence.load_skill_relation_hotfix_rows_like_cpp().await {
        SkillCatalogHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        SkillCatalogHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    use wow_data::SkillStoreLoadSourceLikeCpp::{CustomSql, OfficialSql};
    Ok(
        wow_data::SkillStore::compose_effective_from_hotfix_overlays_like_cpp(
            base,
            rows.official_abilities
                .into_iter()
                .map(|row| skill_line_ability_source_like_cpp(row, OfficialSql)),
            rows.custom_abilities
                .into_iter()
                .map(|row| skill_line_ability_source_like_cpp(row, CustomSql)),
            rows.official_race_class_infos
                .into_iter()
                .map(|row| skill_race_class_info_source_like_cpp(row, OfficialSql)),
            rows.custom_race_class_infos
                .into_iter()
                .map(|row| skill_race_class_info_source_like_cpp(row, CustomSql)),
            removals,
            skill_line_store,
        ),
    )
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
    fn app_preserves_skill_line_then_relations_then_world_tiers_order() {
        let source = include_str!("app.rs");
        let skill_line = source
            .find("load_skill_line_store_like_cpp")
            .expect("SkillLine catalog stage must remain composed");
        let relations = source
            .find("load_skill_store_like_cpp")
            .expect("skill relation catalog stage must remain composed");
        let tiers = source
            .find("load_skill_tiers_store_like_cpp")
            .expect("independent World skill tiers stage must remain composed");
        assert!(skill_line < relations && relations < tiers);
    }
}
