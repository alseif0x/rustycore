use super::{activate, allocated};
use crate::{SkillStatusOrderError, SkillUpdateState};

#[test]
fn capture_orders_dirty_physical_keys_and_keeps_raw_deleted_and_aliased_rows() {
    let mut skills = allocated([0, 65_536, 1, 65_537, 9]);
    activate(&mut skills, 65_536, 23, 900, SkillUpdateState::Deleted);
    activate(&mut skills, 0, 3, 45, SkillUpdateState::Changed);
    activate(&mut skills, 1, 8, 75, SkillUpdateState::Changed);
    activate(&mut skills, 65_537, 9, 100, SkillUpdateState::Changed);
    skills.fields[3].line = 77;

    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [0, 65_536, 1, 65_537, 9]);
            assert_eq!(count, 5);
            Ok::<_, ()>(vec![65_537, 9, 65_536, 1, 0])
        })
        .unwrap();

    assert_eq!(
        batch
            .rows()
            .iter()
            .map(|row| row.skill())
            .collect::<Vec<_>>(),
        [65_537, 65_536, 1, 0]
    );
    let wide_changed = &batch.rows()[0];
    assert_eq!(wide_changed.state(), SkillUpdateState::Changed);
    assert_eq!(wide_changed.value(), 9);
    assert_eq!(wide_changed.maximum(), 100);
    assert_eq!(wide_changed.profession_slot(), -1);
    assert_eq!(batch.rows()[0].line, 77);

    let deleted = &batch.rows()[1];
    assert_eq!(deleted.state(), SkillUpdateState::Deleted);
    assert_eq!(deleted.value(), 23);
    assert_eq!(deleted.maximum(), 900);
    assert_eq!(deleted.line, 0);
    assert_eq!(skills.pure_value(65_536), 0);
    assert_eq!(skills.status(65_536).unwrap().1, SkillUpdateState::Deleted);

    // Distinct full keys with identical low 16 bits and zero remain separate rows.
    assert_eq!(
        batch.rows()[2].skill() as u16,
        batch.rows()[0].skill() as u16
    );
    assert_eq!(
        batch.rows()[3].skill() as u16,
        batch.rows()[1].skill() as u16
    );
    assert_eq!(batch.rows()[3].profession_slot(), 0); // Full-key source search includes zero.
}

#[test]
fn provider_runs_for_empty_and_clean_owners_and_capture_errors_do_not_mutate() {
    let empty = allocated([]);
    let mut empty_called = false;
    let empty_batch = empty
        .capture_save(|history, count| {
            empty_called = true;
            assert!(history.is_empty());
            assert_eq!(count, 0);
            Ok::<_, ()>(vec![])
        })
        .unwrap();
    assert!(empty_called);
    assert!(empty_batch.rows().is_empty());

    let mut skills = allocated([8, 9]);
    let row = *skills.fields;
    let history = skills.insertions.clone();
    let statuses = [skills.status(8), skills.status(9)];
    let clean = skills
        .capture_save(|keys, count| {
            assert_eq!(keys, [8, 9]);
            assert_eq!(count, 2);
            Ok::<_, ()>(vec![9, 8])
        })
        .unwrap();
    assert!(clean.rows().is_empty());

    assert_eq!(
        skills
            .capture_save(|_, _| Err::<Vec<u32>, _>("order unavailable"))
            .err(),
        Some(SkillStatusOrderError::Source("order unavailable"))
    );
    assert_eq!(
        skills.capture_save(|_, _| Ok::<_, ()>(vec![8, 8])).err(),
        Some(SkillStatusOrderError::InvalidKeySet)
    );
    drop(clean);
    assert!(*skills.fields == row);
    assert_eq!(skills.insertions, history);
    assert_eq!([skills.status(8), skills.status(9)], statuses);
}
