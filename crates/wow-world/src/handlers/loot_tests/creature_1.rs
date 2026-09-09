//! Creature scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn creature_spell_cast_command_sends_start_then_basic_go_after_one_gate_like_cpp() {
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xBB,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}
#[tokio::test]
async fn advanced_combat_logging_receives_the_committed_full_creature_spell_go_like_cpp() {
    // The producer committed the full combat-log frame for this receiver.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    session.represented_set_advanced_combat_logging_like_cpp(true);
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xCC,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("full GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}
#[tokio::test]
async fn creature_spell_go_keeps_the_committed_frame_after_a_preference_toggle_like_cpp() {
    // C++ chooses the combat-log representation while distributing the cast,
    // so toggling advanced logging before this drain must not retroactively
    // change the frame an earlier cast committed.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xBB,
    );
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");
    session.represented_set_advanced_combat_logging_like_cpp(true);

    session
        .process_represented_session_commands_like_cpp()
        .await;

    let _start = send_rx.try_recv().expect("START frame");
    assert_eq!(
        send_rx.try_recv().expect("GO frame"),
        expected_go,
        "the committed basic frame survives a later advanced-logging toggle"
    );
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}
#[tokio::test]
async fn creature_spell_cast_honors_commit_time_visibility_after_exit_like_cpp() {
    // C++ picks recipients synchronously inside `SendSpellGo`, so a
    // visibility exit between that commit and this drain cannot retract a
    // pair the viewer was already selected for.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        session.client_visible_guids_like_cpp.clone(),
        0xBB,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");
    assert!(
        session.client_visible_guids_like_cpp.remove(&source_guid),
        "the caster leaves the client's visible set before the drain"
    );

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}
#[tokio::test]
async fn creature_spell_cast_rejects_command_committed_for_another_session_like_cpp() {
    // A replaced session owns a fresh `HaveAtClient` allocation, so a pair
    // committed against the previous incarnation must not be delivered even
    // when the caster is visible again.
    let (mut session, send_rx, source_guid) = make_visible_creature_spell_session_like_cpp();
    let previous_incarnation = crate::session::mailbox::SharedClientVisibleGuidsLikeCpp::default();
    previous_incarnation.insert(source_guid);
    assert!(
        !session
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&previous_incarnation),
        "the fixture models two distinct session incarnations"
    );
    let command = creature_spell_cast_command_like_cpp(source_guid, previous_incarnation, 0xBB);
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "a command committed for another incarnation delivers nothing"
    );
}
#[test]
fn creature_generated_loot_entry_uses_item_template_addon_follow_loot_rules_like_cpp() {
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 1,
        loot_list_id: 7,
        random_properties_id: -77,
        random_properties_seed: 456,
        context: ItemContext::DungeonNormal as u8,
        store_item_context: LootStoreItemContext {
            store_kind: LootStoreKind::Creature,
            entry: 100,
            item: LootStoreItem {
                item_id: 25,
                reference: 0,
                chance: 100.0,
                needs_quest: true,
                loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 0,
                min_count: 1,
                max_count: 1,
            },
        },
        free_for_all: false,
        follow_loot_rules: false,
        needs_quest: true,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    };

    let default_entry = generated_creature_loot_item_to_entry_like_cpp(
        generated,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    );
    assert!(!default_entry.flags.follow_loot_rules);
    assert_eq!(default_entry.loot_list_id, 7);
    assert_eq!(default_entry.random_properties_id, -77);
    assert_eq!(default_entry.random_properties_seed, 456);
    assert_eq!(default_entry.item_context, ItemContext::DungeonNormal as u8);

    let follow_entry = generated_creature_loot_item_to_entry_like_cpp(
        generated,
        ItemTemplateAddonLootMetadataLikeCpp {
            flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
            quest_log_item_id: 0,
        },
    );
    assert!(follow_entry.flags.follow_loot_rules);
    assert!(follow_entry.flags.needs_quest);
}
#[test]
fn stale_player_map_key_does_not_rebind_creature_loot_authorities_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_709);
    let owner_guid = test_creature_guid(61_710);
    session.set_player_guid(Some(player_guid));
    session.set_state(SessionState::LoggedIn);
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    let canonical_creature = make_canonical_creature_for_session(&session, owner_guid);
    attach_canonical_creature(&mut session, canonical_creature);

    let (map_id, instance_id) = session.current_legacy_runtime_map_key_like_cpp();
    let map_key = wow_map::MapKey::new(u32::from(map_id), instance_id);
    let legacy_before = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .expect("the legacy creature owns its pristine authority");
    let canonical_before = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .expect("the canonical creature owns a separate pristine authority");
    assert!(!legacy_before.shares_storage_like_cpp(&canonical_before));
    assert_eq!(session.current_canonical_player_map_key_like_cpp(), None);

    assert!(
        session
            .represented_owned_loot_authority_like_cpp(owner_guid)
            .is_none(),
        "a logged-in player between maps must fail closed"
    );

    let legacy_after = session
        .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .unwrap();
    let canonical_after = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
        .unwrap();
    assert!(legacy_after.shares_storage_like_cpp(&legacy_before));
    assert!(canonical_after.shares_storage_like_cpp(&canonical_before));
    assert!(
        !legacy_after.shares_storage_like_cpp(&canonical_after),
        "reconciliation must not mutate either stale-map mirror"
    );
}
#[tokio::test]
async fn durable_item_completion_never_auto_releases_creature_or_gameobject_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 61_820);
    session.set_player_guid(Some(player_guid));
    for owner_guid in [
        test_creature_guid(61_821),
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, 61_822),
    ] {
        session.set_active_loot_guid(owner_guid);
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: owner_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: vec![represented_loot_entry(0, 25, player_guid)],
                looted_by_player: false,
            },
        );
        let guard = session.begin_durable_item_loot_persistence_like_cpp();
        super::super::spawn_loot_claim_persistence_worker_like_cpp(
            async { Ok::<(), ()>(()) },
            None,
            Some((
                guard,
                DurableItemLootCompletionLikeCpp {
                    owner_guid,
                    loot_list_id: 0,
                    player_guid,
                    item_owner_auto_release: false,
                    durable_item_money_applied_amount: None,
                    durable_item_money_notified_amount: None,
                    durable_item_money_balance_applied: None,
                    item_fanout: None,
                    runtime_inventory_applied: Arc::new(AtomicBool::new(true)),
                },
            )),
        )
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    }

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(!session.is_disconnecting());
    assert!(session.loot_table.values().all(|loot| !loot.items[0].taken));
    assert!(
        !drain_server_opcodes_like_cpp(&send_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRelease as u16))
    );
}
#[tokio::test]
async fn disconnect_runs_full_creature_release_lifecycle_after_persistence_like_cpp() {
    let (mut session, _send_rx, _second, _second_rx, owner_guid, _player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, false,
        ));
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    let before = session
        .mutate_world_creature(owner_guid, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.set_corpse_despawn_at(Some(Instant::now() + Duration::from_secs(120)));
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
            (
                creature.corpse_despawn_at(),
                creature.has_lootable_dynamic_flag_like_cpp(),
            )
        })
        .unwrap();
    assert!(before.1);

    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;

    let after = session
        .mutate_world_creature(owner_guid, |creature| {
            (
                creature.corpse_despawn_at(),
                creature.has_lootable_dynamic_flag_like_cpp(),
            )
        })
        .unwrap();
    assert!(!after.1);
    assert!(after.0 <= before.0);
}
#[tokio::test]
async fn quest_required_creature_loot_is_not_generated_after_completion_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let quest_id = 8_336;
    let item_id = 20_482;
    let loot_id = 15_274;
    session.set_player_guid(Some(player_guid));
    install_limited_test_item_template(&mut session, item_id, 0);
    install_quest_bound_loot_objective_like_cpp(&mut session, quest_id, item_id, 6, 6);
    session.player_quests.get_mut(&quest_id).unwrap().status =
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP;

    let mut creature_store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: true,
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
    stores.insert(LootStoreKind::Creature, creature_store);
    session.set_loot_stores(Arc::new(stores));

    let complete_loot = session
        .generate_represented_creature_loot_items_for_player_like_cpp(loot_id, player_guid)
        .await
        .unwrap();
    assert!(
        complete_loot.is_empty(),
        "C++ LootItem::AllowedForPlayer rejects QuestRequired items after HasQuestForItem becomes false"
    );

    let status = session.player_quests.get_mut(&quest_id).unwrap();
    status.status = crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
    status.objective_counts[0] = 5;
    let incomplete_loot = session
        .generate_represented_creature_loot_items_for_player_like_cpp(loot_id, player_guid)
        .await
        .unwrap();
    assert_eq!(incomplete_loot.len(), 1);
    assert!(incomplete_loot[0].flags.needs_quest);
}
#[tokio::test]
async fn represented_creature_money_uses_cpp_money_drop_rate() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(1);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        money: 2.5,
        ..LootDropRatesLikeCpp::default()
    });

    let loot = session
        .generate_represented_creature_loot_like_cpp(
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            100,
            100,
            0,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 250);
    assert!(loot.items.is_empty());
}
#[tokio::test]
async fn represented_creature_money_zero_gold_max_stays_zero_like_cpp() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(1);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let loot = session
        .generate_represented_creature_loot_like_cpp(
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            0,
            0,
            0,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 0);
    assert!(loot.items.is_empty());
}
#[tokio::test]
async fn represented_creature_loot_generation_carries_cpp_dungeon_encounter_id() {
    let mut session = make_session();
    let owner_guid = test_creature_guid(19_097);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    let loot = session
        .generate_represented_creature_loot_like_cpp(
            owner_guid,
            ObjectGuid::create_player(1, 42),
            10,
            25,
            0,
            0,
            0,
            615,
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.dungeon_encounter_id, 615);
}
#[tokio::test]
async fn overworld_creature_builds_independent_personal_loot_per_connected_tapper_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();

    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;

    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .expect("the dead creature keeps its object-owned loot authority");
    let (first_normal_slot, second_normal_slot) =
        assert_overworld_personal_loot_generation_like_cpp(&authority, &fixture);
    assert_overworld_personal_loot_claims_are_independent_like_cpp(
        &authority,
        fixture.first_tapper,
        first_normal_slot,
        fixture.second_tapper,
        second_normal_slot,
    )
    .await;
}
#[tokio::test]
async fn cmsg_loot_unit_never_regenerates_after_creature_clear_loot_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .ensure_represented_creature_kill_loot_like_cpp(fixture.owner_guid)
        .await;
    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    fixture
        .session
        .mutate_world_creature(fixture.owner_guid, |creature| {
            creature.creature.clear_loot_like_cpp();
        });
    let retired_generation = authority.generation_like_cpp();

    assert!(
        fixture
            .session
            .represented_loot_response_for_owner_like_cpp(
                fixture.owner_guid,
                fixture.first_tapper,
                false,
            )
            .await
            .is_none()
    );
    assert!(authority.is_retired_like_cpp());
    assert_eq!(authority.generation_like_cpp(), retired_generation);
    assert!(
        authority
            .snapshot_for_player_like_cpp(fixture.first_tapper)
            .is_none()
    );
}
#[test]
fn stale_kill_generator_cannot_install_after_creature_lifecycle_aba_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    let authority = fixture
        .session
        .represented_owned_loot_authority_like_cpp(fixture.owner_guid)
        .unwrap();
    let expected_generation = authority.generation_like_cpp();
    let expected_revision = fixture
        .session
        .represented_creature_loot_state_like_cpp(fixture.owner_guid)
        .unwrap()
        .loot_lifecycle_revision;
    let mut stale_pool = authoritative_test_loot_like_cpp(0, true);
    stale_pool.loot_guid = represented_loot_object_guid_like_cpp(fixture.owner_guid);
    stale_pool.allowed_looters = vec![fixture.first_tapper];
    stale_pool.items[0].allowed_looters = vec![fixture.first_tapper];

    fixture
        .session
        .mutate_world_creature(fixture.owner_guid, |creature| {
            creature.creature.clear_loot_like_cpp();
            creature
                .creature
                .set_death_state_runtime(wow_constants::DeathState::JustRespawned, 0);
            creature
                .creature
                .set_death_state_runtime(wow_constants::DeathState::JustDied, 0);
        });

    assert!(
        !fixture
            .session
            .install_represented_creature_kill_loot_if_current_like_cpp(
                fixture.owner_guid,
                &authority,
                expected_generation,
                expected_revision,
                None,
                HashMap::from([(fixture.first_tapper, stale_pool)]),
            )
    );
    assert!(authority.is_retired_like_cpp());
    assert!(
        authority
            .snapshot_for_player_like_cpp(fixture.first_tapper)
            .is_none()
    );
}
#[tokio::test]
async fn represented_creature_loot_captures_group_method_master_and_round_robin_like_cpp() {
    let mut session = make_session();
    let master_guid = ObjectGuid::create_player(1, 42);
    let candidate_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = test_creature_guid(19_049);
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);
    install_master_loot_group(&mut session, master_guid, candidate_guid);

    let loot = session
        .generate_represented_creature_loot_like_cpp(owner_guid, master_guid, 10, 25, 0, 0, 0, 0)
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
    assert_eq!(loot.loot_master, master_guid);
    assert_eq!(loot.round_robin_player, master_guid);
}
#[tokio::test]
async fn loot_unit_live_creature_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_006);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_unit_non_creature_guid_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_019);
    session.set_player_guid(Some(player_guid));

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
}
#[tokio::test]
async fn loot_unit_creature_too_far_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_016);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let mut creature = test_creature(loot_guid, false);
    creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
    register_test_creature_like_cpp(&mut session, creature);

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
}
#[tokio::test]
async fn loot_money_non_allowed_active_creature_does_not_take_coins_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_100);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

    session.handle_loot_money(loot_money_packet()).await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.player_gold_like_cpp(), 0);
    assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 7);
}
#[tokio::test]
async fn loot_item_creature_too_far_uses_cpp_error() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_008);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);

    let mut creature = test_creature(loot_guid, false);
    creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
    register_test_creature_like_cpp(&mut session, creature);
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
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_item_creature_distance_can_use_canonical_map_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_018);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(loot_guid);
    attach_canonical_map_object(
        &mut session,
        AccessorObjectKind::Creature,
        canonical_world_object(loot_guid, 0, Position::new(31.0, 0.0, 0.0, 0.0)),
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
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_TOO_FAR_LIKE_CPP
    );
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_item_creature_pickup_refreshes_canonical_owned_loot_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_115);
    let mut creature = make_canonical_creature_for_session(&session, loot_guid);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(0, 1));
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![represented_loot_entry(0, 25, player_guid)],
            looted_by_player: false,
        },
    );

    mark_loot_item_looted_for_player_like_cpp(
        session.loot_table.get_mut(&loot_guid).unwrap(),
        0,
        player_guid,
    );
    session.refresh_represented_loot_owner_canonical_summary_like_cpp(loot_guid, player_guid);

    let loot = session.loot_table.get(&loot_guid).unwrap();
    assert!(loot.items[0].is_looted_for_player_like_cpp(player_guid));
    assert_eq!(loot.unlooted_count, 0);
    let canonical = canonical_creature_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
}
#[tokio::test]
async fn loot_item_missing_creature_uses_cpp_no_loot_error() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_009);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
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
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    assert_eq!(
        loot_response_failure_reason(&sent),
        LOOT_ERROR_NO_LOOT_LIKE_CPP
    );
    assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(loot_guid));
}
