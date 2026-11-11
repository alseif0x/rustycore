use super::*;
use crate::forever::player::SkillUpdateState::{New, Unchanged};
use std::sync::Arc;

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
    assert!(Arc::strong_count(&fixture.birth) >= 3);
}
