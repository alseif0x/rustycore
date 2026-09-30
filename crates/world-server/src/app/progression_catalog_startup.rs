//! Player XP, quest rewards and reputation catalog startup composition.

use std::sync::Arc;

use anyhow::Context;
use wow_database::WorldStatements;

use crate::catalogs;

pub(super) struct PlayerXpAndQuestRewards {
    pub(super) quest_package_item_store: Arc<wow_data::progression_rewards::QuestPackageItemStore>,
    pub(super) quest_info_store: Arc<wow_data::progression_rewards::QuestInfoStore>,
    pub(super) quest_v2_store: Arc<wow_data::progression_rewards::QuestV2Store>,
    pub(super) quest_money_reward_store: Arc<wow_data::progression_rewards::QuestMoneyRewardStore>,
    pub(super) quest_xp_store: Arc<wow_data::quest_xp::QuestXpStore>,
    pub(super) dbc_path: String,
    pub(super) exploration_base_xp_store: Arc<wow_data::ExplorationBaseXpStoreLikeCpp>,
    pub(super) exploration_base_xp_persistence: wow_database::MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp,
    pub(super) player_xp_table: Arc<Vec<u32>>,
}

pub(super) async fn load_player_xp_and_quest_rewards(
    data_dir: &str,
    locale: &str,
    world_db: &Arc<wow_database::WorldDatabase>,
) -> anyhow::Result<PlayerXpAndQuestRewards> {
    // Load player_xp_for_level table
    let player_xp_table = {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_XP_FOR_LEVEL);
        let mut table = vec![0u32; 82]; // index = level, 0=unused, 81=max
        if let Ok(result) = world_db.query(&stmt).await {
            let mut r = result;
            loop {
                let lvl: u8 = r.try_read::<u8>(0).unwrap_or(0);
                let xp: u32 = r
                    .try_read::<u32>(1)
                    .or_else(|| r.try_read::<i32>(1).map(|value| value as u32))
                    .unwrap_or(0);
                if (lvl as usize) < table.len() {
                    table[lvl as usize] = xp;
                }
                if !r.next_row() {
                    break;
                }
            }
        }
        Arc::new(table)
    };
    let exploration_base_xp_persistence =
        wow_database::MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let exploration_base_xp_store = Arc::new(
        catalogs::exploration_base_xp::load_exploration_base_xp_catalog_like_cpp(
            &exploration_base_xp_persistence,
        )
        .await?,
    );

    // Load QuestXP.db2 for accurate XP rewards
    let dbc_path = format!("{}/dbc/{}", data_dir, locale);
    let quest_xp_store = Arc::new(
        wow_data::quest_xp::QuestXpStore::load(&dbc_path).unwrap_or_else(|e| {
            tracing::warn!("QuestXP.db2 not loaded ({e}), using fallback XP table");
            wow_data::quest_xp::QuestXpStore::default()
        }),
    );
    let quest_money_reward_store = Arc::new(
        wow_data::progression_rewards::QuestMoneyRewardStore::load(data_dir, locale)
            .context("Failed to load QuestMoneyReward.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_v2_store = Arc::new(
        wow_data::progression_rewards::QuestV2Store::load(data_dir, locale)
            .context("Failed to load QuestV2.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_info_store = Arc::new(
        wow_data::progression_rewards::QuestInfoStore::load(data_dir, locale)
            .context("Failed to load QuestInfo.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_package_item_store = Arc::new(
        wow_data::progression_rewards::QuestPackageItemStore::load(data_dir, locale)
            .context("Failed to load QuestPackageItem.db2 — check DataDir and DBC.Locale config")?,
    );
    Ok(PlayerXpAndQuestRewards {
        player_xp_table,
        exploration_base_xp_persistence,
        exploration_base_xp_store,
        dbc_path,
        quest_xp_store,
        quest_money_reward_store,
        quest_v2_store,
        quest_info_store,
        quest_package_item_store,
    })
}

pub(super) struct QuestReputationCatalogs {
    pub(super) reputation_spillover_template_report: wow_data::reputation::RepSpilloverTemplateLoadReportLikeCpp,
    pub(super) reputation_spillover_template_store: Arc<wow_data::reputation::RepSpilloverTemplateStoreLikeCpp>,
    pub(super) creature_onkill_reputation_report: wow_data::reputation::CreatureOnKillReputationLoadReportLikeCpp,
    pub(super) creature_onkill_reputation_store: Arc<wow_data::reputation::CreatureOnKillReputationStoreLikeCpp>,
    pub(super) reputation_reward_rate_report: wow_data::reputation::ReputationRewardRateLoadReportLikeCpp,
    pub(super) reputation_reward_rate_store: Arc<wow_data::reputation::ReputationRewardRateStoreLikeCpp>,
    pub(super) reputation_catalog_persistence: wow_database::MariaDbReputationCatalogPersistenceAdapterLikeCpp,
    pub(super) paragon_reputation_store: Arc<wow_data::progression_rewards::ParagonReputationStore>,
    pub(super) friendship_rep_reaction_store: Arc<wow_data::progression_rewards::FriendshipRepReactionStore>,
    pub(super) faction_template_store: Arc<wow_data::progression_rewards::FactionTemplateStore>,
    pub(super) progression_faction_store: Arc<wow_data::progression_rewards::FactionStore>,
    pub(super) quest_faction_reward_store: Arc<wow_data::progression_rewards::QuestFactionRewardStore>,
}

pub(super) async fn load_quest_reputation_catalogs(
    data_dir: &str,
    locale: &str,
    world_db: &Arc<wow_database::WorldDatabase>,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
) -> anyhow::Result<QuestReputationCatalogs> {
    let quest_faction_reward_store = Arc::new(
        wow_data::progression_rewards::QuestFactionRewardStore::load(data_dir, locale).context(
            "Failed to load QuestFactionReward.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let progression_faction_store = Arc::new(
        wow_data::progression_rewards::FactionStore::load(data_dir, locale).context(
            "Failed to load Faction.db2 progression store — check DataDir and DBC.Locale config",
        )?,
    );
    let faction_template_store = Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::load(data_dir, locale)
            .context("Failed to load FactionTemplate.db2 — check DataDir and DBC.Locale config")?,
    );
    let friendship_rep_reaction_store = Arc::new(
        wow_data::progression_rewards::FriendshipRepReactionStore::load(data_dir, locale)
            .context(
                "Failed to load FriendshipRepReaction.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let paragon_reputation_store = Arc::new(
        wow_data::progression_rewards::ParagonReputationStore::load(data_dir, locale).context(
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

    Ok(QuestReputationCatalogs {
        quest_faction_reward_store,
        progression_faction_store,
        faction_template_store,
        friendship_rep_reaction_store,
        paragon_reputation_store,
        reputation_catalog_persistence,
        reputation_reward_rate_store,
        reputation_reward_rate_report,
        creature_onkill_reputation_store,
        creature_onkill_reputation_report,
        reputation_spillover_template_store,
        reputation_spillover_template_report,
    })
}

pub(super) struct FactionChangeCatalog {
    pub(super) faction_change_report: wow_data::FactionChangeLoadReportLikeCpp,
    pub(super) _faction_change_store: Arc<wow_data::FactionChangeStoreLikeCpp>,
}

pub(super) async fn load_faction_changes(
    gameplay_rule_catalog_persistence: &dyn wow_persistence::GameplayRuleCatalogPersistencePortLikeCpp,
    condition_references: &super::condition_reference_startup::ConditionReferences,
    quest_admission: &super::quest_admission_startup::QuestAdmissionCatalogs,
    spell_store: &wow_data::SpellStore,
    item_stats_store: &wow_data::ItemStatsStore,
) -> anyhow::Result<FactionChangeCatalog> {
    let mut faction_change_outcome = crate::catalogs::gameplay_rule::load_faction_change_store_like_cpp(
        gameplay_rule_catalog_persistence,
        |id| condition_references.achievement_store.contains(id),
        |id| quest_admission.quest_store.get(id).is_some(),
        |id| condition_references.faction_store.contains(id),
        |id| spell_store.get(i32::try_from(id).unwrap_or(-1)).is_some(),
        |id| condition_references.char_titles_store.contains(id),
    )
    .await
    .context("Failed to load C++ faction-change mapping stores")?;
    faction_change_outcome.store = faction_change_outcome.store.with_item_templates_like_cpp(
        item_stats_store
            .sparse_templates_like_cpp()
            .map(
                |(item_id, template)| wow_data::FactionChangeItemTemplateLikeCpp {
                    item_id,
                    other_faction_item_id: template.other_faction_item_id_like_cpp(),
                    flags2: template.flags[1],
                },
            ),
        &mut faction_change_outcome.report,
    );
    for error in &faction_change_outcome.report.validation_errors {
        tracing::error!("{}", error.cpp_message_like_cpp());
    }
    info!(
        "Loaded C++ faction-change pairs: achievements {} rows/{} valid, spells {} rows/{} valid, quests {} rows/{} valid, items {} derived ({} Alliance->Horde, {} Horde->Alliance), reputations {} rows/{} valid, titles {} rows/{} valid ({} validation issues)",
        faction_change_outcome.report.achievement_rows_seen,
        faction_change_outcome.store.achievement_len(),
        faction_change_outcome.report.spell_rows_seen,
        faction_change_outcome.store.spell_len(),
        faction_change_outcome.report.quest_rows_seen,
        faction_change_outcome.store.quest_len(),
        faction_change_outcome.report.item_rows_seen,
        faction_change_outcome.store.item_alliance_to_horde_len(),
        faction_change_outcome.store.item_horde_to_alliance_len(),
        faction_change_outcome.report.reputation_rows_seen,
        faction_change_outcome.store.reputation_len(),
        faction_change_outcome.report.title_rows_seen,
        faction_change_outcome.store.title_len(),
        faction_change_outcome.report.validation_errors.len()
    );
    let _faction_change_store = Arc::new(faction_change_outcome.store);
    Ok(FactionChangeCatalog {
        _faction_change_store,
        faction_change_report: faction_change_outcome.report,
    })
}
