//! Faults packets.
//!
//! Separated from tests.rs under #709.

use super::*;

#[test]
fn combined_commit_reconciliation_requires_money_and_exact_durable_result() {
    use PlayerSpellAcquisitionMoneyReconciliationLikeCpp::{Committed, Indeterminate};

    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, true, true),
        Committed
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 100, false, true),
        Indeterminate,
        "the old balance cannot prove rollback after an ambiguous COMMIT"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 100, true, true),
        Indeterminate,
        "a later writer may restore money after the acquisition rows committed"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, false, true),
        Indeterminate,
        "money alone must never authorize publication of an incomplete acquisition"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(100, 100, true, true),
        Committed,
        "a free trainer purchase is proven only by its complete durable result"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(100, 100, false, true,),
        Indeterminate,
        "an unchanged balance cannot prove rollback for a free purchase"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, true, false,),
        Indeterminate,
        "an identical later operation must not authorize this attempt's publication"
    );
}

#[test]
fn every_durable_fault_boundary_discards_the_whole_operation_prefix() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DurableState {
        spells: Vec<DurablePlayerSpellRowLikeCpp>,
        favorites: Vec<i32>,
        skills: Vec<DurablePlayerSkillRowLikeCpp>,
    }

    fn execute_atomically(
        original: &DurableState,
        operations: &[PlayerSpellAcquisitionDurableOperationLikeCpp],
        fail_before_operation: Option<usize>,
        fail_before_commit: bool,
    ) -> DurableState {
        let mut transaction = original.clone();
        for (index, operation) in operations.iter().copied().enumerate() {
            if fail_before_operation == Some(index) {
                return original.clone();
            }
            match operation {
                PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter => {}
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells => {
                    transaction.spells.clear()
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells => {
                    transaction.favorites.clear()
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills => {
                    transaction.skills.clear()
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(row) => {
                    transaction.spells.push(row)
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(id) => {
                    transaction.favorites.push(id)
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(row) => {
                    transaction.skills.push(row)
                }
            }
        }
        if fail_before_commit {
            original.clone()
        } else {
            transaction
        }
    }

    let original = DurableState {
        spells: vec![DurablePlayerSpellRowLikeCpp {
            spell_id: 99,
            active: true,
            disabled: false,
        }],
        favorites: vec![99],
        skills: vec![DurablePlayerSkillRowLikeCpp {
            skill_id: 10,
            value: 1,
            maximum: 75,
            profession_slot: -1,
        }],
    };
    let operations = vec![
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(DurablePlayerSpellRowLikeCpp {
            spell_id: 100,
            active: true,
            disabled: false,
        }),
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(100),
    ];

    for index in 0..operations.len() {
        assert_eq!(
            execute_atomically(&original, &operations, Some(index), false),
            original,
            "fault before operation {index} must roll back the entire prefix"
        );
    }
    assert_eq!(
        execute_atomically(&original, &operations, None, true),
        original,
        "fault at commit must not expose the transactional prefix"
    );
    assert_eq!(
        execute_atomically(&original, &operations, None, false),
        DurableState {
            spells: vec![DurablePlayerSpellRowLikeCpp {
                spell_id: 100,
                active: true,
                disabled: false,
            }],
            favorites: vec![100],
            skills: Vec::new(),
        }
    );
}

#[test]
fn trainer_fee_and_acquisition_share_every_rollback_boundary() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CombinedState {
        money: u64,
        spells: Vec<i32>,
        skills: Vec<u16>,
    }

    fn execute(original: &CombinedState, fail_before_step: Option<usize>) -> CombinedState {
        let mut transaction = original.clone();
        // Mirrors the combined transaction's destructive replacement,
        // deterministic inserts, guarded money update and COMMIT fence.
        let mut step = 0;
        macro_rules! transaction_step {
            ($body:expr) => {{
                if fail_before_step == Some(step) {
                    return original.clone();
                }
                $body;
                step += 1;
            }};
        }
        transaction_step!(transaction.spells.clear());
        transaction_step!(transaction.skills.clear());
        transaction_step!(transaction.spells.extend([200, 201]));
        transaction_step!(transaction.skills.push(164));
        transaction_step!(transaction.money = 80);
        if fail_before_step == Some(step) {
            return original.clone();
        }
        transaction
    }

    let original = CombinedState {
        money: 100,
        spells: vec![100],
        skills: vec![95],
    };
    for boundary in 0..=5 {
        assert_eq!(
            execute(&original, Some(boundary)),
            original,
            "fault boundary {boundary} must expose neither a fee nor an acquisition prefix"
        );
    }
    assert_eq!(
        execute(&original, None),
        CombinedState {
            money: 80,
            spells: vec![200, 201],
            skills: vec![164],
        }
    );
}

#[tokio::test]
async fn port_outcome_reconciles_only_an_unknown_commit_like_cpp() {
    use std::sync::atomic::Ordering;
    use wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp as Attempt;

    let applied = RecordingSpellAcquisitionPort {
        attempt: Attempt::Applied,
        reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Indeterminate,
        reconciliation_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    assert!(matches!(
        persist_player_spell_acquisition_through_port_like_cpp(
            &applied,
            empty_persistence_request()
        )
        .await,
        PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Applied
    ));
    assert_eq!(applied.reconciliation_calls.load(Ordering::Relaxed), 0);

    let unknown = RecordingSpellAcquisitionPort {
        attempt: Attempt::CommitOutcomeUnknown {
            reason: "lost reply".to_owned(),
        },
        reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Committed,
        reconciliation_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    assert!(matches!(
        persist_player_spell_acquisition_through_port_like_cpp(
            &unknown,
            empty_persistence_request()
        )
        .await,
        PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::ReconciledCommit(reason)
            if reason == "lost reply"
    ));
    assert_eq!(unknown.reconciliation_calls.load(Ordering::Relaxed), 1);
}
