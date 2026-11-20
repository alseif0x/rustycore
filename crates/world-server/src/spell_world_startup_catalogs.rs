//! Spell-click and world spell catalogs produced during ordered startup.

use super::*;

pub(super) struct SpellWorldStartupCatalogs {
    pub(super) npc_spell_click_store: Arc<wow_data::NpcSpellClickStoreLikeCpp>,
    pub(super) spell_target_position_store: Arc<wow_data::SpellTargetPositionStoreLikeCpp>,
    pub(super) spell_proc_store: Arc<wow_data::SpellProcStoreLikeCpp>,
    pub(super) spell_required_store: Arc<wow_data::SpellRequiredStoreLikeCpp>,
    pub(super) spell_group_store: Arc<wow_data::SpellGroupStoreLikeCpp>,
    pub(super) spell_group_stack_rule_store: Arc<wow_data::SpellGroupStackRuleStoreLikeCpp>,
    pub(super) spell_threat_store: Arc<wow_data::SpellThreatStoreLikeCpp>,
    pub(super) spell_linked_rejected_trigger_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_linked_store: Arc<wow_data::SpellLinkedStoreLikeCpp>,
    pub(super) spell_pet_aura_store: Arc<wow_data::SpellPetAuraStoreLikeCpp>,
    pub(super) spell_acquisition_safe_cast_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_acquisition_valid_craft_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_script_exact_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) spell_script_all_rank_root_spell_ids: Arc<BTreeSet<u32>>,
    pub(super) legacy_spell_script_spell_ids: Arc<BTreeSet<u32>>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn load_spell_world_startup_catalogs(
    gameplay_rule_catalog_persistence: &wow_database::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp,
    creature_template_lifecycle_store: &mut Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    spell_world_catalog_persistence: &wow_database::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp,
    map_store: &wow_data::MapStore,
    spell_store: &mut wow_data::SpellStore,
    serverside_spell_store: &wow_data::ServersideSpellStoreLikeCpp,
    spell_chain_store: &wow_data::SpellChainStoreLikeCpp,
    difficulty_store: &wow_data::DifficultyStore,
    spell_aura_options_store: &wow_data::SpellAuraOptionsStore,
    spell_class_options_store: &wow_data::SpellClassOptionsStore,
    spell_procs_per_minute_store: &wow_data::SpellProcsPerMinuteStore,
    chr_races_store: &wow_data::character_progression::ChrRacesStore,
    creature_display_info_store: &wow_data::CreatureDisplayInfoStore,
    spell_acquisition_startup_persistence: &wow_database::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    spell_acquisition_catalog: &wow_data::SpellAcquisitionCatalogLikeCpp,
    spell_aura_restrictions_store: &wow_data::SpellAuraRestrictionsStore,
    spell_casting_requirements_store: &wow_data::SpellCastingRequirementsStore,
    spell_equipped_items_store: &wow_data::SpellEquippedItemsStore,
    spell_area_store: &wow_data::SpellAreaStoreLikeCpp,
    item_stats_store: &wow_data::ItemStatsStore,
    data_dir: &str,
    locale: &str,
) -> Result<SpellWorldStartupCatalogs> {
    let npc_spell_click_store = Arc::new(
        catalogs::gameplay_rule::load_npc_spell_click_store_like_cpp(
            gameplay_rule_catalog_persistence,
            creature_template_lifecycle_store.as_ref(),
            &spell_store,
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
    info!(
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
    let spell_target_position_store = Arc::new(
        crate::spell::world_catalog::load_spell_target_position_like_cpp(
            spell_world_catalog_persistence,
            &spell_store,
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
        &spell_store,
        serverside_spell_store,
        spell_chain_store,
        difficulty_store,
        spell_aura_options_store,
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
        &spell_store,
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
        &spell_store,
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
            &spell_store,
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
        &spell_store,
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
        &spell_store,
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
        &spell_store,
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
        spell::acquisition_loader::load_trainer_static_authority_like_cpp(
            &data_dir,
            &locale,
            spell_acquisition_startup_persistence,
            db2_hotfix_removals,
            &spell_store,
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
    spell_store.install_effective_cast_costs_like_cpp(
        trainer_spell_static_authority.cast_cost_store.clone(),
    );
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
    Ok(SpellWorldStartupCatalogs {
        npc_spell_click_store,
        spell_target_position_store,
        spell_proc_store,
        spell_required_store,
        spell_group_store,
        spell_group_stack_rule_store,
        spell_threat_store,
        spell_linked_rejected_trigger_spell_ids,
        spell_linked_store,
        spell_pet_aura_store,
        spell_acquisition_safe_cast_spell_ids,
        spell_acquisition_valid_craft_spell_ids,
        spell_script_exact_spell_ids,
        spell_script_all_rank_root_spell_ids,
        legacy_spell_script_spell_ids,
    })
}
