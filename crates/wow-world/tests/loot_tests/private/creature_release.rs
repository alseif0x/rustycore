//! Preserved canonical loot lifecycle scenarios.
use super::recovery_support::*;
use std::collections::HashMap;
use wow_world::test_fixtures::loot::*;
use wow_entities::ObjectChangedFields;

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
    let before = mutate_loot_creature_for_test(&mut session, owner_guid, |creature| {
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

    disconnect_loot_cleanup_for_test(&mut session)
        .await;

    let after = mutate_loot_creature_for_test(&mut session, owner_guid, |creature| {
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
    let corpse_deadline_before = mutate_loot_creature_for_test(&mut session, owner_guid, |creature| {
            creature.creature.set_corpse_delay(120, false);
            let deadline = Instant::now() + Duration::from_secs(120);
            creature.set_corpse_despawn_at(Some(deadline));
            creature.corpse_despawn_deadline_ms_like_cpp()
        })
        .flatten()
        .expect("dead creature should already own its normal corpse deadline");

    session.set_player_guid(Some(first_player));
    assert!(reconcile_loot_recovery_cache_for_test(&mut session, owner_guid, first_player));
    session.set_active_loot_guid(owner_guid);
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
        mutate_loot_creature_for_test(&mut session, owner_guid, |creature| {
                creature.corpse_despawn_deadline_ms_like_cpp()
            })
            .flatten(),
        Some(corpse_deadline_before),
        "one empty personal pool must not start global corpse decay while a peer has loot"
    );
    assert!(!authority.is_fully_looted_like_cpp());

    session.set_player_guid(Some(second_player));
    assert!(reconcile_loot_recovery_cache_for_test(&mut session, owner_guid, second_player));
    session.set_active_loot_guid(owner_guid);
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
    assert!(authority.is_fully_looted_like_cpp());
    let (corpse_deadline_after, runtime_elapsed_ms) = mutate_loot_creature_for_test(&mut session, owner_guid, |creature| {
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

#[tokio::test]
async fn authoritative_partial_release_clears_round_robin_for_all_sessions_and_forces_dynflags_like_cpp()
 {
    let first_guid = ObjectGuid::create_player(1, 61_890);
    let second_guid = ObjectGuid::create_player(1, 61_891);
    let mut loot = authoritative_test_loot_like_cpp(7, false);
    loot.round_robin_player = first_guid;
    loot.allowed_looters = vec![first_guid, second_guid];
    let (mut first, _first_rx, mut second, _second_rx, owner_guid, _, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(loot);
    // The helper uses its own deterministic player ids; install the exact
    // current round-robin holder from the opened first session.
    let opened_first = first.player_guid().unwrap();
    let opened_second = second.player_guid().unwrap();
    let authority = loot_recovery_authority_for_test(&mut first, owner_guid)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .generation;
    let mut replacement = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .loot;
    replacement.round_robin_player = opened_first;
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    let replacement_generation = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .generation;
    assert_ne!(generation, replacement_generation);
    authority.add_viewer_like_cpp(opened_first).unwrap();
    bind_loot_view_for_test(&mut first, owner_guid, replacement_generation, &authority);
    assert!(reconcile_loot_recovery_cache_for_test(&mut first, owner_guid, opened_first));
    let _ = mutate_loot_creature_for_test(&mut first, owner_guid, |creature| {
        creature
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .clear_update_mask(false);
    });

    assert!(
        release_loot_owner_for_test(&mut first, owner_guid, opened_first)
            .await
    );

    assert!(
        authority
            .snapshot_for_player_like_cpp(opened_second)
            .unwrap()
            .loot
            .round_robin_player
            .is_empty()
    );
    assert!(reconcile_loot_recovery_cache_for_test(&mut second, owner_guid, opened_second));
    assert!(
        loot_recovery_cache_for_test(&second, owner_guid)
            .unwrap()
            .round_robin_player
            .is_empty()
    );
    assert!(
        mutate_loot_creature_for_test(&mut first, owner_guid, |creature| {
                creature
                    .creature
                    .unit()
                    .world()
                    .object()
                    .changed_fields()
                    .contains(ObjectChangedFields::DYNAMIC_FLAGS)
            })
            .unwrap()
    );
}
