//! Quest and recruit-a-friend reputation scenarios.

use super::*;

#[test]
fn quest_reputation_gain_respects_no_quest_bonus_aura_gate_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.visible_auras.insert(
        1,
        reputation_aura_for_test(1, RepresentedAuraEffectLikeCpp::ModReputationGain, 25, None),
    );

    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            false,
        ),
        125
    );
    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            true,
        ),
        100
    );
}
#[test]
fn reputation_gain_applies_recruit_a_friend_bonus_for_non_spell_sources_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1);
    let recruit_guid = ObjectGuid::create_player(1, 2);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_recruiter_id_like_cpp(2);
    session.set_reputation_rates_like_cpp(ReputationRatesLikeCpp {
        recruit_a_friend_bonus: 0.1,
        recruit_a_friend_distance: 100.0,
        ..ReputationRatesLikeCpp::default()
    });

    let (recruit_tx, _recruit_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let mut recruit_info = broadcast_info(recruit_guid, recruit_tx);
    recruit_info.placement.map_id = 571;
    recruit_info.placement.position = Position::new(25.0, 0.0, 0.0, 0.0);
    recruit_info.identity.account_id = 2;
    recruit_info.identity.recruiter_id = 0;
    player_registry.register_or_replace(recruit_guid, recruit_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(recruit_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_state(SessionState::LoggedIn);

    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Quest,
            80,
            100,
            7,
            false,
        ),
        110
    );
    assert_eq!(
        session.calculate_reputation_gain_like_cpp(
            ReputationGainSourceLikeCpp::Spell,
            80,
            100,
            7,
            false,
        ),
        100,
        "C++ skips Recruit-A-Friend for REPUTATION_SOURCE_SPELL"
    );
}
