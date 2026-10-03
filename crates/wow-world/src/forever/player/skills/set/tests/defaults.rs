use super::*;
use crate::forever::player::SkillUpdateState::{New, Unchanged};
use wow_persistence::forever::creation::SkillTierRow;

#[test]
fn defaults_execute_language_mono_level_and_tier_on_canonical_fields_in_rc_storage_order() {
    let mut ranked = rc(4, 40, 1);
    ranked.flags = 0x10;
    let fixture = Fixture::with_rc(
        vec![
            line(10, 10, 0, 0),
            line(20, 8, 0, 0),
            line(30, 6, 0, 0),
            line(40, 6, 0, 0),
        ],
        rewards(&[10, 20, 30, 40]),
        vec![ranked, rc(3, 30, 0), rc(2, 20, 0), rc(1, 10, 0)],
        vec![SkillTierRow {
            id: 1,
            values: [75; 16],
        }],
        vec![1, 2, 3, 4],
    );
    let initial = fixture.world.initial_skill_fields(1, 1, 1).unwrap();
    let mut skills = initial.initialize_player_skills();
    let mut effects = Effects::default();
    skills
        .learn_default_skills(&fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(
        effects.grants,
        [
            (1010, 10, false),
            (1020, 20, false),
            (1030, 30, false),
            (1040, 40, false)
        ]
    );
    for (id, step, rank, cap) in [
        (10, 0, 300, 300),
        (20, 0, 1, 1),
        (30, 0, 1, 5),
        (40, 1, 75, 75),
    ] {
        assert_eq!(
            (
                skills.step(id),
                skills.pure_value(id),
                skills.pure_maximum(id)
            ),
            (step, rank, cap)
        );
        assert_eq!(skills.status(id).unwrap().1, New);
    }
}

#[test]
fn duplicate_requests_and_a_skill_learned_by_an_earlier_reward_are_checked_live_not_overwritten() {
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 0, 0)],
        rewards(&[10]),
        vec![rc(1, 10, 0), rc(2, 10, 0), rc(3, 20, 0)],
        vec![],
        vec![2, 1, 3],
    );
    let initial = fixture.world.initial_skill_fields(1, 1, 1).unwrap();
    assert_eq!(initial.default_requests().len(), 3);
    let mut skills = initial.initialize_player_skills();
    let sources = fixture.sources();
    let mut effects = Effects {
        reenter: Some((&sources, update(20, 3, 60, 150))),
        ..Default::default()
    };
    skills.learn_default_skills(&sources, &mut effects).unwrap();
    assert_eq!(effects.grants, [(1010, 10, false)]);
    assert_eq!(
        (
            skills.step(20),
            skills.pure_value(20),
            skills.pure_maximum(20)
        ),
        (3, 60, 150)
    );
    assert_eq!(
        effects
            .events
            .iter()
            .filter(|event| matches!(event, Event::Criteria(10, SkillCriteria::Raised, ..)))
            .count(),
        1
    );
}

#[test]
fn level_changed_by_a_prior_grant_admits_later_rc_that_the_initial_plan_could_not_include() {
    let mut later = rc(2, 20, 0);
    later.min_level = 2;
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 0, 0)],
        rewards(&[10, 20]),
        vec![rc(1, 10, 0), later],
        vec![],
        vec![1, 2],
    );
    let initial = fixture.world.initial_skill_fields(1, 1, 1).unwrap();
    assert_eq!(initial.default_requests().len(), 1);
    let mut skills = initial.initialize_player_skills();
    let mut effects = Effects {
        level_after_grant: Some(2),
        ..Default::default()
    };
    skills
        .learn_default_skills(&fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!((skills.pure_value(10), skills.pure_maximum(10)), (1, 5));
    assert_eq!((skills.pure_value(20), skills.pure_maximum(20)), (1, 10));
    assert_eq!(effects.grants.len(), 2);
}

#[test]
fn signed_min_level_unavailable_and_orphan_rc_preserve_source_filters_without_extra_mutations() {
    let mut signed = rc(1, 10, 0);
    signed.min_level = i8::MIN;
    let mut future = rc(2, 20, 0);
    future.min_level = 2;
    let mut unavailable = rc(3, 30, 0);
    unavailable.availability = 0;
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 0, 0), line(30, 6, 0, 0)],
        rewards(&[10]),
        vec![signed, future, unavailable, rc(4, 40, 0)],
        vec![],
        vec![1, 2, 3],
    );
    let initial = fixture.world.initial_skill_fields(1, 1, 1).unwrap();
    let mut skills = initial.initialize_player_skills();
    let mut effects = Effects::default();
    skills
        .learn_default_skills(&fixture.sources(), &mut effects)
        .unwrap();
    assert_eq!(effects.grants, [(1010, 10, false)]);
    assert!(!skills.has_skill(20));
    assert!(!skills.has_skill(30));
    assert_eq!(skills.status(20).unwrap().1, Unchanged);
    assert_eq!(skills.status(30).unwrap().1, Unchanged);
    assert!(skills.status(40).is_none());
}

#[test]
fn required_effect_error_aborts_default_learning_after_its_completed_field_prefix() {
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 0, 0)],
        rewards(&[10, 20]),
        vec![rc(1, 10, 0), rc(2, 20, 0)],
        vec![],
        vec![1, 2],
    );
    let initial = fixture.world.initial_skill_fields(1, 1, 1).unwrap();
    let mut skills = initial.initialize_player_skills();
    let mut effects = Effects {
        fail_at: Some(0),
        ..Default::default()
    };
    assert_eq!(
        skills.learn_default_skills(&fixture.sources(), &mut effects),
        Err(SkillSetError::Effect("unavailable required effect"))
    );
    assert_eq!((skills.pure_value(10), skills.pure_value(20)), (1, 0));
    assert_eq!(skills.status(10).unwrap().1, Unchanged);
    assert!(effects.grants.is_empty());
}

#[test]
fn ordinary_capacity_return_does_not_abort_the_remaining_source_default_calls() {
    let fixture = Fixture::with_rc(
        vec![line(10, 6, 0, 0), line(20, 6, 0, 0)],
        rewards(&[20]),
        vec![rc(1, 10, 0), rc(2, 20, 0)],
        vec![],
        vec![1, 2],
    );
    let mut skills = allocated([0, 20]);
    let mut effects = Effects::default();
    skills
        .learn_default_skills(&fixture.sources(), &mut effects)
        .unwrap();
    assert!(skills.status(10).is_none()); // Slot-zero source return.
    assert_eq!(skills.pure_value(20), 1);
    assert_eq!(effects.grants, [(1020, 20, false)]);
}
