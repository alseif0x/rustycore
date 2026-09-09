//! Owned-loot authority, claims and leases regression scenarios, part 1 of 2.
//!
//! Moved out of the authority.rs root under #642; every test is unchanged.

use super::*;

#[test]
fn equal_authority_state_does_not_imply_shared_storage_like_cpp() {
    let first = OwnedLootAuthority::new();
    let second = OwnedLootAuthority::new();
    let shared_loot = loot(owner(1), 17, Vec::new());

    first.replace_like_cpp(Some(shared_loot.clone()), HashMap::new());
    second.replace_like_cpp(Some(shared_loot), HashMap::new());

    assert_eq!(first, second, "the independent authority states match");
    assert!(
        !first.shares_storage_like_cpp(&second),
        "equal state must not hide two independently claimable Arc owners"
    );
    assert!(first.shares_storage_like_cpp(&first.clone()));
}

#[test]
fn personal_map_suppresses_shared_pool_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let mut personal = HashMap::new();
    personal.insert(first, loot(owner(2), 9, Vec::new()));
    authority.replace_like_cpp(Some(loot(owner(1), 5, Vec::new())), personal);

    let first_snapshot = authority
        .snapshot_for_player_like_cpp(first)
        .expect("personal owner has loot");
    assert_eq!(first_snapshot.scope, OwnedLootScope::Personal(first));
    assert_eq!(first_snapshot.loot.coins, 9);
    assert!(authority.snapshot_for_player_like_cpp(second).is_none());
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 5);
}

#[test]
fn initialize_is_first_writer_wins_and_retire_invalidates_generation() {
    let authority = OwnedLootAuthority::new();
    let first = authority.initialize_like_cpp(Some(loot(owner(1), 5, Vec::new())), HashMap::new());
    let second = authority.initialize_like_cpp(Some(loot(owner(1), 9, Vec::new())), HashMap::new());
    assert!(matches!(first, LootInstallOutcome::Installed { .. }));
    assert_eq!(first.generation(), second.generation());
    assert!(!second.installed());
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 5);

    let retired_generation = authority.retire_like_cpp();
    assert!(retired_generation > first.generation());
    assert!(authority.shared_snapshot_like_cpp().is_none());
    assert_eq!(authority.retire_like_cpp(), retired_generation);
}

#[tokio::test]
async fn normal_item_has_one_winner_and_waiter_observes_commit() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.replace_like_cpp(
        Some(loot(
            owner(1),
            0,
            vec![entry(1, false, vec![first, second])],
        )),
        HashMap::new(),
    );

    let first_lease = authority.reserve_item_like_cpp(first, 1).await.unwrap();
    let waiting_authority = authority.clone();
    let waiter =
        tokio::spawn(async move { waiting_authority.reserve_item_like_cpp(second, 1).await });
    tokio::task::yield_now().await;
    first_lease.commit_like_cpp().unwrap();

    assert_eq!(
        waiter.await.unwrap().unwrap_err(),
        LootClaimError::ItemAlreadyLooted
    );
    assert!(authority.shared_snapshot_like_cpp().unwrap().loot.items[0].taken);
}

#[tokio::test]
async fn item_claim_requires_allowed_looter_and_direct_policy_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, Vec::new())])),
        HashMap::new(),
    );
    assert_eq!(
        authority
            .reserve_item_like_cpp(looter, 1)
            .await
            .unwrap_err(),
        LootClaimError::PlayerNotAllowed
    );

    let mut blocked = entry(1, false, vec![looter]);
    blocked.flags.blocked = true;
    authority.replace_like_cpp(Some(loot(owner(1), 0, vec![blocked])), HashMap::new());
    assert_eq!(
        authority
            .reserve_item_like_cpp(looter, 1)
            .await
            .unwrap_err(),
        LootClaimError::ItemBlocked
    );
    let award = authority
        .reserve_item_for_award_like_cpp(looter, 1)
        .await
        .expect("completed award may claim the blocked slot");
    assert!(award.commit_like_cpp().unwrap());
    assert!(!award.commit_like_cpp().unwrap(), "commit is idempotent");

    let other = player(2);
    let mut won = entry(2, false, vec![looter, other]);
    won.roll_winner = other;
    authority.replace_like_cpp(Some(loot(owner(1), 0, vec![won])), HashMap::new());
    assert_eq!(
        authority
            .reserve_item_for_award_like_cpp(looter, 2)
            .await
            .unwrap_err(),
        LootClaimError::WrongRollWinner
    );
    authority
        .reserve_item_for_award_like_cpp(other, 2)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
}

#[tokio::test]
async fn finished_roll_state_is_published_before_later_direct_claims() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let mut rolled = entry(1, false, vec![first, second]);
    rolled.flags.blocked = true;
    authority.replace_like_cpp(Some(loot(owner(1), 0, vec![rolled])), HashMap::new());
    let roll_generation = authority.shared_snapshot_like_cpp().unwrap().generation;

    assert!(
        authority
            .finish_item_roll_like_cpp(first, roll_generation, 1, false, Some(second))
            .unwrap()
    );
    assert_eq!(
        authority.reserve_item_like_cpp(first, 1).await.unwrap_err(),
        LootClaimError::WrongRollWinner
    );
    authority
        .reserve_item_like_cpp(second, 1)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();

    let mut single_candidate = entry(2, false, vec![first]);
    single_candidate.flags.blocked = true;
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![single_candidate])),
        HashMap::new(),
    );
    let roll_generation = authority.shared_snapshot_like_cpp().unwrap().generation;
    authority
        .finish_item_roll_like_cpp(first, roll_generation, 2, true, None)
        .unwrap();
    let snapshot = authority.shared_snapshot_like_cpp().unwrap();
    assert!(snapshot.loot.items[0].flags.under_threshold);
    assert!(!snapshot.loot.items[0].flags.blocked);
    authority
        .reserve_item_like_cpp(first, 2)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
}

#[tokio::test]
async fn final_clone_drop_rolls_back_and_wakes_waiter() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
        HashMap::new(),
    );

    let lease = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let clone = lease.clone();
    let waiting_authority = authority.clone();
    let waiter =
        tokio::spawn(async move { waiting_authority.reserve_item_like_cpp(looter, 1).await });
    drop(lease);
    tokio::task::yield_now().await;
    assert!(
        !waiter.is_finished(),
        "one clone still owns the reservation"
    );
    drop(clone);

    let retry = waiter.await.unwrap().unwrap();
    assert!(matches!(
        retry.payload_like_cpp(),
        LootClaimPayload::Item(_)
    ));
    retry.commit_like_cpp().unwrap();
}

#[tokio::test]
async fn change_between_busy_check_and_wait_poll_is_not_lost() {
    use std::time::Duration;

    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
        HashMap::new(),
    );
    let first = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let mut changed = authority.inner.changed.subscribe();
    let busy = {
        let mut state = authority.lock_state();
        reserve_item_once(&mut state, looter, 1, LootItemClaimMode::Direct, None)
    };
    assert!(matches!(busy, ReserveAttempt::Wait));

    // This transition is deliberately between the locked Busy result and the first poll of
    // `changed()`. A watch version remembers it; Notify::notify_waiters would lose it.
    first.rollback_like_cpp();
    tokio::time::timeout(Duration::from_secs(1), changed.changed())
        .await
        .expect("reservation wake must not be lost")
        .unwrap();
    let retry = {
        let mut state = authority.lock_state();
        reserve_item_once(&mut state, looter, 1, LootItemClaimMode::Direct, None)
    };
    assert!(matches!(retry, ReserveAttempt::Acquired { .. }));
    authority.retire_like_cpp();
}

#[tokio::test]
async fn ffa_item_is_reserved_and_consumed_once_per_player() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let mut shared = loot(owner(1), 0, vec![entry(1, true, vec![first, second])]);
    shared.unlooted_count = 2;
    shared.player_ffa_items = vec![
        (
            first,
            vec![NotNormalLootItem {
                loot_list_id: 1,
                is_looted: false,
            }],
        ),
        (
            second,
            vec![NotNormalLootItem {
                loot_list_id: 1,
                is_looted: false,
            }],
        ),
    ];
    authority.replace_like_cpp(Some(shared), HashMap::new());

    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for looter in [first, second] {
        let authority = authority.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let lease = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
            lease.commit_like_cpp().unwrap();
        }));
    }
    barrier.wait().await;
    for task in tasks {
        task.await.unwrap();
    }

    let shared = authority.shared_snapshot_like_cpp().unwrap().loot;
    assert_eq!(shared.unlooted_count, 0);
    assert!(shared.items[0].fully_looted_like_cpp());
    assert_eq!(
        authority.reserve_item_like_cpp(first, 1).await.unwrap_err(),
        LootClaimError::ItemAlreadyLooted
    );
    assert_eq!(
        authority
            .reserve_item_like_cpp(second, 1)
            .await
            .unwrap_err(),
        LootClaimError::ItemAlreadyLooted
    );
}

#[tokio::test]
async fn personal_claims_and_ae_owners_are_independent() {
    let first = player(1);
    let second = player(2);
    let authority = OwnedLootAuthority::new();
    let mut personal = HashMap::new();
    personal.insert(first, loot(owner(1), 0, vec![entry(1, false, vec![first])]));
    personal.insert(
        second,
        loot(owner(1), 0, vec![entry(1, false, vec![second])]),
    );
    authority.replace_like_cpp(None, personal);

    let first_lease = authority.reserve_item_like_cpp(first, 1).await.unwrap();
    let second_lease = authority.reserve_item_like_cpp(second, 1).await.unwrap();
    first_lease.commit_like_cpp().unwrap();
    second_lease.commit_like_cpp().unwrap();

    let first_ae = OwnedLootAuthority::new();
    let second_ae = OwnedLootAuthority::new();
    first_ae.replace_like_cpp(Some(money_loot(owner(10), 7, vec![first])), HashMap::new());
    second_ae.replace_like_cpp(
        Some(money_loot(owner(11), 11, vec![second])),
        HashMap::new(),
    );
    first_ae
        .reserve_money_like_cpp(first)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert_eq!(first_ae.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
    assert_eq!(second_ae.shared_snapshot_like_cpp().unwrap().loot.coins, 11);
}

#[tokio::test]
async fn stale_lease_cannot_touch_replacement_generation() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(Some(money_loot(owner(1), 5, vec![looter])), HashMap::new());
    let stale = authority.reserve_money_like_cpp(looter).await.unwrap();

    let retired_generation = authority.retire_like_cpp();
    authority
        .replace_retired_generation_like_cpp(
            retired_generation,
            Some(money_loot(owner(1), 9, vec![looter])),
            HashMap::new(),
        )
        .unwrap();
    assert!(stale.commit_like_cpp().is_err());
    drop(stale);
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 9);
}

#[tokio::test]
async fn allowed_player_can_reserve_and_commit_money_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(Some(money_loot(owner(1), 17, vec![looter])), HashMap::new());

    let claim = authority.reserve_money_like_cpp(looter).await.unwrap();
    assert_eq!(claim.payload_like_cpp(), &LootClaimPayload::Money(17));
    assert!(claim.commit_like_cpp().unwrap());
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
}

#[tokio::test]
async fn money_claim_rejects_player_not_in_allowed_looters_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let allowed = player(1);
    let denied = player(2);
    authority.replace_like_cpp(
        Some(money_loot(owner(1), 19, vec![allowed])),
        HashMap::new(),
    );

    assert_eq!(
        authority.reserve_money_like_cpp(denied).await.unwrap_err(),
        LootClaimError::PlayerNotAllowed
    );
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 19);
}

#[tokio::test]
async fn stale_player_cannot_claim_replacement_generation_money_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let stale_looter = player(1);
    let current_looter = player(2);
    authority.replace_like_cpp(
        Some(money_loot(owner(1), 5, vec![stale_looter])),
        HashMap::new(),
    );
    let stale_generation = authority.generation_like_cpp();

    let retired_generation = authority.retire_like_cpp();
    authority
        .replace_retired_generation_like_cpp(
            retired_generation,
            Some(money_loot(owner(1), 23, vec![current_looter])),
            HashMap::new(),
        )
        .unwrap();
    assert!(authority.generation_like_cpp() > stale_generation);
    assert_eq!(
        authority
            .reserve_money_like_cpp(stale_looter)
            .await
            .unwrap_err(),
        LootClaimError::PlayerNotAllowed
    );
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 23);

    let current_claim = authority
        .reserve_money_like_cpp(current_looter)
        .await
        .unwrap();
    assert!(current_claim.commit_like_cpp().unwrap());
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
}

#[tokio::test]
async fn money_claim_rolls_back_and_later_cpp_request_observes_zero() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.replace_like_cpp(
        Some(money_loot(owner(1), 25, vec![first, second])),
        HashMap::new(),
    );

    let abandoned = authority.reserve_money_like_cpp(first).await.unwrap();
    assert!(abandoned.rollback_like_cpp());
    let winner = authority.reserve_money_like_cpp(second).await.unwrap();
    assert_eq!(winner.payload_like_cpp(), &LootClaimPayload::Money(25));
    assert!(winner.commit_like_cpp().unwrap());
    assert!(!winner.commit_like_cpp().unwrap());
    let zero = authority.reserve_money_like_cpp(first).await.unwrap();
    assert_eq!(zero.payload_like_cpp(), &LootClaimPayload::Money(0));
    assert!(zero.commit_like_cpp().unwrap());
}

#[tokio::test]
async fn concurrent_money_waiter_observes_the_single_committed_winner() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.replace_like_cpp(
        Some(money_loot(owner(1), 31, vec![first, second])),
        HashMap::new(),
    );

    let winner = authority.reserve_money_like_cpp(first).await.unwrap();
    let waiting_authority = authority.clone();
    let waiter =
        tokio::spawn(async move { waiting_authority.reserve_money_like_cpp(second).await });
    tokio::task::yield_now().await;
    assert!(winner.commit_like_cpp().unwrap());
    let zero = waiter.await.unwrap().unwrap();
    assert_eq!(zero.payload_like_cpp(), &LootClaimPayload::Money(0));
    assert!(zero.commit_like_cpp().unwrap());
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
}

#[test]
fn personal_pools_can_be_installed_incrementally_without_replacing_peers() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.upsert_personal_like_cpp(first, loot(owner(1), 7, Vec::new()), false);
    let generation = authority.generation_like_cpp();
    authority.upsert_personal_like_cpp(second, loot(owner(1), 11, Vec::new()), false);

    assert_eq!(authority.generation_like_cpp(), generation);
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(first)
            .unwrap()
            .loot
            .coins,
        7
    );
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(second)
            .unwrap()
            .loot
            .coins,
        11
    );
    let duplicate =
        authority.upsert_personal_like_cpp(first, loot(owner(1), 99, Vec::new()), false);
    assert!(!duplicate.installed());
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(first)
            .unwrap()
            .loot
            .coins,
        7
    );
}

#[tokio::test]
async fn replacing_one_personal_pool_does_not_stale_another_players_claim() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let mut first_loot = loot(owner(1), 0, vec![entry(1, false, vec![first])]);
    first_loot.allowed_looters = vec![first];
    authority.upsert_personal_like_cpp(first, first_loot, false);
    let mut second_loot = loot(owner(1), 0, vec![entry(1, false, vec![second])]);
    second_loot.allowed_looters = vec![second];
    authority.upsert_personal_like_cpp(second, second_loot, false);

    let stale_first_item = authority.reserve_item_like_cpp(first, 1).await.unwrap();
    let stale_first_money = authority.reserve_money_like_cpp(first).await.unwrap();
    let second_claim = authority.reserve_item_like_cpp(second, 1).await.unwrap();
    let first_epoch = authority
        .personal_snapshot_like_cpp(first)
        .unwrap()
        .generation;
    let second_epoch = authority
        .personal_snapshot_like_cpp(second)
        .unwrap()
        .generation;
    let generation = authority.generation_like_cpp();
    let mut replacement = loot(owner(1), 9, vec![entry(2, false, vec![first])]);
    replacement.allowed_looters = vec![first];
    authority.upsert_personal_like_cpp(first, replacement, true);

    assert_eq!(authority.generation_like_cpp(), generation);
    assert_ne!(
        authority
            .personal_snapshot_like_cpp(first)
            .unwrap()
            .generation,
        first_epoch
    );
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(second)
            .unwrap()
            .generation,
        second_epoch
    );
    assert_eq!(
        stale_first_item.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration)
    );
    assert_eq!(
        stale_first_money.commit_like_cpp(),
        Err(LootClaimCommitError::StaleGeneration)
    );
    assert!(second_claim.commit_like_cpp().unwrap());
    assert!(
        authority
            .personal_snapshot_like_cpp(second)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(first)
            .unwrap()
            .loot
            .coins,
        9
    );
    assert!(
        authority
            .reserve_item_like_cpp(first, 2)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert!(
        authority
            .reserve_money_like_cpp(first)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
}

#[tokio::test]
async fn stale_personal_waiter_rejects_replacement_epoch_without_reserving_it() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.upsert_personal_like_cpp(
        looter,
        loot(owner(1), 0, vec![entry(1, false, vec![looter])]),
        false,
    );
    let old_epoch = authority
        .personal_snapshot_like_cpp(looter)
        .unwrap()
        .generation;
    let held = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let waiting_authority = authority.clone();
    let waiter = tokio::spawn(async move {
        waiting_authority
            .reserve_item_for_generation_like_cpp(looter, 1, old_epoch)
            .await
    });
    tokio::task::yield_now().await;

    authority.upsert_personal_like_cpp(
        looter,
        loot(owner(1), 0, vec![entry(1, false, vec![looter])]),
        true,
    );
    assert_eq!(
        waiter.await.unwrap().unwrap_err(),
        LootClaimError::StaleGeneration
    );
    drop(held);
    assert!(
        authority
            .reserve_item_like_cpp(looter, 1)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
}

#[test]
fn pristine_bridge_cannot_resurrect_a_retired_nonzero_generation() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    let first = authority
        .initialize_pristine_like_cpp(Some(money_loot(owner(1), 5, vec![looter])), HashMap::new());
    assert!(first.installed());
    authority.retire_like_cpp();

    let stale = authority
        .initialize_pristine_like_cpp(Some(money_loot(owner(1), 99, vec![looter])), HashMap::new());
    assert!(!stale.installed());
    assert!(
        !authority
            .initialize_like_cpp(Some(money_loot(owner(1), 98, vec![looter])), HashMap::new(),)
            .installed()
    );
    assert!(
        !authority
            .initialize_shared_like_cpp(money_loot(owner(1), 97, vec![looter]))
            .installed()
    );
    assert!(
        !authority
            .upsert_personal_like_cpp(looter, money_loot(owner(1), 96, vec![looter]), true,)
            .installed()
    );
    assert!(authority.is_retired_like_cpp());
    assert!(authority.shared_snapshot_like_cpp().is_none());
}

#[test]
fn viewer_first_open_is_atomic_and_retire_clears_it() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    authority.replace_like_cpp(Some(loot(owner(1), 1, Vec::new())), HashMap::new());

    let first_open = authority.add_viewer_like_cpp(first).unwrap();
    let duplicate = authority.add_viewer_like_cpp(first).unwrap();
    let second_open = authority.add_viewer_like_cpp(second).unwrap();
    assert!(first_open.first_viewer);
    assert!(!duplicate.inserted);
    assert!(!second_open.first_viewer);
    assert_eq!(
        authority.viewers_for_player_like_cpp(first),
        vec![first, second]
    );
    assert!(authority.remove_viewer_like_cpp(first));
    assert!(!authority.remove_viewer_like_cpp(first));
    assert!(authority.remove_viewer_like_cpp(second));
    let reopened = authority.add_viewer_like_cpp(first).unwrap();
    assert!(reopened.inserted);
    assert!(
        !reopened.first_viewer,
        "C++ Loot::_wasOpened survives an empty active-looter set"
    );
    authority.retire_like_cpp();
    assert!(authority.viewers_for_player_like_cpp(second).is_empty());
}

#[test]
fn rejected_view_response_rolls_back_viewer_and_first_open_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let viewer = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![viewer])])),
        HashMap::new(),
    );

    assert_eq!(
        authority.try_open_view_with_snapshot_like_cpp(viewer, |_, _| None::<()>),
        Err(LootClaimError::ResponseEnqueueFailed)
    );
    let rejected = authority.shared_snapshot_like_cpp().unwrap();
    assert!(rejected.loot.players_looting.is_empty());
    assert!(!rejected.loot.looted_by_player);

    let retry = authority.add_viewer_like_cpp(viewer).unwrap();
    assert!(retry.inserted);
    assert!(
        retry.first_viewer,
        "a response the client never observed must not consume C++ Loot::_wasOpened"
    );
}

#[test]
fn exact_generation_release_removes_only_that_viewer() {
    let authority = OwnedLootAuthority::new();
    let first = player(1);
    let second = player(2);
    let generation = authority.replace_like_cpp(
        Some(money_loot(owner(1), 17, vec![first, second])),
        HashMap::new(),
    );
    authority.add_viewer_like_cpp(first).unwrap();
    authority.add_viewer_like_cpp(second).unwrap();

    assert_eq!(
        authority.remove_viewer_if_generation_like_cpp(generation, first),
        Some(true)
    );
    assert_eq!(authority.viewers_for_player_like_cpp(second), vec![second]);
    assert_eq!(
        authority.remove_viewer_if_generation_like_cpp(generation, first),
        Some(false)
    );
    assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 17);
}

#[test]
fn retired_generation_release_cannot_touch_replacement_viewer_or_pool() {
    let authority = OwnedLootAuthority::new();
    let viewer = player(1);
    let old_generation =
        authority.replace_like_cpp(Some(money_loot(owner(1), 7, vec![viewer])), HashMap::new());
    authority.add_viewer_like_cpp(viewer).unwrap();

    authority.retire_like_cpp();
    let replacement_generation =
        authority.replace_like_cpp(Some(money_loot(owner(1), 29, vec![viewer])), HashMap::new());
    authority.add_viewer_like_cpp(viewer).unwrap();

    assert_ne!(replacement_generation, old_generation);
    assert_eq!(
        authority.remove_viewer_if_generation_like_cpp(old_generation, viewer),
        None
    );
    let replacement = authority.shared_snapshot_like_cpp().unwrap();
    assert_eq!(replacement.generation, replacement_generation);
    assert_eq!(replacement.loot.coins, 29);
    assert_eq!(replacement.loot.players_looting, vec![viewer]);
    assert_eq!(replacement.loot.allowed_looters, vec![viewer]);
}

#[tokio::test]
async fn persistence_guard_survives_retire_and_closes_after_commit_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
        HashMap::new(),
    );
    let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();

    let retired_generation = authority.retire_like_cpp();
    assert_eq!(
        authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Retired
    );
    assert_eq!(
        claim.commit_like_cpp(),
        Err(LootClaimCommitError::StateChanged),
        "only the durable-phase owner may resolve a protected reservation"
    );
    assert!(!claim.rollback_like_cpp());
    assert!(persistence.commit_like_cpp().unwrap());

    assert!(authority.is_retired_like_cpp());
    assert!(authority.shared_snapshot_like_cpp().is_none());
    assert!(
        authority
            .replace_retired_generation_like_cpp(
                retired_generation,
                Some(money_loot(owner(1), 9, vec![looter])),
                HashMap::new(),
            )
            .is_some(),
        "the closed lifetime may respawn only after durable completion"
    );
}

#[tokio::test]
async fn persistence_guard_survives_detach_but_detached_owner_never_reopens_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
        HashMap::new(),
    );
    let claim = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();

    let detached_generation = authority.detach_like_cpp();
    assert_eq!(
        authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached
    );
    assert!(persistence.commit_like_cpp().unwrap());
    assert_eq!(
        authority.replace_like_cpp(Some(money_loot(owner(1), 9, vec![looter])), HashMap::new(),),
        0
    );
    assert!(
        authority
            .replace_retired_generation_like_cpp(
                detached_generation,
                Some(money_loot(owner(1), 9, vec![looter])),
                HashMap::new(),
            )
            .is_none()
    );
}

#[tokio::test]
async fn persistence_guard_is_unique_and_external_clones_cannot_resolve_it_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(Some(money_loot(owner(1), 17, vec![looter])), HashMap::new());
    let claim = authority.reserve_money_like_cpp(looter).await.unwrap();
    let clone = claim.clone();
    let mut persistence = claim.begin_persistence_guard_like_cpp().unwrap();

    assert_eq!(
        clone.begin_persistence_guard_like_cpp().unwrap_err(),
        LootClaimCommitError::StateChanged
    );
    assert_eq!(
        clone.commit_like_cpp(),
        Err(LootClaimCommitError::StateChanged)
    );
    assert!(!clone.rollback_like_cpp());
    assert!(persistence.commit_like_cpp().unwrap());
    assert_eq!(
        claim.begin_persistence_guard_like_cpp().unwrap_err(),
        LootClaimCommitError::StateChanged
    );
}

#[tokio::test]
async fn dropped_persistence_guard_reopens_claim_after_failure_like_cpp() {
    let authority = OwnedLootAuthority::new();
    let looter = player(1);
    authority.replace_like_cpp(
        Some(loot(owner(1), 0, vec![entry(1, false, vec![looter])])),
        HashMap::new(),
    );
    let failed = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    let persistence = failed.begin_persistence_guard_like_cpp().unwrap();
    drop(persistence);

    assert_eq!(
        failed.commit_like_cpp(),
        Err(LootClaimCommitError::RolledBack)
    );
    let retry = authority.reserve_item_like_cpp(looter, 1).await.unwrap();
    assert!(retry.commit_like_cpp().unwrap());
}
