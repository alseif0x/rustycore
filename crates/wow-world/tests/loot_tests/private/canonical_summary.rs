//! Canonical loot lifetime regressions moved intact from the application owner.
use super::recovery_support::*;
use std::{collections::HashMap, sync::Barrier};
use wow_loot::mark_loot_item_looted_for_player_like_cpp;
use wow_world::test_fixtures::loot::{
    GameObjectLootWitness, close_retired_loot_views_for_test, loot_cache_mut_for_test,
    mutate_loot_gameobject_for_test, observe_gameobject_loot_for_test,
    refresh_loot_summary_for_test, release_fishing_hole_for_test,
    release_gameobject_observation_for_test, release_loot_owner_for_test,
    share_loot_canonical_map_for_test, upsert_gameobject_pool_for_test,
    upsert_observed_gameobject_pool_for_test,
};
use wow_world::test_fixtures::loot::{
    attach_canonical_creature_for_loot_test as attach_canonical_creature,
    attach_canonical_gameobject_for_loot_test as attach_canonical_gameobject,
    canonical_creature_snapshot_for_loot_test as canonical_creature_snapshot,
    canonical_gameobject_snapshot_for_loot_test as canonical_gameobject_snapshot,
    make_canonical_creature_for_loot_test as make_canonical_creature_for_session,
    make_canonical_gameobject_for_loot_test as make_canonical_gameobject_for_session,
};

#[tokio::test]
async fn loot_item_creature_pickup_refreshes_canonical_owned_loot_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_115);
    let mut creature = make_canonical_creature_for_session(&session, loot_guid);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(0, 1));
    attach_canonical_creature(&mut session, creature);
    session.set_player_guid(Some(player_guid));
    set_loot_for_test(
        &mut session,
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
        loot_cache_mut_for_test(&mut session, loot_guid).unwrap(),
        0,
        player_guid,
    );
    refresh_loot_summary_for_test(&mut session, loot_guid, player_guid);

    let loot = loot_recovery_cache_for_test(&session, loot_guid).unwrap();
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
async fn loot_item_gameobject_pickup_refreshes_canonical_owned_loot_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_gameobject_guid(19_139);
    let mut game_object =
        make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
    game_object.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(0, 1));
    game_object.set_personal_loot_like_cpp(player_guid, GameObjectOwnedLoot::new(0, 1));
    attach_canonical_gameobject(&mut session, game_object);
    session.set_player_guid(Some(player_guid));
    set_loot_for_test(
        &mut session,
        loot_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
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
        loot_cache_mut_for_test(&mut session, loot_guid).unwrap(),
        0,
        player_guid,
    );
    refresh_loot_summary_for_test(&mut session, loot_guid, player_guid);

    let loot = loot_recovery_cache_for_test(&session, loot_guid).unwrap();
    assert!(loot.items[0].is_looted_for_player_like_cpp(player_guid));
    assert_eq!(loot.unlooted_count, 0);
    let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
    assert_eq!(
        canonical.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::default())
    );
    assert_eq!(canonical.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        canonical.loot_for_player_like_cpp(player_guid),
        Some(&GameObjectOwnedLoot::default())
    );
    assert!(canonical.is_fully_looted_like_cpp());
}
