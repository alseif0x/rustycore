//! Quest scenarios for [`super`].
//!
//! Split out of quest_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_mail_quest_giver_sender_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7027;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_mail_template_id = 56;
    quest.reward_mail_delay_secs = 30;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_mails_like_cpp(),
        &[RepresentedQuestRewardMailLikeCpp {
            quest_id,
            mail_template_id: 56,
            delay_secs: 30,
            sender_entry: None,
            quest_giver_guid: Some(player_guid),
            mail_template_lookup_unrepresented: true,
            mail_draft_runtime_unrepresented: true,
            character_db_transaction_unrepresented: true,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_records_reward_reputation_override_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7028;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_faction_ids[2] = 930;
    quest.reward_faction_values[2] = 7;
    quest.reward_faction_overrides[2] = 1200;
    quest.reward_faction_cap_in[2] = 5;
    quest.reward_faction_flags = 1 << 2;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 2,
            faction_id: 930,
            reward_faction_value: 7,
            reward_faction_override: 1200,
            reward_faction_cap_in: 5,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 12,
            reputation_after_reward_rate_like_cpp: 12,
            no_quest_bonus: true,
            no_spillover: true,
            source: RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
            faction_store_lookup_unrepresented: true,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: true,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: true,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_records_reward_reputation_db2_lookup_gap_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7029;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_values[0] = -4;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: -4,
            reward_faction_override: 0,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 0,
            reputation_after_low_level_rate_like_cpp: 0,
            reputation_after_reward_rate_like_cpp: 0,
            no_quest_bonus: false,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: true,
            quest_faction_reward_store_lookup_unrepresented: true,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: true,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_resolves_reward_reputation_db2_value_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7030;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_values[0] = -4;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.set_quest_faction_reward_store(Arc::new(QuestFactionRewardStore::from_entries([
        QuestFactionRewardEntry {
            id: 2,
            difficulty: [0, 5, 10, 15, 250, 350, 500, 750, 1000, 1500],
        },
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: -4,
            reward_faction_override: 0,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 250,
            reputation_after_low_level_rate_like_cpp: 250,
            reputation_after_reward_rate_like_cpp: 250,
            no_quest_bonus: false,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .expect("quest reward faction state")
            .standing,
        250
    );
    let mut pkt = loop {
        let bytes = send_rx
            .try_recv()
            .expect("set faction standing packet from quest reputation");
        let pkt = wow_packet::WorldPacket::from_bytes(&bytes);
        if pkt.server_opcode() == Some(wow_constants::ServerOpcodes::SetFactionStanding) {
            break pkt;
        }
    };
    pkt.skip_opcode();
    assert_eq!(pkt.read_float().expect("achievement bonus"), 0.0);
    assert_eq!(pkt.read_uint32().expect("faction count"), 1);
    assert_eq!(pkt.read_int32().expect("reputation list id"), 5);
    assert_eq!(pkt.read_int32().expect("standing"), 250);
    assert!(!pkt.read_bit().expect("show visual"));
}
#[tokio::test]
async fn quest_giver_choose_reward_skips_missing_reward_reputation_faction_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7031;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 999_999;
    quest.reward_faction_overrides[0] = 1200;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_quest_reward_reputations_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_skips_reward_reputation_at_rank_cap_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7032;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    quest.reward_faction_cap_in[0] = 5;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    let mut manager = wow_map::MapManager::default();
    insert_player_with_reputation(&mut manager, player_guid, 76, 9000);
    attach_map_manager(&mut session, manager);
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_quest_reward_reputations_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_records_reward_reputation_below_rank_cap_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7033;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    quest.reward_faction_cap_in[0] = 6;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    let mut manager = wow_map::MapManager::default();
    insert_player_with_reputation(&mut manager, player_guid, 76, 9000);
    attach_map_manager(&mut session, manager);
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: 0,
            reward_faction_override: 1200,
            reward_faction_cap_in: 6,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 12,
            reputation_after_reward_rate_like_cpp: 12,
            no_quest_bonus: true,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_applies_reputation_reward_rate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7034;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.set_reputation_reward_rate_store(Arc::new(
        ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                faction_id: 76,
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
            session.faction_store().unwrap(),
        )
        .0,
    ));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: 0,
            reward_faction_override: 1200,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 12,
            reputation_after_reward_rate_like_cpp: 18,
            no_quest_bonus: true,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: false,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_applies_low_level_quest_reputation_rate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7036;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.quest_level = 20;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    session.set_player_level_like_cpp(80);
    session.set_reputation_rates_like_cpp(crate::ReputationRatesLikeCpp {
        low_level_quest: 0.5,
        ..crate::ReputationRatesLikeCpp::default()
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: 0,
            reward_faction_override: 1200,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 6,
            reputation_after_reward_rate_like_cpp: 6,
            no_quest_bonus: true,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_skips_zero_reputation_reward_rate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7035;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.set_reputation_reward_rate_store(Arc::new(
        ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                faction_id: 76,
                rates: ReputationRewardRateEntryLikeCpp {
                    quest_rate: 0.0,
                    quest_daily_rate: 1.0,
                    quest_weekly_rate: 1.0,
                    quest_monthly_rate: 1.0,
                    quest_repeatable_rate: 1.0,
                    creature_rate: 1.0,
                    spell_rate: 1.0,
                },
            }],
            session.faction_store().unwrap(),
        )
        .0,
    ));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_quest_reward_reputations_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_sets_daily_lockout_status_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7015;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(session.daily_quests_completed_like_cpp.contains(&quest_id));
    assert!(!session.df_quests_like_cpp.contains(&quest_id));
    assert!(session.last_daily_quest_time_like_cpp > 0);
}
#[tokio::test]
async fn quest_giver_choose_reward_sets_df_lockout_in_daily_table_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7016;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.daily_quests_completed_like_cpp.contains(&quest_id));
    assert!(session.df_quests_like_cpp.contains(&quest_id));
    assert!(session.last_daily_quest_time_like_cpp > 0);
}
#[tokio::test]
async fn quest_giver_choose_reward_sets_weekly_and_monthly_lockouts_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let weekly_id = 7017;
    let monthly_id = 7018;
    let mut weekly = quest_template(weekly_id);
    weekly.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | 0x0000_8000;
    let mut monthly = quest_template(monthly_id);
    monthly.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    monthly.special_flags = 0x0000_0010;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        weekly, monthly,
    ])));
    for quest_id in [weekly_id, monthly_id] {
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;
    }

    assert!(
        session
            .weekly_quests_completed_like_cpp
            .contains(&weekly_id)
    );
    assert!(
        !session
            .weekly_quests_completed_like_cpp
            .contains(&monthly_id)
    );
    assert!(
        session
            .monthly_quests_completed_like_cpp
            .contains(&monthly_id)
    );
    assert!(
        !session
            .monthly_quests_completed_like_cpp
            .contains(&weekly_id)
    );
}
#[tokio::test]
async fn quest_giver_choose_reward_sets_seasonal_lockout_status_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7019;
    let event_id = 9;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.quest_sort_id = -376;
    quest.event_id_for_quest = event_id;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .seasonal_quests_like_cpp
            .get(&event_id)
            .is_some_and(|quests| quests.contains_key(&quest_id))
    );
    assert!(session.seasonal_quest_changed_like_cpp);
}
#[tokio::test]
async fn quest_giver_choose_reward_removes_currency_objective_before_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7016;
    let currency_id = 394;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: currency_id as i32,
        amount: 4,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_gold_like_cpp(5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    assert!(
        session
            .add_currency_quest_reward_like_cpp(
                currency_id,
                10,
                CurrencyGainSourceLikeCpp::QuestReward,
            )
            .unwrap()
            .is_some()
    );
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(session.player_currency_quantity(currency_id), Some(6));
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 6,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(-4),
            quantity_gain_source: None,
            quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
}
