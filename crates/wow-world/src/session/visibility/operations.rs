//! Represented visibility and phase state at the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

fn apply_player_session_visibility_detection_like_cpp(
    player: &mut wow_entities::Player,
    never_visible_for_seer: bool,
    seer_can_never_see_target: bool,
) {
    player
        .unit_mut()
        .set_never_visible_for_seer_like_cpp(never_visible_for_seer);
    player
        .unit_mut()
        .set_seer_can_never_see_target_like_cpp(seer_can_never_see_target);
}

impl WorldSession {
    fn player_session_never_visible_for_seer_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.lifecycle.player_logout_like_cpp() || self.lifecycle.player_loading() == Some(guid)
    }
    pub(crate) fn sync_current_player_session_visibility_detection_like_cpp(&mut self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let never_visible_for_seer = self.player_session_never_visible_for_seer_like_cpp(guid);
        let seer_can_never_see_target =
            crate::session::hub_ref(self).player_can_never_see_target_like_cpp();
        let _ = self
            .core
            .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                apply_player_session_visibility_detection_like_cpp(
                    player,
                    never_visible_for_seer,
                    seer_can_never_see_target,
                );
            });
    }

    pub(crate) fn visible_other_players_from_registry_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Vec<(ObjectGuid, PlayerVisibilityCreateSnapshot)> {
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };
        let Some(registry) = &self.core.player_registry else {
            return Vec::new();
        };
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let source_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();

        registry
            .player_visibility_create_candidates(
                player_guid,
                map_id,
                instance_id,
                *position,
                source_combat_reach,
                visibility_radius,
            )
            .into_iter()
            .filter(|candidate| {
                self.canonical_player_phase_visible_like_cpp(map_id, instance_id, candidate.guid)
                    == Some(true)
            })
            .map(|candidate| (candidate.guid, candidate))
            .collect()
    }
    /// Require the target to exist in the canonical map and apply the same
    /// phase gate as C++ `VisibleNotifier::Visit(PlayerMapType&)`. Missing map
    /// state returns `None` and fails closed at the caller: C++ cannot visit a
    /// registry-only player.
    fn canonical_player_phase_visible_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        target_guid: ObjectGuid,
    ) -> Option<bool> {
        // Resolve the viewer before taking the map lock; the handle resolver
        // takes the same manager and must not recurse into that mutex.
        let player_phase_shift = self.represented_player_phase_shift_like_cpp()?;
        let manager = self.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(u32::from(map_id), instance_id)?;
        let target = map.map().get_typed_player(target_guid)?;
        Some(player_phase_shift.can_see(target.unit().world().phase_shift()))
    }
    pub fn set_phase_store(&mut self, store: Arc<PhaseStore>) {
        self.catalogs.phase_store = Some(store);
    }
    pub(crate) fn represented_player_phase_shift_like_cpp(&self) -> Option<PhaseShift> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().phase_shift().clone());
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(
                self.visibility
                    .represented_player_phase_shift_fixture_like_cpp(),
            );
        }
        canonical
    }
    pub(crate) fn can_see_phase_shift_like_cpp(&self, other: &PhaseShift) -> bool {
        self.represented_player_phase_shift_like_cpp()
            .is_some_and(|phase_shift| phase_shift.can_see(other))
    }
    pub(crate) fn resolved_visible_resting_like_cpp(&self) -> Option<bool> {
        // RestMgr::SetRestFlag/RemoveRestFlag (RestMgr.cpp:99-125): the
        // mask and Player flag belong to the same Player. Read them together.
        let canonical = self.core.with_owned_player_for_rest_like_cpp(|player| {
            let rest = player.rest_state_like_cpp();
            if rest.is_location_initialized_like_cpp() {
                rest.is_resting_by_flag_like_cpp()
            } else {
                (player.data().player_flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0
            }
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let rest = crate::session::hub_ref(self).player_rest_state_snapshot_like_cpp()?;
            if rest.is_location_initialized_like_cpp() {
                return Some(rest.is_resting_by_flag_like_cpp());
            }
            return Some(
                self.lifecycle
                    .represented_loaded_player_flags_for_test_like_cpp()
                    .map(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0)
                    .unwrap_or(rest.is_resting_by_flag_like_cpp()),
            );
        }
        canonical
    }
    /// Resolve the current C++ `Player::m_seer` projection from the canonical
    /// Player. C++ keeps a pointer, but every Rust consumer in this boundary
    /// needs only its GUID; `ActivePlayerData::FarsightObject` is the durable
    /// owner and an empty value means the Player itself.
    pub(in crate::session) fn current_seer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let player_guid = self.player_guid()?;
        if let Some(farsight) = self
            .core
            .current_canonical_player_farsight_object_value_like_cpp()
        {
            if !farsight.is_empty() {
                return Some(farsight);
            }
            #[cfg(test)]
            if let Some(seer_guid) = self.visibility.represented_seer_guid_fixture_like_cpp() {
                // Detached fixtures can model the short C++ ordering window
                // between writing FarsightObject and SetSeer(this).
                if !seer_guid.is_empty() && seer_guid != player_guid {
                    return Some(seer_guid);
                }
            }
            return Some(player_guid);
        }

        #[cfg(test)]
        return self.visibility.represented_seer_guid_fixture_like_cpp();

        #[cfg(not(test))]
        None
    }

    pub(crate) fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_visibility_mut(self);
        state.sync_represented_farsight_clear_from_canonical_like_cpp(&mut hub)
    }
    pub(crate) async fn force_update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        self.visibility.clear_last_visibility_pos_like_cpp();
        wow_world_entities::update_visibility_with_catalogs_like_cpp(self, creature_spawn_catalogs)
            .await;
    }
    pub(crate) async fn flush_pending_visibility_refresh_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self
            .core
            .flags
            .visibility_refresh_pending_like_cpp
            .swap(false, Ordering::AcqRel)
        {
            self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;
        }
    }
    #[cfg(test)]
    pub(crate) async fn force_update_visibility_like_cpp(&mut self) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.force_update_visibility_with_catalogs_like_cpp(&catalogs)
            .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/visibility/operations/f3_shims.rs"]
mod f3_shims;
