use super::*;

#[test]
fn temporary_is_erased_before_real_add_and_disabled_new_spells_skip_all_effects() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                valid: true,
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                valid: true,
                ..SpellFacts::plain(2, 2)
            },
        ],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    book.add_temporary_spell(1);
    book.set_spell_favorite(1, true);
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::New);
    assert!(!book.spell(1).unwrap().favorite());
    assert_eq!(
        book.membership,
        [
            SpellBookMutation::Insert(1),
            SpellBookMutation::Erase(1),
            SpellBookMutation::Insert(1)
        ]
    );
    effects.events.clear();
    let mut disabled = AddPlayerSpell::learned(2, true, false);
    disabled.disabled = true;
    assert!(
        !book
            .add_spell(disabled, &fixture.sources(), &mut effects)
            .unwrap()
    );
    assert!(!book.has_spell(2));
    assert!(effects.events.is_empty());
}
