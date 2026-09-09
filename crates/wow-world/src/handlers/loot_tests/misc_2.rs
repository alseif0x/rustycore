//! Misc scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn disconnect_after_primary_ae_release_closes_secondary_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_711);
    let primary_guid = test_creature_guid(61_712);
    let secondary_guid = test_creature_guid(61_713);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(session.active_loot_guid.is_empty());
    assert!(session.active_loot_view_owners.contains(&secondary_guid));

    session
        .cleanup_shared_runtime_state_on_disconnect_like_cpp()
        .await;

    assert!(session.active_loot_view_owners.is_empty());
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "logout must remove the secondary AE viewer even after the primary was released"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
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

    let stale_observation = session
        .represented_gameobject_loot_install_observation_like_cpp(owner_guid)
        .unwrap();
    session
        .mutate_canonical_gameobject_by_guid_like_cpp(owner_guid, |gameobject| {
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
        session
            .upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                owner_guid,
                second,
                stale_pool.clone(),
                false,
                &stale_observation,
            )
            .is_none(),
        "an async generator from before ClearLoot must lose the lifecycle CAS"
    );
    assert!(authority.is_retired_like_cpp());

    assert!(
        session
            .upsert_represented_personal_gameobject_loot_authority_like_cpp(
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
    second.set_canonical_map_manager(Arc::clone(first.canonical_map_manager.as_ref().unwrap()));
    let start = Arc::new(Barrier::new(2));

    std::thread::scope(|scope| {
        let first_start = Arc::clone(&start);
        let first_session = &mut first;
        let first_handle = scope.spawn(move || {
            first_start.wait();
            first_session
                .release_canonical_fishing_hole_like_cpp(fishing_hole, Some(2))
                .unwrap()
        });
        let second_start = Arc::clone(&start);
        let second_session = &mut second;
        let second_handle = scope.spawn(move || {
            second_start.wait();
            second_session
                .release_canonical_fishing_hole_like_cpp(fishing_hole, Some(2))
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
