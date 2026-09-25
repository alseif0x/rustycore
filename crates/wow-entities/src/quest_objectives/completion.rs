// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free quest objective completion rules.
//!
//! The rules consume a borrowed quest definition and a caller-owned Player
//! status record. Catalog loading, status ownership, and completion effects
//! remain with their existing application owners.

use super::model::{QuestObjective, QuestObjectiveRulesLikeCpp};
use crate::PlayerQuestStatusRecord;
use std::collections::BTreeMap;
use wow_constants::quest::{
    QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP, QUEST_FLAGS_COMPLETION_EVENT_LIKE_CPP,
    QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP, QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP,
    QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP, QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP,
    QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP, QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP,
    QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP, QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_MONSTER_LIKE_CPP, QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
    QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP, QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP,
    QUEST_OBJECTIVE_TALKTO_LIKE_CPP, QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP,
    QUEST_STATUS_INCOMPLETE_LIKE_CPP,
};

pub fn represented_can_complete_quest_after_objective_like_cpp(
    status: &PlayerQuestStatusRecord,
    quest: &QuestObjectiveRulesLikeCpp<'_>,
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

    for objective in quest.objectives {
        if ignored_objective_id != 0 && objective.id == ignored_objective_id {
            continue;
        }

        if (objective.flags
            & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP
                | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP))
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

pub fn represented_quest_objective_completable_like_cpp(
    status: &PlayerQuestStatusRecord,
    quest: &QuestObjectiveRulesLikeCpp<'_>,
    objective_index: usize,
) -> bool {
    let Some(objective) = quest.objectives.get(objective_index) else {
        return false;
    };

    if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP) != 0 {
        let Some((progress_bar_index, progress_bar_objective)) =
            quest.objectives.iter().enumerate().find(|(_, other)| {
                other.obj_type == QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP
                    && (other.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP) == 0
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
        if (previous_objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP) != 0 {
            previous_sequenced_objective_index = Some(previous_index);
            previous_sequenced_objective_complete =
                represented_quest_objective_complete_like_cpp(status, quest, previous_objective);
            break;
        }

        if objective_sequence_satisfied {
            objective_sequence_satisfied =
                represented_quest_objective_complete_like_cpp(status, quest, previous_objective)
                    || (previous_objective.flags
                        & (QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP
                            | QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP))
                        != 0;
        }

        if previous_index == 0 {
            break;
        }
        previous_index -= 1;
    }

    if (objective.flags & QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP) != 0 {
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

pub fn represented_quest_objective_complete_like_cpp(
    status: &PlayerQuestStatusRecord,
    quest: &QuestObjectiveRulesLikeCpp<'_>,
    objective: &QuestObjective,
) -> bool {
    match objective.obj_type {
        QUEST_OBJECTIVE_MONSTER_LIKE_CPP
        | QUEST_OBJECTIVE_ITEM_LIKE_CPP
        | QUEST_OBJECTIVE_GAMEOBJECT_LIKE_CPP
        | QUEST_OBJECTIVE_TALKTO_LIKE_CPP
        | QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP
        | QUEST_OBJECTIVE_WINPVPPETBATTLES_LIKE_CPP
        | QUEST_OBJECTIVE_CRITERIA_TREE_LIKE_CPP
        | QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP
        | QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP
        | QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP => {
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
        QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP => {
            represented_quest_objective_progress_bar_complete_like_cpp(status, quest)
        }
        // Only represented counters and progress bars are available in this rule.
        // Other completion sources need live runtime data and still fail closed.
        _ => false,
    }
}

pub(super) fn represented_quest_objective_progress_bar_complete_like_cpp(
    status: &PlayerQuestStatusRecord,
    quest: &QuestObjectiveRulesLikeCpp<'_>,
) -> bool {
    let mut progress = 0.0_f32;
    for objective in quest.objectives {
        if (objective.flags & QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP) == 0 {
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

pub fn player_has_incomplete_quest_objective_for_object_id_like_cpp<'a>(
    statuses: &BTreeMap<u32, PlayerQuestStatusRecord>,
    mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
    object_id: i32,
) -> bool {
    statuses.values().any(|status| {
        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            return false;
        }
        let Some(quest) = quest_by_id(status.quest_id) else {
            return false;
        };
        quest
            .objectives
            .iter()
            .enumerate()
            .any(|(fallback_index, objective)| {
                if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP
                    || objective.object_id != object_id
                {
                    return false;
                }
                let storage_index = usize::try_from(objective.storage_index)
                    .ok()
                    .unwrap_or(fallback_index);
                status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0)
                    < objective.amount.max(1)
            })
    })
}
