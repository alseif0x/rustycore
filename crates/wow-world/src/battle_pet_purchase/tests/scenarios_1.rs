//! Battle-pet purchase tests, part 1 of 1.
//!
//! Moved out of the battle_pet_purchase.rs root under #656; every test is unchanged.

use super::*;

#[test]
fn status_codes_roundtrip_and_terminal_set_is_closed() {
    for (code, status, terminal) in [
        (0, BattlePetPurchaseStatusLikeCpp::PendingApplication, false),
        (1, BattlePetPurchaseStatusLikeCpp::Completed, true),
        (
            2,
            BattlePetPurchaseStatusLikeCpp::CompensationPending,
            false,
        ),
        (3, BattlePetPurchaseStatusLikeCpp::Compensated, true),
        (4, BattlePetPurchaseStatusLikeCpp::TerminalFailure, true),
    ] {
        assert_eq!(status.as_u8_like_cpp(), code);
        assert_eq!(
            BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(code),
            Some(status)
        );
        assert_eq!(status.is_terminal_like_cpp(), terminal);
    }
    assert_eq!(BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(5), None);
}

#[test]
fn durable_saga_schema_is_pinned_to_the_migration_file() {
    let migration = include_str!(
        "../../../../../sql/updates/characters/wotlk_classic/2026_08_03_00_characters.sql"
    );
    for needle in [
        "CREATE TABLE IF NOT EXISTS `character_battle_pet_purchase`",
        "`request_key` binary(16) NOT NULL",
        "`guid` bigint unsigned NOT NULL",
        "`status` tinyint unsigned NOT NULL",
        "`published` tinyint unsigned NOT NULL DEFAULT 0",
        "`money_before` bigint unsigned NOT NULL",
        "`money_after` bigint unsigned NOT NULL",
        "PRIMARY KEY (`request_key`)",
        "KEY `idx_guid_status` (`guid`,`status`)",
    ] {
        assert!(
            migration.contains(needle),
            "battle-pet purchase migration must contain {needle}"
        );
    }
}

#[tokio::test]
async fn charge_commits_money_and_command_exactly_once_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
    let command = test_command([1; 16], 9);
    let charged = store
        .charge_and_insert_command(command.clone(), test_money_commit_fence_like_cpp())
        .await
        .expect("charge must succeed");
    assert_eq!(charged, BattlePetPurchaseChargeOutcomeLikeCpp::Charged);
    assert_eq!(store.money(9), Some(750));
    assert_eq!(store.command([1; 16]), Some(command));
    assert_eq!(store.money_mutations(), 1);
}

#[tokio::test]
async fn same_token_charge_replay_is_attributed_without_a_second_charge_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
    let command = test_command([2; 16], 9);
    assert_eq!(
        store
            .charge_and_insert_command(command.clone(), test_money_commit_fence_like_cpp())
            .await
            .expect("first charge"),
        BattlePetPurchaseChargeOutcomeLikeCpp::Charged
    );
    // A raw same-token retry fails its guarded statements but the
    // reconcile must find the earlier commit instead of charging again.
    assert_eq!(
        store
            .charge_and_insert_command(command, test_money_commit_fence_like_cpp())
            .await
            .expect("replayed charge"),
        BattlePetPurchaseChargeOutcomeLikeCpp::Charged
    );
    assert_eq!(store.money(9), Some(750));
    assert_eq!(store.money_mutations(), 1);
}

#[tokio::test]
async fn colliding_token_with_different_payload_is_not_attributed_as_a_charge_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
    let existing = test_command([21; 16], 9);
    store.seed_command(existing);

    let mut colliding = test_command([21; 16], 9);
    colliding.species += 1;
    let outcome = store
        .charge_and_insert_command(colliding, test_money_commit_fence_like_cpp())
        .await
        .expect("a key collision is a definite rollback");

    assert_eq!(outcome, BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.money_mutations(), 0);
}

#[tokio::test]
async fn lost_charge_reply_reconciles_to_charged_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
    store.lose_next_charge_reply.store(true, Ordering::SeqCst);
    let charged = store
        .charge_and_insert_command(test_command([3; 16], 9), test_money_commit_fence_like_cpp())
        .await
        .expect("lost reply must reconcile through the durable row");
    assert_eq!(charged, BattlePetPurchaseChargeOutcomeLikeCpp::Charged);
    assert_eq!(store.money(9), Some(750));
    assert_eq!(store.money_mutations(), 1);
}

#[tokio::test]
async fn failed_charge_leaves_no_money_and_no_command_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
    store
        .fail_next_charge_pre_commit
        .store(true, Ordering::SeqCst);
    let outcome = store
        .charge_and_insert_command(test_command([4; 16], 9), test_money_commit_fence_like_cpp())
        .await
        .expect("pre-commit failure");
    assert_eq!(outcome, BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.command([4; 16]), None);
    assert_eq!(store.money_mutations(), 0);
}

#[tokio::test]
async fn guarded_charge_fails_closed_when_money_moved_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 2_000);
    let outcome = store
        .charge_and_insert_command(test_command([5; 16], 9), test_money_commit_fence_like_cpp())
        .await
        .expect("guarded charge");
    assert_eq!(outcome, BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
    assert_eq!(store.money(9), Some(2_000));
    assert_eq!(store.command([5; 16]), None);
}

#[tokio::test]
async fn compensation_refunds_exactly_once_across_replays_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
    let mut command = test_command([6; 16], 9);
    command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
    store.seed_command(command);
    assert_eq!(
        store
            .compensate(
                [6; 16],
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
            durable_money: 1_000
        }
    );
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.money_mutations(), 1);
    assert_eq!(
        store
            .compensate(
                [6; 16],
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("replayed compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
            durable_money: 1_000
        }
    );
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.money_mutations(), 1);
}

#[tokio::test]
async fn lost_compensation_reply_still_refunds_exactly_once_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
    let mut command = test_command([7; 16], 9);
    command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
    store.seed_command(command);
    store
        .lose_next_compensate_reply
        .store(true, Ordering::SeqCst);
    // The lost reply belongs to this call's own committed refund.
    assert_eq!(
        store
            .compensate(
                [7; 16],
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
            durable_money: 1_000
        }
    );
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.money_mutations(), 1);
    // A later replay sees the durable flip and does not refund again.
    assert_eq!(
        store
            .compensate(
                [7; 16],
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("replayed compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
            durable_money: 1_000
        }
    );
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.money_mutations(), 1);
}

#[tokio::test]
async fn committed_refund_with_unreadable_money_keeps_persistence_quarantined_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
    let mut command = test_command([10; 16], 9);
    command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
    store.seed_command(command);
    store
        .fail_next_compensate_post_apply_read
        .store(true, Ordering::SeqCst);
    let money_tracker =
        Arc::new(crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let outcome = store
        .compensate(
            [10; 16],
            wow_entities::MAX_MONEY_AMOUNT,
            Box::new(
                PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::clone(
                    &money_tracker,
                )),
            ),
        )
        .await;
    assert!(matches!(
        outcome,
        Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(_))
    ));
    assert_eq!(store.money(9), Some(1_000));
    assert_eq!(store.money_mutations(), 1);
    assert_eq!(
        store.command([10; 16]).expect("command").status,
        BattlePetPurchaseStatusLikeCpp::Compensated
    );
    assert!(money_tracker.is_indeterminate_like_cpp());
}

#[tokio::test]
async fn compensation_never_refunds_a_completed_command_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
    let mut command = test_command([8; 16], 9);
    command.status = BattlePetPurchaseStatusLikeCpp::Completed;
    store.seed_command(command);
    assert_eq!(
        store
            .compensate(
                [8; 16],
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted
    );
    assert_eq!(store.money(9), Some(750));
    assert_eq!(store.money_mutations(), 0);
}

#[tokio::test]
async fn missing_character_compensation_is_terminal_not_retried_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new();
    let mut command = test_command([9; 16], 9);
    command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
    store.seed_command(command);
    assert_eq!(
        store
            .compensate(
                [9; 16],
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing
    );
}

#[tokio::test]
async fn pending_scan_returns_only_unconverged_commands_bounded_like_cpp() {
    let store = FakeBattlePetPurchaseStoreLikeCpp::new();
    for (index, status) in [
        BattlePetPurchaseStatusLikeCpp::PendingApplication,
        BattlePetPurchaseStatusLikeCpp::CompensationPending,
        BattlePetPurchaseStatusLikeCpp::Completed,
        BattlePetPurchaseStatusLikeCpp::Compensated,
        BattlePetPurchaseStatusLikeCpp::TerminalFailure,
    ]
    .into_iter()
    .enumerate()
    {
        let mut command = test_command([index as u8 + 10; 16], 9);
        command.status = status;
        // A Completed row is converged only once its publication was
        // recorded; this one is fully converged.
        command.published = true;
        store.seed_command(command);
    }
    // A Completed row that was never published is still owed its
    // publication and must be scanned.
    let mut owed = test_command([99; 16], 9);
    owed.status = BattlePetPurchaseStatusLikeCpp::Completed;
    owed.published = false;
    store.seed_command(owed);
    let pending = store
        .load_pending_commands(9, BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP)
        .await
        .expect("scan");
    assert_eq!(pending.len(), 3);
    assert!(
        pending
            .iter()
            .all(|command| !command.status.is_terminal_like_cpp()
                || command.status == BattlePetPurchaseStatusLikeCpp::Completed)
    );
    let bounded = store.load_pending_commands(9, 1).await.expect("scan");
    assert_eq!(bounded.len(), 1);
}
