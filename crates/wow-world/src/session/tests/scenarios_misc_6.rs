//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_player_existing_sync_receives_mount_collision_update_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 42);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
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
        "Tester".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        10,
        0,
    ));
    configure_player_shape_mount_collision_stores_like_cpp(&mut session);
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical map");

    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 1234,
        ..Default::default()
    };
    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    assert_eq!(session.player_mount_display_id_like_cpp, 4321);
    assert!((session.player_collision_height_like_cpp - 7.32).abs() < 0.0001);
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert_eq!(player.unit().data().mount_display_id, 4321);
    assert!((player.unit().collision_height_like_cpp() - 7.32).abs() < 0.0001);
    assert!((player.unit().world().collision_height_like_cpp() - 7.32).abs() < 0.0001);
}
#[test]
fn player_bootstrap_is_consumed_without_a_second_runtime_owner_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    let start = Position::new(1.0, 2.0, 3.0, 4.0);
    session.set_player_gold_like_cpp(1234);
    session.set_player_xp_like_cpp(55);
    session.set_player_next_level_xp_like_cpp(4000);
    session.set_selection_guid_like_cpp(Some(test_creature_guid(77)));
    session.set_known_spells_like_cpp(vec![118, 133]);
    session.player_currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 9,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 9,
            flags: 0,
        },
    );
    let item_guid = ObjectGuid::create_item(1, 500);
    session.insert_inventory_item_like_cpp(
        23,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: 500,
            inventory_type: None,
        },
    );
    let item_object =
        session.make_inventory_item_object(item_guid, 700, guid, 2, 0, ItemContext::None, 23);
    session
        .inventory_item_objects
        .insert(item_guid, item_object);

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "Jaina".to_string(),
        start,
        571,
        1,
        8,
        70,
        0,
    ));

    assert_eq!(session.player_guid(), Some(guid));
    assert_eq!(session.player_name_like_cpp(), Some("Jaina"));
    assert_eq!(session.player_position_like_cpp(), Some(start));
    assert_eq!(session.player_map_id_like_cpp(), 571);
    assert_eq!(session.player_race_like_cpp(), 1);
    assert_eq!(session.player_class_like_cpp(), 8);
    assert_eq!(session.player_level_like_cpp(), 70);
    assert_eq!(session.player_gender_like_cpp(), 0);
    assert_eq!(session.player_gold_like_cpp(), 1234);
    assert_eq!(session.player_xp_like_cpp(), 55);
    assert_eq!(session.player_next_level_xp_like_cpp(), 4000);
    assert_eq!(
        session.selection_guid_like_cpp(),
        Some(test_creature_guid(77))
    );
    assert_eq!(session.known_spells_like_cpp(), &[118, 133]);
    assert_eq!(session.player_currency_quantity(395), Some(9));
    assert_eq!(session.inventory_items_like_cpp()[&23].guid, item_guid);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid].count(),
        2
    );
    assert_eq!(session.player_guid, Some(guid));
    assert_eq!(session.player_name.as_deref(), Some("Jaina"));
    assert_eq!(session.player_position, Some(start));
    assert_eq!(session.current_map_id, 571);

    let moved = Position::new(5.0, 6.0, 7.0, 8.0);
    session.set_player_map_position_like_cpp(1, moved);
    session.set_player_level_like_cpp(71);
    session.set_player_gold_like_cpp(2000);
    session.set_player_xp_like_cpp(66);
    session.learn_known_spell_like_cpp(116);
    session.remove_inventory_item_like_cpp(23);

    assert_eq!(session.player_position_like_cpp(), Some(moved));
    assert_eq!(session.player_map_id_like_cpp(), 1);
    assert_eq!(session.player_level_like_cpp(), 71);
    assert_eq!(session.player_gold_like_cpp(), 2000);
    assert_eq!(session.player_xp_like_cpp(), 66);
    assert!(session.known_spells_like_cpp().contains(&116));
    assert!(!session.inventory_items_like_cpp().contains_key(&23));
    assert_eq!(session.player_position, Some(moved));
    assert_eq!(session.current_map_id, 1);
    assert_eq!(session.player_level, 71);

    session.set_player_guid(None);
    assert_eq!(session.player_guid(), None);
    assert!(!session.player_bootstrap_attached_like_cpp);
}
#[test]
fn gray_level_matches_cpp_formula_and_script_override_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    assert_eq!(session.gray_level(6), 0);
    assert_eq!(session.gray_level(7), 0);
    assert_eq!(session.gray_level(34), 24);
    assert_eq!(session.gray_level(35), 25);
    assert_eq!(session.gray_level(39), 29);
    assert_eq!(session.gray_level(60), 50);
    assert_eq!(session.gray_level(80), 70);

    session.set_represented_gray_level_script_override_like_cpp(80, 79);
    assert_eq!(session.gray_level(80), 79);
}
/// The three shared handles are registration input beside the private registry
/// entry, not gameplay state (#361). A session publishes gameplay facts by
/// replacing the projection, and that publish must not be able to detach or
/// swap a handle a producer resolves against.
/// C++ anchor: `Player::m_clientGUIDs` (`Object.h`) and the session's own
/// visibility/combat-logging state live on the player, never in the copy other
/// players read while distributing a message.
#[test]
fn publishing_gameplay_state_cannot_detach_the_shared_session_handles_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let guid = ObjectGuid::create_player(1, 900);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(4);
    let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(4);
    let registration = broadcast_info_with_command(guid, send_tx, command_tx);
    let visibility = registration.client_visible_guids_like_cpp.clone();
    let refresh = Arc::clone(&registration.visibility_refresh_pending_like_cpp);
    registration
        .advanced_combat_logging_enabled_like_cpp
        .store(true, Ordering::Release);
    let registered = registry.register_or_replace(guid, registration, Default::default());

    let seen = ObjectGuid::create_player(1, 901);
    visibility.insert(seen);

    // The only thing a gameplay publish can replace is the projection.
    assert!(registry.fixture_update(guid, |placement| placement.level = 80));
    assert_eq!(
        registry.group_presence(guid).expect("registered").level,
        80,
        "the projection still carries published gameplay state"
    );

    let recipient = registry.runtime_recipient(guid).expect("registered");
    assert_eq!(recipient.map_id, 0);
    assert!(
        recipient.committed_visibility.contains(&seen),
        "the live visibility handle survives a gameplay publish"
    );
    assert!(
        recipient.advanced_combat_logging,
        "the receiver preference is resolved from the session handle"
    );

    recipient.committed_visibility.remove(&seen);
    assert!(
        !visibility.contains(&seen),
        "resolving against the directory reaches the session's own set"
    );

    registry
        .request_current_visibility_refresh(registered, 0, 0)
        .expect("a current registration accepts a refresh");
    assert!(
        refresh.load(Ordering::Acquire),
        "the retained notify bit is the session's own handle"
    );
}
#[test]
fn player_registry_replacement_rejects_stale_lookup_and_unregister() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 70_001);
    let (first_send_tx, _first_send_rx) = flume::bounded(1);
    let first = registry.register_or_replace(
        guid,
        broadcast_info(guid, first_send_tx),
        Default::default(),
    );
    let (second_send_tx, _second_send_rx) = flume::bounded(1);
    let second = registry.register_or_replace(
        guid,
        broadcast_info(guid, second_send_tx),
        Default::default(),
    );

    assert_ne!(first.generation(), second.generation());
    assert!(registry.lookup_current(first).is_none());
    assert!(registry.lookup_current(second).is_some());
    assert!(!registry.unregister(first));
    assert!(registry.lookup_current(second).is_some());
    assert!(registry.unregister(second));
    assert!(registry.control_address(guid).is_none());
}
#[test]
fn player_registry_control_address_keeps_incarnation_channel_identity() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 70_002);
    let (send_tx, _send_rx) = flume::bounded(1);
    let (first_command_tx, first_command_rx) = flume::bounded(1);
    let first = registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, send_tx.clone(), first_command_tx),
        Default::default(),
    );
    let first_address = registry.control_address(guid).expect("first address");

    let (second_command_tx, second_command_rx) = flume::bounded(1);
    let second = registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, send_tx, second_command_tx),
        Default::default(),
    );
    let second_address = registry.control_address(guid).expect("second address");

    assert_eq!(first_address.registration(), first);
    assert_eq!(second_address.registration(), second);
    first_address
        .try_send(SessionCommand::KickLikeCpp(KickLikeCppCommand {
            reason: "old incarnation".to_string(),
        }))
        .expect("old channel remains independently addressable");
    assert!(first_command_rx.try_recv().is_ok());
    assert!(second_command_rx.try_recv().is_err());
}
#[test]
fn player_registry_stale_channel_cannot_unregister_replacement() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 70_004);
    let (send_tx, _send_rx) = flume::bounded(1);
    let (first_command_tx, _first_command_rx) = flume::bounded(1);
    registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, send_tx.clone(), first_command_tx.clone()),
        Default::default(),
    );

    let (second_command_tx, _second_command_rx) = flume::bounded(1);
    let second = registry.register_or_replace(
        guid,
        broadcast_info_with_command(guid, send_tx, second_command_tx.clone()),
        Default::default(),
    );

    assert!(!registry.unregister_control_channel(guid, &first_command_tx));
    assert!(registry.lookup_current(second).is_some());
    assert!(registry.unregister_control_channel(guid, &second_command_tx));
    assert!(registry.control_address(guid).is_none());
}
#[test]
fn player_registry_replacement_wins_unregister_race() {
    let registry = Arc::new(PlayerRegistry::new());
    let guid = ObjectGuid::create_player(1, 70_003);
    let (first_send_tx, _first_send_rx) = flume::bounded(1);
    let first = registry.register_or_replace(
        guid,
        broadcast_info(guid, first_send_tx),
        Default::default(),
    );
    let barrier = Arc::new(std::sync::Barrier::new(3));

    let unregister_registry = Arc::clone(&registry);
    let unregister_barrier = Arc::clone(&barrier);
    let unregister = std::thread::spawn(move || {
        unregister_barrier.wait();
        unregister_registry.unregister(first)
    });

    let replacement_registry = Arc::clone(&registry);
    let replacement_barrier = Arc::clone(&barrier);
    let replacement = std::thread::spawn(move || {
        let (send_tx, _send_rx) = flume::bounded(1);
        replacement_barrier.wait();
        replacement_registry.register_or_replace(
            guid,
            broadcast_info(guid, send_tx),
            Default::default(),
        )
    });

    barrier.wait();
    let _old_was_removed_first = unregister.join().expect("unregister thread");
    let current = replacement.join().expect("replacement thread");

    assert!(registry.lookup_current(current).is_some());
    assert_eq!(
        registry.control_address(guid).unwrap().registration(),
        current
    );
}
#[test]
fn represented_current_vehicle_seat_switch_gate_matches_cpp() {
    let (mut session, _, _) = make_session();

    assert!(!session.represented_current_vehicle_seat_can_switch_from_like_cpp());

    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK);
    assert!(
        !session.represented_current_vehicle_seat_can_switch_from_like_cpp(),
        "C++ VehicleSeatEntry::CanSwitchFromSeat only checks VEHICLE_SEAT_FLAG_CAN_SWITCH"
    );

    session.player_vehicle_seat_flags_like_cpp =
        Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK | wow_data::VEHICLE_SEAT_FLAG_CAN_SWITCH);
    assert!(session.represented_current_vehicle_seat_can_switch_from_like_cpp());
}
#[test]
fn represented_vehicle_switch_same_vehicle_records_cpp_change_seat_plan() {
    let (mut session, _, _) = make_session();
    let base = test_creature_guid(61_001);

    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_SWITCH);
    session.set_player_moved_unit_guid_like_cpp(base);

    assert!(session.represented_request_vehicle_switch_seat_like_cpp(base, 4));
    assert_eq!(
        session.represented_vehicle_seat_change_requests_like_cpp(),
        &[RepresentedVehicleSeatChangeRequestLikeCpp {
            seat_id: 4,
            next: true,
        }]
    );

    let mut mismatched_status = wow_packet::packets::movement::MovementInfo {
        guid: test_creature_guid(61_002),
        flags: MovementFlag::ROOT | MovementFlag::FORWARD,
        time: 1_001,
        position: Position::new(2.0, 3.0, 4.0, 0.5),
        ..wow_packet::packets::movement::MovementInfo::default()
    };
    assert!(!session.represented_move_change_vehicle_seats_like_cpp(
        &mut mismatched_status,
        ObjectGuid::EMPTY,
        0,
    ));
    assert!(!mismatched_status.flags.contains(MovementFlag::ROOT));
    assert!(
        session
            .represented_vehicle_base_movements_like_cpp()
            .is_empty(),
        "C++ validates status before the GUID mismatch return, but does not copy movement"
    );
    assert_eq!(
        session
            .represented_vehicle_seat_change_requests_like_cpp()
            .len(),
        1,
        "C++ returns when moveChangeVehicleSeats.Status.guid does not match vehicle_base"
    );

    let move_position = Position::new(5.0, 6.0, 7.0, 1.25);
    let mut status = wow_packet::packets::movement::MovementInfo {
        guid: base,
        flags: MovementFlag::ROOT | MovementFlag::FORWARD,
        time: 2_002,
        position: move_position,
        ..wow_packet::packets::movement::MovementInfo::default()
    };
    assert!(session.represented_move_change_vehicle_seats_like_cpp(
        &mut status,
        ObjectGuid::EMPTY,
        u8::MAX,
    ));
    assert_eq!(
        session.represented_vehicle_base_movements_like_cpp(),
        &[RepresentedVehicleBaseMovementLikeCpp {
            vehicle_guid: base,
            sanitized_flags: MovementFlag::FORWARD,
            position: move_position,
            time: 2_002,
        }]
    );
    assert_eq!(
        session.represented_vehicle_seat_change_requests_like_cpp(),
        &[
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: 4,
                next: true,
            },
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: -1,
                next: false,
            },
        ]
    );
}
#[tokio::test]
async fn vehicle_switch_handlers_record_same_vehicle_change_seat_like_cpp() {
    let (mut session, _, _) = make_session();
    let base = test_creature_guid(61_101);
    session.player_vehicle_seat_flags_like_cpp = Some(wow_data::VEHICLE_SEAT_FLAG_CAN_SWITCH);
    session.set_player_moved_unit_guid_like_cpp(base);

    session
        .handle_request_vehicle_switch_seat(
            wow_packet::packets::vehicle::RequestVehicleSwitchSeat {
                vehicle: base,
                seat_index: 2,
            },
        )
        .await;
    session
        .handle_move_change_vehicle_seats(wow_packet::packets::vehicle::MoveChangeVehicleSeats {
            status: wow_packet::packets::movement::MovementInfo {
                guid: base,
                flags: MovementFlag::FORWARD,
                time: 3_003,
                position: Position::new(8.0, 9.0, 10.0, 2.0),
                ..wow_packet::packets::movement::MovementInfo::default()
            },
            dst_vehicle: ObjectGuid::EMPTY,
            dst_seat_index: 1,
        })
        .await;

    assert_eq!(
        session.represented_vehicle_base_movements_like_cpp(),
        &[RepresentedVehicleBaseMovementLikeCpp {
            vehicle_guid: base,
            sanitized_flags: MovementFlag::FORWARD,
            position: Position::new(8.0, 9.0, 10.0, 2.0),
            time: 3_003,
        }]
    );
    assert_eq!(
        session.represented_vehicle_seat_change_requests_like_cpp(),
        &[
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: 2,
                next: true,
            },
            RepresentedVehicleSeatChangeRequestLikeCpp {
                seat_id: -1,
                next: true,
            },
        ]
    );
}
#[test]
fn player_registry_publishes_player_vehicle_kit_snapshot_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 62_050);
    let registry = Arc::new(PlayerRegistry::default());
    bind_canonical_test_player_to_registry_like_cpp(
        &mut session,
        &registry,
        guid,
        Position::ZERO,
        571,
    );
    session.set_player_guid(Some(guid));
    session.set_loaded_player_name_like_cpp("VehicleKitPublisher".to_string());
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(Arc::clone(&registry));

    session.register_in_player_registry();
    session.player_mount_vehicle_kit_like_cpp = Some(
        represented_vehicle_kit_with_passenger_like_cpp(guid, test_creature_guid(62_051), true),
    );
    session.sync_player_registry_state_like_cpp();
    assert!(
        registry
            .vehicle_interaction_snapshot(guid)
            .unwrap()
            .has_vehicle_kit,
        "C++ RideVehicleInteract gates on target Player::GetVehicleKit(), not Player::GetVehicle() passenger state"
    );
}
#[test]
fn represented_ride_vehicle_interact_requires_cpp_gates() {
    let target = ObjectGuid::create_player(1, 62_101);

    let mut no_vehicle_kit = represented_vehicle_interact_session_like_cpp(
        target,
        false,
        Position::new(2.0, 0.0, 0.0, 0.0),
        true,
        wow_data::map::MAP_COMMON,
    );
    assert!(!no_vehicle_kit.represented_ride_vehicle_interact_like_cpp(target));

    let mut not_raid_member = represented_vehicle_interact_session_like_cpp(
        target,
        true,
        Position::new(2.0, 0.0, 0.0, 0.0),
        false,
        wow_data::map::MAP_COMMON,
    );
    assert!(!not_raid_member.represented_ride_vehicle_interact_like_cpp(target));

    let mut too_far = represented_vehicle_interact_session_like_cpp(
        target,
        true,
        Position::new(6.0, 0.0, 0.0, 0.0),
        true,
        wow_data::map::MAP_COMMON,
    );
    assert!(!too_far.represented_ride_vehicle_interact_like_cpp(target));

    let mut arena = represented_vehicle_interact_session_like_cpp(
        target,
        true,
        Position::new(2.0, 0.0, 0.0, 0.0),
        true,
        wow_data::map::MAP_ARENA,
    );
    assert!(!arena.represented_ride_vehicle_interact_like_cpp(target));
}
#[tokio::test]
async fn ride_vehicle_interact_handler_records_enter_vehicle_plan_like_cpp() {
    let target = ObjectGuid::create_player(1, 62_201);
    let mut session = represented_vehicle_interact_session_like_cpp(
        target,
        true,
        Position::new(2.0, 0.0, 0.0, 0.0),
        true,
        wow_data::map::MAP_COMMON,
    );

    session
        .handle_ride_vehicle_interact(wow_packet::packets::vehicle::RideVehicleInteract {
            vehicle: target,
        })
        .await;

    assert_eq!(
        session.represented_vehicle_enter_requests_like_cpp(),
        &[RepresentedVehicleEnterRequestLikeCpp {
            vehicle_guid: target
        }],
        "C++ HandleRideVehicleInteract calls _player->EnterVehicle(target player) after VehicleKit, raid, distance and non-arena gates"
    );
}
#[test]
fn represented_eject_passenger_removes_ejectable_passenger_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 55);
    let passenger_guid = ObjectGuid::create_player(1, 56);
    session.set_player_guid(Some(player_guid));
    session.player_mount_vehicle_kit_like_cpp = Some(
        represented_vehicle_kit_with_passenger_like_cpp(player_guid, passenger_guid, true),
    );

    assert!(session.represented_eject_passenger_like_cpp(passenger_guid));

    assert!(
        session
            .player_mount_vehicle_kit_like_cpp
            .as_ref()
            .unwrap()
            .passenger(0)
            .is_none(),
        "C++ Unit::ExitVehicle removes the passenger from the vehicle seat"
    );
}
#[test]
fn represented_eject_passenger_rejects_non_ejectable_seat_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 57);
    let passenger_guid = ObjectGuid::create_player(1, 58);
    session.set_player_guid(Some(player_guid));
    session.player_mount_vehicle_kit_like_cpp = Some(
        represented_vehicle_kit_with_passenger_like_cpp(player_guid, passenger_guid, false),
    );

    assert!(!session.represented_eject_passenger_like_cpp(passenger_guid));
    assert_eq!(
        session
            .player_mount_vehicle_kit_like_cpp
            .as_ref()
            .unwrap()
            .passenger(0),
        Some(passenger_guid)
    );
}
#[test]
fn represented_eject_passenger_rejects_without_vehicle_kit_or_unit_like_cpp() {
    let (mut session, _, _) = make_session();
    assert!(!session.represented_eject_passenger_like_cpp(ObjectGuid::EMPTY));

    let player_guid = ObjectGuid::create_player(1, 59);
    let passenger_guid = ObjectGuid::create_player(1, 60);
    session.set_player_guid(Some(player_guid));
    session.player_mount_vehicle_kit_like_cpp = Some(
        represented_vehicle_kit_with_passenger_like_cpp(player_guid, passenger_guid, true),
    );

    assert!(!session.represented_eject_passenger_like_cpp(ObjectGuid::EMPTY));
    assert_eq!(
        session
            .player_mount_vehicle_kit_like_cpp
            .as_ref()
            .unwrap()
            .passenger(0),
        Some(passenger_guid)
    );
}
#[test]
fn session_starts_authed() {
    let (session, _, _) = make_session();
    assert_eq!(session.state(), SessionState::Authed);
}
#[test]
fn player_start_config_snapshot_defaults_and_setters_match_cpp_keys() {
    let (mut session, _, _send_rx) = make_session();

    assert!(!session.start_all_explored_like_cpp());
    assert!(!session.start_all_reputation_like_cpp());
    assert!(!session.start_all_spells_like_cpp());

    session.set_start_all_explored_like_cpp(true);
    session.set_start_all_reputation_like_cpp(true);
    session.set_start_all_spells_like_cpp(true);

    assert!(session.start_all_explored_like_cpp());
    assert!(session.start_all_reputation_like_cpp());
    assert!(session.start_all_spells_like_cpp());
}
#[tokio::test]
async fn primary_heal_mechanical_heals_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 743_i32;
    let player_guid = ObjectGuid::create_player(1, 60);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(25, 80);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL,
            effect_base_points: 30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary mechanical heal should execute");

    assert_eq!(session.player_health_like_cpp(), 55);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn primary_heal_pct_heals_percent_of_player_max_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 735_i32;
    let player_guid = ObjectGuid::create_player(1, 52);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(35, 80);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary heal-pct should execute");

    assert_eq!(
        session.player_health_like_cpp(),
        75,
        "C++ CountPctFromMaxHealth(50) heals 40 from an 80 max-health target"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[test]
fn player_attack_rejects_typed_player_victim_without_pvp_snapshot_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 47);
    let victim = ObjectGuid::create_player(1, 48);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    session.start_player_attack_like_cpp(victim);
    {
        let guard = canonical.lock().unwrap();
        let map = guard.find_map(571, 0).unwrap().map();
        let attacker_entity = map.get_typed_player(attacker).unwrap();
        let victim_entity = map.get_typed_player(victim).unwrap();
        assert_eq!(attacker_entity.unit().attacking(), None);
        assert_eq!(attacker_entity.unit().data().target, ObjectGuid::EMPTY);
        assert!(!victim_entity.unit().has_attacker_like_cpp(attacker));
    }
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
