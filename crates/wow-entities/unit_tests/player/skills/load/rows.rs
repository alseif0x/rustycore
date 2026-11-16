use super::fixtures::{Effects, Event, Sources, ordered, row, seed};
use crate::player::skills::{
    PlayerSkills, SkillFields, SkillLoadError, SkillLoadOutcome, SkillUpdateState,
};

fn field(skills: &PlayerSkills, skill: u32) -> &SkillFields {
    let (slot, _) = skills.status(skill).unwrap();
    &skills.fields()[usize::from(slot)]
}

#[test]
fn row_prefix_preserves_occupied_forbidden_nodes_and_last_nonzero_payload() {
    let mut sources = Sources::default();
    sources.add_line(100, 4, 0, 0);
    sources.add_rc(100, 0);
    sources.add_ability(100, 1100, 5);
    sources.abilities.get_mut(&100).unwrap()[0].acquire_method = 1;

    let mut skills = PlayerSkills::initialize([]);
    seed(&mut skills, 777, 6, 42, 55, 7, 9, SkillUpdateState::Changed);
    let mut effects = Effects::new(&sources, 12);
    let mut provider_history = Vec::new();
    let outcome = skills
        .load_skills(
            &[
                row(100, 8, 33, -1),
                row(777, 91, 99, -1),
                row(100, 9, 35, -1),
                row(100, 0, 1, -1),
            ],
            &sources,
            &mut effects,
            |history, count| {
                provider_history.push(history.to_vec());
                assert_eq!(count, 1);
                Ok::<_, &'static str>(history.to_vec())
            },
        )
        .unwrap();

    assert_eq!(outcome, SkillLoadOutcome::Complete);
    assert_eq!(provider_history, [vec![100]]);
    let (slot, state) = skills.status(100).unwrap();
    assert_eq!(state, SkillUpdateState::Deleted);
    assert_eq!(skills.fields()[usize::from(slot)].step(), 0);
    assert_eq!(skills.fields()[usize::from(slot)].rank(), 9);
    assert_eq!(skills.fields()[usize::from(slot)].maximum(), 60);

    let (slot, state) = skills.status(777).unwrap();
    let field = &skills.fields()[usize::from(slot)];
    assert_eq!(state, SkillUpdateState::Changed);
    assert_eq!((field.step(), field.rank(), field.maximum()), (6, 42, 55));
    assert_eq!(field.temporary_bonus(), 7);
    assert_eq!(field.permanent_bonus(), 9);
    assert_eq!(effects.events, [Event::Learned(100, 1100)]);
}

#[test]
fn allowed_rows_update_fields_without_resetting_new_deleted_or_changed_status() {
    let mut sources = Sources::default();
    let states = [
        (200, SkillUpdateState::New),
        (201, SkillUpdateState::Deleted),
        (202, SkillUpdateState::Changed),
    ];
    let mut skills = PlayerSkills::initialize([]);
    for (skill, state) in states {
        sources.add_line(skill, 4, 0, 0);
        sources.add_rc(skill as u16, 0);
        seed(&mut skills, skill, 3, 10, 20, 1, 2, state);
    }
    let mut effects = Effects::new(&sources, 12);

    skills
        .load_skills(
            &[
                row(200, 30, 40, -1),
                row(201, 31, 41, -1),
                row(202, 32, 42, -1),
            ],
            &sources,
            &mut effects,
            ordered,
        )
        .unwrap();

    for (skill, expected_state, expected_rank) in [
        (200, SkillUpdateState::New, 30),
        (201, SkillUpdateState::Deleted, 31),
        (202, SkillUpdateState::Changed, 32),
    ] {
        let (slot, state) = skills.status(skill).unwrap();
        assert_eq!(state, expected_state);
        let field = &skills.fields()[usize::from(slot)];
        assert_eq!(field.rank(), expected_rank);
        assert_eq!(field.maximum(), 60);
        assert_eq!(field.temporary_bonus(), 0);
        assert_eq!(field.permanent_bonus(), 0);
    }
}

#[test]
fn raw_first_exact_tier_and_fixed_range_priority_match_the_row_contract() {
    let mut sources = Sources::default();
    for (skill, category, tier) in [
        (10, 4, 7),
        (960, 4, 0),
        (50, 8, 0),
        (51, 10, 0),
        (52, 9, 0),
        (53, 4, 0),
        (54, 8, 8),
    ] {
        sources.add_line(skill, category, 0, 0);
        sources.add_rc(skill as u16, tier);
    }
    let mut raw = [0; 16];
    raw[0] = 65_536;
    raw[1] = 0;
    raw[2] = 0;
    sources.tiers.insert(7, raw);
    let mut armor_tier = [0; 16];
    armor_tier[0] = 100;
    sources.tiers.insert(8, armor_tier);

    let mut skills = PlayerSkills::initialize([]);
    let mut effects = Effects::new(&sources, 30);
    skills
        .load_skills(
            &[
                row(10, 17, 0, -1),
                row(960, 90, 120, -1),
                row(50, 90, 120, -1),
                row(51, 90, 120, -1),
                row(52, 90, 120, -1),
                row(53, 90, 120, -1),
                row(54, 17, 100, -1),
            ],
            &sources,
            &mut effects,
            ordered,
        )
        .unwrap();

    assert_eq!(
        (field(&skills, 10).step(), field(&skills, 10).rank()),
        (2, 17)
    );
    assert_eq!(
        (field(&skills, 960).rank(), field(&skills, 960).maximum()),
        (1, 1)
    );
    assert_eq!(
        (field(&skills, 50).rank(), field(&skills, 50).maximum()),
        (1, 1)
    );
    assert_eq!(
        (field(&skills, 51).rank(), field(&skills, 51).maximum()),
        (300, 300)
    );
    assert_eq!(
        (field(&skills, 52).step(), field(&skills, 52).maximum()),
        (2, 150)
    );
    assert_eq!(
        (field(&skills, 53).step(), field(&skills, 53).maximum()),
        (0, 150)
    );
    assert_eq!(
        (
            field(&skills, 54).step(),
            field(&skills, 54).rank(),
            field(&skills, 54).maximum()
        ),
        (1, 17, 100)
    );
}

#[test]
fn invalid_profession_index_keeps_the_prefix_and_new_status_node() {
    let mut sources = Sources::default();
    sources.add_line(10, 4, 0, 0);
    sources.add_rc(10, 0);
    sources.add_line(11, 11, 0, 0);
    sources.add_rc(11, 0);
    for invalid_slot in [2, -2] {
        let mut skills = PlayerSkills::initialize([]);
        let mut effects = Effects::new(&sources, 10);
        let mut provider_called = false;

        let error = skills.load_skills(
            &[row(10, 20, 50, -1), row(11, 10, 50, invalid_slot)],
            &sources,
            &mut effects,
            |history, _| {
                provider_called = true;
                Ok::<_, &'static str>(history.to_vec())
            },
        );

        assert!(matches!(
            error,
            Err(SkillLoadError::InvalidProfessionSlot(slot)) if slot == invalid_slot
        ));
        assert!(!provider_called);
        assert_eq!(skills.status(10).unwrap().1, SkillUpdateState::Unchanged);
        assert_eq!(skills.fields()[0].rank(), 20);
        assert_eq!(skills.status(11).unwrap().1, SkillUpdateState::Unchanged);
        assert_eq!(skills.fields()[1].line(), 0);
    }
}
