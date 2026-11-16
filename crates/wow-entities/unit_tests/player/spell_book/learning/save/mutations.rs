use super::*;
use crate::SpellLearningRank;
use crate::player::spell_book::{AddPlayerSpell, RemovePlayerSpell};

#[test]
fn real_remove_of_captured_new_retains_removed_node_and_rejects_temporary_replacement() {
    let fixture = plain_fixture(&[1]);
    let sources = fixture.sources();
    let mut book = PlayerSpellBook::default();
    let mut effects = quiet_effects();
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::New);
    let batch = prepare(&mut book);
    let membership = book.membership.clone();

    book.remove_spell(RemovePlayerSpell::new(1), &sources, &mut effects)
        .unwrap();
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Removed);
    book.add_temporary_spell(1);
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Removed);
    assert_eq!(book.membership, membership);

    book.confirm_save(batch).unwrap();
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Removed);
    assert_eq!(book.membership, membership);
}

#[test]
fn ordinary_new_removal_without_pending_still_erases_and_records_membership() {
    let fixture = plain_fixture(&[2]);
    let sources = fixture.sources();
    let mut book = PlayerSpellBook::default();
    let mut effects = quiet_effects();
    book.add_spell(
        AddPlayerSpell::learned(2, true, false),
        &sources,
        &mut effects,
    )
    .unwrap();
    book.remove_spell(RemovePlayerSpell::new(2), &sources, &mut effects)
        .unwrap();
    assert!(book.spell(2).is_none());
    assert_eq!(
        book.membership,
        [SpellBookMutation::Insert(2), SpellBookMutation::Erase(2)]
    );
}

#[test]
fn captured_new_favorite_active_dependent_and_disabled_writes_remain_dirty() {
    let fixture = plain_fixture(&[10, 11, 12, 13, 14]);
    let sources = fixture.sources();

    let mut favorite_book = PlayerSpellBook::default();
    let mut favorite_effects = quiet_effects();
    favorite_book
        .add_spell(
            AddPlayerSpell::learned(10, true, false),
            &sources,
            &mut favorite_effects,
        )
        .unwrap();
    let favorite_batch = prepare(&mut favorite_book);
    favorite_book.set_spell_favorite(10, false); // Equal assignment is still dirty.
    assert_eq!(
        favorite_book.spell(10).unwrap().state(),
        PlayerSpellState::Changed
    );
    favorite_book.confirm_save(favorite_batch).unwrap();
    assert_eq!(
        favorite_book.spell(10).unwrap().state(),
        PlayerSpellState::Changed
    );

    let mut active_book = PlayerSpellBook::default();
    let mut active_effects = quiet_effects();
    active_book
        .add_spell(
            AddPlayerSpell::learned(11, true, false),
            &sources,
            &mut active_effects,
        )
        .unwrap();
    let active_batch = prepare(&mut active_book);
    active_book
        .add_spell(
            AddPlayerSpell::learned(11, false, false),
            &sources,
            &mut active_effects,
        )
        .unwrap();
    assert_eq!(
        active_book.spell(11).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert!(!active_book.spell(11).unwrap().active());
    assert!(!active_book.spell(11).unwrap().dependent());
    active_book.confirm_save(active_batch).unwrap();
    assert_eq!(
        active_book.spell(11).unwrap().state(),
        PlayerSpellState::Changed
    );

    let mut dependent_book = PlayerSpellBook::default();
    let mut dependent_effects = quiet_effects();
    dependent_book
        .add_spell(
            AddPlayerSpell::learned(12, true, false),
            &sources,
            &mut dependent_effects,
        )
        .unwrap();
    let dependent_batch = prepare(&mut dependent_book);
    dependent_book
        .add_spell(
            AddPlayerSpell::learned(12, true, true),
            &sources,
            &mut dependent_effects,
        )
        .unwrap();
    assert_eq!(
        dependent_book.spell(12).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert!(dependent_book.spell(12).unwrap().active());
    assert!(dependent_book.spell(12).unwrap().dependent());
    dependent_book.confirm_save(dependent_batch).unwrap();
    assert_eq!(
        dependent_book.spell(12).unwrap().state(),
        PlayerSpellState::Changed
    );

    let mut disabled_book = PlayerSpellBook::default();
    let mut disabled_effects = quiet_effects();
    disabled_book
        .add_spell(
            AddPlayerSpell::learned(13, true, false),
            &sources,
            &mut disabled_effects,
        )
        .unwrap();
    let disabled_batch = prepare(&mut disabled_book);
    let mut disable = AddPlayerSpell::learned(13, true, false);
    disable.disabled = true;
    disabled_book
        .add_spell(disable, &sources, &mut disabled_effects)
        .unwrap();
    assert_eq!(
        disabled_book.spell(13).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert!(disabled_book.spell(13).unwrap().disabled());
    disabled_book.confirm_save(disabled_batch).unwrap();
    assert_eq!(
        disabled_book.spell(13).unwrap().state(),
        PlayerSpellState::Changed
    );

    let mut remove_disabled_book = PlayerSpellBook::default();
    let mut remove_disabled_effects = quiet_effects();
    remove_disabled_book
        .add_spell(
            AddPlayerSpell::learned(14, true, false),
            &sources,
            &mut remove_disabled_effects,
        )
        .unwrap();
    let remove_disabled_batch = prepare(&mut remove_disabled_book);
    let mut disable_request = RemovePlayerSpell::new(14);
    disable_request.disabled = true;
    remove_disabled_book
        .remove_spell(disable_request, &sources, &mut remove_disabled_effects)
        .unwrap();
    assert_eq!(
        remove_disabled_book.spell(14).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert!(remove_disabled_book.spell(14).unwrap().disabled());
    remove_disabled_book
        .confirm_save(remove_disabled_batch)
        .unwrap();
    assert_eq!(
        remove_disabled_book.spell(14).unwrap().state(),
        PlayerSpellState::Changed
    );
}

#[test]
fn captured_new_rank_replacement_and_nonlearning_lower_rank_reactivation_keep_intent() {
    let fixture = ranked_fixture();
    let sources = fixture.sources();
    let mut effects = quiet_effects();

    let mut replacement_book = PlayerSpellBook::default();
    seed(
        &mut replacement_book,
        1,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    let batch = prepare(&mut replacement_book);
    replacement_book
        .add_spell(
            AddPlayerSpell::learned(2, true, false),
            &sources,
            &mut effects,
        )
        .unwrap();
    assert!(!replacement_book.spell(1).unwrap().active());
    assert_eq!(
        replacement_book.spell(1).unwrap().state(),
        PlayerSpellState::Changed
    );
    replacement_book.confirm_save(batch).unwrap();
    assert_eq!(
        replacement_book.spell(1).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert_eq!(
        replacement_book.spell(2).unwrap().state(),
        PlayerSpellState::New
    );

    let mut lower_book = PlayerSpellBook::default();
    seed(
        &mut lower_book,
        1,
        PlayerSpellState::New,
        false,
        false,
        false,
        false,
    );
    seed(
        &mut lower_book,
        2,
        PlayerSpellState::New,
        true,
        false,
        false,
        false,
    );
    let batch = prepare(&mut lower_book);
    lower_book
        .remove_spell(RemovePlayerSpell::new(2), &sources, &mut effects)
        .unwrap();

    assert!(lower_book.spell(1).unwrap().active());
    assert_eq!(
        lower_book.spell(1).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert_eq!(
        lower_book.spell(2).unwrap().state(),
        PlayerSpellState::Removed
    );
    lower_book.confirm_save(batch).unwrap();
    assert_eq!(
        lower_book.spell(1).unwrap().state(),
        PlayerSpellState::Changed
    );
    assert_eq!(
        lower_book.spell(2).unwrap().state(),
        PlayerSpellState::Removed
    );
}

#[test]
fn nonlearning_updates_cannot_clear_captured_changed_or_repair_intent() {
    let fixture = plain_fixture(&[20, 21]);
    let sources = fixture.sources();
    let mut effects = quiet_effects();

    let mut changed = PlayerSpellBook::default();
    seed(
        &mut changed,
        20,
        PlayerSpellState::Changed,
        true,
        false,
        false,
        false,
    );
    let batch = prepare(&mut changed);
    let mut same = AddPlayerSpell::learned(20, true, false);
    same.learning = false;
    changed.add_spell(same, &sources, &mut effects).unwrap();
    assert_eq!(
        changed.spell(20).unwrap().state(),
        PlayerSpellState::Changed
    );
    changed.confirm_save(batch).unwrap();
    assert_eq!(
        changed.spell(20).unwrap().state(),
        PlayerSpellState::Unchanged
    );

    let mut repair = PlayerSpellBook::default();
    seed(
        &mut repair,
        21,
        PlayerSpellState::Unchanged,
        true,
        false,
        false,
        false,
    );
    repair.primary_delete_obligations.push(21);
    let batch = prepare(&mut repair);
    let mut lower_rank = AddPlayerSpell::learned(21, false, false);
    lower_rank.learning = false;
    repair
        .add_spell(lower_rank, &sources, &mut effects)
        .unwrap();
    assert_eq!(repair.spell(21).unwrap().state(), PlayerSpellState::Changed);
    repair.confirm_save(batch).unwrap();
    assert_eq!(repair.spell(21).unwrap().state(), PlayerSpellState::Changed);
}

fn ranked_fixture() -> Fixture {
    let mut lower = SpellFacts::plain(1, 1);
    lower.rank = Some(SpellLearningRank {
        first: (77, 0),
        rank: 1,
    });
    lower.next = 2;
    let mut higher = SpellFacts::plain(2, 1);
    higher.rank = Some(SpellLearningRank {
        first: (77, 0),
        rank: 2,
    });
    higher.previous = 1;
    Fixture {
        spells: vec![lower, higher],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    }
}
