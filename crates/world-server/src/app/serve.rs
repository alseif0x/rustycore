//! Listener readiness, realm publication and the existing runtime lifetime.

use std::{sync::Arc, process::ExitCode};

pub(super) async fn serve(
    session_resources: &Arc<crate::session_resources::SessionResources>,
    world_configs: &wow_config::WorldConfigSet,
    world_runtime_state: &Arc<crate::WorldRuntimeStateLikeCpp>,
    data_dir: &str,
    locale: &str,
    mmap_disabled_map_ids: std::collections::HashSet<u32>,
    geography: &super::geography_startup::GeographyBase,
    spell_info: &super::spell_info_startup::SpellInfoStartup,
    spell_cooldowns_store: &Arc<wow_data::SpellCooldownsStore>,
    jump_charge: &super::jump_charge_startup::JumpChargeCatalogs,
    condition_store: &Arc<wow_data::ConditionEntriesByTypeStore>,
    account_lookup: &Arc<dyn wow_network::world_socket::AccountLookup>,
    world_listener_policy: wow_network::WorldListenerPolicyLikeCpp,
    instances: &super::world_instance_startup::WorldInstanceManagers,
    world_spawns: &super::world_object_startup::WorldSpawnStartup,
    active_session_registry: &Arc<crate::ActiveWorldSessionRegistryLikeCpp>,
    battle_pet_account_registry: &Arc<wow_world::BattlePetAccountRegistryLikeCpp>,
    login_db: &Arc<wow_database::LoginDatabase>,
    char_db: &Arc<wow_database::CharacterDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
    realm_availability: &super::realm_startup::RealmAvailability,
    loaded_grid_creature_respawn_caches: &crate::LoadedGridCreatureRespawnCachesLikeCpp,
    area_trigger_template_store: &Arc<wow_data::AreaTriggerTemplateStore>,
    game_event_scheduler: crate::CanonicalGameEventSchedulerLikeCpp,
    player_registry: &Arc<crate::PlayerRegistry>,
    condition_references: &super::condition_reference_startup::ConditionReferences,
    world_state_mgr: &crate::SharedWorldStateMgrLikeCpp,
    group_registry: &Arc<wow_social::group::GroupRegistry>,
    item_guid_allocator_advisory_lock: &mut Option<wow_database::ItemGuidAllocatorAdvisoryLockLikeCpp>,
    game_event_quest_complete_handle: &tokio::task::JoinHandle<()>,
) -> anyhow::Result<ExitCode> {
    // Create SessionManager for ConnectTo flow
    let session_mgr = Arc::new(wow_network::session_mgr::SessionManager::new());

    let network = super::network_configuration::load(
        world_configs, data_dir, locale, mmap_disabled_map_ids,
        &session_resources.player, &session_resources.spells, &session_resources.world,
        &session_resources.progression, spell_info, spell_cooldowns_store, jump_charge,
    )?;
    let mut listeners = super::listener_startup::load(
        network.realm_addr, network.instance_addr, account_lookup, world_listener_policy, session_resources,
        &session_mgr, instances, world_spawns, active_session_registry, world_runtime_state,
        battle_pet_account_registry, network.instance_port, network.max_expansion,
        &network.mmap_runtime_config, &network.mmap_pathfinder, &network.legacy_creature_aggro_config,
    ).await?;
    let _realm_online = super::listener_startup::publish_realm_online(
        login_db.as_ref(), realm_availability.realm_id, &listeners,
    ).await?;
    let mut runtime_tasks = super::runtime_launch::load(
        world_configs, instances, world_spawns, condition_store, geography,
        loaded_grid_creature_respawn_caches, area_trigger_template_store, game_event_scheduler,
        player_registry, active_session_registry, condition_references, world_state_mgr,
        &network.mmap_runtime_config, &network.mmap_pathfinder, &network.legacy_creature_aggro_config,
        group_registry, char_db, login_db, world_db,
    )?;
    super::runtime_supervision::supervise_and_shutdown(super::runtime_supervision::RuntimeSupervisionInputs {
        realm_handle: &mut listeners.realm_handle,
        instance_handle: &mut listeners.instance_handle,
        map_update_handle: &mut runtime_tasks.map_update_handle,
        legacy_creature_runtime_handle: &mut runtime_tasks.legacy_creature_runtime_handle,
        ready_check_tick_handle: &mut runtime_tasks.ready_check_tick_handle,
        respawn_db_writer_handle: &mut runtime_tasks.respawn_db_writer_handle,
        realm_network_abort_handle: &listeners.realm_network_abort_handle,
        instance_network_abort_handle: &listeners.instance_network_abort_handle,
        world_runtime_state: world_runtime_state.as_ref(),
        active_session_registry: active_session_registry.as_ref(),
        battle_pet_account_registry: battle_pet_account_registry.as_ref(),
        respawn_db_producer_stop: runtime_tasks.respawn_db_producer_stop.as_ref(),
        respawn_db_writer_tx: runtime_tasks.respawn_db_writer_tx,
        item_guid_allocator_advisory_lock: item_guid_allocator_advisory_lock
            .take()
            .expect("the startup owner retains the item GUID advisory lock until supervision"),
        game_event_quest_complete_handle: game_event_quest_complete_handle,
        db_keepalive_handle: &runtime_tasks.db_keepalive_handle,
        realm_list_update_handle: &realm_availability.realm_list_update_handle,
        login_db: login_db.as_ref(),
        char_db: char_db.as_ref(),
        realm_id: realm_availability.realm_id,
    })
    .await
}
