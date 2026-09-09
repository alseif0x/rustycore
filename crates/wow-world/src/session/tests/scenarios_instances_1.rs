//! Session scenarios exercising the represented instances responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn closed_instance_link_releases_character_login_claim_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x7FFF_FF02);
    let (mut first, _, _) = make_session();
    let (mut second, _, _) = make_session();

    assert!(first.try_claim_character_login_like_cpp(guid));
    first.set_player_loading(Some(guid));
    first.set_connect_to_key(Some(77));
    let (link_tx, link_rx) = tokio::sync::oneshot::channel();
    first.set_instance_link_rx(Some(link_rx));
    drop(link_tx);

    first.poll_instance_link().await;

    assert_eq!(first.player_loading(), None);
    assert_eq!(first.connect_to_key(), None);
    assert!(
        second.try_claim_character_login_like_cpp(guid),
        "a failed instance handoff must not strand the process-wide login claim"
    );
    second.release_character_login_claim_like_cpp();
}
#[test]
fn represented_player_recent_instance_defaults_to_zero_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        0
    );
}
#[test]
fn represented_player_recent_instance_tracks_id_by_map_like_cpp() {
    let (mut session, _, _) = make_session();

    session.set_represented_player_recent_instance_like_cpp(631, 9001);
    session.set_represented_player_recent_instance_like_cpp(533, 7001);
    session.set_represented_player_recent_instance_like_cpp(631, 9002);

    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        9002
    );
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(533),
        7001
    );
}
#[test]
fn canonical_player_recent_instances_follow_active_detached_and_stale_ownership_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_579);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RecentInstanceOwner".to_string(),
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

    assert!(session.set_represented_player_recent_instance_like_cpp(631, 9_001));
    assert_eq!(
        session.resolved_player_recent_instance_id_like_cpp(631),
        Some(9_001)
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(
        session.resolved_player_recent_instance_id_like_cpp(631),
        Some(9_001)
    );

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement
        .gameplay_state_mut()
        .recent_instances
        .insert(631, 7_777);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(
        session.resolved_player_recent_instance_id_like_cpp(631),
        None
    );
    assert!(!session.set_represented_player_recent_instance_like_cpp(631, 0));
    assert!(!session.forget_represented_player_recent_instance_like_cpp(631));
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .gameplay_state()
                .recent_instances
                .get(&631)
                .copied(),),
        Some(Some(7_777))
    );
}
#[test]
fn create_map_player_context_uses_solo_recent_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let map_entry =
        represented_map_entry_for_create_map_context_like_cpp(631, wow_data::map::MAP_INSTANCE);

    session.represented_dungeon_difficulty_id_like_cpp = 2;
    session.set_represented_player_recent_instance_like_cpp(631, 9001);

    let context = session
        .create_map_player_context_like_cpp(631, map_entry, player_guid)
        .expect("test Player difficulty owner resolves");

    assert_eq!(context.guid_counter, player_guid.counter() as u64);
    assert_eq!(context.player_difficulty_id, 2);
    assert_eq!(context.player_recent_instance_id, 9001);
    assert_eq!(context.group, None);
}
#[test]
fn create_map_player_context_uses_group_recent_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let owner = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.dungeon_difficulty_id = 2;
    group.set_recent_instance_like_cpp(631, owner, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let map_entry =
        represented_map_entry_for_create_map_context_like_cpp(631, wow_data::map::MAP_INSTANCE);

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    let context = session
        .create_map_player_context_like_cpp(631, map_entry, member)
        .expect("test Player difficulty owner resolves");
    let group = context.group.unwrap();

    assert_eq!(context.guid_counter, member.counter() as u64);
    assert_eq!(group.difficulty_id, 2);
    assert_eq!(
        group.recent_instance_owner_guid_counter,
        owner.counter() as u64
    );
    assert_eq!(group.recent_instance_id, 9001);
}
#[test]
fn create_map_player_context_uses_legacy_raid_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let map_entry =
        represented_map_entry_for_create_map_context_like_cpp(249, wow_data::map::MAP_RAID);

    session.represented_raid_difficulty_id_like_cpp = 15;
    session.represented_legacy_raid_difficulty_id_like_cpp = 4;
    install_create_map_difficulty_stores_like_cpp(
        &mut session,
        249,
        3,
        DifficultyFlags::CAN_SELECT | DifficultyFlags::DEFAULT | DifficultyFlags::LEGACY,
    );

    let context = session
        .create_map_player_context_like_cpp(249, map_entry, player_guid)
        .expect("test Player difficulty owner resolves");

    assert_eq!(context.player_difficulty_id, 4);
}
#[test]
fn create_map_difficulty_context_uses_downscaled_map_entries_like_cpp() {
    let (mut session, _, _) = make_session();
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
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        DifficultyEntry {
            id: 5,
            instance_type: MAP_INSTANCE_LIKE_CPP,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 2,
            instance_type: MAP_INSTANCE_LIKE_CPP,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 33,
            difficulty_id: 2,
            lock_id: 9,
            reset_interval: 1,
            max_players: 0,
            flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
        },
    ])));

    let context = session
        .create_map_difficulty_context_like_cpp(33, 5)
        .unwrap();

    assert_eq!(
        context,
        wow_map::CreateMapDifficultyContext {
            difficulty_id: 2,
            has_reset_schedule: true,
            is_instance_id_bound: false,
        }
    );
}
#[test]
fn create_map_difficulty_context_marks_normal_instance_id_bound_like_cpp() {
    let (mut session, _, _) = make_session();
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
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: 1,
        instance_type: MAP_INSTANCE_LIKE_CPP,
        flags: DifficultyFlags::DEFAULT.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 33,
            difficulty_id: 1,
            lock_id: 7,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        },
    ])));

    let context = session
        .create_map_difficulty_context_like_cpp(33, 1)
        .unwrap();

    assert_eq!(
        context,
        wow_map::CreateMapDifficultyContext {
            difficulty_id: 1,
            has_reset_schedule: false,
            is_instance_id_bound: true,
        }
    );
}
#[test]
fn create_map_difficulty_context_rejects_unrepresentable_inputs_like_cpp() {
    let (mut session, _, _) = make_session();

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
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: 1,
        instance_type: MAP_INSTANCE_LIKE_CPP,
        flags: DifficultyFlags::DEFAULT.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([])));

    assert!(
        session
            .create_map_difficulty_context_like_cpp(33, 1)
            .is_none()
    );
}
#[test]
fn create_map_active_instance_lock_context_uses_solo_owner_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.player_guid = Some(player_guid);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let expected_token =
        install_active_instance_lock_mgr_like_cpp(&mut session, player_guid, 631, 3, 9001);

    let context = session
        .create_map_active_instance_lock_context_like_cpp(631, 3)
        .unwrap();

    assert_eq!(
        context,
        wow_map::CreateMapInstanceLockContext {
            instance_id: 9001,
            difficulty_id: 3,
            token: expected_token,
            owner_guid_counter: player_guid.counter() as u64,
        }
    );
    assert_ne!(context.token, 0);
}
#[test]
fn create_map_active_instance_lock_context_uses_group_recent_owner_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let owner = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.set_recent_instance_like_cpp(631, owner, 9001);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.player_guid = Some(member);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let expected_token =
        install_active_instance_lock_mgr_like_cpp(&mut session, owner, 631, 3, 9001);

    let context = session
        .create_map_active_instance_lock_context_like_cpp(631, 3)
        .unwrap();

    assert_eq!(context.instance_id, 9001);
    assert_eq!(context.difficulty_id, 3);
    assert_eq!(context.token, expected_token);
}
#[test]
fn create_map_instance_lock_token_distinguishes_owner_like_cpp() {
    let (mut session, _, _) = make_session();
    let owner_a = ObjectGuid::create_player(1, 42);
    let owner_b = ObjectGuid::create_player(1, 77);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    let entries = session.create_map_db2_entries_like_cpp(631, 3).unwrap();
    let lock = wow_instances::InstanceLock::new(631, 3, u64::MAX, 9001);

    let token_a = create_map_instance_lock_token_like_cpp(owner_a, &entries, &lock);
    let token_b = create_map_instance_lock_token_like_cpp(owner_b, &entries, &lock);

    assert_ne!(token_a, token_b);
}
#[test]
fn create_map_active_instance_lock_context_rejects_missing_lock_inputs_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.player_guid = Some(player_guid);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);

    assert!(
        session
            .create_map_active_instance_lock_context_like_cpp(631, 3)
            .is_none()
    );

    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 0);
    let entries = session.create_map_db2_entries_like_cpp(631, 3).unwrap();
    let mut mgr = wow_instances::InstanceLockMgr::default();
    assert!(
        mgr.create_instance_lock_for_new_instance_at(
            player_guid,
            &entries,
            9001,
            wow_instances::ResetSchedule::default(),
            u64::try_from(unix_now()).unwrap_or(0),
        )
        .is_none()
    );
    session.set_instance_lock_mgr(Arc::new(std::sync::RwLock::new(mgr)));

    assert!(
        session
            .create_map_active_instance_lock_context_like_cpp(631, 3)
            .is_none()
    );
}
#[test]
fn create_map_side_effects_set_player_recent_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let decision = wow_map::CreateMapDecision::Create {
        key: wow_map::MapKey::new(631, 9001),
        difficulty_id: 1,
        kind: wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        side_effects: vec![wow_map::CreateMapSideEffect::SetPlayerRecentInstance {
            instance_id: 9001,
        }],
    };

    let summary = session.apply_create_map_side_effects_like_cpp(631, &decision);

    assert_eq!(
        summary,
        CreateMapSideEffectApplySummaryLikeCpp {
            player_recent_instance_sets: 1,
            ..Default::default()
        }
    );
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        9001
    );
}
#[test]
fn create_map_side_effects_set_group_recent_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let leader = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));
    let decision = wow_map::CreateMapDecision::Create {
        key: wow_map::MapKey::new(631, 9001),
        difficulty_id: 1,
        kind: wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
        side_effects: vec![wow_map::CreateMapSideEffect::SetGroupRecentInstance {
            owner_guid_counter: owner.counter() as u64,
            instance_id: 9001,
        }],
    };

    let summary = session.apply_create_map_side_effects_like_cpp(631, &decision);
    let group = group_registry.get(&group_guid).unwrap();

    assert_eq!(
        summary,
        CreateMapSideEffectApplySummaryLikeCpp {
            group_recent_instance_sets: 1,
            ..Default::default()
        }
    );
    assert_eq!(group.recent_instance_owner_like_cpp(631), owner);
    assert_eq!(group.recent_instance_id_like_cpp(631), 9001);
}
#[test]
fn create_map_side_effects_create_instance_lock_for_new_instance_like_cpp() {
    let (mut session, _, _) = make_session();
    let owner = ObjectGuid::create_player(1, 77);
    session.player_guid = Some(owner);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    session.set_instance_lock_mgr(Arc::new(std::sync::RwLock::new(
        wow_instances::InstanceLockMgr::default(),
    )));
    let decision = wow_map::CreateMapDecision::Create {
        key: wow_map::MapKey::new(631, 9001),
        difficulty_id: 3,
        kind: wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
        side_effects: vec![
            wow_map::CreateMapSideEffect::CreateInstanceLockForNewInstance {
                owner_guid_counter: owner.counter() as u64,
                instance_id: 9001,
            },
            wow_map::CreateMapSideEffect::SetPlayerRecentInstance { instance_id: 9001 },
        ],
    };

    let summary = session.apply_create_map_side_effects_like_cpp(631, &decision);
    let context = session
        .create_map_active_instance_lock_context_like_cpp(631, 3)
        .unwrap();

    assert_eq!(
        summary,
        CreateMapSideEffectApplySummaryLikeCpp {
            player_recent_instance_sets: 1,
            instance_lock_creates: 1,
            ..Default::default()
        }
    );
    assert_eq!(context.instance_id, 9001);
    assert_eq!(context.difficulty_id, 3);
}
#[test]
fn create_map_side_effects_set_instance_lock_instance_id_like_cpp() {
    let (mut session, _, _) = make_session();
    let owner = ObjectGuid::create_player(1, 77);
    session.player_guid = Some(owner);
    install_create_map_active_lock_stores_like_cpp(&mut session, 631, 3, 77, 2);
    install_active_instance_lock_mgr_like_cpp(&mut session, owner, 631, 3, 9001);
    let decision = wow_map::CreateMapDecision::Create {
        key: wow_map::MapKey::new(631, 9002),
        difficulty_id: 3,
        kind: wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: true,
        },
        side_effects: vec![wow_map::CreateMapSideEffect::SetInstanceLockInstanceId {
            instance_id: 9002,
        }],
    };

    let summary = session.apply_create_map_side_effects_like_cpp(631, &decision);
    let context = session
        .create_map_active_instance_lock_context_like_cpp(631, 3)
        .unwrap();

    assert_eq!(
        summary,
        CreateMapSideEffectApplySummaryLikeCpp {
            instance_lock_instance_id_updates: 1,
            ..Default::default()
        }
    );
    assert_eq!(context.instance_id, 9002);
}
#[test]
fn create_map_side_effects_report_pending_unwired_effects_like_cpp() {
    let (mut session, _, _) = make_session();
    let decision = wow_map::CreateMapDecision::Reject {
        side_effects: vec![
            wow_map::CreateMapSideEffect::CreateInstanceLockForNewInstance {
                owner_guid_counter: 42,
                instance_id: 9001,
            },
            wow_map::CreateMapSideEffect::SetInstanceLockInstanceId { instance_id: 9002 },
            wow_map::CreateMapSideEffect::TeleportToBattlegroundEntryPoint,
            wow_map::CreateMapSideEffect::SetGroupRecentInstance {
                owner_guid_counter: 77,
                instance_id: 9003,
            },
        ],
    };

    let summary = session.apply_create_map_side_effects_like_cpp(631, &decision);

    assert_eq!(
        summary,
        CreateMapSideEffectApplySummaryLikeCpp {
            skipped_group_recent_instance_sets: 1,
            skipped_instance_lock_creates: 1,
            skipped_instance_lock_instance_id_updates: 1,
            pending_battleground_entry_teleports: 1,
            ..Default::default()
        }
    );
}
#[test]
fn represented_player_reset_success_forgets_recent_instance_like_cpp() {
    let (mut session, _, _) = make_session();

    session.set_represented_player_recent_instance_like_cpp(631, 9001);

    assert!(
        session.apply_represented_player_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::Success,
            GroupInstanceResetMethodLikeCpp::Manual,
        )
    );
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        0
    );
}
#[test]
fn represented_player_reset_not_empty_forgets_only_on_change_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();

    session.set_represented_player_recent_instance_like_cpp(631, 9001);
    assert!(
        !session.apply_represented_player_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::NotEmpty,
            GroupInstanceResetMethodLikeCpp::Manual,
        )
    );
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        9001
    );

    assert!(
        session.apply_represented_player_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::NotEmpty,
            GroupInstanceResetMethodLikeCpp::OnChangeDifficulty,
        )
    );
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        0
    );
}
#[test]
fn represented_player_reset_cannot_reset_keeps_recent_instance_like_cpp() {
    let (mut session, _, _) = make_session();

    session.set_represented_player_recent_instance_like_cpp(631, 9001);

    assert!(
        !session.apply_represented_player_instance_reset_result_like_cpp(
            631,
            GroupInstanceResetResultLikeCpp::CannotReset,
            GroupInstanceResetMethodLikeCpp::Manual,
        )
    );
    assert_eq!(
        session.represented_player_recent_instance_id_like_cpp(631),
        9001
    );
}
/// (3) Packet is NOT sent when `map_id` in command does not match session map.
/// Mirrors C++ map-id check before HaveAtClient.
#[tokio::test]
async fn send_if_visible_command_rejected_on_wrong_map_id_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1003);
    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 530, // wrong map
                instance_id: 0,
                packet_bytes: vec![0x77],
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "must not send when map_id does not match"
    );
}
/// (6) ExplicitPlayer fix — session with player_map_id == 571 (non-zero) receives
/// the packet when map_id in the command matches and source_guid is visible.
///
/// Regression guard for the bug where ExplicitPlayer routing set map_id=0 in the
/// command, causing gate 2 (`player_map_id_like_cpp() != command.map_id`) to
/// discard the packet for any session on a non-zero map.
///
/// C++ anchor: WorldObject::SendMessageToSet / SendDirectMessage — the explicit
/// receiver path does not apply a sender-side map filter; the receiver's own map
/// is the correct reference.
#[tokio::test]
async fn send_if_visible_explicit_player_delivers_to_session_on_non_zero_map_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1006);
    let packet_bytes = vec![0xAA, 0xBB, 0xCC];
    let manager = shared_map_manager();
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            source_guid,
            777,
            Position::ZERO,
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
        ),
    );
    // Session is on map 571 (non-zero) — the same map ExplicitPlayer routing now
    // reads from the registry entry and places in the command.
    session.state = SessionState::LoggedIn;
    session.set_map_manager(manager);
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571, // matches session map — as set by the fixed ExplicitPlayer routing
                instance_id: 0,
                packet_bytes: packet_bytes.clone(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(
        send_rx
            .try_recv()
            .expect("must deliver to session on map 571"),
        packet_bytes,
        "ExplicitPlayer with correct map_id must reach the session"
    );
    assert!(send_rx.try_recv().is_err(), "no extra packets");
}
#[tokio::test]
async fn bind_uses_cross_map_db_destination_and_misc_area_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 719_i32;
    let player_guid = ObjectGuid::create_player(1, 7025);
    let player_position = Position::new(280.0, 380.0, 56.0, 0.75);
    let db_destination = Position::new(30.0, 40.0, 50.0, 1.25);
    let canonical = shared_canonical_map_manager();
    let bind_effect = wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
        effect_misc_value_1: 777,
        implicit_target_1: wow_data::spell::implicit_targets::TARGET_DEST_DB,
        ..Default::default()
    };
    let spell_info = bind_spell_info_like_cpp(spell_id, vec![bind_effect.clone()]);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "BindTarget".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_zone_area_like_cpp(12, 34);
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell_info.clone());
    session.set_spell_store(Arc::new(spell_store));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: bind_effect.effect_index,
                target_map_id: 1,
                x: db_destination.x,
                y: db_destination.y,
                z: db_destination.z,
                orientation: Some(db_destination.orientation),
            }],
            &target_spell_store,
            |map_id| matches!(map_id, 1 | 571),
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 719,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("TARGET_DEST_DB bind should execute");

    assert_eq!(
        session.represented_homebind_like_cpp(),
        Some(RepresentedHomebindLikeCpp {
            map_id: 1,
            area_id: 777,
            position: db_destination,
        }),
        "C++ EffectBind uses m_targets dst when present and EffectMiscValue as area id"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::BindPointUpdate));
    assert!(opcodes.contains(&ServerOpcodes::PlayerBound));
}
