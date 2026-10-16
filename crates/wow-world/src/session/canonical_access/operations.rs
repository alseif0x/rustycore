//! Accessors that resolve the canonical Player owner.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
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
        if owner_ready {
            if let Some(rows) = self
                .instances
                .take_nonempty_represented_instance_reset_times_for_test_like_cpp()
            {
                let _ = self.core.with_owned_player_mut_like_cpp(|player| {
                    player.replace_instance_reset_times_like_cpp(rows);
                });
            }
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/canonical_access/operations/f3_shims.rs"]
mod f3_shims;
