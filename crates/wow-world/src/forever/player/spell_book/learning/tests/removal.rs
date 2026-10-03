use super::*;
use wow_data::forever_birth::BirthRecords;
use wow_persistence::forever::spells::{SpellLearnRow, SpellRequiredRow};

#[test]
fn absent_removed_temporary_and_already_disabled_guards_have_no_effects() {
    let fixture = Fixture::simple(vec![row(1)]);
    let mut effects = Effects::quiet();
    let mut book = PlayerSpellBook::default();
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    for state in [
        PlayerSpellState::Removed,
        PlayerSpellState::Temporary,
        PlayerSpellState::Changed,
    ] {
        insert(&mut book, 1, state, true, true);
        let mut request = RemovePlayerSpell::new(1);
        request.disabled = true;
        book.remove_spell(request, &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(book.spell(1).unwrap().state(), state);
    }
    assert!(effects.events.is_empty());
}
#[test]
fn new_is_erased_saved_states_retained_as_removed_and_disable_preserves_new() {
    let fixture = Fixture::simple(vec![row(1)]);
    for state in [
        PlayerSpellState::Unchanged,
        PlayerSpellState::Changed,
        PlayerSpellState::New,
    ] {
        for disable in [false, true] {
            let mut book = PlayerSpellBook::default();
            insert(&mut book, 1, state, true, false);
            let mut effects = Effects::quiet();
            let mut request = RemovePlayerSpell::new(1);
            request.disabled = disable;
            request.suppress_messaging = true;
            book.remove_spell(request, &fixture.sources(), &mut effects)
                .unwrap();
            let final_state = if disable {
                Some(if state == PlayerSpellState::New {
                    state
                } else {
                    PlayerSpellState::Changed
                })
            } else if state == PlayerSpellState::New {
                None
            } else {
                Some(PlayerSpellState::Removed)
            };
            assert_eq!(book.spell(1).map(|entry| entry.state()), final_state);
            assert!(!book.has_spell(1));
            assert_eq!(
                effects.events,
                [
                    Event::OwnedAura(1, final_state),
                    Event::Message(
                        SpellBookMessage::Unlearned {
                            spell: 1,
                            suppress_messaging: true
                        },
                        false,
                        false
                    )
                ]
            );
            assert_eq!(
                book.membership.len(),
                if state == PlayerSpellState::New && !disable {
                    2
                } else {
                    1
                }
            );
        }
    }
}
#[test]
fn higher_rank_and_required_dependencies_are_removed_first_with_recursive_default_flags() {
    let fixture = Fixture::new(
        vec![row(1), row(2), row(3)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![SpellRequiredRow {
            spell: 3,
            required: 1,
        }],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    for id in 1..=3 {
        insert(&mut book, id, PlayerSpellState::New, true, false);
    }
    let mut effects = Effects::quiet();
    let mut request = RemovePlayerSpell::new(1);
    request.suppress_messaging = true;
    book.remove_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert!(book.is_empty());
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(2, None),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 2,
                    suppress_messaging: false
                },
                false,
                false
            ),
            Event::OwnedAura(3, None),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 3,
                    suppress_messaging: false
                },
                false,
                false
            ),
            Event::OwnedAura(1, None),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 1,
                    suppress_messaging: true
                },
                false,
                false
            )
        ]
    );
}
#[test]
fn talent_next_rank_is_not_recursively_unlearned() {
    let mut talent = row(2);
    talent.2 = 0x00800000;
    let fixture = Fixture::new(
        vec![row(1), talent],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    for id in 1..=2 {
        insert(&mut book, id, PlayerSpellState::New, true, false);
    }
    let mut effects = Effects::quiet();
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(!book.has_spell(1) && book.has_spell(2));
    assert_eq!(book.len(), 1);
    assert!(
        !effects
            .events
            .iter()
            .any(|event| matches!(event, Event::OwnedAura(2, _)))
    );
}
#[test]
fn other_active_teacher_preserves_child_even_if_teacher_inactive_in_book() {
    let fixture = Fixture::new(
        vec![row(1), row(2), row(3)],
        Default::default(),
        vec![],
        vec![],
        vec![
            SpellLearnRow {
                source: 1,
                learned: 3,
                active: true,
            },
            SpellLearnRow {
                source: 2,
                learned: 3,
                active: true,
            },
        ],
    );
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    insert(&mut book, 2, PlayerSpellState::New, false, false);
    insert(&mut book, 3, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(book.has_spell(3));
    effects.events.clear();
    book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(!book.has_spell(3));
    assert!(effects.events.contains(&Event::OwnedAura(3, None)));
}
#[test]
fn inactive_relationship_and_disabled_teacher_do_not_preserve_child() {
    for (relationship_active, teacher_disabled) in [(false, false), (true, true)] {
        let fixture = Fixture::new(
            vec![row(1), row(2), row(3)],
            Default::default(),
            vec![],
            vec![],
            vec![
                SpellLearnRow {
                    source: 1,
                    learned: 3,
                    active: true,
                },
                SpellLearnRow {
                    source: 2,
                    learned: 3,
                    active: relationship_active,
                },
            ],
        );
        let mut book = PlayerSpellBook::default();
        for id in [1, 3] {
            insert(&mut book, id, PlayerSpellState::New, true, false);
        }
        insert(&mut book, 2, PlayerSpellState::New, true, teacher_disabled);
        let mut effects = Effects::quiet();
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
            .unwrap();
        assert!(!book.has_spell(3));
    }
}
#[test]
fn downgrade_promotes_dependency_and_sends_superseded_without_world_gate_or_unlearned() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, false, false);
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    book.entries.get_mut(&2).unwrap().dependent = true;
    let mut effects = Effects::quiet();
    book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(book.has_active_spell(1) && !book.has_spell(2));
    assert!(book.spell(1).unwrap().dependent());
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Unchanged); // load activation resets earlier dirty state.
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(2, None),
            Event::Message(SpellBookMessage::Superseded { old: 2, new: 1 }, false, true)
        ]
    );
}
#[test]
fn no_lower_activation_still_updates_dependency_and_sends_unlearned() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, false, false);
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    book.entries.get_mut(&2).unwrap().dependent = true;
    let mut effects = Effects::quiet();
    let mut request = RemovePlayerSpell::new(2);
    request.learn_low_rank = false;
    book.remove_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert!(!book.has_active_spell(1));
    assert!(book.spell(1).unwrap().dependent());
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Changed);
    assert!(matches!(
        effects.events.last(),
        Some(Event::Message(
            SpellBookMessage::Unlearned { spell: 2, .. },
            _,
            _
        ))
    ));
}
#[test]
fn required_cycle_is_reported_without_losing_prior_completed_removals() {
    let fixture = Fixture::new(
        vec![row(1), row(2), row(3)],
        Default::default(),
        vec![],
        vec![
            SpellRequiredRow {
                spell: 3,
                required: 1,
            },
            SpellRequiredRow {
                spell: 2,
                required: 1,
            },
            SpellRequiredRow {
                spell: 1,
                required: 2,
            },
        ],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    for id in 1..=3 {
        insert(&mut book, id, PlayerSpellState::New, true, false);
    }
    let mut effects = Effects::quiet();
    assert_eq!(
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects),
        Err(SpellLearningError::Source(
            SpellLearningSourceError::RecursiveRemovalCycle
        ))
    );
    assert!(!book.has_spell(3) && book.has_spell(1) && book.has_spell(2));
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(3, None),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 3,
                    suppress_messaging: false
                },
                false,
                false
            )
        ]
    );
}
#[test]
fn completed_child_reentry_researches_original_without_repeating_state_guards() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        Default::default(),
        vec![],
        vec![SpellRequiredRow {
            spell: 2,
            required: 1,
        }],
        vec![],
    );
    let sources = fixture.sources();
    for state in [PlayerSpellState::New, PlayerSpellState::Unchanged] {
        let mut book = PlayerSpellBook::default();
        insert(&mut book, 1, state, true, false);
        insert(&mut book, 2, PlayerSpellState::New, true, false);
        let mut effects = Effects::quiet();
        effects.reentry_sources = Some(&sources);
        effects.aura_reenter = Some((2, RemovePlayerSpell::new(1)));
        book.remove_spell(RemovePlayerSpell::new(1), &sources, &mut effects)
            .unwrap();
        assert!(!book.has_spell(1));
        assert_eq!(
            effects
                .events
                .iter()
                .filter(|event| matches!(event, Event::OwnedAura(1, _)))
                .count(),
            if state == PlayerSpellState::New { 1 } else { 2 }
        );
    }
}

#[test]
fn dependency_override_cleanup_is_skipped_with_other_teacher_and_not_gated_on_parent_relation_active()
 {
    for other_teacher in [false, true] {
        let mut fixture = Fixture::new(
            vec![row(1), row(2), row(3)],
            Default::default(),
            vec![],
            vec![],
            vec![
                SpellLearnRow {
                    source: 1,
                    learned: 3,
                    active: false,
                },
                SpellLearnRow {
                    source: 2,
                    learned: 3,
                    active: true,
                },
            ],
        );
        crate::forever::spells::spell_book_test_learn_flags(&mut fixture.spells, 1, 3, 777, false);
        let mut book = PlayerSpellBook::default();
        for id in [1, 3] {
            insert(&mut book, id, PlayerSpellState::New, true, false);
        }
        if other_teacher {
            insert(&mut book, 2, PlayerSpellState::New, false, false);
        }
        book.add_override_spell(777, 3);
        let mut effects = Effects::quiet();
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(book.has_spell(3), other_teacher);
        assert_eq!(book.has_override_spell(777, 3), other_teacher);
    }
}

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

#[test]
fn next_definition_assert_precedes_has_spell_and_missing_own_definition_errors_after_aura_removal()
{
    let mut fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    crate::forever::spells::skill_set_test_spell_fields(&mut fixture.spells, 2, None);
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    assert_eq!(
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects),
        Err(SpellLearningError::Source(
            SpellLearningSourceError::MissingAssertedDefinition
        ))
    );
    assert!(book.has_spell(1));
    assert!(effects.events.is_empty());
    insert(&mut book, 99, PlayerSpellState::Unchanged, true, false);
    assert_eq!(
        book.remove_spell(RemovePlayerSpell::new(99), &fixture.sources(), &mut effects),
        Err(SpellLearningError::Source(
            SpellLearningSourceError::MissingAssertedDefinition
        ))
    );
    assert_eq!(book.spell(99).unwrap().state(), PlayerSpellState::Removed);
    assert_eq!(
        effects.events,
        [Event::OwnedAura(99, Some(PlayerSpellState::Removed))]
    );
}

#[test]
fn invalid_lower_rank_reactivation_fences_global_cleanup_after_completed_removal_prefix() {
    let mut invalid = row(1);
    let mut craft = spell_effect(24, 0, 0, 0.);
    craft.item_type = 99;
    invalid.3.push(craft);
    let fixture = Fixture::new(
        vec![invalid, row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, false, false);
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    assert_eq!(
        book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects),
        Err(SpellLearningError::Source(
            SpellLearningSourceError::UnauthorizedGlobalCleanup
        ))
    );
    assert!(!book.has_spell(2));
    assert!(!book.has_active_spell(1));
    assert_eq!(effects.events, [Event::OwnedAura(2, None)]);
}

#[test]
fn lower_rank_manual_dependency_sync_can_demote_unlike_addspell_promotion_only() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    for dependent in [false, true] {
        let mut book = PlayerSpellBook::default();
        insert(&mut book, 1, PlayerSpellState::Unchanged, false, false);
        insert(&mut book, 2, PlayerSpellState::New, true, false);
        book.entries.get_mut(&1).unwrap().dependent = !dependent;
        book.entries.get_mut(&2).unwrap().dependent = dependent;
        let mut effects = Effects::quiet();
        let mut request = RemovePlayerSpell::new(2);
        request.learn_low_rank = false;
        book.remove_spell(request, &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(book.spell(1).unwrap().dependent(), dependent);
        assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Changed);
    }
}

#[test]
fn disabling_current_does_not_activate_a_disabled_lower_rank_or_publish_superseded() {
    let fixture = Fixture::new(
        vec![row(1), row(2)],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, false, true);
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    let mut request = RemovePlayerSpell::new(2);
    request.disabled = true;
    book.remove_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert!(!book.has_spell(1) && !book.has_spell(2));
    assert_eq!(book.len(), 2);
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(2, Some(PlayerSpellState::New)),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 2,
                    suppress_messaging: false
                },
                false,
                false
            )
        ]
    );
}
