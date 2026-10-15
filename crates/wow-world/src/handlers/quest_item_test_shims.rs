// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the item-objective, source-item and
//! eligibility bodies moved to `wow_world_quest::quest_session` (#1363 slice
//! 6b): the four `cfg(test)` wrappers that were nested in production impls of
//! `handlers/quest/objectives.rs` and `handlers/quest/state.rs`, moved
//! verbatim except that their one call names the moved free function; and
//! one-call method forwards for the moved functions the scenarios call, with
//! this session as the host, so no call site changes.

#![cfg(test)]

use wow_data::quest::QuestStore;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            entry_id,
            quest_log_item_id,
            count,
        )
        .await
    }

    pub(crate) async fn apply_quest_item_added_objective_progress_like_cpp(
        &mut self,
        entry_id: u32,
        quest_log_item_id: u32,
        count: u32,
    ) -> Vec<u32> {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::apply_quest_item_added_objective_progress_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            entry_id,
            quest_log_item_id,
            count,
        )
        .await
    }

    pub(super) async fn apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
        &mut self,
        quest_store: &QuestStore,
        object_id: i32,
        count_i32: i32,
    ) -> Vec<(u32, i32)> {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_quest::apply_quest_source_item_bound_objective_progress_for_object_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            quest_store,
            object_id,
            count_i32,
        )
        .await
    }

    pub(crate) async fn complete_represented_quest_after_add_if_ready_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        wow_world_quest::complete_represented_quest_after_add_with_generator_like_cpp(
            self,
            generator.as_ref(),
            quest,
        )
        .await
    }

    pub(crate) fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        wow_world_quest::can_take_quest(self, quest)
    }
    pub(crate) fn apply_quest_item_removed_like_cpp(&mut self, entry_id: u32) -> Option<Vec<u32>> {
        wow_world_quest::apply_quest_item_removed_like_cpp(self, entry_id)
    }
}
