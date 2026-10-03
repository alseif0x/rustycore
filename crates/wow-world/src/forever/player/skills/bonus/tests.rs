use super::*;
use crate::forever::player::skills::SkillFields;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, SkillLineRecord},
};

fn birth(edges: &[(u32, u32)]) -> BirthCatalog {
    BirthRecords {
        skill_lines: edges
            .iter()
            .map(|&(id, parent_skill)| SkillLineRecord {
                id,
                parent_skill,
                category: 6, // Bonus propagation is not limited to professions.
                spell_icon_file: 0,
                can_link: 0,
                parent_tier_index: 0,
                flags: 0,
                spell_book_spell: 0,
                expansion_name_shared_string: 0,
                horde_expansion_name_shared_string: 0,
            })
            .collect(),
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .unwrap()
}
fn allocated(ids: impl IntoIterator<Item = u32>) -> PlayerSkills {
    let mut state = PlayerSkills::from_ids(ids);
    for field in state
        .fields
        .iter_mut()
        .filter(|field| field.starting_rank != 0)
    {
        field.step = 2;
        field.rank = 7;
        field.maximum = 75;
    }
    state
}

#[test]
fn both_bonus_kinds_propagate_to_all_live_descendants_without_a_save_state_or_rank_change() {
    let birth = birth(&[(4, 2), (3, 1), (2, 1), (1, 0)]);
    let mut state = allocated([1, 2, 3, 4]);
    state.status.get_mut(&2).unwrap().state = SkillUpdateState::New;
    state.status.get_mut(&3).unwrap().state = SkillUpdateState::Changed;
    for permanent in [false, true] {
        state.modify_bonus(&birth, 1, 5, permanent).unwrap();
        for skill in 1..=4 {
            assert_eq!(state.pure_value(skill), 7);
            assert_eq!(state.pure_maximum(skill), 75);
            assert_eq!(state.step(skill), 2);
            assert_eq!(
                state.fields[usize::from(state.status(skill).unwrap().0)].starting_rank,
                1
            );
            assert_eq!(
                if permanent {
                    state.permanent_bonus(skill)
                } else {
                    state.temporary_bonus(skill)
                },
                5
            );
        }
    }
    assert_eq!(state.value(4), 17);
    assert_eq!(state.base_value(4), 12);
    assert_eq!(state.status(1), Some((0, SkillUpdateState::Unchanged)));
    assert_eq!(state.status(2), Some((1, SkillUpdateState::New)));
    assert_eq!(state.status(3), Some((2, SkillUpdateState::Changed)));
    state.modify_bonus(&birth, 1, -5, false).unwrap();
    state.modify_bonus(&birth, 1, -5, true).unwrap();
    assert_eq!(state.value(4), 7);
}

#[test]
fn absent_deleted_and_zero_rank_nodes_prune_their_descendants_without_mutation() {
    let birth = birth(&[(1, 0), (2, 1), (3, 2), (4, 1), (5, 4)]);
    for root in [1, 2] {
        for mode in 0..3 {
            let mut state = allocated([1, 2, 3, 4, 5]);
            let slot = usize::from(state.status(root).unwrap().0);
            match mode {
                0 => {
                    state.status.remove(&root);
                }
                1 => {
                    state.status.get_mut(&root).unwrap().state = SkillUpdateState::Deleted;
                }
                _ => {
                    state.fields[slot].rank = 0;
                }
            }
            let fields = state.fields.clone();
            state.modify_bonus(&birth, 1, 5, false).unwrap();
            assert_eq!(state.fields[2].temporary_bonus, 0); // Child 3 is pruned.
            if root == 1 {
                assert!(state.fields.iter().zip(fields.iter()).all(|(a, b)| a == b));
            } else {
                assert_eq!(state.temporary_bonus(1), 5);
                assert_eq!(state.temporary_bonus(4), 5);
                assert_eq!(state.temporary_bonus(5), 5);
            }
        }
    }
}

#[test]
fn zero_identity_missing_line_and_uint16_narrowing_are_not_invented_gates_or_saturation() {
    let birth = birth(&[]);
    let mut state = allocated([0, 1]);
    state.fields[0].temporary_bonus = i16::MAX;
    state.modify_bonus(&birth, 0, 1, false).unwrap();
    assert_eq!(state.fields[0].temporary_bonus, i16::MIN);
    assert!(!state.has_skill(0)); // HasSkill's separate source zero gate.
    state.modify_bonus(&birth, 1, -1, true).unwrap();
    assert_eq!(state.fields[1].permanent_bonus, u16::MAX);
    assert_eq!(state.permanent_bonus(1), -1);
    assert_eq!(state.base_value(1), 6); // Pure 7 + unsigned 65535, then narrowing.
    state.modify_bonus(&birth, 1, 1, true).unwrap();
    assert_eq!(state.fields[1].permanent_bonus, 0);
}

#[test]
fn signed_overflow_retains_only_the_completed_depth_first_prefix_in_storage_child_order() {
    let birth = birth(&[(1, 0), (30, 1), (20, 1), (10, 1), (11, 10)]);
    let mut state = allocated([1, 10, 11, 20, 30]);
    state.fields[3].permanent_bonus = 1;
    assert_eq!(
        state.modify_bonus(&birth, 1, i32::MAX, true),
        Err(SkillBonusError::SignedAdditionOverflow)
    );
    for slot in [0, 1, 2] {
        assert_eq!(state.fields[slot].permanent_bonus, u16::MAX);
    }
    assert_eq!(state.fields[3].permanent_bonus, 1); // Failed addition not written.
    assert_eq!(state.fields[4].permanent_bonus, 0); // Later sibling not reached.
    let mut state = allocated([1]);
    state.fields[0].temporary_bonus = -1;
    assert_eq!(
        state.modify_bonus(&birth, 1, i32::MIN, false),
        Err(SkillBonusError::SignedAdditionOverflow)
    );
    assert_eq!(state.fields[0].temporary_bonus, -1);
}

#[test]
fn active_child_cycle_errors_before_repeating_a_mutation_but_an_inactive_cycle_is_pruned() {
    let birth = birth(&[(1, 2), (2, 1)]);
    let mut state = allocated([1, 2]);
    assert_eq!(
        state.modify_bonus(&birth, 1, 5, false),
        Err(SkillBonusError::RecursiveChildCycle)
    );
    assert_eq!(state.temporary_bonus(1), 5);
    assert_eq!(state.temporary_bonus(2), 5);
    state.fields[1].rank = 0;
    state.modify_bonus(&birth, 1, 5, false).unwrap();
    assert_eq!(state.temporary_bonus(1), 10);
    assert_eq!(state.fields[1].temporary_bonus, 5);
}

#[test]
fn full_capacity_descendant_chain_has_no_speculative_work_or_recursion_depth_limit() {
    let edges: Vec<_> = (1..=300).map(|skill| (skill, skill - 1)).collect();
    let birth = birth(&edges);
    let mut state = allocated(1..=300);
    state.modify_bonus(&birth, 1, -20, false).unwrap();
    assert!(
        state
            .fields
            .iter()
            .all(|field| field.temporary_bonus == -20)
    );
    assert!(
        state
            .fields
            .iter()
            .all(|field| field.rank == 7 && field.maximum == 75)
    );
    assert!(state.fields.iter().all(|field| field.starting_rank == 1));
    assert!(
        state
            .status
            .values()
            .all(|status| status.state == SkillUpdateState::Unchanged)
    );
    assert_eq!(state.value(300), 0);
    assert!(state.fields[299] != SkillFields::default());
}
