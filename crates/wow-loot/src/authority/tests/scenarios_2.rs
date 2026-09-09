//! Owned-loot authority, claims and leases regression scenarios, part 2 of 2.
//!
//! Moved out of the authority.rs root under #642; every test is unchanged.

use super::*;

#[tokio::test]
async fn persistence_commit_snapshot_excludes_viewers_opened_after_money_transition_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let late = player(2);
    authority.replace_like_cpp(
        Some(money_loot(owner(1), 17, vec![first, late])),
        HashMap::new(),
    );
    authority.add_viewer_like_cpp(first).unwrap();

    let claim = authority.reserve_money_like_cpp(first).await.unwrap();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
    let (first_commit, committed_snapshot) = persistence.commit_with_snapshot_like_cpp().unwrap();
    assert!(first_commit);
    assert_eq!(
        committed_snapshot.unwrap().loot.players_looting,
        vec![first],
        "the durable fanout snapshot is captured at the same serialized transition as coins=0"
    );

    authority.add_viewer_like_cpp(late).unwrap();
    assert_eq!(
        authority
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .players_looting,
        vec![first, late],
        "a later opener observes zero directly and is not retroactively part of the commit fanout"
    );
}

#[tokio::test]
async fn persistence_commit_snapshot_excludes_viewers_opened_after_item_transition_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let late = player(2);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![first, late])])),
        HashMap::new(),
    );
    authority.add_viewer_like_cpp(first).unwrap();

    let claim = authority.reserve_item_like_cpp(first, 1).await.unwrap();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
    let (first_commit, committed_snapshot) = persistence.commit_with_snapshot_like_cpp().unwrap();
    assert!(first_commit);
    let committed_snapshot = committed_snapshot.unwrap();
    assert_eq!(committed_snapshot.loot.players_looting, vec![first]);
    assert!(committed_snapshot.loot.items[0].taken);

    let (_, late_snapshot) = authority
        .open_view_with_snapshot_like_cpp(late, |snapshot, _| snapshot.clone())
        .unwrap();
    assert_eq!(late_snapshot.loot.players_looting, vec![first, late]);
    assert!(late_snapshot.loot.items[0].taken);
    assert_eq!(
        committed_snapshot.loot.players_looting,
        vec![first],
        "a viewer that first observes the consumed item is outside the commit fanout cut"
    );
}

#[tokio::test]
async fn one_personal_persistence_does_not_block_peer_scope_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.upsert_personal_like_cpp(
        first,
        loot(owner(1), 0, vec![entry(1, false, vec![first])]),
        false,
    );
    let first_claim = authority.reserve_item_like_cpp(first, 1).await.unwrap();
    let mut first_persistence = first_claim.begin_persistence_guard_like_cpp().unwrap();

    assert!(
        authority
            .upsert_personal_like_cpp(
                second,
                loot(owner(1), 0, vec![entry(1, false, vec![second])]),
                false,
            )
            .installed(),
        "P1 durable work must not serialize an independent P2 pool"
    );
    let second_claim = authority.reserve_item_like_cpp(second, 1).await.unwrap();
    assert!(second_claim.commit_like_cpp().unwrap());

    assert!(
        !authority
            .upsert_personal_like_cpp(
                first,
                loot(owner(1), 0, vec![entry(2, false, vec![first])]),
                true,
            )
            .installed(),
        "the exact persisting P1 pool cannot be replaced"
    );
    assert!(first_persistence.commit_like_cpp().unwrap());
}

#[test]
fn lifecycle_observation_is_invalidated_by_late_personal_upsert_like_cpp() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let late = player(2);
    authority.replace_like_cpp(Some(loot(owner(1), 0, vec![])), HashMap::new());
    authority.add_viewer_like_cpp(first).unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;
    let close = authority
        .close_viewer_if_generation_like_cpp(generation, first)
        .unwrap();
    assert!(close.whole_object_fully_looted);

    let applications = AtomicUsize::new(0);
    assert!(
        authority
            .with_fully_looted_lifecycle_observation_like_cpp(
                close.object_generation,
                close.lifecycle_revision,
                || applications.fetch_add(1, Ordering::SeqCst),
            )
            .is_some()
    );
    assert!(
        authority
            .upsert_personal_like_cpp(
                late,
                loot(owner(1), 0, vec![entry(1, false, vec![late])]),
                false,
            )
            .installed()
    );
    assert!(
        authority
            .with_fully_looted_lifecycle_observation_like_cpp(
                close.object_generation,
                close.lifecycle_revision,
                || applications.fetch_add(1, Ordering::SeqCst),
            )
            .is_none(),
        "an upsert after close must invalidate the pre-upsert lifecycle observation"
    );
    assert_eq!(applications.load(Ordering::SeqCst), 1);
}

#[test]
fn detached_lifecycle_observation_requires_every_loot_view_to_be_closed_like_cpp() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.replace_like_cpp(Some(loot(owner(1), 0, vec![])), HashMap::new());
    let first_open = authority.add_viewer_like_cpp(first).unwrap();
    let second_open = authority.add_viewer_like_cpp(second).unwrap();

    authority
        .close_viewer_if_generation_like_cpp(second_open.generation, second)
        .unwrap();
    assert!(
        authority
            .fully_looted_unviewed_lifecycle_observation_like_cpp()
            .is_none(),
        "the remaining viewer belongs to the global authority, not the detached worker session"
    );

    authority
        .close_viewer_if_generation_like_cpp(first_open.generation, first)
        .unwrap();
    let observation = authority
        .fully_looted_unviewed_lifecycle_observation_like_cpp()
        .expect("the fully-looted owner is unviewed after both releases");
    let applications = AtomicUsize::new(0);
    assert!(
        authority
            .with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                observation.object_generation,
                observation.lifecycle_revision,
                || applications.fetch_add(1, Ordering::SeqCst),
            )
            .is_some()
    );

    authority.add_viewer_like_cpp(first).unwrap();
    assert!(
        authority
            .with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                observation.object_generation,
                observation.lifecycle_revision,
                || applications.fetch_add(1, Ordering::SeqCst),
            )
            .is_none(),
        "a view opened after the snapshot must still veto the serialized lifecycle mutation"
    );
    assert_eq!(applications.load(Ordering::SeqCst), 1);
}

#[test]
fn round_robin_clear_is_generation_guarded_and_returns_authoritative_snapshot_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let mut shared = loot(owner(1), 0, vec![entry(1, false, vec![first, second])]);
    shared.round_robin_player = first;
    authority.replace_like_cpp(Some(shared), HashMap::new());
    let generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;

    let cleared = authority
        .clear_round_robin_if_generation_like_cpp(generation, first)
        .unwrap();
    assert!(cleared.cleared);
    assert!(cleared.snapshot.loot.round_robin_player.is_empty());

    let mut replacement = loot(owner(1), 0, vec![entry(1, false, vec![first, second])]);
    replacement.round_robin_player = second;
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    assert!(
        authority
            .clear_round_robin_if_generation_like_cpp(generation, second)
            .is_none(),
        "a stale release cannot clear round robin on a replacement pool"
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(second)
            .unwrap()
            .loot
            .round_robin_player,
        second
    );
}

#[tokio::test]
async fn wait_for_persisting_claims_observes_commit_and_failure_boundaries_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
        HashMap::new(),
    );
    let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
    let waiting = authority.clone();
    let waiter = tokio::spawn(async move {
        waiting.wait_for_persisting_claims_like_cpp().await;
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());
    assert!(persistence.commit_like_cpp().unwrap());
    waiter.await.unwrap();

    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(2, false, vec![looter])])),
        HashMap::new(),
    );
    let failed = authority.reserve_item_like_cpp(looter, 2).await.unwrap();
    let persistence = failed.begin_persistence_guard_like_cpp().unwrap();
    drop(persistence);
    authority.wait_for_persisting_claims_like_cpp().await;
    assert!(authority.reserve_item_like_cpp(looter, 2).await.is_ok());
}

#[tokio::test]
async fn commit_unknown_quarantine_is_terminal_fail_closed_and_drains_waiters_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    let original = loot(owner(1), 0, vec![entry(1, false, vec![looter])]);
    authority.replace_like_cpp(Some(original.clone()), HashMap::new());
    let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();
    let waiting = authority.clone();
    let waiter = tokio::spawn(async move {
        waiting.wait_for_persisting_claims_like_cpp().await;
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());

    assert!(persistence.quarantine_commit_unknown_like_cpp());
    drop(persistence);
    tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("quarantine removes the persisting token")
        .unwrap();

    assert_eq!(
        authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Quarantined
    );
    assert_eq!(
        claim.commit_like_cpp(),
        Err(LootClaimCommitError::StateChanged)
    );
    assert!(matches!(
        authority.reserve_item_like_cpp(looter, 1).await,
        Err(LootClaimError::Retired)
    ));
    assert_eq!(
        authority.replace_like_cpp(Some(original.clone()), HashMap::new()),
        0
    );
    assert!(
        !authority
            .initialize_pristine_like_cpp(Some(original), HashMap::new())
            .installed()
    );
}

#[tokio::test]
async fn whole_object_skinned_requires_every_active_skinning_pool_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let mut first_pool = loot(owner(1), 0, vec![]);
    first_pool.loot_type = wow_constants::LootType::Skinning as u8;
    let mut second_pool = loot(owner(1), 0, vec![entry(1, false, vec![second])]);
    second_pool.loot_type = wow_constants::LootType::Skinning as u8;
    authority.replace_like_cpp(
        None,
        [(first, first_pool), (second, second_pool)]
            .into_iter()
            .collect(),
    );
    authority.add_viewer_like_cpp(first).unwrap();
    let first_generation = authority
        .snapshot_for_player_like_cpp(first)
        .unwrap()
        .generation;
    let first_close = authority
        .close_viewer_if_generation_like_cpp(first_generation, first)
        .unwrap();
    assert!(first_close.snapshot.loot.is_looted_like_cpp());
    assert!(!first_close.whole_object_fully_looted);
    assert!(!first_close.whole_object_fully_skinned);

    authority
        .reserve_item_like_cpp(second, 1)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    authority.add_viewer_like_cpp(second).unwrap();
    let second_generation = authority
        .snapshot_for_player_like_cpp(second)
        .unwrap()
        .generation;
    let second_close = authority
        .close_viewer_if_generation_like_cpp(second_generation, second)
        .unwrap();
    assert!(second_close.whole_object_fully_looted);
    assert!(second_close.whole_object_fully_skinned);
}
