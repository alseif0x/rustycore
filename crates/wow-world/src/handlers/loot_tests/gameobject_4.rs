//! Gameobject scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn authoritative_partial_gameobject_release_drops_cache_and_reopen_rehydrates_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 451);
    let owner_guid = test_gameobject_guid(19_451);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(player_guid));
    let mut pool = authoritative_test_loot_like_cpp(11, true);
    pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    pool.allowed_looters = vec![player_guid];
    pool.items[0].allowed_looters = vec![player_guid];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(player_guid, pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        owner_guid,
        owner_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    let source = GameObjectLootSource {
        personal_loot_id: 55,
        chest_consumable: false,
        chest_restock_time_secs: 7,
        ..Default::default()
    };
    session.record_represented_gameobject_chest_release_metadata_like_cpp(owner_guid, source);
    assert!(session.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid));
    session.set_active_loot_guid(owner_guid);
    let response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        &session.loot_table[&owner_guid],
        player_guid,
    );
    session.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        session
            .do_loot_release_owner_like_cpp(owner_guid, player_guid)
            .await
    );
    assert!(!session.loot_table.contains_key(&owner_guid));
    assert!(
        !session
            .represented_loot_cache_generations_like_cpp
            .contains_key(&owner_guid)
    );
    assert!(
        !session
            .represented_personal_loot_money
            .contains_key(&(owner_guid, player_guid))
    );
    let before_reopen = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("release preserves the canonical personal pool");
    assert_eq!(before_reopen.loot.coins, 11);
    assert!(!before_reopen.loot.items[0].taken);

    session
        .open_represented_gameobject_chest_like_cpp(owner_guid, source)
        .await;
    assert!(session.loot_table.contains_key(&owner_guid));
    assert!(
        session
            .represented_personal_loot_owners
            .contains(&owner_guid)
    );
    assert_eq!(
        session
            .represented_personal_loot_money
            .get(&(owner_guid, player_guid)),
        Some(&11)
    );
    assert!(session.is_active_loot_guid(owner_guid));

    let slot = before_reopen.loot.items[0].loot_list_id;
    authority
        .reserve_item_like_cpp(player_guid, slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .reserve_item_like_cpp(player_guid, slot)
            .await
            .is_err(),
        "rehydration must not manufacture a second claim"
    );
}
#[tokio::test]
async fn personal_gameobject_release_deactivates_only_after_every_pool_is_looted_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let first_player = ObjectGuid::create_player(1, 51);
    let second_player = ObjectGuid::create_player(1, 52);
    let owner_guid = test_gameobject_guid(19_138);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(first_player));

    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
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
    second_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    second_pool.allowed_looters = vec![second_player];
    second_pool.items[0].allowed_looters = vec![second_player];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(
                None,
                HashMap::from([(first_player, first_pool), (second_player, second_pool),]),
            )
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_position_like_cpp(Position::ZERO);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        owner_guid,
        owner_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        owner_guid,
        GameObjectLootSource {
            personal_loot_id: 55,
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );

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
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::Activated,
        "one empty personal pool must not globally deactivate a chest while a peer has loot"
    );
    assert!(!authority.is_retired_like_cpp());
    assert!(!authority.is_fully_looted_like_cpp());
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&owner_guid)
            .unwrap()
            .per_player_state_player_guid,
        Some(first_player),
        "C++ still runs OnLootRelease for the selected empty personal pool"
    );

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
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::JustDeactivated,
        "the last empty personal pool must globally deactivate the chest"
    );
    assert!(authority.is_fully_looted_like_cpp());
    assert!(!authority.is_retired_like_cpp());

    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let mut manager = manager.lock().unwrap();
    manager
        .find_map_mut(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap()
        .map_mut()
        .update_game_object_like_cpp(owner_guid, 1, 0);
    drop(manager);
    assert!(
        authority.is_retired_like_cpp(),
        "the canonical JustDeactivated update must clear and retire the completed authority"
    );
}
#[test]
fn personal_gameobject_upsert_before_release_invalidates_global_deactivation_like_cpp() {
    let mut session = make_session();
    let first = ObjectGuid::create_player(1, 61_860);
    let late = ObjectGuid::create_player(1, 61_861);
    let owner_guid = test_gameobject_guid(61_862);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(first));
    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(first));
    authority.add_viewer_like_cpp(first).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;
    let close = authority
        .close_viewer_if_generation_like_cpp(generation, first)
        .unwrap();
    assert!(close.whole_object_fully_looted);

    let mut late_pool = authoritative_test_loot_like_cpp(0, true);
    late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    late_pool.allowed_looters = vec![late];
    late_pool.items[0].allowed_looters = vec![late];
    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_like_cpp(
                owner_guid, late, late_pool, false,
            )
            .is_some()
    );

    assert!(
        session
            .set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                owner_guid,
                &authority,
                close.object_generation,
                close.lifecycle_revision,
                LootState::JustDeactivated,
                None,
                0,
                false,
            )
            .is_none(),
        "the late pool revision must invalidate the earlier fully-looted observation"
    );
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::Activated
    );
    assert!(authority.snapshot_for_player_like_cpp(late).is_some());
}
#[test]
fn personal_gameobject_release_before_upsert_rejects_resurrection_like_cpp() {
    let mut session = make_session();
    let first = ObjectGuid::create_player(1, 61_870);
    let late = ObjectGuid::create_player(1, 61_871);
    let owner_guid = test_gameobject_guid(61_872);
    let mut gameobject =
        make_canonical_gameobject_for_session(&session, owner_guid, GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_loot_state(LootState::Activated, Some(first));
    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(first));
    authority.add_viewer_like_cpp(first).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;
    let close = authority
        .close_viewer_if_generation_like_cpp(generation, first)
        .unwrap();
    assert!(
        session
            .set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                owner_guid,
                &authority,
                close.object_generation,
                close.lifecycle_revision,
                LootState::JustDeactivated,
                None,
                0,
                false,
            )
            .is_some()
    );

    let mut late_pool = authoritative_test_loot_like_cpp(0, true);
    late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner_guid.realm_id(),
        owner_guid.map_id(),
        0,
        0,
        owner_guid.counter() + 1,
    );
    late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    late_pool.allowed_looters = vec![late];
    late_pool.items[0].allowed_looters = vec![late];
    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_like_cpp(
                owner_guid, late, late_pool, false,
            )
            .is_none(),
        "a generator finishing after JustDeactivated must not resurrect the object"
    );
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::JustDeactivated
    );
    assert!(authority.snapshot_for_player_like_cpp(late).is_none());
}
#[tokio::test]
async fn loot_release_fishing_gameobjects_follow_cpp_state_branches() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_node = test_gameobject_guid(19_033);
    let fishing_hole = test_gameobject_guid(19_034);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(fishing_node);
    session.add_active_loot_view_owner_like_cpp(fishing_hole);
    for (guid, go_type, loot_type) in [
        (
            fishing_node,
            GAMEOBJECT_TYPE_FISHING_NODE as u8,
            LOOT_TYPE_FISHING_LIKE_CPP,
        ),
        (
            fishing_hole,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
            LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
        ),
    ] {
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            go_type,
        );
        session.loot_table.insert(
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 0,
                unlooted_count: 1,
                loot_type,
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
    }

    session
        .handle_loot_release(loot_release_packet(fishing_node))
        .await;
    session
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&fishing_node)
            .unwrap()
            .loot_state,
        Some(LootState::JustDeactivated)
    );
    let hole_state = session
        .represented_gameobject_use_states
        .get(&fishing_hole)
        .unwrap();
    assert_eq!(hole_state.loot_state, Some(LootState::Ready));
    assert_eq!(hole_state.personal_loot_uses, 1);
}
#[tokio::test]
async fn gameobject_loot_release_fishing_hole_uses_canonical_use_count_when_represented_stale_like_cpp()
 {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let fishing_hole = test_gameobject_guid(19_136);
    let mut game_object = make_canonical_gameobject_for_session(
        &session,
        fishing_hole,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    game_object.add_use_like_cpp();
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(fishing_hole);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        fishing_hole,
        fishing_hole.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    session.record_represented_fishing_hole_max_opens_like_cpp(fishing_hole, 2);
    session.loot_table.insert(
        fishing_hole,
        CreatureLoot {
            loot_guid: fishing_hole,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
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
        .handle_loot_release(loot_release_packet(fishing_hole))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let canonical = canonical_gameobject_snapshot(&session, fishing_hole).unwrap();
    assert_eq!(canonical.use_times(), 2);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    let hole_state = session
        .represented_gameobject_use_states
        .get(&fishing_hole)
        .unwrap();
    assert_eq!(hole_state.personal_loot_uses, 2);
    assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
}
#[tokio::test]
async fn loot_release_personal_chest_records_per_player_despawn_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let restocked_chest = test_gameobject_guid(19_039);
    let fallback_chest = test_gameobject_guid(19_040);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(restocked_chest);
    session.add_active_loot_view_owner_like_cpp(fallback_chest);
    session
        .client_visible_guids_like_cpp
        .insert(restocked_chest);
    session.client_visible_guids_like_cpp.insert(fallback_chest);

    for (guid, restock_time) in [(restocked_chest, 45), (fallback_chest, 0)] {
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            guid,
            GameObjectLootSource {
                personal_loot_id: 7_001,
                chest_restock_time_secs: restock_time,
                chest_consumable: false,
                ..Default::default()
            },
        );
        session.loot_table.insert(
            guid,
            CreatureLoot {
                loot_guid: guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );
    }

    session
        .handle_loot_release(loot_release_packet(restocked_chest))
        .await;
    session
        .handle_loot_release(loot_release_packet(fallback_chest))
        .await;

    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(send_rx.try_recv().is_ok());
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&restocked_chest)
    );
    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&fallback_chest)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&restocked_chest)
            .unwrap()
            .per_player_despawn_secs,
        Some(45)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&fallback_chest)
            .unwrap()
            .per_player_despawn_secs,
        Some(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS)
    );
    assert!(
        session
            .represented_gameobject_use_states
            .get(&restocked_chest)
            .unwrap()
            .per_player_despawn_until
            .is_some()
    );
    assert!(session.represented_gameobject_is_per_player_despawned_like_cpp(restocked_chest));
}
#[tokio::test]
async fn loot_release_personal_chest_without_have_at_client_sends_no_out_of_range_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = test_gameobject_guid(19_137);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_active_loot_guid(chest_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    session.record_represented_gameobject_chest_release_metadata_like_cpp(
        chest_guid,
        GameObjectLootSource {
            personal_loot_id: 7_001,
            chest_restock_time_secs: 45,
            chest_consumable: false,
            ..Default::default()
        },
    );
    session.loot_table.insert(
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(chest_guid))
        .await;

    let release_bytes = send_rx.try_recv().unwrap();
    let mut release = WorldPacket::from_bytes(&release_bytes);
    assert_eq!(
        release.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert!(send_rx.try_recv().is_err());
    let state = session
        .represented_gameobject_use_states
        .get(&chest_guid)
        .unwrap();
    assert_eq!(state.per_player_despawn_secs, Some(45));
    assert!(state.per_player_despawn_until.is_some());
    assert_eq!(state.per_player_state_player_guid, Some(player_guid));
}
#[tokio::test]
async fn loot_release_shared_chest_restock_starts_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 42);
    let partial_chest = test_gameobject_guid(19_041);
    let full_chest = test_gameobject_guid(19_042);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(partial_chest);
    session.add_active_loot_view_owner_like_cpp(full_chest);

    for guid in [partial_chest, full_chest] {
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            guid,
            guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            guid,
            GameObjectLootSource {
                loot_id: 7_001,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                ..Default::default()
            },
        );
    }
    session.loot_table.insert(
        partial_chest,
        CreatureLoot {
            loot_guid: partial_chest,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );
    session.loot_table.insert(
        full_chest,
        CreatureLoot {
            loot_guid: full_chest,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
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
        .handle_loot_release(loot_release_packet(partial_chest))
        .await;
    session
        .handle_loot_release(loot_release_packet(full_chest))
        .await;

    let partial_state = session
        .represented_gameobject_use_states
        .get(&partial_chest)
        .unwrap();
    assert_eq!(partial_state.loot_state, Some(LootState::Activated));
    assert!(partial_state.chest_restock_until.is_some());
    assert!(session.loot_table.contains_key(&partial_chest));

    let full_state = session
        .represented_gameobject_use_states
        .get(&full_chest)
        .unwrap();
    assert_eq!(full_state.loot_state, Some(LootState::NotReady));
    assert!(full_state.chest_restock_until.is_some());
    assert!(!session.loot_table.contains_key(&full_chest));
}
#[tokio::test]
async fn process_pending_shared_chest_restock_clears_loot_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let chest_guid = test_gameobject_guid(19_043);
    session.set_state(SessionState::LoggedIn);
    session.record_represented_gameobject_runtime_state_like_cpp(
        0,
        chest_guid,
        chest_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    {
        let state = session
            .represented_gameobject_use_states
            .get_mut(&chest_guid)
            .unwrap();
        state.loot_state = Some(LootState::Activated);
        state.chest_restock_until = Some(Instant::now() - Duration::from_secs(1));
    }
    session.loot_table.insert(
        chest_guid,
        CreatureLoot {
            loot_guid: chest_guid,
            coins: 7,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&chest_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(LootState::Ready));
    assert!(state.chest_restock_until.is_none());
    assert!(!session.loot_table.contains_key(&chest_guid));
}
