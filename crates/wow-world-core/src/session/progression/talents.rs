// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented talents and glyph slots, and their published state.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use std::sync::Arc;

use super::MAX_SPECIALIZATIONS_LIKE_CPP;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    RepresentedTalentResetScriptHookLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
};
use wow_data::progression_rewards::NumTalentsAtLevelStore;
use wow_data::TalentStore;

const NEEDED_TALENT_POINT_PER_TIER_LIKE_CPP: u32 = 5;

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::ProgressionState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_talent_reset_script_hooks_like_cpp(
        &self,
    ) -> &[RepresentedTalentResetScriptHookLikeCpp] {
        &self.represented_talent_reset_script_hooks_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_talent_respec_criteria_events_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecCriteriaEventLikeCpp] {
        &self.represented_talent_respec_criteria_events_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub fn validate_represented_talent_learn_like_cpp(&self, talent_id: u32, rank: u8) -> bool {
        let Some(available_points) = self.resolved_player_character_points_like_cpp() else {
            return false;
        };
        let available_points = available_points.max(0) as u32;
        if available_points == 0 {
            return false;
        }

        let Some(runtime) = self.player_talent_runtime_snapshot_like_cpp() else {
            return false;
        };
        let active_group = runtime.active_group_like_cpp();
        let Some(talent) = self
            .catalogs
            .talent_store()
            .and_then(|store| store.get(talent_id))
        else {
            return false;
        };

        let Some(talents) = runtime.talent_group_like_cpp(active_group) else {
            return false;
        };
        if let Some(current_rank) = talents.get(&talent_id) {
            if *current_rank >= rank {
                return false;
            }
        }

        let needed_talent_points =
            self.represented_needed_talent_points_for_learn_like_cpp(talent_id, rank);
        if needed_talent_points > available_points {
            return false;
        }

        for (prereq_talent, prereq_rank) in talent.prereq_talent.iter().zip(talent.prereq_rank) {
            let Ok(prereq_talent_id) = u32::try_from(*prereq_talent) else {
                return false;
            };
            if prereq_talent_id == 0 {
                continue;
            }

            let Ok(required_rank) = u8::try_from(prereq_rank) else {
                return false;
            };
            if talents
                .get(&prereq_talent_id)
                .is_none_or(|known_rank| *known_rank < required_rank)
            {
                return false;
            }
        }

        if talent.tier_id > 0 {
            let Some(talent_store) = self.catalogs.talent_store() else {
                return false;
            };
            let spent_points = talent_store
                .iter()
                .filter(|entry| entry.tab_id == talent.tab_id)
                .filter_map(|entry| {
                    talents
                        .get(&entry.id)
                        .map(|rank| {
                            entry
                                .spell_rank
                                .get(usize::from(*rank))
                                .copied()
                                .unwrap_or(0)
                        })
                        .filter(|spell_id| *spell_id != 0)
                        .map(|_| u32::from(talents[&entry.id]) + 1)
                })
                .sum::<u32>();

            if spent_points < u32::from(talent.tier_id) * NEEDED_TALENT_POINT_PER_TIER_LIKE_CPP {
                return false;
            }
        }

        true
    }

    fn represented_needed_talent_points_for_learn_like_cpp(&self, talent_id: u32, rank: u8) -> u32 {
        let Some(runtime) = self.player_talent_runtime_snapshot_like_cpp() else {
            return u32::from(rank) + 1;
        };
        let Some(talents) = runtime.talent_group_like_cpp(runtime.active_group_like_cpp()) else {
            return u32::from(rank) + 1;
        };
        if let Some(current_rank) = talents.get(&talent_id) {
            (i32::from(*current_rank) - i32::from(rank) + 1).max(0) as u32
        } else {
            u32::from(rank) + 1
        }
    }

    pub fn resolved_update_talent_data_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::UpdateTalentData> {
        self.build_update_talent_data_packet_like_cpp(
            self.resolved_player_character_points_like_cpp()?,
        )
    }

    fn build_update_talent_data_packet_like_cpp(
        &self,
        character_points: i32,
    ) -> Option<wow_packet::packets::misc::UpdateTalentData> {
        let runtime = self.player_talent_runtime_snapshot_like_cpp()?;
        let group_count =
            (1 + usize::from(runtime.bonus_groups_like_cpp())).min(MAX_SPECIALIZATIONS_LIKE_CPP);
        let mut groups = Vec::with_capacity(group_count);
        for (group_index, glyph_ids) in runtime
            .glyph_groups_like_cpp()
            .take(group_count)
            .copied()
            .enumerate()
        {
            let talents = runtime
                .talent_group_like_cpp(group_index as u8)
                .map(|talents| talents.iter().collect::<Vec<_>>())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(talent_id, rank)| {
                    self.represented_talent_info_like_cpp(*talent_id, *rank)
                })
                .collect();
            groups.push(wow_packet::packets::misc::TalentGroupInfoLikeCpp {
                spec_id: MAX_SPECIALIZATIONS_LIKE_CPP as u8,
                talents,
                glyph_ids,
            });
        }

        Some(wow_packet::packets::misc::UpdateTalentData {
            unspent_talent_points: character_points.max(0) as u32,
            active_group: runtime.active_group_like_cpp(),
            groups,
            is_pet_talents: false,
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_update_talent_data_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::UpdateTalentData {
        self.build_update_talent_data_packet_like_cpp(self.player_character_points_like_cpp())
            .expect("test Player talent owner must resolve")
    }

    pub fn represented_next_reset_talents_cost_like_cpp(
        &self,
        now_secs: u64,
    ) -> Option<u32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .talent_runtime_like_cpp()
                .next_reset_talents_cost_like_cpp(now_secs)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .player_talent_runtime_snapshot_like_cpp()
                .map(|runtime| runtime.next_reset_talents_cost_like_cpp(now_secs));
        }
        canonical
    }

    pub fn represented_talent_reset_cost_like_cpp(&self) -> Option<u32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .talent_runtime_like_cpp()
                .reset_talents_cost_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .progression
                    .represented_talent_reset_cost_like_cpp,
            );
        }
        canonical
    }

    pub fn represented_talent_reset_time_secs_like_cpp(&self) -> Option<u64> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player
                .talent_runtime_like_cpp()
                .reset_talents_time_secs_like_cpp()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.fixtures
                    .progression
                    .represented_talent_reset_time_secs_like_cpp,
            );
        }
        canonical
    }

    pub fn represented_active_glyphs_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::ActiveGlyphs {
        // C++ maps active glyphs to bindable spell ids through GlyphBindableSpell.db2.
        // That store is not session-wired yet, so this remains an intentionally empty
        // full update while UpdateTalentData carries the loaded glyph ids.
        wow_packet::packets::misc::ActiveGlyphs {
            glyphs: Vec::new(),
            is_full_update: true,
        }
    }
}

impl crate::session::HubMut<'_> {
    /// The owner-dispatch hook the canonical-ownership regressions drive.
    ///
    /// Production has no such caller: every transition goes through a named
    /// operation. This exists so the active/detached/replacement coverage can
    /// still exercise the dispatch itself (#752).
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_talent_runtime_for_test_like_cpp<R>(
        &mut self,
        apply: impl FnOnce(&mut wow_entities::PlayerTalentRuntimeState) -> R,
    ) -> Option<R> {
        self.mutate_player_talent_runtime_like_cpp(apply)
    }

    /// C++ `Player::_LoadGlyphs` storing one persisted slot (`Player.cpp:26573`).
    pub fn install_loaded_glyph_like_cpp(
        &mut self,
        talent_group: u8,
        glyph_slot: u8,
        glyph_id: u16,
    ) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.set_glyph_like_cpp(talent_group, glyph_slot, glyph_id)
        })
        .unwrap_or(false)
    }

    /// C++ `Player::_LoadTalents` completing.
    pub fn mark_talents_loaded_like_cpp(&mut self) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.mark_talents_loaded_like_cpp();
        })
        .is_some()
    }

    /// C++ `Player::_LoadGlyphs` completing.
    pub fn mark_glyphs_loaded_like_cpp(&mut self) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.mark_glyphs_loaded_like_cpp();
        })
        .is_some()
    }

    /// Install the groups a committed talent reset leaves behind
    /// (C++ `Player::ResetTalents`, `Player.cpp:3505`).
    pub fn install_reset_talent_groups_like_cpp(
        &mut self,
        groups: [std::collections::BTreeMap<u32, u8>;
            wow_entities::PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP],
    ) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.replace_talent_groups_like_cpp(groups);
        })
        .is_some()
    }

    pub fn set_represented_active_talent_group_like_cpp(
        &mut self,
        active_group: u8,
    ) -> bool {
        let active_group = active_group.min((MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8);
        if self.shared().represented_active_talent_group_like_cpp() != Some(active_group) {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.set_active_group_like_cpp(active_group);
        })
        .is_some()
    }

    pub fn set_represented_bonus_talent_groups_like_cpp(
        &mut self,
        bonus_groups: u8,
    ) -> bool {
        let bonus_groups = bonus_groups.min((MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8);
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.set_bonus_groups_like_cpp(bonus_groups);
        })
        .is_some()
    }

    pub fn record_represented_talent_respec_criteria_like_cpp(
        &mut self,
        cost: u32,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .progression
            .represented_talent_respec_criteria_events_like_cpp
            .push(
                RepresentedTalentRespecCriteriaEventLikeCpp::MoneySpentOnRespecs { amount: cost },
            );
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .progression
            .represented_talent_respec_criteria_events_like_cpp
            .push(RepresentedTalentRespecCriteriaEventLikeCpp::TotalRespecs { quantity: 1 });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = cost;
    }

    pub fn set_represented_talent_reset_state_like_cpp(
        &mut self,
        reset_cost: u32,
        reset_time_secs: u64,
    ) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.set_reset_talents_state_like_cpp(reset_cost, reset_time_secs);
        })
        .is_some()
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_talent_reset_script_hook_like_cpp(&mut self, no_cost: bool) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .progression
            .represented_talent_reset_script_hooks_like_cpp
            .push(RepresentedTalentResetScriptHookLikeCpp { no_cost });
    }

    pub fn reset_represented_glyphs_like_cpp(&mut self) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _ = self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.clear_glyphs_like_cpp();
        });
    }
}

impl crate::session::HubMut<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    fn store_player_talent_fixture_like_cpp(
        &mut self,
        runtime: wow_entities::PlayerTalentRuntimeState,
    ) -> bool {
        if self.core.player_handle_like_cpp.is_none() {
            self.fixtures.progression.represented_talents_like_cpp =
                runtime.talent_groups_snapshot_like_cpp();
            self.fixtures
                .progression
                .represented_talents_loaded_like_cpp = runtime.talents_loaded_like_cpp();
            self.fixtures.progression.represented_glyphs_like_cpp =
                runtime.glyph_groups_snapshot_like_cpp();
            self.fixtures.progression.represented_glyphs_loaded_like_cpp =
                runtime.glyphs_loaded_like_cpp();
            self.fixtures
                .progression
                .represented_active_talent_group_like_cpp = runtime.active_group_like_cpp();
            self.fixtures
                .progression
                .represented_bonus_talent_groups_like_cpp = runtime.bonus_groups_like_cpp();
            self.fixtures
                .progression
                .represented_talent_reset_cost_like_cpp = runtime.reset_talents_cost_like_cpp();
            self.fixtures
                .progression
                .represented_talent_reset_time_secs_like_cpp =
                runtime.reset_talents_time_secs_like_cpp();
            return true;
        }
        false
    }

    /// Incarnation dispatch for one named talent transition.
    ///
    /// The two talent-reset adapters retained in World use this bridge until
    /// F5. Other transitions go through the named operations below and on
    /// `PlayerTalentRuntimeState` (#752).
    pub fn mutate_player_talent_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerTalentRuntimeState) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let mut runtime = self.shared().player_talent_runtime_snapshot_like_cpp()?;
            let result = f(&mut runtime);
            return self
                .store_player_talent_fixture_like_cpp(runtime)
                .then_some(result);
        }
        // C++ Player::AddTalent / SetGlyph mutate the Player-owned containers
        // (Player.cpp:2644-2695,25477-25481), not a Session write-back copy.
        self.core
            .with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().talents))
    }

    /// C++ `Player::_LoadTalents` storing one persisted row (`Player.cpp:26623`).
    pub fn install_loaded_talent_row_like_cpp(
        &mut self,
        talent_group: u8,
        talent_id: u32,
        rank: u8,
    ) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.add_talent_like_cpp(talent_group, talent_id, rank)
        })
        .unwrap_or(false)
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_talent_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerTalentRuntimeState> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.talent_runtime_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let mut runtime = wow_entities::PlayerTalentRuntimeState::default();
            runtime.replace_talent_groups_like_cpp(
                self.fixtures
                    .progression
                    .represented_talents_like_cpp
                    .clone(),
            );
            runtime.replace_glyph_groups_like_cpp(
                self.fixtures.progression.represented_glyphs_like_cpp,
            );
            if self
                .fixtures
                .progression
                .represented_talents_loaded_like_cpp
            {
                runtime.mark_talents_loaded_like_cpp();
            }
            if self.fixtures.progression.represented_glyphs_loaded_like_cpp {
                runtime.mark_glyphs_loaded_like_cpp();
            }
            runtime.set_active_group_like_cpp(
                self.fixtures
                    .progression
                    .represented_active_talent_group_like_cpp,
            );
            runtime.set_bonus_groups_like_cpp(
                self.fixtures
                    .progression
                    .represented_bonus_talent_groups_like_cpp,
            );
            runtime.set_reset_talents_state_like_cpp(
                self.fixtures
                    .progression
                    .represented_talent_reset_cost_like_cpp,
                self.fixtures
                    .progression
                    .represented_talent_reset_time_secs_like_cpp,
            );
            return Some(runtime);
        }
        canonical
    }

    pub fn represented_active_talent_group_like_cpp(&self) -> Option<u8> {
        self.player_talent_runtime_snapshot_like_cpp()
            .map(|runtime| runtime.active_group_like_cpp())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_spent_talent_points_count_like_cpp(&self) -> Option<u32> {
        let runtime = self.player_talent_runtime_snapshot_like_cpp()?;
        Some(
            runtime
                .talent_group_like_cpp(runtime.active_group_like_cpp())
                .into_iter()
                .flat_map(|talents| talents.iter())
                .filter(|(talent_id, rank)| {
                    self.represented_talent_info_like_cpp(**talent_id, **rank)
                        .is_some()
                })
                .map(|(_, rank)| u32::from(*rank) + 1)
                .sum(),
        )
    }

    pub fn represented_talent_info_like_cpp(
        &self,
        talent_id: u32,
        rank: u8,
    ) -> Option<wow_packet::packets::misc::TalentInfoLikeCpp> {
        let talent = self.catalogs.talent_store()?.get(talent_id)?;
        let spell_id = talent.spell_rank.get(usize::from(rank)).copied()?;
        if spell_id <= 0 {
            return None;
        }
        if !self.represented_spell_valid_for_talent_like_cpp(spell_id) {
            return None;
        }

        Some(wow_packet::packets::misc::TalentInfoLikeCpp { talent_id, rank })
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn talent_store(&self) -> Option<&Arc<TalentStore>> {
        self.talent_store.as_ref()
    }

    pub fn num_talents_at_level_store(&self) -> Option<&Arc<NumTalentsAtLevelStore>> {
        self.num_talents_at_level_store.as_ref()
    }
}
