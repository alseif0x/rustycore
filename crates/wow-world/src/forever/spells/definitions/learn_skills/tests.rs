use super::super::learning_fixtures as f;
use super::*;
#[test]
fn first_matching_effect_source_order_regular_only_and_uint16_casts_are_retained() {
    let mut a = f::effect(118, 0, -1, -2.75);
    a.scaling_variance = 0.5;
    let mut b = f::effect(118, 0, 7, 12.5);
    b.scaling_variance = 0.5;
    let s = f::before_skills(vec![
        ((20, 0), f::definition(vec![a, f::effect(40, 0, 0, 0.0)])),
        ((10, 0), f::definition(vec![b])),
        (
            (30, 0),
            f::definition(vec![
                f::effect(40, 0, 0, 0.0),
                f::effect(118, 0, 9, f32::NAN),
            ]),
        ),
        ((40, 2), f::definition(vec![f::effect(118, 0, 9, f32::NAN)])),
    ]);
    let mut calls = 0;
    let s = s
        .with_learn_skills(&f::items(), &mut |low, high| {
            assert_eq!((low, high), (-0.25, 0.25));
            calls += 1;
            Ok(0.0)
        })
        .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(
        s.spell_learn_skill(20),
        Some(&SpellLearnSkillNode {
            skill: u16::MAX,
            step: 65534,
            value: 0,
            max_value: 0
        })
    );
    assert_eq!(s.spell_learn_skill(10).unwrap().step, 12);
    assert_eq!(
        s.spell_learn_skill(30),
        Some(&SpellLearnSkillNode {
            skill: 118,
            step: 1,
            value: 1,
            max_value: 1
        })
    );
    assert!(s.spell_learn_skill(40).is_none());
    assert_eq!(
        s.learn_skill_counts(),
        Some(LearnSkillCounts {
            regular_definitions: 3,
            nodes: 3,
            skill_values: 2,
            dual_wield: 1
        })
    );
    assert!(matches!(
        s.with_learn_skills(&f::items(), &mut f::no_draw),
        Err(SpellLearnSkillError::AlreadyApplied)
    ));
}
#[test]
fn rng_and_undefined_value_failures_drop_the_consumed_phase() {
    let mut effect = f::effect(118, 0, 1, 2.0);
    effect.scaling_variance = 1.0;
    let s = f::before_skills(vec![((1, 0), f::definition(vec![effect]))]);
    assert!(matches!(
        s.with_learn_skills(&f::items(), &mut |_, _| Err(
            SpellValueError::RandomSourceUnavailable
        )),
        Err(SpellLearnSkillError::Value(
            SpellValueError::RandomSourceUnavailable
        ))
    ));
    let s = f::before_skills(vec![(
        (1, 0),
        f::definition(vec![f::effect(118, 0, 1, f32::NAN)]),
    )]);
    assert!(matches!(
        s.with_learn_skills(&f::items(), &mut f::no_draw),
        Err(SpellLearnSkillError::Value(
            SpellValueError::UndefinedIntegerCast
        ))
    ));
}
#[test]
fn admission_and_empty_regular_definitions_do_not_invent_a_skill() {
    let mut s = f::before_skills(vec![]);
    s.required = None;
    assert!(matches!(
        s.with_learn_skills(&f::items(), &mut f::no_draw),
        Err(SpellLearnSkillError::RequiresRequiredSpells)
    ));
    let s = f::before_skills(vec![((1, 0), f::definition(vec![]))])
        .with_learn_skills(&f::items(), &mut f::no_draw)
        .unwrap();
    assert!(s.spell_learn_skill(1).is_none());
    assert_eq!(s.learn_skill_counts().unwrap().regular_definitions, 1);
}
