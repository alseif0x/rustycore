use super::*;

#[test]
fn cone_width_uses_double_epsilon_and_leaves_nan_and_infinity_unequal() {
    let key = (900_001, 0);
    for (width, expected) in [
        (0.0, false),
        (-0.0, false),
        (0.0000004, false),
        (0.0000006, true),
        (0.000005, true),
        (f32::NAN, true),
        (f32::INFINITY, true),
    ] {
        let mut d = definition(vec![]);
        d.fields.width = width;
        let mut s = fresh(Default::default(), [(key, d)]);
        super::super::tail::local(&mut s, key, true, &mut Default::default());
        assert_eq!(flags(&s, key) & CONE_LINE != 0, expected);
        assert_eq!(flags(&s, key) & TALENT, TALENT);
    }
}

fn visual_relation() -> SpellXSpellVisualRecord {
    SpellXSpellVisualRecord {
        id: 1,
        difficulty_id: 0,
        spell_visual_id: 2,
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id: 0,
        spell_id: 900_001,
    }
}

#[test]
fn ammo_uses_final_missile_set_links_speed_and_only_types_six_or_seven() {
    let key = (900_001, 0);
    for (speed, r#type, expected) in [
        (0.0, 6, false),
        (-1.0, 7, false),
        (f32::NAN, 6, false),
        (1.0, 5, false),
        (1.0, 6, true),
        (1.0, 7, true),
        (f32::INFINITY, 6, true),
    ] {
        let mut visual = dependency_rows::spell_visual(2, 6, b"");
        visual.spell_visual_missile_set_id = 3;
        let mut missile = dependency_rows::spell_visual_missile(4, 6, b"");
        missile.spell_visual_missile_set_id = 3;
        missile.spell_visual_effect_name_id = 5;
        let mut name = dependency_rows::spell_visual_effect_name(5, 6, b"");
        name.r#type = r#type;
        let mut d = definition(vec![]);
        d.fields.speed = speed;
        d.visuals = vec![999, 1];
        let mut s = fresh(
            SpellRecords {
                spell_x_spell_visuals: vec![visual_relation()],
                spell_visuals: vec![visual],
                spell_visual_missiles: vec![missile],
                spell_visual_effect_names: vec![name],
                ..Default::default()
            },
            [(key, d)],
        );
        let mut counts = CustomAttributeCounts::default();
        super::super::tail::ammo_and_leave_world(&mut s, key, &mut counts);
        assert_eq!(flags(&s, key) & NEEDS_AMMO != 0, expected);
        assert_eq!(counts.ammo_assignments, usize::from(expected));
    }
}

#[test]
fn leave_world_uses_word_zero_not_word_one_and_missing_visual_is_not_ammo() {
    let key = (900_001, 0);
    for word in 0..2 {
        let mut d = definition(vec![]);
        d.fields.aura_interrupt_flags[word] = 0x80000;
        d.fields.speed = 1.0;
        d.visuals = vec![1];
        let mut s = fresh(
            SpellRecords {
                spell_x_spell_visuals: vec![visual_relation()],
                ..Default::default()
            },
            [(key, d)],
        );
        super::super::tail::ammo_and_leave_world(&mut s, key, &mut Default::default());
        assert_eq!(flags(&s, key) & CANNOT_SAVE != 0, word == 0);
        assert_eq!(flags(&s, key) & NEEDS_AMMO, 0);
    }
}

#[test]
fn final_pass_clears_crit_and_marks_every_liquid_spell_difficulty_without_binary_propagation() {
    let a = (900_001, -1);
    let b = (900_002, 0);
    let c = (900_002, i16::MAX);
    let mut trigger = effect(6, 23, 0.0);
    trigger.trigger_spell = b.0;
    let mut parent = definition(vec![trigger]);
    parent.custom_attributes = CAN_CRIT;
    parent.fields.attributes[2] = 0x2000_0000;
    let mut binary = definition(vec![]);
    binary.custom_attributes = BINARY;
    let mut liquid = dependency_rows::liquid_type(7, 6, b"");
    liquid.spell_id = b.0;
    let mut duplicate = dependency_rows::liquid_type(8, 6, b"");
    duplicate.spell_id = b.0;
    let mut empty = dependency_rows::liquid_type(9, 6, b"");
    empty.spell_id = 0;
    let mut s = prefix(
        fresh(
            SpellRecords {
                liquid_types: vec![liquid, duplicate, empty],
                ..Default::default()
            },
            [(a, parent), (b, binary), (c, definition(vec![]))],
        ),
        vec![a, c, b],
        Default::default(),
    );
    let mut counts = CustomAttributeCounts::default();
    super::super::tail::second_pass_and_liquids(&mut s, &mut counts).unwrap();
    assert_eq!(flags(&s, a) & (BINARY | CAN_CRIT | CANNOT_SAVE), 0);
    assert_eq!(flags(&s, b), BINARY | CANNOT_SAVE);
    assert_eq!(flags(&s, c), CANNOT_SAVE);
    assert_eq!((counts.crit_clears, counts.liquid_assignments), (1, 4));
}

#[test]
fn final_talent_membership_marks_all_existing_difficulties_not_rank_or_override_ids() {
    let a = (900_001, -1);
    let b = (900_001, 7);
    let c = (900_002, 0);
    let mut talent = dependency_rows::talent(1, 6, b"");
    talent.spell_id = a.0;
    talent.overrides_spell_id = c.0;
    let s = admitted(
        SpellRecords {
            talents: vec![talent],
            ..Default::default()
        },
        [
            (a, definition(vec![])),
            (b, definition(vec![])),
            (c, definition(vec![])),
        ],
        vec![c, a, b],
    )
    .with_custom_attributes(&items(), &mut no_draw)
    .unwrap();
    assert_eq!(flags(&s, a) & TALENT, TALENT);
    assert_eq!(flags(&s, b) & TALENT, TALENT);
    assert_eq!(flags(&s, c) & TALENT, 0);
}

fn difficulty(id: u32, fallback: i16) -> DifficultyRecord {
    DifficultyRecord {
        id,
        name: SpellText::default(),
        instance_type: 0,
        order_index: 0,
        old_enum_value: 0,
        fallback_difficulty_id: fallback,
        min_players: 0,
        max_players: 0,
        flags: 0,
        item_context: 0,
        toggle_difficulty_id: 0,
        group_size_health_curve_id: 0,
        group_size_dmg_curve_id: 0,
        group_size_spell_points_curve_id: 0,
        unknown1105: 0,
    }
}

#[test]
fn second_binary_branch_uses_difficulty_none_and_keeps_the_source_not_binary_condition() {
    let key = (900_001, 7);
    for (kind, initial_binary, should_error) in
        [(6, false, true), (6, true, false), (3, false, false)]
    {
        let mut e = effect(kind, 23, 0.0);
        e.trigger_spell = 900_002;
        let mut d = definition(vec![e]);
        d.custom_attributes = if initial_binary { BINARY } else { 0 };
        let mut s = prefix(
            fresh(
                SpellRecords {
                    difficulties: vec![difficulty(0, 1), difficulty(1, 0)],
                    ..Default::default()
                },
                [(key, d)],
            ),
            vec![key],
            Default::default(),
        );
        let result = super::super::tail::second_pass_and_liquids(&mut s, &mut Default::default());
        if should_error {
            assert_eq!(
                result,
                Err(SpellCustomAttributeError::Value(
                    SpellValueError::DefinitionLookup(
                        super::super::super::SpellDefinitionError::DifficultyCycle
                    )
                ))
            );
        } else {
            result.unwrap();
            assert_eq!(flags(&s, key) & BINARY != 0, initial_binary);
        }
    }
}
