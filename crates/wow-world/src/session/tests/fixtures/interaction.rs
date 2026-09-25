//! Interaction, vehicle, and resurrection fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn send_new_item_plan(
    delivery: SendNewItemDelivery,
) -> SendNewItemPlan {
    SendNewItemPlan {
        player_guid: ObjectGuid::create_player(1, 42),
        item_guid: ObjectGuid::create_item(1, 500),
        item_entry: 9001,
        item_instance: SendNewItemInstancePlan {
            item_id: 9001,
            random_properties_seed: 456,
            random_properties_id: -77,
            modifications: vec![
                SendNewItemModifier {
                    value: 123,
                    modifier_type: 3,
                },
                SendNewItemModifier {
                    value: 25,
                    modifier_type: 5,
                },
            ],
        },
        slot: 4,
        slot_in_bag: 7,
        quest_log_item_id: 777,
        quantity: 3,
        quantity_in_inventory: 9,
        battle_pet_species_id: 123,
        battle_pet_breed_id: 0xBC,
        battle_pet_breed_quality: 0x1A,
        battle_pet_level: 25,
        pushed: true,
        created: false,
        display_text: SendNewItemDisplayText::EncounterLoot,
        dungeon_encounter_id: 615,
        is_encounter_loot: true,
        delivery,
    }
}

pub(in crate::session::tests) fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(1);
    broadcast_info_with_command(guid, send_tx, command_tx)
}

pub(in crate::session::tests) fn broadcast_info_with_command(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp::new(
            format!("Player{}", guid.counter()),
            guid.counter() as u32,
            0,
            1,
            1,
            0,
            2,
        ),
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

pub(in crate::session::tests) fn represented_vehicle_interact_map_store_like_cpp(
    instance_type: i8,
) -> Arc<MapStore> {
    Arc::new(MapStore::from_entries([wow_data::MapEntry {
        id: 571,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]))
}

pub(in crate::session::tests) fn insert_represented_vehicle_target_like_cpp(
    registry: &PlayerRegistry,
    canonical: &SharedCanonicalMapManager,
    target_guid: ObjectGuid,
    position: Position,
    has_vehicle_kit_like_cpp: bool,
) {
    let (send_tx, _send_rx) = flume::bounded(4);
    let mut info = broadcast_info(target_guid, send_tx);
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.position = position;
    registry.register_or_replace(target_guid, info, Default::default());
    add_canonical_test_player_on_map(canonical, target_guid, position, 571, 0);
    assert!(
        with_canonical_player_at_mut_like_cpp(canonical, target_guid, 571, 0, |player| {
            player.gameplay_state_mut().mount_vehicle_kit = has_vehicle_kit_like_cpp.then(|| {
                represented_vehicle_kit_with_passenger_like_cpp(
                    target_guid,
                    test_creature_guid(62_000),
                    true,
                )
            });
        })
        .is_some()
    );
}

pub(in crate::session::tests) fn represented_vehicle_interact_session_like_cpp(
    target_guid: ObjectGuid,
    target_has_vehicle_kit: bool,
    target_position: Position,
    group_target: bool,
    map_instance_type: i8,
) -> WorldSession {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 62_001);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    if group_target {
        group.add_member(target_guid);
    }
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_name_like_cpp("RideVehicleInteractTester".to_string());
    session.set_player_map_position_like_cpp(571, Position::new(0.0, 0.0, 0.0, 0.0));
    session.set_player_registry(Arc::clone(&registry));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_map_store(represented_vehicle_interact_map_store_like_cpp(
        map_instance_type,
    ));
    insert_represented_vehicle_target_like_cpp(
        &registry,
        &canonical,
        target_guid,
        target_position,
        target_has_vehicle_kit,
    );

    session
}

pub(in crate::session::tests) fn represented_vehicle_kit_with_passenger_like_cpp(
    base_guid: ObjectGuid,
    passenger_guid: ObjectGuid,
    ejectable: bool,
) -> wow_entities::Vehicle {
    let mut vehicle = wow_entities::Vehicle::new(
        base_guid,
        TypeId::Player,
        Position::ZERO,
        77,
        0,
        [(
            0,
            wow_entities::VehicleSeatInfo {
                id: 1007,
                attachment_offset: Position::ZERO,
                can_enter_or_exit: true,
                usable_by_override: false,
                can_control: false,
                can_switch_from_seat: false,
                ejectable,
                disables_gravity: false,
                passenger_not_selectable: false,
                keep_pet: false,
            },
            wow_entities::VehicleSeatAddon::default(),
        )],
    );
    vehicle.install();
    assert!(vehicle.add_vehicle_passenger(passenger_guid, 0));
    vehicle
}

pub(in crate::session::tests) fn configure_self_resurrect_canonical_player_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    health: u32,
    max_health: u32,
) {
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "SelfRez".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(health, max_health);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_power_index(PowerType::Rage, Some(1));
            player
                .unit_mut()
                .set_power_index(PowerType::Energy, Some(3));
            player.unit_mut().set_power_index(PowerType::Focus, Some(4));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.unit_mut().set_max_power(PowerType::Rage, 100);
            player.unit_mut().set_power(PowerType::Rage, 50);
            player.unit_mut().set_max_power(PowerType::Energy, 100);
            player.unit_mut().set_power(PowerType::Energy, 30);
            player.unit_mut().set_max_power(PowerType::Focus, 100);
            player.unit_mut().set_power(PowerType::Focus, 40);
            player.clear_data_changes();
        })
        .unwrap();
}
