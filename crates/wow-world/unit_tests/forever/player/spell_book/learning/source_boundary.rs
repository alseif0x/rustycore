use super::*;

#[test]
fn item_fit_receives_the_real_borrowed_definition_from_the_admitted_owner() {
    let mut passive = row(1);
    passive.1.attributes[0] = 0x40;
    passive.1.equipped_item_class = 2;
    passive.3.push(spell_effect(6, 77, 0, 0.));
    let fixture = Fixture::simple(vec![passive]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    // The callback checks real World-only view APIs (difficulty and exact owner),
    // beyond the lower contract. An ID or scalar substitute cannot satisfy it.
    effects.item_fit_view = Some((&fixture.spells, 1, 0));
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    assert_eq!(
        effects.events,
        [Event::Fits(1), Event::Aura(1), Event::MountQuery(1)]
    );
    assert!(book.has_active_spell(1));
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::New);
    assert_eq!(book.len(), 1);
}
