use super::*;

#[test]
fn new_lower_rank_captured_active_return_is_not_its_final_active_field() {
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
    insert(&mut book, 2, PlayerSpellState::Unchanged, true, true);
    let mut effects = Effects::quiet();
    effects.world = true;
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    // Source rank loop tests old.active, not old.disabled/HasSpell.
    assert!(!book.spell(1).unwrap().active());
    assert!(effects.events.contains(&Event::Message(
        SpellBookMessage::Superseded { old: 1, new: 2 },
        true,
        false
    )));
}

#[test]
fn existing_lower_rank_sees_known_next_and_preserves_superseded_transition_return() {
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
    insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
    insert(&mut book, 2, PlayerSpellState::New, false, false); // HasSpell does not require active.
    let mut effects = Effects::quiet();
    effects.world = true;
    assert!(
        !book
            .add_spell(
                AddPlayerSpell::learned(1, true, false),
                &fixture.sources(),
                &mut effects
            )
            .unwrap()
    );
    assert_eq!(
        effects.events,
        [Event::Message(
            SpellBookMessage::Superseded { old: 1, new: 2 },
            false,
            false
        )]
    );
}

#[test]
fn disabled_learn_snapshots_flags_then_enables_next_and_required_without_quest_for_disabled() {
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
                required: vec![3],
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
            SpellFacts {
                valid: true,
                ..SpellFacts::plain(3, 3)
            },
        ],
        lines: vec![(0, None)],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    for id in 1..=3 {
        insert(&mut book, id, PlayerSpellState::Unchanged, false, true);
    }
    book.entries.get_mut(&1).unwrap().favorite = true;
    let mut effects = Effects::quiet();
    effects.world = true;
    book.learn_spell(
        LearnPlayerSpell::new(1, true, 7),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    for id in 1..=3 {
        assert!(book.has_spell(id) && !book.has_active_spell(id));
    }
    assert!(book.spell(1).unwrap().favorite());
    assert!(
        !effects
            .events
            .iter()
            .any(|event| matches!(event, Event::Quest(_)))
    );
    assert!(!effects.events.iter().any(|event| matches!(
        event,
        Event::Message(SpellBookMessage::Learned { .. }, _, _)
    )));
}

#[test]
fn learn_sends_captured_favorite_trait_suppression_and_duplicate_still_updates_quest() {
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
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Removed, true, false);
    book.entries.get_mut(&1).unwrap().favorite = true;
    let mut effects = Effects::quiet();
    effects.world = true;
    let mut request = LearnPlayerSpell::new(1, false, 0);
    request.suppress_messaging = true;
    request.trait_data = Some(PlayerSpellTrait::new(-4, 2));
    book.learn_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.events.contains(&Event::Message(
        SpellBookMessage::Learned {
            spell: 1,
            favorite: true,
            trait_definition: Some(-4),
            suppress_messaging: true
        },
        true,
        true
    )));
    effects.events.clear();
    book.learn_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.events, [Event::Quest(1)]);
}

#[test]
fn previous_rank_skill_reentry_inserts_original_before_outer_try_emplace_and_forces_changed() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 118,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 5,
                        mask: [0; 4],
                    }],
                    has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
                    ..DefinitionFacts::regular(1)
                }),
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                learned_skill: Some(SpellLearningSkill {
                    skill: 5,
                    step: 1,
                    value: 0,
                    max_value: 0,
                }),
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
                learned_skill: None,
                ..SpellFacts::plain(2, 1)
            },
        ],
        lines: vec![
            (
                5,
                Some(SkillLineMetadata {
                    id: 5,
                    category: 7,
                    parent_skill: 0,
                    parent_tier_index: 0,
                }),
            ),
            (0, None),
        ],
        race_class: vec![(
            5,
            Some(SkillRaceClassMetadata {
                id: 1,
                skill: 5,
                flags: 0,
                min_level: 1,
                tier: 0,
            }),
        )],
        tiers: vec![((0, 0), None)],
        defaults: vec![],
    };
    let sources = fixture.sources();
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.skill_reenter = Some((5, 2));
    effects.reentry_sources = Some(&sources);
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(2, true, false),
            &sources,
            &mut effects
        )
        .unwrap()
    );
    assert_eq!(book.spell(2).unwrap().state(), PlayerSpellState::Changed);
    assert!(!book.spell(2).unwrap().dependent());
    assert!(book.has_active_spell(2) && book.has_spell(1) && !book.has_active_spell(1));
    assert_eq!(
        book.membership,
        [SpellBookMutation::Insert(1), SpellBookMutation::Insert(2)]
    );
}

#[test]
fn cyclic_sql_learning_terminates_via_real_existing_flags_not_a_blanket_id_guard() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                forward: vec![SpellLearningNode {
                    source: 1,
                    spell: 2,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: false,
                }],
                reverse: vec![SpellLearningNode {
                    source: 2,
                    spell: 1,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: false,
                }],
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                forward: vec![SpellLearningNode {
                    source: 2,
                    spell: 1,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: false,
                }],
                reverse: vec![SpellLearningNode {
                    source: 1,
                    spell: 2,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: false,
                }],
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
    assert!(
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects
        )
        .unwrap()
    );
    assert!(book.has_active_spell(1) && book.has_active_spell(2));
    assert!(book.spell(1).unwrap().dependent() && book.spell(2).unwrap().dependent());
    assert_eq!(
        book.membership,
        [SpellBookMutation::Insert(1), SpellBookMutation::Insert(2)]
    );
    assert_eq!(effects.events, [Event::MountQuery(2), Event::MountQuery(1)]);
}
