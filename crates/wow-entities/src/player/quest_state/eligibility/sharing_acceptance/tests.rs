use super::PlayerQuestGameplayState;
use std::cell::RefCell;
use std::collections::HashMap;
use wow_constants::quest::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
use wow_data_model::quest::QuestEligibilityRules;

fn rules(
    id: u32,
    group: i32,
    repeatable: bool,
    daily: bool,
    dungeon_finder: bool,
) -> QuestEligibilityRules<'static> {
    QuestEligibilityRules::new(
        id,
        repeatable,
        group,
        0,
        &[],
        &[],
        daily,
        dungeon_finder,
        false,
        false,
        false,
        0,
    )
}

#[test]
fn candidate_status_is_read_before_unconditional_rewarded_rejection() {
    for repeatable in [false, true] {
        let trace = RefCell::new(Vec::new());
        let quest = rules(1, 5, repeatable, false, false);
        let allowed = PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || -> [QuestEligibilityRules<'static>; 0] { panic!("no peers on rejection") },
            |id| {
                trace.borrow_mut().push(("status", id));
                None
            },
            |id| {
                trace.borrow_mut().push(("rewarded", id));
                true
            },
            |_| panic!("no DF lookup on candidate rejection"),
            |_| panic!("no daily lookup on candidate rejection"),
        );
        assert!(!allowed);
        assert_eq!(*trace.borrow(), [("status", 1), ("rewarded", 1)]);
    }
}

#[test]
fn missing_and_stored_none_candidate_allow_without_peer_enumeration() {
    for status in [None, Some(QUEST_STATUS_NONE_LIKE_CPP)] {
        let quest = rules(1, 0, false, false, false);
        assert!(
            PlayerQuestGameplayState::sharing_acceptance_after_expansion(
                &quest,
                || -> [QuestEligibilityRules<'static>; 0] { panic!("nonpositive group") },
                |id| {
                    assert_eq!(id, 1);
                    status
                },
                |id| {
                    assert_eq!(id, 1);
                    false
                },
                |_| panic!("no DF lookup"),
                |_| panic!("no daily lookup"),
            )
        );
    }
}

#[test]
fn every_non_none_candidate_status_blocks_after_rewarded_lookup() {
    for status in [
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        QUEST_STATUS_FAILED_LIKE_CPP,
        QUEST_STATUS_REWARDED_LIKE_CPP,
        0xFE,
    ] {
        let trace = RefCell::new(Vec::new());
        let quest = rules(1, 5, false, false, false);
        assert!(
            !PlayerQuestGameplayState::sharing_acceptance_after_expansion(
                &quest,
                || -> [QuestEligibilityRules<'static>; 0] { panic!("blocked candidate") },
                |id| {
                    trace.borrow_mut().push(("status", id));
                    Some(status)
                },
                |id| {
                    trace.borrow_mut().push(("rewarded", id));
                    false
                },
                |_| panic!("no DF lookup"),
                |_| panic!("no daily lookup"),
            )
        );
        assert_eq!(*trace.borrow(), [("status", 1), ("rewarded", 1)]);
    }
}

#[test]
fn nonpositive_group_never_enumerates_peers() {
    for group in [0, -1, i32::MIN] {
        let quest = rules(1, group, false, false, false);
        assert!(
            PlayerQuestGameplayState::sharing_acceptance_after_expansion(
                &quest,
                || -> [QuestEligibilityRules<'static>; 0] { panic!("nonpositive group") },
                |_| None,
                |_| false,
                |_| panic!("no DF lookup"),
                |_| panic!("no daily lookup"),
            )
        );
    }
}

#[test]
fn positive_group_filters_peers_and_skips_self_before_membership() {
    let trace = RefCell::new(Vec::new());
    let quest = rules(1, 5, false, false, false);
    let peers = [
        rules(2, 9, false, true, true),
        rules(1, 5, false, true, true),
        rules(3, 5, false, false, false),
    ];
    assert!(
        PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || {
                trace.borrow_mut().push(("peers", 0));
                peers
            },
            |id| {
                trace.borrow_mut().push(("status", id));
                None
            },
            |id| {
                trace.borrow_mut().push(("rewarded", id));
                false
            },
            |_| panic!("unmatched/self peers must not read DF"),
            |_| panic!("unmatched/self peers must not read daily"),
        )
    );
    assert_eq!(
        *trace.borrow(),
        [
            ("status", 1),
            ("rewarded", 1),
            ("peers", 0),
            ("status", 3),
            ("rewarded", 3),
        ]
    );
}

#[test]
fn dungeon_finder_block_precedes_daily_and_peer_status() {
    let trace = RefCell::new(Vec::new());
    let quest = rules(1, 5, false, false, false);
    assert!(
        !PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || [rules(2, 5, false, true, true)],
            |id| {
                assert_eq!(id, 1);
                None
            },
            |id| {
                assert_eq!(id, 1);
                false
            },
            |id| {
                trace.borrow_mut().push(("DF", id));
                true
            },
            |_| panic!("DF block must stop before daily"),
        )
    );
    assert_eq!(*trace.borrow(), [("DF", 2)]);
}

#[test]
fn daily_check_remains_independent_when_df_membership_is_absent() {
    let trace = RefCell::new(Vec::new());
    let quest = rules(1, 5, false, false, false);
    assert!(
        !PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || [rules(2, 5, false, true, true)],
            |id| {
                assert_eq!(id, 1);
                None
            },
            |id| {
                assert_eq!(id, 1);
                false
            },
            |id| {
                trace.borrow_mut().push(("DF", id));
                false
            },
            |id| {
                trace.borrow_mut().push(("daily", id));
                true
            },
        )
    );
    assert_eq!(*trace.borrow(), [("DF", 2), ("daily", 2)]);
}

#[test]
fn peer_status_none_allows_and_non_none_blocks_before_rewarded() {
    for status in [None, Some(QUEST_STATUS_NONE_LIKE_CPP), Some(0xFE)] {
        let trace = RefCell::new(Vec::new());
        let quest = rules(1, 5, false, false, false);
        let allowed = PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || [rules(2, 5, false, false, false)],
            |id| {
                trace.borrow_mut().push(("status", id));
                if id == 2 { status } else { None }
            },
            |id| {
                trace.borrow_mut().push(("rewarded", id));
                false
            },
            |_| panic!("non-DF peer"),
            |_| panic!("non-daily peer"),
        );
        assert_eq!(
            allowed,
            status.unwrap_or(QUEST_STATUS_NONE_LIKE_CPP) == QUEST_STATUS_NONE_LIKE_CPP
        );
        let mut expected = vec![("status", 1), ("rewarded", 1), ("status", 2)];
        if allowed {
            expected.push(("rewarded", 2));
        }
        assert_eq!(*trace.borrow(), expected);
    }
}

#[test]
fn both_repeatable_peers_skip_rewarded_lookup_other_combinations_block() {
    for (candidate_repeatable, peer_repeatable) in
        [(false, false), (false, true), (true, false), (true, true)]
    {
        let trace = RefCell::new(Vec::new());
        let quest = rules(1, 5, candidate_repeatable, false, false);
        let allowed = PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || [rules(2, 5, peer_repeatable, false, false)],
            |_| None,
            |id| {
                trace.borrow_mut().push(id);
                id == 2
            },
            |_| panic!("non-DF peer"),
            |_| panic!("non-daily peer"),
        );
        assert_eq!(allowed, candidate_repeatable && peer_repeatable);
        assert_eq!(*trace.borrow(), if allowed { vec![1] } else { vec![1, 2] });
    }
}

#[test]
fn first_blocking_peer_stops_iterator_before_later_rows() {
    let visited = RefCell::new(Vec::new());
    let quest = rules(1, 5, false, false, false);
    let peers = [
        rules(2, 5, false, false, false),
        rules(3, 5, false, false, false),
    ];
    assert!(
        !PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || peers
                .into_iter()
                .inspect(|peer| visited.borrow_mut().push(peer.id())),
            |id| (id == 2).then_some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
            |id| {
                assert_eq!(id, 1);
                false
            },
            |_| panic!("non-DF peer"),
            |_| panic!("non-daily peer"),
        )
    );
    assert_eq!(*visited.borrow(), [2]);
}

#[test]
fn raw_hashmap_peer_iteration_order_is_preserved() {
    let store = HashMap::from([
        (3, rules(3, 5, false, false, false)),
        (1, rules(1, 5, false, false, false)),
        (2, rules(2, 5, false, false, false)),
    ]);
    let expected = store.values().map(|peer| peer.id()).collect::<Vec<_>>();
    let visited = RefCell::new(Vec::new());
    let quest = rules(1, 5, false, false, false);
    assert!(
        PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || store
                .values()
                .copied()
                .inspect(|peer| visited.borrow_mut().push(peer.id())),
            |_| None,
            |_| false,
            |_| panic!("non-DF peer"),
            |_| panic!("non-daily peer"),
        )
    );
    assert_eq!(*visited.borrow(), expected);
}

#[test]
fn weekly_monthly_and_seasonal_flags_do_not_add_membership_queries() {
    let trace = RefCell::new(Vec::new());
    let quest = rules(1, 5, false, false, false);
    let peer =
        QuestEligibilityRules::new(2, false, 5, 0, &[], &[], false, false, true, true, true, 9);
    assert!(
        PlayerQuestGameplayState::sharing_acceptance_after_expansion(
            &quest,
            || [peer],
            |id| {
                trace.borrow_mut().push(("status", id));
                None
            },
            |id| {
                trace.borrow_mut().push(("rewarded", id));
                false
            },
            |_| panic!("non-DF peer"),
            |_| panic!("non-daily peer"),
        )
    );
    assert_eq!(
        *trace.borrow(),
        [
            ("status", 1),
            ("rewarded", 1),
            ("status", 2),
            ("rewarded", 2),
        ]
    );
}
