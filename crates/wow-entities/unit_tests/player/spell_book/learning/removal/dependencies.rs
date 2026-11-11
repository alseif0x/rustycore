use super::super::*;

#[test]
fn higher_rank_and_required_dependencies_are_removed_first_with_recursive_default_flags() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                required: vec![3],
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
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
            SpellFacts::plain(3, 3),
        ],
        lines: vec![(0, None)],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                definition: Some(DefinitionFacts {
                    custom: 8388608,
                    ..DefinitionFacts::regular(2)
                }),
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
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                forward: vec![SpellLearningNode {
                    source: 1,
                    spell: 3,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: false,
                }],
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                forward: vec![SpellLearningNode {
                    source: 2,
                    spell: 3,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: false,
                }],
                ..SpellFacts::plain(2, 2)
            },
            SpellFacts {
                reverse: vec![
                    SpellLearningNode {
                        source: 1,
                        spell: 3,
                        overrides_spell: 0,
                        active: true,
                        auto_learned: false,
                    },
                    SpellLearningNode {
                        source: 2,
                        spell: 3,
                        overrides_spell: 0,
                        active: true,
                        auto_learned: false,
                    },
                ],
                ..SpellFacts::plain(3, 3)
            },
        ],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
        let fixture = Fixture {
            spells: vec![
                SpellFacts {
                    forward: vec![SpellLearningNode {
                        source: 1,
                        spell: 3,
                        overrides_spell: 0,
                        active: true,
                        auto_learned: false,
                    }],
                    ..SpellFacts::plain(1, 1)
                },
                SpellFacts {
                    forward: vec![SpellLearningNode {
                        source: 2,
                        spell: 3,
                        overrides_spell: 0,
                        active: relationship_active,
                        auto_learned: false,
                    }],
                    ..SpellFacts::plain(2, 2)
                },
                SpellFacts {
                    reverse: vec![
                        SpellLearningNode {
                            source: 1,
                            spell: 3,
                            overrides_spell: 0,
                            active: true,
                            auto_learned: false,
                        },
                        SpellLearningNode {
                            source: 2,
                            spell: 3,
                            overrides_spell: 0,
                            active: relationship_active,
                            auto_learned: false,
                        },
                    ],
                    ..SpellFacts::plain(3, 3)
                },
            ],
            lines: vec![],
            race_class: vec![],
            tiers: vec![],
            defaults: vec![],
        };
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
fn required_cycle_is_reported_without_losing_prior_completed_removals() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                required: vec![3, 2],
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                required: vec![1],
                ..SpellFacts::plain(2, 2)
            },
            SpellFacts::plain(3, 3),
        ],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                required: vec![2],
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts::plain(2, 2),
        ],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
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
        let fixture = Fixture {
            spells: vec![
                SpellFacts {
                    forward: vec![SpellLearningNode {
                        source: 1,
                        spell: 3,
                        overrides_spell: 777,
                        active: false,
                        auto_learned: false,
                    }],
                    ..SpellFacts::plain(1, 1)
                },
                SpellFacts {
                    forward: vec![SpellLearningNode {
                        source: 2,
                        spell: 3,
                        overrides_spell: 0,
                        active: true,
                        auto_learned: false,
                    }],
                    ..SpellFacts::plain(2, 2)
                },
                SpellFacts {
                    reverse: vec![
                        SpellLearningNode {
                            source: 1,
                            spell: 3,
                            overrides_spell: 777,
                            active: false,
                            auto_learned: false,
                        },
                        SpellLearningNode {
                            source: 2,
                            spell: 3,
                            overrides_spell: 0,
                            active: true,
                            auto_learned: false,
                        },
                    ],
                    ..SpellFacts::plain(3, 3)
                },
            ],
            lines: vec![],
            race_class: vec![],
            tiers: vec![],
            defaults: vec![],
        };
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
