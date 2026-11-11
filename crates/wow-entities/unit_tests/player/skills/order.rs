//! Status-order admission with existing SetSkill facts/effects, not a native oracle.
use super::*;
use crate::{
    SkillStatusOrderError,
    SkillUpdateState::{Changed, Deleted, New, Unchanged},
};
use std::collections::HashMap;

#[test]
fn duplicate_initialization_keeps_first_status_slot_and_logs_only_successful_keys() {
    let skills = PlayerSkills::initialize([10, 10, 65536, 0]);
    assert_eq!(skills.insertions, [10, 65536, 0]);
    assert_eq!(skills.status(10), Some((0, Unchanged)));
    assert_eq!(skills.status(65536), Some((2, Unchanged)));
    assert_eq!(skills.status(0), Some((3, Unchanged)));
    assert_eq!(skills.fields()[0].line(), 10);
    assert_eq!(skills.fields()[1].line(), 10);
    assert_eq!(skills.fields()[2].line(), 0);
    for field in &skills.fields()[..4] {
        assert_eq!(field.starting_rank(), 1);
        assert_eq!(field.rank(), 0);
    }
    assert!(!skills.has_skill(10) && !skills.has_skill(65536));
    let limited = PlayerSkills::initialize(std::iter::repeat_n(9, 300).chain([20]));
    assert_eq!(limited.insertions, [9]);
    assert_eq!(limited.status(9), Some((0, Unchanged)));
    assert!(limited.status(20).is_none());
    assert_eq!(limited.fields()[299].line(), 9);
}

#[test]
fn provider_order_borrows_canonical_fields_for_zero_deleted_inactive_and_wide_keys() {
    let fixture = Fixture::new(vec![line(10, 6, 0, 0)], vec![]);
    let mut skills = allocated([0, 10, 65536, 10, 20]);
    activate(&mut skills, 10, 7, 75, Unchanged);
    activate(&mut skills, 65536, 3, 9, Changed);
    skills
        .set_skill(
            update(10, 0, 0, 0),
            fixture.sources(),
            &mut Effects::default(),
        )
        .unwrap();
    let entries = skills
        .source_statuses(|history, count| {
            assert_eq!(history, [0, 10, 65536, 20]);
            assert_eq!(count, 4); // Physical map size, not five written fields or 300 slots.
            Ok::<_, ()>(vec![65536, 20, 0, 10])
        })
        .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.skill())
            .collect::<Vec<_>>(),
        [65536, 20, 0, 10]
    );
    for (entry, (slot, state)) in
        entries
            .iter()
            .zip([(2, Changed), (4, Unchanged), (0, Unchanged), (1, Deleted)])
    {
        assert_eq!(entry.slot(), slot);
        assert_eq!(entry.state(), state);
        assert!(std::ptr::eq(entry.fields(), &skills.fields()[slot]));
    }
    assert_eq!(entries[0].fields().line(), 0);
    assert_eq!(entries[0].fields().rank(), 3);
    assert_eq!(entries[3].fields().rank(), 0);
}

#[test]
fn empty_status_map_still_calls_provider_and_accepts_only_empty_output() {
    let skills = allocated([]);
    let mut called = false;
    let entries = skills
        .source_statuses(|history, count| {
            called = true;
            assert!(history.is_empty());
            assert_eq!(count, 0);
            Ok::<_, ()>(vec![])
        })
        .unwrap();
    assert!(called && entries.is_empty());
    assert_eq!(
        skills.source_statuses(|_, _| Ok::<_, ()>(vec![0])).err(),
        Some(SkillStatusOrderError::InvalidKeySet)
    );
    assert_eq!(
        skills.source_statuses(|_, _| Err::<Vec<u32>, _>(17)).err(),
        Some(SkillStatusOrderError::Source(17))
    );
}

#[test]
fn malformed_provider_and_source_failure_preserve_fields_statuses_and_insertions() {
    let skills = allocated([10, 20]);
    let fields = *skills.fields;
    let statuses = skills
        .status
        .iter()
        .map(|(&key, status)| (key, (status.slot, status.state)))
        .collect::<HashMap<_, _>>();
    let insertions = skills.insertions.clone();
    for keys in [
        vec![],
        vec![10],
        vec![10, 10],
        vec![20, 20],
        vec![10, 30],
        vec![10, 20, 30],
    ] {
        assert_eq!(
            skills
                .source_statuses(|history, count| {
                    assert_eq!(history, [10, 20]);
                    assert_eq!(count, 2);
                    Ok::<_, ()>(keys)
                })
                .err(),
            Some(SkillStatusOrderError::InvalidKeySet)
        );
    }
    assert_eq!(
        skills
            .source_statuses(|_, _| Err::<Vec<u32>, _>("native unavailable"))
            .err(),
        Some(SkillStatusOrderError::Source("native unavailable"))
    );
    assert!(skills.fields() == &fields);
    assert!(
        skills
            .status
            .iter()
            .all(|(&key, status)| statuses.get(&key) == Some(&(status.slot, status.state)))
    );
    assert_eq!(skills.status.len(), statuses.len());
    assert_eq!(skills.insertions, insertions);
}

#[test]
fn captured_slot_records_child_before_parent_and_retains_both_aliased_statuses() {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0), line(20, 6, 10, 4)], vec![]);
    let mut skills = allocated([1]);
    skills
        .set_skill(
            update(10, 2, 7, 75),
            fixture.sources(),
            &mut Effects::default(),
        )
        .unwrap();
    assert_eq!(skills.insertions, [1, 20, 10]);
    assert_eq!(skills.status(20), Some((1, New)));
    assert_eq!(skills.status(10), Some((1, New)));
    let entries = skills
        .source_statuses(|history, count| {
            assert_eq!(history, [1, 20, 10]);
            assert_eq!(count, 3);
            Ok::<_, ()>(vec![20, 10, 1])
        })
        .unwrap();
    assert_eq!((entries[0].skill(), entries[1].skill()), (20, 10));
    assert!(std::ptr::eq(entries[0].fields(), entries[1].fields()));
    assert_eq!(entries[0].fields().line(), 10);
    assert_eq!(entries[0].fields().rank(), 7);
}

#[test]
fn payload_bonus_noop_and_deactivation_retain_status_membership_without_new_events() {
    let fixture = Fixture::new(vec![line(10, 6, 0, 0), line(20, 6, 0, 0)], vec![]);
    let mut skills = allocated([10, 20]);
    activate(&mut skills, 10, 7, 75, Unchanged);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 2, 9, 100), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.status(10), Some((0, Changed)));
    skills
        .modify_bonus(fixture.sources(), 10, 3, false)
        .unwrap();
    skills.modify_bonus(fixture.sources(), 10, 4, true).unwrap();
    assert_eq!(
        (skills.temporary_bonus(10), skills.permanent_bonus(10)),
        (3, 4)
    );
    skills
        .set_skill(update(10, 2, 9, 100), fixture.sources(), &mut effects)
        .unwrap();
    skills
        .set_skill(update(10, 0, 0, 0), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.status(10), Some((0, Deleted)));
    skills
        .set_skill(update(20, 1, 1, 75), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.status(20), Some((1, New)));
    skills
        .set_skill(update(20, 0, 0, 0), fixture.sources(), &mut effects)
        .unwrap();
    skills
        .set_skill(update(20, 0, 0, 0), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.status(20), Some((1, Unchanged)));
    assert_eq!(skills.insertions, [10, 20]);
    let entries = skills
        .source_statuses(|history, count| {
            assert_eq!(history, [10, 20]);
            assert_eq!(count, 2);
            Ok::<_, ()>(vec![10, 20])
        })
        .unwrap();
    assert_eq!(entries[0].state(), Deleted);
    assert_eq!(entries[1].state(), Unchanged);
}

#[test]
fn failure_before_absent_status_insert_retains_field_prefix_but_no_new_membership() {
    let fixture = Fixture::new(vec![line(20, 6, 0, 0)], vec![]);
    let mut skills = allocated([1]);
    let mut effects = Effects {
        fail_at: Some(0),
        ..Default::default()
    };
    assert_eq!(
        skills.set_skill(update(20, 1, 3, 75), fixture.sources(), &mut effects),
        Err(SkillSetError::Effect("unavailable required effect"))
    );
    assert_eq!(effects.events, [Event::Enchant(20, 0, 3, 0)]);
    assert_eq!(
        (skills.fields()[1].line(), skills.fields()[1].rank()),
        (20, 3)
    );
    assert!(skills.status(20).is_none());
    assert_eq!(skills.insertions, [1]);
    let entries = skills
        .source_statuses(|history, count| {
            assert_eq!(history, [1]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![1])
        })
        .unwrap();
    assert_eq!(entries[0].skill(), 1);
    // Completed-prefix observation only: this failed construction is not saveable.
}

#[test]
fn occupied_insert_preserves_old_slot_state_and_records_no_event() {
    let mut skills = allocated([7]);
    // Existing effects reenter through rewards after insertion, not through the
    // earlier enchantment callback. Exercise the same occupied helper directly.
    assert!(!skills.try_insert_status(
        7,
        crate::player::skills::Status {
            slot: 299,
            state: Deleted
        }
    ));
    assert_eq!(skills.status(7), Some((0, Unchanged)));
    assert_eq!(skills.insertions, [7]);
}
