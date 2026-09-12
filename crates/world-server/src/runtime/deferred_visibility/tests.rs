//! Production composition: the respawn cadence must not swallow map notifiers.

use super::*;
use crate::runtime::map::{
    CanonicalRespawnConditionSchedulerLikeCpp, LoadedGridCreatureRespawnCachesLikeCpp,
};
use crate::runtime::map_tick::canonical_map_update_tick_set_inactive_like_cpp;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use wow_core::{ObjectGuid, Position};
use wow_entities::{ObjectNotifyFlags, Player};
use wow_world::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use wow_world::session::mailbox::{DurableCreatureRuntimeCommandsLikeCpp, SessionCommand};

fn caches() -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Default::default(),
        sparring_store: Default::default(),
        difficulty_store: Default::default(),
        base_stats_store: Default::default(),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: Default::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Default::default(),
        creature_addon_store: Default::default(),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Default::default(),
        gameobject_override_store: Default::default(),
    }
}

#[test]
fn deferred_visibility_routes_effective_tick_before_respawn_timer_and_survives_full_queue() {
    let position = Position::xyz(10.0, 20.0, 30.0);
    let key = wow_map::MapKey::new(1, 0);
    let guid = ObjectGuid::create_player(1, 588_010);
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(key.map_id, key.instance_id);
    let mut player = Box::new(Player::new(Some(588_010), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, key, position)
        .unwrap();
    manager
        .with_player_mut_like_cpp(handle, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
        })
        .unwrap();
    let grid = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .get_ngrid_mut(wow_map::compute_grid_coord(position.x, position.y))
        .unwrap();
    grid.set_state(wow_map::GridStateKind::Active);
    *grid.info_mut().relocation_timer_mut() =
        wow_map::PeriodicTimer::new(wow_map::DEFAULT_VISIBILITY_NOTIFY_PERIOD, 0);

    let canonical = Arc::new(Mutex::new(manager));
    let registry = PlayerRegistry::new();
    assert!(registry.bind_canonical_map_manager(canonical.clone()));
    let (send_tx, _send_rx) = flume::bounded(8);
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .unwrap();
    let durable = Arc::new(Mutex::new(DurableCreatureRuntimeCommandsLikeCpp::default()));
    registry.register_or_replace(
        guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new("Visibility", 1, 0, 1, 1, 0, 2),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 1,
                instance_id: 0,
                position,
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: vec![],
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx,
            session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
            durable_creature_runtime_commands_like_cpp: durable.clone(),
            client_visible_guids_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );
    let metadata = crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
        wow_map::SpawnStore::new(),
        BTreeMap::new(),
    );
    let conditions = wow_data::ConditionEntriesByTypeStore::from_conditions_like_cpp([]);
    let maps = wow_data::MapStore::from_entries([]);
    let caches = caches();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(60_000);
    let tick = |diff, scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp| {
        canonical_map_update_tick_set_inactive_like_cpp(
            &mut canonical.lock().unwrap(),
            None,
            diff,
            scheduler,
            &metadata,
            &conditions,
            &maps,
            &caches,
        )
    };
    assert!(tick(9, &mut scheduler).is_none());
    assert_eq!(scheduler.timer_ms(), 60_000);
    let summary = tick(1, &mut scheduler).expect("visibility bypasses respawn cadence");
    assert_eq!(scheduler.timer_ms(), 59_990);
    assert_eq!(summary.player_visibility_refresh_intents.len(), 1);
    assert_eq!(
        summary.player_visibility_refresh_intents[0].handle(),
        handle
    );
    // This is the production delivery function, called with the map guard gone.
    assert!(canonical.try_lock().is_ok());
    deliver_deferred_player_visibility_like_cpp(
        &summary.player_visibility_refresh_intents,
        &registry,
    );
    assert_eq!(command_rx.len(), 1);
    let commands = durable.lock().unwrap().drain_like_cpp();
    assert!(
        matches!(commands.as_slice(), [SessionCommand::RefreshDeferredPlayerVisibilityLikeCpp(intent)] if intent.handle() == handle)
    );
    assert!(tick(1, &mut scheduler).is_none());
    assert!(durable.lock().unwrap().drain_like_cpp().is_empty());
    canonical
        .lock()
        .unwrap()
        .detach_player_like_cpp(handle)
        .unwrap();
    deliver_deferred_player_visibility_like_cpp(
        &summary.player_visibility_refresh_intents,
        &registry,
    );
    assert!(durable.lock().unwrap().drain_like_cpp().is_empty());
}
