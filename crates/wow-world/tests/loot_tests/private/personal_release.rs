//! Canonical personal-pool release completion and reopening the same lifetime.

use super::recovery_support::*;
use std::collections::HashMap;
use wow_world::test_fixtures::loot::{
    make_canonical_gameobject_for_loot_test as make_canonical_gameobject_for_session,
    attach_canonical_gameobject_for_loot_test as attach_canonical_gameobject,
    canonical_gameobject_snapshot_for_loot_test as canonical_gameobject_snapshot,
    loot_fixture_response, open_loot_response_for_test, release_loot_owner_for_test,
    open_money_loot_normally_for_test, has_cached_loot_generation_for_test, personal_loot_money_entry_for_test, update_loot_gameobject_for_test,
};

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
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        owner_guid,
        owner_guid.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    record_gameobject_chest_release_metadata_for_loot_test(&mut session, 
        owner_guid,
        GameObjectLootSource {
            personal_loot_id: 55,
            chest_consumable: false,
            chest_restock_time_secs: 7,
            ..Default::default()
        },
    );

    session.set_player_guid(Some(first_player));
    assert!(reconcile_loot_cache_for_test(&mut session, owner_guid, first_player));
    set_active_loot_guid_for_test(&mut session, owner_guid);
    let response = loot_fixture_response(
        owner_guid,
        loot_recovery_cache_for_test(&session, owner_guid).unwrap(),
        first_player,
    );
    open_loot_response_for_test(&mut session, owner_guid, first_player, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        release_loot_owner_for_test(&mut session, owner_guid, first_player)
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
        gameobject_loot_release_snapshot_for_test(&session, owner_guid)
            .unwrap()
            .per_player_state_player_guid,
        Some(first_player),
        "C++ still runs OnLootRelease for the selected empty personal pool"
    );

    session.set_player_guid(Some(second_player));
    assert!(reconcile_loot_cache_for_test(&mut session, owner_guid, second_player));
    set_active_loot_guid_for_test(&mut session, owner_guid);
    let response = loot_fixture_response(
        owner_guid,
        loot_recovery_cache_for_test(&session, owner_guid).unwrap(),
        second_player,
    );
    open_loot_response_for_test(&mut session, owner_guid, second_player, response);
    let claim = authority
        .reserve_item_like_cpp(second_player, 0)
        .await
        .unwrap();
    assert_eq!(claim.commit_like_cpp(), Ok(true));
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        release_loot_owner_for_test(&mut session, owner_guid, second_player)
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

    update_loot_gameobject_for_test(&session, owner_guid, 1, 0);
    assert!(
        authority.is_retired_like_cpp(),
        "the canonical JustDeactivated update must clear and retire the completed authority"
    );
}

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
    record_represented_gameobject_runtime_state_for_test(&mut session, 
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
    record_gameobject_chest_release_metadata_for_loot_test(&mut session, owner_guid, source);
    assert!(reconcile_loot_cache_for_test(&mut session, owner_guid, player_guid));
    set_active_loot_guid_for_test(&mut session, owner_guid);
    let response = loot_fixture_response(
        owner_guid,
        loot_recovery_cache_for_test(&session, owner_guid).unwrap(),
        player_guid,
    );
    open_loot_response_for_test(&mut session, owner_guid, player_guid, response);
    let _ = drain_server_opcodes_like_cpp(&send_rx);

    assert!(
        release_loot_owner_for_test(&mut session, owner_guid, player_guid)
            .await
    );
    assert!(!has_loot_for_test(&session, owner_guid));
    assert!(
        !has_cached_loot_generation_for_test(&session, owner_guid)
    );
    assert!(
        !personal_loot_money_entry_for_test(&session, owner_guid, player_guid).is_some()
    );
    let before_reopen = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("release preserves the canonical personal pool");
    assert_eq!(before_reopen.loot.coins, 11);
    assert!(!before_reopen.loot.items[0].taken);

    open_money_loot_normally_for_test(&mut session, owner_guid, source)
        .await;
    assert!(has_loot_for_test(&session, owner_guid));
    assert!(
        personal_loot_marker_for_test(&session, owner_guid, player_guid).0
    );
    assert_eq!(
        personal_loot_money_entry_for_test(&session, owner_guid, player_guid),
        Some(&11)
    );
    assert!(is_active_loot_guid_for_test(&session, owner_guid));

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
