//! Canonical loot lifetime regressions moved intact from the application owner.
use super::recovery_support::*;
use std::{collections::HashMap, sync::Barrier};
use wow_loot::mark_loot_item_looted_for_player_like_cpp;
use wow_world::test_fixtures::loot::{GameObjectLootWitness, observe_gameobject_loot_for_test, upsert_gameobject_pool_for_test, upsert_observed_gameobject_pool_for_test, release_gameobject_observation_for_test, mutate_loot_gameobject_for_test, release_fishing_hole_for_test, release_loot_owner_for_test, close_retired_loot_views_for_test, refresh_loot_summary_for_test, loot_cache_mut_for_test, share_loot_canonical_map_for_test};
use wow_world::test_fixtures::loot::{
    make_canonical_gameobject_for_loot_test as make_canonical_gameobject_for_session,
    attach_canonical_gameobject_for_loot_test as attach_canonical_gameobject,
    canonical_gameobject_snapshot_for_loot_test as canonical_gameobject_snapshot,
    make_canonical_creature_for_loot_test as make_canonical_creature_for_session,
    attach_canonical_creature_for_loot_test as attach_canonical_creature,
    canonical_creature_snapshot_for_loot_test as canonical_creature_snapshot,
};

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
        release_gameobject_observation_for_test(&mut session, 
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
        upsert_gameobject_pool_for_test(&mut session, 
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
        upsert_gameobject_pool_for_test(&mut session, 
                owner_guid, late, late_pool, false,
            )
            .is_some()
    );

    assert!(
        release_gameobject_observation_for_test(&mut session, 
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
fn personal_fishing_hole_restock_accepts_second_lifecycle_and_rejects_old_observation_like_cpp() {
    let mut session = make_session();
    let first = ObjectGuid::create_player(1, 61_880);
    let second = ObjectGuid::create_player(1, 61_881);
    let owner_guid = test_gameobject_guid(61_882);
    let mut gameobject = make_canonical_gameobject_for_session(
        &session,
        owner_guid,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    gameobject.set_loot_state(LootState::Activated, Some(first));
    let mut first_pool = authoritative_test_loot_like_cpp(0, false);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    first_pool.loot_type = LOOT_TYPE_FISHINGHOLE_LIKE_CPP;
    first_pool.allowed_looters = vec![first];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    let first_generation = authority.generation_like_cpp();
    attach_canonical_gameobject(&mut session, gameobject);
    session.set_player_guid(Some(second));

    let stale_observation = observe_gameobject_loot_for_test(&mut session, owner_guid)
        .unwrap();
    mutate_loot_gameobject_for_test(&mut session, owner_guid, |gameobject| {
            gameobject.clear_loot_like_cpp();
            gameobject.set_loot_state(LootState::Ready, None);
        })
        .unwrap();

    let mut stale_pool = authoritative_test_loot_like_cpp(0, true);
    stale_pool.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    stale_pool.loot_type = LOOT_TYPE_FISHINGHOLE_LIKE_CPP;
    stale_pool.allowed_looters = vec![second];
    stale_pool.items[0].allowed_looters = vec![second];
    assert!(
        upsert_observed_gameobject_pool_for_test(&mut session, 
                owner_guid,
                second,
                stale_pool.clone(),
                false,
                false,
                &stale_observation,
            )
            .is_none(),
        "an async generator from before ClearLoot must lose the lifecycle CAS"
    );
    assert!(authority.is_retired_like_cpp());

    assert!(
        upsert_gameobject_pool_for_test(&mut session, 
                owner_guid, second, stale_pool, false,
            )
            .is_some(),
        "a generator started after Ready must install the new fishing-hole lifetime"
    );
    assert!(authority.generation_like_cpp() > first_generation);
    assert!(authority.snapshot_for_player_like_cpp(second).is_some());
    assert_eq!(
        canonical_gameobject_snapshot(&session, owner_guid)
            .unwrap()
            .loot_state(),
        LootState::Ready
    );
}

#[test]
fn concurrent_fishing_hole_releases_cannot_finish_ready_after_max_like_cpp() {
    let mut first = make_session();
    let mut second = make_session();
    let fishing_hole = test_gameobject_guid(61_910);
    let gameobject = make_canonical_gameobject_for_session(
        &first,
        fishing_hole,
        GAMEOBJECT_TYPE_FISHING_HOLE as u8,
    );
    attach_canonical_gameobject(&mut first, gameobject);
    share_loot_canonical_map_for_test(&first, &mut second);
    let start = Arc::new(Barrier::new(2));

    std::thread::scope(|scope| {
        let first_start = Arc::clone(&start);
        let first_session = &mut first;
        let first_handle = scope.spawn(move || {
            first_start.wait();
            release_fishing_hole_for_test(first_session, fishing_hole, Some(2))
                .unwrap()
        });
        let second_start = Arc::clone(&start);
        let second_session = &mut second;
        let second_handle = scope.spawn(move || {
            second_start.wait();
            release_fishing_hole_for_test(second_session, fishing_hole, Some(2))
                .unwrap()
        });
        let first_outcome = first_handle.join().unwrap();
        let second_outcome = second_handle.join().unwrap();
        assert_eq!(
            [first_outcome.0, second_outcome.0].into_iter().max(),
            Some(2)
        );
        assert!([first_outcome.1, second_outcome.1].contains(&LootState::JustDeactivated));
    });

    let canonical = canonical_gameobject_snapshot(&first, fishing_hole).unwrap();
    assert_eq!(canonical.use_times(), 2);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
}

#[test]
fn personal_encounter_late_upsert_cannot_cross_clear_loot_like_cpp() {
    let mut session = make_session();
    let first_player = ObjectGuid::create_player(1, 342);
    let late_player = ObjectGuid::create_player(1, 377);
    let gameobject_guid = test_gameobject_guid(91_020);
    let mut gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    let mut first_pool = authoritative_test_loot_like_cpp(0, true);
    first_pool.loot_guid = represented_loot_object_guid_like_cpp(gameobject_guid);
    first_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    first_pool.allowed_looters = vec![first_player];
    first_pool.items[0].allowed_looters = vec![first_player];
    assert!(
        gameobject
            .initialize_loot_authority_like_cpp(None, HashMap::from([(first_player, first_pool)]),)
            .installed()
    );
    let authority = gameobject.loot_authority_like_cpp().clone();
    attach_canonical_gameobject(&mut session, gameobject);
    let observation = observe_gameobject_loot_for_test(&mut session, gameobject_guid)
        .expect("late generation observes the active chest lifetime");

    let mut stale_late_pool = authoritative_test_loot_like_cpp(0, true);
    stale_late_pool.loot_guid = ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        gameobject_guid.realm_id(),
        gameobject_guid.map_id(),
        0,
        0,
        gameobject_guid.counter() + 1,
    );
    stale_late_pool.loot_type = LOOT_TYPE_CHEST_LIKE_CPP;
    stale_late_pool.allowed_looters = vec![late_player];
    stale_late_pool.items[0].allowed_looters = vec![late_player];
    set_personal_loot_for_loot_test(&mut session, gameobject_guid, late_player, 0);

    mutate_loot_gameobject_for_test(&mut session, gameobject_guid, |gameobject| {
        gameobject.clear_loot_like_cpp();
    });

    assert!(
        upsert_observed_gameobject_pool_for_test(&mut session, 
                gameobject_guid,
                late_player,
                stale_late_pool,
                false,
                true,
                &observation,
            )
            .is_none()
    );
    assert!(authority.is_retired_like_cpp());
    assert!(authority.personal_snapshots_like_cpp().is_empty());
    assert!(
        !personal_loot_marker_for_test(&session, gameobject_guid, late_player).1.is_some()
    );
    assert!(!has_loot_for_test(&session, gameobject_guid));
}
