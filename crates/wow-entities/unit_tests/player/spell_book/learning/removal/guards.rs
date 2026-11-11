use super::super::*;

#[test]
fn absent_removed_temporary_and_already_disabled_guards_have_no_effects() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
            valid: true,
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
    let fixture = Fixture {
        spells: vec![SpellFacts {
            valid: true,
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
fn next_definition_assert_precedes_has_spell_and_missing_own_definition_errors_after_aura_removal()
{
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                valid: true,
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                definition: None,
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 2,
                }),
                previous: 1,
                next: 0,
                abilities: vec![SpellLearningAbility {
                    skill_line: 0,
                    acquire_method: 0,
                    trivial_rank_high: 0,
                }],
                ..SpellFacts::plain(2, 1)
            },
            SpellFacts {
                definition: None,
                ..SpellFacts::plain(99, 99)
            },
        ],
        lines: vec![(0, None)],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 24,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    }],
                    has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
                    ..DefinitionFacts::regular(1)
                }),
                valid: false,
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                valid: true,
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 2,
                }),
                previous: 1,
                next: 0,
                abilities: vec![SpellLearningAbility {
                    skill_line: 0,
                    acquire_method: 0,
                    trivial_rank_high: 0,
                }],
                ..SpellFacts::plain(2, 1)
            },
        ],
        lines: vec![(0, None)],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
