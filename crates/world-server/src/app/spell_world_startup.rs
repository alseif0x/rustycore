//! Ordered SQL spell catalogs and static trainer authority at world startup.

use anyhow::Context;
use std::collections::BTreeSet;
use std::sync::Arc;
use tracing::info;

pub(super) struct SpellWorldStartupInputs<'a> {
    pub(super) data_dir: &'a str,
    pub(super) locale: &'a str,
    pub(super) spell_world_catalog_persistence: &'a dyn wow_persistence::SpellWorldCatalogPersistencePortLikeCpp,
    pub(super) spell_acquisition_startup_persistence: &'a dyn wow_persistence::SpellAcquisitionStartupPersistencePortLikeCpp,
    pub(super) db2_hotfix_removals: &'a wow_data::Db2HotfixRemovalStoreLikeCpp,
    pub(super) spell_store: &'a wow_data::SpellStore,
    pub(super) map_store: &'a wow_data::MapStore,
    pub(super) spell_chain_store: &'a wow_data::SpellChainStoreLikeCpp,
    pub(super) spell_aura_options_store: &'a wow_data::SpellAuraOptionsStore,
    pub(super) spell_misc_store: &'a wow_data::SpellMiscStore,
    pub(super) spell_class_options_store: &'a wow_data::SpellClassOptionsStore,
    pub(super) spell_procs_per_minute_store: &'a wow_data::SpellProcsPerMinuteStore,
    pub(super) chr_races_store: &'a wow_data::character_progression::ChrRacesStore,
    pub(super) creature_display_info_store: &'a wow_data::CreatureDisplayInfoStore,
    pub(super) spell_acquisition_catalog: &'a wow_data::SpellAcquisitionCatalogLikeCpp,
    pub(super) spell_aura_restrictions_store: &'a wow_data::SpellAuraRestrictionsStore,
    pub(super) spell_casting_requirements_store: &'a wow_data::SpellCastingRequirementsStore,
    pub(super) spell_equipped_items_store: &'a wow_data::SpellEquippedItemsStore,
    pub(super) spell_area_store: &'a wow_data::SpellAreaStoreLikeCpp,
    pub(super) item_stats_store: &'a wow_data::ItemStatsStore,
}

pub(super) struct SpellWorldStartupDiagnostics {
    _spell_pet_aura_errors: Vec<wow_data::SpellPetAuraLoadErrorLikeCpp>,
    _spell_linked_errors: Vec<wow_data::SpellLinkedLoadErrorLikeCpp>,
    _spell_linked_warnings: Vec<wow_data::SpellLinkedLoadWarningLikeCpp>,
    _spell_threat_errors: Vec<wow_data::SpellThreatLoadErrorLikeCpp>,
    _spell_group_stack_rule_errors: Vec<wow_data::SpellGroupStackRuleLoadErrorLikeCpp>,
    _spell_group_errors: Vec<wow_data::SpellGroupLoadErrorLikeCpp>,
    _spell_required_errors: Vec<wow_data::SpellRequiredLoadErrorLikeCpp>,
    _spell_proc_errors: Vec<wow_data::SpellProcLoadErrorLikeCpp>,
}

pub(super) struct SpellWorldStartup {
    pub(super) diagnostics: SpellWorldStartupDiagnostics,
    pub(super) spell_totem_model_outcome: wow_data::SpellTotemModelLoadOutcomeLikeCpp,
    pub(super) legacy_spell_script_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_script_all_rank_root_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_script_exact_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_acquisition_valid_craft_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_acquisition_safe_cast_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_linked_rejected_trigger_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_pet_aura_store: Arc<wow_data::SpellPetAuraStoreLikeCpp>,
    pub(super) spell_linked_store: Arc<wow_data::SpellLinkedStoreLikeCpp>,
    pub(super) spell_threat_store: Arc<wow_data::SpellThreatStoreLikeCpp>,
    pub(super) spell_group_stack_rule_store: Arc<wow_data::SpellGroupStackRuleStoreLikeCpp>,
    pub(super) spell_group_store: Arc<wow_data::SpellGroupStoreLikeCpp>,
    pub(super) spell_required_store: Arc<wow_data::SpellRequiredStoreLikeCpp>,
    pub(super) spell_proc_store: Arc<wow_data::SpellProcStoreLikeCpp>,
    pub(super) spell_target_position_store: Arc<wow_data::SpellTargetPositionStoreLikeCpp>,
}

pub(super) async fn load_spell_world_startup(
    inputs: SpellWorldStartupInputs<'_>,
) -> anyhow::Result<SpellWorldStartup> {
    let SpellWorldStartupInputs {
        data_dir,
        locale,
        spell_world_catalog_persistence,
        spell_acquisition_startup_persistence,
        db2_hotfix_removals,
        spell_store,
        map_store,
        spell_chain_store,
        spell_aura_options_store,
        spell_misc_store,
        spell_class_options_store,
        spell_procs_per_minute_store,
        chr_races_store,
        creature_display_info_store,
        spell_acquisition_catalog,
        spell_aura_restrictions_store,
        spell_casting_requirements_store,
        spell_equipped_items_store,
        spell_area_store,
        item_stats_store,
    } = inputs;

    let spell_target_position_store = Arc::new(
        crate::spell::world_catalog::load_spell_target_position_like_cpp(
            spell_world_catalog_persistence,
            spell_store,
            |map_id| map_store.get(u32::from(map_id)).is_some(),
        )
        .await
        .context("Failed to load C++ spell_target_position rows")?,
    );
    info!(
        "Loaded {} C++ spell_target_position rows ({} missing maps, {} missing spells, {} missing effects, {} zero positions, {} unsupported target rows skipped)",
        spell_target_position_store.len(),
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_missing_map,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_missing_spell,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_missing_effect,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_zero_position,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_unsupported_target
    );
    let spell_proc_outcome = crate::spell::world_catalog::load_spell_proc_like_cpp(
        spell_world_catalog_persistence,
        spell_store,
        spell_chain_store,
        spell_aura_options_store,
        spell_misc_store,
        spell_class_options_store,
        spell_procs_per_minute_store,
    )
    .await
    .context("Failed to load C++ spell_proc rows")?;
    let spell_proc_store = Arc::new(spell_proc_outcome.store);
    info!(
        "Loaded {} C++ spell_proc rows and generated {} implicit spell proc entries ({} validation issues)",
        spell_proc_outcome.loaded_row_count,
        spell_proc_outcome.generated_entry_count,
        spell_proc_outcome.errors.len()
    );
    let spell_required_outcome = crate::spell::world_catalog::load_spell_required_like_cpp(
        spell_world_catalog_persistence,
        spell_store,
        spell_chain_store,
    )
    .await
    .context("Failed to load C++ spell_required rows")?;
    let spell_required_store = Arc::new(spell_required_outcome.store);
    info!(
        "Loaded {} C++ spell_required rows ({} validation issues)",
        spell_required_outcome.loaded_row_count,
        spell_required_outcome.errors.len()
    );
    let spell_group_outcome = crate::spell::world_catalog::load_spell_group_like_cpp(
        spell_world_catalog_persistence,
        spell_store,
        spell_chain_store,
    )
    .await
    .context("Failed to load C++ spell_group rows")?;
    let spell_group_store = Arc::new(spell_group_outcome.store);
    info!(
        "Loaded {} C++ spell_group expanded definitions ({} validation issues)",
        spell_group_outcome.loaded_row_count,
        spell_group_outcome.errors.len()
    );
    let spell_group_stack_rule_outcome =
        crate::spell::world_catalog::load_spell_group_stack_rule_like_cpp(
            spell_world_catalog_persistence,
            spell_group_store.as_ref(),
            spell_store,
            spell_chain_store,
        )
        .await
        .context("Failed to load C++ spell_group_stack_rules rows")?;
    let spell_group_stack_rule_store = Arc::new(spell_group_stack_rule_outcome.store);
    info!(
        "Loaded {} C++ spell_group_stack_rules rows and parsed {} same-effect groups ({} validation issues)",
        spell_group_stack_rule_outcome.loaded_row_count,
        spell_group_stack_rule_outcome.same_effect_parsed_count,
        spell_group_stack_rule_outcome.errors.len()
    );
    let spell_threat_outcome = crate::spell::world_catalog::load_spell_threat_like_cpp(
        spell_world_catalog_persistence,
        spell_store,
    )
    .await
    .context("Failed to load C++ spell_threat rows")?;
    let spell_threat_store = Arc::new(spell_threat_outcome.store);
    info!(
        "Loaded {} C++ spell_threat rows ({} missing spells)",
        spell_threat_outcome.loaded_row_count,
        spell_threat_outcome.errors.len()
    );
    let spell_linked_outcome = crate::spell::world_catalog::load_spell_linked_like_cpp(
        spell_world_catalog_persistence,
        spell_store,
    )
    .await
    .context("Failed to load C++ spell_linked_spell rows")?;
    let spell_linked_rejected_trigger_spell_ids = Arc::new(
        spell_linked_outcome
            .errors
            .iter()
            .map(|error| error.row.spell_trigger.unsigned_abs())
            .collect::<BTreeSet<_>>(),
    );
    let spell_linked_store = Arc::new(spell_linked_outcome.store);
    info!(
        "Loaded {} C++ spell_linked_spell rows ({} validation issues, {} warnings)",
        spell_linked_outcome.loaded_row_count,
        spell_linked_outcome.errors.len(),
        spell_linked_outcome.warnings.len()
    );
    let spell_totem_model_outcome = crate::spell::world_catalog::load_spell_totem_model_like_cpp(
        spell_world_catalog_persistence,
        |spell_id| spell_store.get(spell_id as i32).is_some(),
        |race_id| chr_races_store.get(u32::from(race_id)).is_some(),
        |display_id| creature_display_info_store.get(display_id).is_some(),
    )
    .await
    .context("Failed to load C++ spell_totem_model rows")?;
    info!(
        "Loaded {} C++ spell_totem_model rows ({} validation issues)",
        spell_totem_model_outcome.loaded_row_count,
        spell_totem_model_outcome.errors.len()
    );
    let spell_pet_aura_outcome = crate::spell::world_catalog::load_spell_pet_aura_like_cpp(
        spell_world_catalog_persistence,
        spell_store,
    )
    .await
    .context("Failed to load C++ spell_pet_auras rows")?;
    let spell_pet_aura_store = Arc::new(spell_pet_aura_outcome.store);
    info!(
        "Loaded {} C++ spell_pet_auras rows ({} validation issues)",
        spell_pet_aura_outcome.loaded_row_count,
        spell_pet_aura_outcome.errors.len()
    );
    let trainer_spell_static_authority =
        crate::spell::acquisition_loader::load_trainer_static_authority_like_cpp(
            data_dir,
            locale,
            spell_acquisition_startup_persistence,
            db2_hotfix_removals,
            spell_store,
            spell_chain_store,
            spell_acquisition_catalog,
            spell_linked_store.as_ref(),
            spell_pet_aura_store.as_ref(),
            spell_aura_restrictions_store,
            spell_casting_requirements_store,
            spell_equipped_items_store,
            spell_area_store,
            |item_id| item_stats_store.sparse_template(item_id).is_some(),
        )
        .await
        .context("Failed to audit normal trainer wrapper authority")?;
    let spell_acquisition_safe_cast_spell_ids =
        Arc::new(trainer_spell_static_authority.safe_cast_spell_ids);
    let spell_acquisition_valid_craft_spell_ids =
        Arc::new(trainer_spell_static_authority.valid_craft_spell_ids);
    let spell_script_exact_spell_ids =
        Arc::new(trainer_spell_static_authority.spell_script_exact_spell_ids);
    let spell_script_all_rank_root_spell_ids =
        Arc::new(trainer_spell_static_authority.spell_script_all_rank_root_spell_ids);
    let legacy_spell_script_spell_ids =
        Arc::new(trainer_spell_static_authority.legacy_spell_script_spell_ids);
    info!(
        safe_cast_count = spell_acquisition_safe_cast_spell_ids.len(),
        valid_craft_count = spell_acquisition_valid_craft_spell_ids.len(),
        spell_script_exact_count = spell_script_exact_spell_ids.len(),
        spell_script_all_rank_count = spell_script_all_rank_root_spell_ids.len(),
        legacy_spell_script_count = legacy_spell_script_spell_ids.len(),
        "Loaded fail-closed normal trainer spell-acquisition authority"
    );
    let diagnostics = SpellWorldStartupDiagnostics {
        _spell_pet_aura_errors: spell_pet_aura_outcome.errors,
        _spell_linked_errors: spell_linked_outcome.errors,
        _spell_linked_warnings: spell_linked_outcome.warnings,
        _spell_threat_errors: spell_threat_outcome.errors,
        _spell_group_stack_rule_errors: spell_group_stack_rule_outcome.errors,
        _spell_group_errors: spell_group_outcome.errors,
        _spell_required_errors: spell_required_outcome.errors,
        _spell_proc_errors: spell_proc_outcome.errors,
    };
    Ok(SpellWorldStartup {
        spell_target_position_store,
        spell_proc_store,
        spell_required_store,
        spell_group_store,
        spell_group_stack_rule_store,
        spell_threat_store,
        spell_linked_store,
        spell_pet_aura_store,
        spell_linked_rejected_trigger_spell_ids,
        spell_acquisition_safe_cast_spell_ids,
        spell_acquisition_valid_craft_spell_ids,
        spell_script_exact_spell_ids,
        spell_script_all_rank_root_spell_ids,
        legacy_spell_script_spell_ids,
        spell_totem_model_outcome,
        diagnostics,
    })
}

pub(super) struct SpellAreaCatalog {
    pub(super) spell_area_errors: Vec<wow_data::SpellAreaLoadErrorLikeCpp>,
    pub(super) spell_area_store: Arc<wow_data::SpellAreaStoreLikeCpp>,
}

pub(super) async fn load_area_catalog(
    spell_world_catalog_persistence: &dyn wow_persistence::SpellWorldCatalogPersistencePortLikeCpp,
    spell_store: &wow_data::SpellStore,
    area_table_store: &wow_data::AreaTableStore,
    quest_store: &wow_data::quest::QuestStore,
) -> anyhow::Result<SpellAreaCatalog> {
    let spell_area_outcome = crate::spell::world_catalog::load_spell_area_like_cpp(
        spell_world_catalog_persistence,
        |spell_id| spell_store.get(spell_id as i32).is_some(),
        |area_id| area_table_store.get(area_id).is_some(),
        |quest_id| quest_store.get(quest_id).is_some(),
    )
    .await
    .context("Failed to load C++ spell_area rows")?;
    let spell_area_store = Arc::new(spell_area_outcome.store);
    info!(
        "Loaded {} C++ spell_area rows ({} validation issues; SpellInfo no-aura-cancel mutation still pending)",
        spell_area_outcome.loaded_row_count,
        spell_area_outcome.errors.len()
    );
    Ok(SpellAreaCatalog {
        spell_area_store,
        spell_area_errors: spell_area_outcome.errors,
    })
}

pub(super) async fn load_world_spell_catalogs(
    data_dir: &str,
    locale: &str,
    spell_world_catalog_persistence: &dyn wow_persistence::SpellWorldCatalogPersistencePortLikeCpp,
    spell_acquisition_startup_persistence: &dyn wow_persistence::SpellAcquisitionStartupPersistencePortLikeCpp,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    spell_store: &wow_data::SpellStore,
    map_store: &wow_data::MapStore,
    spell_acquisition: &super::spell_acquisition_startup::SpellAcquisitionCatalogs,
    spell_info: &super::spell_info_startup::SpellInfoStartup,
    spell_procs_per_minute_store: &wow_data::SpellProcsPerMinuteStore,
    chr_races_store: &wow_data::character_progression::ChrRacesStore,
    creature_display_info_store: &wow_data::CreatureDisplayInfoStore,
    spell_area_store: &wow_data::SpellAreaStoreLikeCpp,
    item_stats_store: &wow_data::ItemStatsStore,
) -> anyhow::Result<SpellWorldStartup> {
    let spell_world = load_spell_world_startup(
        SpellWorldStartupInputs {
            data_dir: data_dir,
            locale: locale,
            spell_world_catalog_persistence: spell_world_catalog_persistence,
            spell_acquisition_startup_persistence: spell_acquisition_startup_persistence,
            db2_hotfix_removals: db2_hotfix_removals,
            spell_store: spell_store,
            map_store: map_store,
            spell_chain_store: spell_acquisition.spell_chain_store.as_ref(),
            spell_aura_options_store: spell_info.spell_aura_options_store.as_ref(),
            spell_misc_store: spell_info.spell_misc_store.as_ref(),
            spell_class_options_store: spell_info.spell_class_options_store.as_ref(),
            spell_procs_per_minute_store: spell_procs_per_minute_store,
            chr_races_store: chr_races_store,
            creature_display_info_store: creature_display_info_store,
            spell_acquisition_catalog: spell_acquisition.spell_acquisition_catalog.as_ref(),
            spell_aura_restrictions_store: spell_info.spell_aura_restrictions_store.as_ref(),
            spell_casting_requirements_store: spell_info.spell_casting_requirements_store.as_ref(),
            spell_equipped_items_store: spell_info.spell_equipped_items_store.as_ref(),
            spell_area_store: spell_area_store,
            item_stats_store: item_stats_store,
        },
    )
    .await?;
    Ok(spell_world)
}
