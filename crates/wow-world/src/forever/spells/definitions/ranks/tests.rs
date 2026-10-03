use super::super::learning_fixtures as f;
use super::*;
use wow_data::{forever_birth::BirthRecords, forever_spells::*};
fn load(ids: &[u32], relations: Vec<(u32, u32)>) -> SpellDefinitionSeeds {
    f::seeds(
        ids.iter()
            .map(|&id| ((id, 0), f::definition(vec![])))
            .collect(),
        Default::default(),
        BirthRecords {
            abilities: relations
                .into_iter()
                .enumerate()
                .map(|(i, (a, b))| f::ability(i as u32 + 1, a, b))
                .collect(),
            ..Default::default()
        },
    )
    .with_spell_ranks()
    .unwrap()
}
#[test]
fn source_chain_endpoints_regular_identity_and_rank_queries_are_canonical() {
    let s = load(&[1, 2, 3, 9], vec![(1, 2), (2, 3)]);
    assert_eq!(
        (
            s.first_spell_in_chain(3),
            s.last_spell_in_chain(1),
            s.next_spell_in_chain(1),
            s.previous_spell_in_chain(3)
        ),
        (1, 3, 2, 2)
    );
    assert_eq!(
        (s.spell_rank(1), s.spell_rank(2), s.spell_rank(3)),
        (1, 2, 3)
    );
    assert_eq!(s.spell_rank_node(2).unwrap().first, (1, 0));
    assert_eq!(s.spell_with_rank(1, 3, true), Ok(3));
    assert_eq!(s.spell_with_rank(3, 1, false), Ok(1));
    assert_eq!(s.spell_with_rank(9, 2, true), Ok(0));
    assert_eq!(s.spell_with_rank(9, 2, false), Ok(9));
    assert_eq!(
        (
            s.first_spell_in_chain(9),
            s.last_spell_in_chain(9),
            s.spell_rank(9)
        ),
        (9, 9, 0)
    );
    assert_eq!(
        s.spell_with_rank(3, 4, false),
        Err(SpellRankError::UndefinedRankTraversal)
    );
    assert_eq!(
        s.spell_with_rank(1, 0, true),
        Err(SpellRankError::UndefinedRankTraversal)
    );
    assert!(matches!(
        s.with_spell_ranks(),
        Err(SpellRankError::AlreadyApplied)
    ));
}
#[test]
fn storage_order_overwrites_link_but_preserves_old_has_previous_membership() {
    let s = load(&[1, 2, 3, 4], vec![(1, 2), (1, 3), (2, 4)]);
    assert_eq!(s.next_spell_in_chain(1), 3);
    assert!(s.spell_rank_node(2).is_none());
    assert!(s.spell_rank_node(4).is_none());
    assert_eq!(
        (
            s.spell_rank_counts().unwrap().overwritten_links,
            s.spell_rank_counts().unwrap().roots
        ),
        (1, 1)
    );
    // Converging roots retain source's mutable shared-node overwrite, not a
    // normalized graph in which old roots must agree with every descendant.
    let s = load(&[1, 2, 3, 4], vec![(1, 3), (2, 3), (3, 4)]);
    assert_eq!(s.first_spell_in_chain(1), 1);
    assert_eq!(s.first_spell_in_chain(3), 2);
    assert_eq!(s.spell_rank_node(3).unwrap().previous, Some((2, 0)));
    assert_eq!(s.last_spell_in_chain(1), 4);
}
#[test]
fn rootless_cycles_are_source_noops_but_reachable_infinite_walk_is_rejected() {
    assert_eq!(
        load(&[1, 2], vec![(1, 2), (2, 1)])
            .spell_rank_counts()
            .unwrap()
            .nodes,
        0
    );
    let s = f::seeds(
        (1..=3).map(|id| ((id, 0), f::definition(vec![]))).collect(),
        Default::default(),
        BirthRecords {
            abilities: vec![
                f::ability(1, 1, 2),
                f::ability(2, 2, 3),
                f::ability(3, 3, 2),
            ],
            ..Default::default()
        },
    );
    assert!(matches!(
        s.with_spell_ranks(),
        Err(SpellRankError::ReachableCycle)
    ));
}
#[test]
fn uint8_rank_wrap_and_signed_ability_id_bits_are_not_clamped() {
    let ids: Vec<_> = (1..=257).collect();
    let s = load(&ids, (1..257).map(|i| (i, i + 1)).collect());
    assert_eq!(
        (s.spell_rank(255), s.spell_rank(256), s.spell_rank(257)),
        (255, 0, 1)
    );
    let s = load(&[u32::MAX, u32::MAX - 1], vec![(u32::MAX, u32::MAX - 1)]);
    assert_eq!(s.first_spell_in_chain(u32::MAX - 1), u32::MAX);
}
#[test]
fn missing_relations_uncertainty_and_phase_guards_never_create_definitions() {
    let mut s = f::seeds(
        vec![],
        Default::default(),
        BirthRecords {
            abilities: vec![f::ability(1, 1, 2)],
            unknown_ability_records: 17,
            ..Default::default()
        },
    );
    s.target_caps = None;
    assert!(matches!(
        s.with_spell_ranks(),
        Err(SpellRankError::RequiresTargetCaps)
    ));
    let mut s = f::seeds(vec![], Default::default(), Default::default());
    s.skill_line_abilities = None;
    assert!(matches!(
        s.with_spell_ranks(),
        Err(SpellRankError::MissingSkillAbilityMap)
    ));
    let s = f::seeds(
        vec![],
        Default::default(),
        BirthRecords {
            abilities: vec![f::ability(1, 1, 2)],
            unknown_ability_records: 17,
            ..Default::default()
        },
    )
    .with_spell_ranks()
    .unwrap();
    assert!(s.is_empty());
    assert_eq!(
        (
            s.spell_rank_counts().unwrap().skipped_missing_definitions,
            s.spell_rank_counts()
                .unwrap()
                .unavailable_baseline_abilities
        ),
        (1, 17)
    );
}
