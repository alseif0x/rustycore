use super::*;
use wow_persistence::forever::spells::SpellLearnRow;

#[test]
fn automatic_relationship_still_has_override_and_removal_cleans_it_even_when_child_absent() {
    let mut fixture = Fixture::new(
        vec![row(1), row(3)],
        Default::default(),
        vec![],
        vec![],
        vec![SpellLearnRow {
            source: 1,
            learned: 3,
            active: true,
        }],
    );
    crate::forever::spells::spell_book_test_learn_flags(&mut fixture.spells, 1, 3, 777, true);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert!(!book.has_spell(3));
    assert!(book.has_override_spell(777, 3));
    effects.events.clear();
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(!book.has_override_spell(777, 3));
    assert!(
        !effects
            .events
            .iter()
            .any(|event| matches!(event, Event::OwnedAura(3, _)))
    );
}
