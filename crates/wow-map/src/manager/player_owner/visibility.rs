//! Map-selected deferred player visibility publication.
//!
//! C++ `Map.cpp:830-905` selects the expired active-grid notifiers and resets
//! their flags after `DelayedUnitRelocation::Visit(PlayerMapType&)`
//! (`GridNotifiers.cpp:217-236`). Preserve that selection as an owned delivery
//! obligation; the Session adapter owns its actual client ledger and packets.

use super::{MapKey, MapManager, ObjectGuid, PlayerHandle, PlayerResidenceLikeCpp};

/// Permission to refresh one map-selected player's represented visibility.
///
/// This carries no client GUID set or precomputed CREATE/out-of-range decision.
/// Construction belongs exclusively to the canonical map owner. A residence
/// revision prevents an away-and-back transfer from reviving an old request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerVisibilityRefreshIntentLikeCpp {
    handle: PlayerHandle,
    map_key: MapKey,
    residence_revision: u64,
    viewpoint_guid: ObjectGuid,
}

impl PlayerVisibilityRefreshIntentLikeCpp {
    pub const fn handle(self) -> PlayerHandle {
        self.handle
    }

    pub const fn map_key(self) -> MapKey {
        self.map_key
    }

    pub const fn residence_revision(self) -> u64 {
        self.residence_revision
    }

    pub const fn viewpoint_guid(self) -> ObjectGuid {
        self.viewpoint_guid
    }
}

impl MapManager {
    /// Drain each coalesced map selection once. The caller must transfer the
    /// resulting obligations to its delivery rail after releasing this guard.
    pub fn take_player_visibility_refresh_intents_like_cpp(
        &mut self,
    ) -> Vec<PlayerVisibilityRefreshIntentLikeCpp> {
        let mut intents: Vec<_> = self
            .player_owners_like_cpp
            .values_mut()
            .filter_map(|owner| owner.pending_visibility_refresh.take())
            .collect();
        intents.retain(|intent| self.player_visibility_refresh_intent_is_current_like_cpp(*intent));
        intents.sort_by_key(|intent| intent.handle.guid());
        intents
    }

    /// Check incarnation, active residence and viewpoint under the caller's
    /// canonical manager guard. This observation is not a lease across await.
    pub fn player_visibility_refresh_intent_is_current_like_cpp(
        &self,
        intent: PlayerVisibilityRefreshIntentLikeCpp,
    ) -> bool {
        let Some(owner) = self.current_player_owner_like_cpp(intent.handle) else {
            return false;
        };
        if owner.residence_revision != intent.residence_revision
            || self.checked_player_residence_like_cpp(intent.handle)
                != Ok(PlayerResidenceLikeCpp::Active(intent.map_key))
        {
            return false;
        }
        let Some(map) = self.find_map(intent.map_key.map_id, intent.map_key.instance_id) else {
            return false;
        };
        if self.current_player_visibility_viewpoint_like_cpp(intent.handle.guid(), intent.map_key)
            != Some(intent.viewpoint_guid)
        {
            return false;
        }
        map.map()
            .map_object(intent.viewpoint_guid)
            .is_some_and(|viewpoint| {
                let position = viewpoint.position();
                viewpoint.object().is_in_world()
                    && crate::coords::is_valid_map_coord_4d(
                        position.x,
                        position.y,
                        position.z,
                        position.orientation,
                    )
            })
    }

    fn current_player_visibility_viewpoint_like_cpp(
        &self,
        guid: ObjectGuid,
        key: MapKey,
    ) -> Option<ObjectGuid> {
        let player = self
            .find_map(key.map_id, key.instance_id)?
            .map()
            .get_typed_player(guid)?;
        // Same canonical viewpoint projection as Map's relocation selector.
        let farsight = player.active_data().farsight_object;
        Some(if farsight.is_empty() { guid } else { farsight })
    }

    pub(in crate::manager) fn retain_selected_player_visibility_refreshes_like_cpp(
        &mut self,
        updated_maps: impl IntoIterator<Item = MapKey>,
    ) {
        for key in updated_maps {
            let Some(map) = self.find_map(key.map_id, key.instance_id) else {
                continue;
            };
            // Do not copy the plan's candidate GUID sets: they are neither
            // detection-filtered nor based on the Session's real client ledger.
            let selected: Vec<_> = map
                .last_process_relocation_notifies_outcome_like_cpp
                .visibility_plans
                .player_plans
                .iter()
                .flat_map(|plan| {
                    // C++ PlayerRelocationNotifier also updates nearby players
                    // which are not themselves awaiting a notify (GridNotifiers
                    // .cpp:137-152). Recompute their current ledger through the
                    // same fenced rail, without setting another notify flag.
                    // The plan's empty previous-client set still cannot select
                    // reciprocal updates for former out-of-range observers.
                    std::iter::once((plan.player_guid, Some(plan.viewpoint_guid))).chain(
                        plan.visibility_plan
                            .reciprocal_player_updates
                            .iter()
                            .copied()
                            .map(|guid| (guid, None)),
                    )
                })
                .collect();
            for (guid, selected_viewpoint) in selected {
                let Some(owner) = self.player_owners_like_cpp.get(&guid) else {
                    continue;
                };
                let Some(viewpoint_guid) = selected_viewpoint
                    .or_else(|| self.current_player_visibility_viewpoint_like_cpp(guid, key))
                else {
                    continue;
                };
                let intent = PlayerVisibilityRefreshIntentLikeCpp {
                    handle: PlayerHandle {
                        guid,
                        generation: owner.generation,
                    },
                    map_key: key,
                    residence_revision: owner.residence_revision,
                    viewpoint_guid,
                };
                if !self.player_visibility_refresh_intent_is_current_like_cpp(intent) {
                    continue;
                }
                self.player_owners_like_cpp
                    .get_mut(&guid)
                    .expect("validated canonical owner remains under this manager guard")
                    .pending_visibility_refresh = Some(intent);
            }
        }
    }
}

#[cfg(test)]
mod tests;
