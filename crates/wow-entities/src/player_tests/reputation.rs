//! #735 — the canonical Player owns its reputation state and its invariants.
//!
//! C++ `Player` holds `ReputationMgr m_reputationMgr` (`Player.h:3116`) and the
//! manager writes those members directly. These pin the state-changing
//! invariants that belong with the Player, independently of the catalog-driven
//! rules that live in `wow-world`.

use super::*;

use wow_constants::reputation::{
    REPUTATION_BOTTOM_LIKE_CPP, REPUTATION_CAP_LIKE_CPP, ReputationFlagsLikeCpp,
    ReputationRankLikeCpp,
};

use crate::{PlayerFactionStateLikeCpp, ReputationRankCounterLikeCpp};

/// One visible Alliance standing, reused by the persistence round trip.
pub(super) fn alliance_reputation_state_like_cpp() -> crate::PlayerReputationStateLikeCpp {
    let mut reputation = crate::PlayerReputationStateLikeCpp::default();
    reputation.insert_faction_like_cpp(PlayerFactionStateLikeCpp {
        faction_id: super::TEAM_ALLIANCE_ID,
        standing: 4_200,
        flags: ReputationFlagsLikeCpp::VISIBLE,
        ..Default::default()
    });
    reputation
}

fn seeded_faction_like_cpp() -> PlayerFactionStateLikeCpp {
    PlayerFactionStateLikeCpp::new_like_cpp(72, 4, ReputationFlagsLikeCpp::VISIBLE)
}

#[test]
fn seeded_faction_state_matches_cpp_initialize_shape() {
    let state = seeded_faction_like_cpp();

    assert_eq!(state.faction_id, 72);
    assert_eq!(state.reputation_list_id, 4);
    assert_eq!(state.standing, 0);
    assert_eq!(state.visual_standing_increase, 0);
    assert_eq!(state.flags, ReputationFlagsLikeCpp::VISIBLE);
    // C++ marks a freshly initialized faction for both the client and the DB.
    assert!(state.need_send);
    assert!(state.need_save);
}

#[test]
fn faction_states_are_keyed_by_reputation_list_id_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();

    reputation.insert_faction_like_cpp(PlayerFactionStateLikeCpp::new_like_cpp(
        72,
        4,
        ReputationFlagsLikeCpp::VISIBLE,
    ));
    reputation.insert_faction_like_cpp(PlayerFactionStateLikeCpp::new_like_cpp(
        76,
        4,
        ReputationFlagsLikeCpp::NONE,
    ));

    // C++ `FactionStateList` is a map on `ReputationListID` (`ReputationMgr.h:63`):
    // the second faction replaces the first rather than shadowing it.
    assert_eq!(reputation.faction_count_like_cpp(), 1);
    assert_eq!(
        reputation.faction_like_cpp(4).map(|state| state.faction_id),
        Some(76)
    );
}

#[test]
fn faction_states_iterate_in_reputation_list_id_order_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();
    for (faction_id, list_id) in [(76_u32, 9_u32), (72, 4), (80, 6)] {
        reputation.insert_faction_like_cpp(PlayerFactionStateLikeCpp::new_like_cpp(
            faction_id,
            list_id,
            ReputationFlagsLikeCpp::NONE,
        ));
    }

    let order: Vec<_> = player
        .reputation_like_cpp()
        .factions_like_cpp()
        .map(|state| state.reputation_list_id)
        .collect();

    assert_eq!(order, vec![4, 6, 9]);
}

#[test]
fn standing_is_clamped_into_the_cpp_reputation_range() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();
    reputation.insert_faction_like_cpp(seeded_faction_like_cpp());

    assert!(reputation.set_standing_like_cpp(4, REPUTATION_CAP_LIKE_CPP + 5_000));
    assert_eq!(
        reputation.faction_like_cpp(4).unwrap().standing,
        REPUTATION_CAP_LIKE_CPP
    );

    assert!(reputation.set_standing_like_cpp(4, REPUTATION_BOTTOM_LIKE_CPP - 5_000));
    assert_eq!(
        reputation.faction_like_cpp(4).unwrap().standing,
        REPUTATION_BOTTOM_LIKE_CPP
    );

    // An unknown faction is not created by a standing write.
    assert!(!reputation.set_standing_like_cpp(9, 100));
    assert_eq!(reputation.faction_count_like_cpp(), 1);
}

#[test]
fn clearing_factions_resets_counters_and_the_pending_visual_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();
    reputation.insert_faction_like_cpp(seeded_faction_like_cpp());
    reputation.adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Exalted, 1);
    reputation.set_send_faction_increased_like_cpp(true);

    // C++ `ReputationMgr::Initialize` clears the list and both counters before
    // reseeding, so a stale rank count cannot survive a re-initialization.
    reputation.clear_factions_like_cpp();

    assert_eq!(reputation.faction_count_like_cpp(), 0);
    assert_eq!(reputation.rank_counters_like_cpp().exalted, 0);
    assert!(!reputation.send_faction_increased_like_cpp());
    // Forced reactions are a separate C++ member and survive re-initialization.
}

#[test]
fn rank_counters_never_wrap_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();

    reputation.adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Honored, -1);
    assert_eq!(reputation.rank_counters_like_cpp().honored, 0);

    for _ in 0..u16::from(u8::MAX) + 2 {
        reputation.adjust_rank_counter_like_cpp(ReputationRankCounterLikeCpp::Revered, 1);
    }
    assert_eq!(reputation.rank_counters_like_cpp().revered, u8::MAX);
}

#[test]
fn forced_reactions_insert_replace_and_erase_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();

    reputation.set_forced_reaction_like_cpp(72, Some(ReputationRankLikeCpp::Hostile));
    assert_eq!(
        reputation.forced_reaction_like_cpp(72),
        Some(ReputationRankLikeCpp::Hostile)
    );

    reputation.set_forced_reaction_like_cpp(72, Some(ReputationRankLikeCpp::Friendly));
    assert_eq!(
        reputation.forced_reaction_like_cpp(72),
        Some(ReputationRankLikeCpp::Friendly)
    );

    reputation.set_forced_reaction_like_cpp(72, None);
    assert!(reputation.forced_reaction_like_cpp(72).is_none());
}

#[test]
fn player_forced_rank_helpers_keep_an_existing_rank_like_cpp() {
    let mut player = Player::new(Some(1), false);
    player
        .reputation_mut_like_cpp()
        .set_forced_reaction_like_cpp(87, Some(ReputationRankLikeCpp::Exalted));

    // C++ `ApplyForceReaction` does not downgrade an existing forced rank when
    // the same faction is forced again.
    player.set_forced_reputation_rank_like_cpp(87, true);
    assert_eq!(
        player.reputation_like_cpp().forced_reaction_like_cpp(87),
        Some(ReputationRankLikeCpp::Exalted)
    );
    assert!(player.has_forced_reputation_rank_like_cpp(87));

    player.set_forced_reputation_rank_like_cpp(87, false);
    assert!(!player.has_forced_reputation_rank_like_cpp(87));
}

#[test]
fn at_war_and_presence_reads_follow_the_owned_faction_flags_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let mut hostile =
        PlayerFactionStateLikeCpp::new_like_cpp(72, 4, ReputationFlagsLikeCpp::AT_WAR);
    hostile.standing = -6_000;
    player
        .reputation_mut_like_cpp()
        .insert_faction_like_cpp(hostile);

    assert!(player.is_at_war_with_faction_like_cpp(72));
    assert!(player.has_reputation_state_like_cpp(72));
    assert!(!player.is_at_war_with_faction_like_cpp(76));
    assert!(!player.has_reputation_state_like_cpp(76));
}

#[test]
fn send_and_save_flags_are_independent_like_cpp() {
    let mut player = Player::new(Some(1), false);
    let reputation = player.reputation_mut_like_cpp();
    reputation.insert_faction_like_cpp(seeded_faction_like_cpp());

    let faction = reputation.faction_mut_like_cpp(4).unwrap();
    faction.need_save = false;
    assert!(faction.need_send);

    let faction = reputation.faction_mut_like_cpp(4).unwrap();
    faction.need_send = false;
    faction.need_save = true;
    assert!(!reputation.faction_like_cpp(4).unwrap().need_send);
    assert!(reputation.faction_like_cpp(4).unwrap().need_save);
}
