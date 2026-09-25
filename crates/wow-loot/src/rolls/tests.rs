use super::*;

fn votes(rows: &[(i64, u8, u8)]) -> HashMap<ObjectGuid, RepresentedLootRollVote> {
    rows.iter()
        .map(|&(player, vote, roll_number)| {
            (
                ObjectGuid::create_player(1, player),
                RepresentedLootRollVote { vote, roll_number },
            )
        })
        .collect()
}

#[test]
fn need_beats_higher_greed_and_disenchant_without_mutating_votes() {
    let voters = votes(&[
        (1, ROLL_VOTE_GREED_LIKE_CPP, 100),
        (2, ROLL_VOTE_NEED_LIKE_CPP, 1),
        (3, ROLL_VOTE_DISENCHANT_LIKE_CPP, 100),
    ]);
    let before = voters.clone();
    let winner = (
        ObjectGuid::create_player(1, 2),
        voters[&ObjectGuid::create_player(1, 2)],
    );
    assert_eq!(
        represented_loot_roll_current_winner_like_cpp(&voters),
        Some(winner)
    );
    assert_eq!(
        represented_loot_roll_finish_winner_like_cpp(&voters),
        Some(Some(winner))
    );
    assert_eq!(voters, before);
}

#[test]
fn final_selection_waits_for_pending_votes_but_timeout_selection_does_not() {
    let voters = votes(&[
        (1, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP, 0),
        (2, ROLL_VOTE_NEED_LIKE_CPP, 50),
    ]);
    assert_eq!(represented_loot_roll_finish_winner_like_cpp(&voters), None);
    assert_eq!(
        represented_loot_roll_current_winner_like_cpp(&voters).map(|(player, _)| player),
        Some(ObjectGuid::create_player(1, 2)),
    );
}

#[test]
fn pass_invalid_unknown_and_empty_votes_produce_no_winner() {
    for voters in [
        HashMap::new(),
        votes(&[
            (1, ROLL_VOTE_PASS_LIKE_CPP, 100),
            (2, ROLL_VOTE_NOT_VALID_LIKE_CPP, 100),
            (3, 99, 100),
        ]),
    ] {
        assert_eq!(
            represented_loot_roll_finish_winner_like_cpp(&voters),
            Some(None)
        );
        assert_eq!(represented_loot_roll_current_winner_like_cpp(&voters), None);
    }
}

#[test]
fn greed_and_disenchant_share_priority_and_use_the_highest_roll() {
    let voters = votes(&[
        (1, ROLL_VOTE_GREED_LIKE_CPP, 49),
        (2, ROLL_VOTE_DISENCHANT_LIKE_CPP, 50),
    ]);
    assert_eq!(
        represented_loot_roll_finish_winner_like_cpp(&voters)
            .flatten()
            .map(|(player, _)| player),
        Some(ObjectGuid::create_player(1, 2)),
    );
}

#[test]
fn tied_rolls_preserve_the_same_maps_iteration_order() {
    let voters = votes(&[
        (1, ROLL_VOTE_NEED_LIKE_CPP, 50),
        (2, ROLL_VOTE_NEED_LIKE_CPP, 50),
    ]);
    let expected = voters.iter().next().map(|(player, vote)| (*player, *vote));
    assert_eq!(
        represented_loot_roll_finish_winner_like_cpp(&voters),
        Some(expected)
    );
    assert_eq!(
        represented_loot_roll_current_winner_like_cpp(&voters),
        expected
    );
}

#[test]
fn roll_mask_preserves_greed_only_and_enchanting_skill_gates() {
    let greed_only = ItemFlags2::CanOnlyRollGreed as u32;
    assert_eq!(
        represented_loot_roll_valid_rolls_like_cpp(None, Some(75), 75),
        0x0f
    );
    assert_eq!(
        represented_loot_roll_valid_rolls_like_cpp(None, Some(75), 74),
        0x07
    );
    assert_eq!(
        represented_loot_roll_valid_rolls_like_cpp(None, None, u16::MAX),
        0x07
    );
    assert_eq!(
        represented_loot_roll_valid_rolls_like_cpp(Some(greed_only), Some(75), 75),
        0x0d
    );
    assert_eq!(
        represented_loot_roll_valid_rolls_like_cpp(Some(greed_only), Some(75), 74),
        0x05
    );
}
