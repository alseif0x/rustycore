//! Wiring of quest-slot valuation to the existing catalogs and Player owner.

use super::*;

#[test]
fn quest_reputation_source_preserves_legacy_type_identity_and_priority() {
    assert_eq!(
        std::any::TypeId::of::<RepresentedQuestRewardReputationSourceLikeCpp>(),
        std::any::TypeId::of::<wow_progression::QuestReputationSource>(),
    );
    let monthly = wow_data::quest::QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    let repeatable = wow_data::quest::QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP;
    for (flags, special_flags, expected) in [
        (
            QUEST_FLAGS_DAILY_LIKE_CPP | QUEST_FLAGS_WEEKLY_LIKE_CPP,
            monthly | repeatable,
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
        ),
        (
            QUEST_FLAGS_WEEKLY_LIKE_CPP,
            monthly | repeatable,
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest,
        ),
        (
            0,
            monthly | repeatable,
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest,
        ),
        (
            0,
            repeatable,
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest,
        ),
        (0, 0, RepresentedQuestRewardReputationSourceLikeCpp::Quest),
    ] {
        let (mut session, send_rx) = make_session();
        let mut quest = quest_template(7100);
        quest.flags = flags;
        quest.special_flags = special_flags;
        quest.reward_faction_ids[0] = 72;
        quest.reward_faction_overrides[0] = 1200;
        record_quest_reward_reputation_for_test(&mut session, &quest);
        let rewards = represented_quest_reward_reputations_for_test(&session);
        assert_eq!(rewards.len(), 1);
        assert_eq!(rewards[0].source, expected);
        assert!(send_rx.try_recv().is_err());
    }
}

#[test]
fn quest_reputation_wiring_preserves_slots_override_mask_and_missing_store_diagnostics() {
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(7101);
    quest.reward_faction_ids[0] = 72;
    quest.reward_faction_overrides[0] = 1250;
    quest.reward_faction_cap_in[0] = 5;
    quest.reward_faction_flags = 1;
    // Zero faction IDs are skipped even when an override is present.
    quest.reward_faction_overrides[1] = 1000;
    quest.reward_faction_ids[2] = 76;
    quest.reward_faction_values[2] = -4;
    quest.reward_faction_ids[3] = 930;
    quest.reward_faction_overrides[3] = -1200;
    quest.reward_faction_cap_in[3] = 5;
    record_quest_reward_reputation_for_test(&mut session, &quest);
    let rewards = represented_quest_reward_reputations_for_test(&session);
    assert_eq!(
        rewards.iter().map(|reward| reward.slot).collect::<Vec<_>>(),
        vec![0, 2, 3]
    );
    assert_eq!(rewards[0].base_reputation_before_gain, 12);
    assert!(rewards[0].no_quest_bonus);
    assert!(rewards[0].no_spillover);
    assert!(!rewards[0].quest_faction_reward_store_lookup_unrepresented);
    assert!(rewards[0].reputation_rank_cap_check_unrepresented);
    assert_eq!(rewards[1].base_reputation_before_gain, 0);
    assert_eq!(rewards[1].reputation_after_low_level_rate_like_cpp, 0);
    assert_eq!(rewards[1].reputation_after_reward_rate_like_cpp, 0);
    assert!(rewards[1].quest_faction_reward_store_lookup_unrepresented);
    assert!(rewards[1].reputation_reward_rate_lookup_unrepresented);
    assert!(!rewards[1].no_quest_bonus);
    assert_eq!(rewards[2].base_reputation_before_gain, -12);
    assert!(!rewards[2].no_spillover);
    assert!(!rewards[2].reputation_rank_cap_check_unrepresented);
    assert!(
        rewards
            .iter()
            .all(|reward| reward.faction_store_lookup_unrepresented)
    );
    assert!(
        rewards
            .iter()
            .all(|reward| reward.modify_reputation_runtime_unrepresented)
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_reputation_wiring_present_tables_fail_closed_for_missing_rows_and_invalid_indices() {
    for reward_value in [4, -4, 10, -10, i32::MIN] {
        for populated in [false, true] {
            let (mut session, send_rx) = make_session();
            let mut quest = quest_template(7102);
            quest.reward_faction_ids[0] = 72;
            quest.reward_faction_values[0] = reward_value;
            let rows = if populated {
                vec![
                    QuestFactionRewardEntry {
                        id: 1,
                        difficulty: [0; 10],
                    },
                    QuestFactionRewardEntry {
                        id: 2,
                        difficulty: [0; 10],
                    },
                ]
            } else {
                Vec::new()
            };
            session.set_quest_faction_reward_store(Arc::new(
                QuestFactionRewardStore::from_entries(rows),
            ));
            record_quest_reward_reputation_for_test(&mut session, &quest);
            assert!(represented_quest_reward_reputations_for_test(&session).is_empty());
            assert!(send_rx.try_recv().is_err());
        }
    }
}

#[test]
fn quest_reputation_wiring_present_faction_store_skips_unknown_faction_before_valuation() {
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(7103);
    quest.reward_faction_ids[0] = 72;
    quest.reward_faction_overrides[0] = 1200;
    quest.reward_faction_ids[1] = 76;
    quest.reward_faction_values[1] = -4;
    session.set_faction_store(Arc::new(FactionStore::from_entries([])));
    record_quest_reward_reputation_for_test(&mut session, &quest);
    assert!(represented_quest_reward_reputations_for_test(&session).is_empty());
    assert!(send_rx.try_recv().is_err());
}

pub(super) fn install_world_map_catalogs(session: &mut WorldSession) {
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(
        wow_map::MapManager::default(),
    )));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
}

fn canonical_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session();
    install_world_map_catalogs(&mut session);
    attach_player_controller_for_test(
        &mut session,
        ObjectGuid::create_player(1, 7104),
        "QuestReputationOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    ensure_world_map_for_current_player_for_test(&mut session).expect("world map");
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(72, 5),
    ])));
    (session, send_rx)
}

#[test]
fn quest_reputation_wiring_installs_valuation_before_standing_publication() {
    let (mut session, send_rx) = canonical_session();
    let mut quest = quest_template(7104);
    quest.quest_level = 20;
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_faction_ids[0] = 72;
    quest.reward_faction_overrides[0] = 1200;
    session.set_reputation_rates_like_cpp(wow_world::ReputationRatesLikeCpp {
        gain: 2.0,
        low_level_quest: 0.5,
        ..Default::default()
    });
    let factions = faction_store_for_test(&session).unwrap();
    session.set_reputation_reward_rate_store(Arc::new(
        ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                faction_id: 72,
                rates: ReputationRewardRateEntryLikeCpp {
                    quest_rate: 1.0,
                    quest_daily_rate: 1.5,
                    quest_weekly_rate: 1.0,
                    quest_monthly_rate: 1.0,
                    quest_repeatable_rate: 1.0,
                    creature_rate: 1.0,
                    spell_rate: 1.0,
                },
            }],
            factions.as_ref(),
        )
        .0,
    ));
    record_quest_reward_reputation_for_test(&mut session, &quest);
    let rewards = represented_quest_reward_reputations_for_test(&session);
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].base_reputation_before_gain, 12);
    assert_eq!(rewards[0].reputation_after_low_level_rate_like_cpp, 6);
    assert_eq!(rewards[0].reputation_after_reward_rate_like_cpp, 9);
    assert!(!rewards[0].modify_reputation_runtime_unrepresented);
    assert_eq!(reputation_standing_for_test(&session, 5), Some(18));
    let bytes = send_rx.try_recv().expect("standing publication");
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::SetFactionStanding)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_float().unwrap(), 0.0);
    assert_eq!(packet.read_uint32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 5);
    assert_eq!(packet.read_int32().unwrap(), 18);
    assert!(!packet.read_bit().unwrap());
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_reputation_wiring_rank_cap_prevents_canonical_write_and_publication() {
    let (mut session, send_rx) = canonical_session();
    set_reputation_standing_for_test(&mut session, 5, 9000);
    let mut quest = quest_template(7105);
    quest.reward_faction_ids[0] = 72;
    quest.reward_faction_overrides[0] = 1200;
    quest.reward_faction_cap_in[0] = 5;
    record_quest_reward_reputation_for_test(&mut session, &quest);
    assert!(represented_quest_reward_reputations_for_test(&session).is_empty());
    assert_eq!(reputation_standing_for_test(&session, 5), Some(9000));
    assert!(send_rx.try_recv().is_err());
}
