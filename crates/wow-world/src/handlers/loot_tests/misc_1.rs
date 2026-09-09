//! Misc scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn stale_release_keeps_replacement_viewer_and_pool_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let mut replacement = authoritative_test_loot_like_cpp(13, true);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    authority.add_viewer_like_cpp(first_guid).unwrap();

    assert!(
        first
            .do_loot_release_owner_like_cpp(owner, first_guid)
            .await
    );

    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert_eq!(snapshot.loot.coins, 13);
    assert!(!snapshot.loot.items[0].taken);
    assert!(snapshot.loot.players_looting.contains(&first_guid));
    assert!(!first.active_loot_view_owners.contains(&owner));
}
#[tokio::test]
async fn remote_master_timeout_then_release_still_fans_out_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The target already closed its own view. The source will close while
    // the detached target worker owns the claim as `Persisting`.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    second.set_loot_item_store_test_commit_gate_like_cpp(Arc::clone(&commit_gate));

    let mut request = Box::pin(first.request_represented_remote_master_loot_give_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        entry,
        Some(claim),
    ));
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        drain_server_opcodes_like_cpp(&first_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16)),
        "the pre-COMMIT route survives request timeout and CMSG_LOOT_RELEASE"
    );
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "completion must run AllLootRemovedFromCorpse without an active view"
    );
}
#[tokio::test]
async fn detached_remote_claim_waits_for_every_authority_viewer_before_corpse_lifecycle_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first
        .mutate_world_creature(owner, |creature| {
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The remote winner closes, while the original looter deliberately
    // keeps the same authoritative Loot window open.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let player_registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    player_registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&player_registry));
    second.set_player_registry(player_registry);
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    let request = first.request_represented_remote_loot_roll_winner_store_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        vec![entry],
        false,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert!(
        first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "the detached winner cannot finish lifecycle while the original viewer remains open"
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .players_looting,
        vec![first_guid]
    );

    first.handle_loot_release(loot_release_packet(owner)).await;
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "the final real viewer release performs the ordinary C++ corpse transition"
    );
}
#[tokio::test]
async fn failed_disenchant_batch_grants_zero_and_original_slot_retries_like_cpp() {
    let (mut session, _rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(player_guid, 700);
    let grants = Arc::new(AtomicUsize::new(0));
    // Both material grants are already planned when this seam rejects the
    // one transaction, modelling a failure while persisting its second
    // output. Runtime observes neither output.
    session.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), false);

    assert!(
        !session
            .store_direct_disenchant_batch_like_cpp(
                &materials,
                0,
                Some(&claim),
                Some(LootItemClaimCommitContextLikeCpp {
                    owner_guid: owner,
                    loot_obj: represented_loot_object_guid_like_cpp(owner),
                    loot_list_id: 0,
                    player_guid,
                    free_for_all: false,
                }),
            )
            .await
    );
    assert_eq!(grants.load(Ordering::SeqCst), 0);
    assert!(!claim.is_committed_like_cpp());
    drop(claim);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_ok()
    );
}
#[tokio::test]
async fn remote_disenchant_timeout_then_release_fans_out_once_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    first
        .mutate_world_creature(owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(second_guid, 700);

    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, 700, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    second.set_loot_item_store_test_commit_gate_like_cpp(Arc::clone(&commit_gate));

    let mut request = Box::pin(
        first.request_represented_remote_loot_roll_winner_store_like_cpp(
            second_guid,
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            0,
            materials,
            true,
            Some(claim),
        ),
    );
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        second.process_represented_session_commands_like_cpp().await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!(grants.load(Ordering::SeqCst), 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(second_guid, 0)
            .await
            .is_err(),
        "the original roll claim remains terminal after every material commits"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
            .count(),
        1,
        "the material batch publishes the original roll removal exactly once"
    );
    assert!(
        !first
            .mutate_world_creature(owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "post-COMMIT completion must finish the corpse lifecycle without an active view"
    );
}
#[tokio::test]
async fn dungeon_encounter_builds_independent_unlocked_personal_pools_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
    let encounter_id = 733;
    fixture
        .session
        .mutate_world_creature(fixture.owner_guid, |creature| {
            creature.creature.ai_ownership_mut().dungeon_encounter_id = encounter_id;
        });
    fixture
        .session
        .represented_locked_dungeon_encounters
        .insert((fixture.second_tapper, encounter_id));

    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;

    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    assert!(personal.contains_key(&fixture.first_tapper));
    assert!(!personal.contains_key(&fixture.second_tapper));
    assert!(!personal.contains_key(&fixture.disconnected_tapper));
    let first = &personal[&fixture.first_tapper].loot;
    assert_eq!(first.dungeon_encounter_id, encounter_id);
    assert_eq!(first.allowed_looters, vec![fixture.first_tapper]);
}
#[tokio::test]
async fn represented_personal_encounter_locked_or_empty_late_player_does_not_install_like_cpp() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut locked, locked_rx) = make_session_with_send_capacity(8);
    let (mut empty, empty_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 242);
    let locked_player = ObjectGuid::create_player(1, 277);
    let empty_player = ObjectGuid::create_player(1, 288);
    let gameobject_guid = test_gameobject_guid(91_019);
    let personal_loot_id = 10_019;
    let item_id = 80_019;
    let encounter_id = 734;

    for (session, player) in [
        (&mut first, first_player),
        (&mut locked, locked_player),
        (&mut empty, empty_player),
    ] {
        session.set_player_guid(Some(player));
        session.set_player_position_like_cpp(Position::ZERO);
    }
    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    let manager = Arc::clone(first.canonical_map_manager.as_ref().unwrap());
    locked.set_canonical_map_manager(Arc::clone(&manager));
    empty.set_canonical_map_manager(manager);

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut locked, item_id, 0);
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: personal_loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    let stores = Arc::new(stores);
    first.set_loot_stores(Arc::clone(&stores));
    locked.set_loot_stores(stores);
    locked
        .represented_locked_dungeon_encounters
        .insert((locked_player, encounter_id));

    let source = GameObjectLootSource {
        loot_id: 0,
        dungeon_encounter_id: encounter_id,
        personal_loot_id,
        ..Default::default()
    };
    first
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let first_before = authority
        .snapshot_for_player_like_cpp(first_player)
        .unwrap();

    locked
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    empty
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(authority.personal_snapshots_like_cpp().len(), 1);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_player)
            .unwrap(),
        first_before
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(locked_player)
            .is_none()
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(empty_player)
            .is_none()
    );
    assert!(!locked.is_active_loot_guid(gameobject_guid));
    assert!(!empty.is_active_loot_guid(gameobject_guid));
    assert!(locked_rx.try_recv().is_err());
    assert!(empty_rx.try_recv().is_err());
}
#[tokio::test]
async fn represented_gathering_node_first_use_records_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_003);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 0,
        triggered_event_id: 123,
        xp_difficulty: 0,
        spell_id: 0,
        max_loots: 10,
        linked_trap_entry: 456,
    };

    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_003, source)
        .await;
    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_003, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 123,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 456,
            },
        ]
    );
}
#[tokio::test]
async fn represented_fishing_hole_updates_catch_criteria_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_005);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    session
        .open_represented_fishing_hole_like_cpp(gameobject_guid, 190_000, 123)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated {
                gameobject_guid,
                player_guid,
                gameobject_entry: 190_000,
            }
        ]
    );
}
#[tokio::test]
async fn represented_gathering_node_runtime_state_matches_cpp_side_effects() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_007);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GatheringNodeUseSource {
        loot_id: 0,
        despawn_delay_secs: 15,
        triggered_event_id: 0,
        xp_difficulty: 0,
        spell_id: 777,
        max_loots: 1,
        linked_trap_entry: 0,
    };

    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_007, source)
        .await;
    session
        .open_represented_gathering_node_like_cpp(gameobject_guid, 190_007, source)
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("represented gathering use records GO state");
    assert_eq!(state.personal_loot_uses, 1);
    assert_eq!(state.go_state, Some(GoState::Active));
    assert_eq!(
        state.dynamic_flags & GO_DYNFLAG_LO_NO_INTERACT,
        GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.loot_state, Some(LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
    assert_eq!(state.despawn_delay_secs, Some(15));
    assert!(state.despawn_delay_until.is_some());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 190_007,
                spell_id: 777,
                go_type: GAMEOBJECT_TYPE_GATHERING_NODE,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: player_guid,
                spell_id: 777,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
}
#[tokio::test]
async fn gathering_node_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_009);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(
            SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
                    gameobject_guid,
                    map_id: 571,
                    instance_id: 0,
                    go_type: wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8,
                    loot_state: Some(wow_entities::LootState::Activated as u8),
                    loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                    go_state: Some(wow_entities::GoState::Active as i8),
                    dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                    gathering_node_loot_id: Some(190_009),
                    personal_loot_uses: 1,
                    linked_trap_entry: Some(191_009),
                    linked_trap_guid: Some(test_gameobject_guid(91_010)),
                },
            ),
        )
        .expect("command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .expect("synced gathering node state");
    assert_eq!(
        state.go_type,
        Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8)
    );
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert_eq!(
        state.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.gathering_node_loot_id, Some(190_009));
    assert_eq!(state.personal_loot_uses, 1);
    assert_eq!(state.linked_trap_entry, Some(191_009));
    assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_010)));
}
#[test]
fn historical_stack_binding_adds_only_soulbound_like_cpp() {
    let mut session = make_session();
    let item_id = 25;
    let historical = Item::new(0);

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    let flags = session.stored_existing_item_dynamic_flags_like_cpp(
        item_id,
        INVENTORY_SLOT_ITEM_START,
        &historical,
    );
    assert_eq!(flags, ItemFieldFlags::SOULBOUND.bits());
    assert_eq!(flags & ItemFieldFlags::NEW_ITEM.bits(), 0);

    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnEquip,
    );
    assert_eq!(
        session.stored_existing_item_dynamic_flags_like_cpp(
            item_id,
            wow_entities::INVENTORY_SLOT_BAG_START,
            &historical,
        ),
        ItemFieldFlags::SOULBOUND.bits(),
        "C++ binds an OnEquip item when that item is stored in a bag-equipment position"
    );
    assert_eq!(
        session.stored_existing_item_dynamic_flags_like_cpp(
            item_id,
            INVENTORY_SLOT_ITEM_START,
            &historical,
        ),
        0,
        "C++ does not bind an OnEquip stack in an ordinary backpack slot"
    );
}
#[tokio::test]
async fn failed_existing_stack_store_publishes_neither_count_nor_binding() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 77);
    let item_id = 25;
    session.set_player_guid(Some(player_guid));
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        90_000,
    )));
    install_limited_test_item_template_with_flags2_and_bonding(
        &mut session,
        item_id,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: item_id,
            db_guid: 77,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        item_id,
        player_guid,
        4,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(item);

    let (persistence, requests) = PlayerInventoryPersistencePortFixtureLikeCpp::new_like_cpp(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "fixture rollback".into(),
        },
    );
    session.set_player_inventory_persistence_port_like_cpp(persistence);

    let stored = session
        .store_direct_loot_item_like_cpp(
            &LootEntry {
                loot_list_id: 0,
                item_id,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            },
            0,
        )
        .await;

    assert!(!stored);
    let historical = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .expect("failed transaction keeps the historical stack");
    assert_eq!(historical.count(), 4);
    assert_eq!(historical.item_flags_bits(), 0);
    let requests = requests.lock().unwrap();
    let [wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootDirectItemGrant(request)] =
        requests.as_slice()
    else {
        panic!("direct loot must use its semantic inventory persistence variant");
    };
    assert_eq!(request.existing_stacks.len(), 1);
    assert_eq!(request.existing_stacks[0].item_guid, 77);
    assert_eq!(request.existing_stacks[0].new_count, 5);
    assert_eq!(
        request.existing_stacks[0].dynamic_flags,
        Some(ItemFieldFlags::SOULBOUND.bits())
    );
    assert!(request.new_stacks.is_empty());
    assert_eq!(request.stored_item_source, None);
}
#[test]
fn random_enchantment_selection_uses_cpp_weighted_chances() {
    let group = [
        ItemRandomEnchantmentTemplateEntry {
            group_id: 1,
            enchantment_id: 10,
            chance: 0.0,
        },
        ItemRandomEnchantmentTemplateEntry {
            group_id: 1,
            enchantment_id: 11,
            chance: 100.0,
        },
    ];
    assert_eq!(
        select_weighted_random_enchantment_like_cpp(&group, &mut StdRng::seed_from_u64(5)),
        Some(11)
    );
}
