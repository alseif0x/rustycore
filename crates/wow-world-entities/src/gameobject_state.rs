use std::time::Instant;

use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use crate::WorldEntitiesState;

impl WorldEntitiesState {
    pub fn record_represented_gameobject_runtime_state_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: wow_core::Position,
        go_type: u8,
    ) {
        let linked_trap_guid =
            self.canonical_gameobject_linked_trap_guid_like_cpp(hub.shared(), guid);
        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.map_id = Some(map_id);
        state.position = Some(position);
        state.go_type = Some(go_type);
        if linked_trap_guid.is_some() {
            state.linked_trap_guid = linked_trap_guid;
        }
        self.upsert_canonical_gameobject_map_object_like_cpp(hub, map_id, guid, entry, position);
    }

    pub fn represented_gameobject_is_per_player_despawned_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> bool {
        let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) else {
            return false;
        };
        match state.per_player_despawn_until {
            Some(until) if until > Instant::now() => true,
            Some(_) => {
                state.per_player_despawn_until = None;
                state.per_player_despawn_secs = None;
                state.per_player_state_player_guid = None;
                false
            }
            None => false,
        }
    }

    pub fn send_represented_gameobject_despawn_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        gameobject_guid: ObjectGuid,
    ) {
        hub.core
            .send_packet(&wow_packet::packets::misc::GameObjectDespawn {
                object_guid: gameobject_guid,
            });
    }

    pub fn send_represented_gameobject_out_of_range_for_player_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        gameobject_guid: ObjectGuid,
        map_id: u16,
    ) {
        hub.core.send_packet(
            &wow_packet::packets::update::UpdateObject::out_of_range_objects(
                vec![gameobject_guid],
                map_id,
            ),
        );
    }

    /// Consume map-owned represented `GameObject::Update` `UpdateObjectVisibilityOnDestroy`
    /// evidence into this session's C++-like `HaveAtClient` state.
    ///
    /// C++ source-of-truth: `GameObject::Update` calls
    /// `UpdateObjectVisibilityOnDestroy()`, which delegates to
    /// `WorldObject::DestroyForNearbyPlayers`; that visits only Players inside
    /// `GetVisibilityRange()`, checks `HaveAtClient`, sends destroy for the
    /// player, then erases the object's GUID from `Player::m_clientGUIDs`.
    /// `AnyPlayerInObjectRangeCheck(..., false)` intentionally does not filter
    /// by `Player::IsAlive()`, so dead players remain eligible when phase,
    /// range, and `HaveAtClient` pass. This represented seam consumes only the
    /// exact GUIDs snapshotted in the
    /// canonical `ManagedMap` update summary, gates by same-map canonical Player
    /// position plus typed GameObject in-world state/range, and never mutates
    /// canonical map objects.
    pub fn send_represented_gameobject_visibility_on_destroy_from_last_update_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> usize {
        let Some(key) = hub.core.current_canonical_player_map_key_like_cpp() else {
            return 0;
        };
        let Ok(packet_map_id) = u16::try_from(key.map_id) else {
            return 0;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return 0;
        };
        let player_guid = hub.core.player_guid();
        let destroyable_guids = {
            let Some(player_guid) = player_guid else {
                return 0;
            };
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
            let player_world = player.unit().world().clone();
            let player_phase_shift = player_world.phase_shift().clone();
            let visibility_range = map.visibility_range();
            let represented_gameobject_phase_shifts = &self.represented_gameobject_phase_shifts;

            managed_map
                .last_game_objects_update_summary()
                .generic_visibility_on_destroy_guids
                .iter()
                .copied()
                .filter(|guid| {
                    guid.is_game_object()
                        && map.get_typed_game_object(*guid).is_some_and(|gameobject| {
                            if !gameobject.world().object().is_in_world() {
                                return false;
                            }
                            let gameobject_phase_shift = represented_gameobject_phase_shifts
                                .get(guid)
                                .unwrap_or_else(|| gameobject.world().phase_shift());
                            player_phase_shift.can_see(gameobject_phase_shift)
                                && gameobject.world().is_within_dist(
                                    &player_world,
                                    visibility_range,
                                    // is3D=false: C++ visibility distance is 2D
                                    // (CanSeeOrDetect -> IsWithinDist is3D=false; Object.cpp:1587-1609).
                                    false,
                                    true,
                                    true,
                                )
                        })
                })
                .collect::<Vec<_>>()
        };

        let mut seen = std::collections::HashSet::new();
        let mut sent = 0;
        for guid in destroyable_guids {
            if !seen.insert(guid) {
                continue;
            }
            if !hub.core.client_visible_guids_like_cpp.remove(&guid) {
                continue;
            }
            hub.core
                .send_packet(&wow_packet::packets::update::UpdateObject::destroy_objects(
                    vec![guid],
                    packet_map_id,
                ));
            sent += 1;
        }
        sent
    }
}
