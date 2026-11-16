use super::*;
use crate::SkillUpdateState::{Changed, Deleted, New, Unchanged};

fn has_field_bit(words: &[u32; 66], bit: usize) -> bool {
    words[bit / 32] & (1 << (bit % 32)) != 0
}

#[test]
fn equal_set_skill_marks_only_written_field_bits_without_queue_requests() {
    let fixture = Fixture::new(vec![line(10, 6, 0, 0)], vec![]);
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 10, 75, Unchanged);
    skills.clear_changes();
    let mut effects = Effects::default();

    skills
        .set_skill(update(10, 1, 10, 75), fixture.sources(), &mut effects)
        .unwrap();

    assert_eq!(effects.requests, 0);
    assert!(effects.request_positions.is_empty());
    assert!(has_field_bit(skills.field_changes(), 0));
    assert!(has_field_bit(skills.field_changes(), 301));
    assert!(has_field_bit(skills.field_changes(), 601));
    assert!(has_field_bit(skills.field_changes(), 1201));
    assert!(!has_field_bit(skills.field_changes(), 1));
    assert!(!has_field_bit(skills.field_changes(), 901));
    assert!(!has_field_bit(skills.field_changes(), 1501));
    assert!(!has_field_bit(skills.field_changes(), 1801));
}

#[test]
fn new_profession_slot_marks_all_seven_fields_and_the_profession_before_effects() {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0)], vec![]);
    let mut skills = allocated([1]);
    skills.clear_changes();
    let mut effects = Effects::default();

    skills
        .set_skill(update(10, 2, 7, 75), fixture.sources(), &mut effects)
        .unwrap();

    assert_eq!(skills.status(10), Some((1, New)));
    assert_eq!(skills.profession_lines(), &[10, 0]);
    assert_eq!(skills.profession_changes(), 1);
    let expected_bits = [0, 2, 302, 602, 902, 1202, 1502, 1802];
    for bit in expected_bits {
        assert!(has_field_bit(skills.field_changes(), bit));
    }
    let expected_mask = expected_bits.into_iter().fold([0; 66], |mut words, bit| {
        words[bit / 32] |= 1 << (bit % 32);
        words
    });
    assert_eq!(skills.field_changes(), &expected_mask);
    assert_eq!(effects.requests, 6);
    assert_eq!(effects.request_positions, [0; 6]);
    assert_eq!(effects.events.first(), Some(&Event::Enchant(10, 0, 7, 0)));
}

#[test]
fn decreases_enchant_before_field_writes_and_riding_increases_enchant_before_mount_update() {
    for (id, new, expected) in [
        (
            10,
            5,
            vec![
                Event::Enchant(10, 10, 5, 10),
                Event::Learn(10, 5, Some(Unchanged)),
                Event::Criteria(10, SkillCriteria::Raised, 5),
                Event::Criteria(10, SkillCriteria::AchieveStep, 5),
            ],
        ),
        (
            762,
            12,
            vec![
                Event::Learn(762, 12, Some(Unchanged)),
                Event::Enchant(762, 10, 12, 12),
                Event::Mount,
                Event::Criteria(762, SkillCriteria::Raised, 12),
                Event::Criteria(762, SkillCriteria::AchieveStep, 12),
            ],
        ),
        (
            762,
            10,
            vec![
                Event::Learn(762, 10, Some(Unchanged)),
                Event::Criteria(762, SkillCriteria::Raised, 10),
                Event::Criteria(762, SkillCriteria::AchieveStep, 10),
            ],
        ),
    ] {
        let fixture = Fixture::new(vec![line(id, 6, 0, 0)], rewards(&[id as u16]));
        let mut skills = allocated([id]);
        activate(&mut skills, id, 10, 75, Unchanged);
        let mut effects = Effects::default();
        skills
            .set_skill(update(id, 3, new, 100), fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(effects.events, expected);
        assert_eq!(skills.status(id), Some((0, Changed)));
        assert_eq!((skills.step(id), skills.pure_maximum(id)), (3, 100));
        assert_eq!(
            effects.request_positions,
            match (id, new) {
                (10, 5) => vec![1, 1, 1],
                (762, 12) => vec![0, 0, 0],
                (762, 10) => vec![0, 0],
                _ => unreachable!(),
            }
        );
    }
}

#[test]
fn relearning_deleted_is_changed_not_new_and_existing_dirty_states_do_not_refresh_auras() {
    for state in [Deleted, New, Changed] {
        let fixture = Fixture::new(vec![line(10, 11, 0, 0)], rewards(&[10]));
        let mut skills = allocated([10]);
        skills.status.get_mut(&10).unwrap().state = state;
        let mut effects = Effects::default();
        skills
            .set_skill(update(10, 1, 1, 75), fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(
            skills.status(10).unwrap().1,
            if state == Deleted { Changed } else { state }
        );
        assert_eq!(
            effects
                .events
                .iter()
                .filter(|event| matches!(event, Event::Aura(..)))
                .count(),
            if state == Deleted { 3 } else { 0 }
        );
        assert_eq!(
            skills.profession_lines(),
            if state == Deleted { &[10, 0] } else { &[0, 0] }
        );
    }
}

#[test]
fn classic_child_copies_parent_pure_rank_without_changing_parent_or_using_requested_cap() {
    for category in [9, 11] {
        let fixture = Fixture::new(vec![line(10, category, 0, 0), line(20, 6, 10, 4)], vec![]);
        let mut skills = allocated([10, 20]);
        activate(&mut skills, 10, 75, 75, New);
        skills.fields[0].temporary_bonus = 30;
        skills.fields[0].permanent_bonus = 40;
        let mut effects = Effects::default();
        skills
            .set_skill(update(20, 4, 1, 300), fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(
            (
                skills.step(20),
                skills.pure_value(20),
                skills.pure_maximum(20)
            ),
            (1, 75, 75)
        );
        assert_eq!((skills.pure_value(10), skills.pure_maximum(10)), (75, 75));
        assert!(
            effects
                .events
                .iter()
                .all(|event| !matches!(event, Event::Learn(10, ..) | Event::Enchant(10, ..)))
        );
    }
}

#[test]
fn normal_child_uses_first_rc_tier_and_stored_parent_gate_differs_from_absent_child() {
    // The adapter's first-RC selection is separately retained in World.
    let mut fixture = Fixture::new(
        vec![line(10, 6, 0, 0), line(20, 6, 10, 2)],
        rewards(&[10, 20]),
    );
    fixture.selected_rc.insert(10, rc(2, 10, 2));
    fixture.tiers.extend([((2, 0), 75), ((2, 1), 150)]);
    let mut skills = allocated([10, 20]);
    activate(&mut skills, 10, 99, 100, New);
    let mut effects = Effects::default();
    skills
        .set_skill(update(20, 2, 1, 10), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        (
            skills.step(10),
            skills.pure_value(10),
            skills.pure_maximum(10)
        ),
        (2, 99, 150)
    );
    assert!(matches!(
        effects.events.first(),
        Some(Event::Learn(10, 99, Some(New)))
    ));
    // Existing child skips parent activation when its step is already high.
    skills.fields[0].step = 3;
    effects.events.clear();
    skills
        .set_skill(update(20, 2, 1, 10), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.step(10), 3);
    assert!(matches!(effects.events.first(), Some(Event::Learn(20, ..))));
    // The source absent-child branch has NO current-parent-step gate, even
    // for newVal=0. Its parent effect executes before inserting this child.
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 99, 100, New);
    skills.fields[0].step = 3;
    effects.events.clear();
    skills
        .set_skill(update(20, 0, 0, 0), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.step(10), 2);
    assert!(matches!(
        effects.events.first(),
        Some(Event::Learn(10, 99, Some(New)))
    ));
    assert_eq!(skills.status(20), Some((1, New)));
}

#[test]
fn parent_sync_uses_live_child_comparison_and_preserves_pure_values_and_new_state() {
    let fixture = Fixture::new(
        vec![line(10, 11, 0, 0), line(20, 6, 10, 4), line(30, 6, 10, 4)],
        rewards(&[10, 20, 30]),
    );
    let mut skills = allocated([10, 20, 30]);
    activate(&mut skills, 10, 10, 75, New);
    activate(&mut skills, 20, 10, 75, New);
    activate(&mut skills, 30, 10, 75, New);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 2, 20, 150), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects
            .events
            .iter()
            .filter_map(|event| if let Event::Learn(skill, ..) = event {
                Some(*skill)
            } else {
                None
            })
            .collect::<Vec<_>>(),
        [10, 20, 30]
    );
    for id in [10, 20, 30] {
        assert_eq!(
            (
                skills.step(id),
                skills.pure_value(id),
                skills.pure_maximum(id)
            ),
            (2, 20, 150)
        );
    }
    effects.events.clear();
    skills
        .set_skill(update(10, 2, 20, 150), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects
            .events
            .iter()
            .filter_map(|event| if let Event::Learn(skill, ..) = event {
                Some(*skill)
            } else {
                None
            })
            .collect::<Vec<_>>(),
        [10]
    );
}

#[test]
fn reward_reentry_uses_same_fields_and_outer_effects_keep_captured_old_rank_without_double_refresh()
{
    let fixture = Fixture::new(vec![line(10, 11, 0, 0)], rewards(&[10]));
    let sources = fixture.sources();
    let mut skills = allocated([10]);
    let mut effects = Effects {
        reenter: Some((sources, update(10, 3, 20, 150))),
        ..Default::default()
    };
    skills
        .set_skill(update(10, 1, 7, 75), sources, &mut effects)
        .unwrap();
    assert_eq!(
        (
            skills.step(10),
            skills.pure_value(10),
            skills.pure_maximum(10)
        ),
        (3, 20, 150)
    );
    // Inner operation observed rank 7, then changed it to 20. Its state update
    // marks Changed (old != 0), so the outer one does not re-activate or refresh.
    assert_eq!(skills.status(10), Some((0, Changed)));
    assert_eq!(
        effects.events,
        [
            Event::Learn(10, 7, Some(Unchanged)),
            Event::Learn(10, 20, Some(Unchanged)),
            Event::Enchant(10, 7, 20, 20),
            Event::Criteria(10, SkillCriteria::Raised, 20),
            Event::Criteria(10, SkillCriteria::AchieveStep, 20),
            Event::Enchant(10, 0, 7, 20),
            Event::Criteria(10, SkillCriteria::Raised, 20),
            Event::Criteria(10, SkillCriteria::AchieveStep, 20),
        ]
    );
    assert_eq!(skills.profession_lines(), &[0, 0]);
}

#[test]
fn profession_projection_uses_signed_id_bits_first_empty_slot_and_no_duplicate_filter() {
    let fixture = Fixture::new(
        vec![
            line(10, 11, 0, 0),
            line(u32::MAX, 11, 0, 0),
            line(20, 9, 0, 0),
            line(30, 11, 10, 0),
        ],
        vec![],
    );
    let mut skills = allocated([10, u32::MAX, 20, 30]);
    assert_eq!(skills.profession_slot(0), Some(0));
    skills.assign_profession(&fixture.birth, 20, || {}); // Secondary is not profession.
    skills.assign_profession(&fixture.birth, 30, || {}); // Child is not root profession.
    assert_eq!(skills.profession_lines(), &[0, 0]);
    skills.assign_profession(&fixture.birth, u32::MAX, || {});
    assert_eq!(skills.profession_lines(), &[-1, 0]);
    assert_eq!(skills.profession_slot(u32::MAX), Some(0));
    skills.assign_profession(&fixture.birth, u32::MAX, || {}); // Source comment != code.
    assert_eq!(skills.profession_lines(), &[-1, -1]);
    skills.assign_profession(&fixture.birth, 10, || {});
    assert_eq!(skills.profession_slot(10), None);
    assert_eq!(skills.profession_slot(0), None);
}
