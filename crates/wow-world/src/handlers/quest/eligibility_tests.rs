//! Direct recurrence-gate tests for `WorldSession::can_take_quest`.
//!
//! Target C++ SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
//! `Player::CanTakeQuest` (Player.cpp:14087-14096), `SatisfyQuestDay`
//! (15387-15401), `SatisfyQuestWeek` (15403-15410), and `SatisfyQuestMonth`
//! (15439-15446). These cases exercise the handle-less test-state fallback;
//! they do not prove Player loading, cooldown reset, or persistence parity.

use super::*;
use std::sync::Arc;
use wow_data::quest::{
    QUEST_FLAGS_DAILY_LIKE_CPP, QUEST_FLAGS_WEEKLY_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP, QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP, QuestStore,
    QuestTemplate,
};

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    let mut session = WorldSession::new(
        1,
        "QuestStatusTest".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    );
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    (session, send_rx)
}

fn quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 2,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; wow_data::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
    }
}

fn install_quest(session: &mut WorldSession, quest: &QuestTemplate) {
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest.clone()])));
}

#[test]
fn can_take_quest_blocks_daily_already_completed_like_cpp() {
    // C++ SatisfyQuestDay: daily IDs are checked in DailyQuestsCompleted.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7600u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .daily_quests_completed_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_allows_daily_not_yet_completed_like_cpp() {
    // C++ SatisfyQuestDay permits a daily absent from DailyQuestsCompleted.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7601u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    install_quest(&mut session, &quest);

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_df_quest_already_completed_like_cpp() {
    // C++ SatisfyQuestDay checks the separate DFQuests set for DF quests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7602u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .df_quests_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_weekly_already_completed_like_cpp() {
    // C++ SatisfyQuestWeek rejects IDs present in m_weeklyquests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7603u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .weekly_quests_completed_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_monthly_already_completed_like_cpp() {
    // C++ SatisfyQuestMonth rejects IDs present in m_monthlyquests.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7605u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    install_quest(&mut session, &quest);
    session
        .quest_test_fixture_like_cpp
        .monthly_quests_completed_like_cpp
        .insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}
