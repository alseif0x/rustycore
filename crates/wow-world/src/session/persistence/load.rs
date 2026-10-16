//! Represented load and hydration at the Session boundary.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) async fn load_instance_time_restrictions_like_cpp(&mut self) {
        crate::session::cx_lifecycle(self)
            .load_instance_time_restrictions_like_cpp()
            .await
    }
    pub(crate) fn load_character_reputation_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = wow_progression::mgr::CharacterReputationRowLikeCpp>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_character_reputation_rows_like_cpp(&mut hub, rows)
    }
    pub(crate) fn load_represented_xp_rest_bonus_like_cpp(
        &mut self,
        rest_state: u8,
        rest_bonus: f32,
    ) {
        // C++ `RestMgr::LoadRestBonus` restores both DB values verbatim. It does
        // not clamp or recompute the state until a later `AddRestBonus` reaches
        // `SetRestBonus` (for example when offline time is applied).
        // C++-created rows only contain the declared PlayerRestState values.
        // Normalize legacy Rust rows that persisted the old invalid value 0,
        // while preserving every valid DB state verbatim like LoadRestBonus.
        let rest_state = if wow_entities::valid_player_rest_state_like_cpp(rest_state) {
            rest_state
        } else {
            REST_STATE_NORMAL_LIKE_CPP
        };
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.clear_represented_rest_flags_for_character_load_like_cpp();
            let _ = self.mutate_player_rest_state_like_cpp(|state| {
                state.install_loaded_rest_like_cpp(rest_state, rest_bonus);
            });
            return;
        }
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            player.load_xp_rest_bonus_like_cpp(rest_state, rest_bonus);
        });
    }
    #[cfg(test)]
    fn clear_represented_rest_flags_for_character_load_like_cpp(&mut self) {
        let loaded_resting = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
            .is_some_and(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0);
        let _canonical =
            wow_world_instances::with_owned_player_mut_for_rest_like_cpp(self, |player| {
                let mut state = player.rest_state_like_cpp().clone();
                state.reset_location_tracking_like_cpp();
                player.replace_rest_state_like_cpp(state);
                if loaded_resting {
                    player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
                }
            });
        #[cfg(test)]
        if _canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_flag_mask_like_cpp = 0;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_location_initialized_like_cpp = false;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_defer_rest_flag_sync_like_cpp = false;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_deferred_rest_flag_update_dirty_like_cpp = false;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_inn_area_trigger_id_like_cpp = 0;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_time_secs_like_cpp = 0;
        }
    }
    pub(crate) fn mark_represented_talents_loaded_like_cpp(&mut self) {
        let _ = crate::session::hub_mut(self).mark_talents_loaded_like_cpp();
        wow_world_quest::refresh_represented_talent_points_like_cpp(self);
    }
    /// Set the player loading GUID (ConnectTo flow).
    pub fn set_player_loading(&mut self, guid: Option<ObjectGuid>) {
        self.lifecycle.set_player_loading(guid);
        wow_world_entities::sync_current_player_session_visibility_detection_like_cpp(self);
    }
    pub(crate) fn player_loading(&self) -> Option<ObjectGuid> {
        self.lifecycle.player_loading()
    }
    pub async fn load_tutorials_data_like_cpp(&mut self) {
        self.lifecycle.begin_tutorials_load_like_cpp();

        let Some(port) = self
            .lifecycle
            .session_account_state_port_like_cpp()
            .map(std::sync::Arc::clone)
        else {
            warn!(
                account = self.core.account_id,
                "LoadTutorialsData skipped: session account-state port unavailable"
            );
            return;
        };

        match port.load_tutorials_like_cpp(self.core.account_id).await {
            wow_persistence::SessionTutorialsLoadOutcomeLikeCpp::Loaded(values) => {
                self.lifecycle.load_tutorials_data_values_like_cpp(values);
            }
            wow_persistence::SessionTutorialsLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.core.account_id,
                    "LoadTutorialsData query failed: {reason}"
                );
            }
        }
    }
    pub async fn load_global_account_data_like_cpp(&mut self) {
        self.load_account_data_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP)
            .await;
    }
    async fn load_account_data_like_cpp(&mut self, guid: ObjectGuid, mask: u32) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_account_data_like_cpp(&mut hub, guid, mask).await
    }
    pub(crate) fn set_loaded_player_name_like_cpp(&mut self, name: String) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_name_like_cpp(&mut hub, name)
    }
    pub(crate) fn set_loaded_player_identity_like_cpp(
        &mut self,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        let initialize_reputation = crate::session::hub_ref(self).player_race_like_cpp() != race
            || crate::session::hub_ref(self).player_class_like_cpp() != class
            || self
                .core
                .with_owned_player_like_cpp(|player| {
                    player.reputation_like_cpp().faction_count_like_cpp() == 0
                })
                .unwrap_or(true);
        if self.core.player_map_id_like_cpp() != map_id
            || crate::session::hub_ref(self).player_race_like_cpp() != race
            || crate::session::hub_ref(self).player_class_like_cpp() != class
            || crate::session::hub_ref(self).player_gender_like_cpp() != gender
        {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.core.current_map_id = map_id;
        let gray_level = crate::session::hub_ref(self).gray_level(level);
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_race_class_gender(race, class, crate::session::gender_from_u8(gender));
                player.set_level_and_gray_level_like_cpp(level, gray_level);
            })
            .is_some();
        if canonical {
            self.core.player_identity_bootstrap_like_cpp = None;
        } else {
            #[cfg(not(test))]
            if self.core.player_handle_like_cpp.is_some() {
                return;
            }
            self.core.player_identity_bootstrap_like_cpp =
                Some(super::PlayerIdentityBootstrapLikeCpp {
                    name: crate::session::hub_ref(self).player_name_like_cpp(),
                    race,
                    class,
                    level,
                    gender,
                });
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.fixtures.identity.player_race = race;
                self.fixtures.identity.player_class = class;
                self.fixtures.identity.player_level = level;
                self.fixtures.identity.player_gender = gender;
            }
        }
        crate::session::hub_mut(self).set_player_faction_for_race_like_cpp(race);
        if initialize_reputation {
            crate::session::hub_mut(self).initialize_reputation_mgr_like_cpp();
        }
        wow_world_quest::refresh_represented_talent_points_like_cpp(self);
    }
}

impl crate::session::LifecycleCx<'_> {
    #[cfg(test)]
    pub(in crate::session) fn load_instance_time_restriction_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        self.instances
            .replace_represented_instance_reset_times_for_test_like_cpp(rows);
    }

    pub(crate) async fn load_instance_time_restrictions_like_cpp(&mut self) {
        let _ = self
            .instances
            .replace_instance_reset_times_like_cpp(&mut self.hub, []);

        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            warn!(
                account = self.hub.core.account_id,
                "LoadInstanceTimeRestrictions skipped: Player lifecycle port unavailable"
            );
            return;
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions {
                    account_id: self.hub.core.account_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.hub.core.account_id,
                    "LoadInstanceTimeRestrictions query failed: {reason}"
                );
                return;
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.hub.core.account_id,
                    "Player lifecycle port returned the wrong auxiliary login data for instance time restrictions"
                );
                return;
            }
        };

        let _ = self.instances.replace_instance_reset_times_like_cpp(
            &mut self.hub,
            rows.into_iter()
                .map(|row| (row.instance_id, row.release_time)),
        );
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/persistence/load/f3_shims.rs"]
mod f3_shims;
