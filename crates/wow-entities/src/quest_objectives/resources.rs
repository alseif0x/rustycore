// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Threshold-based quest objective planning.
//!
//! Money, currency and reputation adapters all use the same C++ threshold
//! transition shape. This module only observes caller-owned status/catalog
//! values; it does not mutate Player state or publish completion packets.

use super::model::QuestObjectiveRulesLikeCpp;
use crate::PlayerQuestStatusRecord;
use std::collections::BTreeMap;
use wow_constants::quest::{QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThresholdQuestObjectiveChangeLikeCpp {
    pub quest_id: u32,
    pub objective_id: u32,
    pub required: i32,
    pub objective_was_complete: bool,
    pub objective_is_now_complete: bool,
}

/// Plan threshold transitions for an objective type and object identity.
/// `max_reputation` selects the inverse (`value <= required`) threshold used by
/// C++ `QUEST_OBJECTIVE_MAX_REPUTATION`; all other supported sources use `>=`.
pub fn plan_threshold_quest_objective_changes_like_cpp<'a>(
    statuses: &BTreeMap<u32, PlayerQuestStatusRecord>,
    mut quest_by_id: impl FnMut(u32) -> Option<QuestObjectiveRulesLikeCpp<'a>>,
    objective_type: u8,
    object_id: i32,
    previous_value: i64,
    next_value: i64,
    max_reputation: bool,
) -> Vec<ThresholdQuestObjectiveChangeLikeCpp> {
    let change = next_value.saturating_sub(previous_value);
    statuses
        .values()
        .filter(|status| {
            status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP
                || status.status == QUEST_STATUS_COMPLETE_LIKE_CPP
        })
        .flat_map(|status| {
            let quest = quest_by_id(status.quest_id)?;
            Some(quest.objectives.iter().filter_map(move |objective| {
                if objective.obj_type != objective_type || objective.object_id != object_id {
                    return None;
                }
                let objective_was_complete = if max_reputation {
                    previous_value <= i64::from(objective.amount)
                } else {
                    previous_value >= i64::from(objective.amount)
                };
                if objective_was_complete && change >= 0 {
                    return None;
                }
                let objective_is_now_complete = if max_reputation {
                    next_value <= i64::from(objective.amount)
                } else {
                    next_value >= i64::from(objective.amount)
                };
                Some(ThresholdQuestObjectiveChangeLikeCpp {
                    quest_id: status.quest_id,
                    objective_id: objective.id,
                    required: objective.amount,
                    objective_was_complete,
                    objective_is_now_complete,
                })
            }))
        })
        .flatten()
        .collect()
}
