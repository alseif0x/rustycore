//! Gameobject lookup, catalogs and nearby search for the represented Session.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_gameobject_display_info_store(&mut self, store: Arc<GameObjectDisplayInfoStore>) {
        self.catalogs.gameobjects.display_info_store = Some(store);
    }
    pub fn set_gameobject_template_lifecycle_store(
        &mut self,
        store: Arc<GameObjectTemplateLifecycleStoreLikeCpp>,
    ) {
        self.catalogs.gameobject_template_lifecycle_store_like_cpp = Some(store);
    }
    pub(in crate::session) fn visible_gameobject_guids_from_last_update_summary_like_cpp<F>(
        &self,
        select_guids: F,
    ) -> Option<(u32, u32, u64, Vec<ObjectGuid>)>
    where
        F: FnOnce(&wow_map::map::GameObjectsUpdateSummaryLikeCpp) -> Vec<ObjectGuid>,
    {
        let key = self.core.current_canonical_player_map_key_like_cpp()?;
        let manager = self.core.canonical_map_manager.as_ref()?;
        let player_guid = self.player_guid()?;
        let represented_seer_guid = self.current_seer_guid_like_cpp();
        let direct_target_seer_gate_allows_send = represented_seer_guid.is_none_or(|seer_guid| {
            seer_guid.is_empty()
                || seer_guid == player_guid
                || crate::session::hub_ref(self).represented_player_has_active_vehicle_like_cpp()
        });

        let manager = manager.lock().ok()?;
        let managed_map = manager.find_map(key.map_id, key.instance_id)?;
        let map = managed_map.map();
        let player = map.get_typed_player(player_guid)?;
        if !player.unit().world().object().is_in_world() {
            return None;
        }
        let player_position = player.unit().world().position();
        let player_phase_shift = player.unit().world().phase_shift().clone();
        let visibility_range = map.visibility_range();
        let mut shared_vision_target_guids = map
            .typed_combat_unit_guids_like_cpp()
            .into_iter()
            .filter(|target_guid| *target_guid != player_guid)
            .filter(|target_guid| {
                represented_seer_guid.is_some_and(|seer_guid| seer_guid == *target_guid)
            })
            .filter(|target_guid| {
                map.get_typed_player(*target_guid).is_some_and(|target| {
                    target.unit().world().object().is_in_world()
                        && target
                            .unit()
                            .subsystems()
                            .control
                            .shared_vision_guids
                            .contains(&player_guid)
                }) || map
                    .with_creature_like_cpp(*target_guid, |target| {
                        target.unit().world().object().is_in_world()
                            && target
                                .unit()
                                .subsystems()
                                .control
                                .shared_vision_guids
                                .contains(&player_guid)
                    })
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        shared_vision_target_guids.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
        let dynamic_object_target_guid = represented_seer_guid.filter(|seer_guid| {
            if !seer_guid.is_dynamic_object() {
                return false;
            }
            let Some(dynamic_object) = map.get_typed_dynamic_object(*seer_guid) else {
                return false;
            };
            if !dynamic_object.world().object().is_in_world() {
                return false;
            }
            let caster_guid = dynamic_object
                .bound_caster()
                .unwrap_or_else(|| dynamic_object.caster_guid());
            caster_guid == player_guid && caster_guid.is_player()
        });
        let summary = managed_map.last_game_objects_update_summary();
        let guids = select_guids(&summary)
            .into_iter()
            .filter(|guid| {
                if !guid.is_game_object() {
                    return false;
                }
                let Some(gameobject) = map.get_typed_game_object(*guid) else {
                    return self.core.client_visible_guids_like_cpp.contains(guid);
                };
                if !gameobject.world().object().is_in_world() {
                    return false;
                }
                let gameobject_phase_shift = self
                    .world_entities
                    .represented_gameobject_phase_shift_like_cpp(*guid)
                    .unwrap_or_else(|| gameobject.world().phase_shift());
                let gameobject_position = gameobject.world().position();
                let direct_target_allows = direct_target_seer_gate_allows_send
                    && player_phase_shift.can_see(gameobject_phase_shift)
                    && gameobject_position.is_within_dist_2d(&player_position, visibility_range);
                if direct_target_allows {
                    return true;
                }
                if shared_vision_target_guids.iter().any(|target_guid| {
                    if let Some(target) = map.get_typed_player(*target_guid) {
                        let target_world = target.unit().world();
                        target_world.phase_shift().can_see(gameobject_phase_shift)
                            && gameobject_position
                                .is_within_dist_2d(&target_world.position(), visibility_range)
                    } else {
                        map.with_creature_like_cpp(*target_guid, |target| {
                            let target_world = target.unit().world();
                            target_world.phase_shift().can_see(gameobject_phase_shift)
                                && gameobject_position
                                    .is_within_dist_2d(&target_world.position(), visibility_range)
                        })
                        .unwrap_or(false)
                    }
                }) {
                    return true;
                }
                dynamic_object_target_guid.is_some_and(|dynamic_object_guid| {
                    map.get_typed_dynamic_object(dynamic_object_guid)
                        .is_some_and(|dynamic_object| {
                            let dynamic_object_world = dynamic_object.world();
                            dynamic_object_world
                                .phase_shift()
                                .can_see(gameobject_phase_shift)
                                && gameobject_position.is_within_dist_2d(
                                    &dynamic_object_world.position(),
                                    visibility_range,
                                )
                        })
                })
            })
            .collect::<Vec<_>>();
        Some((
            key.map_id,
            key.instance_id,
            managed_map.update_calls().len() as u64,
            guids,
        ))
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_query/f3_shims.rs"]
mod f3_shims;
