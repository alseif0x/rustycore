//! Creature scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn loot_release_keeps_unlooted_creature_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_creature_guid(19_013);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |world_creature| {
        world_creature
            .creature
            .set_personal_loot_like_cpp(player_guid, CreatureOwnedLoot::new(0, 1));
    });
    let corpse_despawn_before = session
        .mutate_world_creature(loot_guid, |creature| {
            creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .unwrap()
        .expect("C++ arms corpse removal when the creature reaches JUST_DIED");
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, other_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(
        !session.loot_table.contains_key(&loot_guid),
        "the closed session view is a discardable cache; the creature authority keeps loot"
    );
    assert!(session.reconcile_represented_loot_cache_like_cpp(loot_guid, player_guid));
    assert_eq!(session.loot_table[&loot_guid].coins, 7);
    assert!(session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session.loot_table.get(&loot_guid).unwrap().players_looting,
        vec![other_guid]
    );
    assert_eq!(
        session
            .mutate_world_creature(loot_guid, |creature| {
                creature.corpse_despawn_deadline_ms_like_cpp()
            })
            .unwrap(),
        Some(corpse_despawn_before),
        "releasing partial loot must not change the existing corpse timer"
    );
}
#[tokio::test]
async fn creature_owned_loot_release_partial_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);
    let loot_guid = test_creature_guid(19_113);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let corpse_despawn_before = session
        .mutate_world_creature(loot_guid, |creature| {
            creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .unwrap()
        .expect("C++ arms corpse removal when the creature reaches JUST_DIED");
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid, other_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(7, 0))
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(other_guid),
        Some(&CreatureOwnedLoot::new(7, 0))
    );
    assert!(!canonical.is_fully_looted_like_cpp());
    assert!(session.reconcile_represented_loot_cache_like_cpp(loot_guid, player_guid));
    assert_eq!(session.loot_table[&loot_guid].coins, 7);
    assert!(session.loot_table.contains_key(&loot_guid));
    assert_eq!(
        session
            .mutate_world_creature(loot_guid, |creature| {
                creature.corpse_despawn_deadline_ms_like_cpp()
            })
            .unwrap(),
        Some(corpse_despawn_before),
        "releasing partial canonical loot must not change the existing corpse timer"
    );
}
#[tokio::test]
async fn creature_owned_loot_release_fully_consumed_uses_canonical_is_fully_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_114);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |creature| {
        creature.creature.set_corpse_delay(120, false);
    });
    assert_eq!(
        session
            .mutate_world_creature(loot_guid, |creature| creature.corpse_delay_secs_like_cpp())
            .unwrap(),
        120
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
    let corpse_despawn_at = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("fully looted corpse should start decay timer");
    let remaining = corpse_despawn_at.saturating_duration_since(Instant::now());
    assert!(
        (55..=60).contains(&remaining.as_secs()),
        "C++ uses corpse_delay * Rate.Corpse.Decay.Looted; got {remaining:?}"
    );
}
#[tokio::test]
async fn personal_creature_release_starts_decay_only_after_every_pool_is_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let first_player = ObjectGuid::create_player(1, 53);
    let second_player = ObjectGuid::create_player(1, 54);
    let owner_guid = test_creature_guid(19_119);
    let mut creature = make_canonical_creature_for_session(&session, owner_guid);

    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.allowed_looters = vec![first_player];
    let mut second_pool = authoritative_test_loot_like_cpp(0, true);
    second_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    second_pool.allowed_looters = vec![second_player];
    second_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        creature
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, first_pool), (second_player, second_pool),]),
            )
            .installed()
    );
    let authority = creature.loot_authority_like_cpp().clone();
    attach_canonical_creature(&mut session, creature);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    let corpse_deadline_before = session
        .mutate_world_creature(owner_guid, |creature| {
            creature.creature.set_corpse_delay(120, false);
            let deadline = Instant::now() + Duration::from_secs(120);
            creature.set_corpse_despawn_at(Some(deadline));
            creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .flatten()
        .expect("dead creature should already own its normal corpse deadline");

    session.set_player_guid(Some(first_player));
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, first_player));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        first_player,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, first_player, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, first_player)
            .await
    );
    assert_eq!(
        session
            .mutate_world_creature(owner_guid, |creature| {
                creature.corpse_despawn_deadline_ms_like_cpp()
            })
            .flatten(),
        Some(corpse_deadline_before),
        "one empty personal pool must not start global corpse decay while a peer has loot"
    );
    assert!(!authority.is_fully_looted_like_cpp());

    session.set_player_guid(Some(second_player));
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, second_player));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        second_player,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, second_player, response);
    let claim = authority
        .reserve_item_like_cpp(second_player, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, second_player)
            .await
    );
    assert!(authority.is_fully_looted_like_cpp());
    let (corpse_deadline_after, runtime_elapsed_ms) = session
        .mutate_world_creature(owner_guid, |creature| {
            creature
                .corpse_despawn_deadline_ms_like_cpp()
                .map(|deadline| (deadline, creature.runtime_elapsed_ms_like_cpp()))
        })
        .flatten()
        .expect("last personal pool should start looted-corpse decay");
    assert!(corpse_deadline_after < corpse_deadline_before);
    let remaining_ms = corpse_deadline_after.saturating_sub(runtime_elapsed_ms);
    assert!((55_000..=60_000).contains(&remaining_ms));
}
#[test]
fn creature_loot_release_dynamic_flags_are_viewer_dependent_like_cpp() {
    let mut session = make_session();
    let first_player = ObjectGuid::create_player(1, 61);
    let second_player = ObjectGuid::create_player(1, 62);
    let unrelated_player = ObjectGuid::create_player(1, 63);
    let owner_guid = test_creature_guid(19_120);
    let mut creature = make_canonical_creature_for_session(&session, owner_guid);

    let mut consumed_pool = authoritative_test_loot_like_cpp(0, false);
    consumed_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    consumed_pool.allowed_looters = vec![first_player];
    let mut live_pool = authoritative_test_loot_like_cpp(0, true);
    live_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    live_pool.allowed_looters = vec![second_player];
    live_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        creature
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, consumed_pool), (second_player, live_pool),]),
            )
            .installed()
    );
    let authority = creature.loot_authority_like_cpp().clone();
    attach_canonical_creature(&mut session, creature);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    session.set_player_guid(Some(first_player));

    let update = UnitDataValuesDeltaUpdate {
        object_data: Some(ObjectDataValuesUpdate {
            changed_object_type_mask: 1,
            object_data_mask: 1 << 2,
            entry_id: 0,
            dynamic_flags: UnitDynFlags::Lootable as u32,
            scale: 1.0,
        }),
        ..UnitDataValuesDeltaUpdate::default()
    };
    let dynamic_flags_for = |viewer_guid| {
        session
            .creature_loot_release_values_for_viewer_like_cpp(
                owner_guid,
                viewer_guid,
                false,
                Some(&authority),
                update.clone(),
            )
            .object_data
            .unwrap()
            .dynamic_flags
    };

    assert_eq!(dynamic_flags_for(first_player), 0);
    assert_eq!(
        dynamic_flags_for(second_player),
        UnitDynFlags::Lootable as u32,
        "one exhausted personal pool must not hide another player's live loot"
    );
    assert_eq!(dynamic_flags_for(unrelated_player), 0);
}
#[test]
fn creature_loot_visibility_applies_full_cpp_allowed_to_loot_gate() {
    let round_robin_owner = ObjectGuid::create_player(1, 64);
    let other_player = ObjectGuid::create_player(1, 65);
    let mut loot = authoritative_test_loot_like_cpp(0, true);
    loot.loot_method = LOOT_METHOD_ROUND_ROBIN_LIKE_CPP;
    loot.round_robin_player = round_robin_owner;
    loot.allowed_looters = vec![round_robin_owner, other_player];
    loot.items[0].allowed_looters = vec![round_robin_owner, other_player];
    loot.items[0].flags.follow_loot_rules = true;

    assert!(creature_loot_is_allowed_to_player_like_cpp(
        true,
        false,
        &loot,
        round_robin_owner,
    ));
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(true, false, &loot, other_player),
        "ordinary shared round-robin loot belongs only to the selected player"
    );
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(false, false, &loot, round_robin_owner,),
        "C++ rejects loot visibility for a living creature"
    );
    assert!(
        !creature_loot_is_allowed_to_player_like_cpp(true, true, &loot, round_robin_owner,),
        "C++ HasPendingBind suppresses loot visibility"
    );

    loot.items[0].flags.follow_loot_rules = false;
    assert!(
        creature_loot_is_allowed_to_player_like_cpp(true, false, &loot, other_player),
        "quest/conditional/free-for-player loot remains visible outside round robin"
    );
}
#[tokio::test]
async fn creature_loot_release_command_retries_without_blocking_source_like_cpp() {
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .unwrap();
    let creature_guid = test_creature_guid(19_121);
    assert_eq!(
        queue_creature_loot_release_command_reliably_like_cpp(
            &command_tx,
            SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(
                crate::session::mailbox::SendCreatureLootReleaseValuesUpdateLikeCppCommand {
                    creature_guid,
                    map_id: 0,
                    instance_id: 0,
                    unit_values_update: UnitDataValuesDeltaUpdate::default(),
                    authority: None,
                },
            ),
        ),
        CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying,
        "a full peer queue must schedule retry without blocking this session"
    );
    assert!(matches!(
        command_rx.recv().unwrap(),
        SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp
    ));
    let queued = tokio::time::timeout(Duration::from_secs(1), command_rx.recv_async())
        .await
        .expect("detached retry should enqueue after capacity opens")
        .unwrap();
    assert!(matches!(
        queued,
        SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(command)
            if command.creature_guid == creature_guid
    ));
}
#[tokio::test]
async fn creature_owned_loot_release_does_not_extend_expired_corpse_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_118);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));
    let expired = Instant::now() - Duration::from_secs(1);
    let deadline_before = session
        .mutate_world_creature(loot_guid, |creature| {
            creature.creature.set_corpse_delay(0, false);
            creature.creature.mark_ai_dead(0);
            creature.creature.set_corpse_delay(120, false);
            creature.set_corpse_despawn_at(Some(expired));
            creature.corpse_despawn_deadline_ms_like_cpp().unwrap()
        })
        .unwrap();
    assert!(
        session
            .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_due_like_cpp())
            .unwrap()
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(!session.loot_table.contains_key(&loot_guid));
    let deadline_after = session
        .mutate_world_creature(loot_guid, |creature| {
            creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .unwrap()
        .expect("expired corpse must retain its existing lifecycle deadline");
    assert_eq!(deadline_after, deadline_before);
    assert!(
        session
            .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_due_like_cpp())
            .unwrap()
    );
}
#[tokio::test]
async fn creature_owned_loot_release_fully_consumed_removes_lootable_dynflag_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_116);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.client_visible_guids_like_cpp.insert(loot_guid);
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |creature| {
        creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    });
    assert!(
        session
            .mutate_world_creature(loot_guid, |creature| creature
                .has_lootable_dynamic_flag_like_cpp())
            .unwrap()
    );
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx),
        vec![
            wow_constants::ServerOpcodes::LootRelease as u16,
            wow_constants::ServerOpcodes::UpdateObject as u16,
        ],
        "C++ ForceUpdateFieldChange must become a visible VALUES update so the client removes the loot cursor"
    );
    assert!(
        !session
            .mutate_world_creature(loot_guid, |creature| creature
                .has_lootable_dynamic_flag_like_cpp())
            .unwrap(),
        "C++ LootHandler::DoLootRelease removes UNIT_DYNFLAG_LOOTABLE when the creature is fully looted"
    );
}
#[tokio::test]
async fn creature_skinning_loot_release_despawns_corpse_immediately_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_115);
    let creature = make_canonical_creature_for_session(&session, loot_guid);
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    let _ = session.mutate_world_creature(loot_guid, |creature| {
        creature.creature.set_corpse_delay(120, false);
    });
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_SKINNING_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(loot_guid))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let corpse_despawn_at = session
        .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
        .unwrap()
        .expect("skinned corpse should start decay timer");
    let remaining = corpse_despawn_at.saturating_duration_since(Instant::now());
    assert_eq!(
        remaining.as_secs(),
        0,
        "C++ sets m_corpseRemoveTime = now for fully looted LOOT_SKINNING; got {remaining:?}"
    );
}
#[tokio::test]
async fn authoritative_partial_personal_creature_release_drops_cache_and_reopen_rehydrates_like_cpp()
 {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;
    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let before_release = authority
        .snapshot_for_player_like_cpp(fixture.first_tapper)
        .unwrap();
    let slot = before_release
        .loot
        .items
        .iter()
        .find(|item| item.item_id == fixture.normal_item_id)
        .unwrap()
        .loot_list_id;
    assert!(
        fixture
            .session
            .reconcile_represented_loot_cache_like_cpp(fixture.owner_guid, fixture.first_tapper,)
    );
    let opened = authority
        .add_viewer_like_cpp(fixture.first_tapper)
        .expect("the authoritative personal pool opens");
    fixture.session.set_active_loot_guid(fixture.owner_guid);
    fixture
        .session
        .active_loot_view_generations_like_cpp
        .insert(fixture.owner_guid, opened.generation);
    fixture
        .session
        .active_loot_view_authorities_like_cpp
        .insert(fixture.owner_guid, authority.clone());

    assert!(
        fixture
            .session
            .do_loot_release_owner_like_cpp(fixture.owner_guid, fixture.first_tapper)
            .await
    );
    assert!(!fixture.session.loot_table.contains_key(&fixture.owner_guid));
    assert!(
        !fixture
            .session
            .represented_loot_cache_generations_like_cpp
            .contains_key(&fixture.owner_guid)
    );
    assert!(
        !fixture
            .session
            .represented_personal_loot_money
            .contains_key(&(fixture.owner_guid, fixture.first_tapper))
    );
    let after_release = authority
        .snapshot_for_player_like_cpp(fixture.first_tapper)
        .unwrap();
    assert_eq!(after_release.generation, before_release.generation);
    assert_eq!(after_release.scope, before_release.scope);
    assert_eq!(after_release.loot.coins, before_release.loot.coins);
    assert_eq!(after_release.loot.items, before_release.loot.items);
    assert!(after_release.loot.players_looting.is_empty());
    assert!(
        after_release.loot.looted_by_player,
        "C++ keeps the per-Loot was-opened state after closing the viewer"
    );

    let response = fixture
        .session
        .represented_loot_response_for_owner_like_cpp(
            fixture.owner_guid,
            fixture.first_tapper,
            false,
        )
        .await
        .expect("the creature authority rehydrates a personal view");
    assert_eq!(response.coins, 7);
    assert!(
        fixture
            .session
            .represented_personal_loot_owners
            .contains(&fixture.owner_guid)
    );
    assert_eq!(
        fixture
            .session
            .represented_personal_loot_money
            .get(&(fixture.owner_guid, fixture.first_tapper)),
        Some(&7)
    );
    authority
        .reserve_item_like_cpp(fixture.first_tapper, slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .reserve_item_like_cpp(fixture.first_tapper, slot)
            .await
            .is_err()
    );
}
