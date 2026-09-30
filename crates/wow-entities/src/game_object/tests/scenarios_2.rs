//! GameObject template, loot and runtime state regression scenarios, part 2 of 3.
//!
//! Moved out of the game_object.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn gameobject_linked_trap_guid_defaults_and_can_be_cleared_like_cpp() {
    let mut go = GameObject::new();
    assert_eq!(go.linked_trap_guid_like_cpp(), ObjectGuid::EMPTY);
    let trap_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 1, 9002, 2);

    go.set_linked_trap_like_cpp(trap_guid);
    assert_eq!(go.linked_trap_guid_like_cpp(), trap_guid);

    go.set_linked_trap_like_cpp(ObjectGuid::EMPTY);
    assert_eq!(go.linked_trap_guid_like_cpp(), ObjectGuid::EMPTY);
}

#[test]
fn gameobject_update_no_despawn_delay_stays_updated_like_cpp() {
    let mut go = GameObject::new();

    let outcome = go.update_like_cpp(40);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.despawn_delay_before_ms, 0);
    assert_eq!(outcome.despawn_delay_after_ms, 0);
    assert!(!outcome.despawn_or_unsummon_requested);
    assert!(outcome.world_update_would_run);
    assert!(outcome.ai_update_not_represented);
    assert!(outcome.go_type_impl_update_not_represented);
}

#[test]
fn gameobject_update_decrements_pending_despawn_delay_like_cpp() {
    let mut go = GameObject::new();
    assert!(go.schedule_despawn_or_unsummon_like_cpp(100, 7));

    let outcome = go.update_like_cpp(40);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.despawn_delay_before_ms, 100);
    assert_eq!(outcome.despawn_delay_after_ms, 60);
    assert_eq!(outcome.despawn_respawn_time_secs, 7);
    assert_eq!(go.despawn_delay(), 60);
    assert!(!outcome.despawn_or_unsummon_requested);
}

#[test]
fn gameobject_update_expired_despawn_delay_requests_immediate_despawn_like_cpp() {
    let mut exact = GameObject::new();
    assert!(exact.schedule_despawn_or_unsummon_like_cpp(40, 9));
    let exact_outcome = exact.update_like_cpp(40);
    assert_eq!(
        exact_outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRequested
    );
    assert_eq!(exact_outcome.despawn_delay_before_ms, 40);
    assert_eq!(exact_outcome.despawn_delay_after_ms, 0);
    assert_eq!(exact_outcome.despawn_respawn_time_secs, 9);
    assert!(exact_outcome.despawn_or_unsummon_requested);
    assert_eq!(exact.despawn_delay(), 0);
    assert_eq!(exact.despawn_respawn_time(), 9);

    let mut overshoot = GameObject::new();
    assert!(overshoot.schedule_despawn_or_unsummon_like_cpp(40, 11));
    let overshoot_outcome = overshoot.update_like_cpp(50);
    assert_eq!(
        overshoot_outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRequested
    );
    assert_eq!(overshoot_outcome.despawn_delay_before_ms, 40);
    assert_eq!(overshoot_outcome.despawn_delay_after_ms, 0);
    assert_eq!(overshoot_outcome.despawn_respawn_time_secs, 11);
    assert!(overshoot_outcome.despawn_or_unsummon_requested);
    assert_eq!(overshoot.despawn_respawn_time(), 11);
}

#[test]
fn gameobject_update_despawn_scheduler_only_shortens_pending_delay_like_cpp() {
    let mut go = GameObject::new();

    assert!(!go.schedule_despawn_or_unsummon_like_cpp(0, 3));
    assert_eq!(go.despawn_delay(), 0);
    assert_eq!(go.despawn_respawn_time(), 0);

    assert!(go.schedule_despawn_or_unsummon_like_cpp(100, 7));
    assert_eq!(go.despawn_delay(), 100);
    assert_eq!(go.despawn_respawn_time(), 7);

    assert!(!go.schedule_despawn_or_unsummon_like_cpp(150, 9));
    assert_eq!(go.despawn_delay(), 100);
    assert_eq!(go.despawn_respawn_time(), 7);

    assert!(go.schedule_despawn_or_unsummon_like_cpp(40, 11));
    assert_eq!(go.despawn_delay(), 40);
    assert_eq!(go.despawn_respawn_time(), 11);
}

#[test]
fn gameobject_owned_loot_is_looted_matches_cpp_gold_and_unlooted_count() {
    let empty = GameObjectOwnedLoot::default();
    assert_eq!(empty.gold(), 0);
    assert_eq!(empty.unlooted_count(), 0);
    assert!(empty.is_looted_like_cpp());

    let gold_only = GameObjectOwnedLoot::new(1, 0);
    assert_eq!(gold_only.gold(), 1);
    assert_eq!(gold_only.unlooted_count(), 0);
    assert!(!gold_only.is_looted_like_cpp());

    let items_only = GameObjectOwnedLoot::new(0, 1);
    assert_eq!(items_only.gold(), 0);
    assert_eq!(items_only.unlooted_count(), 1);
    assert!(!items_only.is_looted_like_cpp());

    assert!(!GameObjectOwnedLoot::new(1, 1).is_looted_like_cpp());
}

#[test]
fn gameobject_personal_authority_updates_summary_and_clear_retires_it() {
    let player = ObjectGuid::create_player(1, 77);
    let mut gameobject = GameObject::new();
    let full_loot = CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 23,
        unlooted_count: 1,
        loot_type: 9,
        dungeon_encounter_id: 0,
        loot_method: 5,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player],
        items: Vec::new(),
        looted_by_player: false,
    };
    gameobject.upsert_personal_loot_authority_like_cpp(player, full_loot, false);

    assert_eq!(
        gameobject.personal_loot_like_cpp(player),
        Some(&GameObjectOwnedLoot::new(23, 1))
    );
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .snapshot_for_player_like_cpp(player)
            .is_some()
    );
    gameobject.clear_loot_like_cpp();
    assert!(gameobject.loot_authority_like_cpp().is_retired_like_cpp());
    assert_eq!(gameobject.personal_loot_count_like_cpp(), 0);
}

#[test]
fn clear_loot_restock_starts_second_personal_generation_for_same_gameobject() {
    let player = ObjectGuid::create_player(1, 78);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.upsert_personal_loot_authority_like_cpp(
        player,
        owned_loot_fixture_like_cpp(7, 0),
        false,
    );
    let first_generation = gameobject.loot_authority_like_cpp().generation_like_cpp();

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    let authority = gameobject.loot_authority_like_cpp().clone();
    let tombstone_generation = authority.generation_like_cpp();
    let lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    assert!(gameobject.install_personal_loot_if_lifecycle_like_cpp(
        &authority,
        tombstone_generation,
        lifecycle_revision,
        player,
        owned_loot_fixture_like_cpp(19, 0),
        false,
    ));
    let second = gameobject
        .loot_authority_like_cpp()
        .snapshot_for_player_like_cpp(player)
        .expect("restocked personal pool");
    assert_eq!(second.loot.coins, 19);
    assert!(gameobject.loot_authority_like_cpp().generation_like_cpp() > first_generation);
    assert!(!gameobject.loot_authority_like_cpp().is_retired_like_cpp());
}

#[test]
fn stale_personal_generator_cannot_cross_gameobject_clear_loot_lifecycle() {
    let player = ObjectGuid::create_player(1, 79);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.upsert_personal_loot_authority_like_cpp(
        player,
        owned_loot_fixture_like_cpp(3, 0),
        false,
    );
    let stale_authority = gameobject.loot_authority_like_cpp().clone();
    let stale_generation = stale_authority.generation_like_cpp();
    let stale_lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);

    assert!(!gameobject.install_personal_loot_if_lifecycle_like_cpp(
        &stale_authority,
        stale_generation,
        stale_lifecycle_revision,
        player,
        owned_loot_fixture_like_cpp(99, 0),
        false,
    ));
    assert!(gameobject.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .snapshot_for_player_like_cpp(player)
            .is_none()
    );
}

#[test]
fn clear_loot_restock_installs_shared_generation_once_for_same_lifecycle() {
    let player = ObjectGuid::create_player(1, 80);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.initialize_shared_loot_authority_like_cpp(owned_loot_fixture_like_cpp(7, 0));

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    let authority = gameobject.loot_authority_like_cpp().clone();
    let tombstone_generation = authority.generation_like_cpp();
    let lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    assert!(gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &authority,
        tombstone_generation,
        lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(19, 0)),
        HashMap::new(),
    ));
    let installed_generation = authority.generation_like_cpp();
    assert_eq!(
        authority
            .shared_snapshot_like_cpp()
            .expect("restocked shared pool")
            .loot
            .coins,
        19
    );

    assert!(gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &authority,
        tombstone_generation,
        lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(99, 0)),
        HashMap::new(),
    ));
    assert_eq!(authority.generation_like_cpp(), installed_generation);
    assert_eq!(
        authority
            .shared_snapshot_like_cpp()
            .expect("the first shared install remains authoritative")
            .loot
            .coins,
        19,
        "a concurrent generator for the same lifecycle must not replace the first pool"
    );
}

#[test]
fn stale_shared_generator_cannot_cross_second_clear_of_retired_gameobject_lifecycle() {
    let player = ObjectGuid::create_player(1, 81);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.initialize_shared_loot_authority_like_cpp(owned_loot_fixture_like_cpp(3, 0));

    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    let stale_authority = gameobject.loot_authority_like_cpp().clone();
    let stale_generation = stale_authority.generation_like_cpp();
    let stale_lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();

    // `OwnedLootAuthority::retire_like_cpp` is intentionally idempotent for
    // an existing tombstone. The GameObject revision must still reject the
    // async generator captured before this second C++ `ClearLoot`.
    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);
    assert_eq!(stale_authority.generation_like_cpp(), stale_generation);
    assert!(
        gameobject.loot_lifecycle_revision_like_cpp() > stale_lifecycle_revision,
        "the entity-local lifetime advances even when the authority tombstone does not"
    );

    assert!(!gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &stale_authority,
        stale_generation,
        stale_lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(99, 0)),
        HashMap::new(),
    ));
    assert!(gameobject.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .is_none()
    );
    assert_eq!(gameobject.shared_loot_like_cpp(), None);
}

#[test]
fn shared_generator_cannot_install_after_gameobject_authority_rebind() {
    let player = ObjectGuid::create_player(1, 82);
    let mut gameobject = GameObject::new();
    gameobject.set_loot_state(LootState::Activated, Some(player));
    gameobject.initialize_shared_loot_authority_like_cpp(owned_loot_fixture_like_cpp(5, 0));
    gameobject.clear_loot_like_cpp();
    gameobject.set_loot_state(LootState::Ready, None);

    let stale_authority = gameobject.loot_authority_like_cpp().clone();
    let stale_generation = stale_authority.generation_like_cpp();
    let lifecycle_revision = gameobject.loot_lifecycle_revision_like_cpp();
    let replacement = OwnedLootAuthority::new();
    assert!(gameobject.rebind_loot_authority_like_cpp(replacement.clone()));

    assert!(!gameobject.install_loot_authority_if_lifecycle_like_cpp(
        &stale_authority,
        stale_generation,
        lifecycle_revision,
        Some(owned_loot_fixture_like_cpp(77, 0)),
        HashMap::new(),
    ));
    assert!(replacement.is_pristine_like_cpp());
    assert!(replacement.shared_snapshot_like_cpp().is_none());
}

#[test]
fn gameobject_fully_looted_reads_active_authority_without_summary_refresh() {
    let authority = OwnedLootAuthority::new();
    authority.replace_like_cpp(Some(owned_loot_fixture_like_cpp(31, 0)), HashMap::new());
    let mut gameobject = GameObject::new();
    gameobject.rebind_loot_authority_like_cpp(authority.clone());
    assert_eq!(
        gameobject.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(31, 0))
    );
    assert!(!gameobject.is_fully_looted_like_cpp());

    authority.replace_like_cpp(Some(owned_loot_fixture_like_cpp(0, 0)), HashMap::new());

    assert_eq!(
        gameobject.shared_loot_like_cpp(),
        Some(&GameObjectOwnedLoot::new(31, 0)),
        "the compatibility summary remains deliberately stale"
    );
    assert!(
        gameobject.is_fully_looted_like_cpp(),
        "lifecycle decisions must read the active object-owned authority"
    );
}

#[test]
fn gameobject_is_fully_looted_checks_shared_and_personal_loot_like_cpp() {
    let mut go = GameObject::new();
    assert!(go.is_fully_looted_like_cpp());
    assert_eq!(go.shared_loot_like_cpp(), None);
    assert_eq!(go.personal_loot_count_like_cpp(), 0);

    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(10, 0));
    assert!(!go.is_fully_looted_like_cpp());

    go.set_shared_loot_like_cpp(GameObjectOwnedLoot::default());
    assert!(go.is_fully_looted_like_cpp());

    let looted_player = ObjectGuid::new(1, 100);
    let unlooted_player = ObjectGuid::new(1, 200);
    go.set_personal_loot_like_cpp(looted_player, GameObjectOwnedLoot::default());
    assert!(go.is_fully_looted_like_cpp());
    assert_eq!(
        go.personal_loot_like_cpp(looted_player),
        Some(&GameObjectOwnedLoot::default())
    );

    go.set_personal_loot_like_cpp(unlooted_player, GameObjectOwnedLoot::new(0, 1));
    assert_eq!(go.personal_loot_count_like_cpp(), 2);
    assert!(!go.is_fully_looted_like_cpp());

    go.set_personal_loot_like_cpp(unlooted_player, GameObjectOwnedLoot::default());
    assert!(go.is_fully_looted_like_cpp());
}
