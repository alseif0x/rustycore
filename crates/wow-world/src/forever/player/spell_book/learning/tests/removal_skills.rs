use super::*;
use wow_data::forever_birth::BirthRecords;
use wow_persistence::forever::creation::SkillTierRow;

#[test]
fn first_skill_rank_removal_zeros_the_source_skill_after_pet_slots_and_profession_points() {
    let mut spell = row(1);
    spell.3.push(spell_effect(118, 0, 5, 1.));
    let fixture = Fixture::new(
        vec![spell],
        BirthRecords {
            skill_lines: vec![line(5, 9)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
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
    let mut spell = row(1);
    spell.3.push(spell_effect(118, 0, 5, 1.));
    let fixture = Fixture::new(
        vec![spell],
        BirthRecords {
            skill_lines: vec![line(5, 9)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
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
#[test]
fn prior_tier_restoration_clamps_value_and_cap_without_inventing_minimum_one() {
    let mut first = row(1);
    first.3.push(spell_effect(118, 0, 5, 1.));
    let mut second = row(2);
    second.3.push(spell_effect(118, 0, 5, 2.));
    let fixture = Fixture::new(
        vec![first, second],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            skill_lines: vec![line(5, 9)],
            race_class: vec![rc(1, 5, 1)],
            ..Default::default()
        },
        vec![SkillTierRow {
            id: 1,
            values: [75; 16],
        }],
        vec![],
        vec![],
    );
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
    let mut first = row(1);
    first.3.push(spell_effect(118, 0, 5, 1.));
    let mut nearer = row(2);
    nearer.3.push(spell_effect(118, 0, 6, 2.));
    let mut removed = row(4);
    removed.3.push(spell_effect(118, 0, 7, 3.));
    let fixture = Fixture::new(
        vec![first, nearer, row(3), removed],
        BirthRecords {
            abilities: vec![
                ability(1, 1, 2, 0),
                ability(2, 2, 3, 0),
                ability(3, 3, 4, 0),
            ],
            skill_lines: vec![line(5, 7), line(6, 7), line(7, 7)],
            race_class: vec![rc(1, 5, 0), rc(2, 6, 0), rc(3, 7, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
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
    let mut first = row(1);
    first.3.push(spell_effect(40, 0, 0, 0.));
    let mut second = row(2);
    second.3.push(spell_effect(118, 0, 7, 2.));
    let fixture = Fixture::new(
        vec![first, second],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    insert(&mut book, 2, PlayerSpellState::New, true, false);
    let mut effects = Effects::quiet();
    effects.skills.insert(118, (200, 300));
    book.remove_spell(RemovePlayerSpell::new(2), &fixture.sources(), &mut effects)
        .unwrap();
    assert!(effects.events.contains(&Event::Skill(118, 1, 1, 1)));
}
#[test]
fn unavailable_pet_data_and_effect_failure_keep_mutated_prefix_and_stop_later_commands() {
    let mut spell = row(1);
    spell.3.push(spell_effect(0, 0, 0, 0.));
    let fixture = Fixture::simple(vec![spell]);
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
    let mut spell = row(1);
    spell.1.attributes[0] = 0x40;
    spell.3 = vec![
        spell_effect(155, 0, 0, 0.),
        spell_effect(0, 0, 0, 0.),
        spell_effect(40, 0, 0, 0.),
    ];
    let fixture = Fixture::simple(vec![spell]);
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
    let fixture = Fixture::simple(vec![row(1)]);
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
    let fixture = Fixture::simple(vec![row(1), row(2), row(3)]);
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
fn prior_zero_step_tier_narrows_before_always_max_and_cap_and_value_clamp() {
    let mut first = row(1);
    first.3.push(spell_effect(118, 0, 5, 0.));
    let mut second = row(2);
    second.3.push(spell_effect(118, 0, 5, 1.));
    let mut rc = rc(1, 5, 1);
    rc.flags = 0x10;
    let mut values = [0; 16];
    values[0] = 75;
    values[15] = 70000;
    let fixture = Fixture::new(
        vec![first, second],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0)],
            skill_lines: vec![line(5, 9)],
            race_class: vec![rc],
            ..Default::default()
        },
        vec![SkillTierRow { id: 1, values }],
        vec![],
        vec![],
    );
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
    let mut zero = row(0);
    zero.3.push(spell_effect(118, 0, 9, 1.));
    let mut removed = row(3);
    removed.3.push(spell_effect(118, 0, 5, 1.));
    let fixture = Fixture::new(
        vec![zero, row(1), row(2), removed],
        BirthRecords {
            abilities: vec![ability(1, 1, 2, 0), ability(2, 2, 3, 0)],
            skill_lines: vec![line(9, 7)],
            race_class: vec![rc(1, 9, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
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

#[test]
fn unavailable_old_trait_does_not_fake_cleanup_or_publish_after_removal() {
    let fixture = Fixture::simple(vec![row(1)]);
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
    let mut spell = row(1);
    spell.3 = vec![spell_effect(155, 0, 0, 0.), spell_effect(40, 0, 0, 0.)];
    let fixture = Fixture::simple(vec![spell]);
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
