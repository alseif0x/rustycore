// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session consumption of map-owned `Map::SendObjectUpdates` snapshots.
//!
//! The map owns update-field mutation and clears masks while it still owns the
//! map guard.  This module consumes the resulting owned snapshots afterwards,
//! applies the receiver's committed visibility, and writes packets without
//! retaining a map or directory guard across the send.

use super::*;
use wow_entities::{
    PlayerValuesUpdate, TYPEID_ACTIVE_PLAYER, TYPEID_OBJECT, TYPEID_PLAYER, TYPEID_UNIT,
    UnitValuesUpdate, UpdateFieldSectionKind, UpdateFieldVisibilityFlags, filter_disallowed_fields,
};
use wow_packet::ServerPacket;

pub(crate) fn dynamic_object_create_data_from_canonical_like_cpp(
    guid: wow_core::ObjectGuid,
    dynamic_object: &wow_entities::DynamicObject,
) -> wow_packet::packets::update::DynamicObjectCreateData {
    let object = dynamic_object.world();
    let object_data = object.object().object_data_values();
    let data = dynamic_object.data();
    wow_packet::packets::update::DynamicObjectCreateData {
        guid,
        entry_id: u32::try_from(object_data.entry_id).unwrap_or(0),
        dynamic_flags: object_data.dynamic_flags,
        scale: object_data.scale,
        position: object.position(),
        caster: data.caster,
        dynamic_object_type: data.dynamic_object_type,
        spell_visual_id: data.spell_visual_id,
        spell_id: data.spell_id,
        radius: data.radius,
        cast_time_ms: data.cast_time_ms,
    }
}

pub(crate) fn represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
    guid: wow_core::ObjectGuid,
    bytes: &[u8],
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    guid.hash(&mut hasher);
    bytes.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn represented_gameobject_dynamic_flags_update_like_cpp(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    dynamic_flags: u32,
) -> Option<wow_packet::packets::update::UpdateObject> {
    let mut mask = wow_entities::UpdateMask::new(wow_entities::OBJECT_DATA_BITS);
    mask.set(wow_entities::OBJECT_DATA_PARENT_BIT);
    mask.set(wow_entities::OBJECT_DATA_DYNAMIC_FLAGS_BIT);
    let values_update = wow_entities::GameObjectValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
        object_data: Some(wow_entities::ObjectDataUpdate {
            mask,
            values: wow_entities::ObjectDataValues {
                entry_id: 0,
                dynamic_flags,
                scale: 0.0,
            },
        }),
        game_object_data: None,
    };
    crate::entity_update_bridge::game_object_values_update_to_update_object(
        guid,
        map_id,
        &values_update,
    )
}

fn filter_unit_values_update_for_target_like_cpp(update: &mut UnitValuesUpdate, owner: bool) {
    let flags = owner
        .then_some(UpdateFieldVisibilityFlags::OWNER)
        .unwrap_or_else(UpdateFieldVisibilityFlags::empty);
    if let Some(unit_data) = update.unit_data.as_mut() {
        filter_disallowed_fields(UpdateFieldSectionKind::UnitData, &mut unit_data.mask, flags);
        if !unit_data.mask.is_any_set() {
            update.unit_data = None;
        }
    }
    update.changed_object_type_mask = update
        .object_data
        .as_ref()
        .map(|_| 1_u32 << TYPEID_OBJECT)
        .unwrap_or(0)
        | update
            .unit_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_UNIT)
            .unwrap_or(0);
}

fn filter_player_values_update_for_target_like_cpp(update: &mut PlayerValuesUpdate, owner: bool) {
    let flags = owner
        .then_some(UpdateFieldVisibilityFlags::OWNER)
        .unwrap_or_else(UpdateFieldVisibilityFlags::empty);
    if let Some(unit_data) = update.unit_data.as_mut() {
        filter_disallowed_fields(UpdateFieldSectionKind::UnitData, &mut unit_data.mask, flags);
        if !unit_data.mask.is_any_set() {
            update.unit_data = None;
        }
    }
    if let Some(player_data) = update.player_data.as_mut() {
        filter_disallowed_fields(
            UpdateFieldSectionKind::PlayerData,
            &mut player_data.mask,
            flags,
        );
        if !player_data.mask.is_any_set() {
            update.player_data = None;
        }
    }
    if !owner {
        update.active_player_data = None;
    }
    update.changed_object_type_mask = update
        .object_data
        .as_ref()
        .map(|_| 1_u32 << TYPEID_OBJECT)
        .unwrap_or(0)
        | update
            .unit_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_UNIT)
            .unwrap_or(0)
        | update
            .player_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_PLAYER)
            .unwrap_or(0)
        | update
            .active_player_data
            .as_ref()
            .map(|_| 1_u32 << TYPEID_ACTIVE_PLAYER)
            .unwrap_or(0);
}

impl WorldSession {
    /// Deliver the Player and Unit VALUES snapshots captured by the canonical
    /// map's `Map::SendObjectUpdates` phase.
    ///
    /// C++ `WorldObject::BuildUpdate` (`Object.cpp:3680-3728`) builds one
    /// viewer-specific block for every nearby player that already has the
    /// source in `m_clientGUIDs`.  The map snapshot is immutable by the time
    /// this function runs, so the session performs the same final map,
    /// instance, incarnation, phase, distance and `HaveAtClient` checks before
    /// sending.  Player self updates retain ActivePlayerData; observer updates
    /// strip that owner-only section, matching `BuildValuesUpdate`'s target
    /// flags.  Creature/Pet values use the existing shared Unit update bridge;
    /// viewer-dependent loot projections remain owned by their dedicated
    /// producer paths.
    pub(crate) fn send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(
        &mut self,
    ) -> usize {
        let Some(key) = self.current_canonical_player_map_key_like_cpp() else {
            return 0;
        };
        let Ok(packet_map_id) = u16::try_from(key.map_id) else {
            return 0;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return 0;
        };
        let Some(viewer_guid) = self.player_guid() else {
            return 0;
        };

        let mut packets = Vec::new();
        let update_generation;
        {
            let Ok(manager) = manager.lock() else {
                return 0;
            };
            let Some(managed_map) = manager.find_map(key.map_id, key.instance_id) else {
                return 0;
            };
            let map = managed_map.map();
            let Some(viewer) = map.get_typed_player(viewer_guid) else {
                return 0;
            };
            if !viewer.unit().world().object().is_in_world() {
                return 0;
            }
            let viewer_world = viewer.unit().world();
            let viewer_phase = viewer_world.phase_shift().clone();
            let viewer_position = viewer_world.position();
            let visibility_range = map.visibility_range();
            update_generation = managed_map.update_calls().len() as u64;
            let summary = managed_map.last_send_object_updates_summary_like_cpp();

            for represented_update in summary.player_values_updates {
                let source_guid = represented_update.guid;
                let source_is_viewer = source_guid == viewer_guid;
                let source_visible = source_is_viewer
                    || (self.client_visible_guids_like_cpp.contains(&source_guid)
                        && map.get_typed_player(source_guid).is_some_and(|source| {
                            let source_world = source.unit().world();
                            source_world.object().is_in_world()
                                && viewer_phase.can_see(source_world.phase_shift())
                                && source_world
                                    .position()
                                    .is_within_dist_2d(&viewer_position, visibility_range)
                        }));
                if !source_visible {
                    continue;
                }

                let mut values_update = represented_update.values_update;
                // `BuildValuesUpdate` filters UnitData/PlayerData by the
                // receiver's flags and exposes ActivePlayerData only to the
                // owner. The map snapshot is captured once with the complete
                // masks, so apply that receiver-specific filter here.
                filter_player_values_update_for_target_like_cpp(
                    &mut values_update,
                    source_is_viewer,
                );
                let Some(update) = player_values_update_to_update_object(
                    source_guid,
                    packet_map_id,
                    &values_update,
                ) else {
                    continue;
                };
                packets.push((source_guid, update.to_bytes()));
            }

            for represented_update in summary.unit_values_updates {
                let source_guid = represented_update.guid;
                if !self.client_visible_guids_like_cpp.contains(&source_guid) {
                    continue;
                }
                let source_visibility = map
                    .with_creature_or_pet_like_cpp(source_guid, |source, owner| {
                        let source_world = source.unit().world();
                        (
                            source_world.object().is_in_world()
                                && viewer_phase.can_see(source_world.phase_shift())
                                && source_world
                                    .position()
                                    .is_within_dist_2d(&viewer_position, visibility_range),
                            owner == Some(viewer_guid),
                        )
                    })
                    .unwrap_or((false, false));
                if !source_visibility.0 {
                    continue;
                }
                let mut values_update = represented_update.values_update;
                filter_unit_values_update_for_target_like_cpp(
                    &mut values_update,
                    source_visibility.1,
                );
                let Some(update) =
                    unit_values_update_to_update_object(source_guid, packet_map_id, &values_update)
                else {
                    continue;
                };
                packets.push((source_guid, update.to_bytes()));
            }
        }

        // A session can change instance while the map lock is released.  The
        // snapshot is scoped to the admitted map key; fail closed before any
        // packet leaves the Session if that key is no longer current.
        if self.current_canonical_player_map_key_like_cpp() != Some(key) {
            return 0;
        }

        let mut sent = 0;
        for (source_guid, bytes) in packets {
            // Recheck `HaveAtClient` after dropping the map lock.  A queued
            // visibility transition may have retired the source meanwhile.
            if source_guid != viewer_guid
                && !self.client_visible_guids_like_cpp.contains(&source_guid)
            {
                continue;
            }
            let fingerprint = crate::session::
                represented_dynamic_object_values_update_delivery_fingerprint_like_cpp(
                    source_guid,
                    &bytes,
                );
            if !self
                .represented_player_unit_values_updates_delivered_like_cpp
                .insert((
                    key.map_id,
                    key.instance_id,
                    update_generation,
                    source_guid,
                    fingerprint,
                ))
            {
                continue;
            }
            self.send_raw_packet(&bytes);
            sent += 1;
        }
        sent
    }
}
