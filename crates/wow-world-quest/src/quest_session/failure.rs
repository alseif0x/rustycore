// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Native Player state and failure-packet participants for `Player::FailQuest`.
//!
//! Rust represents the C++ timed-quest membership/timer through the single
//! canonical `PlayerQuestStatusRecord::end_time_secs` owner. Clearing it
//! removes the failed quest from later timer admission; no parallel timed-ID
//! index is needed. `QuestLog.EndTime` is an independent update field and
//! remains unchanged by `FailQuest`.

use wow_conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP,
};
use wow_constants::InventoryResult;
use wow_data::quest::QuestTemplate;
use wow_entities::{Player, PlayerQuestStatusSaveTypeLikeCpp};
use wow_packet::packets::quest::{QuestGiverQuestFailed, QuestUpdateFailedTimer};

use super::QuestSessionHostLikeCpp;

const QUEST_LOG_STATE_FAIL_LIKE_CPP: u32 = 0x0002;

/// Apply only the native `SetQuestStatus` state write before Application runs
/// its callbacks and update publication.
fn begin_player_quest_failure_like_cpp(
    player: &mut Player,
    quest: &QuestTemplate,
) -> Result<bool, &'static str> {
    let quests = &player.gameplay_state().quests;
    if !quests.status_authority_complete_like_cpp() {
        return Err("canonical Player quest-status authority is incomplete");
    }

    let Some(status) = quests.status_like_cpp(quest.id) else {
        return Ok(false);
    };
    let eligible = status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
        || (status.status == QUEST_STATUS_COMPLETE_LIKE_CPP
            && quest.limit_time_secs != 0
            && quest.objectives.is_empty());
    if !eligible {
        return Ok(false);
    }

    let quests = &mut player.gameplay_state_mut().quests;
    let Some(status) = quests.status_mut_like_cpp(quest.id) else {
        return Err("eligible quest status disappeared during native failure");
    };
    status.status = QUEST_STATUS_FAILED_LIKE_CPP;
    if !quest.is_turn_in_like_cpp() {
        quests.mark_status_save_like_cpp(quest.id, PlayerQuestStatusSaveTypeLikeCpp::Default);
    }

    Ok(true)
}

/// Apply the post-callback native slot/timer writes and return the live slot
/// found after callbacks, if the quest still occupies one.
fn finish_player_quest_failure_like_cpp(
    player: &mut Player,
    quest: &QuestTemplate,
) -> Result<Option<u8>, &'static str> {
    let quests = &player.gameplay_state().quests;
    if !quests.status_authority_complete_like_cpp() {
        return Err("canonical Player quest-status authority is incomplete");
    }
    // C++ FindQuestSlot scans the live update fields after SetQuestStatus has
    // completed its callbacks and SendQuestUpdate. Do not reuse status.slot.
    let live_slot = player.find_quest_slot_like_cpp(quest.id);
    if let Some(slot) = live_slot
        && !player.set_quest_slot_state_like_cpp(slot, QUEST_LOG_STATE_FAIL_LIKE_CPP)
    {
        return Err("live native quest-log slot could not be marked failed");
    }

    if quest.limit_time_secs != 0 {
        let quests = &mut player.gameplay_state_mut().quests;
        if quests.status_like_cpp(quest.id).is_none() {
            // C++ `m_QuestStatus[questId]` default-constructs QuestStatusData
            // after callbacks removed its row: status None, neutral metadata,
            // and MAX_QUEST_LOG_SIZE as the non-slot sentinel. Do not dirty
            // this row or replace an earlier delete intent.
            quests.set_status_like_cpp(quest.id, QUEST_STATUS_NONE_LIKE_CPP, false);
        }
        let Some(status) = quests.status_mut_like_cpp(quest.id) else {
            return Err("timed quest status could not be default-constructed");
        };
        // The current Rust Player model stores the quest timer in the
        // canonical status end time. FailQuest does not clear QuestLog.EndTime.
        status.end_time_secs = 0;
    }

    Ok(live_slot)
}

/// Begin the native Player half of `Player::FailQuest` against the session's
/// exact generation-checked Player owner. The caller runs the complete
/// `SetQuestStatus` callbacks and `SendQuestUpdate` choreography only after
/// this returns `Ok(true)`.
pub fn begin_native_quest_failure_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest: &QuestTemplate,
) -> Result<bool, &'static str> {
    host.core()
        .with_owned_player_mut_like_cpp(|player| begin_player_quest_failure_like_cpp(player, quest))
        .ok_or("canonical Player owner is unavailable")?
}

/// Finish native failure state and emit the post-callback failure packet.
/// Application calls this only after the full `SetQuestStatus` callback and
/// `SendQuestUpdate` sequence has completed.
pub fn finish_native_quest_failure_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest: &QuestTemplate,
) -> Result<(), &'static str> {
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            finish_player_quest_failure_like_cpp(player, quest)
        })
        .ok_or("canonical Player owner is unavailable")??;

    if quest.limit_time_secs != 0 {
        let _ = host.send_packet(&QuestUpdateFailedTimer { quest_id: quest.id });
    } else {
        let _ = host.send_packet(&QuestGiverQuestFailed {
            quest_id: quest.id,
            reason: InventoryResult::Ok as u32,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_entities::PlayerQuestStatusRecord;

    fn quest_template(
        quest_id: u32,
        quest_type: u8,
        limit_time_secs: i64,
        objectives: Vec<wow_data::quest::QuestObjective>,
    ) -> QuestTemplate {
        QuestTemplate {
            id: quest_id,
            quest_type,
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
            reward_display_spell: [0; 3],
            reward_spell: 0,
            reward_honor: 0,
            reward_title_id: 0,
            reward_skill_line_id: 0,
            reward_skill_points: 0,
            reward_mail_template_id: 0,
            reward_mail_delay_secs: 0,
            reward_mail_sender_entry: 0,
            reward_faction_ids: [0; 5],
            reward_faction_values: [0; 5],
            reward_faction_overrides: [0; 5],
            reward_faction_cap_in: [0; 5],
            reward_faction_flags: 0,
            source_item_id: 0,
            source_item_count: 0,
            source_spell_id: 0,
            limit_time_secs,
            expansion: 0,
            flags: 0,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags: 0,
            event_id_for_quest: 0,
            reward_items: [0; 4],
            reward_amounts: [0; 4],
            reward_currencies: [0; 4],
            reward_currency_amounts: [0; 4],
            item_drop: [0; 4],
            item_drop_quantity: [0; 4],
            log_title: String::new(),
            log_description: String::new(),
            quest_description: String::new(),
            area_description: String::new(),
            quest_completion_log: String::new(),
            objectives,
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
            reward_choice_items: [(0, 0); 6],
            reward_choice_item_types: [0; 6],
        }
    }

    fn objective(quest_id: u32) -> wow_data::quest::QuestObjective {
        wow_data::quest::QuestObjective {
            id: 1,
            quest_id,
            obj_type: 0,
            order: 0,
            storage_index: 0,
            object_id: 1,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        }
    }

    fn player_with_status(quest_id: u32, status: u8, slot: u8, end_time_secs: i64) -> Player {
        let mut player = Player::new(None, false);
        let quests = &mut player.gameplay_state_mut().quests;
        quests.set_status_authority_complete_like_cpp(true);
        quests.insert_status_like_cpp(
            quest_id,
            PlayerQuestStatusRecord {
                quest_id,
                status,
                explored: true,
                accept_time_secs: 17,
                end_time_secs,
                objective_counts: vec![2, 5],
                slot,
            },
        );
        player
    }

    #[test]
    fn incomplete_failure_retains_objectives_slot_timer_and_marks_default_dirty() {
        let quest_id = 7008;
        let mut player =
            player_with_status(quest_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP, 3, 1_800_000_000);
        let quest = quest_template(quest_id, 2, 600, vec![objective(quest_id)]);

        assert!(begin_player_quest_failure_like_cpp(&mut player, &quest).unwrap());

        let state = &player.gameplay_state().quests;
        let status = state.status_like_cpp(quest_id).unwrap();
        assert_eq!(status.status, QUEST_STATUS_FAILED_LIKE_CPP);
        assert!(status.explored);
        assert_eq!(status.accept_time_secs, 17);
        assert_eq!(status.end_time_secs, 1_800_000_000);
        assert_eq!(status.objective_counts, [2, 5]);
        assert_eq!(status.slot, 3);
        assert_eq!(
            state.status_save_type_like_cpp(quest_id),
            Some(PlayerQuestStatusSaveTypeLikeCpp::Default)
        );
    }

    #[test]
    fn eligibility_matches_incomplete_and_empty_timed_complete_cases() {
        let quest_id = 7009;
        let mut player =
            player_with_status(quest_id, QUEST_STATUS_COMPLETE_LIKE_CPP, 4, 1_800_000_100);
        let timed_empty = quest_template(quest_id, 2, 1, Vec::new());
        assert!(begin_player_quest_failure_like_cpp(&mut player, &timed_empty).unwrap());

        for quest in [
            quest_template(quest_id, 2, 0, Vec::new()),
            quest_template(quest_id, 2, 1, vec![objective(quest_id)]),
        ] {
            let mut completed =
                player_with_status(quest_id, QUEST_STATUS_COMPLETE_LIKE_CPP, 4, 1_800_000_100);
            assert!(!begin_player_quest_failure_like_cpp(&mut completed, &quest).unwrap());
            assert_eq!(
                completed
                    .gameplay_state()
                    .quests
                    .status_like_cpp(quest_id)
                    .unwrap()
                    .status,
                QUEST_STATUS_COMPLETE_LIKE_CPP
            );
            assert_eq!(
                completed
                    .gameplay_state()
                    .quests
                    .status_save_type_like_cpp(quest_id),
                None
            );
        }

        let mut failed =
            player_with_status(quest_id, QUEST_STATUS_FAILED_LIKE_CPP, 4, 1_800_000_100);
        assert!(!begin_player_quest_failure_like_cpp(&mut failed, &timed_empty).unwrap());

        let turn_in_id = 7010;
        let mut turn_in = player_with_status(turn_in_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP, 5, 0);
        assert!(
            begin_player_quest_failure_like_cpp(
                &mut turn_in,
                &quest_template(turn_in_id, 0, 0, Vec::new())
            )
            .unwrap()
        );
        assert_eq!(
            turn_in
                .gameplay_state()
                .quests
                .status_save_type_like_cpp(turn_in_id),
            None
        );
    }

    #[test]
    fn timed_finish_rescans_live_slot_and_preserves_quest_log_end_time() {
        let quest_id = 7011;
        let mut player =
            player_with_status(quest_id, QUEST_STATUS_FAILED_LIKE_CPP, 2, 1_800_000_200);
        assert!(player.set_quest_slot_like_cpp(2, 9999));
        assert!(player.set_quest_slot_like_cpp(8, quest_id));
        assert!(player.set_quest_slot_end_time_like_cpp(8, 1_800_000_300));
        assert!(player.set_quest_slot_state_like_cpp(8, 1));
        assert!(player.set_quest_slot_counter_like_cpp(8, 7, 42));
        let quest = quest_template(quest_id, 2, 600, Vec::new());

        let live_slot = finish_player_quest_failure_like_cpp(&mut player, &quest).unwrap();

        assert_eq!(live_slot, Some(8));
        assert_eq!(
            player.quest_log_values_like_cpp()[2].state_flags() & QUEST_LOG_STATE_FAIL_LIKE_CPP,
            0
        );
        let row = player.quest_log_values_like_cpp()[8];
        assert_eq!(row.state_flags(), 1 | QUEST_LOG_STATE_FAIL_LIKE_CPP);
        assert_eq!(row.end_time(), 1_800_000_300);
        assert_eq!(row.objective_progress()[7], 42);
        assert_eq!(
            player
                .gameplay_state()
                .quests
                .status_like_cpp(quest_id)
                .unwrap()
                .end_time_secs,
            0
        );
    }

    #[test]
    fn timed_finish_after_callback_removal_recreates_only_cpp_default_status() {
        let quest_id = 7014;
        let mut player =
            player_with_status(quest_id, QUEST_STATUS_FAILED_LIKE_CPP, 2, 1_800_000_500);
        assert!(player.set_quest_slot_like_cpp(2, 9999));
        assert!(player.set_quest_slot_like_cpp(8, quest_id));
        assert!(player.set_quest_slot_end_time_like_cpp(8, 1_800_000_600));
        assert!(
            player
                .gameplay_state_mut()
                .quests
                .remove_status_like_cpp(quest_id)
                .is_some()
        );
        let quest = quest_template(quest_id, 2, 600, Vec::new());

        let live_slot = finish_player_quest_failure_like_cpp(&mut player, &quest).unwrap();

        assert_eq!(live_slot, Some(8));
        let state = &player.gameplay_state().quests;
        let recreated = state.status_like_cpp(quest_id).unwrap();
        assert_eq!(recreated.status, QUEST_STATUS_NONE_LIKE_CPP);
        assert!(!recreated.explored);
        assert_eq!(recreated.accept_time_secs, 0);
        assert_eq!(recreated.end_time_secs, 0);
        assert!(recreated.objective_counts.is_empty());
        assert_eq!(recreated.slot, 25);
        assert_eq!(
            state.status_save_type_like_cpp(quest_id),
            Some(PlayerQuestStatusSaveTypeLikeCpp::Delete)
        );
        assert_eq!(
            player.quest_log_values_like_cpp()[2].state_flags() & QUEST_LOG_STATE_FAIL_LIKE_CPP,
            0
        );
        assert_eq!(
            player.quest_log_values_like_cpp()[8].state_flags() & QUEST_LOG_STATE_FAIL_LIKE_CPP,
            QUEST_LOG_STATE_FAIL_LIKE_CPP
        );
        assert_eq!(
            player.quest_log_values_like_cpp()[8].end_time(),
            1_800_000_600
        );
    }

    #[test]
    fn non_timed_finish_leaves_timer_unchanged_and_handles_absent_live_slot() {
        let quest_id = 7012;
        let mut player =
            player_with_status(quest_id, QUEST_STATUS_FAILED_LIKE_CPP, 2, 1_800_000_400);
        let quest = quest_template(quest_id, 2, 0, Vec::new());

        assert_eq!(
            finish_player_quest_failure_like_cpp(&mut player, &quest).unwrap(),
            None
        );
        assert_eq!(
            player
                .gameplay_state()
                .quests
                .status_like_cpp(quest_id)
                .unwrap()
                .end_time_secs,
            1_800_000_400
        );
    }

    #[test]
    fn incomplete_quest_authority_is_an_error() {
        let quest_id = 7013;
        let mut player = player_with_status(quest_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP, 2, 0);
        player
            .gameplay_state_mut()
            .quests
            .set_status_authority_complete_like_cpp(false);
        let quest = quest_template(quest_id, 2, 0, Vec::new());

        assert_eq!(
            begin_player_quest_failure_like_cpp(&mut player, &quest),
            Err("canonical Player quest-status authority is incomplete")
        );
        assert_eq!(
            finish_player_quest_failure_like_cpp(&mut player, &quest),
            Err("canonical Player quest-status authority is incomplete")
        );
    }
}
