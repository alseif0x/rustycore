//! Represented talents and glyph slots, and their published state.
//!
//! Moved out of the Session root under #611. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_talent_store(&mut self, store: Arc<TalentStore>) {
        self.catalogs.talent_store = Some(store);
    }
    pub fn set_num_talents_at_level_store(&mut self, store: Arc<NumTalentsAtLevelStore>) {
        self.catalogs.num_talents_at_level_store = Some(store);
        self.refresh_represented_talent_points_like_cpp();
    }

    pub(crate) fn reset_represented_talents_like_cpp(&mut self) {
        let _ = crate::session::hub_mut(self).mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.clear_talents_like_cpp();
        });
        // Login reconstructs a fresh C++ Player after this reset. Retaining the
        // previous character's runtime edges would affect GetCastSpellInfo, and
        // retaining per-spell traits could contaminate a coincident spell ID.
        let _ = self.clear_represented_trait_and_override_state_like_cpp();
        self.invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp();
    }
    pub(crate) fn reset_represented_active_talents_like_cpp(&mut self) -> bool {
        let Some(talent_group) =
            crate::session::hub_ref(self).represented_active_talent_group_like_cpp()
        else {
            return false;
        };
        let talent_group_index = usize::from(talent_group);
        if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
            return false;
        }

        let Some(active_talents) = crate::session::hub_mut(self)
            .mutate_player_talent_runtime_like_cpp(|runtime| {
                runtime.take_talent_group_like_cpp(talent_group)
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

        crate::session::hub_mut(self).record_represented_talent_reset_script_hook_like_cpp(true);
        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_TALENTS_LIKE_CPP, true);
        crate::session::hub_mut(self).remove_represented_pet_not_in_slot_like_cpp();

        if self.reset_represented_active_talents_like_cpp() {
            let Some(talent_data) =
                crate::session::hub_ref(self).resolved_update_talent_data_packet_like_cpp()
            else {
                return false;
            };
            self.send_packet(&talent_data);
            self.core
                .send_notification_like_cpp(self.reset_talents_notification_text_like_cpp());
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

        if !crate::session::hub_ref(self)
            .validate_represented_talent_learn_like_cpp(talent_id, rank)
        {
            return false;
        }

        let Some(runtime) = crate::session::hub_ref(self).player_talent_runtime_snapshot_like_cpp()
        else {
            return false;
        };
        let talent_group = runtime.active_group_like_cpp();
        let previous_rank = runtime
            .talent_group_like_cpp(talent_group)
            .and_then(|talents| talents.get(&talent_id).copied());
        let learned = crate::session::hub_mut(self).load_represented_talent_row_like_cpp(
            talent_tabs,
            talent_id,
            rank,
            talent_group,
        );
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
    #[cfg(test)]
    fn represented_calculate_talents_points_like_cpp(&self) -> Option<u32> {
        let base_points = self
            .catalogs
            .num_talents_at_level_store()
            .map(|store| {
                store.num_talents_at_level_like_cpp(
                    u32::from(crate::session::hub_ref(self).player_level_like_cpp()),
                    crate::session::hub_ref(self).player_class_like_cpp(),
                )
            })
            .unwrap_or(0);
        Some(base_points + self.represented_quest_rewarded_talent_points_like_cpp()?)
    }
    pub(crate) fn refresh_represented_talent_points_like_cpp(&mut self) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
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
            .catalogs
            .num_talents_at_level_store()
            .map(|store| {
                store.num_talents_at_level_like_cpp(
                    u32::from(crate::session::hub_ref(self).player_level_like_cpp()),
                    crate::session::hub_ref(self).player_class_like_cpp(),
                )
            })
            .unwrap_or(0);
        // Borrow immutable catalog policy; Player owns counting and field mutation.
        let _points = self.core.with_owned_player_mut_like_cpp(|player| {
            player.refresh_represented_talent_points_like_cpp(base_points, |talent_id, rank| {
                crate::session::hub_ref(self)
                    .represented_talent_info_like_cpp(talent_id, rank)
                    .is_some()
            })
        });
        #[cfg(test)]
        if let Some(points) = _points {
            self.fixtures.progression.player_character_points_like_cpp = points;
        }
    }
    fn reset_talents_notification_text_like_cpp(&self) -> String {
        let text = self.trinity_string_like_cpp(LANG_RESET_TALENTS_LIKE_CPP);
        if text == "<error>" {
            LANG_RESET_TALENTS_TEXT_LIKE_CPP.to_string()
        } else {
            text.to_string()
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/progression/talents/f3_shims.rs"]
mod f3_shims;
