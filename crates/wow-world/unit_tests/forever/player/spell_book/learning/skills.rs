use super::*;
use wow_data::forever_birth::BirthRecords;
use wow_persistence::forever::creation::SkillTierRow;

#[test]
fn classic_zero_learn_skill_starts_at_one_tier_step_zero_promotes_to_uint32_max() {
    let mut spell = row(1);
    spell.3.push(spell_effect(118, 0, 5, 0.));
    let mut values = [0; 16];
    values[0] = 75;
    values[15] = 750;
    let fixture = Fixture::new(
        vec![spell],
        BirthRecords {
            skill_lines: vec![line(5, 9)],
            race_class: vec![rc(1, 5, 1)],
            ..Default::default()
        },
        vec![SkillTierRow { id: 1, values }],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.free = 2;
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.events,
        [Event::Skill(5, 0, 1, 750), Event::MountQuery(1)]
    );
    assert_eq!(effects.free, 2); // Category 9 is secondary, independent of the tier range.
}

#[test]
fn learned_skill_does_not_reduce_existing_rank_or_cap_and_from_skill_skips_its_command() {
    let mut spell = row(1);
    spell.3.push(spell_effect(118, 0, 5, 2.));
    let fixture = Fixture::new(
        vec![spell],
        BirthRecords {
            skill_lines: vec![line(5, 7)],
            race_class: vec![rc(1, 5, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    let mut book = PlayerSpellBook::default();
    let mut effects = Effects::quiet();
    effects.skills.insert(5, (80, 100));
    book.add_spell(
        AddPlayerSpell::learned(1, true, false),
        &fixture.sources(),
        &mut effects,
    )
    .unwrap();
    assert!(effects.events.contains(&Event::Skill(5, 2, 80, 100)));
    let mut book = PlayerSpellBook::default();
    effects.events.clear();
    let mut request = AddPlayerSpell::learned(1, true, false);
    request.from_skill = 5;
    book.add_spell(request, &fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.events, [Event::MountQuery(1)]);
}

#[test]
fn language_mono_and_always_max_follow_source_range_and_flag_precedence() {
    for (category, flags, value, maximum) in [(10, 0, 300, 300), (8, 0, 1, 1), (7, 0x10, 50, 50)] {
        let mut spell = row(1);
        spell.3.push(spell_effect(118, 0, 5, 1.));
        let mut rc = rc(1, 5, 0);
        rc.flags = flags;
        let fixture = Fixture::new(
            vec![spell],
            BirthRecords {
                skill_lines: vec![line(5, category)],
                race_class: vec![rc],
                ..Default::default()
            },
            vec![],
            vec![],
            vec![],
        );
        let mut book = PlayerSpellBook::default();
        let mut effects = Effects::quiet();
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects,
        )
        .unwrap();
        assert!(effects.events.contains(&Event::Skill(5, 1, value, maximum)));
    }
}

#[test]
fn automatic_char_level_uses_live_has_skill_but_runeforging_special_case_still_calls_default() {
    let mut automatic = ability(1, 0, 1, 5);
    automatic.acquire_method = 2;
    let fixture = Fixture::new(
        vec![row(1)],
        BirthRecords {
            skill_lines: vec![line(5, 7), line(960, 7)],
            abilities: vec![automatic, ability(2, 0, 1, 960)],
            race_class: vec![rc(1, 5, 0), rc(2, 960, 0)],
            ..Default::default()
        },
        vec![],
        vec![],
        vec![],
    );
    for present in [false, true] {
        let mut book = PlayerSpellBook::default();
        let mut effects = Effects::quiet();
        if present {
            effects.skills.insert(5, (1, 10));
        }
        effects.skills.insert(960, (1, 1));
        book.add_spell(
            AddPlayerSpell::learned(1, true, false),
            &fixture.sources(),
            &mut effects,
        )
        .unwrap();
        assert_eq!(
            effects.events.contains(&Event::Skill(5, 0, 1, 50)),
            !present
        );
        assert!(effects.events.contains(&Event::Skill(960, 0, 1, 1)));
    }
}

#[test]
fn unranked_profession_category_and_parent_control_points_through_add_and_remove() {
    for (category, parent, primary) in [(11, 0, true), (9, 0, false), (11, 6, false)] {
        let mut skill_line = line(5, category);
        skill_line.parent_skill = parent;
        let mut spell = row(1);
        spell.3.push(spell_effect(118, 0, 5, 1.));
        let fixture = Fixture::new(
            vec![spell],
            BirthRecords {
                skill_lines: vec![skill_line, line(6, 11)],
                race_class: vec![rc(1, 5, 0)],
                ..Default::default()
            },
            vec![],
            vec![],
            vec![],
        );
        assert!(fixture.spells.spell_rank_node(1).is_none());
        let mut book = PlayerSpellBook::default();
        let mut effects = Effects::quiet();
        effects.free = 2;
        assert!(
            book.add_spell(
                AddPlayerSpell::learned(1, true, false),
                &fixture.sources(),
                &mut effects,
            )
            .unwrap()
        );
        // Exercise the absent-chain rank-1 fallback through the complete Add operation.
        let mut added = vec![];
        if primary {
            added.push(Event::Points(1));
        }
        added.extend([Event::Skill(5, 1, 1, 50), Event::MountQuery(1)]);
        assert_eq!(effects.events, added);
        assert_eq!(effects.free, if primary { 1 } else { 2 });
        assert!(book.has_spell(1));
        assert!(book.has_active_spell(1));
        assert_eq!(book.spell(1).unwrap().state(), PlayerSpellState::New);
        assert_eq!(book.len(), 1);

        effects.events.clear();
        effects.pet_auras.insert((1, 0));
        book.remove_spell(RemovePlayerSpell::new(1), &fixture.sources(), &mut effects)
            .unwrap();
        let mut removed = vec![
            Event::OwnedAura(1, None),
            Event::PetQuery(1, 0),
            Event::PetRemove(1, 0),
        ];
        if primary {
            removed.push(Event::Points(2));
        }
        removed.extend([
            Event::Skill(5, 0, 0, 0),
            Event::Message(
                SpellBookMessage::Unlearned {
                    spell: 1,
                    suppress_messaging: false,
                },
                false,
                false,
            ),
        ]);
        assert_eq!(effects.events, removed);
        assert_eq!(effects.free, 2);
        assert!(!book.has_spell(1));
        assert!(!book.has_active_spell(1));
        assert!(book.spell(1).is_none());
        assert!(book.is_empty());
    }
}
