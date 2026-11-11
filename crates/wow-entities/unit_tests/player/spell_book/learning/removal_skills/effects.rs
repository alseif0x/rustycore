use super::super::*;

#[test]
fn unavailable_pet_data_and_effect_failure_keep_mutated_prefix_and_stop_later_commands() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
            definition: Some(DefinitionFacts {
                effects: vec![EffectFacts {
                    kind: 0,
                    aura: false,
                    aura_kinds: vec![],
                    misc: 0,
                    mask: [0; 4],
                }],
                has_effect: vec![(36, false), (44, false), (155, false), (40, false)],
                ..DefinitionFacts::regular(1)
            }),
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    for unavailable in [false, true] {
        let mut book = PlayerSpellBook::default();
        insert(&mut book, 1, PlayerSpellState::Unchanged, true, false);
        let mut effects = Effects::quiet();
        effects.pet_catalog_unavailable = unavailable;
        if !unavailable {
            effects.fail = Some(Event::OwnedAura(1, Some(PlayerSpellState::Removed)));
        }
        assert_eq!(
            book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects),
            Err(SpellLearningError::Effect(if unavailable {
                "pet aura catalog unavailable"
            } else {
                "required effect failed"
            }))
        );
        assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::Removed);
        assert_eq!(effects.events.len(), if unavailable { 2 } else { 1 });
    }
}

#[test]
fn passive_weapon_capability_removal_keeps_pet_slot_order_and_offhand_before_publication() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
            definition: Some(DefinitionFacts {
                attributes: [64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                effects: vec![
                    EffectFacts {
                        kind: 155,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    },
                    EffectFacts {
                        kind: 0,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    },
                    EffectFacts {
                        kind: 40,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    },
                ],
                has_effect: vec![(36, false), (44, false), (155, true), (40, true)],
                ..DefinitionFacts::regular(1)
            }),
            learned_skill: Some(SpellLearningSkill {
                skill: 118,
                step: 1,
                value: 1,
                max_value: 1,
            }),
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.titan = true;
    effects.titan_penalty = 22;
    effects.dual = true;
    effects.offhand = true;
    effects.pet_auras.insert((1, 2));
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(1, None),
            Event::PetQuery(1, 0),
            Event::PetQuery(1, 1),
            Event::PetQuery(1, 2),
            Event::PetRemove(1, 2),
            Event::Skill(118, 0, 0, 0),
            Event::RemoveAura(22),
            Event::TitanOff,
            Event::DualOff,
            Event::Offhand,
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
    assert!(!effects.titan && !effects.dual);
    assert_eq!(effects.titan_penalty, 0);
}

#[test]
fn offhand_failure_prevents_success_publication_without_restoring_the_removed_spell() {
    let fixture = Fixture {
        spells: vec![SpellFacts::plain(1, 1)],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.offhand = true;
    effects.fail = Some(Event::Offhand);
    assert_eq!(
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects),
        Err(SpellLearningError::Effect("required effect failed"))
    );
    assert!(!book.has_spell(1));
    assert_eq!(effects.events, [Event::OwnedAura(1, None), Event::Offhand]);
}

#[test]
fn root_trait_and_override_group_retire_after_downgrade_phase() {
    let fixture = Fixture {
        spells: vec![
            SpellFacts::plain(1, 1),
            SpellFacts::plain(2, 2),
            SpellFacts::plain(3, 3),
        ],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    book.entries.get_mut(&1).unwrap().trait_data = Some(PlayerSpellTrait::new(4, 1));
    book.add_override_spell(0, 1);
    book.add_override_spell(1, 2);
    book.add_override_spell(1, 3);
    let mut effects = Effects::quiet();
    effects.traits.insert(4, 0);
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(!book.overrides.contains_key(&0) && !book.overrides.contains_key(&1));
    assert_eq!(
        effects.events,
        [
            Event::OwnedAura(1, None),
            Event::Trait(4),
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
fn unavailable_old_trait_does_not_fake_cleanup_or_publish_after_removal() {
    let fixture = Fixture {
        spells: vec![SpellFacts::plain(1, 1)],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    book.entries.get_mut(&1).unwrap().trait_data = Some(PlayerSpellTrait::new(4, 1));
    book.add_override_spell(0, 1);
    book.add_override_spell(1, 2);
    let mut effects = Effects::quiet();
    effects.trait_unavailable = true;
    assert_eq!(
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects),
        Err(SpellLearningError::Effect("trait catalog unavailable"))
    );
    assert!(!book.has_spell(1));
    assert!(book.has_override_spell(0, 1) && book.has_override_spell(1, 2));
    assert_eq!(effects.events, [Event::OwnedAura(1, None), Event::Trait(4)]);
}

#[test]
fn nonpassive_capability_spells_do_not_disable_weapon_flags_but_offhand_config_still_runs() {
    let fixture = Fixture {
        spells: vec![SpellFacts {
            definition: Some(DefinitionFacts {
                attributes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                effects: vec![
                    EffectFacts {
                        kind: 155,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    },
                    EffectFacts {
                        kind: 40,
                        aura: false,
                        aura_kinds: vec![],
                        misc: 0,
                        mask: [0; 4],
                    },
                ],
                has_effect: vec![(36, false), (44, false), (155, true), (40, true)],
                ..DefinitionFacts::regular(1)
            }),
            learned_skill: Some(SpellLearningSkill {
                skill: 118,
                step: 1,
                value: 1,
                max_value: 1,
            }),
            ..SpellFacts::plain(1, 1)
        }],
        lines: vec![],
        race_class: vec![],
        tiers: vec![],
        defaults: vec![],
    };
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 1, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.titan = true;
    effects.dual = true;
    effects.offhand = true;
    book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.titan && effects.dual);
    assert!(effects.events.contains(&Event::Offhand));
    assert!(!effects.events.iter().any(|event| matches!(
        event,
        Event::TitanOff | Event::DualOff | Event::RemoveAura(_)
    )));
}
