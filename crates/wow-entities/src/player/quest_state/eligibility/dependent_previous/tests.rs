use super::PlayerQuestGameplayState;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use wow_data_model::quest::QuestEligibilityRules;

fn rules(previous_ids: &[u32]) -> QuestEligibilityRules<'_> {
    QuestEligibilityRules::new(
        9957,
        false,
        0,
        0,
        previous_ids,
        &[],
        false,
        false,
        false,
        false,
        false,
        0,
    )
}

fn dependent_previous_blocks(
    store: &HashMap<u32, i32>,
    quest: &QuestEligibilityRules<'_>,
    rewarded: &HashSet<u32>,
) -> bool {
    PlayerQuestGameplayState::dependent_previous_quest_ids_block(
        quest.dependent_previous_quest_ids(),
        |id| store.get(&id).copied(),
        |group| {
            store
                .iter()
                .filter_map(move |(&id, &candidate_group)| (candidate_group == group).then_some(id))
        },
        |id| rewarded.contains(&id),
    )
}

#[test]
fn dependent_previous_catalog_preserves_lazy_lookup_and_negative_group_requirements() {
    let first_id = 9955;
    let peer_id = 9956;
    let missing_id = 9958;
    let store = HashMap::from([(first_id, 0), (peer_id, 0)]);
    let mut rewarded = HashSet::from([first_id]);

    let previous_ids = [first_id, missing_id];
    let quest = rules(&previous_ids);
    assert!(!dependent_previous_blocks(&store, &quest, &rewarded,));
    let previous_ids = [missing_id, first_id];
    let quest = rules(&previous_ids);
    assert!(dependent_previous_blocks(&store, &quest, &rewarded,));

    let store = HashMap::from([(first_id, -12), (peer_id, -12)]);
    let previous_ids = [first_id];
    let quest = rules(&previous_ids);
    assert!(dependent_previous_blocks(&store, &quest, &rewarded,));
    rewarded.insert(peer_id);
    assert!(!dependent_previous_blocks(&store, &quest, &rewarded,));
}

#[derive(Debug, PartialEq, Eq)]
enum Read {
    Lookup(u32),
    Rewarded(u32),
    Group(i32),
    Peer(u32),
}

#[test]
fn empty_dependent_previous_ids_do_not_read_catalog_groups_or_rewards() {
    assert!(
        !PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[],
            |_| -> Option<i32> { panic!("empty prerequisite list must not look up a row") },
            |_| -> std::iter::Empty<u32> {
                panic!("empty prerequisite list must not enumerate a group")
            },
            |_| panic!("empty prerequisite list must not read rewarded membership"),
        )
    );
}

#[test]
fn missing_previous_row_blocks_before_rewarded_membership_or_later_candidates() {
    let reads = RefCell::new(Vec::new());
    assert!(
        PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[10, 11],
            |id| {
                reads.borrow_mut().push(Read::Lookup(id));
                None
            },
            |_| -> std::iter::Empty<u32> { panic!("missing row has no group") },
            |_| panic!("missing row must reject before reading rewarded membership"),
        )
    );
    assert_eq!(*reads.borrow(), [Read::Lookup(10)]);
}

#[test]
fn each_previous_lookup_precedes_rewarded_read_and_unrewarded_advances() {
    let reads = RefCell::new(Vec::new());
    assert!(
        !PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[10, 11, 12],
            |id| {
                reads.borrow_mut().push(Read::Lookup(id));
                Some(0)
            },
            |_| -> std::iter::Empty<u32> { panic!("nonnegative rows do not enumerate groups") },
            |id| {
                reads.borrow_mut().push(Read::Rewarded(id));
                id == 11
            },
        )
    );
    assert_eq!(
        *reads.borrow(),
        [
            Read::Lookup(10),
            Read::Rewarded(10),
            Read::Lookup(11),
            Read::Rewarded(11),
        ],
    );
}

#[test]
fn rewarded_nonnegative_group_allows_before_later_missing_row() {
    for group in [0, 7] {
        let reads = RefCell::new(Vec::new());
        assert!(
            !PlayerQuestGameplayState::dependent_previous_quest_ids_block(
                &[10, 99],
                |id| {
                    reads.borrow_mut().push(Read::Lookup(id));
                    (id == 10).then_some(group)
                },
                |_| -> std::iter::Empty<u32> {
                    panic!("rewarded nonnegative group must not enumerate peers")
                },
                |id| {
                    reads.borrow_mut().push(Read::Rewarded(id));
                    true
                },
            )
        );
        assert_eq!(*reads.borrow(), [Read::Lookup(10), Read::Rewarded(10)]);
    }
}

#[test]
fn all_unrewarded_previous_rows_block_without_enumerating_negative_groups() {
    let reads = RefCell::new(Vec::new());
    assert!(
        PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[10, 11],
            |id| {
                reads.borrow_mut().push(Read::Lookup(id));
                Some(-12)
            },
            |_| -> std::iter::Empty<u32> {
                panic!("unrewarded candidates do not enumerate groups")
            },
            |id| {
                reads.borrow_mut().push(Read::Rewarded(id));
                false
            },
        )
    );
    assert_eq!(
        *reads.borrow(),
        [
            Read::Lookup(10),
            Read::Rewarded(10),
            Read::Lookup(11),
            Read::Rewarded(11),
        ],
    );
}

#[test]
fn negative_group_skips_self_and_first_failed_peer_stops_without_next_alternative() {
    let reads = RefCell::new(Vec::new());
    let trace = &reads;
    let peers = [10, 11, 12];
    assert!(
        PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[10, 99],
            |id| {
                reads.borrow_mut().push(Read::Lookup(id));
                Some(-12)
            },
            |group| {
                reads.borrow_mut().push(Read::Group(group));
                peers.iter().copied().inspect(move |id| {
                    trace.borrow_mut().push(Read::Peer(*id));
                })
            },
            |id| {
                reads.borrow_mut().push(Read::Rewarded(id));
                id == 10
            },
        )
    );
    assert_eq!(
        *reads.borrow(),
        [
            Read::Lookup(10),
            Read::Rewarded(10),
            Read::Group(-12),
            Read::Peer(10),
            Read::Peer(11),
            Read::Rewarded(11),
        ],
    );
}

#[test]
fn negative_group_all_rewarded_allows_without_rechecking_self_or_later_candidates() {
    let reads = RefCell::new(Vec::new());
    let trace = &reads;
    let peers = [10, 11, 12];
    assert!(
        !PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[10, 99],
            |id| {
                reads.borrow_mut().push(Read::Lookup(id));
                Some(-12)
            },
            |group| {
                reads.borrow_mut().push(Read::Group(group));
                peers.iter().copied().inspect(move |id| {
                    trace.borrow_mut().push(Read::Peer(*id));
                })
            },
            |id| {
                reads.borrow_mut().push(Read::Rewarded(id));
                true
            },
        )
    );
    assert_eq!(
        *reads.borrow(),
        [
            Read::Lookup(10),
            Read::Rewarded(10),
            Read::Group(-12),
            Read::Peer(10),
            Read::Peer(11),
            Read::Rewarded(11),
            Read::Peer(12),
            Read::Rewarded(12),
        ],
    );
}

#[test]
fn rewarded_negative_group_with_no_peers_allows() {
    let reads = RefCell::new(Vec::new());
    assert!(
        !PlayerQuestGameplayState::dependent_previous_quest_ids_block(
            &[10, 99],
            |id| {
                reads.borrow_mut().push(Read::Lookup(id));
                Some(-12)
            },
            |group| {
                reads.borrow_mut().push(Read::Group(group));
                std::iter::empty::<u32>()
            },
            |id| {
                reads.borrow_mut().push(Read::Rewarded(id));
                true
            },
        )
    );
    assert_eq!(
        *reads.borrow(),
        [Read::Lookup(10), Read::Rewarded(10), Read::Group(-12)],
    );
}
