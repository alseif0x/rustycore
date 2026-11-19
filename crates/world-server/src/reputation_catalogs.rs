//! Faction and reputation catalogs produced during ordered startup.

use super::*;

pub(super) struct ReputationCatalogs {
    pub(super) quest_faction_reward_store:
        Arc<wow_data::progression_rewards::QuestFactionRewardStore>,
    pub(super) progression_faction_store: Arc<wow_data::progression_rewards::FactionStore>,
    pub(super) faction_template_store: Arc<wow_data::progression_rewards::FactionTemplateStore>,
    pub(super) friendship_rep_reaction_store:
        Arc<wow_data::progression_rewards::FriendshipRepReactionStore>,
    pub(super) paragon_reputation_store: Arc<wow_data::progression_rewards::ParagonReputationStore>,
    pub(super) reputation_reward_rate_store:
        Arc<wow_data::reputation::ReputationRewardRateStoreLikeCpp>,
    pub(super) creature_onkill_reputation_store:
        Arc<wow_data::reputation::CreatureOnKillReputationStoreLikeCpp>,
    pub(super) reputation_spillover_template_store:
        Arc<wow_data::reputation::RepSpilloverTemplateStoreLikeCpp>,
}

pub(super) async fn load_reputation_catalogs(
    data_dir: &str,
    locale: &str,
    world_db: &Arc<WorldDatabase>,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
) -> Result<ReputationCatalogs> {
    let quest_faction_reward_store = Arc::new(
        wow_data::progression_rewards::QuestFactionRewardStore::load(&data_dir, &locale).context(
            "Failed to load QuestFactionReward.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let progression_faction_store = Arc::new(
        wow_data::progression_rewards::FactionStore::load(&data_dir, &locale).context(
            "Failed to load Faction.db2 progression store — check DataDir and DBC.Locale config",
        )?,
    );
    let faction_template_store = Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::load(&data_dir, &locale)
            .context("Failed to load FactionTemplate.db2 — check DataDir and DBC.Locale config")?,
    );
    let friendship_rep_reaction_store = Arc::new(
        wow_data::progression_rewards::FriendshipRepReactionStore::load(&data_dir, &locale)
            .context(
                "Failed to load FriendshipRepReaction.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let paragon_reputation_store = Arc::new(
        wow_data::progression_rewards::ParagonReputationStore::load(&data_dir, &locale).context(
            "Failed to load ParagonReputation.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let reputation_catalog_persistence =
        wow_database::MariaDbReputationCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let (reputation_reward_rate_store, reputation_reward_rate_report) =
        catalogs::reputation::load_reward_rate_store_like_cpp(
            &reputation_catalog_persistence,
            &progression_faction_store,
        )
        .await
        .context("Failed to load reputation_reward_rate")?;
    let reputation_reward_rate_store = Arc::new(reputation_reward_rate_store);
    tracing::info!(
        loaded = reputation_reward_rate_store.len(),
        skipped = reputation_reward_rate_report.skipped.len(),
        "Loaded reputation_reward_rate like C++"
    );
    let (creature_onkill_reputation_store, creature_onkill_reputation_report) =
        catalogs::reputation::load_creature_onkill_store_like_cpp(
            &reputation_catalog_persistence,
            creature_template_lifecycle_store,
            &progression_faction_store,
        )
        .await
        .context("Failed to load creature_onkill_reputation")?;
    let creature_onkill_reputation_store = Arc::new(creature_onkill_reputation_store);
    tracing::info!(
        loaded = creature_onkill_reputation_store.len(),
        skipped = creature_onkill_reputation_report.skipped.len(),
        "Loaded creature_onkill_reputation like C++"
    );
    let (reputation_spillover_template_store, reputation_spillover_template_report) =
        catalogs::reputation::load_spillover_template_store_like_cpp(
            &reputation_catalog_persistence,
            &progression_faction_store,
        )
        .await
        .context("Failed to load reputation_spillover_template")?;
    let reputation_spillover_template_store = Arc::new(reputation_spillover_template_store);
    tracing::info!(
        loaded = reputation_spillover_template_store.len(),
        skipped = reputation_spillover_template_report.skipped.len(),
        "Loaded reputation_spillover_template like C++"
    );

    Ok(ReputationCatalogs {
        quest_faction_reward_store,
        progression_faction_store,
        faction_template_store,
        friendship_rep_reaction_store,
        paragon_reputation_store,
        reputation_reward_rate_store,
        creature_onkill_reputation_store,
        reputation_spillover_template_store,
    })
}
