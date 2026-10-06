//! Represented gameobject state publication, despawn and respawn.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn record_represented_gameobject_runtime_state_like_cpp(
        &mut self,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: wow_core::Position,
        go_type: u8,
    ) {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.record_represented_gameobject_runtime_state_like_cpp(
            &mut hub, map_id, guid, entry, position, go_type,
        )
    }
    pub(in crate::session) fn represented_gameobject_go_state_for_viewer_like_cpp(
        &self,
        state: &RepresentedGameObjectUseState,
        now: Instant,
    ) -> wow_entities::GoState {
        if state.per_player_state_player_guid == self.player_guid()
            && state
                .per_player_go_state_until
                .is_none_or(|until| until > now)
            && let Some(per_player_go_state) = state.per_player_go_state
        {
            return per_player_go_state;
        }

        state.go_state.unwrap_or(wow_entities::GoState::Ready)
    }
    pub(in crate::session) fn send_represented_gameobject_despawn_to_visible_set_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) {
        use wow_packet::ServerPacket;

        let packet = wow_packet::packets::misc::GameObjectDespawn {
            object_guid: gameobject_guid,
        };
        self.send_packet(&packet);
        let _ = {
            let (s, h) = crate::session::split_loot_ref(self);
            s.queue_visible_gameobject_packet_for_same_map_like_cpp(
                h,
                gameobject_guid,
                packet.to_bytes(),
            )
        };
    }
    pub(in crate::session) fn send_represented_gameobject_delete_packets_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) {
        self.send_represented_gameobject_despawn_to_visible_set_like_cpp(gameobject_guid);
        self.world_entities
            .restore_represented_gameobject_override_flags_like_cpp(gameobject_guid);
        let map_id = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(gameobject_guid)
            .and_then(|state| state.map_id)
            .unwrap_or_else(|| self.core.player_map_id_like_cpp());
        self.send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
            vec![gameobject_guid],
            map_id,
        ));
    }
    pub(crate) fn send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(
        &mut self,
    ) -> usize {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(&mut hub)
    }
    /// Consume map-owned represented `GameObject::Update` `SendGameObjectDespawn()`
    /// evidence into this session's outbound stream without mutating canonical map
    /// state or erasing `Player::m_clientGUIDs` represented visibility.
    ///
    /// C++ source-of-truth: `GameObject::Update` in `GO_JUST_DEACTIVATED` calls
    /// `SendGameObjectDespawn()` when `IsDespawnAtAction()` or anim progress is
    /// non-zero; `SendGameObjectDespawn()` sends `SMSG_GAMEOBJECT_DESPAWN` to the
    /// visible set. `SendMessageToSetInRange` constructs `MessageDistDeliverer`
    /// with `required3dDist=false` by default, so this packet's visibility range
    /// gate is 2D (`GetExactDist2dSq`) rather than 3D. `MessageDistDeliverer`
    /// filters each target by `Player::InSamePhase(src->GetPhaseShift())` before
    /// range and `HaveAtClient`; this represented seam consumes the canonical
    /// typed Player phase and canonical typed GameObject phase, with only the
    /// existing represented GameObject phase map as an explicit legacy DB-spawn
    /// phase fallback matching create-visibility seams. It consumes only GUIDs
    /// from the canonical `ManagedMap` update summary and gates by same-map typed
    /// Player/GameObject, canonical Player in-world state because C++ only visits
    /// in-world `PlayerMapType` entries through `Cell::VisitWorldObjects`,
    /// GameObject in-world state, same-phase visibility, 2D visibility range,
    /// session-local `HaveAtClient`, and either the C++ direct-
    /// target `target->m_seer == target || target->GetVehicle()` branch for this
    /// session Player, the bounded represented `PlayerMapType`/`CreatureMapType`
    /// shared-vision fanout branch (`viewer->m_seer == target`) over canonical
    /// typed Player/Creature records in the same map, or the bounded represented
    /// `DynamicObjectMapType` branch for this session's caster/viewer when its
    /// represented `m_seer` is the canonical DynamicObject. Shared-vision and
    /// DynamicObject scanning here are deterministic and represented; this is not
    /// exact C++ cell traversal and does not claim skipped/team receivers,
    /// `GameObjectMapType`, ObjectAccessor/global fanout, or full
    /// `SendMessageToSet` parity.
    pub(crate) fn send_represented_gameobject_visual_despawn_from_last_update_like_cpp(
        &mut self,
    ) -> usize {
        let Some(key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return 0;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return 0;
        };
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let represented_seer_guid = self.current_seer_guid_like_cpp();
        let direct_target_seer_gate_allows_send = represented_seer_guid.is_none_or(|seer_guid| {
            seer_guid.is_empty()
                || seer_guid == player_guid
                || crate::session::hub_ref(self).represented_player_has_active_vehicle_like_cpp()
        });

        let (update_generation, despawnable_guids) = {
            let Ok(manager) = manager.lock() else {
                return 0;
            };
            let Some(managed_map) = manager.find_map(key.map_id, key.instance_id) else {
                return 0;
            };
            let map = managed_map.map();
            let Some(player) = map.get_typed_player(player_guid) else {
                return 0;
            };
            if !player.unit().world().object().is_in_world() {
                return 0;
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
            let guids = managed_map
                .last_game_objects_update_summary()
                .generic_visual_despawn_guids
                .iter()
                .copied()
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
                        && gameobject_position
                            .is_within_dist_2d(&player_position, visibility_range);
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
                                    && gameobject_position.is_within_dist_2d(
                                        &target_world.position(),
                                        visibility_range,
                                    )
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
            (managed_map.update_calls().len() as u64, guids)
        };

        let mut seen = std::collections::HashSet::new();
        let mut sent = 0;
        for guid in despawnable_guids {
            if !seen.insert(guid) || !self.core.client_visible_guids_like_cpp.contains(&guid) {
                continue;
            }
            if !self
                .visibility
                .admit_gameobject_visual_despawn_delivery_like_cpp(
                    key.map_id,
                    key.instance_id,
                    update_generation,
                    guid,
                )
            {
                continue;
            }
            {
                let (s, mut h) = crate::session::split_world_entities_mut(self);
                s.send_represented_gameobject_despawn_like_cpp(&mut h, guid)
            };
            sent += 1;
        }
        sent
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_state/f3_shims.rs"]
mod f3_shims;
