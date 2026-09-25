// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player objective rules over borrowed definitions and caller-owned status records.
//!
//! Catalog loading, canonical Player mutation, persistence and publication remain
//! with their existing owners. There is no Session, data-store or clock dependency.

mod completion;
mod items;
mod model;
mod resources;

pub use completion::{
    player_has_incomplete_quest_objective_for_object_id_like_cpp,
    represented_can_complete_quest_after_objective_like_cpp,
    represented_quest_objective_completable_like_cpp,
    represented_quest_objective_complete_like_cpp,
};
pub use items::{
    apply_quest_item_added_bound_to_statuses_like_cpp,
    apply_quest_item_added_non_bound_to_statuses_like_cpp,
    apply_quest_item_removed_to_statuses_like_cpp,
};
pub use model::{QuestObjective, QuestObjectiveRulesLikeCpp};
pub use resources::{
    ThresholdQuestObjectiveChangeLikeCpp, plan_threshold_quest_objective_changes_like_cpp,
};

#[cfg(test)]
mod tests;
