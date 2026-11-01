// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the slice 7c progression families
//! (#1363). The battle-pet XP-percent aura moved to `wow_world_spell`; its
//! entry point forwards to the moved body with the session's spell-state
//! split, so the scenarios keep their method syntax. The other bodies are the
//! `#[cfg(test)]` methods that were nested in the production impls of the
//! rest, progression, skill, reputation, pet and combat-state files; they
//! moved verbatim, so the classifier counts them as test code and the session
//! hotspot's test lines do not grow.

#![cfg(test)]

use std::collections::BTreeSet;
use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_data::AreaTriggerDb2Store;

use crate::session::{
    PLAYER_FLAGS_RESTING_LIKE_CPP, RepresentedCombatStatRecalculationLikeCpp,
    RepresentedDeclinePetitionLikeCpp, RepresentedQueryPetitionLikeCpp,
    RepresentedQuestObjectiveProgressEventLikeCpp, RepresentedSignPetitionLikeCpp, WorldSession,
};

impl WorldSession {
    pub(crate) fn apply_represented_battle_pet_xp_pct_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.apply_represented_battle_pet_xp_pct_aura_like_cpp(
            &mut hub,
            spell_id,
            caster_guid,
            effect,
        )
    }

    // Rest state and XP rest bonus (from `session/rest_progression.rs`).
    pub(crate) fn replace_player_rest_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerRestState,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_for_rest_like_cpp(|player| {
                player.set_xp_rest_info_like_cpp(
                    state.rest_bonus_like_cpp().clamp(0.0, u32::MAX as f32) as u32,
                    state.rest_state_like_cpp(),
                );
                if state.is_location_initialized_like_cpp() {
                    if state.is_resting_by_flag_like_cpp() {
                        player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                    } else {
                        player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                    }
                }
                player.replace_rest_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_bonus_xp_like_cpp = state.rest_bonus_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_state_xp_like_cpp = state.rest_state_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_flag_mask_like_cpp = state.rest_flag_mask_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_location_initialized_like_cpp =
                state.is_location_initialized_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_defer_rest_flag_sync_like_cpp = state.defers_flag_sync_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_deferred_rest_flag_update_dirty_like_cpp =
                state.deferred_flag_update_dirty_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_inn_area_trigger_id_like_cpp = state.inn_trigger_id_like_cpp();
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_time_secs_like_cpp = state.rest_time_secs_like_cpp();
            return true;
        }
        canonical
    }
    pub(crate) fn set_represented_xp_rest_bonus_like_cpp(&mut self, rest_bonus: f32) -> u8 {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self.fixture_set_xp_rest_bonus_like_cpp(rest_bonus);
        }
        let at_max = crate::session::hub_ref(self).player_is_at_configured_max_level_like_cpp();
        let raf = self.represented_recruit_a_friend_xp_rest_state_applies_like_cpp();
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_xp_rest_bonus_like_cpp(rest_bonus, at_max, raf)
            })
            .unwrap_or(0)
    }
    pub(crate) fn apply_offline_xp_rest_bonus_like_cpp(
        &mut self,
        logout_time_secs: u64,
        now_secs: u64,
        was_logout_resting: bool,
    ) -> f32 {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.apply_offline_xp_rest_bonus_with_policy_like_cpp(
            &policy,
            logout_time_secs,
            now_secs,
            was_logout_resting,
        )
    }
    pub(crate) fn update_represented_online_xp_rest_bonus_like_cpp(
        &mut self,
        now_secs: u64,
    ) -> (f32, u8) {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.update_represented_online_xp_rest_bonus_with_policy_like_cpp(&policy, now_secs)
    }
    pub(crate) fn tick_represented_online_xp_rest_bonus_with_roll_like_cpp(
        &mut self,
        now_secs: u64,
        update_roll_passed: bool,
    ) {
        let policy = self.player_rest_rate_policy_for_test_like_cpp();
        self.tick_represented_online_xp_rest_bonus_with_roll_and_policy_like_cpp(
            &policy,
            now_secs,
            update_roll_passed,
        );
    }
    pub(crate) fn revalidate_represented_tavern_resting_like_cpp(&mut self) {
        let db2 = self
            .catalogs
            .area_trigger_db2_store
            .clone()
            .unwrap_or_else(|| Arc::new(AreaTriggerDb2Store::from_entries([])));
        self.revalidate_represented_tavern_resting_with_catalog_like_cpp(db2.as_ref());
    }
    pub(crate) fn represented_player_flags_for_create_like_cpp(&self) -> (u32, u32) {
        self.resolved_player_flags_for_create_like_cpp()
            .unwrap_or_else(|| {
                (
                    self.lifecycle
                        .represented_loaded_player_flags_for_test_like_cpp()
                        .unwrap_or(0),
                    self.lifecycle
                        .represented_loaded_player_flags_ex_for_test_like_cpp()
                        .unwrap_or(0),
                )
            })
    }

    // Kill credit, XP table and money (from `session/progression_adapters.rs`).
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
    pub(crate) fn set_player_xp_table(&mut self, table: Arc<Vec<u32>>) {
        self.catalogs.player_xp_table = Some(table);
        self.refresh_next_level_xp();
    }
    pub(crate) fn refresh_next_level_xp(&mut self) {
        let catalogs = self.progression_catalogs_for_test_like_cpp();
        crate::session::hub_mut(self).refresh_next_level_xp_with_catalogs_like_cpp(&catalogs);
    }
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

    // Skill tombstones (from `session/progression/skills.rs`).
    pub(crate) fn fixture_clear_player_skill_tombstones_like_cpp(&mut self) {
        let Some(records) = crate::session::hub_ref(self).resolved_player_skill_records_like_cpp()
        else {
            return;
        };
        let Some(loaded) = ({
            let (s, h) = crate::session::split_lifecycle_ref(self);
            s.resolved_player_skill_records_loaded_like_cpp(h)
        }) else {
            return;
        };
        let complete = crate::session::hub_ref(self)
            .complete_player_skill_records_like_cpp()
            .is_some();
        let occupied =
            crate::session::hub_ref(self).complete_player_skill_occupied_slots_like_cpp();
        let _ = self.replace_player_skill_runtime_exact_like_cpp(
            records,
            loaded,
            complete,
            occupied,
            BTreeSet::new(),
        );
    }

    // Reputation (from `session/progression/reputation.rs`).
    pub(crate) fn apply_represented_first_login_reputation_like_cpp(&mut self) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        crate::session::hub_mut(self)
            .apply_represented_first_login_reputation_with_catalogs_like_cpp(&player_bootstrap)
    }
    pub(crate) async fn reputation_changed_like_cpp(&mut self, faction_id: u32, change: i32) {
        self.quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                    faction_id,
                    change,
                },
            );
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }

    // Petitions and the tapper-pet kill hook (from `session/pets/pet.rs`).
    pub(crate) fn represented_sign_petitions_like_cpp(&self) -> &[RepresentedSignPetitionLikeCpp] {
        self.social.represented_sign_petitions_for_test_like_cpp()
    }
    pub(crate) fn represented_decline_petitions_like_cpp(
        &self,
    ) -> &[RepresentedDeclinePetitionLikeCpp] {
        self.social
            .represented_decline_petitions_for_test_like_cpp()
    }
    pub(crate) fn represented_query_petitions_like_cpp(
        &self,
    ) -> &[RepresentedQueryPetitionLikeCpp] {
        self.social.represented_query_petitions_for_test_like_cpp()
    }

    // Combat stat recalculations (from `session/combat/state.rs`).
    pub(crate) fn represented_combat_stat_recalculations_like_cpp(
        &self,
    ) -> &[RepresentedCombatStatRecalculationLikeCpp] {
        self.inventory
            .represented_combat_stat_recalculations_for_test_like_cpp()
    }
}
