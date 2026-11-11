use super::super::*;

#[test]
fn prior_tier_restoration_clamps_value_and_cap_without_inventing_minimum_one() {
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
                    category: 9,
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
                tier: 1,
            }),
        )],
        tiers: vec![((1, 0), Some(75))],
        defaults: vec![],
    };
    for (value, maximum, expected_value, expected_maximum) in
        [(200, 300, 75, 75), (10, 20, 10, 20), (0, 0, 0, 0)]
    {
        let mut book = PlayerSpellBook::default();
        insert(&mut book, 2, PlayerSpellState::New, true, false);
        let mut effects = Effects::quiet();
        effects.skills.insert(5, (value, maximum));
        book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
            .unwrap();
        assert!(
            effects
                .events
                .contains(&Event::Skill(5, 1, expected_value, expected_maximum))
        );
    }
}

#[test]
fn previous_rank_gap_uses_first_rank_quirk_instead_of_nearest_skill_node() {
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
                        misc: 6,
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
                next: 3,
                learned_skill: Some(SpellLearningSkill {
                    skill: 6,
                    step: 2,
                    value: 0,
                    max_value: 0,
                }),
                ..SpellFacts::plain(2, 1)
            },
            SpellFacts {
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 3,
                }),
                previous: 2,
                next: 4,
                learned_skill: None,
                ..SpellFacts::plain(3, 1)
            },
            SpellFacts {
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 118,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 7,
                        mask: [0; 4],
                    }],
                    has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
                    ..DefinitionFacts::regular(4)
                }),
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 4,
                }),
                previous: 3,
                next: 0,
                learned_skill: Some(SpellLearningSkill {
                    skill: 7,
                    step: 3,
                    value: 0,
                    max_value: 0,
                }),
                ..SpellFacts::plain(4, 1)
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
            (
                6,
                Some(SkillLineMetadata {
                    id: 6,
                    category: 7,
                    parent_skill: 0,
                    parent_tier_index: 0,
                }),
            ),
            (
                7,
                Some(SkillLineMetadata {
                    id: 7,
                    category: 7,
                    parent_skill: 0,
                    parent_tier_index: 0,
                }),
            ),
        ],
        race_class: vec![
            (
                5,
                Some(SkillRaceClassMetadata {
                    id: 1,
                    skill: 5,
                    flags: 0,
                    min_level: 1,
                    tier: 0,
                }),
            ),
            (
                6,
                Some(SkillRaceClassMetadata {
                    id: 2,
                    skill: 6,
                    flags: 0,
                    min_level: 1,
                    tier: 0,
                }),
            ),
            (
                7,
                Some(SkillRaceClassMetadata {
                    id: 3,
                    skill: 7,
                    flags: 0,
                    min_level: 1,
                    tier: 0,
                }),
            ),
        ],
        tiers: vec![((0, 0), None)],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 4, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.skills.insert(5, (100, 200));
    effects.skills.insert(6, (100, 200));
    book.remove_spell(RemovePlayerSpell::new(4), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.events.contains(&Event::Skill(5, 1, 50, 50)));
    assert!(
        !effects
            .events
            .iter()
            .any(|event| matches!(event, Event::Skill(6, _, _, _) | Event::Skill(7, _, _, _)))
    );
}

#[test]
fn fixed_maximum_previous_dual_wield_skill_uses_previous_value_before_cap() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 40,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    }],
                    has_effect: vec![(36, false), (44, false), (155, false), (40, true)],
                    ..DefinitionFacts::regular(1)
                }),
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                learned_skill: Some(SpellLearningSkill {
                    skill: 118,
                    step: 1,
                    value: 1,
                    max_value: 1,
                }),
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 118,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 7,
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
                    skill: 7,
                    step: 2,
                    value: 0,
                    max_value: 0,
                }),
                ..SpellFacts::plain(2, 1)
            },
        ],
        lines: vec![(0, None)],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.skills.insert(118, (200, 300));
    book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.events.contains(&Event::Skill(118, 1, 1, 1)));
}

#[test]
fn prior_zero_step_tier_narrows_before_always_max_and_cap_and_value_clamp() {
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
                    step: 0,
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
                    step: 1,
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
                    category: 9,
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
                flags: 16,
                min_level: 1,
                tier: 1,
            }),
        )],
        tiers: vec![((1, 0), Some(75)), ((1, 4294967295), Some(70000))],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.skills.insert(5, (0, 9000));
    book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(
        effects
            .events
            .contains(&Event::Skill(5, 0, 70000u32 as u16, 70000u32 as u16))
    );
}

#[test]
fn exhausted_gap_search_still_queries_first_of_zero_as_in_source() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts {
                definition: Some(DefinitionFacts {
                    effects: vec![EffectFacts {
                        kind: 118,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 9,
                        mask: [0; 4],
                    }],
                    has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
                    ..DefinitionFacts::regular(0)
                }),
                learned_skill: Some(SpellLearningSkill {
                    skill: 9,
                    step: 1,
                    value: 0,
                    max_value: 0,
                }),
                ..SpellFacts::plain(0, 0)
            },
            SpellFacts {
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 1,
                }),
                previous: 0,
                next: 2,
                learned_skill: None,
                ..SpellFacts::plain(1, 1)
            },
            SpellFacts {
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 2,
                }),
                previous: 1,
                next: 3,
                learned_skill: None,
                ..SpellFacts::plain(2, 1)
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
                    ..DefinitionFacts::regular(3)
                }),
                rank: Some(SpellLearningRank {
                    first: (1, 0),
                    rank: 3,
                }),
                previous: 2,
                next: 0,
                learned_skill: Some(SpellLearningSkill {
                    skill: 5,
                    step: 1,
                    value: 0,
                    max_value: 0,
                }),
                ..SpellFacts::plain(3, 1)
            },
        ],
        lines: vec![(
            9,
            Some(SkillLineMetadata {
                id: 9,
                category: 7,
                parent_skill: 0,
                parent_tier_index: 0,
            }),
        )],
        race_class: vec![(
            9,
            Some(SkillRaceClassMetadata {
                id: 1,
                skill: 9,
                flags: 0,
                min_level: 1,
                tier: 0,
            }),
        )],
        tiers: vec![((0, 0), None)],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 3, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.skills.insert(9, (100, 200));
    book.remove_spell(RemovePlayerSpell::new(3), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.events.contains(&Event::Skill(9, 1, 50, 50)));
    assert!(
        !effects
            .events
            .iter()
            .any(|event| matches!(event, Event::Skill(5, _, _, _)))
    );
}
