use super::*;
use crate::PlayerQuestStatusRecord;
use wow_constants::quest::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP,
};
use wow_data_model::quest::QuestEligibilityRules;

fn rules(id: u32) -> QuestEligibilityRules<'static> {
    rules_with(
        id,
        0,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
fn rules_with(
    id: u32,
    exclusive_group: i32,
    repeatable: bool,
    daily: bool,
    dungeon_finder: bool,
    weekly: bool,
    monthly: bool,
    seasonal: bool,
    event_id: u16,
    previous_quest_id: i32,
    dependent_previous_quest_ids: &'static [u32],
    dependent_breadcrumb_quest_ids: &'static [u32],
) -> QuestEligibilityRules<'static> {
    QuestEligibilityRules::new(
        id,
        repeatable,
        exclusive_group,
        previous_quest_id,
        dependent_previous_quest_ids,
        dependent_breadcrumb_quest_ids,
        daily,
        dungeon_finder,
        weekly,
        monthly,
        seasonal,
        event_id,
    )
}

fn status(quest_id: u32, status: u8) -> PlayerQuestStatusRecord {
    PlayerQuestStatusRecord {
        quest_id,
        status,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: Vec::new(),
        slot: 0,
    }
}

#[test]
fn can_take_quest_blocks_daily_already_completed_like_cpp() {
    let quest = rules_with(
        7600,
        0,
        false,
        true,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_daily_like_cpp(quest.id(), true);

    assert_eq!(
        state.quest_day_cooldown_block(&quest),
        Some(QuestDayCooldownBlock::Daily)
    );
}

#[test]
fn can_take_quest_allows_daily_not_yet_completed_like_cpp() {
    let quest = rules_with(
        7601,
        0,
        false,
        true,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let state = PlayerQuestGameplayState::default();

    assert_eq!(state.quest_day_cooldown_block(&quest), None);
}

#[test]
fn can_take_quest_blocks_df_quest_already_completed_like_cpp() {
    let quest = rules_with(
        7602,
        0,
        false,
        false,
        true,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_df_quest_like_cpp(quest.id(), true);

    assert_eq!(
        state.quest_day_cooldown_block(&quest),
        Some(QuestDayCooldownBlock::DungeonFinder)
    );
}

#[test]
fn can_take_quest_blocks_weekly_already_completed_like_cpp() {
    let quest = rules_with(
        7603,
        0,
        false,
        false,
        false,
        true,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_weekly_like_cpp(quest.id(), true);

    assert!(state.quest_weekly_cooldown_blocks(&quest));
}

#[test]
fn can_take_quest_blocks_monthly_already_completed_like_cpp() {
    let quest = rules_with(
        7605,
        0,
        false,
        false,
        false,
        false,
        true,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_monthly_like_cpp(quest.id(), true);

    assert!(state.quest_monthly_cooldown_blocks(&quest));
}

#[test]
fn can_take_quest_allows_weekly_not_yet_completed_like_cpp() {
    let quest = rules_with(
        7604,
        0,
        false,
        false,
        false,
        true,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let state = PlayerQuestGameplayState::default();

    assert!(!state.quest_weekly_cooldown_blocks(&quest));
}

#[test]
fn can_take_quest_allows_monthly_not_yet_completed_like_cpp() {
    let quest = rules_with(
        7606,
        0,
        false,
        false,
        false,
        false,
        true,
        false,
        0,
        0,
        &[],
        &[],
    );
    let state = PlayerQuestGameplayState::default();

    assert!(!state.quest_monthly_cooldown_blocks(&quest));
}

#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_rewarded_non_repeatable_like_cpp() {
    let quest = rules_with(
        9911,
        5,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let peer = rules_with(
        9910,
        5,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_rewarded_like_cpp(peer.id(), true);

    assert!(state.exclusive_group_peer_blocks(&quest, &peer));
}

#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_active_like_cpp() {
    let quest = rules_with(
        9913,
        7,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let peer = rules_with(
        9912,
        7,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.insert_status_like_cpp(
        peer.id(),
        status(peer.id(), QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    assert!(state.exclusive_group_peer_blocks(&quest, &peer));
}

#[test]
fn can_take_quest_exclusive_group_positive_no_conflicting_peer_allows_like_cpp() {
    let quest = rules_with(
        9915,
        9,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let peer = rules_with(
        9914,
        9,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let state = PlayerQuestGameplayState::default();

    assert!(!state.exclusive_group_peer_blocks(&quest, &peer));
}

#[test]
fn can_take_quest_exclusive_group_zero_never_blocks_like_cpp() {
    let state = PlayerQuestGameplayState::default();
    let zero_group = rules(9916);
    let zero_peer = rules(9918);
    let negative_group = rules_with(
        9917,
        -3,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let negative_peer = rules_with(
        9919,
        -3,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );

    assert!(!state.exclusive_group_peer_blocks(&zero_group, &zero_peer));
    assert!(!state.exclusive_group_peer_blocks(&negative_group, &negative_peer));
}

#[test]
fn can_take_quest_blocks_when_dependent_breadcrumb_in_log_like_cpp() {
    let breadcrumb_quest_id = 9930;
    let dependent_ids = [breadcrumb_quest_id];
    let mut state = PlayerQuestGameplayState::default();
    state.insert_status_like_cpp(
        breadcrumb_quest_id,
        status(breadcrumb_quest_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    assert!(state.dependent_breadcrumb_quests_block(&dependent_ids));

    let empty_state = PlayerQuestGameplayState::default();
    assert!(!empty_state.dependent_breadcrumb_quests_block(&dependent_ids));
}

#[test]
fn quest_status_rule_preserves_rewarded_repeatability_and_active_key_semantics() {
    let quest = rules(10);
    let mut state = PlayerQuestGameplayState::default();
    state.set_rewarded_like_cpp(quest.id(), true);
    assert_eq!(
        state.quest_status_block(&quest),
        Some(QuestStatusBlock::AlreadyRewarded)
    );

    let repeatable = rules_with(
        10,
        0,
        true,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    assert_eq!(state.quest_status_block(&repeatable), None);
    state.insert_status_like_cpp(quest.id(), status(quest.id(), QUEST_STATUS_NONE_LIKE_CPP));
    assert_eq!(
        state.quest_status_block(&repeatable),
        Some(QuestStatusBlock::AlreadyActive)
    );
    assert_eq!(
        state.quest_status_block(&quest),
        Some(QuestStatusBlock::AlreadyRewarded)
    );
}

#[test]
fn previous_quest_rule_keeps_signed_status_and_i32_min_unsigned_abs() {
    let mut state = PlayerQuestGameplayState::default();
    assert!(state.previous_quest_requirement_satisfied(0));
    assert!(!state.previous_quest_requirement_satisfied(i32::MIN));
    state.insert_status_like_cpp(
        i32::MIN.unsigned_abs(),
        status(i32::MIN.unsigned_abs(), QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );
    assert!(state.previous_quest_requirement_satisfied(i32::MIN));

    let active_id = 11;
    assert!(!state.previous_quest_requirement_satisfied(active_id as i32));
    state.set_rewarded_like_cpp(active_id, true);
    assert!(state.previous_quest_requirement_satisfied(active_id as i32));
    assert!(!state.previous_quest_requirement_satisfied(-(active_id as i32)));
    state.insert_status_like_cpp(
        active_id,
        status(active_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );
    assert!(state.previous_quest_requirement_satisfied(-(active_id as i32)));
    state.insert_status_like_cpp(active_id, status(active_id, QUEST_STATUS_COMPLETE_LIKE_CPP));
    assert!(!state.previous_quest_requirement_satisfied(-(active_id as i32)));
}

#[test]
fn quest_day_cooldown_prioritizes_df_over_daily_even_without_df_membership() {
    let quest = rules_with(
        22,
        0,
        false,
        true,
        true,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_daily_like_cpp(quest.id(), true);

    assert_eq!(state.quest_day_cooldown_block(&quest), None);

    state.set_df_quest_like_cpp(quest.id(), true);
    assert_eq!(
        state.quest_day_cooldown_block(&quest),
        Some(QuestDayCooldownBlock::DungeonFinder),
    );
}

#[test]
fn exclusive_group_peer_keeps_daily_check_independent_from_df_check() {
    let quest = rules_with(
        20,
        4,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let peer = rules_with(
        21,
        4,
        false,
        true,
        true,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.set_daily_like_cpp(peer.id(), true);

    assert!(state.exclusive_group_peer_blocks(&quest, &peer));
}

#[test]
fn exclusive_group_peer_blocks_on_status_key_even_when_record_is_none() {
    let quest = rules_with(
        30,
        8,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let peer = rules_with(
        31,
        8,
        false,
        false,
        false,
        false,
        false,
        false,
        0,
        0,
        &[],
        &[],
    );
    let mut state = PlayerQuestGameplayState::default();
    state.insert_status_like_cpp(peer.id(), status(peer.id(), QUEST_STATUS_NONE_LIKE_CPP));

    assert!(state.exclusive_group_peer_blocks(&quest, &peer));
}

#[test]
fn dependent_breadcrumb_membership_blocks_only_cpp_incomplete_complete_or_failed_statuses() {
    let breadcrumb_ids = [40];
    for status_value in [
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
        QUEST_STATUS_FAILED_LIKE_CPP,
    ] {
        assert!(
            PlayerQuestGameplayState::dependent_breadcrumb_quest_ids_block(&breadcrumb_ids, |id| {
                (id == 40).then_some(status_value)
            },)
        );
    }
    assert!(
        !PlayerQuestGameplayState::dependent_breadcrumb_quest_ids_block(&breadcrumb_ids, |_| Some(
            QUEST_STATUS_NONE_LIKE_CPP
        ),)
    );
}

// Original seasonal metadata inputs, shared with the APP bridge matrix.
fn test_quest_template(id: u32) -> wow_data_model::quest::QuestTemplate {
    wow_data_model::quest::QuestTemplate {
        id,
        quest_type: 0,
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
        reward_display_spell: [0; wow_constants::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; wow_constants::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; wow_constants::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; wow_constants::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; wow_constants::quest::QUEST_REWARD_REPUTATIONS_COUNT],
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
        reward_items: [0; wow_constants::quest::QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; wow_constants::quest::QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; wow_constants::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_constants::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; wow_constants::quest::QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; wow_constants::quest::QUEST_ITEM_DROP_COUNT],
        log_title: String::new(),
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
        reward_choice_items: [(0, 0); wow_constants::quest::QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; wow_constants::quest::QUEST_REWARD_CHOICES_COUNT],
    }
}

fn seasonal_test_quest_template(
    id: u32,
    quest_sort_id: i32,
    event_id_for_quest: u16,
) -> wow_data_model::quest::QuestTemplate {
    let mut quest = test_quest_template(id);
    quest.quest_sort_id = quest_sort_id;
    quest.min_level = 0;
    quest.event_id_for_quest = event_id_for_quest;
    quest
}

#[test]
fn can_take_quest_rejects_completed_seasonal_bucket_quest_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    let quest = seasonal_test_quest_template(12_345, -376, 9);
    state.seed_seasonal_quest_like_cpp(9, 12_345, 100);

    assert!(state.quest_seasonal_cooldown_blocks(&quest.eligibility_rules()));
}

#[test]
fn can_take_quest_allows_seasonal_when_only_other_event_bucket_has_quest_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    let quest = seasonal_test_quest_template(12_345, -376, 9);
    state.seed_seasonal_quest_like_cpp(10, 12_345, 100);

    assert!(!state.quest_seasonal_cooldown_blocks(&quest.eligibility_rules()));
}

#[test]
fn can_take_quest_allows_seasonal_when_bucket_missing_or_empty_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    let quest = seasonal_test_quest_template(12_345, -376, 9);

    assert!(!state.quest_seasonal_cooldown_blocks(&quest.eligibility_rules()));

    state.ensure_seasonal_event_like_cpp(9);
    assert!(!state.quest_seasonal_cooldown_blocks(&quest.eligibility_rules()));
}

#[test]
fn can_take_quest_allows_non_seasonal_even_when_same_bucket_has_quest_like_cpp() {
    let mut state = PlayerQuestGameplayState::default();
    let quest = seasonal_test_quest_template(12_345, -101, 9);
    state.seed_seasonal_quest_like_cpp(9, 12_345, 100);

    assert!(!state.quest_seasonal_cooldown_blocks(&quest.eligibility_rules()));
}
