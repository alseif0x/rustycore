use super::*;

#[test]
fn every_mode_executes_its_complete_ordered_obligation_set() {
    use FinalizationStep::*;
    for (mode, expected) in [
        (
            FinalizationMode::CharacterSelection,
            vec![
                NativeTransfer,
                LootSettlement,
                Buyback,
                CharacterSave,
                Mounts,
                Toys,
                Heirlooms,
                Appearances,
                Illusions,
                CharacterOffline,
                Retirement,
                LogoutPublication,
                CharacterAccountOffline,
                Release,
            ],
        ),
        (
            FinalizationMode::Disconnect,
            vec![
                NativeTransfer,
                LootSettlement,
                Buyback,
                CharacterSave,
                Mounts,
                Toys,
                Heirlooms,
                Appearances,
                Illusions,
                CharacterOffline,
                CharacterAccountOffline,
                LoginAccountOffline,
                Retirement,
                Release,
            ],
        ),
        (
            FinalizationMode::TimedLogout,
            vec![
                NativeTransfer,
                LootSettlement,
                Buyback,
                CharacterSave,
                Mounts,
                Toys,
                Heirlooms,
                Appearances,
                Illusions,
                CharacterOffline,
                Retirement,
                LogoutPublication,
                CharacterAccountOffline,
                LoginAccountOffline,
                Release,
            ],
        ),
    ] {
        let mut operation = SessionFinalization::new(mode, true, None);
        for step in expected {
            assert_eq!(operation.next_step(), Some(step));
            assert!(operation.begin(step));
            assert_eq!(
                operation.report().outcome(step),
                FinalizationOutcome::InFlight
            );
            assert!(operation.finish(step, FinalizationOutcome::Applied));
        }
        assert_eq!(operation.next_step(), None);
        operation.complete();
        assert_eq!(
            operation.report().disposition,
            FinalizationDisposition::Complete
        );
    }
}

#[test]
fn every_failed_or_cancelled_obligation_retains_and_prevents_replay() {
    for failure in [
        FinalizationOutcome::DefinitelyRolledBack,
        FinalizationOutcome::Unknown,
        FinalizationOutcome::Unavailable,
        FinalizationOutcome::Deferred,
        FinalizationOutcome::RetirementFailed,
        FinalizationOutcome::InFlight,
    ] {
        for mode in [
            FinalizationMode::CharacterSelection,
            FinalizationMode::Disconnect,
            FinalizationMode::TimedLogout,
        ] {
            let mut prefix = SessionFinalization::new(mode, true, None);
            while let Some(target) = prefix.next_step() {
                let mut operation = SessionFinalization::new(mode, true, None);
                while operation.next_step() != Some(target) {
                    let step = operation.next_step().unwrap();
                    assert!(operation.begin(step));
                    assert!(operation.finish(step, FinalizationOutcome::Applied));
                }
                assert!(operation.begin(target));
                if failure == FinalizationOutcome::InFlight {
                    operation.interrupt();
                } else {
                    assert!(!operation.finish(target, failure));
                }
                assert_eq!(operation.report().outcome(target), failure);
                assert_eq!(
                    operation.report().disposition,
                    FinalizationDisposition::RetainAndEscalate
                );
                assert!(!operation.begin(target));
                assert_eq!(operation.next_step(), None);
                operation.complete();
                assert_eq!(
                    operation.report().disposition,
                    FinalizationDisposition::RetainAndEscalate
                );
                assert!(prefix.begin(target));
                assert!(prefix.finish(target, FinalizationOutcome::Applied));
            }
        }
    }
}

#[test]
fn no_player_still_requires_login_account_offline_on_disconnect() {
    let mut operation = SessionFinalization::new(FinalizationMode::Disconnect, false, None);
    for step in [
        FinalizationStep::NativeTransfer,
        FinalizationStep::LoginAccountOffline,
        FinalizationStep::Release,
    ] {
        assert_eq!(operation.next_step(), Some(step));
        assert!(operation.begin(step));
        assert!(operation.finish(step, FinalizationOutcome::Applied));
    }
    operation.complete();
    assert_eq!(
        operation.report().disposition,
        FinalizationDisposition::Complete
    );
    assert_eq!(
        operation.report().outcome(FinalizationStep::CharacterSave),
        FinalizationOutcome::NotAttempted
    );
}

#[test]
fn character_save_success_is_not_finalization_success() {
    let mut operation = SessionFinalization::new(FinalizationMode::Disconnect, true, None);
    while let Some(step) = operation.next_step() {
        assert!(operation.begin(step));
        assert!(operation.finish(step, FinalizationOutcome::Applied));
        if step == FinalizationStep::CharacterSave {
            break;
        }
    }
    operation.complete();
    assert_eq!(
        operation.report().disposition,
        FinalizationDisposition::RetainAndEscalate
    );
    assert_eq!(
        operation.report().outcome(FinalizationStep::Retirement),
        FinalizationOutcome::NotAttempted
    );
}

#[test]
fn invalid_order_and_missing_receipts_never_complete() {
    let mut operation = SessionFinalization::new(FinalizationMode::Disconnect, true, None);
    assert!(!operation.begin(FinalizationStep::Release));
    assert_eq!(
        operation.report().disposition,
        FinalizationDisposition::RetainAndEscalate
    );
    let mut operation = SessionFinalization::new(FinalizationMode::Disconnect, true, None);
    assert!(!operation.finish(
        FinalizationStep::NativeTransfer,
        FinalizationOutcome::Applied
    ));
}

#[test]
fn unresolved_collection_keeps_exact_request_without_replay() {
    let mut operation = SessionFinalization::new(FinalizationMode::Disconnect, true, None);
    while operation.next_step() != Some(FinalizationStep::Mounts) {
        let step = operation.next_step().unwrap();
        assert!(operation.begin(step));
        assert!(operation.finish(step, FinalizationOutcome::Applied));
    }
    let request = wow_persistence::AccountCollectionSaveLikeCpp::Mounts(vec![
        wow_persistence::AccountMountRowLikeCpp {
            bnet_account_id: 1,
            mount_spell_id: 42,
            flags: 1,
        },
    ]);
    assert!(operation.begin(FinalizationStep::Mounts));
    operation.retain_collection(request.clone());
    operation.interrupt();
    assert_eq!(operation.retained_collection, Some(request));
    assert_eq!(
        operation.report().outcome(FinalizationStep::Mounts),
        FinalizationOutcome::InFlight
    );
    assert!(!operation.begin(FinalizationStep::Mounts));
}
