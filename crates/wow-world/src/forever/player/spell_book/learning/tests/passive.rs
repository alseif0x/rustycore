use super::*;
#[test]
fn passive_stance_and_aura_state_short_circuits_preserve_source_precedence() {
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.1.stances = 2;
    spell.1.caster_aura_state = 7;
    let fixture = Fixture::simple(vec![spell]);
    for (form, aura, cast) in [
        (0, false, false),
        (1, true, false),
        (2, false, false),
        (2, true, true),
    ] {
        let mut book = PlayerSpellBook::default();
        let mut effects = Effects::quiet();
        effects.form = form;
        if aura {
            effects.aura_states.insert(7);
        }
        assert!(
            book.add_spell(
                AddPlayerSpell::learned(1, true, false),
                &fixture.sources(),
                &mut effects
            )
            .unwrap()
        );
        assert_eq!(effects.events.contains(&Event::Cast(1)), cast);
    }
}
#[test]
fn item_required_passive_aura_is_added_without_cast_even_when_stance_does_not_match() {
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.1.stances = 2;
    spell.1.equipped_item_class = 2;
    spell.3.push(spell_effect(6, 1, 0, 0.));
    let fixture = Fixture::simple(vec![spell]);
    for (present, fits, aura) in [
        (false, true, true),
        (false, false, false),
        (true, true, false),
    ] {
        let mut book = PlayerSpellBook::default();
        let mut effects = Effects::quiet();
        effects.item_fits = fits;
        if present {
            effects.auras.insert(1);
        }
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects,
        )
        .unwrap();
        assert_eq!(effects.events.contains(&Event::Aura(1)), aura);
        assert!(!effects.events.contains(&Event::Cast(1)));
        assert_eq!(effects.events.contains(&Event::Fits(1)), !present);
    }
}
#[test]
fn skill_step_early_return_only_happens_after_an_actual_cast() {
    let mut spell = row(1);
    spell.3.push(spell_effect(44, 0, 0, 0.));
    let fixture = Fixture::simple(vec![spell]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    assert!(
        !book
            .add_spell(
                AddPlayerSpell::learned(1, true, false),
                &fixture.sources(),
                &mut effects
            )
            .unwrap()
    );
    assert_eq!(effects.events, [Event::Cast(1)]);
    assert!(book.has_active_spell(1));
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.1.stances = 2;
    spell.3.push(spell_effect(44, 0, 0, 0.));
    let fixture = Fixture::simple(vec![spell]);
    let mut book = PlayerSpellBook::default();
    effects.events.clear();
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    assert_eq!(effects.events, [Event::MountQuery(1)]);
}
#[test]
fn talent_learn_cast_overrides_passive_stance_unless_loading() {
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.1.stances = 2;
    spell.2 = 0x00800000;
    let mut e = spell_effect(36, 0, 0, 0.);
    e.trigger_spell = 2;
    spell.3.push(e);
    let fixture = Fixture::simple(vec![spell, row(2)]);
    for loading in [false, true] {
        let mut book = PlayerSpellBook::default();
        let mut effects = Effects::quiet();
        let mut request = AddPlayerSpell::learned(1, true, false);
        request.loading = loading;
        book.add_spell(request, &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(effects.events.contains(&Event::Cast(1)), !loading);
    }
}
#[test]
fn undefined_mask_shift_and_failed_aura_effect_are_errors_with_admitted_prefix() {
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.1.stances = 1;
    let fixture = Fixture::simple(vec![spell]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.form = 65;
    assert_eq!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        ),
        Err(SpellLearningError::Source(
            SpellLearningSourceError::UndefinedShapeshiftMask
        ))
    );
    assert!(book.has_spell(1));
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.1.equipped_item_class = 2;
    spell.3.push(spell_effect(6, 1, 0, 0.));
    let fixture = Fixture::simple(vec![spell]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.fail = Some(Event::Aura(1));
    assert_eq!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        ),
        Err(SpellLearningError::Effect("required effect failed"))
    );
    assert!(book.has_spell(1));
    assert_eq!(effects.events, [Event::Fits(1), Event::Aura(1)]);
}
