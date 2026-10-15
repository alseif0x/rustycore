// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The dynamic-object VALUES publication from the last map-owned
//! `Map::SendObjectUpdates` snapshot (#1363 slice 11a; moved from the World
//! `session/instances/map_key.rs`).

use wow_world_core::entity_update_bridge::dynamic_object_values_update_to_update_object;

use super::VisibilitySessionHostLikeCpp;

/// Consume the last map-owned represented `Map::SendObjectUpdates` stable
/// DynamicObject VALUES snapshot into this session's outbound packet stream.
///
/// Source of truth remains canonical `Map::map_objects` as snapshotted by
/// `ManagedMap::last_send_object_updates_summary_like_cpp()`. This helper
/// gates snapshot delivery through represented direct Player,
/// PlayerMapType/CreatureMapType shared-vision, or DynamicObjectMapType
/// receiver-source evidence before the final session `HaveAtClient`
/// visibility check, and never reads live DynamicObject changed masks or
/// mutates canonical map state.
pub fn send_represented_dynamic_object_values_updates_from_last_map_send_object_updates_like_cpp<
    H: VisibilitySessionHostLikeCpp,
>(
    host: &mut H,
) -> usize {
    let Some(key) = host.core().current_canonical_player_map_key_like_cpp() else {
        return 0;
    };
    let Ok(packet_map_id) = u16::try_from(key.map_id) else {
        return 0;
    };
    let Some(manager) = host.core().canonical_map_manager.as_ref() else {
        return 0;
    };
    let Some(player_guid) = host.player_guid() else {
        return 0;
    };
    let represented_seer_guid = host.current_seer_guid_like_cpp();
    let (update_generation, updates) = {
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
        let player_world = player.unit().world();
        let player_phase_shift = player_world.phase_shift().clone();
        let player_position = player_world.position();
        let visibility_range = map.visibility_range();
        let shared_vision_source_guids = map
            .typed_combat_unit_guids_like_cpp()
            .into_iter()
            .filter(|source_guid| *source_guid != player_guid)
            .filter(|source_guid| {
                map.get_typed_player(*source_guid).is_some_and(|source| {
                    source.unit().world().object().is_in_world()
                        && source
                            .unit()
                            .subsystems()
                            .control
                            .shared_vision_guids
                            .contains(&player_guid)
                }) || map
                    .with_creature_like_cpp(*source_guid, |source| {
                        source.unit().world().object().is_in_world()
                            && source
                                .unit()
                                .subsystems()
                                .control
                                .shared_vision_guids
                                .contains(&player_guid)
                    })
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        let dynamic_object_seer_guid = represented_seer_guid.filter(|seer_guid| {
            if !seer_guid.is_dynamic_object() {
                return false;
            }
            let Some(seer) = map.get_typed_dynamic_object(*seer_guid) else {
                return false;
            };
            if !seer.world().object().is_in_world() {
                return false;
            }
            let caster_guid = seer.bound_caster().unwrap_or_else(|| seer.caster_guid());
            caster_guid == player_guid && caster_guid.is_player()
        });

        let updates = managed_map
            .last_send_object_updates_summary_like_cpp()
            .dynamic_object_values_updates
            .into_iter()
            .filter(|represented_update| {
                let guid = represented_update.guid;
                if !guid.is_dynamic_object() {
                    return false;
                }
                let Some(updated) = map.get_typed_dynamic_object(guid) else {
                    return false;
                };
                if !updated.world().object().is_in_world() {
                    return false;
                }
                // C++ visibility distance is 2D (CanSeeOrDetect -> GetSightRange ->
                // IsWithinDist(obj, range, is3D=false); Object.cpp:1587-1609). Mirror the
                // already-2D entry filters and the creature/GameObject values fanout.
                let updated_world = updated.world();
                let direct_player_allows = player_phase_shift.can_see(updated_world.phase_shift())
                    && updated_world
                        .position()
                        .is_within_dist_2d(&player_position, visibility_range);
                if direct_player_allows {
                    return true;
                }
                if shared_vision_source_guids.iter().any(|source_guid| {
                    if let Some(source) = map.get_typed_player(*source_guid) {
                        let source_world = source.unit().world();
                        source_world
                            .phase_shift()
                            .can_see(updated_world.phase_shift())
                            && updated_world
                                .position()
                                .is_within_dist_2d(&source_world.position(), visibility_range)
                    } else {
                        map.with_creature_like_cpp(*source_guid, |source| {
                            let source_world = source.unit().world();
                            source_world
                                .phase_shift()
                                .can_see(updated_world.phase_shift())
                                && updated_world
                                    .position()
                                    .is_within_dist_2d(&source_world.position(), visibility_range)
                        })
                        .unwrap_or(false)
                    }
                }) {
                    return true;
                }
                dynamic_object_seer_guid.is_some_and(|seer_guid| {
                    map.get_typed_dynamic_object(seer_guid).is_some_and(|seer| {
                        let seer_world = seer.world();
                        seer_world
                            .phase_shift()
                            .can_see(updated_world.phase_shift())
                            && updated_world
                                .position()
                                .is_within_dist_2d(&seer_world.position(), visibility_range)
                    })
                })
            })
            .collect::<Vec<_>>();
        (managed_map.update_calls().len() as u64, updates)
    };

    use wow_packet::ServerPacket;

    let mut sent = 0;
    for represented_update in updates {
        let guid = represented_update.guid;
        if !host.core().client_visible_guids_like_cpp.contains(&guid) {
            continue;
        }
        let Some(update) = dynamic_object_values_update_to_update_object(
            guid,
            packet_map_id,
            &represented_update.values_update,
        ) else {
            continue;
        };
        let bytes = update.to_bytes();
        let fingerprint =
            wow_world_visibility::represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
                guid, &bytes,
            );
        if !host
            .visibility_mut()
            .admit_dynamic_object_values_update_delivery_like_cpp(
                key.map_id,
                key.instance_id,
                update_generation,
                guid,
                fingerprint,
            )
        {
            continue;
        }
        host.send_packet(&update);
        sent += 1;
    }
    sent
}
