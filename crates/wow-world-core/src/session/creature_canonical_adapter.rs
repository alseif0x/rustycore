// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature canonical adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use std::collections::HashSet;

use crate::session::SharedCanonicalMapManager;
use wow_core::{ObjectGuid, Position};

pub fn relocate_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    position: Position,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().relocate_map_object_like_cpp(guid, position);
}

/// One creature mutation root outcome, ready to be emitted after the canonical
/// guard has been released.
///
/// #1263 C1. Since F6-8E the canonical incarnation is the only creature store,
/// so there is no transported representation left to admit: the record says
/// which root ran, on which creature and map key, whether the mutation
/// callback ran and what became of it.
#[derive(Debug, Clone, Copy)]
pub struct CreatureMutationRootCaptureLikeCpp {
    /// Production root that ran the mutation.
    pub root: &'static str,
    pub owner_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    /// Whether the root's mutation callback ran.
    pub mutation_invoked: bool,
    /// `applied` | `mutation_root_refused` | `canonical_application_refused` |
    /// `owner_unavailable` | `incarnation_absent`.
    pub application: &'static str,
    /// Whether the mutation landed on the canonical incarnation.
    pub applied: bool,
}

/// #1263 C1 capture: the single emission point for creature mutation root
/// outcomes. Callers copy a [`CreatureMutationRootCaptureLikeCpp`] under the
/// guard and call this **only after** it is released, so no I/O ever runs
/// inside a guard.
pub fn emit_creature_mutation_root_capture_like_cpp(capture: &CreatureMutationRootCaptureLikeCpp) {
    tracing::info!(
        target: "rustycore::capture::c1",
        capture_id = "C1",
        root = capture.root,
        owner_guid = %capture.owner_guid,
        map_id = capture.map_id,
        instance_id = capture.instance_id,
        mutation_invoked = capture.mutation_invoked,
        application = capture.application,
        applied = capture.applied,
        "RUSTYCORE_CAPTURE_C1 creature mutation root outcome"
    );
}

/// The creature's current threat-list GUIDs on the locked canonical map.
pub fn creature_threat_guids_on_map_like_cpp(
    map: &wow_map::ManagedMapInnerLikeCpp,
    guid: ObjectGuid,
) -> Vec<ObjectGuid> {
    map.with_creature_like_cpp(guid, |current| {
        current.unit().subsystems().combat.sorted_threat_guids()
    })
    .unwrap_or_default()
}

/// Bring every other unit's `threatened by me` reference in line with the
/// creature's threat list after the creature's combat state changed, given
/// the threat GUIDs it held before the change (C++ keeps both ends of a
/// `ThreatReference` on the live objects, so one change updates both).
pub fn reconcile_creature_threat_references_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    guid: ObjectGuid,
    old_threat_guids: Vec<ObjectGuid>,
) {
    let current_threat_guids: HashSet<_> = creature_threat_guids_on_map_like_cpp(map, guid)
        .into_iter()
        .collect();
    let removed_threat_guids: Vec<_> = old_threat_guids
        .into_iter()
        .filter(|threat_guid| !current_threat_guids.contains(threat_guid))
        .collect();
    for added_guid in current_threat_guids {
        let threat_ref = map
            .with_creature_like_cpp(guid, |creature| {
                creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_ref(added_guid)
                    .copied()
            })
            .flatten();
        let Some(threat_ref) = threat_ref else {
            continue;
        };
        if let Some(player) = map.get_typed_player_mut(added_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        } else if let Some(creature) = map.get_typed_creature_mut(added_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(guid, threat_ref);
        }
    }
    for removed_guid in removed_threat_guids {
        if let Some(player) = map.get_typed_player_mut(removed_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        } else if let Some(creature) = map.get_typed_creature_mut(removed_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(guid);
        }
    }
}

pub fn remove_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
) {
    let Ok(mut manager) = manager.lock() else {
        return;
    };
    remove_canonical_creature_map_object_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        guid,
    );
}

/// [`remove_canonical_creature_map_object_on_map_like_cpp`] on a canonical
/// manager the caller already holds (#1263 F6-8D3a-3: the admitted executor
/// runs the lifecycle under its one canonical guard).
pub fn remove_canonical_creature_map_object_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
) {
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return;
    };
    let _ = map.map_mut().remove_from_map_like_cpp(guid, true);
}

pub fn add_canonical_creature_respawn_info_and_remove_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    info: wow_map::RespawnInfoLikeCpp,
) -> (bool, bool) {
    let Ok(mut manager) = manager.lock() else {
        return (false, false);
    };
    add_canonical_creature_respawn_info_and_remove_map_object_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        guid,
        info,
    )
}

pub fn add_canonical_creature_respawn_info_and_remove_map_object_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    guid: ObjectGuid,
    info: wow_map::RespawnInfoLikeCpp,
) -> (bool, bool) {
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return (false, false);
    };

    let respawn_added = matches!(
        map.map_mut().add_respawn_info_like_cpp(info),
        wow_map::AddRespawnInfoOutcomeLikeCpp::Inserted
            | wow_map::AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    let object_removed = map.map_mut().remove_from_map_like_cpp(guid, true).is_ok();
    (respawn_added, object_removed)
}

pub fn remove_canonical_respawn_time_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    let Ok(mut manager) = manager.lock() else {
        return false;
    };
    remove_canonical_respawn_time_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        object_type,
        spawn_id,
    )
}

pub fn remove_canonical_respawn_time_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    object_type: wow_map::SpawnObjectType,
    spawn_id: wow_map::SpawnId,
) -> bool {
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return false;
    };
    map.map_mut()
        .remove_respawn_time_like_cpp(object_type, spawn_id)
        .is_some()
}
