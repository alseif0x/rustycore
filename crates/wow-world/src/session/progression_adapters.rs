// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Progression adapters: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use super::GivePlayerXpScriptDispatcherLikeCpp;
use super::{Arc, ObjectGuid, QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP};
use super::{WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP, WorldSession, catalogs};

impl WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn set_give_player_xp_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: GivePlayerXpScriptDispatcherLikeCpp,
    ) {
        self.give_player_xp_script_dispatcher_like_cpp = Some(dispatcher);
    }

    pub(crate) fn set_championing_faction_like_cpp(&mut self, faction_id: u32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_championing_faction_like_cpp(faction_id);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.championing_faction_like_cpp = faction_id;
        }
    }

    pub(crate) fn resolved_championing_faction_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.gameplay_state().championing_faction_id);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.championing_faction_like_cpp);
        }
        canonical
    }

    /// Level at which mobs give 0 XP ("gray") — C++ `Trinity::XP::GetGrayLevel`.
    pub(crate) fn gray_level(&self, pl: u8) -> u8 {
        let level = if pl < 7 {
            0
        } else if pl < 35 {
            let count = (15..=pl).filter(|level| level % 5 == 0).count() as u8;
            (pl - 7).saturating_sub(count.saturating_sub(1))
        } else {
            pl.saturating_sub(10)
        };
        #[cfg(test)]
        let level = self
            .represented_gray_level_script_overrides_like_cpp
            .get(&pl)
            .copied()
            .unwrap_or(level);
        level
    }

    #[cfg(test)]
    pub(crate) fn set_represented_gray_level_script_override_like_cpp(
        &mut self,
        player_level: u8,
        gray_level: u8,
    ) {
        self.represented_gray_level_script_overrides_like_cpp
            .insert(player_level, gray_level);
        if self.player_level_like_cpp() == player_level {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                player.gameplay_state_mut().gray_level = gray_level;
            });
        }
    }

    /// Zero-difference table — C++ `Trinity::XP::GetZeroDifference`.
    pub(in crate::session) fn zero_difference(&self, pl: u8) -> u8 {
        match pl {
            0..=3 => 5,
            4..=9 => 6,
            10..=11 => 7,
            12..=15 => 8,
            16..=19 => 9,
            20..=29 => 11,
            30..=39 => 12,
            40..=44 => 13,
            45..=49 => 14,
            50..=54 => 15,
            55..=59 => 16,
            _ => 17,
        }
    }

    pub(in crate::session) fn represented_championing_faction_for_kill_like_cpp(
        &self,
    ) -> Option<u32> {
        let championing_faction = self.resolved_championing_faction_like_cpp()?;
        if championing_faction == 0 {
            return None;
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        if !self
            .map_store()
            .and_then(|store| store.get(map_id))
            .is_some_and(|entry| entry.is_non_raid_dungeon_like_cpp())
        {
            return None;
        }
        let difficulty_id = self.current_map_difficulty_id_like_cpp();
        let is_wrath_max_level_lfg = self
            .lfg_dungeons_store
            .as_ref()
            .and_then(|store| store.get_by_map_and_difficulty_like_cpp(map_id, difficulty_id))
            .is_some_and(|dungeon| {
                dungeon.target_level == WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP
            });
        is_wrath_max_level_lfg.then_some(championing_faction)
    }

    pub(crate) async fn killed_player_credit_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        victim_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_PLAYERKILLS with ObjectID 0.
        self.update_represented_storing_value_quest_objective_progress_like_cpp(
            item_guid_generator,
            QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP,
            0,
            1,
            victim_guid,
        )
        .await;
    }

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

    pub(crate) async fn kill_credit_criteria_tree_objective_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        criteria_tree_id: u32,
    ) {
        // C++ QUEST_OBJECTIVE_CRITERIA_TREE.
        self.update_represented_storing_flag_quest_objective_progress_like_cpp(
            item_guid_generator,
            14,
            criteria_tree_id as i32,
            1,
        )
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
    pub fn set_player_xp_table(&mut self, table: Arc<Vec<u32>>) {
        self.player_xp_table = Some(table);
        self.refresh_next_level_xp();
    }

    #[cfg(test)]
    pub fn set_exploration_xp_rate_like_cpp(&mut self, rate: f32) {
        self.exploration_xp_rate_like_cpp = rate.max(0.0);
    }

    #[cfg(test)]
    pub fn set_min_discovered_scaled_xp_ratio_like_cpp(&mut self, ratio: u32) {
        self.min_discovered_scaled_xp_ratio_like_cpp = ratio.min(100);
    }

    #[cfg(test)]
    pub(crate) fn refresh_next_level_xp(&mut self) {
        let catalogs = self.progression_catalogs_for_test_like_cpp();
        self.refresh_next_level_xp_with_catalogs_like_cpp(&catalogs);
    }

    /// C++ `Player::SetXP` updates this field every time XP changes. It uses
    /// the client build's compile-time `MAX_LEVEL`, not the configurable or
    /// account-expansion-specific active maximum.
    pub(crate) fn resolved_player_scaling_level_delta_like_cpp(&self) -> Option<i32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.active_data().scaling_player_level_delta);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                if self.player_level_like_cpp() < WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP
                    && self.player_xp < self.player_next_level_xp / 2
                {
                    -1
                } else {
                    0
                },
            );
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_scaling_level_delta_like_cpp(&self) -> i32 {
        self.resolved_player_scaling_level_delta_like_cpp()
            .expect("test Player progression owner must resolve")
    }

    pub(crate) fn set_player_xp_like_cpp(&mut self, xp: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let xp = xp.min(i32::MAX as u32) as i32;
                player.set_xp(xp);
                player.mark_xp_changed_like_cpp();
                let scaling_level_delta = if player.unit().data().level
                    < i32::from(WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP)
                    && xp < player.active_data().next_level_xp / 2
                {
                    -1
                } else {
                    0
                };
                player.set_scaling_player_level_delta_like_cpp(scaling_level_delta);
                player.mark_scaling_player_level_delta_changed_like_cpp();
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_xp = xp;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_next_level_xp_like_cpp(&mut self, xp: u32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                let next_level_xp = xp.min(i32::MAX as u32) as i32;
                player.set_next_level_xp(next_level_xp);
                // Rust hydrates the Character row before its XP table refresh,
                // while C++ has NextLevelXP ready before `SetXP`. Recompute the
                // dependent SetXP field here so the final canonical value is
                // independent of that transitional load ordering.
                let scaling_level_delta = if player.unit().data().level
                    < i32::from(WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP)
                    && player.active_data().xp < next_level_xp / 2
                {
                    -1
                } else {
                    0
                };
                player.set_scaling_player_level_delta_like_cpp(scaling_level_delta);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_next_level_xp = xp;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_selection_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_selection(guid.unwrap_or_default()))
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.selection_guid = guid;
        }
    }

    #[cfg(test)]
    pub(crate) fn player_gold_like_cpp(&self) -> u64 {
        self.resolved_player_money_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_gold)
            })
            .expect("test Player money owner must resolve")
    }

    pub(crate) fn resolved_player_character_points_like_cpp(&self) -> Option<i32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.active_data().character_points);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_character_points_like_cpp);
        }
        canonical
    }

    pub(crate) fn resolved_player_xp_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.active_data().xp.max(0) as u32);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_xp);
        }
        canonical
    }

    pub(in crate::session) fn resolved_player_xp_for_level_like_cpp(
        &self,
        level: u8,
    ) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.player_xp_for_level_like_cpp(level))
            .flatten();
        #[cfg(test)]
        if canonical.is_none() {
            return self
                .player_xp_table
                .as_ref()
                .and_then(|table| table.get(usize::from(level)).copied())
                .or_else(|| self.resolved_player_next_level_xp_like_cpp());
        }
        canonical
    }

    pub(crate) fn resolved_player_next_level_xp_like_cpp(&self) -> Option<u32> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.active_data().next_level_xp.max(0) as u32);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_next_level_xp);
        }
        canonical
    }

    #[cfg(test)]
    pub(crate) fn player_character_points_like_cpp(&self) -> i32 {
        self.resolved_player_character_points_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_character_points_like_cpp)
            })
            .expect("test Player progression owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn player_xp_like_cpp(&self) -> u32 {
        self.resolved_player_xp_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_xp)
            })
            .expect("test Player progression owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn player_next_level_xp_like_cpp(&self) -> u32 {
        self.resolved_player_next_level_xp_like_cpp()
            .or_else(|| {
                self.player_handle_like_cpp
                    .is_none()
                    .then_some(self.player_next_level_xp)
            })
            .expect("test Player progression owner must resolve")
    }

    #[allow(dead_code)]
    pub(crate) fn selection_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let canonical = self.with_owned_player_like_cpp(|player| player.unit().data().target);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.selection_guid;
        }
        canonical.filter(|guid| !guid.is_empty())
    }
}
