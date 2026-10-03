use super::*;
use crate::forever::player::SkillUpdateState::{Changed, Deleted, New, Unchanged};
use std::sync::Arc;
use wow_persistence::forever::creation::SkillTierRow;

#[test]
fn source_preallocation_activation_orders_rewards_enchants_criteria_status_then_bonus_auras() {
    let fixture = Fixture::with_rc(
        vec![line(10, 11, 0, 0)],
        rewards(&[10]),
        vec![rc(1, 10, 0)],
        vec![],
        vec![1],
    );
    let initial = fixture.world.initial_skill_fields(1, 1, 1).unwrap();
    let mut skills = initial.initialize_player_skills();
    let mut effects = Effects {
        bonus: Some((&fixture.birth, 5)),
        ..Default::default()
    };
    assert_eq!(
        skills.set_skill(update(10, 2, 7, 75), &fixture.sources(), &mut effects),
        Ok(SkillSetOutcome::Finished)
    );
    assert_eq!(
        effects.events,
        [
            Event::Learn(10, 7, Some(Unchanged)),
            Event::Enchant(10, 0, 7, 7),
            Event::Criteria(10, SkillCriteria::Raised, 7),
            Event::Criteria(10, SkillCriteria::AchieveStep, 7),
            Event::Aura(10, 30, Some(New)),
            Event::Aura(10, 400, Some(New)),
            Event::Aura(10, 98, Some(New)),
        ]
    );
    assert_eq!(skills.status(10), Some((0, New)));
    assert_eq!(skills.profession_lines(), &[10, 0]);
    assert_eq!(
        (
            skills.pure_value(10),
            skills.value(10),
            skills.step(10),
            skills.pure_maximum(10)
        ),
        (7, 12, 2, 75)
    );
    assert!(std::ptr::eq(
        fixture.world.skill_birth_catalog().unwrap(),
        fixture.birth.as_ref()
    ));
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
            .set_skill(update(id, 3, new, 100), &fixture.sources(), &mut effects)
            .unwrap();
        assert_eq!(effects.events, expected);
        assert_eq!(skills.status(id), Some((0, Changed)));
        assert_eq!((skills.step(id), skills.pure_maximum(id)), (3, 100));
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
            .set_skill(update(10, 1, 1, 75), &fixture.sources(), &mut effects)
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
            .set_skill(update(20, 4, 1, 300), &fixture.sources(), &mut effects)
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
    let mut first = [0; 16];
    first[0] = 75;
    first[1] = 150;
    let mut other = [0; 16];
    other[0] = 999;
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 10, 2)],
        rewards(&[10, 20]),
        vec![rc(1, 10, 1), rc(2, 10, 2)],
        vec![
            SkillTierRow {
                id: 1,
                values: other,
            },
            SkillTierRow {
                id: 2,
                values: first,
            },
        ],
        vec![2, 1],
    );
    let mut skills = allocated([10, 20]);
    activate(&mut skills, 10, 99, 100, New);
    let mut effects = Effects::default();
    skills
        .set_skill(update(20, 2, 1, 10), &fixture.sources(), &mut effects)
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
        .set_skill(update(20, 2, 1, 10), &fixture.sources(), &mut effects)
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
        .set_skill(update(20, 0, 0, 0), &fixture.sources(), &mut effects)
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
        .set_skill(update(10, 2, 20, 150), &fixture.sources(), &mut effects)
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
        .set_skill(update(10, 2, 20, 150), &fixture.sources(), &mut effects)
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
    assert!(Arc::strong_count(&fixture.birth) >= 3);
}

#[test]
fn reward_reentry_uses_same_fields_and_outer_effects_keep_captured_old_rank_without_double_refresh()
{
    let fixture = Fixture::new(vec![line(10, 11, 0, 0)], rewards(&[10]));
    let sources = fixture.sources();
    let mut skills = allocated([10]);
    let mut effects = Effects {
        reenter: Some((&sources, update(10, 3, 20, 150))),
        ..Default::default()
    };
    skills
        .set_skill(update(10, 1, 7, 75), &sources, &mut effects)
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
    skills.assign_profession(&fixture.birth, 20); // Secondary is not profession.
    skills.assign_profession(&fixture.birth, 30); // Child is not root profession.
    assert_eq!(skills.profession_lines(), &[0, 0]);
    skills.assign_profession(&fixture.birth, u32::MAX);
    assert_eq!(skills.profession_lines(), &[-1, 0]);
    assert_eq!(skills.profession_slot(u32::MAX), Some(0));
    skills.assign_profession(&fixture.birth, u32::MAX); // Source comment != code.
    assert_eq!(skills.profession_lines(), &[-1, -1]);
    skills.assign_profession(&fixture.birth, 10);
    assert_eq!(skills.profession_slot(10), None);
    assert_eq!(skills.profession_slot(0), None);
}
