//! Composition boundary for immutable C++ World skill rules.

use anyhow::{Result, bail};
use wow_persistence::{
    FishingBaseSkillPersistenceRowLikeCpp, SkillTierPersistenceRowLikeCpp,
    SkillWorldRulesLoadOutcomeLikeCpp, SkillWorldRulesPersistencePortLikeCpp,
};

fn fishing_row_like_cpp(
    row: FishingBaseSkillPersistenceRowLikeCpp,
) -> wow_data::FishingBaseSkillRowLikeCpp {
    wow_data::FishingBaseSkillRowLikeCpp {
        area_id: row.area_id,
        skill: row.skill,
    }
}

fn skill_tier_row_like_cpp(row: SkillTierPersistenceRowLikeCpp) -> wow_data::SkillTiersRowLikeCpp {
    wow_data::SkillTiersRowLikeCpp {
        id: row.id,
        value: row.value,
    }
}

fn compose_fishing_rows_like_cpp(
    outcome: SkillWorldRulesLoadOutcomeLikeCpp<FishingBaseSkillPersistenceRowLikeCpp>,
    area_store: &wow_data::AreaTableStore,
) -> Result<wow_data::FishingBaseSkillStoreLikeCpp> {
    match outcome {
        SkillWorldRulesLoadOutcomeLikeCpp::Loaded(rows) => Ok(
            wow_data::FishingBaseSkillStoreLikeCpp::from_rows_validated_like_cpp(
                rows.into_iter().map(fishing_row_like_cpp),
                area_store,
            ),
        ),
        SkillWorldRulesLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn compose_skill_tier_rows_like_cpp(
    outcome: SkillWorldRulesLoadOutcomeLikeCpp<SkillTierPersistenceRowLikeCpp>,
) -> Result<wow_data::SkillTiersStoreLikeCpp> {
    match outcome {
        SkillWorldRulesLoadOutcomeLikeCpp::Loaded(rows) => {
            Ok(wow_data::SkillTiersStoreLikeCpp::from_rows_like_cpp(
                rows.into_iter().map(skill_tier_row_like_cpp),
            ))
        }
        SkillWorldRulesLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_fishing_base_skill_store_like_cpp(
    persistence: &dyn SkillWorldRulesPersistencePortLikeCpp,
    area_store: &wow_data::AreaTableStore,
) -> Result<wow_data::FishingBaseSkillStoreLikeCpp> {
    compose_fishing_rows_like_cpp(
        persistence.load_fishing_base_skill_rows_like_cpp().await,
        area_store,
    )
}

pub(super) async fn load_skill_tiers_store_like_cpp(
    persistence: &dyn SkillWorldRulesPersistencePortLikeCpp,
) -> Result<wow_data::SkillTiersStoreLikeCpp> {
    compose_skill_tier_rows_like_cpp(persistence.load_skill_tier_rows_like_cpp().await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_rows_preserve_signed_fishing_and_all_tier_values() {
        assert_eq!(
            fishing_row_like_cpp(FishingBaseSkillPersistenceRowLikeCpp {
                area_id: 7,
                skill: -8,
            }),
            wow_data::FishingBaseSkillRowLikeCpp {
                area_id: 7,
                skill: -8,
            }
        );
        let value = std::array::from_fn(|index| index as u32 + 1);
        assert_eq!(
            skill_tier_row_like_cpp(SkillTierPersistenceRowLikeCpp { id: 9, value }),
            wow_data::SkillTiersRowLikeCpp { id: 9, value }
        );
    }

    #[test]
    fn failures_stop_before_each_domain_store_is_published() {
        let fishing = compose_fishing_rows_like_cpp(
            SkillWorldRulesLoadOutcomeLikeCpp::Failed {
                reason: "fishing read failed".into(),
            },
            &wow_data::AreaTableStore::default(),
        );
        assert_eq!(fishing.unwrap_err().to_string(), "fishing read failed");

        let tiers = compose_skill_tier_rows_like_cpp(SkillWorldRulesLoadOutcomeLikeCpp::Failed {
            reason: "tier read failed".into(),
        });
        assert_eq!(tiers.unwrap_err().to_string(), "tier read failed");
    }

    #[test]
    fn empty_and_loaded_batches_publish_only_complete_domain_stores() {
        let areas = wow_data::AreaTableStore::from_entries([wow_data::AreaTableEntry {
            id: 7,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        }]);
        let empty = compose_fishing_rows_like_cpp(
            SkillWorldRulesLoadOutcomeLikeCpp::Loaded(Vec::new()),
            &areas,
        )
        .unwrap();
        assert!(empty.is_empty());

        let tiers =
            compose_skill_tier_rows_like_cpp(SkillWorldRulesLoadOutcomeLikeCpp::Loaded(vec![
                SkillTierPersistenceRowLikeCpp {
                    id: 9,
                    value: std::array::from_fn(|index| index as u32 + 1),
                },
            ]))
            .unwrap();
        assert_eq!(
            tiers.get_skill_tier_like_cpp(9).unwrap().value,
            std::array::from_fn(|index| index as u32 + 1)
        );
    }

    #[test]
    fn app_keeps_fishing_before_skill_tiers_with_one_adapter() {
        let source = include_str!("app.rs");
        assert_eq!(
            source
                .matches("MariaDbSkillWorldRulesPersistenceAdapterLikeCpp::new")
                .count(),
            1
        );
        let fishing = source
            .find("load_fishing_base_skill_store_like_cpp")
            .expect("fishing skill rules must remain composed");
        let tiers = source
            .find("load_skill_tiers_store_like_cpp")
            .expect("skill tiers must remain composed");
        assert!(fishing < tiers);
    }
}
