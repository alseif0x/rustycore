//! Session scenarios exercising the represented persistence responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn canonical_player_persistent_metadata_follows_detached_and_stale_ownership_like_cpp() {
    let quest_status = |quest_id, status| wow_entities::PlayerQuestStatusRecord {
        quest_id,
        status,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: Vec::new(),
        slot: 0,
    };
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_567);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "MetadataOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    let old_handle = session.player_handle_like_cpp.expect("canonical handle");
    let share_sender = ObjectGuid::create_player(1, 5_568);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 5_569, 10);
    let transport_guid = ObjectGuid::create_transport(HighGuid::Transport, 7_005);
    let transport_info = wow_packet::packets::movement::TransportInfo {
        guid: transport_guid,
        x: 1.0,
        y: 2.0,
        z: 3.0,
        o: 0.5,
        seat: 4,
        time: 123,
        prev_time: Some(122),
        vehicle_id: Some(99),
    };

    session.set_loaded_player_flags_like_cpp(0x10);
    session.set_loaded_player_flags_ex_like_cpp(0x20);
    session.set_watched_faction_index_like_cpp(42);
    assert!(session.set_loaded_quest_completed_bit_like_cpp(42));
    assert_eq!(session.load_represented_explored_zones_like_cpp("1 0"), 1);
    session.set_represented_pending_quest_sharing_like_cpp(share_sender, 400);
    session.set_player_transport_info_like_cpp(Some(transport_info.clone()));
    assert!(session.set_player_currencies_like_cpp(HashMap::from([(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 10,
            weekly_quantity: 1,
            tracked_quantity: 2,
            increased_cap_quantity: 3,
            earned_quantity: 4,
            flags: 5,
        },
    )])));
    assert!(session.set_player_pet_guid_like_cpp(Some(pet_guid)));
    assert!(session.set_player_vehicle_seat_state_like_cpp(Some(0x10), Some(1001)));
    session.set_player_zone_area_like_cpp(100, 101);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    session.set_player_pvp_state_like_cpp(true, true, true);
    session.set_player_game_master_like_cpp(true);
    session.set_player_mounted_like_cpp(true);
    assert!(
        session
            .mutate_active_player_update_state_like_cpp(|state| {
                state.active_local_flags = 0x11;
                state.active_transport_server_time = 111;
                state.multi_action_bars = 0x1f;
            })
            .is_some()
    );
    session.set_player_moved_unit_guid_like_cpp(pet_guid);
    assert!(
        session
            .mutate_player_unit_presentation_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().set_scale(1.25);
            })
            .is_some()
    );
    assert!(
        session
            .mutate_player_mount_vehicle_kit_like_cpp(|kit| {
                *kit = Some(represented_vehicle_kit_with_passenger_like_cpp(
                    player_guid,
                    test_creature_guid(5_570),
                    true,
                ));
            })
            .is_some()
    );
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.set_daily_like_cpp(100, true);
                state.set_last_daily_quest_time_secs_like_cpp(10);
                state.insert_status_like_cpp(
                    200,
                    quest_status(200, crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP),
                );
                state.set_rewarded_like_cpp(300, true);
            })
            .is_some()
    );
    assert_eq!(
        session.resolved_player_flags_for_create_like_cpp(),
        Some((0x210, 0x20))
    );
    assert_eq!(session.resolved_watched_faction_index_like_cpp(), Some(42));
    assert_eq!(session.player_zone_area_like_cpp(), Some((100, 101)));
    assert_eq!(session.player_is_pvp_like_cpp(player_guid), Some(true));
    assert_eq!(
        session.player_has_in_pvp_flag_like_cpp(player_guid),
        Some(true)
    );
    assert_eq!(session.player_is_game_master_like_cpp(), Some(true));
    assert_eq!(
        session.player_unit_presentation_snapshot_like_cpp(),
        Some((UnitFlags::PLAYER_CONTROLLED | UnitFlags::MOUNT, 1, 1.25))
    );
    assert_eq!(
        session.active_player_update_state_like_cpp(),
        Some((0x11, 111, 0x1f))
    );
    assert_eq!(session.player_moved_unit_guid_like_cpp(), Some(pet_guid));
    assert_eq!(
        session
            .player_quest_gameplay_snapshot_like_cpp()
            .map(|state| (
                state.daily_quest_ids_like_cpp().clone(),
                state.last_daily_quest_time_secs_like_cpp(),
                state.pending_share_like_cpp(),
            )),
        Some((BTreeSet::from([100]), 10, Some((share_sender, 400))))
    );
    assert_eq!(
        session.player_transport_state_like_cpp(),
        Some(Some(wow_entities::PlayerTransportState {
            guid: transport_guid,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
            seat: 4,
            time: 123,
            prev_time: Some(122),
            vehicle_id: Some(99),
        }))
    );

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    session.set_loaded_player_flags_like_cpp(0x30);
    session.set_loaded_player_flags_ex_like_cpp(0x40);
    session.set_watched_faction_index_like_cpp(43);
    assert!(session.clear_loaded_quest_completed_bit_like_cpp(42));
    assert_eq!(session.load_represented_explored_zones_like_cpp("2 0"), 1);
    session.clear_represented_pending_quest_sharing_like_cpp();
    session.set_player_transport_position_like_cpp(Some(Position::new(4.0, 5.0, 6.0, 0.75)));
    assert!(session.set_player_currencies_like_cpp(HashMap::from([(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Changed,
            quantity: 20,
            weekly_quantity: 2,
            tracked_quantity: 3,
            increased_cap_quantity: 4,
            earned_quantity: 5,
            flags: 6,
        },
    )])));
    assert!(session.set_player_pet_guid_like_cpp(None));
    assert!(session.set_player_vehicle_seat_state_like_cpp(Some(0x20), Some(1002)));
    session.set_player_zone_area_like_cpp(200, 201);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    session.set_player_pvp_state_like_cpp(false, false, false);
    session.set_player_game_master_like_cpp(false);
    session.set_player_mounted_like_cpp(false);
    assert!(
        session
            .mutate_active_player_update_state_like_cpp(|state| {
                state.active_local_flags = 0x22;
                state.active_transport_server_time = 222;
                state.multi_action_bars = 0x2f;
            })
            .is_some()
    );
    session.set_player_moved_unit_guid_like_cpp(player_guid);
    assert!(
        session
            .mutate_player_unit_presentation_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().set_scale(1.75);
            })
            .is_some()
    );
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.set_daily_like_cpp(101, true);
                state.set_last_daily_quest_time_secs_like_cpp(20);
                state.insert_status_like_cpp(
                    201,
                    quest_status(201, crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP),
                );
                state.set_rewarded_like_cpp(301, true);
            })
            .is_some()
    );
    assert_eq!(
        session.resolved_player_flags_for_create_like_cpp(),
        Some((0x30, 0x40))
    );
    assert_eq!(session.resolved_watched_faction_index_like_cpp(), Some(43));
    assert_eq!(session.player_zone_area_like_cpp(), Some((200, 201)));
    assert_eq!(session.player_is_pvp_like_cpp(player_guid), Some(false));
    assert_eq!(
        session.player_has_in_pvp_flag_like_cpp(player_guid),
        Some(false)
    );
    assert_eq!(session.player_is_game_master_like_cpp(), Some(false));
    assert_eq!(
        session.player_unit_presentation_snapshot_like_cpp(),
        Some((UnitFlags::PLAYER_CONTROLLED, 0, 1.75))
    );
    assert_eq!(
        session.active_player_update_state_like_cpp(),
        Some((0x22, 222, 0x2f))
    );
    assert_eq!(session.player_moved_unit_guid_like_cpp(), Some(player_guid));

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.replace_all_player_flags(0x100);
    replacement.replace_all_player_flags_ex(0x200);
    replacement.set_watched_faction_index_like_cpp(99);
    replacement
        .gameplay_state_mut()
        .quests
        .set_daily_like_cpp(999, true);
    replacement
        .gameplay_state_mut()
        .quests
        .insert_status_like_cpp(
            9_999,
            quest_status(9_999, crate::conditions::QUEST_STATUS_FAILED_LIKE_CPP),
        );
    replacement
        .gameplay_state_mut()
        .quests
        .set_rewarded_like_cpp(9_998, true);
    replacement
        .gameplay_state_mut()
        .quests
        .set_pending_share_like_cpp(Some((share_sender, 999)));
    replacement.gameplay_state_mut().transport = Some(wow_entities::PlayerTransportState {
        guid: transport_guid,
        x: 10.0,
        y: 20.0,
        z: 30.0,
        orientation: 1.5,
        seat: 2,
        time: 456,
        prev_time: Some(455),
        vehicle_id: Some(100),
    });
    replacement.gameplay_state_mut().currencies.insert(
        395,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 999,
            weekly_quantity: 9,
            tracked_quantity: 9,
            increased_cap_quantity: 9,
            earned_quantity: 9,
            flags: 9,
        },
    );
    replacement.gameplay_state_mut().pet_guid = Some(pet_guid);
    replacement.gameplay_state_mut().vehicle_seat_flags = Some(0x99);
    replacement.gameplay_state_mut().vehicle_seat_id = Some(1999);
    replacement.gameplay_state_mut().active_local_flags = 0x99;
    replacement
        .gameplay_state_mut()
        .active_transport_server_time = 999;
    replacement.gameplay_state_mut().multi_action_bars = 0x3f;
    replacement
        .unit_mut()
        .subsystems_mut()
        .control
        .set_moved_unit(Some(pet_guid));
    replacement.gameplay_state_mut().world_local = wow_entities::PlayerWorldLocalState {
        zone_id: 900,
        area_id: 901,
        zone_area_authority_complete: true,
        pvp_hostile: true,
        pvp_end_timer: Some(123),
        contested_pvp_timer: 456,
        is_outdoors: Some(true),
    };
    replacement
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    replacement.set_game_master_like_cpp(true);
    replacement.unit_mut().set_unit_flags_like_cpp(
        UnitFlags::PLAYER_CONTROLLED | UnitFlags::MOUNT | UnitFlags::IMMUNE,
    );
    replacement.unit_mut().set_mount_display_id(77);
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_scale(1.5);
    replacement.gameplay_state_mut().mount_vehicle_kit =
        Some(represented_vehicle_kit_with_passenger_like_cpp(
            player_guid,
            test_creature_guid(5_571),
            false,
        ));
    assert!(replacement.set_quest_completed_bit_like_cpp(77, true));
    assert!(replacement.set_explored_zones_block_like_cpp(0, 0x400));
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.resolved_player_flags_for_create_like_cpp(), None);
    assert_eq!(session.player_explored_zones_snapshot_like_cpp(), None);
    assert_eq!(session.resolved_watched_faction_index_like_cpp(), None);
    assert_eq!(session.player_quest_gameplay_snapshot_like_cpp(), None);
    assert_eq!(session.player_transport_state_like_cpp(), None);
    assert_eq!(session.player_currencies_like_cpp(), None);
    assert_eq!(session.player_pet_guid_state_like_cpp(), None);
    assert_eq!(session.player_vehicle_seat_state_like_cpp(), None);
    assert_eq!(session.player_mount_vehicle_kit_snapshot_like_cpp(), None);
    assert_eq!(session.player_world_local_state_like_cpp(), None);
    assert_eq!(session.player_zone_area_like_cpp(), None);
    assert_eq!(session.player_is_pvp_like_cpp(player_guid), None);
    assert_eq!(session.player_has_in_pvp_flag_like_cpp(player_guid), None);
    assert_eq!(session.player_is_game_master_like_cpp(), None);
    assert_eq!(session.player_unit_presentation_snapshot_like_cpp(), None);
    assert_eq!(session.active_player_update_state_like_cpp(), None);
    assert_eq!(session.player_moved_unit_guid_like_cpp(), None);
    assert!(!session.set_near_teleport_pending_like_cpp(
        true,
        Some((571, Position::new(1.0, 2.0, 3.0, 0.0))),
        Some((1, 2)),
    ));
    assert_eq!(
        session.handle_move_teleport_ack_like_cpp(player_guid, 1, 2),
        MoveTeleportAckActionLikeCpp::MissingPlayerOwner
    );
    assert!(!session.near_teleport_pending_like_cpp());
    session.set_loaded_player_flags_like_cpp(0xdead);
    session.set_loaded_player_flags_ex_like_cpp(0xbeef);
    session.set_watched_faction_index_like_cpp(5);
    assert!(!session.set_loaded_quest_completed_bit_like_cpp(42));
    assert_eq!(session.load_represented_explored_zones_like_cpp("4 0"), 0);
    session.set_represented_pending_quest_sharing_like_cpp(share_sender, 0xdead);
    session.set_player_transport_info_like_cpp(None);
    assert!(!session.set_player_currencies_like_cpp(HashMap::new()));
    assert!(!session.set_player_pet_guid_like_cpp(None));
    assert!(!session.set_player_vehicle_seat_state_like_cpp(None, None));
    session.set_player_zone_area_like_cpp(0xdead, 0xbeef);
    session.set_player_zone_area_authority_complete_like_cpp(false);
    session.set_player_pvp_state_like_cpp(false, false, true);
    session.set_player_game_master_like_cpp(false);
    session.set_player_mounted_like_cpp(false);
    assert!(
        session
            .mutate_active_player_update_state_like_cpp(|state| {
                state.active_local_flags = 0xdead;
                state.active_transport_server_time = -1;
                state.multi_action_bars = 0;
            })
            .is_none()
    );
    session.set_player_moved_unit_guid_like_cpp(ObjectGuid::EMPTY);
    assert!(
        session
            .mutate_player_unit_presentation_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().set_scale(9.0);
            })
            .is_none()
    );
    assert!(
        session
            .mutate_player_mount_vehicle_kit_like_cpp(|kit| *kit = None)
            .is_none()
    );
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.set_daily_like_cpp(0xdead, true);
                state.remove_status_like_cpp(9_999);
                state.set_rewarded_like_cpp(0xbeef, true);
            })
            .is_none()
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.gameplay_state().active_local_flags,
                player.gameplay_state().active_transport_server_time,
                player.gameplay_state().multi_action_bars,
                player.unit().subsystems().control.unit_moved_by_me,
            )),
        Some((0x99, 999, 0x3f, Some(pet_guid)))
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.data().player_flags,
                player.data().player_flags_ex,
                player.watched_faction_index_like_cpp(),
                player.quest_completed_block_like_cpp(1),
                player.explored_zones_block_like_cpp(0),
                player
                    .gameplay_state()
                    .quests
                    .daily_quest_ids_like_cpp()
                    .clone(),
                player.gameplay_state().quests.statuses_snapshot_like_cpp(),
                player
                    .gameplay_state()
                    .quests
                    .rewarded_quest_ids_like_cpp()
                    .clone(),
                player.gameplay_state().quests.pending_share_like_cpp(),
                player.gameplay_state().transport.clone(),
                player.gameplay_state().currencies.clone(),
            )),
        Some((
            0x100,
            0x200,
            99,
            Some(1 << 12),
            Some(0x400),
            BTreeSet::from([999]),
            BTreeMap::from([(
                9_999,
                quest_status(9_999, crate::conditions::QUEST_STATUS_FAILED_LIKE_CPP),
            )]),
            BTreeSet::from([9_998]),
            Some((share_sender, 999)),
            Some(wow_entities::PlayerTransportState {
                guid: transport_guid,
                x: 10.0,
                y: 20.0,
                z: 30.0,
                orientation: 1.5,
                seat: 2,
                time: 456,
                prev_time: Some(455),
                vehicle_id: Some(100),
            }),
            HashMap::from([(
                395,
                PlayerCurrency {
                    state: PlayerCurrencyState::Unchanged,
                    quantity: 999,
                    weekly_quantity: 9,
                    tracked_quantity: 9,
                    increased_cap_quantity: 9,
                    earned_quantity: 9,
                    flags: 9,
                },
            )]),
        ))
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.is_game_master_like_cpp(),
                player.unit().unit_flags_like_cpp(),
                player.unit().data().mount_display_id,
                player.unit().world().object().scale(),
            )),
        Some((
            true,
            UnitFlags::PLAYER_CONTROLLED | UnitFlags::MOUNT | UnitFlags::IMMUNE,
            77,
            1.5,
        ))
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.gameplay_state().world_local,
                player.unit().pvp_flags_like_cpp(),
            )),
        Some((
            wow_entities::PlayerWorldLocalState {
                zone_id: 900,
                area_id: 901,
                zone_area_authority_complete: true,
                pvp_hostile: true,
                pvp_end_timer: Some(123),
                contested_pvp_timer: 456,
                is_outdoors: Some(true),
            },
            UnitPvpFlags::PVP,
        ))
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| (
                player.gameplay_state().pet_guid,
                player.gameplay_state().vehicle_seat_flags,
                player.gameplay_state().vehicle_seat_id,
                player
                    .gameplay_state()
                    .mount_vehicle_kit
                    .as_ref()
                    .map(|kit| (kit.vehicle_id(), kit.status())),
            )),
        Some((
            Some(pet_guid),
            Some(0x99),
            Some(1999),
            Some((77, wow_entities::VehicleStatus::Installed)),
        ))
    );
}
#[test]
fn load_completed_achievement_rows_like_cpp_clears_stale_and_deduplicates() {
    let (mut session, _, _) = make_session();
    session
        .represented_completed_achievements_like_cpp
        .insert(7777);

    session.load_completed_achievement_rows_like_cpp([9001, 9001, 0, 9002]);

    assert_eq!(session.represented_completed_achievements_like_cpp.len(), 2);
    assert!(
        session
            .represented_completed_achievements_like_cpp
            .contains(&9001)
    );
    assert!(
        session
            .represented_completed_achievements_like_cpp
            .contains(&9002)
    );
    assert!(
        !session
            .represented_completed_achievements_like_cpp
            .contains(&7777)
    );
    assert!(
        !session
            .represented_completed_achievements_like_cpp
            .contains(&0)
    );
}
#[test]
fn canonical_loading_player_bypasses_existing_raid_in_progress_gate_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let leader = ObjectGuid::create_player(1, 76);
    let member = ObjectGuid::create_player(1, 77);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        member,
        "RaidInProgressLoading".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.set_player_loading(Some(member));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 0);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.raid_difficulty_id = 3;
    group.add_member(member);
    group.set_recent_instance_like_cpp(631, leader, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.create_map_entry(
            631,
            9001,
            3,
            wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: false,
            },
        );
        map.set_instance_encounter_in_progress_like_cpp(true);
    }

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Existing {
            key: wow_map::MapKey::new(631, 9001),
            difficulty_id: 3,
            side_effects: Vec::new(),
        })
    );
    assert!(send_rx.try_recv().is_err());
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 9001)
            .unwrap()
            .map()
            .get_typed_player(member)
            .is_some(),
        "loading/relog entry should still synchronize the represented player"
    );
}
#[test]
fn loaded_customizations_update_canonical_player_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 52);
    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(&canonical, guid, Position::ZERO, 571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));

    let customizations = vec![
        wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate {
            option_id: 50,
            choice_id: 60,
        },
    ];
    session.set_loaded_player_customizations_like_cpp(customizations.clone());

    let actual = with_canonical_player_at_like_cpp(&canonical, guid, 571, 0, |player| {
        crate::canonical_player_access::canonical_player_presentation_like_cpp(player).4
    })
    .unwrap();
    assert_eq!(actual, vec![(50, 60)]);
}
#[test]
fn first_login_flag_removal_is_not_persisted_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    const AT_LOGIN_FIRST_LIKE_CPP: u16 = 0x020;

    session.set_represented_at_login_flags_like_cpp(AT_LOGIN_FIRST_LIKE_CPP);

    assert!(session.apply_represented_first_login_flag_if_needed_like_cpp());
    assert_eq!(
        session.represented_at_login_flags_like_cpp() & AT_LOGIN_FIRST_LIKE_CPP,
        0,
        "C++ CharacterHandler removes AT_LOGIN_FIRST from the in-memory player flags"
    );
    assert!(
        session
            .represented_at_login_flag_removals_like_cpp()
            .is_empty(),
        "C++ calls RemoveAtLoginFlag(AT_LOGIN_FIRST) with persist=false"
    );
}
#[test]
fn player_attack_rejects_player_loading_visibility_like_cpp() {
    let (mut attacker_session, _, _) = make_session();
    let (mut victim_session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 51);
    let victim = ObjectGuid::create_player(1, 52);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 571,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    canonical.lock().unwrap().create_world_map(571, 0);
    attacker_session.set_canonical_map_manager(Arc::clone(&canonical));
    victim_session.set_canonical_map_manager(Arc::clone(&canonical));
    attacker_session.set_map_store(Arc::clone(&map_store));
    victim_session.set_map_store(map_store);

    attacker_session.attach_player_controller_like_cpp(SessionPlayerController::new(
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    victim_session.attach_player_controller_like_cpp(SessionPlayerController::new(
        victim,
        "Rogue".to_string(),
        Position::new(11.0, 20.0, 30.0, 0.0),
        571,
        1,
        4,
        80,
        0,
    ));
    let _ = attacker_session.ensure_canonical_world_map_for_current_player_like_cpp();
    let _ = victim_session.ensure_canonical_world_map_for_current_player_like_cpp();
    victim_session
        .mutate_canonical_player_by_guid_like_cpp(victim, |player| {
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();

    victim_session.set_player_loading(Some(victim));
    assert_eq!(
        attacker_session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );
    assert_eq!(attacker_session.combat_target, None);

    victim_session.set_player_loading(None);
    assert_eq!(
        attacker_session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(attacker).unwrap().unit().attacking(),
        Some(victim)
    );
    assert!(
        map.get_typed_player(victim)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(attacker)
    );
}
#[test]
fn logout_save_snapshot_uses_canonical_xp_money_and_health_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 70);
    let stale_canonical_position = Position::new(44.0, 55.0, 66.0, 1.25);
    let latest_session_position = Position::new(77.0, 88.0, 99.0, 2.5);

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
        player_guid,
        "Saver".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        10,
        0,
    ));
    session.set_loaded_player_powers_like_cpp([111, 222, 0, 0, 0, 0, 0, 0, 0, 0]);
    session.set_player_xp_like_cpp(1);
    session.set_player_gold_like_cpp(2);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .relocate(stale_canonical_position);
            player.unit_mut().set_level(42);
            player.set_xp(1234);
            player.set_money(5678);
            player.unit_mut().set_max_health(900);
            player.unit_mut().set_health(456);
            player.unit_mut().set_max_power(PowerType::Mana, 1000);
            player.unit_mut().set_power(PowerType::Mana, 321);
        })
        .unwrap();
    session.set_player_health_like_cpp(456, 900);
    session.set_player_position_like_cpp(latest_session_position);

    let snapshot = session
        .current_player_save_to_db_snapshot_like_cpp()
        .unwrap();
    assert_eq!(
        snapshot,
        PlayerSaveToDbSnapshotLikeCpp {
            guid: player_guid,
            map_id: 571,
            instance_id: 0,
            position: latest_session_position,
            level: 42, // Player::SaveToDB reads canonical Unit::GetLevel.
            xp: 1234,
            money: 5678,
            health: 456,
            max_health: 900,
            powers: loaded_character_power_snapshot_like_cpp([321, 222, 0, 0, 0, 0, 0, 0, 0, 0,]),
        }
    );
    assert_eq!(
        session.player_position_like_cpp(),
        Some(latest_session_position)
    );
    assert_eq!(session.player_level_like_cpp(), 10);
    assert_eq!(session.player_xp_like_cpp(), 1234);
    assert_eq!(session.player_gold_like_cpp(), 5678);
    assert_eq!(session.player_health_like_cpp(), 456);
}
