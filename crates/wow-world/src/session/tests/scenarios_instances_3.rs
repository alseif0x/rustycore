//! Session scenarios exercising the represented instances responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn instance_time_restriction_load_rows_match_cpp_insert_semantics() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session
        .represented_instance_reset_times_like_cpp
        .insert(99, 9_999);

    session.load_instance_time_restriction_rows_like_cpp([(10, 1_000), (20, 2_000), (10, 3_000)]);

    assert_eq!(session.represented_instance_reset_times_like_cpp.len(), 2);
    assert_eq!(
        session.represented_instance_reset_times_like_cpp.get(&10),
        Some(&1_000)
    );
    assert_eq!(
        session.represented_instance_reset_times_like_cpp.get(&20),
        Some(&2_000)
    );
    assert!(
        !session
            .represented_instance_reset_times_like_cpp
            .contains_key(&99)
    );
}
#[test]
fn canonical_instance_count_blocks_new_distinct_instance_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 89);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceCap".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    session.set_max_instances_per_hour_like_cpp(5);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    for instance_id in 100..105 {
        session
            .represented_instance_reset_times_like_cpp
            .insert(instance_id, u64::MAX);
    }

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("too many instances abort"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP,
        }
        .to_bytes()
    );
}
#[test]
fn canonical_instance_count_allows_dead_player_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "DeadInstanceCap".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    session.set_player_alive_like_cpp(false);
    session.set_max_instances_per_hour_like_cpp(1);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    session
        .represented_instance_reset_times_like_cpp
        .insert(100, u64::MAX);

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_instance_count_honors_ignore_farm_limit_flag_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 91);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "IgnoreFarmLimit".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    session.set_max_instances_per_hour_like_cpp(1);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 631,
            instance_type: wow_data::map::MAP_RAID,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: wow_data::map::MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT,
        },
    ])));
    session
        .represented_instance_reset_times_like_cpp
        .insert(100, u64::MAX);

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_instance_entry_records_enter_time_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 92);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceEnterTime".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { key, .. }) if key.instance_id != 0
    ));
    assert!(
        session
            .represented_instance_reset_times_like_cpp
            .contains_key(&1)
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_instance_ignore_raid_config_bypasses_raid_group_requirement_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 82);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RaidIgnoreConfig".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.set_instance_ignore_raid_like_cpp(true);
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        &mut session,
        631,
        3,
        77,
        2,
        2,
        25,
    );

    assert!(matches!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Create { .. })
    ));
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn canonical_player_dungeon_create_map_creates_temporary_lock_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let owner = ObjectGuid::create_player(1, 64);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        owner,
        "DungeonCreate".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    session.set_instance_lock_mgr(Arc::new(std::sync::RwLock::new(
        wow_instances::InstanceLockMgr::default(),
    )));

    let decision = session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .unwrap();
    let lock_context = session
        .create_map_active_instance_lock_context_like_cpp(631, 3)
        .unwrap();

    assert_eq!(
        decision,
        wow_map::CreateMapDecision::Create {
            key: wow_map::MapKey::new(631, 1),
            difficulty_id: 3,
            kind: wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            side_effects: vec![
                wow_map::CreateMapSideEffect::CreateInstanceLockForNewInstance {
                    owner_guid_counter: owner.counter() as u64,
                    instance_id: 1,
                },
                wow_map::CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 1 },
            ],
        }
    );
    assert_eq!(lock_context.instance_id, 1);
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        1
    );

    let manager = canonical.lock().unwrap();
    let map = manager.find_map(631, 1).unwrap();
    assert_eq!(map.instance_lock_token(), Some(lock_context.token));
    assert_eq!(map.instance_lock_context(), Some(lock_context));
    assert!(
        map.map().get_typed_player(owner).is_some(),
        "player snapshot should be synchronized into the created dungeon map"
    );
}
#[test]
fn canonical_player_dungeon_create_map_reuses_active_lock_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let owner = ObjectGuid::create_player(1, 65);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        owner,
        "DungeonReuse".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let expected_token =
        install_active_instance_lock_mgr_like_cpp(&mut session, owner, 631, 3, 9001);

    let decision = session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .unwrap();

    assert_eq!(
        decision,
        wow_map::CreateMapDecision::Create {
            key: wow_map::MapKey::new(631, 9001),
            difficulty_id: 3,
            kind: wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            side_effects: vec![wow_map::CreateMapSideEffect::SetPlayerRecentInstance {
                instance_id: 9001,
            }],
        }
    );

    let manager = canonical.lock().unwrap();
    assert_eq!(
        manager.find_map(631, 9001).unwrap().instance_lock_context(),
        Some(wow_map::CreateMapInstanceLockContext {
            instance_id: 9001,
            difficulty_id: 3,
            token: expected_token,
            owner_guid_counter: owner.counter() as u64,
        })
    );
}
#[test]
fn canonical_player_existing_instance_map_rejects_incompatible_player_lock_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let leader = ObjectGuid::create_player(1, 67);
    let member = ObjectGuid::create_player(1, 68);
    let instance_owner = ObjectGuid::create_player(1, 69);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        member,
        "DungeonLockReject".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.set_recent_instance_like_cpp(631, instance_owner, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    let entries = session.create_map_db2_entries_like_cpp(631, 3).unwrap();
    let now = u64::try_from(unix_now()).unwrap_or(0);
    let mut mgr = wow_instances::InstanceLockMgr::default();
    let target_lock = mgr
        .create_instance_lock_for_new_instance_at(
            instance_owner,
            &entries,
            9001,
            wow_instances::ResetSchedule::default(),
            now,
        )
        .unwrap()
        .clone();
    mgr.update_instance_lock_for_player_at(
        member,
        &entries,
        wow_instances::InstanceLockUpdateEvent {
            instance_id: 9002,
            new_data: String::new(),
            instance_completed_encounters_mask: 0,
            completed_encounter_bit: None,
            entrance_world_safe_loc_id: None,
        },
        wow_instances::ResetSchedule::default(),
        now,
    )
    .unwrap();
    session.set_instance_lock_mgr(Arc::new(std::sync::RwLock::new(mgr)));

    let target_context = wow_map::CreateMapInstanceLockContext {
        instance_id: 9001,
        difficulty_id: 3,
        token: create_map_instance_lock_token_like_cpp(instance_owner, &entries, &target_lock),
        owner_guid_counter: instance_owner.counter() as u64,
    };
    {
        let mut manager = canonical.lock().unwrap();
        manager
            .create_map_entry(
                631,
                9001,
                3,
                wow_map::ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
            )
            .set_instance_lock_context(Some(target_context));
    }

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 631,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: wow_instances::TransferAbortReason::LockedToDifferentInstance as u32,
        }
        .to_bytes()
    );
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(631, 9001)
            .unwrap()
            .map()
            .get_typed_player(member)
            .is_none(),
        "rejected player must not be synchronized into the incompatible instance map"
    );
}
#[test]
fn canonical_player_dungeon_create_map_regenerates_conflicting_encounter_lock_instance_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let owner = ObjectGuid::create_player(1, 66);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        owner,
        "DungeonConflict".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        631,
        1,
        1,
        80,
        0,
    ));
    session.represented_raid_difficulty_id_like_cpp = 3;
    install_create_map_encounter_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let active_token = install_active_instance_lock_mgr_like_cpp(&mut session, owner, 631, 3, 9001);

    {
        let mut manager = canonical.lock().unwrap();
        manager.init_instance_ids(9001);
        for instance_id in 1..=9001 {
            manager.register_instance_id(instance_id);
        }
        manager
            .create_map_entry(
                631,
                9001,
                3,
                wow_map::ManagedMapKind::Dungeon {
                    has_reset_schedule: true,
                },
            )
            .set_instance_lock_token(Some(active_token.wrapping_add(1)));
    }

    let decision = session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .unwrap();
    let lock_context = session
        .create_map_active_instance_lock_context_like_cpp(631, 3)
        .unwrap();

    assert_eq!(
        decision,
        wow_map::CreateMapDecision::Create {
            key: wow_map::MapKey::new(631, 9002),
            difficulty_id: 3,
            kind: wow_map::ManagedMapKind::Dungeon {
                has_reset_schedule: true,
            },
            side_effects: vec![
                wow_map::CreateMapSideEffect::SetInstanceLockInstanceId { instance_id: 9002 },
                wow_map::CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 9002 },
            ],
        }
    );
    assert_eq!(lock_context.instance_id, 9002);
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        9002
    );

    let manager = canonical.lock().unwrap();
    assert_eq!(
        manager.find_map(631, 9001).unwrap().instance_lock_token(),
        Some(active_token.wrapping_add(1))
    );
    assert_eq!(
        manager.find_map(631, 9002).unwrap().instance_lock_token(),
        Some(lock_context.token)
    );
    assert!(
        manager
            .find_map(631, 9002)
            .unwrap()
            .map()
            .get_typed_player(owner)
            .is_some(),
        "player snapshot should enter the regenerated instance map"
    );
}
#[test]
fn canonical_world_map_login_binding_rejects_dungeon_missing_difficulty_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let guid = ObjectGuid::create_player(1, 42);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 33,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        guid,
        "Player".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        33,
        1,
        1,
        10,
        0,
    ));

    assert_eq!(
        session.ensure_canonical_world_map_for_current_player_like_cpp(),
        Some(wow_map::CreateMapDecision::Reject {
            side_effects: Vec::new()
        })
    );
    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 33,
            arg: 0,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(canonical.lock().unwrap().find_map(33, 0).is_none());
}
#[test]
fn canonical_loot_object_lookups_and_mutations_use_player_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_700);
    let creature_guid = test_creature_guid(61_701);
    let gameobject_guid = test_gameobject_guid(61_702, 61_702);
    let position = Position::new(10.0, 20.0, 30.0, 1.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);
    add_canonical_test_creature_on_map(&canonical, creature_guid, 9_100, position, 0, 571, 0);
    add_canonical_test_creature_on_map(&canonical, creature_guid, 9_107, position, 0, 571, 7);
    add_canonical_test_gameobject_on_map(&canonical, gameobject_guid, 9_200, position, 571, 0);
    add_canonical_test_gameobject_on_map(&canonical, gameobject_guid, 9_207, position, 571, 7);

    assert_eq!(
        session
            .canonical_creature_access_like_cpp(creature_guid)
            .map(|access| access.entry),
        Some(9_107)
    );
    assert_eq!(
        session
            .canonical_gameobject_access_like_cpp(gameobject_guid)
            .map(|access| access.entry),
        Some(9_207)
    );
    assert_eq!(
        session.mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature.unit_mut().set_health(37);
            creature.current_health()
        }),
        Some(37)
    );
    assert!(
        session
            .mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
                gameobject.set_created_by(player_guid)
            },)
            .is_some()
    );

    let guard = canonical.lock().unwrap();
    let default_map = guard.find_map(571, 0).unwrap().map();
    assert_eq!(
        default_map
            .with_creature_like_cpp(creature_guid, Clone::clone)
            .unwrap()
            .current_health(),
        100
    );
    assert!(
        default_map
            .get_typed_game_object(gameobject_guid)
            .unwrap()
            .owner_guid()
            .is_empty()
    );
    let player_map = guard.find_map(571, 7).unwrap().map();
    assert_eq!(
        player_map
            .with_creature_like_cpp(creature_guid, Clone::clone)
            .unwrap()
            .current_health(),
        37
    );
    assert_eq!(
        player_map
            .get_typed_game_object(gameobject_guid)
            .unwrap()
            .owner_guid(),
        player_guid
    );
}
#[test]
fn canonical_loot_lookup_fails_closed_while_player_is_in_two_maps_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_703);
    let creature_guid = test_creature_guid(61_704);
    let gameobject_guid = test_gameobject_guid(61_705, 61_705);
    let position = Position::new(10.0, 20.0, 30.0, 1.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransferringOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 8);
    add_canonical_test_creature_on_map(&canonical, creature_guid, 9_107, position, 0, 571, 7);
    add_canonical_test_gameobject_on_map(&canonical, gameobject_guid, 9_207, position, 571, 7);

    assert_eq!(session.current_canonical_player_map_key_like_cpp(), None);
    assert_eq!(session.canonical_object_lookup_map_key_like_cpp(571), None);
    assert!(
        session
            .read_canonical_creature_loot_authority_like_cpp(creature_guid)
            .is_none()
    );
    assert!(
        session
            .read_canonical_gameobject_loot_authority_like_cpp(gameobject_guid)
            .is_none()
    );
}
#[tokio::test]
async fn legacy_loot_authority_lookup_uses_player_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let legacy = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 61_705);
    let creature_guid = test_creature_guid(61_706);
    let position = Position::new(15.0, 25.0, 35.0, 1.5);

    session.set_map_manager(Arc::clone(&legacy));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "InstanceOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 7);
    let canonical_position = Position::new(115.0, 125.0, 135.0, 2.5);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_257,
        canonical_position,
        571,
        7,
        33,
    );

    let make_loot = |coins| CreatureLoot {
        loot_guid: creature_guid,
        coins,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid],
        items: Vec::new(),
        looted_by_player: false,
    };
    let make_creature = |instance_id, coins| {
        let mut creature = crate::map_manager::WorldCreature::new(
            creature_guid,
            9_250,
            position,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        );
        creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(571, instance_id)
            .unwrap();
        creature
            .creature
            .initialize_shared_loot_authority_like_cpp(make_loot(coins));
        creature
    };
    let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
    {
        let mut manager = legacy.write().unwrap();
        assert!(manager.add_creature(571, 0, grid_x, grid_y, make_creature(0, 100)));
        assert!(manager.add_creature(571, 7, grid_x, grid_y, make_creature(7, 700)));
    }

    let selected_authority = session
        .read_legacy_creature_loot_authority_like_cpp(creature_guid)
        .expect("the current instance owns a legacy creature runtime");
    assert_eq!(
        selected_authority
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        700
    );

    let claim = selected_authority
        .reserve_money_like_cpp(player_guid)
        .await
        .expect("instance 7 money pool is claimable");
    assert!(claim.commit_like_cpp().unwrap());

    {
        let manager = canonical.lock().unwrap();
        let creature = manager
            .find_map(571, 7)
            .unwrap()
            .map()
            .with_creature_like_cpp(creature_guid, Clone::clone)
            .unwrap();
        assert_eq!(creature.entry(), 9_257);
        assert_eq!(creature.level(), 33);
        assert_eq!(creature.position(), canonical_position);
    }

    let manager = legacy.read().unwrap();
    assert_eq!(
        manager
            .find_creature(571, 0, creature_guid)
            .unwrap()
            .creature
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        100,
        "claiming instance 7 must not steal from the same spawn in instance 0"
    );
    assert_eq!(
        manager
            .find_creature(571, 7, creature_guid)
            .unwrap()
            .creature
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        0
    );
}
#[test]
fn player_registry_publishes_instance_group_party_type_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 49);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, Position::new(1.0, 2.0, 3.0, 0.0), 571, 0);
    let group_registry = Arc::new(GroupRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);

    let home_group = GroupInfo::new(guid);
    let home_group_guid = home_group.group_guid;
    group_registry.register_group_like_cpp(home_group_guid, home_group);

    let mut instance_group = GroupInfo::new(guid);
    instance_group.group_category = wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP;
    let instance_group_guid = instance_group.group_guid;
    group_registry.register_group_like_cpp(instance_group_guid, instance_group);

    session.set_player_guid(Some(guid));
    session.set_player_map_position_like_cpp(571, position);
    session.player_name = Some("InstancePartyTypeTester".to_string());
    session.group_guid = Some(home_group_guid);
    session.set_player_registry(Arc::clone(&registry));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session.register_in_player_registry();

    let party_type = canonical_party_type_for_test(&canonical, guid);
    assert_eq!(
        party_type,
        [
            wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
            wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP
        ]
    );
}
