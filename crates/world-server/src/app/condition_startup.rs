use anyhow::Context;
use std::sync::Arc;
use tracing::{info, warn};

pub(super) struct ConditionStartupInputs<'a> {
    pub(super) gossip_store: &'a mut wow_data::GossipStore,
    pub(super) spell_store: &'a mut wow_data::SpellStore,
    pub(super) phase_info_store: &'a mut wow_data::PhaseInfoStore,
    pub(super) graveyard_store: &'a mut wow_data::GraveyardStore,
    pub(super) item_store: &'a wow_data::ItemStore,
    pub(super) area_table_store: &'a wow_data::AreaTableStore,
    pub(super) skill_line_store: &'a wow_data::SkillLineStore,
    pub(super) map_store: &'a wow_data::MapStore,
    pub(super) phase_store: &'a wow_data::PhaseStore,
    pub(super) quest_store: &'a wow_data::quest::QuestStore,
    pub(super) area_trigger_db2_store: &'a wow_data::AreaTriggerDb2Store,
    pub(super) spawn_group_store: &'a wow_data::SpawnGroupTemplateStore,
    pub(super) creature_template_store: &'a wow_data::WorldIdStore,
    pub(super) gameobject_template_store: &'a wow_data::WorldIdStore,
    pub(super) trainer_store: &'a wow_data::WorldIdStore,
    pub(super) conversation_line_template_store: &'a wow_data::WorldIdStore,
    pub(super) area_trigger_template_store: &'a wow_data::AreaTriggerTemplateStore,
    pub(super) creature_spawn_store: &'a wow_data::WorldSpawnIdStore,
    pub(super) gameobject_spawn_store: &'a wow_data::WorldSpawnIdStore,
    pub(super) active_event_store: &'a wow_data::WorldIdStore,
    pub(super) world_state_store: &'a wow_data::WorldIdStore,
    pub(super) difficulty_store: &'a wow_data::DifficultyStore,
    pub(super) faction_store: &'a wow_data::Db2IdStore,
    pub(super) achievement_store: &'a wow_data::Db2IdStore,
    pub(super) char_titles_store: &'a wow_data::Db2IdStore,
    pub(super) battle_pet_species_store: &'a wow_data::Db2IdStore,
    pub(super) scenario_step_store: &'a wow_data::Db2IdStore,
    pub(super) scene_script_package_store: &'a wow_data::Db2IdStore,
    pub(super) player_condition_store: &'a wow_data::PlayerConditionStore,
    pub(super) loot_stores: &'a wow_loot::LootStores,
}

pub(super) async fn load_condition_startup(
    persistence: &dyn wow_persistence::ConditionDisableCatalogPersistencePortLikeCpp,
    inputs: ConditionStartupInputs<'_>,
    world_configs: &wow_config::WorldConfigSet,
) -> anyhow::Result<Arc<wow_data::ConditionEntriesByTypeStore>> {
    let ConditionStartupInputs {
        gossip_store,
        spell_store,
        phase_info_store,
        graveyard_store,
        item_store,
        area_table_store,
        skill_line_store,
        map_store,
        phase_store,
        quest_store,
        area_trigger_db2_store,
        spawn_group_store,
        creature_template_store,
        gameobject_template_store,
        trainer_store,
        conversation_line_template_store,
        area_trigger_template_store,
        creature_spawn_store,
        gameobject_spawn_store,
        active_event_store,
        world_state_store,
        difficulty_store,
        faction_store,
        achievement_store,
        char_titles_store,
        battle_pet_species_store,
        scenario_step_store,
        scene_script_package_store,
        player_condition_store,
        loot_stores,
    } = inputs;

    let mut condition_load_report =
        crate::catalogs::condition_disable::load_conditions_like_cpp(persistence, |_| 0)
            .await
            .context("Failed to load C++ conditions table")?;
    let loot_template_exists = |source_type: wow_constants::ConditionSourceType,
                                source_group: u32| {
        wow_loot::loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
            .is_some_and(|store| store.have_loot_for(source_group))
    };
    let loot_source_entry_exists =
        |source_type: wow_constants::ConditionSourceType, source_group: u32, source_entry: i32| {
            let Some(source_entry) = u32::try_from(source_entry).ok() else {
                return false;
            };
            let Some(store) =
                wow_loot::loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
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
                spell_store: Some(&*spell_store),
                area_table_store: Some(area_table_store),
                skill_line_store: Some(skill_line_store),
                map_store: Some(map_store),
                phase_store: Some(phase_store),
                quest_store: Some(quest_store),
                area_trigger_db2_store: Some(area_trigger_db2_store),
                graveyard_store: Some(&*graveyard_store),
                spawn_group_store: Some(spawn_group_store),
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
                max_skill_value: Some(crate::max_skill_value_like_cpp(world_configs)),
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
        Some(&mut *gossip_store),
        Some(&mut *spell_store),
        Some(&mut *phase_info_store),
        Some(&mut *graveyard_store),
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
    wow_world::conditions::set_condition_mgr_store_like_cpp(Arc::clone(&condition_store));
    Ok(condition_store)
}

pub(super) async fn load_world_conditions(
    persistence: &dyn wow_persistence::ConditionDisableCatalogPersistencePortLikeCpp,
    gossip_store: &mut wow_data::GossipStore,
    spell_store: &mut wow_data::SpellStore,
    phase_info_store: &mut wow_data::PhaseInfoStore,
    graveyard_store: &mut wow_data::GraveyardStore,
    inventory: &super::inventory_catalogs::InventoryBaseCatalogs,
    geography: &super::geography_startup::GeographyBase,
    skills: &super::skill_catalogs::SkillCatalogs,
    phase_store: &Arc<wow_data::PhaseStore>,
    quest_admission: &super::quest_admission_startup::QuestAdmissionCatalogs,
    spawn_references: &super::condition_reference_startup::SpawnReferences,
    trainer_store: &Arc<wow_data::WorldIdStore>,
    condition_references: &super::condition_reference_startup::ConditionReferences,
    area_trigger_template_store: &Arc<wow_data::AreaTriggerTemplateStore>,
    spawn_ids: &super::condition_reference_startup::SpawnIds,
    active_event_store: &Arc<wow_data::WorldIdStore>,
    world_state_store: &Arc<wow_data::WorldIdStore>,
    creature_runtime: &super::creature_catalog_startup::CreatureRuntimeCatalogs,
    loot: &super::loot_startup::LootCatalogs,
    world_configs: &wow_config::WorldConfigSet,
) -> anyhow::Result<Arc<wow_data::ConditionEntriesByTypeStore>> {
    let condition_store = load_condition_startup(
        persistence,
        ConditionStartupInputs {
            gossip_store: gossip_store,
            spell_store: spell_store,
            phase_info_store: phase_info_store,
            graveyard_store: graveyard_store,
            item_store: inventory.item_store.as_ref(),
            area_table_store: geography.area_table_store.as_ref(),
            skill_line_store: skills.skill_line_store.as_ref(),
            map_store: geography.map_store.as_ref(),
            phase_store: phase_store.as_ref(),
            quest_store: quest_admission.quest_store.as_ref(),
            area_trigger_db2_store: geography.area_trigger_db2_store.as_ref(),
            spawn_group_store: &spawn_references.spawn_group_store,
            creature_template_store: spawn_references.creature_template_store.as_ref(),
            gameobject_template_store: spawn_references.gameobject_template_store.as_ref(),
            trainer_store: trainer_store.as_ref(),
            conversation_line_template_store: condition_references
                .conversation_line_template_store
                .as_ref(),
            area_trigger_template_store: area_trigger_template_store.as_ref(),
            creature_spawn_store: spawn_ids.creature_spawn_store.as_ref(),
            gameobject_spawn_store: spawn_ids.gameobject_spawn_store.as_ref(),
            active_event_store: active_event_store.as_ref(),
            world_state_store: world_state_store.as_ref(),
            difficulty_store: creature_runtime.difficulty_store.as_ref(),
            faction_store: condition_references.faction_store.as_ref(),
            achievement_store: condition_references.achievement_store.as_ref(),
            char_titles_store: condition_references.char_titles_store.as_ref(),
            battle_pet_species_store: condition_references.battle_pet_species_store.as_ref(),
            scenario_step_store: condition_references.scenario_step_store.as_ref(),
            scene_script_package_store: condition_references.scene_script_package_store.as_ref(),
            player_condition_store: condition_references.player_condition_store.as_ref(),
            loot_stores: loot.loot_stores.as_ref(),
        },
        world_configs,
    )
    .await?;
    Ok(condition_store)
}
