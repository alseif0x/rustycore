//! ConditionMgr loading and attachment catalogs produced during ordered startup.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn load_condition_store(
    condition_disable_catalog_persistence: &wow_database::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp,
    loot_stores: &LootStores,
    item_store: &wow_data::ItemStore,
    spell_store: &mut wow_data::SpellStore,
    area_table_store: &wow_data::AreaTableStore,
    skill_line_store: &wow_data::SkillLineStore,
    map_store: &wow_data::MapStore,
    phase_store: &wow_data::PhaseStore,
    quest_store: &wow_data::quest::QuestStore,
    area_trigger_db2_store: &wow_data::AreaTriggerDb2Store,
    graveyard_store: &mut wow_data::GraveyardStore,
    spawn_group_store: &wow_data::SpawnGroupTemplateStore,
    creature_template_store: &wow_data::WorldIdStore,
    gameobject_template_store: &wow_data::WorldIdStore,
    trainer_store: &wow_data::WorldIdStore,
    conversation_line_template_store: &wow_data::WorldIdStore,
    area_trigger_template_store: &wow_data::AreaTriggerTemplateStore,
    creature_spawn_store: &wow_data::WorldSpawnIdStore,
    gameobject_spawn_store: &wow_data::WorldSpawnIdStore,
    active_event_store: &wow_data::WorldIdStore,
    world_state_store: &wow_data::WorldIdStore,
    difficulty_store: &wow_data::DifficultyStore,
    faction_store: &wow_data::Db2IdStore,
    achievement_store: &wow_data::Db2IdStore,
    char_titles_store: &wow_data::Db2IdStore,
    battle_pet_species_store: &wow_data::Db2IdStore,
    scenario_step_store: &wow_data::Db2IdStore,
    scene_script_package_store: &wow_data::Db2IdStore,
    player_condition_store: &wow_data::PlayerConditionStore,
    max_skill_value: u32,
    gossip_store: &mut wow_data::GossipStore,
    phase_info_store: &mut wow_data::PhaseInfoStore,
) -> Result<Arc<wow_data::conditions::ConditionEntriesByTypeStore>> {
    let mut condition_load_report = catalogs::condition_disable::load_conditions_like_cpp(
        condition_disable_catalog_persistence,
        |_| 0,
    )
    .await
    .context("Failed to load C++ conditions table")?;
    let loot_template_exists = |source_type: wow_constants::ConditionSourceType,
                                source_group: u32| {
        loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
            .is_some_and(|store| store.have_loot_for(source_group))
    };
    let loot_source_entry_exists = |source_type: wow_constants::ConditionSourceType,
                                    source_group: u32,
                                    source_entry: i32| {
        let Some(source_entry) = u32::try_from(source_entry).ok() else {
            return false;
        };
        let Some(store) = loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
        else {
            return false;
        };
        let Some(template) = store.get_loot_for(source_group) else {
            return false;
        };

        item_store.get(source_entry).is_some() || template.is_reference_like_cpp(source_entry)
    };
    let externally_skipped_conditions =
        wow_data::conditions::apply_external_condition_validation_like_cpp(
            &mut condition_load_report,
            wow_data::conditions::ConditionExternalValidationStoresLikeCpp {
                item_store: Some(item_store),
                spell_store: Some(&spell_store),
                area_table_store: Some(area_table_store),
                skill_line_store: Some(skill_line_store),
                map_store: Some(map_store),
                phase_store: Some(phase_store),
                quest_store: Some(quest_store),
                area_trigger_db2_store: Some(area_trigger_db2_store),
                graveyard_store: Some(&graveyard_store),
                spawn_group_store: Some(&spawn_group_store),
                creature_template_store: Some(creature_template_store),
                gameobject_template_store: Some(gameobject_template_store),
                trainer_store: Some(trainer_store),
                conversation_line_template_store: Some(conversation_line_template_store),
                area_trigger_template_store: Some(area_trigger_template_store),
                creature_spawn_store: Some(creature_spawn_store),
                gameobject_spawn_store: Some(gameobject_spawn_store),
                active_event_store: Some(active_event_store),
                world_state_store: Some(world_state_store),
                difficulty_store: Some(difficulty_store),
                faction_store: Some(faction_store),
                achievement_store: Some(achievement_store),
                char_titles_store: Some(char_titles_store),
                battle_pet_species_store: Some(battle_pet_species_store),
                scenario_step_store: Some(scenario_step_store),
                scene_script_package_store: Some(scene_script_package_store),
                player_condition_store: Some(player_condition_store),
                max_skill_value: Some(max_skill_value),
                loot_template_exists: Some(&loot_template_exists),
                loot_source_entry_exists: Some(&loot_source_entry_exists),
            },
        );
    for skipped in &condition_load_report.skipped {
        warn!(
            "Condition row skipped during C++ load-shape parsing: {:?}: {:?}",
            skipped.row, skipped.reason
        );
    }
    for skipped in &externally_skipped_conditions {
        warn!(
            "Condition row skipped during C++ external validation: {:?}: {:?}",
            skipped.condition, skipped.reason
        );
    }
    for warning in &condition_load_report.warnings {
        warn!("Condition load warning: {warning:?}");
    }
    let condition_store = Arc::new(condition_load_report.into_store_like_cpp());
    let condition_attachment_report = wow_data::attach_loaded_conditions_like_cpp(
        condition_store.as_ref(),
        Some(gossip_store),
        Some(spell_store),
        Some(phase_info_store),
        Some(graveyard_store),
    );
    for missing in &condition_attachment_report.gossip_menus.missing_menus {
        warn!(
            "ConditionMgr gossip attachment warning: GossipMenu {} not found for condition id {:?}",
            missing.source_group, missing
        );
    }
    for missing in &condition_attachment_report
        .gossip_menu_items
        .missing_menu_items
    {
        warn!(
            "ConditionMgr gossip attachment warning: GossipMenuId {} Item {} not found for condition id {:?}",
            missing.source_group, missing.source_entry, missing
        );
    }
    info!(
        "Loaded C++ ConditionMgr store: {} buckets, {} externally skipped conditions, {} spell-click aura spell ids, {} spell implicit target condition rows attached ({} deferred), {} gossip menu condition rows attached ({} missing menus), {} gossip menu option condition rows attached ({} missing items), {} phase condition rows attached, {} graveyard condition rows attached",
        condition_store.bucket_count(),
        externally_skipped_conditions.len(),
        condition_attachment_report.spell_click_aura_spell_ids.len(),
        condition_attachment_report.spell_implicit_target_condition_count,
        condition_attachment_report.deferred_spell_implicit_target_condition_count,
        condition_attachment_report
            .gossip_menus
            .attached_condition_count,
        condition_attachment_report.gossip_menus.missing_menus.len(),
        condition_attachment_report
            .gossip_menu_items
            .attached_condition_count,
        condition_attachment_report
            .gossip_menu_items
            .missing_menu_items
            .len(),
        condition_attachment_report.phases.attached_condition_count,
        condition_attachment_report
            .graveyards
            .attached_condition_count
    );
    Ok(condition_store)
}
