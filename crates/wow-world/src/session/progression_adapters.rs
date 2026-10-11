// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Progression adapters: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP};
use super::{WorldSession, catalogs};

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn killed_player_credit_like_cpp(
        &mut self,
        victim_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.killed_player_credit_with_generator_like_cpp(generator.as_ref(), victim_guid)
            .await;
    }

    #[cfg(test)]
    pub(crate) async fn kill_credit_criteria_tree_objective_like_cpp(
        &mut self,
        criteria_tree_id: u32,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.kill_credit_criteria_tree_objective_with_generator_like_cpp(
            generator.as_ref(),
            criteria_tree_id,
        )
        .await;
    }

    /// Set the player XP table (xp required per level).
    #[cfg(test)]
    pub(crate) fn set_player_xp_table(&mut self, table: Arc<Vec<u32>>) {
        self.catalogs.player_xp_table = Some(table);
        self.refresh_next_level_xp();
    }

    #[cfg(test)]
    pub(crate) fn refresh_next_level_xp(&mut self) {
        let catalogs = self.progression_catalogs_for_test_like_cpp();
        crate::session::hub_mut(self).refresh_next_level_xp_with_catalogs_like_cpp(&catalogs);
    }

    #[cfg(test)]
    pub(crate) fn player_gold_like_cpp(&self) -> u64 {
        self.resolved_player_money_like_cpp()
            .or_else(|| {
                self.core
                    .player_handle_like_cpp
                    .is_none()
                    .then_some(self.inventory.player_gold_for_test_like_cpp())
            })
            .expect("test Player money owner must resolve")
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/progression_adapters/f3_shims.rs"]
mod f3_shims;
