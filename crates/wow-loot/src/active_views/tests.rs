//! Pure open-view state regressions; packet and persistence cases stay in world.
use super::*;

#[test]
fn active_loot_guid_tracks_cpp_loot_target_guid_comparisons() {
    let mut views = LootViews::default();
    let loot_guid = ObjectGuid::create_item(1, 700);
    let other_guid = ObjectGuid::create_item(1, 701);

    assert!(!views.is_primary(loot_guid));
    views.set_primary(loot_guid);
    assert!(views.is_primary(loot_guid));
    assert!(!views.is_primary(other_guid));

    views.remove_owner(other_guid);
    assert!(views.is_primary(loot_guid));
    views.remove_owner(loot_guid);
    assert!(!views.is_primary(loot_guid));
}

fn authority_with_money(player: ObjectGuid) -> OwnedLootAuthority {
    let authority = OwnedLootAuthority::new();
    authority.replace_like_cpp(
        Some(crate::CreatureLoot {
            loot_guid: ObjectGuid::create_item(1, 700),
            coins: 7,
            unlooted_count: 0,
            loot_type: 1,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player],
            items: Vec::new(),
            looted_by_player: false,
        }),
        HashMap::new(),
    );
    authority
}

fn money_claim(authority: &OwnedLootAuthority, player: ObjectGuid) -> LootClaimLease {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    // A fresh, unreserved pool is immediately ready; no executor is needed.
    let mut future = std::pin::pin!(authority.reserve_money_like_cpp(player));
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(Ok(claim)) => claim,
        Poll::Ready(Err(error)) => panic!("fresh money claim rejected: {error:?}"),
        Poll::Pending => panic!("fresh money claim unexpectedly waited"),
    }
}

#[test]
fn equal_generations_on_distinct_authorities_do_not_match() {
    let player = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_item(1, 700);
    let first = authority_with_money(player);
    let other = authority_with_money(player);
    let generation = first.snapshot_for_player_like_cpp(player).unwrap().generation;
    let other_generation = other.snapshot_for_player_like_cpp(player).unwrap().generation;
    assert_eq!(generation, other_generation);
    let mut views = LootViews::default();
    views.set_primary(owner);
    views.bind_opened(owner, generation, &first);

    assert!(views.matches_authority(owner, &first, Some(generation)));
    assert!(!views.matches_authority(owner, &other, Some(other_generation)));
    assert!(!views.matches_authority(owner, &first, Some(generation + 1)));
    assert!(!views.matches_authority(owner, &first, None));
}

#[test]
fn recreated_authority_does_not_reuse_retired_view_binding() {
    let player = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_item(1, 700);
    let original = authority_with_money(player);
    let generation = original.snapshot_for_player_like_cpp(player).unwrap().generation;
    let mut views = LootViews::default();
    views.set_primary(owner);
    views.bind_opened(owner, generation, &original);
    original.retire_like_cpp();

    let replacement = authority_with_money(player);
    let replacement_generation = replacement.snapshot_for_player_like_cpp(player).unwrap().generation;
    assert_eq!(generation, replacement_generation);
    assert!(!views.matches_authority(owner, &replacement, Some(replacement_generation)));
    let retired_generation = original.snapshot_for_player_like_cpp(player).map(|snapshot| snapshot.generation);
    assert!(!views.matches_authority(owner, &original, retired_generation));

    views.bind_opened(owner, replacement_generation, &replacement);
    assert!(views.matches_authority(owner, &replacement, Some(replacement_generation)));
    assert!(!views.matches_authority(owner, &original, Some(generation)));
}

#[test]
fn reset_replaces_all_views_and_empty_reset_closes_tracking() {
    let first = ObjectGuid::create_item(1, 700);
    let second = ObjectGuid::create_item(1, 701);
    let next = ObjectGuid::create_item(1, 702);
    let authority = OwnedLootAuthority::new();
    let mut views = LootViews::default();
    views.set_primary(first);
    views.add_owner(second);
    views.bind_opened(first, 3, &authority);
    views.bind_opened(second, 4, &authority);
    views.set_primary(next);

    assert!(views.is_primary(next));
    assert_eq!(views.owner_count(), 1);
    for owner in [first, second] {
        assert!(!views.contains_owner(&owner));
        assert_eq!(views.generation(&owner), None);
        assert!(views.authority(&owner).is_none());
    }
    assert_eq!(views.generation(&next), None);
    assert!(views.authority(&next).is_none());
    views.bind_opened(next, 5, &authority);
    views.set_primary(ObjectGuid::EMPTY);
    assert!(!views.has_views());
    assert!(!views.has_owners());
    assert_eq!(views.generation(&next), None);
    assert!(views.bound_authorities().next().is_none());
}

#[test]
fn removing_primary_preserves_secondary_without_promoting_it() {
    let primary = ObjectGuid::create_item(1, 700);
    let secondary = ObjectGuid::create_item(1, 701);
    let authority = OwnedLootAuthority::new();
    let mut views = LootViews::default();
    views.set_primary(primary);
    views.add_owner(secondary);
    views.bind_opened(primary, 3, &authority);
    views.bind_opened(secondary, 4, &authority);
    views.remove_owner(primary);

    assert!(views.primary_guid().is_empty());
    assert!(!views.is_primary(secondary));
    assert!(!views.is_primary(ObjectGuid::EMPTY));
    assert!(views.has_views());
    assert_eq!(views.owner_selection(), vec![secondary]);
    assert_eq!(views.generation(&primary), None);
    assert!(views.authority(&primary).is_none());
    assert!(views.matches_authority(secondary, &authority, Some(4)));
}

#[test]
fn bootstrap_preserves_each_existing_binding_independently() {
    let owner = ObjectGuid::create_item(1, 700);
    let other_owner = ObjectGuid::create_item(1, 701);
    let first = OwnedLootAuthority::new();
    let other = OwnedLootAuthority::new();
    let mut views = LootViews::default();
    views.record_generation(owner, 3);
    views.bootstrap_binding(owner, 4, &first);
    assert!(views.matches_authority(owner, &first, Some(3)));
    assert!(!views.matches_authority(owner, &first, Some(4)));
    views.bootstrap_binding(owner, 5, &other);
    assert!(views.matches_authority(owner, &first, Some(3)));
    assert!(!views.matches_authority(owner, &other, Some(3)));

    views.bind_authority_if_changed(other_owner, &first);
    views.bootstrap_binding(other_owner, 6, &other);
    assert!(views.matches_authority(other_owner, &first, Some(6)));
    assert!(!views.matches_authority(other_owner, &other, Some(6)));
}

#[test]
fn owner_selection_preserves_set_iteration_and_empty_fallback() {
    let mut views = LootViews::default();
    assert_eq!(views.owner_selection(), vec![ObjectGuid::EMPTY]);
    views.add_owner(ObjectGuid::EMPTY);
    assert!(!views.has_views());
    for counter in 700..732 {
        views.add_owner(ObjectGuid::create_item(1, counter));
    }
    let original_order: Vec<_> = views.owners().copied().collect();
    assert_eq!(views.owner_selection(), original_order);
    assert_eq!(views.owner_selection().len(), 32);
    assert_eq!(views.primary_guid(), ObjectGuid::create_item(1, 700));
}

#[test]
fn claims_require_both_original_allocation_and_generation() {
    let player = ObjectGuid::create_player(1, 42);
    let owner = ObjectGuid::create_item(1, 700);
    let original = authority_with_money(player);
    let other = authority_with_money(player);
    let claim = money_claim(&original, player);
    let other_claim = money_claim(&other, player);
    assert_eq!(claim.generation_like_cpp(), other_claim.generation_like_cpp());
    let mut views = LootViews::default();
    views.bind_opened(owner, claim.generation_like_cpp(), &original);
    assert!(views.matches_claim(owner, &claim));
    assert!(!views.matches_claim(owner, &other_claim));
    views.record_generation(owner, claim.generation_like_cpp() + 1);
    assert!(!views.matches_claim(owner, &claim));
}

#[test]
fn reopening_replaces_only_a_changed_authority() {
    let owner = ObjectGuid::create_item(1, 700);
    let first = OwnedLootAuthority::new();
    let other = OwnedLootAuthority::new();
    let mut views = LootViews::default();
    views.bind_opened(owner, 3, &first);
    views.bind_authority_if_changed(owner, &first);
    assert!(views.matches_authority(owner, &first, Some(3)));
    views.bind_authority_if_changed(owner, &other);
    assert!(views.matches_authority(owner, &other, Some(3)));
    assert!(!views.matches_authority(owner, &first, Some(3)));
    assert_eq!(views.generation(&owner), Some(3));
}
