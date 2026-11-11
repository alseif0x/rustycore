use super::super::*;

#[test]
fn downgrade_promotes_dependency_and_sends_superseded_without_world_gate_or_unlearned() {
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
fn lower_rank_manual_dependency_sync_can_demote_unlike_addspell_promotion_only() {
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
