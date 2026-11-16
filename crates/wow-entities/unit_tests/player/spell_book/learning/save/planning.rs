use super::*;
use crate::player::spell_book::{SpellBookOrderError, SpellSavePrepareError};

#[test]
fn five_states_emit_exact_native_ordered_actions_and_only_removed_nodes_erase() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        0,
        PlayerSpellState::Unchanged,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        1,
        PlayerSpellState::Changed,
        false,
        false,
        true,
        true,
    );
    seed(&mut book, 2, PlayerSpellState::New, true, true, false, true);
    seed(
        &mut book,
        u32::MAX,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        true,
    );
    seed(
        &mut book,
        7,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        true,
    );

    let batch = prepare(&mut book);

    assert_actions(
        &batch,
        &[
            SpellSaveAction::DeletePrimary { spell: 1 },
            SpellSaveAction::InsertPrimary {
                spell: 1,
                active: false,
                disabled: true,
            },
            SpellSaveAction::DeleteFavorite { spell: 1 },
            SpellSaveAction::InsertFavorite { spell: 1 },
            SpellSaveAction::DeleteFavorite { spell: 2 },
            SpellSaveAction::InsertFavorite { spell: 2 },
            SpellSaveAction::DeletePrimary { spell: u32::MAX },
        ],
    );
    assert_eq!(book.spell(0).unwrap().state(), PlayerSpellState::Unchanged);
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Changed);
    assert_eq!(book.spell(2).unwrap().state(), PlayerSpellState::New);
    assert_eq!(book.spell(7).unwrap().state(), PlayerSpellState::Temporary);
    assert!(book.spell(u32::MAX).is_none());
    assert_eq!(
        book.membership.last(),
        Some(&SpellBookMutation::Erase(u32::MAX))
    );
}

#[test]
fn provider_receives_clean_and_temporary_membership_and_controls_zero_full_width_order() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        u32::MAX,
        PlayerSpellState::Changed,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        0,
        PlayerSpellState::New,
        false,
        false,
        true,
        false,
    );
    seed(
        &mut book,
        5,
        PlayerSpellState::Unchanged,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        6,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        false,
    );
    let mut seen = None;

    let batch = book
        .prepare_save(|history, count| {
            seen = Some((history.to_vec(), count));
            let mut keys = ordered(history, count)?;
            keys.reverse();
            Ok::<_, &'static str>(keys)
        })
        .unwrap();

    assert_eq!(seen, Some((book.membership.clone(), 4)));
    assert_actions(
        &batch,
        &[
            SpellSaveAction::InsertPrimary {
                spell: 0,
                active: false,
                disabled: true,
            },
            SpellSaveAction::DeleteFavorite { spell: 0 },
            SpellSaveAction::DeletePrimary { spell: u32::MAX },
            SpellSaveAction::InsertPrimary {
                spell: u32::MAX,
                active: true,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: u32::MAX },
        ],
    );
}

#[test]
fn retry_obligations_precede_orphans_and_coalesce_with_live_states() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        2,
        PlayerSpellState::New,
        true,
        false,
        false,
        true,
    );
    seed(
        &mut book,
        1,
        PlayerSpellState::Changed,
        true,
        true,
        false,
        false,
    );
    seed(
        &mut book,
        3,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        true,
    );
    seed(
        &mut book,
        4,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        true,
    );
    seed(
        &mut book,
        5,
        PlayerSpellState::Unchanged,
        false,
        false,
        false,
        false,
    );
    book.primary_delete_obligations = vec![9, 2, 1, 3, 4, 5];

    let batch = prepare(&mut book);

    assert_actions(
        &batch,
        &[
            SpellSaveAction::DeletePrimary { spell: 9 },
            SpellSaveAction::DeletePrimary { spell: 2 },
            SpellSaveAction::InsertPrimary {
                spell: 2,
                active: true,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 2 },
            SpellSaveAction::InsertFavorite { spell: 2 },
            SpellSaveAction::DeletePrimary { spell: 1 },
            SpellSaveAction::DeleteFavorite { spell: 1 },
            SpellSaveAction::DeletePrimary { spell: 3 },
            SpellSaveAction::DeletePrimary { spell: 4 },
            SpellSaveAction::DeletePrimary { spell: 5 },
            SpellSaveAction::InsertPrimary {
                spell: 5,
                active: false,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 5 },
        ],
    );
    assert_eq!(book.spell(3), None);
    assert!(book.spell(4).is_some());
}

#[test]
fn removed_nodes_erase_in_the_supplied_nonnumeric_source_order() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        90,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        10,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );

    let batch = book
        .prepare_save(|history, count| {
            assert_eq!(ordered(history, count)?, [90, 10]);
            Ok::<_, &'static str>(vec![90, 10])
        })
        .unwrap();

    assert_actions(
        &batch,
        &[
            SpellSaveAction::DeletePrimary { spell: 90 },
            SpellSaveAction::DeletePrimary { spell: 10 },
        ],
    );
    assert_eq!(
        book.membership,
        [
            SpellBookMutation::Insert(90),
            SpellBookMutation::Insert(10),
            SpellBookMutation::Erase(90),
            SpellBookMutation::Erase(10),
        ]
    );
}

#[test]
fn empty_clean_source_errors_busy_and_exhaustion_preserve_the_owner() {
    let mut empty = PlayerSpellBook::default();
    let mut called = false;
    let batch = empty
        .prepare_save(|history, count| {
            called = true;
            assert!(history.is_empty());
            assert_eq!(count, 0);
            Ok::<_, &'static str>(Vec::new())
        })
        .unwrap();
    assert!(called);
    assert_actions(&batch, &[]);
    empty.confirm_save(batch).unwrap();

    let mut clean = PlayerSpellBook::default();
    seed(
        &mut clean,
        10,
        PlayerSpellState::Unchanged,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut clean,
        11,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        false,
    );
    let mut seen = None;
    let clean_batch = clean
        .prepare_save(|history, count| {
            seen = Some((history.to_vec(), count));
            ordered(history, count)
        })
        .unwrap();
    assert_eq!(seen, Some((clean.membership.clone(), 2)));
    assert_actions(&clean_batch, &[]);
    clean.rollback_save(clean_batch).unwrap();

    let mut invalid = PlayerSpellBook::default();
    seed(
        &mut invalid,
        3,
        PlayerSpellState::Changed,
        true,
        false,
        false,
        true,
    );
    seed(
        &mut invalid,
        4,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    invalid.primary_delete_obligations.push(77);
    let membership = invalid.membership.clone();
    let entry = *invalid.spell(3).unwrap();
    let removed = *invalid.spell(4).unwrap();
    let result = invalid.prepare_save(|_, _| Ok::<_, &'static str>(vec![]));
    assert!(matches!(
        result,
        Err(SpellSavePrepareError::Order(
            SpellBookOrderError::InvalidKeySet
        ))
    ));
    assert_eq!(invalid.membership, membership);
    assert_eq!(*invalid.spell(3).unwrap(), entry);
    assert_eq!(*invalid.spell(4).unwrap(), removed);
    assert_eq!(invalid.primary_delete_obligations, [77]);
    assert!(invalid.pending_save.is_none());
    assert_eq!(invalid.save_sequence, 0);

    let source_error = invalid.prepare_save(|_, _| Err::<Vec<u32>, _>("order unavailable"));
    assert!(matches!(
        source_error,
        Err(SpellSavePrepareError::Order(SpellBookOrderError::Source(
            "order unavailable"
        )))
    ));
    assert_eq!(invalid.membership, membership);
    assert_eq!(*invalid.spell(3).unwrap(), entry);
    assert_eq!(*invalid.spell(4).unwrap(), removed);
    assert_eq!(invalid.primary_delete_obligations, [77]);
    assert_eq!(invalid.save_sequence, 0);

    let mut pending = PlayerSpellBook::default();
    let first = prepare(&mut pending);
    let mut busy_provider_called = false;
    let busy = pending.prepare_save(|_, _| {
        busy_provider_called = true;
        Ok::<_, &'static str>(Vec::new())
    });
    assert!(matches!(busy, Err(SpellSavePrepareError::PendingOperation)));
    assert!(!busy_provider_called);
    drop(first);
    let still_busy = pending.prepare_save(|_, _| Ok::<_, &'static str>(Vec::new()));
    assert!(matches!(
        still_busy,
        Err(SpellSavePrepareError::PendingOperation)
    ));

    let mut exhausted = PlayerSpellBook::default();
    exhausted.save_sequence = u64::MAX;
    let mut exhaustion_provider_called = false;
    let result = exhausted.prepare_save(|_, _| {
        exhaustion_provider_called = true;
        Ok::<_, &'static str>(Vec::new())
    });
    assert!(matches!(
        result,
        Err(SpellSavePrepareError::SequenceExhausted)
    ));
    assert!(!exhaustion_provider_called);
    assert!(exhausted.pending_save.is_none());
    assert!(exhausted.membership.is_empty());
}
