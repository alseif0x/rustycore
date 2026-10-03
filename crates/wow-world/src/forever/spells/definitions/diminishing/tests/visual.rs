use super::*;

fn condition(id: u32) -> UnitConditionRecord {
    UnitConditionRecord {
        id,
        flags: 0,
        variable: [0; 8],
        op: [0; 8],
        value: [0; 8],
    }
}

pub(super) fn relation(
    id: u32,
    visual: u32,
    player: u32,
    unit: u16,
    priority: i32,
    probability: f32,
) -> SpellXSpellVisualRecord {
    SpellXSpellVisualRecord {
        id,
        difficulty_id: 0,
        spell_visual_id: visual,
        probability,
        flags: 0,
        priority,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: u16::MAX,
        viewer_player_condition_id: u32::MAX,
        caster_unit_condition_id: unit,
        caster_player_condition_id: player,
        spell_id: 900_001,
    }
}

#[test]
fn null_caster_conditions_short_circuit_but_missing_nonzero_rows_remain_eligible() {
    let c = raw(SpellRecords {
        spell_x_spell_visuals: vec![
            relation(1, 11, 7, 1, 9, 1.0),
            relation(2, 22, 0, 2, 8, 1.0),
            relation(3, 33, 0, 3, 7, f32::NAN),
        ],
        unit_conditions: vec![condition(2)],
        ..Default::default()
    });
    let mut d = definition(6, [0; 4]);
    d.visuals = vec![1, 2, 3];
    let mut selections = 0;
    let chosen =
        super::super::visual::null_caster(&c, &d, &mut no_select, &mut selections).unwrap();
    assert_eq!((chosen, selections), (33, 0));
    // Nonzero absent condition is not rejected; present row 2 is rejected.
    // Viewer conditions are ignored by source GetSpellXSpellVisualId.
}

#[test]
fn first_eligible_priority_and_accepted_singleton_ignore_probability() {
    let c = raw(SpellRecords {
        spell_x_spell_visuals: vec![
            relation(1, 11, 0, 0, 9, -1.0),
            relation(2, 22, 7, 0, 9, 100.0),
            relation(3, 33, 0, 0, 8, 1.0),
        ],
        ..Default::default()
    });
    let mut d = definition(6, [0; 4]);
    d.visuals = vec![1, 2, 3];
    let result = super::super::visual::null_caster(&c, &d, &mut no_select, &mut 0).unwrap();
    assert_eq!(result, 11);
}

#[test]
fn equal_priority_candidates_retain_order_and_raw_weights_with_checked_selection() {
    let c = raw(SpellRecords {
        spell_x_spell_visuals: vec![
            relation(1, 11, 0, 0, 9, 0.0),
            relation(2, 22, 0, 0, 9, -3.0),
            relation(3, 33, 0, 0, 8, 1.0),
        ],
        ..Default::default()
    });
    let mut d = definition(6, [0; 4]);
    d.visuals = vec![2, 1, 3];
    let mut selections = 0;
    let result = super::super::visual::null_caster(
        &c,
        &d,
        &mut |weights| {
            assert_eq!(weights, [-3.0, 0.0]);
            Ok(1)
        },
        &mut selections,
    )
    .unwrap();
    assert_eq!((result, selections), (11, 1));
    assert_eq!(
        super::super::visual::null_caster(&c, &d, &mut |_| Ok(2), &mut 0),
        Err(SpellDiminishingError::InvalidSelection)
    );
}

#[test]
fn no_eligible_visual_looks_up_zero_and_missing_raw_link_is_an_error() {
    let c = raw(SpellRecords {
        spell_x_spell_visuals: vec![relation(0, 77, 0, 0, 0, 0.0), relation(1, 11, 0, 0, 9, 1.0)],
        unit_conditions: vec![condition(0)],
        ..Default::default()
    });
    let mut d = definition(6, [0; 4]);
    d.visuals = vec![1];
    assert_eq!(
        super::super::visual::null_caster(&c, &d, &mut no_select, &mut 0).unwrap(),
        77
    );
    d.visuals = vec![999];
    assert_eq!(
        super::super::visual::null_caster(&c, &d, &mut no_select, &mut 0),
        Err(SpellDiminishingError::MissingVisualRecord)
    );
}

#[test]
fn priest_queries_are_repeated_in_source_order_and_short_circuit_on_first_match() {
    use DiminishingGroup::*;
    let d = definition(6, [0x20000, 0x200000, 0x20, 0]);
    for (outputs, expected, queries) in [
        (vec![52021], Stun, 1),
        (vec![0, 39068], Incapacitate, 2),
        (vec![0, 0, 52019], Incapacitate, 3),
        (vec![0, 0, 0, 39025], Silence, 4),
        (vec![0, 0, 0, 0], None, 4),
    ] {
        let mut at = 0;
        assert_eq!(
            super::super::groups::compute(&d, 900_001, &mut || {
                let value = outputs[at];
                at += 1;
                Ok(value)
            })
            .unwrap(),
            expected
        );
        assert_eq!(at, queries);
    }
    assert_eq!(
        super::super::groups::compute(&d, 900_001, &mut || Err(
            SpellDiminishingError::SelectionUnavailable
        )),
        Err(SpellDiminishingError::SelectionUnavailable)
    );
    let mut positive = d;
    positive.negative_effects = [false; 32];
    assert_eq!(
        super::super::groups::compute(&positive, 900_001, &mut no_visual).unwrap(),
        None
    );
}

#[test]
fn unknown_condition_coverage_or_failed_draw_cannot_publish_a_completed_phase() {
    let key = (900_001, 0);
    let make = |unknown| {
        let mut d = definition(6, [0, 0, 0x20, 0]);
        d.visuals = vec![1, 2];
        let mut records = SpellRecords {
            spell_x_spell_visuals: vec![
                relation(1, 52021, 0, 0, 0, 1.0),
                relation(2, 52019, 0, 0, 0, 1.0),
            ],
            ..Default::default()
        };
        records.unknown_baseline_records[48] = unknown;
        admitted(vec![(key, d)], records, vec![key])
    };
    assert!(matches!(
        make(1).with_diminishing_info(&mut no_select),
        Err(SpellDiminishingError::IncompleteDependencies)
    ));
    assert!(matches!(
        make(0).with_diminishing_info(&mut |_| Err(SpellDiminishingError::SelectionUnavailable)),
        Err(SpellDiminishingError::SelectionUnavailable)
    ));
}
