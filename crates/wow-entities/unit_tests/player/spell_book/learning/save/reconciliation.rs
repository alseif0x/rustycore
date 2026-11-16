use super::*;
use crate::player::spell_book::PlayerSpellTrait;

#[test]
fn exact_dirty_and_repair_rows_confirm_clean_but_removed_and_temporary_facts_survive() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        1,
        PlayerSpellState::New,
        true,
        false,
        false,
        true,
    );
    seed(
        &mut book,
        2,
        PlayerSpellState::Changed,
        false,
        true,
        true,
        false,
    );
    seed(
        &mut book,
        3,
        PlayerSpellState::Unchanged,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        4,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        5,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        true,
    );
    book.primary_delete_obligations.push(3);
    book.primary_delete_obligations.push(5);
    book.primary_delete_obligations.push(99);
    let membership_before = book.membership.clone();

    let batch = prepare(&mut book);
    book.confirm_save(batch).unwrap();

    for spell in [1, 2, 3] {
        assert_eq!(
            book.spell(spell).unwrap().state(),
            PlayerSpellState::Unchanged
        );
    }
    assert!(book.spell(4).is_none());
    assert_eq!(book.spell(5).unwrap().state(), PlayerSpellState::Temporary);
    assert!(book.spell(99).is_none());
    assert!(book.primary_delete_obligations.is_empty());
    let mut expected_membership = membership_before;
    expected_membership.push(SpellBookMutation::Erase(4));
    assert_eq!(book.membership, expected_membership);
}

#[test]
fn current_edits_remain_changed_and_trait_only_differences_do_not_block_confirmation() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        10,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        11,
        PlayerSpellState::Changed,
        true,
        false,
        false,
        true,
    );
    seed(
        &mut book,
        12,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    let batch = prepare(&mut book);

    book.entries.get_mut(&10).unwrap().active = false;
    book.entries.get_mut(&11).unwrap().favorite = false;
    book.entries.get_mut(&12).unwrap().trait_data = Some(PlayerSpellTrait::new(0x123456, 257));
    book.confirm_save(batch).unwrap();

    assert_eq!(book.spell(10).unwrap().state(), PlayerSpellState::Changed);
    assert!(!book.spell(10).unwrap().active());
    assert_eq!(book.spell(11).unwrap().state(), PlayerSpellState::Changed);
    assert!(!book.spell(11).unwrap().favorite());
    assert_eq!(book.spell(12).unwrap().state(), PlayerSpellState::Unchanged);
    assert_eq!(
        book.spell(12).unwrap().trait_data(),
        Some(PlayerSpellTrait::new(0x123456, 257))
    );
}

#[test]
fn delete_only_confirmation_repairs_new_permanent_entries_and_preserves_temporary_or_absence() {
    let fixture = plain_fixture(&[20]);
    let sources = fixture.sources();
    let mut book = PlayerSpellBook::default();
    book.primary_delete_obligations = vec![20, 21, 22];
    seed(
        &mut book,
        21,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        22,
        PlayerSpellState::Temporary,
        true,
        false,
        false,
        false,
    );
    let batch = prepare(&mut book);

    let mut effects = quiet_effects();
    let mut replacement = AddPlayerSpell::learned(20, true, false);
    replacement.learning = false;
    book.add_spell(replacement, &sources, &mut effects).unwrap();
    book.remove_temporary_spell(22);
    book.confirm_save(batch).unwrap();

    assert_eq!(book.spell(20).unwrap().state(), PlayerSpellState::New);
    assert_eq!(book.spell(21).unwrap().state(), PlayerSpellState::Temporary);
    assert!(book.spell(22).is_none());
    assert!(book.primary_delete_obligations.is_empty());
}

#[test]
fn old_obligation_is_consumed_before_new_missing_primary_intent_is_added() {
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        30,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    book.primary_delete_obligations.push(30);
    let batch = prepare(&mut book);
    assert_actions(
        &batch,
        &[
            SpellSaveAction::DeletePrimary { spell: 30 },
            SpellSaveAction::InsertPrimary {
                spell: 30,
                active: true,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 30 },
        ],
    );

    // Defensive missing-node fault injection after a primary INSERT was
    // planned. The numeric fence must survive consuming old debt.
    book.erase(30);
    book.confirm_save(batch).unwrap();
    assert_eq!(book.primary_delete_obligations, [30]);

    let mut temporary = PlayerSpellBook::default();
    seed(
        &mut temporary,
        31,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    temporary.primary_delete_obligations.push(31);
    let batch = prepare(&mut temporary);
    // This temporary replacement is also fault injection: normal AddTemporary
    // cannot replace an occupied New node while an operation is pending.
    temporary.entries.get_mut(&31).unwrap().state = PlayerSpellState::Temporary;
    temporary.confirm_save(batch).unwrap();
    assert_eq!(
        temporary.spell(31).unwrap().state(),
        PlayerSpellState::Temporary
    );
    assert_eq!(temporary.primary_delete_obligations, [31]);
}

#[test]
fn rollback_removed_retries_temporary_cleanup_and_relearn_delete_before_insert() {
    let mut temporary = PlayerSpellBook::default();
    seed(
        &mut temporary,
        50,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    let removed_batch = prepare(&mut temporary);
    assert!(temporary.spell(50).is_none());
    temporary.rollback_save(removed_batch).unwrap();
    assert_eq!(temporary.primary_delete_obligations, [50]);

    temporary.add_temporary_spell(50);
    assert_eq!(
        temporary.spell(50).unwrap().state(),
        PlayerSpellState::Temporary
    );
    let retry = prepare(&mut temporary);
    assert_actions(&retry, &[SpellSaveAction::DeletePrimary { spell: 50 }]);
    temporary.confirm_save(retry).unwrap();
    assert!(temporary.primary_delete_obligations.is_empty());
    temporary.remove_temporary_spell(50);
    assert!(temporary.spell(50).is_none());

    let fixture = plain_fixture(&[51, 52]);
    let sources = fixture.sources();
    let mut relearned = PlayerSpellBook::default();
    seed(
        &mut relearned,
        51,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    let removed_batch = prepare(&mut relearned);
    relearned.rollback_save(removed_batch).unwrap();
    assert_eq!(relearned.primary_delete_obligations, [51]);

    let mut effects = quiet_effects();
    relearned
        .add_spell(
            AddPlayerSpell::learned(51, true, false),
            &sources,
            &mut effects,
        )
        .unwrap();
    assert_eq!(relearned.spell(51).unwrap().state(), PlayerSpellState::New);
    let retry = prepare(&mut relearned);
    assert_actions(
        &retry,
        &[
            SpellSaveAction::DeletePrimary { spell: 51 },
            SpellSaveAction::InsertPrimary {
                spell: 51,
                active: true,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 51 },
        ],
    );
    relearned.confirm_save(retry).unwrap();
    assert_eq!(
        relearned.spell(51).unwrap().state(),
        PlayerSpellState::Unchanged
    );
    assert!(relearned.primary_delete_obligations.is_empty());

    let mut unchanged = PlayerSpellBook::default();
    seed(
        &mut unchanged,
        52,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    let removed_batch = prepare(&mut unchanged);
    unchanged.rollback_save(removed_batch).unwrap();
    let mut nonlearning = AddPlayerSpell::learned(52, true, false);
    nonlearning.learning = false;
    unchanged
        .add_spell(nonlearning, &sources, &mut effects)
        .unwrap();
    assert_eq!(
        unchanged.spell(52).unwrap().state(),
        PlayerSpellState::Unchanged
    );
    let repair = prepare(&mut unchanged);
    assert_actions(
        &repair,
        &[
            SpellSaveAction::DeletePrimary { spell: 52 },
            SpellSaveAction::InsertPrimary {
                spell: 52,
                active: true,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 52 },
        ],
    );
    unchanged.confirm_save(repair).unwrap();
    assert_eq!(
        unchanged.spell(52).unwrap().state(),
        PlayerSpellState::Unchanged
    );
    assert!(unchanged.primary_delete_obligations.is_empty());
}

#[test]
fn rollback_preserves_live_new_and_changed_mutations_for_the_next_save() {
    let fixture = plain_fixture(&[60, 61]);
    let sources = fixture.sources();
    let mut book = PlayerSpellBook::default();
    seed(
        &mut book,
        60,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    seed(
        &mut book,
        61,
        PlayerSpellState::Changed,
        true,
        false,
        false,
        false,
    );
    let membership = book.membership.clone();
    let first = prepare(&mut book);

    book.set_spell_favorite(60, true);
    let mut effects = quiet_effects();
    book.add_spell(
        AddPlayerSpell::learned(61, false, false),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(book.spell(60).unwrap().state(), PlayerSpellState::Changed);
    assert!(book.spell(60).unwrap().favorite());
    assert_eq!(book.spell(61).unwrap().state(), PlayerSpellState::Changed);
    assert!(!book.spell(61).unwrap().active());
    book.rollback_save(first).unwrap();

    assert_eq!(book.spell(60).unwrap().state(), PlayerSpellState::Changed);
    assert!(book.spell(60).unwrap().favorite());
    assert_eq!(book.spell(61).unwrap().state(), PlayerSpellState::Changed);
    assert!(!book.spell(61).unwrap().active());
    assert_eq!(book.membership, membership);
    assert!(book.primary_delete_obligations.is_empty());

    let retry = prepare(&mut book);
    assert_actions(
        &retry,
        &[
            SpellSaveAction::DeletePrimary { spell: 60 },
            SpellSaveAction::InsertPrimary {
                spell: 60,
                active: true,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 60 },
            SpellSaveAction::InsertFavorite { spell: 60 },
            SpellSaveAction::DeletePrimary { spell: 61 },
            SpellSaveAction::InsertPrimary {
                spell: 61,
                active: false,
                disabled: false,
            },
            SpellSaveAction::DeleteFavorite { spell: 61 },
        ],
    );
    book.rollback_save(retry).unwrap();
}

#[test]
fn no_pending_wrong_operation_and_rollback_preserve_their_fences() {
    let mut original = PlayerSpellBook::default();
    seed(
        &mut original,
        40,
        PlayerSpellState::Removed,
        false,
        false,
        false,
        false,
    );
    let first = prepare(&mut original);
    original.rollback_save(first).unwrap();
    assert!(original.spell(40).is_none());
    assert_eq!(original.primary_delete_obligations, [40]);
    let original_batch = prepare(&mut original); // This is operation 1.

    let mut stale_source = PlayerSpellBook::default();
    let stale = prepare(&mut stale_source); // Independently begins at operation 0.
    assert_eq!(
        original.confirm_save(stale),
        Err(SpellSaveFinishError::WrongOperation)
    );
    assert!(original.pending_save.is_some());
    original.rollback_save(original_batch).unwrap();
    assert!(original.spell(40).is_none());
    assert_eq!(original.primary_delete_obligations, [40]);

    let mut pending_source = PlayerSpellBook::default();
    let dropped = prepare(&mut pending_source);
    let mut no_pending_book = PlayerSpellBook::default();
    assert_eq!(
        no_pending_book.confirm_save(dropped),
        Err(SpellSaveFinishError::NoPending)
    );
    assert!(matches!(
        pending_source.prepare_save(ordered),
        Err(SpellSavePrepareError::PendingOperation)
    ));
}
