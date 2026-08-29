//! Composition boundary between SQLx-free reputation rows and `wow-data` stores.

use anyhow::{Result, bail};
use tracing::{info, warn};
use wow_persistence::{
    CreatureOnKillReputationPersistenceRowLikeCpp, ReputationCatalogLoadOutcomeLikeCpp,
    ReputationCatalogPersistencePortLikeCpp, ReputationRewardRatePersistenceRowLikeCpp,
    ReputationSpilloverTemplatePersistenceRowLikeCpp,
};

fn loaded_rows_like_cpp<T>(outcome: ReputationCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        ReputationCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        ReputationCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

fn reward_rate_row_like_cpp(
    row: ReputationRewardRatePersistenceRowLikeCpp,
) -> wow_data::reputation::ReputationRewardRateRowLikeCpp {
    wow_data::reputation::ReputationRewardRateRowLikeCpp {
        faction_id: row.faction_id,
        rates: wow_data::reputation::ReputationRewardRateEntryLikeCpp {
            quest_rate: row.quest_rate,
            quest_daily_rate: row.quest_daily_rate,
            quest_weekly_rate: row.quest_weekly_rate,
            quest_monthly_rate: row.quest_monthly_rate,
            quest_repeatable_rate: row.quest_repeatable_rate,
            creature_rate: row.creature_rate,
            spell_rate: row.spell_rate,
        },
    }
}

fn creature_onkill_row_like_cpp(
    row: CreatureOnKillReputationPersistenceRowLikeCpp,
) -> wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
    wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
        creature_id: row.creature_id,
        entry: wow_data::reputation::CreatureOnKillReputationEntryLikeCpp {
            rep_faction_1: row.rep_faction_1,
            rep_faction_2: row.rep_faction_2,
            reputation_max_cap_1: row.reputation_max_cap_1,
            rep_value_1: row.rep_value_1,
            reputation_max_cap_2: row.reputation_max_cap_2,
            rep_value_2: row.rep_value_2,
            is_team_award_1: row.is_team_award_1,
            is_team_award_2: row.is_team_award_2,
            team_dependent: row.team_dependent,
        },
    }
}

fn spillover_template_row_like_cpp(
    row: ReputationSpilloverTemplatePersistenceRowLikeCpp,
) -> wow_data::reputation::RepSpilloverTemplateRowLikeCpp {
    wow_data::reputation::RepSpilloverTemplateRowLikeCpp {
        faction_id: row.faction_id,
        template: wow_data::reputation::RepSpilloverTemplateLikeCpp {
            faction: row.faction,
            faction_rate: row.faction_rate,
            faction_rank: row.faction_rank,
        },
    }
}

pub(super) async fn load_reward_rate_store_like_cpp(
    persistence: &dyn ReputationCatalogPersistencePortLikeCpp,
    faction_store: &wow_data::progression_rewards::FactionStore,
) -> Result<(
    wow_data::reputation::ReputationRewardRateStoreLikeCpp,
    wow_data::reputation::ReputationRewardRateLoadReportLikeCpp,
)> {
    let rows = loaded_rows_like_cpp(persistence.load_reward_rate_rows_like_cpp().await)?;
    if rows.is_empty() {
        info!("Loaded `reputation_reward_rate`, table is empty");
        return Ok(Default::default());
    }

    let (store, report) =
        wow_data::reputation::ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            rows.into_iter().map(reward_rate_row_like_cpp),
            faction_store,
        );
    for skipped in &report.skipped {
        warn!(
            faction_id = skipped.faction_id,
            reason = ?skipped.reason,
            "Skipping reputation_reward_rate row like C++"
        );
    }
    info!(
        "Loaded {} reputation_reward_rate rows ({} skipped)",
        report.loaded,
        report.skipped.len()
    );
    Ok((store, report))
}

pub(super) async fn load_creature_onkill_store_like_cpp(
    persistence: &dyn ReputationCatalogPersistencePortLikeCpp,
    creature_template_store: &wow_data::creature_template::CreatureTemplateLifecycleStoreLikeCpp,
    faction_store: &wow_data::progression_rewards::FactionStore,
) -> Result<(
    wow_data::reputation::CreatureOnKillReputationStoreLikeCpp,
    wow_data::reputation::CreatureOnKillReputationLoadReportLikeCpp,
)> {
    let rows = loaded_rows_like_cpp(persistence.load_creature_onkill_rows_like_cpp().await)?;
    if rows.is_empty() {
        info!(
            "Loaded 0 creature award reputation definitions. DB table `creature_onkill_reputation` is empty."
        );
        return Ok(Default::default());
    }

    let (store, report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            rows.into_iter().map(creature_onkill_row_like_cpp),
            creature_template_store,
            faction_store,
        );
    for skipped in &report.skipped {
        warn!(
            creature_id = skipped.creature_id,
            reason = ?skipped.reason,
            "Skipping creature_onkill_reputation row like C++"
        );
    }
    info!(
        "Loaded {} creature award reputation definitions ({} skipped)",
        report.loaded,
        report.skipped.len()
    );
    Ok((store, report))
}

pub(super) async fn load_spillover_template_store_like_cpp(
    persistence: &dyn ReputationCatalogPersistencePortLikeCpp,
    faction_store: &wow_data::progression_rewards::FactionStore,
) -> Result<(
    wow_data::reputation::RepSpilloverTemplateStoreLikeCpp,
    wow_data::reputation::RepSpilloverTemplateLoadReportLikeCpp,
)> {
    let rows = loaded_rows_like_cpp(persistence.load_spillover_template_rows_like_cpp().await)?;
    if rows.is_empty() {
        info!("Loaded `reputation_spillover_template`, table is empty");
        return Ok(Default::default());
    }

    let (store, report) =
        wow_data::reputation::RepSpilloverTemplateStoreLikeCpp::from_rows_like_cpp(
            rows.into_iter().map(spillover_template_row_like_cpp),
            faction_store,
        );
    for skipped in &report.skipped {
        warn!(
            faction_id = skipped.faction_id,
            reason = ?skipped.reason,
            "Skipping reputation_spillover_template row like C++"
        );
    }
    info!(
        "Loaded {} reputation_spillover_template rows ({} skipped)",
        report.loaded,
        report.skipped.len()
    );
    Ok((store, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    #[derive(Clone)]
    struct FakeReputationCatalogPersistenceLikeCpp {
        reward_rates:
            ReputationCatalogLoadOutcomeLikeCpp<ReputationRewardRatePersistenceRowLikeCpp>,
        creature_onkill:
            ReputationCatalogLoadOutcomeLikeCpp<CreatureOnKillReputationPersistenceRowLikeCpp>,
        spillover:
            ReputationCatalogLoadOutcomeLikeCpp<ReputationSpilloverTemplatePersistenceRowLikeCpp>,
    }

    impl ReputationCatalogPersistencePortLikeCpp for FakeReputationCatalogPersistenceLikeCpp {
        fn load_reward_rate_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            ReputationCatalogLoadOutcomeLikeCpp<ReputationRewardRatePersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.reward_rates.clone() })
        }

        fn load_creature_onkill_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            ReputationCatalogLoadOutcomeLikeCpp<CreatureOnKillReputationPersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.creature_onkill.clone() })
        }

        fn load_spillover_template_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            ReputationCatalogLoadOutcomeLikeCpp<ReputationSpilloverTemplatePersistenceRowLikeCpp>,
        > {
            Box::pin(async { self.spillover.clone() })
        }
    }

    fn persistence_like_cpp() -> FakeReputationCatalogPersistenceLikeCpp {
        FakeReputationCatalogPersistenceLikeCpp {
            reward_rates: ReputationCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            creature_onkill: ReputationCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
            spillover: ReputationCatalogLoadOutcomeLikeCpp::Loaded(Vec::new()),
        }
    }

    #[tokio::test]
    async fn reward_rows_cross_the_port_then_keep_catalog_validation_like_cpp() {
        let factions = wow_data::progression_rewards::FactionStore::from_entries([
            wow_data::progression_rewards::FactionEntry::for_test_like_cpp(7, 0),
        ]);
        let mut persistence = persistence_like_cpp();
        persistence.reward_rates = ReputationCatalogLoadOutcomeLikeCpp::Loaded(vec![
            ReputationRewardRatePersistenceRowLikeCpp {
                faction_id: 7,
                quest_rate: 1.0,
                quest_daily_rate: 2.0,
                quest_weekly_rate: 3.0,
                quest_monthly_rate: 4.0,
                quest_repeatable_rate: 5.0,
                creature_rate: 6.0,
                spell_rate: 7.0,
            },
            ReputationRewardRatePersistenceRowLikeCpp {
                faction_id: 999,
                quest_rate: 1.0,
                quest_daily_rate: 1.0,
                quest_weekly_rate: 1.0,
                quest_monthly_rate: 1.0,
                quest_repeatable_rate: 1.0,
                creature_rate: 1.0,
                spell_rate: 1.0,
            },
        ]);

        let (store, report) = load_reward_rate_store_like_cpp(&persistence, &factions)
            .await
            .unwrap();
        assert_eq!(report.loaded, 1);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(store.get(7).unwrap().spell_rate, 7.0);
        assert!(store.get(999).is_none());
    }

    #[tokio::test]
    async fn empty_and_failed_loads_remain_distinct_before_catalog_publication() {
        let factions = wow_data::progression_rewards::FactionStore::from_entries([]);
        let persistence = persistence_like_cpp();
        let (store, report) = load_reward_rate_store_like_cpp(&persistence, &factions)
            .await
            .unwrap();
        assert!(store.is_empty());
        assert_eq!(report.loaded, 0);
        assert!(report.skipped.is_empty());

        let mut failed = persistence;
        failed.reward_rates = ReputationCatalogLoadOutcomeLikeCpp::Failed {
            reason: "reward query failed".to_string(),
        };
        let error = load_reward_rate_store_like_cpp(&failed, &factions)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "reward query failed");
    }

    #[test]
    fn every_typed_row_maps_without_field_or_slot_reordering() {
        let creature =
            creature_onkill_row_like_cpp(CreatureOnKillReputationPersistenceRowLikeCpp {
                creature_id: 10,
                rep_faction_1: u32::MAX,
                rep_faction_2: 20,
                is_team_award_1: true,
                reputation_max_cap_1: 4,
                rep_value_1: -30,
                is_team_award_2: false,
                reputation_max_cap_2: 5,
                rep_value_2: 40,
                team_dependent: true,
            });
        assert_eq!(creature.creature_id, 10);
        assert_eq!(creature.entry.rep_faction_1, u32::MAX);
        assert_eq!(creature.entry.rep_value_1, -30);
        assert_eq!(creature.entry.reputation_max_cap_2, 5);
        assert!(creature.entry.team_dependent);

        let spillover =
            spillover_template_row_like_cpp(ReputationSpilloverTemplatePersistenceRowLikeCpp {
                faction_id: 50,
                faction: [1, 2, 3, 4, 5],
                faction_rate: [0.1, 0.2, 0.3, 0.4, 0.5],
                faction_rank: [3, 4, 5, 6, 7],
            });
        assert_eq!(spillover.faction_id, 50);
        assert_eq!(spillover.template.faction, [1, 2, 3, 4, 5]);
        assert_eq!(spillover.template.faction_rank, [3, 4, 5, 6, 7]);
    }
}
