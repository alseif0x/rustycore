// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player viewpoint cleanup before physical removal.

use super::*;
use crate::map_rules::player_set_viewpoint_outcome_like_cpp;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Bounded map-owned cleanup for the late C++ `Player::RemoveFromWorld()`
    /// `GetViewpoint()` -> `SetViewpoint(viewpoint, false)` branch.
    ///
    /// Source-of-truth anchors:
    /// - `Player.cpp:1567-1585` runs this after `Unit::RemoveFromWorld()` and
    ///   item cleanup while the Player still exists.
    /// - `Player.cpp:25344-25387` clears `FarsightObject`, removes Unit shared
    ///   vision for Unit targets, requests `SetSeer(this)`, and does not request
    ///   `UpdateVisibilityOf` on remove.
    /// - `Player.cpp:25389-25395` resolves `GetViewpoint()` from
    ///   `FarsightObject` through `TYPEMASK_SEER`.
    ///
    /// Ownership: only canonical same-map `Map::entity_world` typed records are
    /// consulted/mutated. DynamicObject targets clear only the removing Player's
    /// `FarsightObject` when it still equals the target GUID; this branch never
    /// resolves `DynamicObject::bound_caster()` or toggles DynamicObject caster
    /// viewpoint state because that lifecycle belongs to DynamicObject removal.
    /// There is no ObjectAccessor/session fallback, no packet fanout, and no real
    /// SetSeer implementation in this seam. Vehicle-base skipping stays open
    /// because this map-owned cleanup has no Player vehicle base runtime; the Unit
    /// helper is called with `vehicle_base_guid: None`.
    pub(super) fn cleanup_player_remove_from_world_viewpoint_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> Option<PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp> {
        let player_record = self.map_object_record(player_guid)?;
        if player_record.kind() != AccessorObjectKind::Player
            || !player_record.object().object().is_in_world()
        {
            return None;
        }

        let viewpoint_guid = player_record
            .player()
            .map(|player| player.active_data().farsight_object)?;
        if viewpoint_guid.is_empty() {
            return None;
        }

        let outcome = |status,
                       player_set_viewpoint: Option<PlayerSetViewpointOutcomeLikeCpp>,
                       dynamic_object_caster_viewpoint: Option<
            DynamicObjectCasterViewpointOutcomeLikeCpp,
        >,
                       update_visibility_requested,
                       set_seer_requested| {
            PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp {
                player_guid,
                viewpoint_guid,
                status,
                player_set_viewpoint,
                dynamic_object_caster_viewpoint,
                update_visibility_requested,
                set_seer_requested,
                object_accessor_fanout_represented: false,
            }
        };

        let Some(target_record) = self.map_object_record(viewpoint_guid) else {
            return Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::MissingTarget,
                None,
                None,
                false,
                false,
            ));
        };
        let target_kind = target_record.kind();
        if !target_record.object().object().is_in_world() {
            return Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotInWorld,
                None,
                None,
                false,
                false,
            ));
        }

        match target_kind {
            AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
                let player_set_viewpoint = self.apply_player_set_viewpoint_unit_like_cpp(
                    player_guid,
                    viewpoint_guid,
                    false,
                    None,
                );
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedUnitViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            AccessorObjectKind::DynamicObject => {
                let player_set_viewpoint = match self.get_typed_player_mut(player_guid) {
                    Some(player) if player.active_data().farsight_object == viewpoint_guid => {
                        player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                        player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    Some(_) => player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                    None => player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                        None,
                        false,
                        false,
                    ),
                };
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            AccessorObjectKind::Player => {
                let player_set_viewpoint = match self.get_typed_player_mut(player_guid) {
                    Some(player) if player.active_data().farsight_object == viewpoint_guid => {
                        player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                        player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    _ => player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                };
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedPlayerViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            _ => Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotSeer,
                None,
                None,
                false,
                false,
            )),
        }
    }
}
