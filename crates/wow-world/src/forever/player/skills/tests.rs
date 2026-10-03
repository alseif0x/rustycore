use super::*;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, SkillLineRecord},
};

fn birth(category: i8, parent_exists: bool) -> BirthCatalog {
    let line = |id, category, parent_skill| SkillLineRecord {
        id,
        category,
        parent_skill,
        spell_icon_file: 0,
        can_link: 0,
        parent_tier_index: 1,
        flags: 0,
        spell_book_spell: 0,
        expansion_name_shared_string: 0,
        horde_expansion_name_shared_string: 0,
    };
    let mut lines = vec![line(20, 6, 10)];
    if parent_exists {
        lines.push(line(10, category, 0));
    }
    BirthRecords {
        skill_lines: lines,
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .unwrap()
}
fn active(skill: u32, rank: u16, maximum: u16, temporary: i16, permanent: u16) -> PlayerSkills {
    let mut state = PlayerSkills::from_ids([skill]);
    state.fields[0].step = 7;
    state.fields[0].rank = rank;
    state.fields[0].maximum = maximum;
    state.fields[0].temporary_bonus = temporary;
    state.fields[0].permanent_bonus = permanent;
    state
}
fn query(state: &PlayerSkills, skill: u32) -> (bool, u16, u16, u16, u16, u16, u16, i16, i16) {
    (
        state.has_skill(skill),
        state.step(skill),
        state.pure_value(skill),
        state.pure_maximum(skill),
        state.value(skill),
        state.maximum(skill),
        state.base_value(skill),
        state.temporary_bonus(skill),
        state.permanent_bonus(skill),
    )
}

#[test]
fn preallocation_sets_only_line_and_starting_rank_without_learning_or_a_save_mutation() {
    let state = PlayerSkills::from_ids([10, 20]);
    assert_eq!(state.status(10), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(state.status(20), Some((1, SkillUpdateState::Unchanged)));
    assert_eq!(
        (state.fields[0].line(), state.fields[0].starting_rank()),
        (10, 1)
    );
    assert_eq!(
        (state.fields[1].line(), state.fields[1].starting_rank()),
        (20, 1)
    );
    assert!(
        state.fields[2..]
            .iter()
            .all(|field| field == &SkillFields::default())
    );
    assert_eq!(query(&state, 10), (false, 0, 0, 0, 0, 0, 0, 0, 0));
}

#[test]
fn capacity_and_uint16_field_narrowing_do_not_truncate_the_status_identity() {
    let state = PlayerSkills::from_ids(1..=301);
    assert_eq!(state.status.len(), 300);
    assert_eq!(state.status(300), Some((299, SkillUpdateState::Unchanged)));
    assert!(state.status(301).is_none());
    let state = active(65536, 3, 9, 0, 0);
    assert_eq!(state.fields[0].line(), 0);
    assert!(state.has_skill(65536)); // Source queries status/rank, not narrowed line.
    assert!(!state.has_skill(0));
}

#[test]
fn zero_id_missing_deleted_and_zero_rank_share_source_query_defaults() {
    for state in [active(0, 10, 20, 3, 4), active(1, 0, 20, 3, 4), {
        let mut state = active(1, 10, 20, 3, 4);
        state.status.get_mut(&1).unwrap().state = SkillUpdateState::Deleted;
        state
    }] {
        assert_eq!(query(&state, 0), (false, 0, 0, 0, 0, 0, 0, 0, 0));
        assert_eq!(query(&state, 1), (false, 0, 0, 0, 0, 0, 0, 0, 0));
        assert_eq!(query(&state, 99), (false, 0, 0, 0, 0, 0, 0, 0, 0));
    }
}

#[test]
fn live_status_variants_do_not_change_queries_or_clamp_rank_to_maximum() {
    for flag in [
        SkillUpdateState::Unchanged,
        SkillUpdateState::Changed,
        SkillUpdateState::New,
    ] {
        let mut state = active(1, 200, 100, 3, 4);
        state.status.get_mut(&1).unwrap().state = flag;
        assert_eq!(query(&state, 1), (true, 7, 200, 100, 207, 107, 204, 3, 4));
    }
}

#[test]
fn negative_temporary_bonus_clamps_only_the_signed_sum_not_the_base_or_pure_value() {
    let state = active(1, 200, 100, -300, 0);
    assert_eq!(query(&state, 1), (true, 7, 200, 100, 0, 0, 200, -300, 0));
}

#[test]
fn permanent_field_is_unsigned_for_sums_but_signed_for_its_named_value_getter() {
    let state = active(1, 1, 2, 0, u16::MAX);
    assert_eq!(query(&state, 1), (true, 7, 1, 2, 0, 1, 0, 0, -1));
    let state = active(1, u16::MAX, u16::MAX, i16::MAX, u16::MAX);
    assert_eq!(state.value(1), 32765);
    assert_eq!(state.base_value(1), 65534);
    assert_eq!(state.pure_value(1), u16::MAX);
}

#[test]
fn classic_profession_child_copies_parent_pure_state_not_requested_values_or_bonuses() {
    for category in [9, 11] {
        let birth = birth(category, true);
        let state = active(10, 75, 150, 30, 40);
        let input = SkillUpdate {
            skill: 20,
            step: 5,
            value: 1,
            maximum: 300,
        };
        assert_eq!(
            state.normalize_classic_skill_update(&birth, input),
            Some(SkillUpdate {
                skill: 20,
                step: 7,
                value: 75,
                maximum: 150
            })
        );
        assert_eq!(PlayerSkills::classic_profession_skill(&birth, 20), 10);
        assert_eq!(state.pure_value(10), 75); // Query does not alter parent rank.
    }
}

#[test]
fn missing_zero_rank_or_deleted_profession_parent_disables_a_nonzero_child_request() {
    let birth = birth(11, true);
    let mut deleted = active(10, 75, 150, 0, 0);
    deleted.status.get_mut(&10).unwrap().state = SkillUpdateState::Deleted;
    for state in [
        PlayerSkills::from_ids([]),
        active(10, 0, 150, 0, 0),
        deleted,
    ] {
        assert_eq!(
            state.normalize_classic_skill_update(
                &birth,
                SkillUpdate {
                    skill: 20,
                    step: 1,
                    value: 10,
                    maximum: 100
                }
            ),
            Some(SkillUpdate {
                skill: 20,
                step: 0,
                value: 0,
                maximum: 0
            })
        );
    }
}

#[test]
fn zero_child_request_and_nonprofession_or_missing_parent_keep_source_parameters() {
    let state = active(10, 75, 150, 0, 0);
    let zero = SkillUpdate {
        skill: 20,
        step: 3,
        value: 0,
        maximum: 200,
    };
    assert_eq!(
        state.normalize_classic_skill_update(&birth(11, true), zero),
        Some(zero)
    );
    let update = SkillUpdate {
        skill: 20,
        step: 3,
        value: 5,
        maximum: 200,
    };
    for birth in [birth(8, true), birth(6, true), birth(11, false)] {
        assert_eq!(
            state.normalize_classic_skill_update(&birth, update),
            Some(update)
        );
        assert_eq!(PlayerSkills::classic_profession_skill(&birth, 20), 20);
        assert_eq!(PlayerSkills::classic_profession_skill(&birth, 99), 99);
        assert!(
            state
                .normalize_classic_skill_update(
                    &birth,
                    SkillUpdate {
                        skill: 99,
                        ..update
                    }
                )
                .is_none()
        );
    }
}
