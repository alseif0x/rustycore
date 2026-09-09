//! Gameobject scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn shared_gameobject_normal_item_with_two_looters_counts_once_like_cpp() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let mut entry = represented_loot_entry(0, 25, first);
    entry.allowed_looters = vec![first, second];
    let mut loot = CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(test_gameobject_guid(91_101)),
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: vec![entry],
        looted_by_player: false,
    };

    prepare_represented_shared_loot_generation_like_cpp(&mut loot, &[first, second]);

    assert_eq!(loot.allowed_looters, vec![first, second]);
    assert_eq!(loot.items[0].allowed_looters, vec![first, second]);
    assert_eq!(loot.unlooted_count, 1);
    assert!(loot.items[0].flags.counted);
}
#[test]
fn shared_gameobject_item_evaluates_each_looter_after_roll_like_cpp() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let store_item_context = LootStoreItemContext {
        store_kind: LootStoreKind::Reference,
        entry: 900,
        item: LootStoreItem {
            item_id: 25,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
    };
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 1,
        loot_list_id: 0,
        random_properties_id: 0,
        random_properties_seed: 0,
        context: ItemContext::None as u8,
        store_item_context,
        free_for_all: false,
        follow_loot_rules: true,
        needs_quest: false,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    };
    let mut evaluated = Vec::new();

    let entry = generated_shared_gameobject_loot_item_to_entry_like_cpp(
        generated,
        ItemTemplateAddonLootMetadataLikeCpp::default(),
        &[first, second],
        |context, looter| {
            evaluated.push((context, looter));
            looter == second
        },
    );

    assert_eq!(
        evaluated,
        vec![(store_item_context, first), (store_item_context, second)]
    );
    assert_eq!(entry.item_id, 25, "the rolled candidate remains present");
    assert_eq!(entry.allowed_looters, vec![second]);
}
#[test]
fn chest_allowed_looters_ignore_range_only_in_same_dungeon_instance_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    let mut member = broadcast_info(member_guid, member_tx.clone());
    member.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(member_guid, member, Default::default());
    session.set_player_registry(Arc::clone(&registry));

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
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(canonical);
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "LootOwner".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical loot owner map");
    install_group_loot_group(&mut session, player_guid, member_guid);
    assert_eq!(
        session.represented_group_looters_at_reward_distance_like_cpp(player_guid),
        vec![player_guid]
    );

    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
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
    assert_eq!(
        session.represented_group_looters_at_reward_distance_like_cpp(player_guid),
        vec![player_guid, member_guid]
    );

    let mut wrong_instance = broadcast_info(member_guid, member_tx);
    wrong_instance.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    wrong_instance.placement.instance_id = 1;
    registry.register_or_replace(member_guid, wrong_instance, Default::default());
    assert_eq!(
        session.represented_group_looters_at_reward_distance_like_cpp(player_guid),
        vec![player_guid]
    );
}
#[tokio::test]
async fn represented_gameobject_chest_loot_carries_cpp_source_metadata() {
    let mut session = make_session();
    let gameobject_guid = test_gameobject_guid(91_001);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            ObjectGuid::create_player(1, 42),
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.loot_type, LOOT_TYPE_CHEST_LIKE_CPP);
    assert_eq!(loot.dungeon_encounter_id, 733);
    assert_eq!(loot.loot_method, 0);
}
#[tokio::test]
async fn represented_gameobject_chest_uses_resolved_template_money_like_cpp() {
    let mut session = make_session();
    let gameobject_guid = test_gameobject_guid(91_022);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);

    let loot = session
        .generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
            gameobject_guid,
            ObjectGuid::create_player(1, 42),
            GameObjectLootSource::default(),
            &[],
            (123, 123),
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.coins, 123);
}
#[tokio::test]
async fn represented_non_encounter_personal_chest_keeps_two_session_pools_independent_like_cpp() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut second, _second_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 42);
    let second_player = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_015);
    let personal_loot_id = 10_015;
    let item_id = 80_015;

    first.set_player_guid(Some(first_player));
    second.set_player_guid(Some(second_player));
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);

    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    second.set_canonical_map_manager(Arc::clone(
        first
            .canonical_map_manager
            .as_ref()
            .expect("both sessions share the canonical map owner"),
    ));

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut second, item_id, 0);
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
    second.set_loot_stores(stores);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: true,
        dungeon_encounter_id: 0,
        personal_loot_id,
        ..Default::default()
    };

    first
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    second
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .expect("canonical chest remains map-owned")
        .loot_authority_like_cpp()
        .clone();
    assert!(authority.shared_snapshot_like_cpp().is_none());
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 2);

    let first_pool = personal.get(&first_player).unwrap();
    let second_pool = personal.get(&second_player).unwrap();
    assert_ne!(first_pool.loot.loot_guid, second_pool.loot.loot_guid);
    assert_eq!(first_pool.loot.loot_method, 0);
    assert_eq!(second_pool.loot.loot_method, 0);
    for (player, pool) in [(first_player, first_pool), (second_player, second_pool)] {
        assert_eq!(pool.loot.allowed_looters, vec![player]);
        assert_eq!(pool.loot.items.len(), 1);
        assert_eq!(pool.loot.items[0].item_id, item_id);
        assert_eq!(pool.loot.items[0].allowed_looters, vec![player]);
    }

    let first_slot = first_pool.loot.items[0].loot_list_id;
    authority
        .reserve_item_like_cpp(first_player, first_slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_player)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        !authority
            .snapshot_for_player_like_cpp(second_player)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}
#[tokio::test]
async fn represented_empty_non_encounter_personal_chest_still_opens_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 141);
    let gameobject_guid = test_gameobject_guid(91_021);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 0,
                personal_loot_id: 10_021,
                ..Default::default()
            },
        )
        .await;

    let authority = canonical_gameobject_snapshot(&session, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let pool = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("C++ retains the empty non-encounter personal pool");
    assert!(loot_is_looted_like_cpp(&pool.loot));
    assert_eq!(pool.loot.allowed_looters, vec![player_guid]);
    assert!(session.is_active_loot_guid(gameobject_guid));
    let _response = recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
}
#[tokio::test]
async fn represented_empty_personal_encounter_chest_does_not_install_or_open_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_016);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 733,
                personal_loot_id: 10_016,
                ..Default::default()
            },
        )
        .await;

    let gameobject = canonical_gameobject_snapshot(&session, gameobject_guid).unwrap();
    assert!(gameobject.loot_authority_like_cpp().is_pristine_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .personal_snapshots_like_cpp()
            .is_empty()
    );
    assert!(!session.loot_table.contains_key(&gameobject_guid));
    assert!(
        !session
            .represented_personal_loot_owners
            .contains(&gameobject_guid)
    );
    assert!(
        !session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, player_guid))
    );
    assert!(!session.is_active_loot_guid(gameobject_guid));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn represented_nonempty_personal_encounter_chest_keeps_live_canonical_pool_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_017);
    let personal_loot_id = 10_017;
    let item_id = 80_017;
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    install_limited_test_item_template(&mut session, item_id, 0);
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
    session.set_loot_stores(Arc::new(stores));

    session
        .open_represented_gameobject_chest_like_cpp(
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 733,
                personal_loot_id,
                ..Default::default()
            },
        )
        .await;

    let gameobject = canonical_gameobject_snapshot(&session, gameobject_guid).unwrap();
    let pool = gameobject
        .loot_authority_like_cpp()
        .snapshot_for_player_like_cpp(player_guid)
        .expect("nonempty encounter pool remains map-owned");
    assert_eq!(pool.loot.allowed_looters, vec![player_guid]);
    assert_eq!(pool.loot.items.len(), 1);
    assert_eq!(pool.loot.items[0].item_id, item_id);
    assert!(!loot_is_looted_like_cpp(&pool.loot));
    assert!(session.is_active_loot_guid(gameobject_guid));
}
#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_uses_current_player_when_no_tap_list_like_cpp()
 {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_008);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            player_guid,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![player_guid]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![player_guid])
    );
    assert_eq!(loot.coins, 0);
    assert!(
        session
            .represented_personal_loot_owners
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, player_guid))
    );
}
#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_uses_tap_list_like_cpp() {
    let mut session = make_session();
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 77);
    let non_player_tapper = ObjectGuid::create_item(1, 900);
    let gameobject_guid = test_gameobject_guid(91_009);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    session.represented_gameobject_tap_lists.insert(
        gameobject_guid,
        vec![
            second_tapper,
            non_player_tapper,
            first_tapper,
            second_tapper,
        ],
    );
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            first_tapper,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![first_tapper, second_tapper]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![first_tapper, second_tapper])
    );
    assert_eq!(loot.coins, 0);
    assert!(
        session
            .represented_personal_loot_owners
            .contains(&gameobject_guid)
    );
    assert!(
        session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, first_tapper))
    );
    assert!(
        session
            .represented_personal_loot_money
            .contains_key(&(gameobject_guid, second_tapper))
    );
}
#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_skips_locked_tappers_like_cpp() {
    let mut session = make_session();
    let locked_tapper = ObjectGuid::create_player(1, 42);
    let open_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_010);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    session
        .represented_gameobject_tap_lists
        .insert(gameobject_guid, vec![locked_tapper, open_tapper]);
    session
        .represented_locked_dungeon_encounters
        .insert((locked_tapper, 733));
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = session
        .generate_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            locked_tapper,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![open_tapper]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![open_tapper])
    );
}
#[tokio::test]
async fn represented_gameobject_personal_encounter_open_does_not_auto_allow_non_tapper_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_011);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session
        .represented_gameobject_tap_lists
        .insert(gameobject_guid, vec![other_tapper]);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(gameobject_guid));
    assert_eq!(
        session
            .loot_table
            .get(&gameobject_guid)
            .unwrap()
            .allowed_looters,
        vec![other_tapper]
    );
}
#[tokio::test]
async fn represented_gameobject_personal_encounter_open_reads_player_money_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_012);
    let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);
    session.loot_table.insert(
        gameobject_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    session
        .represented_personal_loot_owners
        .insert(gameobject_guid);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, player_guid), 123);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    let mut response =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
    assert_eq!(response.read_packed_guid().unwrap(), gameobject_guid);
    assert_eq!(response.read_packed_guid().unwrap(), loot_object);
    // failure_reason: C++ LootResponse::FailureReason defaults to 17 (LOOT_ERROR_NO_LOOT,
    // LootPackets.h:72 "Most common value") and is left unset on a successful loot — the
    // client ignores it once the window opens. (Previously, wrongly asserted as 0.)
    assert_eq!(response.read_uint8().unwrap(), 17);
    assert_eq!(response.read_uint8().unwrap(), LOOT_TYPE_CHEST_LIKE_CPP);
    assert_eq!(response.read_uint8().unwrap(), 0);
    assert_eq!(response.read_uint8().unwrap(), 2);
    assert_eq!(response.read_uint32().unwrap(), 123);
}
#[tokio::test]
async fn represented_gameobject_personal_encounter_money_pickup_consumes_only_player_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_013);
    let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(gameobject_guid);
    session.loot_table.insert(
        gameobject_guid,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 999,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, other_tapper],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    session
        .represented_personal_loot_owners
        .insert(gameobject_guid);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, player_guid), 123);
    session
        .represented_personal_loot_money
        .insert((gameobject_guid, other_tapper), 456);

    session.handle_loot_money(loot_money_packet()).await;

    let mut notify =
        recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootMoneyNotify);
    assert_eq!(notify.read_uint64().unwrap(), 123);
    assert_eq!(
        session
            .represented_personal_loot_money
            .get(&(gameobject_guid, player_guid)),
        Some(&0)
    );
    assert_eq!(
        session
            .represented_personal_loot_money
            .get(&(gameobject_guid, other_tapper)),
        Some(&456)
    );
    assert_eq!(session.loot_table.get(&gameobject_guid).unwrap().coins, 999);
}
#[test]
fn represented_gameobject_personal_encounter_items_are_single_tapper_like_cpp() {
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 77);
    let mut loot = CreatureLoot {
        loot_guid: represented_loot_object_guid_like_cpp(test_gameobject_guid(91_014)),
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
        dungeon_encounter_id: 733,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![first_tapper, second_tapper],
        items: vec![
            LootEntry {
                loot_list_id: 0,
                item_id: 1_001,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![first_tapper, second_tapper],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            },
            LootEntry {
                loot_list_id: 1,
                item_id: 1_002,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags {
                    freeforall: true,
                    ..LootEntryFlags::default()
                },
                allowed_looters: vec![first_tapper, second_tapper],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: vec![first_tapper],
                taken: false,
            },
        ],
        looted_by_player: false,
    };
    let mut rng = StdRng::seed_from_u64(7);

    assign_represented_personal_loot_items_like_cpp(
        &mut loot,
        &[first_tapper, second_tapper],
        &mut rng,
    );

    assert_eq!(loot.unlooted_count, 2);
    assert_eq!(loot.items[0].allowed_looters.len(), 1);
    assert_eq!(loot.items[1].allowed_looters.len(), 1);
    assert!([first_tapper, second_tapper].contains(&loot.items[0].allowed_looters[0]));
    assert!([first_tapper, second_tapper].contains(&loot.items[1].allowed_looters[0]));
    assert!(loot.items[0].flags.counted);
    assert!(!loot.items[1].flags.counted);
    assert_eq!(loot.items[1].ffa_looted_by, Vec::<ObjectGuid>::new());
    assert_eq!(loot.player_ffa_items.len(), 1);
    assert_eq!(loot.player_ffa_items[0].1[0].loot_list_id, 1);
}
#[tokio::test]
async fn represented_gameobject_chest_first_generation_records_use_effects_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_002);
    session.set_player_guid(Some(player_guid));
    session
        .client_visible_guids_like_cpp
        .insert(gameobject_guid);

    let source = GameObjectLootSource {
        loot_id: 55,
        use_group_loot_rules: false,
        dungeon_encounter_id: 0,
        personal_loot_id: 0,
        push_loot_id: 0,
        triggered_event_id: 777,
        linked_trap_entry: 888,
        ..Default::default()
    };

    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    session
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 777,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 888,
            },
        ]
    );
}
