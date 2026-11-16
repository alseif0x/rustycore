mod mutations;
mod planning;
mod reconciliation;

use super::fixtures::{Effects, Fixture, SpellFacts};
use crate::player::spell_book::{
    AddPlayerSpell, PlayerSpellBook, PlayerSpellEntry, PlayerSpellState, SpellBookMutation,
    SpellSaveAction, SpellSaveBatch, SpellSaveFinishError, SpellSavePrepareError,
};

pub(super) fn plain_fixture(ids: &[u32]) -> Fixture {
    Fixture {
        spells: ids.iter().map(|&id| SpellFacts::plain(id, id)).collect(),
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    }
}

pub(super) fn ordered(
    history: &[SpellBookMutation],
    count: usize,
) -> Result<Vec<u32>, &'static str> {
    let mut keys = Vec::new();
    for mutation in history {
        match *mutation {
            SpellBookMutation::Insert(spell) => {
                if !keys.contains(&spell) {
                    keys.push(spell);
                }
            }
            SpellBookMutation::Erase(spell) => keys.retain(|&key| key != spell),
        }
    }
    if keys.len() == count {
        Ok(keys)
    } else {
        Err("inconsistent membership fixture")
    }
}

pub(super) fn seed(
    book: &mut PlayerSpellBook,
    spell: u32,
    state: PlayerSpellState,
    active: bool,
    dependent: bool,
    disabled: bool,
    favorite: bool,
) {
    let (entry, inserted) = book.try_emplace(spell);
    assert!(inserted);
    *entry = PlayerSpellEntry {
        state,
        active,
        dependent,
        disabled,
        favorite,
        trait_data: None,
    };
}

pub(super) fn prepare(book: &mut PlayerSpellBook) -> SpellSaveBatch {
    book.prepare_save(ordered).unwrap()
}

pub(super) fn quiet_effects() -> Effects<'static> {
    Effects::quiet()
}

pub(super) fn assert_actions(batch: &SpellSaveBatch, expected: &[SpellSaveAction]) {
    assert_eq!(batch.actions(), expected);
}
