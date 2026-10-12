// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

use crate::session::GivePlayerXpScriptDispatcherLikeCpp;

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn set_give_player_xp_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: GivePlayerXpScriptDispatcherLikeCpp,
    ) {
        self.config
            .set_give_player_xp_script_dispatcher_like_cpp(dispatcher)
    }
    pub(crate) fn resolved_championing_faction_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).resolved_championing_faction_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_represented_gray_level_script_override_like_cpp(
        &mut self,
        player_level: u8,
        gray_level: u8,
    ) {
        crate::session::hub_mut(self)
            .set_represented_gray_level_script_override_like_cpp(player_level, gray_level)
    }
    #[cfg(test)]
    pub fn set_exploration_xp_rate_like_cpp(&mut self, rate: f32) {
        self.config.set_exploration_xp_rate_like_cpp(rate)
    }
    #[cfg(test)]
    pub fn set_min_discovered_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.config
            .set_min_discovered_scaled_xp_ratio_like_cpp(ratio)
    }
    #[cfg(test)]
    pub(crate) fn player_scaling_level_delta_like_cpp(&self) -> i32 {
        crate::session::hub_ref(self).player_scaling_level_delta_like_cpp()
    }
    pub(crate) fn resolved_player_character_points_like_cpp(&self) -> Option<i32> {
        crate::session::hub_ref(self).resolved_player_character_points_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_character_points_like_cpp(&self) -> i32 {
        crate::session::hub_ref(self).player_character_points_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_xp_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).player_xp_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_next_level_xp_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).player_next_level_xp_like_cpp()
    }
    pub(crate) fn set_player_next_level_xp_like_cpp(&mut self, xp: u32) -> bool {
        crate::session::hub_mut(self).set_player_next_level_xp_like_cpp(xp)
    }
    pub(crate) fn resolved_player_next_level_xp_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).resolved_player_next_level_xp_like_cpp()
    }
    pub(crate) fn set_player_xp_like_cpp(&mut self, xp: u32) -> bool {
        crate::session::hub_mut(self).set_player_xp_like_cpp(xp)
    }
}

impl WorldSession {
    pub(crate) fn set_championing_faction_like_cpp(&mut self, faction_id: u32) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_championing_faction_like_cpp(faction_id);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.core.player_handle_like_cpp.is_none() {
            self.fixtures.progression.championing_faction_like_cpp = faction_id;
        }
    }
    pub(crate) async fn killed_player_credit_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        victim_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_PLAYERKILLS with ObjectID 0.
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            crate::session::quest::QuestObjectiveOperationLikeCpp::StoringValue {
                objective_type: QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
                object_id: 0,
                add_count: 1,
                credit_guid: victim_guid,
            },
        )
        .await;
    }
    pub(crate) async fn kill_credit_criteria_tree_objective_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        criteria_tree_id: u32,
    ) {
        // C++ QUEST_OBJECTIVE_CRITERIA_TREE.
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            crate::session::quest::QuestObjectiveOperationLikeCpp::StoringFlag {
                objective_type: 14,
                object_id: criteria_tree_id as i32,
                add_count: 1,
            },
        )
        .await;
    }
}
