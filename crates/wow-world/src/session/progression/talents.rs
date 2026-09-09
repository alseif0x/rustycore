//! Represented talents and glyph slots, and their published state.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_talent_store(&mut self, store: Arc<TalentStore>) {
        self.talent_store = Some(store);
    }
    pub(crate) fn talent_store(&self) -> Option<&Arc<TalentStore>> {
        self.talent_store.as_ref()
    }
    pub fn set_num_talents_at_level_store(&mut self, store: Arc<NumTalentsAtLevelStore>) {
        self.num_talents_at_level_store = Some(store);
        self.refresh_represented_talent_points_like_cpp();
    }
    pub(crate) fn num_talents_at_level_store(&self) -> Option<&Arc<NumTalentsAtLevelStore>> {
        self.num_talents_at_level_store.as_ref()
    }
    pub(crate) fn player_talent_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerTalentRuntimeState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.talent_runtime_like_cpp().clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(wow_entities::PlayerTalentRuntimeState {
                talent_groups: self.represented_talents_like_cpp.clone(),
                talents_loaded: self.represented_talents_loaded_like_cpp,
                glyph_groups: self.represented_glyphs_like_cpp,
                glyphs_loaded: self.represented_glyphs_loaded_like_cpp,
                active_group: self.represented_active_talent_group_like_cpp,
                bonus_groups: self.represented_bonus_talent_groups_like_cpp,
                reset_talents_cost: self.represented_talent_reset_cost_like_cpp,
                reset_talents_time_secs: self.represented_talent_reset_time_secs_like_cpp,
            });
        }
        canonical
    }
    #[cfg(test)]
    fn store_player_talent_fixture_like_cpp(
        &mut self,
        runtime: wow_entities::PlayerTalentRuntimeState,
    ) -> bool {
        if self.player_handle_like_cpp.is_none() {
            self.represented_talents_like_cpp = runtime.talent_groups;
            self.represented_talents_loaded_like_cpp = runtime.talents_loaded;
            self.represented_glyphs_like_cpp = runtime.glyph_groups;
            self.represented_glyphs_loaded_like_cpp = runtime.glyphs_loaded;
            self.represented_active_talent_group_like_cpp = runtime.active_group;
            self.represented_bonus_talent_groups_like_cpp = runtime.bonus_groups;
            self.represented_talent_reset_cost_like_cpp = runtime.reset_talents_cost;
            self.represented_talent_reset_time_secs_like_cpp = runtime.reset_talents_time_secs;
            return true;
        }
        false
    }
    pub(in crate::session) fn mutate_player_talent_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerTalentRuntimeState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut runtime = self.player_talent_runtime_snapshot_like_cpp()?;
            let result = f(&mut runtime);
            return self
                .store_player_talent_fixture_like_cpp(runtime)
                .then_some(result);
        }
        // C++ Player::AddTalent / SetGlyph mutate the Player-owned containers
        // (Player.cpp:2644-2695,25477-25481), not a Session write-back copy.
        self.with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().talents))
    }
    pub(crate) fn set_represented_active_talent_group_like_cpp(
        &mut self,
        active_group: u8,
    ) -> bool {
        let active_group = active_group.min((MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8);
        if self.represented_active_talent_group_like_cpp() != Some(active_group) {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.mutate_player_talent_runtime_like_cpp(|runtime| runtime.active_group = active_group)
            .is_some()
    }
    pub(crate) fn represented_active_talent_group_like_cpp(&self) -> Option<u8> {
        self.player_talent_runtime_snapshot_like_cpp()
            .map(|runtime| runtime.active_group)
    }
    pub(crate) fn set_represented_bonus_talent_groups_like_cpp(
        &mut self,
        bonus_groups: u8,
    ) -> bool {
        let bonus_groups = bonus_groups.min((MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8);
        self.mutate_player_talent_runtime_like_cpp(|runtime| runtime.bonus_groups = bonus_groups)
            .is_some()
    }
    pub(crate) fn reset_represented_talents_like_cpp(&mut self) {
        let _ = self.mutate_player_talent_runtime_like_cpp(|runtime| {
            for talents in &mut runtime.talent_groups {
                talents.clear();
            }
            runtime.talents_loaded = false;
        });
        // Login reconstructs a fresh C++ Player after this reset. Retaining the
        // previous character's runtime edges would affect GetCastSpellInfo, and
        // retaining per-spell traits could contaminate a coincident spell ID.
        let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.override_spells.clear();
            runtime.trait_definition_ids.clear();
            runtime.trait_config_rows.clear();
            runtime.trait_config_rows_complete = false;
            runtime.trait_entry_rows_complete = false;
            runtime.trait_entry_rows_empty = false;
        });
        self.invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp();
    }
    pub(crate) fn reset_represented_active_talents_like_cpp(&mut self) -> bool {
        let Some(talent_group) = self.represented_active_talent_group_like_cpp() else {
            return false;
        };
        let talent_group_index = usize::from(talent_group);
        if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
            return false;
        }

        let Some(active_talents) = self
            .mutate_player_talent_runtime_like_cpp(|runtime| {
                runtime
                    .talents_loaded
                    .then(|| std::mem::take(&mut runtime.talent_groups[talent_group_index]))
            })
            .flatten()
        else {
            return false;
        };
        for (talent_id, rank) in active_talents {
            self.remove_represented_active_talent_side_effects_like_cpp(talent_id, rank);
        }
        self.refresh_represented_talent_points_like_cpp();
        true
    }
    pub(crate) fn apply_represented_login_talent_reset_if_needed_like_cpp(&mut self) -> bool {
        const AT_LOGIN_RESET_TALENTS_LIKE_CPP: u16 = 0x004;

        if !self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_TALENTS_LIKE_CPP) != 0)
        {
            return false;
        }

        self.record_represented_talent_reset_script_hook_like_cpp(true);
        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_TALENTS_LIKE_CPP, true);
        self.remove_represented_pet_not_in_slot_like_cpp();

        if self.reset_represented_active_talents_like_cpp() {
            let Some(talent_data) = self.resolved_update_talent_data_packet_like_cpp() else {
                return false;
            };
            self.send_packet(&talent_data);
            self.send_notification_like_cpp(self.reset_talents_notification_text_like_cpp());
            return true;
        }

        false
    }
    pub(crate) fn learn_represented_talent_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        requested_rank: u16,
    ) -> bool {
        if !self.represented_talents_loaded_like_cpp() {
            return false;
        }

        let Ok(rank) = u8::try_from(requested_rank) else {
            return false;
        };

        if !self.validate_represented_talent_learn_like_cpp(talent_id, rank) {
            return false;
        }

        let Some(runtime) = self.player_talent_runtime_snapshot_like_cpp() else {
            return false;
        };
        let talent_group = runtime.active_group;
        let previous_rank = runtime
            .talent_groups
            .get(usize::from(talent_group))
            .and_then(|talents| talents.get(&talent_id).copied());
        let learned =
            self.load_represented_talent_row_like_cpp(talent_tabs, talent_id, rank, talent_group);
        if learned {
            self.apply_represented_active_talent_spell_side_effects_like_cpp(
                talent_id,
                previous_rank,
                rank,
                talent_group,
            );
            self.refresh_represented_talent_points_like_cpp();
        }
        learned
    }
    fn validate_represented_talent_learn_like_cpp(&self, talent_id: u32, rank: u8) -> bool {
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
        let talent_group_index = usize::from(runtime.active_group);
        if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
            return false;
        }

        let Some(talent) = self.talent_store().and_then(|store| store.get(talent_id)) else {
            return false;
        };

        let talents = &runtime.talent_groups[talent_group_index];
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
            let Some(talent_store) = self.talent_store() else {
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
        let talent_group_index = usize::from(runtime.active_group);
        let Some(talents) = runtime.talent_groups.get(talent_group_index) else {
            return u32::from(rank) + 1;
        };
        if let Some(current_rank) = talents.get(&talent_id) {
            (i32::from(*current_rank) - i32::from(rank) + 1).max(0) as u32
        } else {
            u32::from(rank) + 1
        }
    }
    #[cfg(test)]
    fn represented_spent_talent_points_count_like_cpp(&self) -> Option<u32> {
        let runtime = self.player_talent_runtime_snapshot_like_cpp()?;
        let talent_group_index = usize::from(runtime.active_group);
        Some(
            runtime
                .talent_groups
                .get(talent_group_index)
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
    #[cfg(test)]
    fn represented_calculate_talents_points_like_cpp(&self) -> Option<u32> {
        let base_points = self
            .num_talents_at_level_store()
            .map(|store| {
                store.num_talents_at_level_like_cpp(
                    u32::from(self.player_level_like_cpp()),
                    self.player_class_like_cpp(),
                )
            })
            .unwrap_or(0);
        Some(base_points + self.represented_quest_rewarded_talent_points_like_cpp()?)
    }
    pub(crate) fn refresh_represented_talent_points_like_cpp(&mut self) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let Some(spent) = self.represented_spent_talent_points_count_like_cpp() else {
                return;
            };
            let Some(available) = self
                .represented_calculate_talents_points_like_cpp()
                .map(|points| points.saturating_sub(spent))
            else {
                return;
            };
            self.set_player_character_points_like_cpp(available.min(i32::MAX as u32) as i32);
            return;
        }
        let base_points = self
            .num_talents_at_level_store()
            .map(|store| {
                store.num_talents_at_level_like_cpp(
                    u32::from(self.player_level_like_cpp()),
                    self.player_class_like_cpp(),
                )
            })
            .unwrap_or(0);
        // Borrow immutable catalog policy; Player owns counting and field mutation.
        let _points = self.with_owned_player_mut_like_cpp(|player| {
            player.refresh_represented_talent_points_like_cpp(base_points, |talent_id, rank| {
                self.represented_talent_info_like_cpp(talent_id, rank)
                    .is_some()
            })
        });
        #[cfg(test)]
        if let Some(points) = _points {
            self.player_character_points_like_cpp = points;
        }
    }
    pub(in crate::session) fn represented_talent_info_like_cpp(
        &self,
        talent_id: u32,
        rank: u8,
    ) -> Option<wow_packet::packets::misc::TalentInfoLikeCpp> {
        let talent = self.talent_store()?.get(talent_id)?;
        let spell_id = talent.spell_rank.get(usize::from(rank)).copied()?;
        if spell_id <= 0 {
            return None;
        }
        if !self.represented_spell_valid_for_talent_like_cpp(spell_id) {
            return None;
        }

        Some(wow_packet::packets::misc::TalentInfoLikeCpp { talent_id, rank })
    }
    pub(crate) fn resolved_update_talent_data_packet_like_cpp(
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
        let group_count = (1 + usize::from(runtime.bonus_groups)).min(MAX_SPECIALIZATIONS_LIKE_CPP);
        let mut groups = Vec::with_capacity(group_count);
        for (group_index, glyph_ids) in runtime
            .glyph_groups
            .iter()
            .take(group_count)
            .copied()
            .enumerate()
        {
            let talents = runtime.talent_groups[group_index]
                .iter()
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
            active_group: runtime.active_group,
            groups,
            is_pet_talents: false,
        })
    }
    #[cfg(test)]
    pub(crate) fn represented_update_talent_data_packet_like_cpp(
        &self,
    ) -> wow_packet::packets::misc::UpdateTalentData {
        self.build_update_talent_data_packet_like_cpp(self.player_character_points_like_cpp())
            .expect("test Player talent owner must resolve")
    }
    pub(crate) fn represented_next_reset_talents_cost_like_cpp(
        &self,
        now_secs: u64,
    ) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .talent_runtime_like_cpp()
                .next_reset_talents_cost_like_cpp(now_secs)
        });
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .player_talent_runtime_snapshot_like_cpp()
                .map(|runtime| runtime.next_reset_talents_cost_like_cpp(now_secs));
        }
        canonical
    }
    pub(in crate::session) fn record_represented_talent_respec_criteria_like_cpp(
        &mut self,
        cost: u32,
    ) {
        #[cfg(test)]
        self.represented_talent_respec_criteria_events_like_cpp
            .push(
                RepresentedTalentRespecCriteriaEventLikeCpp::MoneySpentOnRespecs { amount: cost },
            );
        #[cfg(test)]
        self.represented_talent_respec_criteria_events_like_cpp
            .push(RepresentedTalentRespecCriteriaEventLikeCpp::TotalRespecs { quantity: 1 });
        #[cfg(not(test))]
        let _ = cost;
    }
    fn reset_talents_notification_text_like_cpp(&self) -> String {
        let text = self.trinity_string_like_cpp(LANG_RESET_TALENTS_LIKE_CPP);
        if text == "<error>" {
            LANG_RESET_TALENTS_TEXT_LIKE_CPP.to_string()
        } else {
            text.to_string()
        }
    }
    pub(crate) fn set_represented_talent_reset_state_like_cpp(
        &mut self,
        reset_cost: u32,
        reset_time_secs: u64,
    ) -> bool {
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.reset_talents_cost = reset_cost;
            runtime.reset_talents_time_secs = reset_time_secs;
        })
        .is_some()
    }
    pub(crate) fn represented_talent_reset_cost_like_cpp(&self) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.talent_runtime_like_cpp().reset_talents_cost
        });
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.represented_talent_reset_cost_like_cpp);
        }
        canonical
    }
    pub(crate) fn represented_talent_reset_time_secs_like_cpp(&self) -> Option<u64> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.talent_runtime_like_cpp().reset_talents_time_secs
        });
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(self.represented_talent_reset_time_secs_like_cpp);
        }
        canonical
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_talent_reset_script_hook_like_cpp(&mut self, no_cost: bool) {
        #[cfg(test)]
        self.represented_talent_reset_script_hooks_like_cpp
            .push(RepresentedTalentResetScriptHookLikeCpp { no_cost });
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_reset_script_hooks_like_cpp(
        &self,
    ) -> &[RepresentedTalentResetScriptHookLikeCpp] {
        &self.represented_talent_reset_script_hooks_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_respec_criteria_events_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecCriteriaEventLikeCpp] {
        &self.represented_talent_respec_criteria_events_like_cpp
    }

    pub(crate) fn reset_represented_glyphs_like_cpp(&mut self) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _ = self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.glyph_groups = [[0; wow_entities::PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP];
                wow_entities::PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP];
            runtime.glyphs_loaded = false;
        });
    }
    pub(crate) fn represented_active_glyphs_packet_like_cpp(
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
