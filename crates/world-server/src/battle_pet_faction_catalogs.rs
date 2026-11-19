//! Battle-pet selection and faction-change catalogs produced during ordered startup.

use super::*;

pub(super) struct BattlePetFactionCatalogs {
    pub(super) battle_pet_selection_store:
        Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    pub(super) _faction_change_store: Arc<wow_data::FactionChangeStoreLikeCpp>,
}

pub(super) async fn load_battle_pet_faction_catalogs(
    world_db: &Arc<WorldDatabase>,
    battle_pet_species_entry_store: &wow_data::BattlePetSpeciesStore,
    gameplay_rule_catalog_persistence: &wow_database::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp,
    achievement_store: &wow_data::Db2IdStore,
    quest_store: &wow_data::quest::QuestStore,
    faction_store: &wow_data::Db2IdStore,
    spell_store: &wow_data::SpellStore,
    char_titles_store: &wow_data::Db2IdStore,
    item_stats_store: &wow_data::ItemStatsStore,
) -> Result<BattlePetFactionCatalogs> {
    // C++ loads breeds and qualities independently and tolerates either
    // unavailable table as an empty catalog.
    let battle_pet_selection_persistence =
        wow_database::MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let battle_pet_selection_store = Arc::new(
        catalogs::battle_pet_selection::load_battle_pet_selection_store_like_cpp(
            &battle_pet_selection_persistence,
            |species| {
                battle_pet_species_entry_store
                    .get(species)
                    .map(|entry| entry.flags)
            },
        )
        .await,
    );
    info!(
        "Loaded {} battle-pet breed/quality selection rows",
        battle_pet_selection_store.len_like_cpp()
    );

    let mut faction_change_outcome = catalogs::gameplay_rule::load_faction_change_store_like_cpp(
        gameplay_rule_catalog_persistence,
        |id| achievement_store.contains(id),
        |id| quest_store.get(id).is_some(),
        |id| faction_store.contains(id),
        |id| spell_store.get(i32::try_from(id).unwrap_or(-1)).is_some(),
        |id| char_titles_store.contains(id),
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
    let faction_change_store = Arc::new(faction_change_outcome.store);

    Ok(BattlePetFactionCatalogs {
        battle_pet_selection_store,
        _faction_change_store: faction_change_store,
    })
}
