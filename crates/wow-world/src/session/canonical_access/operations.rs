//! Accessors that resolve the canonical Player owner.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn with_owned_player_mut_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self.core.mutate_canonical_player_like_cpp(f);
        }
        self.core.with_owned_player_mut_like_cpp(f)
    }
    /// Resolve or construct the single canonical Player without transferring
    /// it between maps. This is the Rust equivalent of the live `Player*`
    /// passed through C++ `MapManager::CreateMap` while instance side effects
    /// are being applied.
    pub(in crate::session) fn ensure_canonical_player_owner_exists_like_cpp(
        &mut self,
        key: wow_map::MapKey,
    ) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };

        if self.core.player_handle_like_cpp.is_none() {
            let adopted = {
                let Ok(mut manager) = manager.lock() else {
                    return false;
                };
                manager.adopt_active_player_like_cpp(guid)
            };
            match adopted {
                Ok(handle) => self.core.player_handle_like_cpp = Some(handle),
                Err(wow_map::PlayerOwnerError::ActivePlayerMissing { .. }) => {
                    // Initial Player construction resolves map difficulty through
                    // MapManager, so it must run outside the manager lock.
                    let Some(player) = self.initial_player_box_like_cpp(key) else {
                        return false;
                    };
                    let Ok(mut manager) = manager.lock() else {
                        return false;
                    };
                    let handle = match manager.adopt_active_player_like_cpp(guid) {
                        Ok(handle) => handle,
                        Err(wow_map::PlayerOwnerError::ActivePlayerMissing { .. }) => {
                            let Ok(handle) = manager.install_detached_player_like_cpp(player)
                            else {
                                return false;
                            };
                            handle
                        }
                        Err(_) => return false,
                    };
                    self.core.player_handle_like_cpp = Some(handle);
                }
                Err(_) => return false,
            }
        }

        let Some(handle) = self.core.player_handle_like_cpp else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        let owner_ready = manager.player_residence_like_cpp(handle).is_some();
        drop(manager);
        #[cfg(test)]
        if owner_ready
            && !self
                .instances
                .represented_instance_reset_times_like_cpp
                .is_empty()
        {
            let rows =
                std::mem::take(&mut self.instances.represented_instance_reset_times_like_cpp);
            let _ = self.core.with_owned_player_mut_like_cpp(|player| {
                player.replace_instance_reset_times_like_cpp(rows);
            });
        }
        owner_ready
    }

    #[cfg(test)]
    pub(in crate::session) fn mutate_player_rest_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerRestState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            let mut state = crate::session::hub_ref(self).player_rest_state_snapshot_like_cpp()?;
            let result = f(&mut state);
            return self
                .replace_player_rest_state_like_cpp(state)
                .then_some(result);
        }
        self.core
            .with_owned_player_mut_like_cpp(|player| player.mutate_rest_state_like_cpp(f))
    }

    pub(in crate::session) fn set_player_rest_flag_like_cpp(
        &mut self,
        rest_flag: u32,
        trigger_id: u32,
    ) -> bool {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_rest_flag_like_cpp(rest_flag, trigger_id, || {
                wow_core::GameTime::now().as_secs()
            })
        });
        if let Some(changed) = canonical {
            return changed;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_rest_state_like_cpp(|state| {
                    state.set_flag_like_cpp(rest_flag, trigger_id, || {
                        wow_core::GameTime::now().as_secs()
                    })
                })
                .unwrap_or(false);
        }
        false
    }

    pub(in crate::session) fn remove_player_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.remove_rest_flag_like_cpp(rest_flag));
        if let Some(changed) = canonical {
            return changed;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_rest_state_like_cpp(|state| state.remove_flag_like_cpp(rest_flag))
                .unwrap_or(false);
        }
        false
    }

    pub(in crate::session) fn defer_player_rest_flag_sync_like_cpp(&mut self) -> bool {
        if self
            .core
            .with_owned_player_mut_like_cpp(|player| player.defer_rest_flag_sync_like_cpp())
            .is_some()
        {
            return true;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_rest_state_like_cpp(|state| state.defer_flag_sync_like_cpp())
                .is_some();
        }
        false
    }

    pub(in crate::session) fn end_player_rest_flag_sync_like_cpp(&mut self) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.end_deferred_rest_flag_sync_like_cpp());
        if let Some(dirty) = canonical {
            return dirty;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_rest_state_like_cpp(|state| state.end_deferred_flag_sync_like_cpp())
                .unwrap_or(false);
        }
        false
    }

    pub(in crate::session) fn clear_player_deferred_rest_flag_update_like_cpp(&mut self) -> bool {
        if self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.clear_deferred_rest_flag_update_like_cpp()
            })
            .is_some()
        {
            return true;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_rest_state_like_cpp(|state| {
                    state.clear_deferred_flag_update_like_cpp()
                })
                .is_some();
        }
        false
    }

    pub(in crate::session) fn take_player_deferred_rest_flag_update_dirty_like_cpp(
        &mut self,
    ) -> bool {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.take_deferred_rest_flag_update_dirty_like_cpp()
        });
        if let Some(dirty) = canonical {
            return dirty;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_rest_state_like_cpp(|state| {
                    state.take_deferred_flag_update_like_cpp()
                })
                .unwrap_or(false);
        }
        false
    }

    /// Transitional login seam: consume the already loaded Session values
    /// once, install the Player under MapManager, then let every later load
    /// step mutate that generation-checked canonical value. The retained
    /// Session fields are retired family-by-family in this issue.
    pub(in crate::session) fn install_detached_canonical_player_from_session_like_cpp(
        &mut self,
        bootstrap_position: Position,
    ) -> bool {
        if self.core.player_handle_like_cpp.is_some() {
            return true;
        }
        let Some(guid) = self.core.player_guid else {
            return false;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let key = wow_map::MapKey::new(u32::from(self.core.current_map_id), 0);
        let Some(player) = self
            .build_initial_player_for_owner_like_cpp(key, Some(bootstrap_position))
            .map(Box::new)
        else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let handle = match manager.adopt_active_player_like_cpp(guid) {
            Ok(handle) => handle,
            Err(wow_map::PlayerOwnerError::ActivePlayerMissing { .. }) => {
                let Ok(handle) = manager.install_detached_player_like_cpp(player) else {
                    return false;
                };
                handle
            }
            Err(_) => return false,
        };
        self.core.player_handle_like_cpp = Some(handle);
        self.core.player_identity_bootstrap_like_cpp = None;
        true
    }
    pub(crate) fn owned_player_cuf_profiles_like_cpp(
        &self,
    ) -> Option<(Vec<Option<wow_entities::PlayerCufProfile>>, bool)> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            (state.cuf_profiles.clone(), state.cuf_profiles_loaded)
        });
        if canonical.is_some() {
            return canonical;
        }

        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return Some((
                self.fixtures
                    .presentation
                    .cuf_profiles_like_cpp
                    .iter()
                    .map(|profile| profile.clone().map(player_cuf_profile_from_packet_like_cpp))
                    .collect(),
                self.fixtures.presentation.cuf_profiles_loaded_like_cpp,
            ));
        }
        None
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/canonical_access/operations/f3_shims.rs"]
mod f3_shims;
