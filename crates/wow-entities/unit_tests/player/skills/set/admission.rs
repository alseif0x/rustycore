use super::*;
use crate::SkillUpdateState::{New, Unchanged};

#[test]
fn missing_line_returns_before_effects_and_empty_slot_zero_is_source_no_free_slot() {
    let fixture = Fixture::new(vec![line(10, 6, 0, 0), line(301, 6, 0, 0)], vec![]);
    let mut skills = allocated([]);
    let mut effects = Effects::default();
    assert_eq!(
        skills.set_skill(update(99, 1, 1, 75), fixture.sources(), &mut effects),
        Ok(SkillSetOutcome::MissingLine)
    );
    assert_eq!(
        skills.set_skill(update(10, 1, 1, 75), fixture.sources(), &mut effects),
        Ok(SkillSetOutcome::NoFreeSlot)
    );
    assert!(effects.events.is_empty());
    assert!(skills.status(10).is_none());
    let mut skills = allocated(1..=300);
    assert_eq!(
        skills.set_skill(update(301, 1, 1, 75), fixture.sources(), &mut effects),
        Ok(SkillSetOutcome::NoFreeSlot)
    );
    assert!(effects.events.is_empty());
}

#[test]
fn captured_absent_parent_slot_and_zero_rank_children_preserve_source_aliasing_not_a_reservation_fix()
 {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0), line(20, 6, 10, 4)], vec![]);
    let mut skills = allocated([1]);
    let mut effects = Effects::default();
    skills
        .set_skill(update(10, 2, 7, 75), fixture.sources(), &mut effects)
        .unwrap();
    // Source selects slot 1, then inserts child 20 in slot 1, then overwrites
    // that captured slot with parent 10. Do not silently rebase/reserve it.
    assert_eq!(skills.status(10), Some((1, New)));
    assert_eq!(skills.status(20), Some((1, New)));
    assert_eq!(skills.fields[1].line(), 10);
    assert_eq!(skills.pure_value(20), 7);
    assert_eq!(effects.events.first(), Some(&Event::Enchant(20, 0, 0, 0)));
    assert_eq!(skills.profession_lines(), &[10, 0]);
    assert_eq!(
        effects
            .events
            .iter()
            .filter(|event| matches!(event, Event::Learn(20, ..)))
            .count(),
        0
    );
}

#[test]
fn no_free_slot_return_still_runs_profession_final_sync_and_high_id_retains_identity_bits() {
    let fixture = Fixture::new(
        vec![line(10, 11, 0, 0), line(20, 6, 10, 4), line(65536, 6, 0, 0)],
        vec![],
    );
    let mut skills = allocated([0, 20]);
    activate(&mut skills, 20, 4, 75, New);
    let mut effects = Effects::default();
    assert_eq!(
        skills.set_skill(update(10, 1, 1, 75), fixture.sources(), &mut effects),
        Ok(SkillSetOutcome::NoFreeSlot)
    );
    assert_eq!(effects.events, [Event::Enchant(20, 4, 0, 4)]);
    assert_eq!(skills.status(20), Some((1, Unchanged)));
    let mut skills = allocated([1]);
    effects.events.clear();
    skills
        .set_skill(update(65536, 1, 3, 10), fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(skills.fields[1].line(), 0);
    assert_eq!(skills.status(65536), Some((1, New)));
    assert!(skills.has_skill(65536));
}

#[test]
fn required_effect_failure_retains_prefix_without_success_status_or_final_child_sync() {
    let fixture = Fixture::new(vec![line(10, 11, 0, 0), line(20, 6, 10, 4)], rewards(&[10]));
    let mut skills = allocated([10, 20]);
    let mut effects = Effects {
        fail_at: Some(0),
        ..Default::default()
    };
    assert_eq!(
        skills.set_skill(update(10, 1, 7, 75), fixture.sources(), &mut effects),
        Err(SkillSetError::Effect("unavailable required effect"))
    );
    assert_eq!(skills.pure_value(10), 7);
    assert_eq!(skills.pure_value(20), 0);
    assert_eq!(skills.status(10), Some((0, Unchanged)));
    assert_eq!(skills.profession_lines(), &[0, 0]);
    assert_eq!(effects.events, [Event::Learn(10, 7, Some(Unchanged))]);
}

#[test]
fn recursive_parent_cycle_is_an_error_before_mutation_not_a_depth_limited_success() {
    let mut fixture = Fixture::new(vec![line(10, 6, 20, 1), line(20, 6, 10, 1)], vec![]);
    fixture
        .selected_rc
        .extend([(10, rc(1, 10, 1)), (20, rc(2, 20, 1))]);
    fixture.tiers.insert((1, 0), 75);
    let mut skills = allocated([10, 20]);
    let mut effects = Effects::default();
    assert_eq!(
        skills.set_skill(update(10, 1, 1, 75), fixture.sources(), &mut effects),
        Err(SkillSetError::RecursiveSkillCycle)
    );
    assert_eq!((skills.pure_value(10), skills.pure_value(20)), (0, 0));
    assert!(effects.events.is_empty());
}
