// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! instance capability handler tests.

use super::*;
use wow_constants::UnitStandStateType;
use wow_packet::packets::misc::SetSavedInstanceExtend;
use wow_persistence::{
    InstanceLockPersistenceLoadOutcomeLikeCpp, InstanceLockPersistenceMutationLikeCpp,
    InstanceLockPersistenceOutcomeLikeCpp, InstanceLockPersistencePlanLikeCpp,
    InstanceLockPersistencePortLikeCpp, PersistenceFutureLikeCpp,
};

struct InstanceLockPersistenceFixtureLikeCpp {
    outcome: InstanceLockPersistenceOutcomeLikeCpp,
    plans: Mutex<Vec<InstanceLockPersistencePlanLikeCpp>>,
}

impl InstanceLockPersistenceFixtureLikeCpp {
    fn new(outcome: InstanceLockPersistenceOutcomeLikeCpp) -> Self {
        Self {
            outcome,
            plans: Mutex::new(Vec::new()),
        }
    }
}

impl InstanceLockPersistencePortLikeCpp for InstanceLockPersistenceFixtureLikeCpp {
    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceLoadOutcomeLikeCpp> {
        Box::pin(async {
            InstanceLockPersistenceLoadOutcomeLikeCpp::Failed {
                reason: "not used by handler fixture".to_string(),
            }
        })
    }

    fn commit_plan_like_cpp<'a>(
        &'a self,
        plan: InstanceLockPersistencePlanLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceOutcomeLikeCpp> {
        self.plans.lock().unwrap().push(plan);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn set_difficulty_id_resets_instances_before_solo_dungeon_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let entries = wow_instances::MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 10,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 10,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    mgr.update_instance_lock_for_player_at(
        player_guid,
        &entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 100,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        now,
    );

    session.set_player_guid(Some(player_guid));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_player_map_position_like_cpp(0, Position::ZERO);
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        },
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));
    let mgr = Arc::new(std::sync::RwLock::new(mgr));
    session.set_instance_lock_mgr(Arc::clone(&mgr));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    let reset = send_rx.try_recv().expect("instance reset packet");
    assert_eq!(
        WorldPacket::from_bytes(&reset).server_opcode(),
        Some(ServerOpcodes::InstanceReset)
    );
    assert_eq!(&reset[2..], &[0x77, 0x02, 0x00, 0x00]);
    let difficulty = send_rx.try_recv().expect("dungeon difficulty packet");
    assert_eq!(
        WorldPacket::from_bytes(&difficulty).server_opcode(),
        Some(ServerOpcodes::SetDungeonDifficulty)
    );
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn set_difficulty_id_inside_instanceable_map_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        2,
        1,
        DifficultyFlags::CAN_SELECT,
    )])));
    session.set_map_store(Arc::new(MapStore::from_entries([map_entry(
        0,
        wow_data::map::MAP_INSTANCE,
    )])));

    session
        .handle_set_difficulty_id(set_difficulty_request(2))
        .await;

    assert_eq!(session.represented_dungeon_difficulty_id_like_cpp(), 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn stand_state_update_uses_cpp_realm_connection_and_values_use_instance() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 9013);
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    add_canonical_test_player_on_map_for_misc_test(&canonical, player_guid, position, 571, 0);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, position);
    session.set_canonical_map_manager(canonical);

    session
        .handle_stand_state_change(stand_state_change_packet(UnitStandStateType::Sit as u32))
        .await;

    assert_eq!(
        WorldPacket::from_bytes(&realm_rx.try_recv().expect("realm stand packet")).server_opcode(),
        Some(ServerOpcodes::StandStateUpdate)
    );
    assert_eq!(
        WorldPacket::from_bytes(&instance_rx.try_recv().expect("instance VALUES packet"))
            .server_opcode(),
        Some(ServerOpcodes::UpdateObject)
    );
    assert!(realm_rx.try_recv().is_err());
    assert!(instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn reset_instances_handler_resets_player_lock_and_sends_cpp_success_packet() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let entries = wow_instances::MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 10,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 10,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    mgr.update_instance_lock_for_player_at(
        player_guid,
        &entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 100,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        now,
    );

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(0, Position::ZERO);
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        },
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));
    let mgr = Arc::new(std::sync::RwLock::new(mgr));
    session.set_instance_lock_mgr(Arc::clone(&mgr));
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());

    session
        .handle_reset_instances(WorldPacket::from_bytes(&[]))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::InstanceReset as u16
    );
    assert_eq!(&sent[2..], &[0x77, 0x02, 0x00, 0x00]);
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .is_none()
    );
    assert!(matches!(
        port.plans.lock().unwrap()[0].mutations.as_slice(),
        [InstanceLockPersistenceMutationLikeCpp::ForceExpireCharacterLock { .. }]
    ));
}

#[tokio::test]
async fn set_saved_instance_extend_updates_lock_and_sends_calendar_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let entries = wow_instances::MapDb2Entries {
        map_id: 631,
        difficulty_id: 4,
        lock_id: 10,
        reset_interval: wow_instances::MapDifficultyResetInterval::Weekly,
        max_players: 10,
        is_flex_locking: true,
        is_using_encounter_locks: false,
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    mgr.update_instance_lock_for_player_at(
        player_guid,
        &entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 100,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        now,
    );

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(0, Position::ZERO);
    session.set_map_store(Arc::new(MapStore::from_entries([
        MapEntry {
            id: 0,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        },
    ])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([difficulty_entry(
        4,
        2,
        DifficultyFlags::empty(),
    )])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 10,
            reset_interval: 2,
            max_players: 0,
            flags: 0,
        },
    ])));
    let mgr = Arc::new(std::sync::RwLock::new(mgr));
    session.set_instance_lock_mgr(Arc::clone(&mgr));
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());

    session
        .handle_set_saved_instance_extend(SetSavedInstanceExtend {
            map_id: 631,
            difficulty_id: 4,
            extend: true,
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::CalendarRaidLockoutUpdated as u16
    );
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    let _server_time = pkt.read_uint32().unwrap();
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    let old_remaining = pkt.read_int32().unwrap();
    let new_remaining = pkt.read_int32().unwrap();
    assert!(new_remaining > old_remaining);
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .unwrap()
            .extended
    );
    assert!(matches!(
        port.plans.lock().unwrap()[0].mutations.as_slice(),
        [InstanceLockPersistenceMutationLikeCpp::UpdateCharacterLockExtension { .. }]
    ));
}

#[tokio::test]
async fn set_saved_instance_extend_current_map_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_map_position_like_cpp(631, Position::ZERO);

    session
        .handle_set_saved_instance_extend(SetSavedInstanceExtend {
            map_id: 631,
            difficulty_id: 4,
            extend: true,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[test]
fn send_pending_raid_lock_sets_pending_bind_like_cpp_for_stop_prompt() {
    let (mut session, send_rx) = make_session();

    session.send_pending_raid_lock_like_cpp(77, 0xA5, true, false);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::PendingRaidLock as u16
    );
    assert_eq!(
        session.pending_bind,
        Some(crate::session::RepresentedPendingBind {
            map_id: 0,
            instance_id: 77,
            completed_mask: 0xA5,
            time_until_lock_ms: 60_000,
        })
    );
}

#[test]
fn send_pending_raid_lock_warning_only_does_not_set_pending_bind_like_cpp() {
    let (mut session, _send_rx) = make_session();

    session.send_pending_raid_lock_like_cpp(77, 0xA5, false, true);

    assert!(session.pending_bind.is_none());
}

#[tokio::test]
async fn instance_lock_response_accept_confirms_and_clears_pending_bind_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let mgr =
        install_pending_bind_instance_context_like_cpp(&mut session, player_guid, 631, 9001, 4, 10);
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Committed,
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());
    session.pending_bind = Some(crate::session::RepresentedPendingBind {
        map_id: 631,
        instance_id: 9001,
        completed_mask: 0xA5,
        time_until_lock_ms: 60_000,
    });

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
        .await;

    assert!(session.pending_bind.is_none());
    assert_eq!(session.represented_confirmed_pending_binds, vec![9001]);
    assert_eq!(session.represented_repop_at_graveyard_count, 0);

    let entries = session.create_map_db2_entries_like_cpp(631, 4).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let lock = mgr
        .read()
        .unwrap()
        .find_active_instance_lock_at(player_guid, &entries, now)
        .cloned()
        .expect("accepting a matching pending bind creates the player instance lock");
    assert_eq!(lock.instance_id, 9001);
    assert_eq!(lock.data.completed_encounters_mask, 0xA5);
    assert!(!lock.is_new);

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::InstanceSaveCreated as u16
    );
    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([sent[0], sent[1]]),
        ServerOpcodes::CalendarRaidLockoutAdded as u16
    );
    let mut pkt = WorldPacket::from_bytes(&sent[2..]);
    assert_eq!(pkt.read_uint64().unwrap(), 9001);
    let _server_time = pkt.read_uint32().unwrap();
    assert_eq!(pkt.read_int32().unwrap(), 631);
    assert_eq!(pkt.read_uint32().unwrap(), 4);
    assert!(pkt.read_int32().unwrap() > 0);
    assert!(matches!(
        port.plans.lock().unwrap()[0].mutations.as_slice(),
        [
            InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock { .. },
            InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock { .. }
        ]
    ));
}

#[tokio::test]
async fn pending_bind_commit_failure_keeps_mutation_but_publishes_nothing_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let mgr =
        install_pending_bind_instance_context_like_cpp(&mut session, player_guid, 631, 9001, 4, 10);
    let port = Arc::new(InstanceLockPersistenceFixtureLikeCpp::new(
        InstanceLockPersistenceOutcomeLikeCpp::Failed {
            reason: "fixture commit failure".to_string(),
        },
    ));
    session.set_instance_lock_persistence_port_like_cpp(port.clone());
    session.pending_bind = Some(crate::session::RepresentedPendingBind {
        map_id: 631,
        instance_id: 9001,
        completed_mask: 0xA5,
        time_until_lock_ms: 60_000,
    });

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
        .await;

    assert!(session.pending_bind.is_none());
    assert!(session.represented_confirmed_pending_binds.is_empty());
    assert!(send_rx.try_recv().is_err());
    assert_eq!(port.plans.lock().unwrap().len(), 1);

    let entries = session.create_map_db2_entries_like_cpp(631, 4).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .is_some(),
        "the represented path already mutated memory before the failed commit, matching the pre-port ordering"
    );
}

#[tokio::test]
async fn instance_lock_response_accept_mismatched_instance_only_clears_pending_bind_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let mgr =
        install_pending_bind_instance_context_like_cpp(&mut session, player_guid, 631, 9001, 4, 10);
    session.pending_bind = Some(crate::session::RepresentedPendingBind {
        map_id: 631,
        instance_id: 9002,
        completed_mask: 0xA5,
        time_until_lock_ms: 60_000,
    });

    session
        .handle_instance_lock_response(WorldPacket::from_bytes(&[0x80]))
        .await;

    assert!(session.pending_bind.is_none());
    assert!(session.represented_confirmed_pending_binds.is_empty());
    assert!(send_rx.try_recv().is_err());

    let entries = session.create_map_db2_entries_like_cpp(631, 4).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        mgr.read()
            .unwrap()
            .find_active_instance_lock_at(player_guid, &entries, now)
            .is_none(),
        "C++ ConfirmPendingBind returns before creating a lock when current InstanceMap id mismatches"
    );
}
