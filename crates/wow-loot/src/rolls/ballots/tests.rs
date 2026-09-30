use super::*;
use crate::LootEntryFlags;
use std::cell::Cell;

fn player(id: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, id)
}

fn entry(allowed_looters: Vec<ObjectGuid>) -> LootEntry {
    LootEntry {
        loot_list_id: 7,
        item_id: 25,
        quantity: 2,
        random_properties_id: -3,
        random_properties_seed: 4,
        item_context: 5,
        flags: LootEntryFlags {
            follow_loot_rules: true,
            freeforall: true,
            blocked: true,
            counted: true,
            under_threshold: false,
            needs_quest: true,
        },
        allowed_looters,
        roll_winner: player(9),
        ffa_looted_by: vec![player(8)],
        taken: true,
    }
}

fn two_voters() -> RollBallots {
    RollBallots::start(
        &mut entry(vec![player(1), player(2)]),
        2,
        player(1),
        false,
        |_| Some(false),
    )
    .unwrap()
}

#[test]
fn zero_or_one_connected_looter_unblocks_before_any_pass_lookup() {
    for count in [0, 1] {
        let mut item = entry(vec![player(1), player(2), player(2)]);
        assert!(RollBallots::start(&mut item, count, player(1), false, |_| {
            panic!("under-threshold entries must not query pass preferences")
        })
        .is_none());
        assert!(!item.flags.blocked);
        assert!(item.flags.under_threshold);
        assert!(item.flags.follow_loot_rules);
        assert!(item.flags.freeforall);
        assert!(item.flags.counted);
        assert!(item.flags.needs_quest);
        assert_eq!(item.allowed_looters, vec![player(1), player(2), player(2)]);
        assert_eq!(item.roll_winner, player(9));
        assert_eq!(item.ffa_looted_by, vec![player(8)]);
        assert!(item.taken);
        assert_eq!(
            (
                item.loot_list_id,
                item.item_id,
                item.quantity,
                item.random_properties_id,
                item.random_properties_seed,
                item.item_context,
            ),
            (7, 25, 2, -3, 4, 5),
        );
    }
}

#[test]
fn duplicate_looters_repeat_queries_in_order_and_last_insert_wins() {
    let current = player(1);
    let remote = player(2);
    let missing = player(3);
    let mut item = entry(vec![
        current, remote, missing, remote, current, ObjectGuid::EMPTY,
    ]);
    let mut queried = Vec::new();
    let ballots = RollBallots::start(&mut item, 2, current, true, |guid| {
        queried.push(guid);
        match queried.len() {
            1 => Some(false),
            3 => Some(true),
            _ => None,
        }
    })
    .unwrap();
    assert_eq!(queried, vec![remote, missing, remote, ObjectGuid::EMPTY]);
    assert_eq!(ballots.votes().count(), 4);
    assert_eq!(ballots.vote(current).unwrap().vote, ROLL_VOTE_PASS_LIKE_CPP);
    assert_eq!(ballots.vote(remote).unwrap().vote, ROLL_VOTE_PASS_LIKE_CPP);
    assert_eq!(ballots.vote(missing).unwrap().vote, ROLL_VOTE_NOT_VALID_LIKE_CPP);
    assert_eq!(
        ballots.vote(ObjectGuid::EMPTY).unwrap().vote,
        ROLL_VOTE_NOT_VALID_LIKE_CPP,
    );
    assert!(ballots.votes().all(|(_, vote)| vote.roll_number == 0));
    assert_eq!(
        item.allowed_looters,
        vec![current, remote, missing, remote, current, ObjectGuid::EMPTY],
    );
    assert!(item.flags.blocked);
    assert!(!item.flags.under_threshold);
    assert_eq!(item.roll_winner, player(9));
}

#[test]
fn pass_and_unknown_votes_never_draw() {
    assert_eq!(
        RollBallots::prepare_vote(ROLL_VOTE_PASS_LIKE_CPP, || panic!("Pass must not draw")),
        Some(None),
    );
    for unknown in [
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, ROLL_VOTE_NOT_VALID_LIKE_CPP, 99,
    ] {
        assert_eq!(
            RollBallots::prepare_vote(unknown, || panic!("unknown vote must not draw")),
            None,
        );
    }
}

#[test]
fn each_known_numbered_vote_draws_once_with_the_callers_value() {
    let draws = Cell::new(0);
    for vote in [
        ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_DISENCHANT_LIKE_CPP,
    ] {
        let consumed = String::from("one-shot draw");
        assert_eq!(
            RollBallots::prepare_vote(vote, || {
                drop(consumed);
                draws.set(draws.get() + 1);
                73
            }),
            Some(Some(73)),
        );
    }
    assert_eq!(draws.get(), 3);
}

#[test]
fn draws_happen_before_missing_membership_rejection() {
    let draws = Cell::new(0);
    let mut ballots = two_voters();
    let number = RollBallots::prepare_vote(ROLL_VOTE_GREED_LIKE_CPP, || {
        draws.set(draws.get() + 1);
        42
    })
    .unwrap();
    assert!(!ballots.record_vote(player(99), ROLL_VOTE_GREED_LIKE_CPP, number));
    assert_eq!(draws.get(), 1);
    assert!(ballots.vote(player(99)).is_none());
    assert!(ballots.votes().all(|(_, vote)| {
        vote.vote == ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP && vote.roll_number == 0
    }));
}

#[test]
fn pass_after_need_keeps_the_previous_number_and_removes_winning_eligibility() {
    let mut ballots = two_voters();
    let number = RollBallots::prepare_vote(ROLL_VOTE_NEED_LIKE_CPP, || 87).unwrap();
    assert!(ballots.record_vote(player(1), ROLL_VOTE_NEED_LIKE_CPP, number));
    assert_eq!(ballots.current_winner().unwrap().0, player(1));
    let number = RollBallots::prepare_vote(ROLL_VOTE_PASS_LIKE_CPP, || panic!("Pass draw"))
        .unwrap();
    assert!(ballots.record_vote(player(1), ROLL_VOTE_PASS_LIKE_CPP, number));
    assert_eq!(
        ballots.vote(player(1)),
        Some(&RepresentedLootRollVote {
            vote: ROLL_VOTE_PASS_LIKE_CPP,
            roll_number: 87,
        }),
    );
    assert_eq!(ballots.current_winner(), None);
    assert_eq!(ballots.finished_winner(), None);
    assert!(ballots.record_vote(player(2), ROLL_VOTE_PASS_LIKE_CPP, None));
    assert_eq!(ballots.finished_winner(), Some(None));
}

#[test]
fn repeated_votes_redraw_and_overwrite_without_prior_vote_guards() {
    let draws = Cell::new(0);
    let mut ballots = two_voters();
    for (vote, number) in [
        (ROLL_VOTE_NEED_LIKE_CPP, 100),
        (ROLL_VOTE_NEED_LIKE_CPP, 1),
        (ROLL_VOTE_GREED_LIKE_CPP, 7),
        (ROLL_VOTE_DISENCHANT_LIKE_CPP, 6),
    ] {
        let prepared = RollBallots::prepare_vote(vote, || {
            draws.set(draws.get() + 1);
            number
        })
        .unwrap();
        assert!(ballots.record_vote(player(1), vote, prepared));
        assert_eq!(
            ballots.vote(player(1)),
            Some(&RepresentedLootRollVote { vote, roll_number: number }),
        );
    }
    assert_eq!(draws.get(), 4);
    assert_eq!(
        ballots.vote(player(1)).unwrap().vote,
        ROLL_VOTE_DISENCHANT_LIKE_CPP,
    );
}

#[test]
fn formerly_invalid_voter_can_vote_and_unknown_preparation_does_not_mutate() {
    let mut ballots = RollBallots::start(
        &mut entry(vec![player(1), player(2)]),
        2,
        player(1),
        false,
        |_| None,
    )
    .unwrap();
    assert_eq!(
        ballots.vote(player(2)).unwrap().vote,
        ROLL_VOTE_NOT_VALID_LIKE_CPP,
    );
    let prepared = RollBallots::prepare_vote(ROLL_VOTE_NEED_LIKE_CPP, || 19).unwrap();
    assert!(ballots.record_vote(player(2), ROLL_VOTE_NEED_LIKE_CPP, prepared));
    if let Some(number) = RollBallots::prepare_vote(99, || panic!("unknown draw")) {
        ballots.record_vote(player(2), 99, number);
    }
    assert_eq!(
        ballots.vote(player(2)),
        Some(&RepresentedLootRollVote {
            vote: ROLL_VOTE_NEED_LIKE_CPP,
            roll_number: 19,
        }),
    );
    assert_eq!(ballots.votes().count(), 2);
}

#[test]
fn ties_follow_borrowed_iteration_and_timeout_ignores_pending_without_mutation() {
    let mut ballots = RollBallots::start(
        &mut entry(vec![player(1), player(2), player(3)]),
        3,
        player(1),
        false,
        |_| Some(false),
    )
    .unwrap();
    assert!(ballots.record_vote(player(1), ROLL_VOTE_NEED_LIKE_CPP, Some(50)));
    assert!(ballots.record_vote(player(2), ROLL_VOTE_NEED_LIKE_CPP, Some(50)));
    let first_need = ballots.votes()
        .find(|(_, vote)| vote.vote == ROLL_VOTE_NEED_LIKE_CPP)
        .map(|(guid, vote)| (*guid, *vote));
    assert_eq!(ballots.finished_winner(), None);
    assert_eq!(ballots.current_winner(), first_need);
    assert_eq!(
        ballots.vote(player(3)).unwrap().vote,
        ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP,
    );
    assert!(ballots.record_vote(player(3), ROLL_VOTE_GREED_LIKE_CPP, Some(100)));
    assert_eq!(ballots.finished_winner(), Some(first_need));
    assert_eq!(ballots.current_winner(), first_need);
    let before = ballots.votes()
        .map(|(guid, vote)| (*guid, *vote))
        .collect::<Vec<_>>();
    let snapshot = ballots.clone();
    assert_eq!(
        snapshot.votes().map(|(guid, vote)| (*guid, *vote)).collect::<Vec<_>>(),
        before,
    );
    assert_eq!(snapshot.finished_winner(), ballots.finished_winner());
}
