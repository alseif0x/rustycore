use super::*;
#[test]
fn missing_and_invalid_definitions_do_not_mutate_and_invalid_load_fences_global_cleanup() {
    let mut craft = row(1);
    let mut e = spell_effect(24, 0, 0, 0.);
    e.item_type = 99;
    craft.3.push(e);
    let fixture = Fixture::simple(vec![craft]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    for id in [1, 99] {
        assert!(
            !book
                .add_spell(
                    AddPlayerSpell::learned(id, true, false),
                    &fixture.sources(),
                    &mut effects
                )
                .unwrap()
        );
        let mut load = AddPlayerSpell::learned(id, true, false);
        load.learning = false;
        assert_eq!(
            book.add_spell(load, &fixture.sources(), &mut effects),
            Err(SpellLearningError::Source(
                SpellLearningSourceError::UnauthorizedGlobalCleanup
            ))
        );
    }
    assert!(book.is_empty() && effects.events.is_empty());
}
#[test]
fn temporary_is_erased_before_real_add_and_disabled_new_spells_skip_all_effects() {
    let fixture = Fixture::simple(vec![row(1), row(2)]);
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
#[test]
fn cast_failure_keeps_admitted_prefix_without_running_later_effects() {
    let mut spell = row(1);
    spell.1.attributes[1] = 0x80000000;
    let fixture = Fixture::simple(vec![spell]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.fail = Some(Event::Cast(1));
    assert_eq!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        ),
        Err(SpellLearningError::Effect("required effect failed"))
    );
    assert!(book.has_active_spell(1));
    assert_eq!(effects.events, [Event::Cast(1)]);
}
#[test]
fn unavailable_trait_lookup_is_not_fabricated_missing_data() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.trait_unavailable = true;
    let mut request = AddPlayerSpell::learned(1, true, false);
    request.trait_data = Some(PlayerSpellTrait::new(7, 1));
    assert_eq!(
        book.add_spell(request, &fixture.sources(), &mut effects),
        Err(SpellLearningError::Effect("trait catalog unavailable"))
    );
    assert_eq!(book.spell(1).unwrap().trait_data(), request.trait_data);
    assert_eq!(effects.events, [Event::Trait(7)]);
}
#[test]
fn source_constructor_rejects_a_different_birth_authority_and_missing_phases() {
    let fixture = Fixture::simple(vec![row(1)]);
    let other = Fixture::simple(vec![row(1)]);
    assert!(matches!(
        SpellLearningSources::new(&fixture.world, &other.spells, &fixture.items, 1, 1),
        Err(SpellLearningSourceError::MismatchedBirthCatalog)
    ));
    let birth = std::sync::Arc::new(
        wow_data::forever_birth::BirthRecords::default()
            .finish(
                Default::default(),
                Default::default(),
                &wow_data::Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap(),
    );
    let seeds = crate::forever::spells::skill_set_test_definitions(birth);
    assert!(matches!(
        SpellLearningSources::new(&fixture.world, &seeds, &fixture.items, 1, 1),
        Err(SpellLearningSourceError::MissingLearningPhases)
    ));
}
