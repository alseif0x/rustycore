use super::super::*;

#[test]
fn higher_primary_profession_rank_neither_consumes_nor_refunds_a_point() {
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
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 118,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 5,
                        mask: [0; 4],
                    }],
                    has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
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
                learned_skill: Some(SpellLearningSkill {
                    skill: 5,
                    step: 2,
                    value: 0,
                    max_value: 0,
                }),
                ..SpellFacts::plain(2, 1)
            },
        ],
        lines: vec![
            (
                5,
                Some(SkillLineMetadata {
                    id: 5,
                    category: 11,
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
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
    // Match Add's implicit previous-rank request, so it does not learn a new
    // first rank and legitimately consume a point on behalf of that operation.
    book.entries.get_mut(&1).unwrap().dependent = true;
    let mut effects = Effects::quiet();
    effects.free = 2;
    // Superseding the old rank makes AddSpell return false despite the newly
    // active spell; that source return value is not a persistence failure.
    assert!(
        !book
            .add_spell(
                AddPlayerSpell::learned(2, true, true),
                &fixture.sources(),
                &mut effects,
            )
            .unwrap()
    );
    assert_eq!(
        effects.events,
        [
            Event::Order,
            Event::Skill(5, 2, 1, 50),
            Event::MountQuery(2)
        ]
    );
    assert_eq!(effects.free, 2);
    assert!(book.has_active_spell(2));
    assert_eq!(book.spell(2).unwrap().state(), PlayerSpellState::New);
    assert!(book.has_spell(1));
    assert!(!book.has_active_spell(1));
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Changed);
    assert!(book.spell(1).unwrap().dependent());
    assert_eq!(book.len(), 2);

    effects.events.clear();
    effects.pet_auras.insert((2, 0));
    book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(2, None),
            Event::PetQuery(2, 0),
            Event::PetRemove(2, 0),
            Event::Skill(5, 1, 1, 50),
            Event::Message(SpellBookMessage::Superseded { old: 2, new: 1 }, false, true),
        ]
    );
    assert_eq!(effects.free, 2);
    assert!(book.spell(2).is_none());
    assert!(!book.has_spell(2));
    assert!(book.has_active_spell(1));
    assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Unchanged);
    assert!(book.spell(1).unwrap().dependent());
    assert_eq!(book.len(), 1);
}

#[test]
fn first_skill_rank_removal_zeros_the_source_skill_after_pet_slots_and_profession_points() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
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
            learned_skill: Some(SpellLearningSkill {
                skill: 5,
                step: 1,
                value: 0,
                max_value: 0,
            }),
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![(
            5,
            Some(SkillLineMetadata {
                id: 5,
                category: 11,
                parent_skill: 0,
                parent_tier_index: 0,
            }),
        )],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.pet_auras.insert((1, 0));
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(1, None),
            Event::PetQuery(1, 0),
            Event::PetRemove(1, 0),
            Event::Points(1),
            Event::Skill(5, 0, 0, 0),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 1,
                    suppress_messaging: false
                },
                false,
                false
            )
        ]
    );
}

#[test]
fn profession_refund_uses_uint32_wrap_and_config_ceiling_not_saturation_or_above_cap_increment() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
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
            learned_skill: Some(SpellLearningSkill {
                skill: 5,
                step: 1,
                value: 0,
                max_value: 0,
            }),
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![(
            5,
            Some(SkillLineMetadata {
                id: 5,
                category: 11,
                parent_skill: 0,
                parent_tier_index: 0,
            }),
        )],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    for (start, maximum, point_event) in [
        (0, 2, Some(1)),
        (1, 2, Some(2)),
        (2, 2, None),
        (u32::MAX, 2, Some(0)),
    ] {
        let mut book = PlayerSpellBook::default();
        insert(&mut book, 1, PlayerSpellState::New, true, false);
        let mut effects = Effects::quiet();
        effects.free = start;
        effects.max_professions = maximum;
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(
            effects
                .events
                .iter()
                .filter_map(|event| if let Event::Points(value) = event {
                    Some(*value)
                } else {
                    None
                })
                .collect::<Vec<_>>(),
            point_event.into_iter().collect::<Vec<_>>()
        );
    }
}
