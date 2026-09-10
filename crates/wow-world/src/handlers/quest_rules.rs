// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest handler rules that read no session state.
//!
//! Moved out of the owner under #680. Every one was already receiver-free,
//! so it cannot read or write the owner's state: these are rules, not owner
//! behaviour. Bodies and signatures are unchanged.

use crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP;
use crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use crate::handlers::quest::PlayerQuestStatus;
use crate::handlers::quest::QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP;
use crate::handlers::quest::QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP;
use crate::handlers::quest::QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP;
use crate::handlers::quest::QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP;
use crate::handlers::quest::QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_TALKTO_LIKE_CPP_LOCAL;
use crate::handlers::quest::QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP_LOCAL;
use crate::handlers::quest::QuestChoiceItemLikeCpp;
use crate::session::*;
use std::collections::HashMap;
use std::collections::HashSet;
use wow_core::GameTime;
use wow_data::quest::QuestStore;

pub(crate) fn apply_quest_item_added_bound_to_statuses_like_cpp(
    quest_store: &QuestStore,
    rewarded_quests: &HashSet<u32>,
    player_quests: &mut HashMap<u32, PlayerQuestStatus>,
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> Option<(u32, i32)> {
    let count = i32::try_from(count).unwrap_or(i32::MAX);
    let mut ordered_quest_ids = player_quests
        .values()
        .map(|status| (status.slot, status.quest_id))
        .collect::<Vec<_>>();
    ordered_quest_ids.sort_unstable();

    for object_id in [entry_id, quest_log_item_id] {
        if object_id == 0 {
            continue;
        }
        let object_id = i32::try_from(object_id).unwrap_or(i32::MAX);
        for &(_, quest_id) in &ordered_quest_ids {
            let Some(status) = player_quests.get_mut(&quest_id) else {
                continue;
            };
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                continue;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            for (objective_index, objective) in quest.objectives.iter().enumerate() {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                    || (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL)
                        == 0
                    || objective.object_id != object_id
                    || !represented_quest_objective_completable_like_cpp(
                        status,
                        quest,
                        objective_index,
                    )
                {
                    continue;
                }
                let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                    continue;
                };
                if status.objective_counts.len() <= storage_index {
                    status.objective_counts.resize(storage_index + 1, 0);
                }
                let current = status.objective_counts[storage_index];
                if current >= objective.amount {
                    continue;
                }
                let new_count = current.saturating_add(count).clamp(0, objective.amount);
                status.objective_counts[storage_index] = new_count;
                if new_count >= objective.amount
                    && represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        quest,
                        objective.id,
                        rewarded_quests.contains(&status.quest_id),
                    )
                {
                    status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
                }
                return Some((status.quest_id, new_count));
            }
        }
    }
    None
}

pub(crate) fn apply_quest_item_added_non_bound_to_statuses_like_cpp(
    quest_store: &QuestStore,
    rewarded_quests: &HashSet<u32>,
    player_quests: &mut HashMap<u32, PlayerQuestStatus>,
    entry_id: u32,
    quest_log_item_id: u32,
    count: u32,
) -> Vec<u32> {
    let entry_object_id = i32::try_from(entry_id).unwrap_or(i32::MAX);
    let mut objective_ids = vec![entry_object_id];
    if quest_log_item_id != 0 {
        objective_ids.push(i32::try_from(quest_log_item_id).unwrap_or(i32::MAX));
    }
    let count = i32::try_from(count).unwrap_or(i32::MAX);
    let mut changed_quest_ids = Vec::new();
    let mut quests_to_complete = Vec::new();

    for status in player_quests.values_mut() {
        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            continue;
        }
        let Some(quest) = quest_store.get(status.quest_id) else {
            continue;
        };
        for (objective_index, objective) in quest.objectives.iter().enumerate() {
            if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                || (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL) != 0
                || !objective_ids.contains(&objective.object_id)
                || !represented_quest_objective_completable_like_cpp(status, quest, objective_index)
            {
                continue;
            }
            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                continue;
            };
            if status.objective_counts.len() <= storage_index {
                status.objective_counts.resize(storage_index + 1, 0);
            }
            let current = status.objective_counts[storage_index];
            if current >= objective.amount {
                continue;
            }
            let new_count = current.saturating_add(count).clamp(0, objective.amount);
            status.objective_counts[storage_index] = new_count;
            changed_quest_ids.push(status.quest_id);
            if new_count >= objective.amount
                && represented_can_complete_quest_after_objective_like_cpp(
                    status,
                    quest,
                    objective.id,
                    rewarded_quests.contains(&status.quest_id),
                )
            {
                quests_to_complete.push(status.quest_id);
            }
        }
    }
    for quest_id in quests_to_complete {
        if let Some(status) = player_quests.get_mut(&quest_id) {
            status.status = QUEST_STATUS_COMPLETE_LIKE_CPP;
        }
    }
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    changed_quest_ids
}

pub(crate) fn apply_quest_item_removed_to_statuses_like_cpp(
    quest_store: &QuestStore,
    player_quests: &mut HashMap<u32, PlayerQuestStatus>,
    entry_id: u32,
    new_non_bank_item_count: u32,
) -> Vec<u32> {
    let Ok(object_id) = i32::try_from(entry_id) else {
        return Vec::new();
    };
    let new_item_count = i32::try_from(new_non_bank_item_count).unwrap_or(i32::MAX);
    let mut changed_quest_ids = Vec::new();

    for status in player_quests.values_mut() {
        let Some(quest) = quest_store.get(status.quest_id) else {
            continue;
        };
        for (objective_index, objective) in quest.objectives.iter().enumerate() {
            if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
                || objective.object_id != object_id
                || !represented_quest_objective_completable_like_cpp(status, quest, objective_index)
            {
                continue;
            }
            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                continue;
            };
            if new_item_count >= objective.amount {
                continue;
            }
            if status.objective_counts.len() <= storage_index {
                status.objective_counts.resize(storage_index + 1, 0);
            }
            if status.objective_counts[storage_index] == new_item_count
                && status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
            {
                continue;
            }
            status.objective_counts[storage_index] = new_item_count.max(0);
            status.status = QUEST_STATUS_INCOMPLETE_LIKE_CPP;
            changed_quest_ids.push(status.quest_id);
        }
    }
    changed_quest_ids.sort_unstable();
    changed_quest_ids.dedup();
    changed_quest_ids
}

pub(crate) fn quest_reward_currency_gain_source_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> CurrencyGainSourceLikeCpp {
    if (quest.flags_ex & QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP) != 0 {
        if (quest.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
            return CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps;
        }

        return CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps;
    }

    if quest.is_daily_like_cpp() {
        CurrencyGainSourceLikeCpp::DailyQuestReward
    } else if quest.is_weekly_like_cpp() {
        CurrencyGainSourceLikeCpp::WeeklyQuestReward
    } else if (quest.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
        CurrencyGainSourceLikeCpp::WorldQuestReward
    } else {
        CurrencyGainSourceLikeCpp::QuestReward
    }
}

pub(crate) fn represented_accept_and_end_time_for_new_quest_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> (i64, i64) {
    let accept_time = GameTime::now().as_secs() as i64;
    let end_time = if quest.limit_time_secs > 0 {
        accept_time.saturating_add(quest.limit_time_secs)
    } else {
        0
    };
    (accept_time, end_time)
}

pub(crate) fn represented_can_complete_quest_after_objective_like_cpp(
    status: &PlayerQuestStatus,
    quest: &wow_data::quest::QuestTemplate,
    ignored_objective_id: u32,
    quest_already_rewarded: bool,
) -> bool {
    if quest.id == 0 {
        return false;
    }

    if !quest.is_repeatable() && quest_already_rewarded {
        return false;
    }

    if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
        return false;
    }

    for objective in &quest.objectives {
        if ignored_objective_id != 0 && objective.id == ignored_objective_id {
            continue;
        }

        if (objective.flags
            & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL
                | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL))
            != 0
        {
            continue;
        }

        if !represented_quest_objective_complete_like_cpp(status, quest, objective) {
            return false;
        }
    }

    if (quest.flags
        & (QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP | QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP))
        != 0
        && !status.explored
    {
        return false;
    }

    if quest.limit_time_secs > 0 && status.end_time_secs == 0 {
        return false;
    }

    true
}

pub(crate) fn represented_quest_objective_completable_like_cpp(
    status: &PlayerQuestStatus,
    quest: &wow_data::quest::QuestTemplate,
    objective_index: usize,
) -> bool {
    let Some(objective) = quest.objectives.get(objective_index) else {
        return false;
    };

    if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) != 0 {
        let Some((progress_bar_index, progress_bar_objective)) =
            quest.objectives.iter().enumerate().find(|(_, other)| {
                other.obj_type == QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL
                    && (other.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) == 0
            })
        else {
            return false;
        };

        return represented_quest_objective_completable_like_cpp(status, quest, progress_bar_index)
            && !represented_quest_objective_complete_like_cpp(
                status,
                quest,
                progress_bar_objective,
            );
    }

    if objective_index == 0 {
        return true;
    }

    let mut previous_index = objective_index - 1;
    let mut objective_sequence_satisfied = true;
    let mut previous_sequenced_objective_complete = false;
    let mut previous_sequenced_objective_index = None;

    loop {
        let previous_objective = &quest.objectives[previous_index];
        if (previous_objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL) != 0 {
            previous_sequenced_objective_index = Some(previous_index);
            previous_sequenced_objective_complete =
                represented_quest_objective_complete_like_cpp(status, quest, previous_objective);
            break;
        }

        if objective_sequence_satisfied {
            objective_sequence_satisfied =
                represented_quest_objective_complete_like_cpp(status, quest, previous_objective)
                    || (previous_objective.flags
                        & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL
                            | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL))
                        != 0;
        }

        if previous_index == 0 {
            break;
        }
        previous_index -= 1;
    }

    if (objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL) != 0 {
        if previous_sequenced_objective_index.is_none() {
            return objective_sequence_satisfied;
        }
        if !previous_sequenced_objective_complete || !objective_sequence_satisfied {
            return false;
        }
    } else if !previous_sequenced_objective_complete {
        if let Some(previous_sequenced_objective_index) = previous_sequenced_objective_index {
            if !represented_quest_objective_completable_like_cpp(
                status,
                quest,
                previous_sequenced_objective_index,
            ) {
                return false;
            }
        }
    }

    true
}

pub(crate) fn represented_quest_objective_complete_like_cpp(
    status: &PlayerQuestStatus,
    quest: &wow_data::quest::QuestTemplate,
    objective: &wow_data::quest::QuestObjective,
) -> bool {
    match objective.obj_type {
        QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_TALKTO_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP_LOCAL
        | QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP_LOCAL => {
            let Ok(storage_index) = usize::try_from(objective.storage_index) else {
                return false;
            };
            status
                .objective_counts
                .get(storage_index)
                .copied()
                .unwrap_or(0)
                >= objective.amount
        }
        QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL => {
            represented_quest_objective_progress_bar_complete_like_cpp(status, quest)
        }
        // Other objective completion sources need live runtime data. This helper is only
        // used as a guard before represented item-objective progress, so fail closed.
        _ => false,
    }
}

pub(crate) fn represented_quest_objective_progress_bar_complete_like_cpp(
    status: &PlayerQuestStatus,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    let mut progress = 0.0_f32;
    for objective in &quest.objectives {
        if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL) == 0 {
            continue;
        }

        let Ok(storage_index) = usize::try_from(objective.storage_index) else {
            continue;
        };
        let count = status
            .objective_counts
            .get(storage_index)
            .copied()
            .unwrap_or(0);
        progress += count as f32 * objective.progress_bar_weight;
        if progress >= 100.0 {
            return true;
        }
    }
    false
}
