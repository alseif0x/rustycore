//! Accessors that resolve the canonical Player owner.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn mutate_canonical_player_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        let guid = self.player_guid()?;
        self.mutate_canonical_player_by_guid_like_cpp(guid, f)
    }
    /// Resolve this session incarnation's canonical `Player` exclusively
    /// through its generation-checked handle.
    ///
    /// Unlike the transitional GUID/map lookup helpers, this deliberately has
    /// no fallback: a stale or missing handle means that the owner is unknown.
    pub(in crate::session) fn with_owned_player_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let handle = self.player_handle_like_cpp?;
        let manager = manager.lock().ok()?;
        let result = manager.with_player_like_cpp(handle, f);
        drop(manager);
        result
    }
    /// Mutating counterpart to `with_owned_player_like_cpp`.
    pub(in crate::session) fn with_owned_player_mut_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let handle = self.player_handle_like_cpp?;
        let mut manager = manager.lock().ok()?;
        let result = manager.with_player_mut_like_cpp(handle, f);
        drop(manager);
        result
    }
    pub(in crate::session) fn with_owned_player_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.canonical_player_snapshot_like_cpp(f);
        }
        self.with_owned_player_like_cpp(f)
    }
    pub(in crate::session) fn with_owned_player_mut_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.mutate_canonical_player_like_cpp(f);
        }
        self.with_owned_player_mut_like_cpp(f)
    }
    pub(crate) fn mutate_canonical_player_by_guid_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        if let Some(handle) = self.player_handle_like_cpp
            && handle.guid() == guid
        {
            return manager.with_player_mut_like_cpp(handle, f);
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let managed = manager.find_map_mut(map_id, instance_id.unwrap_or(0))?;
        let player = managed.map_mut().get_typed_player_mut(guid)?;
        Some(f(player))
    }
    pub(in crate::session) fn canonical_player_has_player_flag_like_cpp(
        &self,
        guid: ObjectGuid,
        flag: u32,
    ) -> Option<bool> {
        if self.player_guid() == Some(guid) {
            let owned = self.with_owned_player_like_cpp(|player| player.has_player_flag(flag));
            if owned.is_some() {
                return owned;
            }
            #[cfg(not(test))]
            return None;
            #[cfg(test)]
            if self.player_handle_like_cpp.is_some() {
                return None;
            }
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed
                    .map()
                    .get_typed_player(guid)
                    .map(|player| player.has_player_flag(flag));
            }
        });
        result
    }
    pub(in crate::session) fn canonical_player_display_ids_like_cpp(&self) -> Option<(u32, u32)> {
        let guid = self.player_guid()?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed.map().get_typed_player(guid).map(|player| {
                    let data = player.unit().data();
                    (
                        u32::try_from(data.display_id).unwrap_or_default(),
                        u32::try_from(data.native_display_id).unwrap_or_default(),
                    )
                });
            }
        });
        result
    }
    pub(in crate::session) fn mutate_player_world_local_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerWorldLocalState) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            mutate.take().expect("world-local mutation runs once")(
                &mut player.gameplay_state_mut().world_local,
            )
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let mut state = self
                .player_world_local_state_like_cpp()
                .expect("handle-less fixture world-local state");
            let result = mutate.take().expect("world-local mutation runs once")(&mut state);
            self.player_zone_id_like_cpp = state.zone_id;
            self.player_area_id_like_cpp = state.area_id;
            self.player_zone_area_authority_complete_like_cpp = state.zone_area_authority_complete;
            self.player_pvp_hostile_like_cpp = state.pvp_hostile;
            self.player_pvp_end_timer_like_cpp = state.pvp_end_timer;
            self.player_contested_pvp_timer_like_cpp = state.contested_pvp_timer;
            self.represented_is_outdoors_like_cpp = state.is_outdoors;
            return Some(result);
        }
        canonical
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
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };

        if self.player_handle_like_cpp.is_none() {
            let adopted = {
                let Ok(mut manager) = manager.lock() else {
                    return false;
                };
                manager.adopt_active_player_like_cpp(guid)
            };
            match adopted {
                Ok(handle) => self.player_handle_like_cpp = Some(handle),
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
                    self.player_handle_like_cpp = Some(handle);
                }
                Err(_) => return false,
            }
        }

        let Some(handle) = self.player_handle_like_cpp else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager.player_residence_like_cpp(handle).is_some()
    }
    pub(in crate::session) fn mutate_player_collection_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerCollectionStateLikeCpp) -> R,
    ) -> Option<R> {
        let mut state = self.player_collection_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        self.replace_player_collection_state_like_cpp(state)
            .then_some(result)
    }
    pub(in crate::session) fn mutate_player_unit_presentation_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.with_owned_player_mut_like_cpp(mutate)
    }
    /// Read this session's own canonical `Player`.
    ///
    /// Resolving the GUID and map key is the only session-local part; the read
    /// itself is the placement-addressed accessor a remote reader uses too
    /// (#252), so one player's canonical state cannot be reached two ways.
    pub(in crate::session) fn canonical_player_snapshot_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        let guid = self.player_guid()?;
        if let (Some(manager), Some(handle)) = (
            self.canonical_map_manager.as_ref(),
            self.player_handle_like_cpp,
        ) && handle.guid() == guid
        {
            return manager.lock().ok()?.with_player_like_cpp(handle, f);
        }
        let key = self.current_canonical_player_map_key_like_cpp();
        let manager = self.canonical_map_manager.as_ref()?;
        let map_id = key
            .as_ref()
            .map(|key| key.map_id)
            .unwrap_or_else(|| u32::from(self.player_map_id_like_cpp()));
        let instance_id = key.as_ref().map(|key| key.instance_id).unwrap_or(0);
        crate::canonical_player_access::with_canonical_player_at_like_cpp(
            manager,
            guid,
            map_id,
            instance_id,
            f,
        )
    }
    pub(crate) fn canonical_player_parry_block_snapshot_like_cpp(&self) -> (bool, bool) {
        self.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().can_parry_like_cpp(),
                player.unit().can_block_like_cpp(),
            )
        })
        .unwrap_or((false, false))
    }
    /// Apply one named canonical cinematic transition, or the handle-less test
    /// mirror that stands in for it. C++ performs these on the Player's own
    /// `CinematicMgr` (`CinematicMgr.h:39`, `CinematicMgr.cpp:46`, `:83`), never
    /// on a borrowed record.
    pub(in crate::session) fn with_player_cinematic_state_like_cpp<R>(
        &mut self,
        mut apply: impl FnMut(&mut wow_entities::PlayerCinematicStateLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            apply(&mut player.gameplay_state_mut().cinematic)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let result = apply(&mut self.represented_cinematic_state_like_cpp);
            return Some(result);
        }
        None
    }
    pub(in crate::session) fn mutate_player_rest_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerRestState) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut state = self.player_rest_state_snapshot_like_cpp()?;
            let result = f(&mut state);
            return self
                .replace_player_rest_state_like_cpp(state)
                .then_some(result);
        }
        self.with_owned_player_mut_like_cpp(|player| player.mutate_rest_state_like_cpp(f))
    }
    /// Transitional login seam: consume the already loaded Session values
    /// once, install the Player under MapManager, then let every later load
    /// step mutate that generation-checked canonical value. The retained
    /// Session fields are retired family-by-family in this issue.
    pub(in crate::session) fn install_detached_canonical_player_from_session_like_cpp(
        &mut self,
        bootstrap_position: Position,
    ) -> bool {
        if self.player_handle_like_cpp.is_some() {
            return true;
        }
        let Some(guid) = self.player_guid else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let key = wow_map::MapKey::new(u32::from(self.current_map_id), 0);
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
        self.player_handle_like_cpp = Some(handle);
        true
    }
    /// Apply a heal to the canonical Player owner and return
    /// `(before, after, max, effective)`.
    pub(in crate::session) fn apply_owned_player_heal_like_cpp(
        &mut self,
        requested_heal: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let before = player.unit().data().health.min(u64::from(max_health)) as u32;
            if !player.unit().is_alive() || before == 0 {
                return (before, before, max_health, 0);
            }
            let after = before.saturating_add(requested_heal).min(max_health);
            player.unit_mut().set_health(u64::from(after));
            (before, after, max_health, after.saturating_sub(before))
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let max_health = self.player_max_health_like_cpp.max(1);
            let before = self.player_health_like_cpp.min(max_health);
            if !self.player_alive_like_cpp || before == 0 {
                return Some((before, before, max_health, 0));
            }
            let after = before.saturating_add(requested_heal).min(max_health);
            self.player_health_like_cpp = after;
            return Some((before, after, max_health, after.saturating_sub(before)));
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_battleground_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerBattlegroundState) -> R,
    ) -> Option<R> {
        let mut state = self.player_battleground_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.install_battleground_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_battleground_type_id_like_cpp = state.represented_type_id;
            self.player_battleground_map_id_like_cpp = state.represented_map_id;
            self.represented_battleground_status_like_cpp = state.represented_status;
            self.represented_battleground_queue_slots_like_cpp = state.represented_queue_slots;
            self.represented_arena_team_id_invited_like_cpp = state.arena_team_id_invited;
            return Some(result);
        }
        canonical.then_some(result)
    }
    pub(crate) fn owned_player_cuf_profiles_like_cpp(
        &self,
    ) -> Option<(Vec<Option<wow_entities::PlayerCufProfile>>, bool)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let state = player.gameplay_state();
            (state.cuf_profiles.clone(), state.cuf_profiles_loaded)
        });
        if canonical.is_some() {
            return canonical;
        }

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some((
                self.cuf_profiles_like_cpp
                    .iter()
                    .map(|profile| profile.clone().map(player_cuf_profile_from_packet_like_cpp))
                    .collect(),
                self.cuf_profiles_loaded_like_cpp,
            ));
        }
        None
    }
}
