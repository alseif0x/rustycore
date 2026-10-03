use super::*;

#[test]
fn exact_effect_and_aura_flag_sets_include_blank_slots_without_aura_admission() {
    let key = (900_001, 0);
    for kind in 0..361 {
        let mut s = fresh(
            Default::default(),
            [(key, definition(vec![effect(kind, 0, 0.0)]))],
        );
        // Enchant cases require the real admitted skill relation even when empty.
        s = prefix(s, vec![key], Default::default());
        let mut counts = CustomAttributeCounts::default();
        super::super::effect_flags::apply(&mut s, key, 0, &mut counts).unwrap();
        let mut expected = 0;
        if matches!(kind, 2 | 9 | 10 | 17 | 31 | 58 | 62 | 75 | 121 | 136 | 165) {
            expected |= CAN_CRIT;
        }
        if matches!(kind, 2 | 58 | 17 | 121 | 31 | 10) {
            expected |= DIRECT_DAMAGE;
        }
        if matches!(kind, 8 | 62 | 67 | 9 | 136 | 137 | 30 | 75) {
            expected |= NO_INITIAL_THREAT;
        }
        if matches!(kind, 96 | 149 | 41 | 42 | 138) {
            expected |= CHARGE;
        }
        if kind == 71 {
            expected |= PICKPOCKET;
        }
        assert_eq!(flags(&s, key), expected, "kind={kind}");
    }
    for aura in 0..665 {
        for kind in [0, 3, 6] {
            let mut s = fresh(
                Default::default(),
                [(key, definition(vec![effect(kind, aura, 0.0)]))],
            );
            super::super::effect_flags::apply(&mut s, key, 0, &mut Default::default()).unwrap();
            let expected = if matches!(aura, 2 | 5 | 6 | 177 | 7 | 12) {
                AURA_CC
            } else {
                0
            } | if matches!(aura, 292 | 236 | 1 | 2 | 378 | 6 | 177 | 398 | 397) {
                CANNOT_SAVE
            } else {
                0
            };
            assert_eq!(flags(&s, key), expected, "kind={kind} aura={aura}");
        }
    }
}

#[test]
fn bleed_and_undefined_shifts_keep_parent_and_active_effect_distinctions() {
    let key = (900_001, 0);
    for (kind, parent, child, expected) in [
        (0, 15, 64, true),
        (0, 0, 15, false),
        (3, 0, 15, true),
        (3, 15, 0, true),
    ] {
        let mut e = effect(kind, 0, 0.0);
        e.mechanic = child;
        let mut d = definition(vec![e]);
        d.fields.mechanic = parent;
        let mut s = fresh(Default::default(), [(key, d)]);
        super::super::effect_flags::apply(&mut s, key, 0, &mut Default::default()).unwrap();
        assert_eq!(flags(&s, key) & IGNORE_ARMOR != 0, expected);
    }
    for (kind, parent, child) in [(0, 64, 0), (3, 0, 64)] {
        let mut e = effect(kind, 0, 0.0);
        e.mechanic = child;
        let mut d = definition(vec![e]);
        d.fields.mechanic = parent;
        let mut s = fresh(Default::default(), [(key, d)]);
        assert_eq!(
            super::super::effect_flags::apply(&mut s, key, 0, &mut Default::default()),
            Err(SpellCustomAttributeError::UndefinedMechanicShift)
        );
    }
    let mut d = definition(vec![]);
    d.fields.mechanic = 64;
    // No effect slots means source never calls GetEffectMechanicMask.
    admitted(Default::default(), [(key, d)], vec![key])
        .with_custom_attributes(&items(), &mut no_draw)
        .unwrap();
}

fn skill(skill_line: u16, skillup_skill_line: i16) -> BirthRecords {
    BirthRecords {
        abilities: vec![SkillAbilityRecord {
            id: 1,
            skill_line,
            spell: 900_001,
            min_skill_rank: 0,
            class_mask: 0,
            supercedes_spell: 0,
            acquire_method: 0,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            unique_bit: 0,
            trade_skill_category: 0,
            skillup_skill_line,
            field_5_5_4_67090_014: [0; 2],
            race_mask: 0,
        }],
        ..Default::default()
    }
}

#[test]
fn enchant_foreign_writes_cover_all_signed_difficulties_but_skip_active_proc_auras() {
    let enchant_key = (900_001, 0);
    for (skill_line, skillup, expected_count) in [(333, 0, 6), (1, 333, 0)] {
        for kind in [53, 54, 360, 156, 92] {
            let mut enchantment = dependency_rows::spell_item_enchantment(7, 6, b"");
            enchantment.effect = [1, 1, 1];
            enchantment.effect_arg = [900_002; 3];
            let mut e = effect(kind, 0, 0.0);
            e.misc_values[0] = 7;
            let keys = [(900_002, i16::MIN), (900_002, -1), (900_002, i16::MAX)];
            let input = fresh(
                SpellRecords {
                    spell_item_enchantments: vec![enchantment],
                    ..Default::default()
                },
                [
                    (enchant_key, definition(vec![e])),
                    (keys[0], definition(vec![effect(0, 42, 0.0)])),
                    (keys[1], definition(vec![effect(3, 42, 0.0)])),
                    (keys[2], definition(vec![effect(6, 42, 0.0)])),
                ],
            );
            let mut s = prefix(
                input,
                vec![keys[2], enchant_key, keys[1], keys[0]],
                skill(skill_line, skillup),
            );
            let mut counts = CustomAttributeCounts::default();
            super::super::effect_flags::apply(&mut s, enchant_key, 0, &mut counts).unwrap();
            assert_eq!(counts.enchant_proc_assignments, expected_count);
            for key in &keys[..2] {
                assert_eq!(flags(&s, *key) & ENCHANT_PROC != 0, expected_count != 0);
            }
            assert_eq!(flags(&s, keys[2]) & ENCHANT_PROC, 0);
            assert!(s.get_exact(900_002, 0).is_none());
            assert_eq!(
                s.catalog.spell_item_enchantment(7).unwrap().effect_arg,
                [900_002; 3]
            );
        }
    }
}
