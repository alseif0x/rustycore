use super::{Effects, Event, Fixture, activate, allocated, line, update};
use crate::{PlayerSkills, SkillUpdateState};

fn fixture() -> Fixture {
    Fixture::new(
        vec![line(10, 6, 0, 0), line(20, 6, 0, 0), line(30, 6, 0, 0)],
        vec![],
    )
}

fn three_dirty_states(skills: &mut PlayerSkills, sources: &Fixture, effects: &mut Effects<'_>) {
    skills
        .set_skill(update(10, 1, 5, 75), sources, effects)
        .unwrap();
    activate(skills, 20, 4, 75, SkillUpdateState::Unchanged);
    skills
        .set_skill(update(20, 1, 6, 75), sources, effects)
        .unwrap();
    activate(skills, 30, 7, 75, SkillUpdateState::Unchanged);
    skills
        .set_skill(update(30, 0, 0, 0), sources, effects)
        .unwrap();
    let deleted_slot = skills.status(30).unwrap().0 as usize;
    skills.fields[deleted_slot].rank = 17;
    skills.fields[deleted_slot].maximum = 99;
}

#[test]
fn matching_ack_cleans_all_dirty_states_without_erasing_or_rewriting_payload() {
    let sources = fixture();
    let mut skills = allocated([10, 20, 30]);
    let mut effects = Effects::default();
    three_dirty_states(&mut skills, &sources, &mut effects);
    let before_fields = *skills.fields;
    let before_professions = *skills.profession_lines();
    let before_insertions = skills.insertions.clone();
    let before_event_count = effects.events.len();

    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10, 20, 30]);
            assert_eq!(count, 3);
            Ok::<_, ()>(vec![30, 20, 10])
        })
        .unwrap();
    assert_eq!(
        batch
            .rows()
            .iter()
            .map(|row| row.skill())
            .collect::<Vec<_>>(),
        [30, 20, 10]
    );
    assert_eq!(batch.rows()[0].state(), SkillUpdateState::Deleted);
    assert_eq!(batch.rows()[0].value(), 17);
    assert_eq!(batch.rows()[0].maximum(), 99);
    assert_eq!(batch.rows()[1].state(), SkillUpdateState::Changed);
    assert_eq!(batch.rows()[2].state(), SkillUpdateState::New);
    assert!(!skills.has_skill(30));

    skills.acknowledge_save(batch);

    for skill in [10, 20, 30] {
        assert_eq!(skills.status(skill).unwrap().1, SkillUpdateState::Unchanged);
    }
    assert!(*skills.fields == before_fields);
    assert_eq!(*skills.profession_lines(), before_professions);
    assert_eq!(skills.insertions, before_insertions);
    assert_eq!(skills.status.len(), 3);
    assert!(skills.has_skill(30));
    assert_eq!(effects.events.len(), before_event_count);
    assert!(
        effects
            .events
            .iter()
            .any(|event| matches!(event, Event::Enchant(..)))
    );
}

#[test]
fn dropping_a_batch_does_not_acknowledge_or_clear_a_dirty_status() {
    let sources = fixture();
    let mut skills = allocated([10]);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 1, 5, 75), &sources, &mut effects)
        .unwrap();

    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    assert_eq!(batch.rows().len(), 1);
    drop(batch);

    assert_eq!(skills.status(10), Some((0, SkillUpdateState::New)));
}

#[test]
fn unchanged_current_status_after_delete_capture_remains_unchanged() {
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 5, 75, SkillUpdateState::Deleted);
    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    let slot = skills.status(10).unwrap().0 as usize;
    skills.fields[slot].maximum = 76;
    skills.status.get_mut(&10).unwrap().state = SkillUpdateState::Unchanged;

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(skills.fields[slot].maximum(), 76);
}

#[test]
fn changed_deleted_payload_after_delete_capture_remains_deleted() {
    let mut skills = allocated([10]);
    activate(&mut skills, 10, 5, 75, SkillUpdateState::Deleted);
    let batch = skills
        .capture_save(|history, count| {
            assert_eq!(history, [10]);
            assert_eq!(count, 1);
            Ok::<_, ()>(vec![10])
        })
        .unwrap();
    let slot = skills.status(10).unwrap().0 as usize;
    skills.fields[slot].maximum = 76;

    skills.acknowledge_save(batch);

    assert_eq!(skills.status(10), Some((0, SkillUpdateState::Deleted)));
    assert_eq!(skills.fields()[slot].maximum(), 76);
}
