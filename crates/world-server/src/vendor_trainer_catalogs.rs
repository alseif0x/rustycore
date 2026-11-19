//! NPC vendor and trainer catalogs produced during ordered startup.

use super::*;

pub(super) struct VendorTrainerCatalogs {
    pub(super) _npc_vendor_store: Arc<wow_data::NpcVendorStoreLikeCpp>,
    pub(super) trainer_data_store: Arc<wow_data::TrainerStoreLikeCpp>,
}

pub(super) async fn load_vendor_trainer_catalogs(
    gameplay_rule_catalog_persistence: &wow_database::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp,
    world_db: &Arc<WorldDatabase>,
    spell_store: &wow_data::SpellStore,
    serverside_spell_store: &wow_data::ServersideSpellStoreLikeCpp,
    difficulty_store: &wow_data::DifficultyStore,
    skill_line_store: &wow_data::SkillLineStore,
    creature_template_store: &wow_data::WorldIdStore,
    gossip_store: &wow_data::GossipStore,
) -> Result<VendorTrainerCatalogs> {
    let npc_vendor_outcome =
        catalogs::gameplay_rule::load_npc_vendor_store_like_cpp(gameplay_rule_catalog_persistence)
            .await
            .context("Failed to load C++ NPC vendor item cache")?;
    for (entry, item) in &npc_vendor_outcome
        .report
        .skipped_item_maxcount_without_incrtime
    {
        tracing::error!(
            "Table `(game_event_)npc_vendor` has `maxcount` set for item {} of vendor (Entry: {}) but `incrtime`=0, ignoring",
            item,
            entry
        );
    }
    for (entry, item) in &npc_vendor_outcome
        .report
        .skipped_item_incrtime_without_maxcount
    {
        tracing::error!(
            "Table `(game_event_)npc_vendor` has `maxcount`=0 for item {} of vendor (Entry: {}) but `incrtime`<>0, ignoring",
            item,
            entry
        );
    }
    for (entry, item) in &npc_vendor_outcome.report.skipped_currency_without_maxcount {
        tracing::error!(
            "Table `(game_event_)npc_vendor` has currency item {} with missing maxcount for vendor ({}), ignoring",
            item,
            entry
        );
    }
    for (entry, item, extended_cost, vendor_type) in &npc_vendor_outcome.report.skipped_duplicates {
        tracing::error!(
            "Table `npc_vendor` has duplicate items {} (with extended cost {}, type {}) for vendor (Entry: {}), ignoring",
            item,
            extended_cost,
            vendor_type,
            entry
        );
    }
    for (entry, reference_entry) in &npc_vendor_outcome.report.skipped_reference_cycles {
        tracing::error!(
            "Table `npc_vendor` has cyclic reference vendor {} while loading vendor {}, ignoring nested reference",
            reference_entry,
            entry
        );
    }
    let npc_vendor_store = Arc::new(npc_vendor_outcome.store);
    info!(
        "Loaded {} C++ vendor items across {} NPC vendors ({} reference rows expanded)",
        npc_vendor_outcome.report.loaded_items,
        npc_vendor_store.len(),
        npc_vendor_outcome.report.reference_rows_seen
    );
    let trainer_catalog_persistence =
        wow_database::MariaDbTrainerCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let trainer_data_outcome = catalogs::trainer::load_trainer_catalog_like_cpp(
        &trainer_catalog_persistence,
        |spell_id| {
            spell_store.contains_spell_info_difficulty_none_like_cpp(
                serverside_spell_store,
                difficulty_store,
                spell_id,
            )
        },
        |skill_line_id| skill_line_store.contains_effective_record_like_cpp(skill_line_id),
        |creature_id| creature_template_store.contains(creature_id),
        |menu_id, option_id| {
            gossip_store
                .menu_items_for_id(menu_id)
                .is_some_and(|items| items.iter().any(|item| item.order_index == option_id))
        },
    )
    .await
    .context("Failed to load C++ trainer cache")?;
    for diagnostic in &trainer_data_outcome
        .report
        .diagnostics_in_load_order_like_cpp
    {
        match diagnostic {
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSpell {
                trainer_id,
                spell_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing spell (SpellId: {}) for TrainerId {}, ignoring",
                spell_id,
                trainer_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSkillLine {
                trainer_id,
                spell_id,
                skill_line_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing skill (ReqSkillLine: {}) for TrainerId {} and SpellId {}, ignoring",
                skill_line_id,
                trainer_id,
                spell_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingRequiredSpell {
                trainer_id,
                spell_id,
                required_index,
                required_spell_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing spell (ReqAbility{}: {}) for TrainerId {} and SpellId {}, ignoring",
                required_index,
                required_spell_id,
                trainer_id,
                spell_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingTrainer {
                trainer_id,
                spell_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing trainer (TrainerId: {}) for SpellId {}, ignoring",
                trainer_id,
                spell_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerLocaleMissingTrainer {
                trainer_id,
                locale,
            } => tracing::error!(
                "Table `trainer_locale` references non-existing trainer (TrainerId: {}) for locale {}, ignoring",
                trainer_id,
                locale
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingCreatureTemplate {
                creature_id,
            } => tracing::error!(
                "Table `creature_trainer` references non-existing creature template (CreatureID: {}), ignoring",
                creature_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingTrainer {
                creature_id,
                trainer_id,
                menu_id,
                option_id,
            } => tracing::error!(
                "Table `creature_trainer` references non-existing trainer (TrainerID: {}) for CreatureID {} MenuID {} OptionID {}, ignoring",
                trainer_id,
                creature_id,
                menu_id,
                option_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingGossipOption {
                creature_id,
                trainer_id,
                menu_id,
                option_id,
            } => tracing::error!(
                "Table `creature_trainer` references non-existing gossip menu option (MenuID {} OptionID {}) for CreatureID {} and TrainerID {}, ignoring",
                menu_id,
                option_id,
                creature_id,
                trainer_id
            ),
        }
    }
    let trainer_data_store = Arc::new(trainer_data_outcome.store);
    info!(
        "Loaded {} C++ Trainers with {} trainer spells and {} creature trainer bindings",
        trainer_data_store.len(),
        trainer_data_store.spell_count_like_cpp(),
        trainer_data_store.creature_trainer_count_like_cpp()
    );

    Ok(VendorTrainerCatalogs {
        _npc_vendor_store: npc_vendor_store,
        trainer_data_store,
    })
}
