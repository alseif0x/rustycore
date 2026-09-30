//! NPC vendor and trainer startup catalog composition.

use anyhow::{Context, Result};
use std::sync::Arc;

pub(super) async fn load_npc_vendor(
    persistence: &dyn wow_persistence::GameplayRuleCatalogPersistencePortLikeCpp,
) -> Result<Arc<wow_data::NpcVendorStoreLikeCpp>> {
    let npc_vendor_outcome =
        crate::catalogs::gameplay_rule::load_npc_vendor_store_like_cpp(persistence)
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
    tracing::info!(
        "Loaded {} C++ vendor items across {} NPC vendors ({} reference rows expanded)",
        npc_vendor_outcome.report.loaded_items,
        npc_vendor_store.len(),
        npc_vendor_outcome.report.reference_rows_seen
    );
    Ok(npc_vendor_store)
}

pub(super) async fn load_trainer<
    SpellExists,
    SkillLineExists,
    CreatureTemplateExists,
    GossipOptionExists,
>(
    persistence: &dyn wow_persistence::TrainerCatalogPersistencePortLikeCpp,
    spell_exists: SpellExists,
    skill_line_exists: SkillLineExists,
    creature_template_exists: CreatureTemplateExists,
    gossip_option_exists: GossipOptionExists,
) -> Result<Arc<wow_data::TrainerStoreLikeCpp>>
where
    SpellExists: FnMut(u32) -> bool,
    SkillLineExists: FnMut(u32) -> bool,
    CreatureTemplateExists: FnMut(u32) -> bool,
    GossipOptionExists: FnMut(u32, u32) -> bool,
{
    let trainer_data_outcome = crate::catalogs::trainer::load_trainer_catalog_like_cpp(
        persistence,
        spell_exists,
        skill_line_exists,
        creature_template_exists,
        gossip_option_exists,
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
    tracing::info!(
        "Loaded {} C++ Trainers with {} trainer spells and {} creature trainer bindings",
        trainer_data_store.len(),
        trainer_data_store.spell_count_like_cpp(),
        trainer_data_store.creature_trainer_count_like_cpp()
    );
    Ok(trainer_data_store)
}

pub(super) struct SpellClickCatalog {
    pub(super) spellclick_template_flags_removed: usize,
    pub(super) spellclick_templates_without_data: Vec<u32>,
    pub(super) npc_spell_click_store: Arc<wow_data::NpcSpellClickStoreLikeCpp>,
}

pub(super) async fn load_spell_click(
    gameplay_rule_catalog_persistence: &dyn wow_persistence::GameplayRuleCatalogPersistencePortLikeCpp,
    creature_template_lifecycle_store: &mut Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    spell_store: &wow_data::SpellStore,
) -> anyhow::Result<SpellClickCatalog> {
    let npc_spell_click_store = Arc::new(
        crate::catalogs::gameplay_rule::load_npc_spell_click_store_like_cpp(
            gameplay_rule_catalog_persistence,
            creature_template_lifecycle_store.as_ref(),
            spell_store,
        )
        .await
        .context("Failed to load C++ npc_spellclick_spells rows")?,
    );
    let spellclick_templates_without_data = npc_spell_click_store
        .templates_with_spellclick_flag_but_no_data_like_cpp(
            creature_template_lifecycle_store
                .entries_like_cpp()
                .map(|template| (template.entry, template.npc_flags)),
        );
    let spellclick_template_flags_removed = Arc::make_mut(creature_template_lifecycle_store)
        .remove_npc_flag_for_entries_like_cpp(
            spellclick_templates_without_data.iter().copied(),
            wow_data::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        );
    tracing::info!(
        "Loaded {} C++ npc_spellclick_spells rows ({} missing creature templates, {} missing spells, {} invalid user types logged-but-loaded like C++, {} templates with UNIT_NPC_FLAG_SPELLCLICK but no data, {} flags removed)",
        npc_spell_click_store.len(),
        npc_spell_click_store
            .load_report_like_cpp()
            .skipped_missing_creature_template,
        npc_spell_click_store
            .load_report_like_cpp()
            .skipped_missing_spell,
        npc_spell_click_store
            .load_report_like_cpp()
            .invalid_user_type_logged_but_loaded_like_cpp,
        spellclick_templates_without_data.len(),
        spellclick_template_flags_removed
    );
    Ok(SpellClickCatalog {
        npc_spell_click_store,
        spellclick_templates_without_data,
        spellclick_template_flags_removed,
    })
}

pub(super) struct NpcServiceCatalogs {
    pub(super) trainer_data_store: Arc<wow_data::TrainerStoreLikeCpp>,
    pub(super) trainer_catalog_persistence: wow_database::MariaDbTrainerCatalogPersistenceAdapterLikeCpp,
    pub(super) npc_vendor_store: Arc<wow_data::NpcVendorStoreLikeCpp>,
}

pub(super) async fn load_services(
    world_db: &Arc<wow_database::WorldDatabase>,
    gameplay_rule_catalog_persistence: &dyn wow_persistence::GameplayRuleCatalogPersistencePortLikeCpp,
    spell_store: &wow_data::SpellStore,
    serverside_spell_store: &wow_data::ServersideSpellStoreLikeCpp,
    difficulty_store: &wow_data::DifficultyStore,
    skill_line_store: &wow_data::SkillLineStore,
    creature_template_store: &wow_data::WorldIdStore,
    gossip_store: &wow_data::GossipStore,
) -> anyhow::Result<NpcServiceCatalogs> {
    let npc_vendor_store =
        load_npc_vendor(gameplay_rule_catalog_persistence)
            .await?;
    let trainer_catalog_persistence =
        wow_database::MariaDbTrainerCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let trainer_data_store = load_trainer(
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
    .await?;
    Ok(NpcServiceCatalogs {
        npc_vendor_store,
        trainer_catalog_persistence,
        trainer_data_store,
    })
}
