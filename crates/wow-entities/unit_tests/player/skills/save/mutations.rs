use super::{Effects, Fixture, activate, allocated, line, update};
use crate::SkillUpdateState;

fn simple_fixture() -> Fixture {
    Fixture::new(
        vec![
            line(10, 6, 0, 0),
            line(20, 6, 0, 0),
            line(40, 11, 0, 0),
            line(41, 11, 0, 0),
        ],
        vec![],
    )
}

#[test]
fn set_skill_edit_and_cancel_during_new_insert_ack_reconcile_to_changed_or_deleted() {
    let sources = simple_fixture();
    let mut edited = allocated([10]);
    let mut edit_effects = Effects::default();
    edited
        .set_skill(update(10, 1, 5, 75), &sources, &mut edit_effects)
        .unwrap();
    let batch = edited
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    assert_eq!(batch.rows()[0].step, 1);
    edited
        .set_skill(update(10, 2, 5, 75), &sources, &mut edit_effects)
        .unwrap();
    assert_eq!(edited.status(10), Some((0, SkillUpdateState::New)));
    edited.acknowledge_save(batch);
    assert_eq!(edited.status(10), Some((0, SkillUpdateState::Changed)));
    assert_eq!(
        (
            edited.fields()[0].step(),
            edited.fields()[0].rank(),
            edited.fields()[0].maximum()
        ),
        (2, 5, 75)
    );
    assert!(edit_effects.events.len() > 0);

    let mut cancelled = allocated([10]);
    let mut cancel_effects = Effects::default();
    cancelled
        .set_skill(update(10, 1, 5, 75), &sources, &mut cancel_effects)
        .unwrap();
    let batch = cancelled
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    cancelled
        .set_skill(update(10, 0, 0, 0), &sources, &mut cancel_effects)
        .unwrap();
    assert_eq!(cancelled.status(10), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(cancelled.fields()[0].rank(), 0);
    cancelled.acknowledge_save(batch);
    assert_eq!(cancelled.status(10), Some((0, SkillUpdateState::Deleted)));
    assert!(cancel_effects.events.len() > 0);
}

#[test]
fn reactivation_while_delete_is_pending_becomes_new_after_ack() {
    let sources = simple_fixture();
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 5, 75, SkillUpdateState::Unchanged);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 0, 0, 0), &sources, &mut effects)
        .unwrap();
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Deleted)));

    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    skills
        .set_skill(update(10, 1, 5, 75), &sources, &mut effects)
        .unwrap();
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Changed)));
    let fields_before_ack = *skills.fields;

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(10), Some((0, SkillUpdateState::New)));
    assert!(*skills.fields == fields_before_ack);
    assert!(effects.events.len() > 0);
}

#[test]
fn bonus_and_starting_rank_changes_do_not_block_matching_ack() {
    let sources = simple_fixture();
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 5, 75, SkillUpdateState::Unchanged);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 1, 6, 75), &sources, &mut effects)
        .unwrap();
    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();

    skills.modify_bonus(&sources, 10, 4, false, || {}).unwrap();
    let slot = skills.status(10).unwrap().0 as usize;
    skills.fields[slot].starting_rank = 7;
    assert_eq!(skills.fields()[slot].temporary_bonus(), 4);
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Changed)));

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(skills.fields()[slot].starting_rank(), 7);
    assert_eq!(skills.fields()[slot].temporary_bonus(), 4);
}

#[test]
fn changed_profession_selection_keeps_the_same_dirty_status_after_ack() {
    let sources = simple_fixture();
    let mut skills = allocated([40, 41]);
    activate(&mut skills, 40, 5, 75, SkillUpdateState::Unchanged);
    let mut effects = Effects::default();
    skills
        .set_skill(update(40, 2, 6, 80), &sources, &mut effects)
        .unwrap();
    skills.professions[0] = 40;
    assert_eq!(skills.profession_slot(40), Some(0));

    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [40, 41]);
            assert_eq!(count, 2);
            Ok::<_, ()>(vec![40, 41])
        })
        .unwrap();
    assert_eq!(batch.rows()[0].profession_slot(), 0);
    skills.professions[0] = 41;
    assert_eq!(skills.status(40), Some((0, SkillUpdateState::Changed)));
    assert_eq!(skills.profession_slot(40), None);

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(40), Some((0, SkillUpdateState::Changed)));
}

#[test]
fn changed_row_deleted_while_update_is_pending_stays_deleted() {
    let sources = simple_fixture();
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 5, 75, SkillUpdateState::Unchanged);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 1, 6, 75), &sources, &mut effects)
        .unwrap();
    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    skills
        .set_skill(update(10, 0, 0, 0), &sources, &mut effects)
        .unwrap();
    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Deleted)));

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Deleted)));
}

#[test]
fn moved_status_slot_is_skipped_without_clearing_the_target_or_neighbor() {
    let sources = simple_fixture();
    let mut skills = allocated([10, 20]);
    activate(&mut skills, 10, 5, 75, SkillUpdateState::Changed);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 1, 6, 75), &sources, &mut effects)
        .unwrap();
    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10, 20]);
            assert_eq!(count, 2);
            Ok::<_, ()>(vec![10, 20])
        })
        .unwrap();
    skills.status.get_mut(&10).unwrap().slot = 1;

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(10), Some((1, SkillUpdateState::Changed)));
    assert_eq!(skills.status(20), Some((1, SkillUpdateState::Unchanged)));
}
