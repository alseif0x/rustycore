use super::fixtures::{Effects, Event, Sources, ordered, row, seed};
use crate::SkillSetError;
use crate::player::skill_sources::{SkillLineMetadata, SkillRewardSpellMetadata, SkillUpdate};
use crate::player::skills::{
    PlayerSkills, SkillFields, SkillLoadError, SkillLoadOutcome, SkillUpdateState,
};

#[test]
fn provider_order_controls_reward_order_after_all_rows_and_exact_set_is_required() {
    let mut sources = Sources::default();
    for skill in [10, 20] {
        sources.add_line(skill, 4, 0, 0);
        sources.add_rc(skill as u16, 0);
        sources.add_ability(skill, skill + 1000, 0);
    }
    let mut skills = PlayerSkills::initialize([], || {});
    let mut effects = Effects::new(&sources, 20);
    effects.observe_skill = Some(10);
    let mut received = None;
    let outcome = skills
        .load_skills(
            &[row(10, 5, 10, -1), row(20, 6, 10, -1)],
            &sources,
            &mut effects,
            |history, count| {
                received = Some((history.to_vec(), count));
                Ok::<_, &'static str>(vec![20, 10])
            },
        )
        .unwrap();
    assert_eq!(outcome, SkillLoadOutcome::Complete);
    assert_eq!(received, Some((vec![10, 20], 2)));
    assert_eq!(
        effects.events,
        [Event::Learned(20, 1020), Event::Learned(10, 1010)]
    );
    assert_eq!(effects.observed_skill, Some(true));

    for malformed in [vec![10, 10], vec![10, 30], vec![10]] {
        let mut skills = PlayerSkills::initialize([], || {});
        let mut effects = Effects::new(&sources, 20);
        let error = skills.load_skills(
            &[row(10, 5, 10, -1), row(20, 6, 10, -1)],
            &sources,
            &mut effects,
            |_, _| Ok::<_, &'static str>(malformed.clone()),
        );
        assert!(matches!(error, Err(SkillLoadError::InvalidOrderKeySet)));
        assert!(effects.events.is_empty());
        assert_eq!(skills.status(10).unwrap().1, SkillUpdateState::Unchanged);
        assert_eq!(skills.status(20).unwrap().1, SkillUpdateState::Unchanged);
    }

    let mut empty_skills = PlayerSkills::initialize([], || {});
    let mut empty_effects = Effects::new(&sources, 20);
    let mut empty_provider_called = false;
    assert_eq!(
        empty_skills
            .load_skills(&[], &sources, &mut empty_effects, |history, count| {
                empty_provider_called = true;
                assert!(history.is_empty());
                assert_eq!(count, 0);
                Ok::<_, &'static str>(Vec::new())
            })
            .unwrap(),
        SkillLoadOutcome::Complete
    );
    assert!(empty_provider_called);
}

#[test]
fn provider_errors_and_source_or_effect_errors_keep_the_applied_prefix() {
    let mut sources = Sources::default();
    sources.add_line(10, 4, 0, 0);
    sources.add_rc(10, 0);
    sources.add_ability(10, 1010, 0);

    let mut skills = PlayerSkills::initialize([], || {});
    let mut effects = Effects::new(&sources, 10);
    let order_error = skills.load_skills(&[row(10, 7, 50, -1)], &sources, &mut effects, |_, _| {
        Err::<Vec<u32>, _>("order unavailable")
    });
    assert!(matches!(
        order_error,
        Err(SkillLoadError::Order("order unavailable"))
    ));
    assert_eq!(skills.fields()[0].rank(), 7);
    let mut expected = [0; 66];
    for bit in [0, 1, 301, 601, 901, 1201, 1501, 1801] {
        expected[bit / 32] |= 1 << (bit % 32);
    }
    assert_eq!(skills.field_changes(), &expected);
    assert_eq!(effects.requests, 4);
    assert!(effects.events.is_empty());

    let mut row_error_sources = Sources::default();
    row_error_sources.add_line(10, 4, 0, 0);
    row_error_sources.add_rc(10, 0);
    row_error_sources.rc_error = Some(20);
    let mut row_error_skills = PlayerSkills::initialize([], || {});
    let mut row_error_effects = Effects::new(&row_error_sources, 10);
    let row_error = row_error_skills.load_skills(
        &[row(10, 7, 50, -1), row(20, 9, 50, -1)],
        &row_error_sources,
        &mut row_error_effects,
        ordered,
    );
    assert!(matches!(
        row_error,
        Err(SkillLoadError::Source("skill race/class source failed"))
    ));
    assert_eq!(row_error_skills.fields()[0].rank(), 7);

    let mut effects = Effects::new(&sources, 10);
    effects.fail_learning = true;
    let mut effect_skills = PlayerSkills::initialize([], || {});
    let effect_error =
        effect_skills.load_skills(&[row(10, 7, 50, -1)], &sources, &mut effects, ordered);
    assert!(matches!(
        effect_error,
        Err(SkillLoadError::SkillSet(SkillSetError::Effect(
            "reward effect failed"
        )))
    ));
    assert_eq!(effect_skills.fields()[0].rank(), 7);
    assert_eq!(effects.events, [Event::Learned(10, 1010)]);

    let mut missing_line_sources = Sources::default();
    missing_line_sources.add_rc(42, 0);
    missing_line_sources.add_ability(42, 1042, 0);
    let mut missing_line_skills = PlayerSkills::initialize([], || {});
    let mut missing_line_effects = Effects::new(&missing_line_sources, 10);
    let missing_line = missing_line_skills.load_skills(
        &[row(42, 7, 50, -1)],
        &missing_line_sources,
        &mut missing_line_effects,
        ordered,
    );
    assert!(matches!(
        missing_line,
        Err(SkillLoadError::MissingLoadedSkillLine(42))
    ));
    assert_eq!(missing_line_skills.fields()[0].rank(), 7);
    assert_eq!(missing_line_effects.events, [Event::Learned(42, 1042)]);
}

#[test]
fn capacity_stops_before_duplicate_but_still_runs_post_row_rewards() {
    let mut sources = Sources::default();
    sources.add_line(500, 4, 0, 0);
    sources.add_rc(500, 0);
    sources.add_ability(500, 1500, 0);
    sources.add_child(
        500,
        SkillLineMetadata {
            id: 501,
            category: 4,
            parent_skill: 500,
            parent_tier_index: 0,
        },
    );
    let mut skills = PlayerSkills::initialize(0..299, || {});
    let mut effects = Effects::new(&sources, 10);
    let mut received = None;
    let outcome = skills
        .load_skills(
            &[row(500, 8, 20, -1), row(500, 0, 0, -1)],
            &sources,
            &mut effects,
            |history, count| {
                received = Some((history.to_vec(), count));
                Ok::<_, &'static str>(history.to_vec())
            },
        )
        .unwrap();
    assert_eq!(outcome, SkillLoadOutcome::CapacityStopped);
    assert_eq!(received, Some((vec![500], 1)));
    assert_eq!(skills.status(500).unwrap().1, SkillUpdateState::Unchanged);
    assert_eq!(skills.status(500).unwrap().0, 299);
    assert_eq!(skills.fields()[299].rank(), 8);
    assert_eq!(skills.status(501), None);
    assert_eq!(effects.events, [Event::Learned(500, 1500)]);
}

#[test]
fn callbacks_reenter_the_same_owner_before_parent_and_live_level_phases() {
    let mut sources = Sources::default();
    sources.add_line(100, 4, 200, 2);
    sources.add_line(101, 4, 0, 0);
    sources.add_line(200, 4, 0, 0);
    sources.add_line(473, 4, 0, 0);
    sources.add_rc(100, 0);
    sources.add_rc(101, 0);
    sources.add_rc(200, 0);
    sources.add_rc(473, 0);
    sources.add_ability(100, 1100, 0);
    sources.add_ability(101, 1101, 0);
    sources.rc_error = Some(200);
    sources.spell_metadata().insert(
        1101,
        SkillRewardSpellMetadata {
            future_condition_id: 0,
            spell_level: 15,
            base_level: 15,
        },
    );

    let mut skills = PlayerSkills::initialize([], || {});
    seed(&mut skills, 200, 0, 5, 50, 0, 0, SkillUpdateState::Changed);
    seed(
        &mut skills,
        473,
        0,
        3,
        20,
        0,
        0,
        SkillUpdateState::Unchanged,
    );
    seed(
        &mut skills,
        162,
        0,
        10,
        20,
        3,
        4,
        SkillUpdateState::Unchanged,
    );
    let mut effects = Effects::new(&sources, 10);
    effects.reenter = Some(SkillUpdate {
        skill: 200,
        step: 2,
        value: 1,
        maximum: 80,
    });
    effects.level_after_reward = Some(20);

    skills
        .load_skills(
            &[row(100, 7, 30, -1), row(101, 9, 30, -1)],
            &sources,
            &mut effects,
            |_, _| Ok::<_, &'static str>(vec![100, 101]),
        )
        .unwrap();

    assert_eq!(skills.status(200).unwrap().1, SkillUpdateState::Changed);
    assert_eq!(
        (
            skills.step(200),
            skills.pure_value(200),
            skills.pure_maximum(200)
        ),
        (2, 1, 80)
    );
    assert!(effects.events.contains(&Event::Learned(101, 1101)));
    assert_eq!(
        (skills.pure_value(473), skills.pure_maximum(473)),
        (17, 100)
    );
}

#[test]
fn parent_tier_clamps_and_backtracks_then_narrows_to_skill_field_width() {
    let mut sources = Sources::default();
    sources.add_line(100, 4, 200, 20);
    sources.add_line(200, 4, 0, 0);
    sources.add_rc(100, 0);
    sources.add_rc(200, 8);
    let mut tier = [0; 16];
    tier[13] = 70_000;
    sources.tiers.insert(8, tier);
    let mut skills = PlayerSkills::initialize([], || {});
    seed(&mut skills, 200, 0, 7, 50, 0, 0, SkillUpdateState::Changed);
    let mut effects = Effects::new(&sources, 10);

    skills
        .load_skills(&[row(100, 9, 30, -1)], &sources, &mut effects, ordered)
        .unwrap();
    assert_eq!(skills.step(200), 20);
    assert_eq!(skills.pure_value(200), 7);
    assert_eq!(skills.pure_maximum(200), 70_000u32 as u16);
}

#[test]
fn ordered_full_width_children_preserve_other_slot_fields_and_profession_fixups_repeat() {
    let mut sources = Sources::default();
    sources.add_line(100, 4, 0, 0);
    sources.add_rc(100, 0);
    sources.add_child(
        100,
        SkillLineMetadata {
            id: 70_000,
            category: 4,
            parent_skill: 100,
            parent_tier_index: 0,
        },
    );
    sources.add_child(
        100,
        SkillLineMetadata {
            id: 300,
            category: 4,
            parent_skill: 100,
            parent_tier_index: 0,
        },
    );
    let mut skills = PlayerSkills::initialize([], || {});
    skills.fields[1] = SkillFields {
        line: 1,
        step: 9,
        rank: 8,
        starting_rank: 7,
        maximum: 6,
        temporary_bonus: 5,
        permanent_bonus: 4,
    };
    let mut effects = Effects::new(&sources, 10);
    skills.clear_changes();
    skills
        .load_skills(&[row(100, 2, 10, -1)], &sources, &mut effects, ordered)
        .unwrap();
    assert_eq!(skills.status(70_000).unwrap().0, 1);
    assert_eq!(skills.status(300).unwrap().0, 2);
    let child = &skills.fields()[1];
    assert_eq!(child.line(), 70_000u32 as u16);
    assert_eq!(child.starting_rank(), 1);
    assert_eq!((child.step(), child.rank(), child.maximum()), (9, 8, 6));
    assert_eq!(child.temporary_bonus(), 5);
    assert_eq!(child.permanent_bonus(), 4);
    for bit in [2, 902] {
        assert_ne!(skills.field_changes()[bit / 32] & (1 << (bit % 32)), 0);
    }
    for bit in [302, 602, 1202, 1502, 1802] {
        assert_eq!(skills.field_changes()[bit / 32] & (1 << (bit % 32)), 0);
    }

    let mut profession_sources = Sources::default();
    profession_sources.add_line(500, 11, 0, 0);
    profession_sources.add_rc(500, 0);
    let mut profession_skills = PlayerSkills::initialize([], || {});
    let mut profession_effects = Effects::new(&profession_sources, 15);
    profession_skills
        .load_skills(
            &[row(500, 75, 75, -1), row(500, 60, 75, -1)],
            &profession_sources,
            &mut profession_effects,
            ordered,
        )
        .unwrap();
    assert_eq!(profession_skills.profession_lines(), &[500, 500]);
    assert_eq!(
        profession_skills.status(500).unwrap().1,
        SkillUpdateState::Changed
    );
    assert_eq!(profession_skills.pure_value(500), 60);
    let (profession_slot, _) = profession_skills.status(500).unwrap();
    assert_eq!(
        profession_skills.fields()[usize::from(profession_slot)].step(),
        1
    );
    assert_eq!(
        profession_skills.fields()[usize::from(profession_slot)].maximum(),
        75
    );

    let mut explicit_sources = Sources::default();
    explicit_sources.add_line(501, 11, 0, 0);
    explicit_sources.add_rc(501, 0);
    let mut explicit_skills = PlayerSkills::initialize([], || {});
    let mut explicit_effects = Effects::new(&explicit_sources, 15);
    explicit_skills
        .load_skills(
            &[row(501, 0, 75, 1)],
            &explicit_sources,
            &mut explicit_effects,
            ordered,
        )
        .unwrap();
    assert_eq!(explicit_skills.profession_lines(), &[0, 501]);
    assert_eq!(
        explicit_skills.status(501).unwrap().1,
        SkillUpdateState::Deleted
    );
    assert_eq!(explicit_skills.fields()[0].line(), 501);
    let mut expected = [0; 66];
    expected[0] = (1 << 0) | (1 << 1);
    assert_eq!(explicit_skills.field_changes(), &expected);
    assert_eq!(explicit_skills.profession_changes(), 1 << 1);
    assert_eq!(explicit_effects.requests, 2);
}
