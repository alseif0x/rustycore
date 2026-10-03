use super::*;
#[test]
fn matching_existing_flags_return_before_favorite_and_trait_and_load_marks_unchanged() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Changed, true, false);
    let mut request = AddPlayerSpell::learned(1, true, false);
    request.favorite = true;
    request.trait_data = Some(PlayerSpellTrait::new(4, 1));
    request.learning = false;
    let mut effects = Effects::quiet();
    assert!(
        !book
            .add_spell(request, &fixture.sources(), &mut effects)
            .unwrap()
    );
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Unchanged);
    assert!(!book.spell(1).unwrap().favorite());
    assert_eq!(book.spell(1).unwrap().trait_data(), None);
    assert!(effects.events.is_empty());
}
#[test]
fn dependent_promotion_dirties_load_and_never_demotes_on_inverse_request() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
    let mut effects = Effects::quiet();
    let mut request = AddPlayerSpell::learned(1, true, true);
    request.learning = false;
    request.favorite = true;
    assert!(
        !book
            .add_spell(request, &fixture.sources(), &mut effects)
            .unwrap()
    );
    assert!(book.spell(1).unwrap().dependent() && book.spell(1).unwrap().favorite());
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Changed);
    request.dependent = false;
    assert!(
        !book
            .add_spell(request, &fixture.sources(), &mut effects)
            .unwrap()
    );
    assert!(book.spell(1).unwrap().dependent());
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Unchanged);
}
#[test]
fn active_transition_returns_before_requested_disable_and_republishes_after_state_change() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
    let mut effects = Effects::quiet();
    effects.world = true;
    let mut request = AddPlayerSpell::learned(1, false, false);
    request.disabled = true;
    assert!(
        !book
            .add_spell(request, &fixture.sources(), &mut effects)
            .unwrap()
    );
    assert!(!book.spell(1).unwrap().active());
    assert!(!book.spell(1).unwrap().disabled());
    assert_eq!(
        effects.events,
        [Event::Message(
            SpellBookMessage::Unlearned {
                spell: 1,
                suppress_messaging: false
            },
            false,
            false
        )]
    );
}
#[test]
fn trait_replacement_removes_old_override_including_zero_but_does_not_add_new_on_early_return() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
    book.entries.get_mut(&1).unwrap().trait_data = Some(PlayerSpellTrait::new(4, 1));
    book.add_override_spell(0, 1);
    let mut effects = Effects::quiet();
    effects.traits.insert(4, 0);
    effects.traits.insert(5, 10);
    let mut request = AddPlayerSpell::learned(1, false, false);
    request.trait_data = Some(PlayerSpellTrait::new(5, 1));
    assert!(
        !book
            .add_spell(request, &fixture.sources(), &mut effects)
            .unwrap()
    );
    assert!(!book.has_override_spell(0, 1));
    assert!(!book.has_override_spell(10, 1));
    assert_eq!(effects.events, [Event::Trait(4)]);
}
#[test]
fn removed_relearning_erases_reinserts_changed_and_disabled_enable_keeps_its_node() {
    let fixture = Fixture::simple(vec![row(1), row(2)]);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Removed, true, false);
    insert(&mut book, 2, PlayerSpellState::Unchanged, false, true);
    let mut effects = Effects::quiet();
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Changed);
    assert!(
        !book
            .add_spell(
                AddPlayerSpell::learned(2, false, false),
                &fixture.sources(),
                &mut effects
            )
            .unwrap()
    );
    assert!(book.has_spell(2) && !book.has_active_spell(2));
    assert_eq!(book.spell(2).unwrap().state(), PlayerSpellState::Changed);
    assert_eq!(
        book.membership,
        [
            SpellBookMutation::Insert(1),
            SpellBookMutation::Insert(2),
            SpellBookMutation::Erase(1),
            SpellBookMutation::Insert(1)
        ]
    );
}
