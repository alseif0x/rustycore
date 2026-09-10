// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map rules that read no map state.
//!
//! Moved out of the owner under #678. Every one was already receiver-free,
//! so it cannot read or write the owner's state: these are rules, not owner
//! behaviour. Bodies and signatures are unchanged.

use crate::map::*;
use crate::spawn::SpawnId;
use std::collections::HashMap;
use std::collections::HashSet;
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_entities::AccessorObjectKind;
use wow_entities::Creature;
use wow_entities::GameObject;
use wow_entities::MapObjectRecord;
use wow_entities::Player;
use wow_entities::Unit;

pub(crate) fn decrement_pool_counter_like_cpp(spawned_pools: &mut HashMap<u32, u32>, pool_id: u32) {
    let counter = spawned_pools.entry(pool_id).or_insert(0);
    if *counter > 0 {
        *counter -= 1;
    }
}

pub(crate) fn ensure_map_guid_sequence_source_like_cpp(
    high: HighGuid,
) -> Result<(), MapGuidSequenceErrorLikeCpp> {
    match high {
        HighGuid::WorldTransaction
        | HighGuid::StaticDoor
        | HighGuid::Transport
        | HighGuid::Conversation
        | HighGuid::Creature
        | HighGuid::Vehicle
        | HighGuid::Pet
        | HighGuid::GameObject
        | HighGuid::DynamicObject
        | HighGuid::AreaTrigger
        | HighGuid::Corpse
        | HighGuid::LootObject
        | HighGuid::SceneObject
        | HighGuid::Scenario
        | HighGuid::AIGroup
        | HighGuid::DynamicDoor
        | HighGuid::Vignette
        | HighGuid::CallForHelp
        | HighGuid::AIResource
        | HighGuid::AILock
        | HighGuid::AILockTicket
        | HighGuid::Cast => Ok(()),
        _ => Err(MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource { high }),
    }
}

pub(crate) fn gameobject_is_spawned_like_cpp(gameobject: &GameObject) -> bool {
    gameobject.respawn_delay_time() == 0
        || (gameobject.respawn_time() > 0 && !gameobject.spawned_by_default())
        || (gameobject.respawn_time() == 0 && gameobject.spawned_by_default())
}

pub(crate) fn map_record_is_unit_like_gameobject_owner_like_cpp(record: &MapObjectRecord) -> bool {
    matches!(
        record.kind(),
        AccessorObjectKind::Player | AccessorObjectKind::Creature | AccessorObjectKind::Pet
    ) && (record.player().is_some() || record.creature().is_some() || record.pet().is_some())
}

pub(crate) fn map_record_unit_like_cpp(record: &MapObjectRecord) -> Option<&Unit> {
    match record.kind() {
        AccessorObjectKind::Player => record.player().map(Player::unit),
        AccessorObjectKind::Creature => record.creature().map(Creature::unit),
        AccessorObjectKind::Pet => record.pet().map(|pet| pet.creature().unit()),
        _ => None,
    }
}

pub(crate) fn map_record_unit_mut_like_cpp(record: &mut MapObjectRecord) -> Option<&mut Unit> {
    match record.kind() {
        AccessorObjectKind::Player => record.player_mut().map(Player::unit_mut),
        AccessorObjectKind::Creature => record.creature_mut().map(Creature::unit_mut),
        AccessorObjectKind::Pet => record.pet_mut().map(|pet| pet.creature_mut().unit_mut()),
        _ => None,
    }
}

pub(crate) fn player_set_viewpoint_outcome_like_cpp(
    player_guid: ObjectGuid,
    target_guid: ObjectGuid,
    apply: bool,
    status: PlayerSetViewpointStatusLikeCpp,
    set_world_object: Option<SetWorldObjectOutcomeLikeCpp>,
    update_visibility_requested: bool,
    set_seer_requested: bool,
) -> PlayerSetViewpointOutcomeLikeCpp {
    PlayerSetViewpointOutcomeLikeCpp {
        player_guid,
        target_guid,
        apply,
        status,
        set_world_object,
        update_visibility_requested,
        set_seer_requested,
    }
}

pub(crate) fn remove_spawn_id_index_entry_like_cpp(
    index: &mut HashMap<SpawnId, HashSet<ObjectGuid>>,
    spawn_id: SpawnId,
    guid: ObjectGuid,
) {
    if spawn_id == 0 {
        return;
    }

    if let Some(guids) = index.get_mut(&spawn_id) {
        guids.remove(&guid);
        if guids.is_empty() {
            index.remove(&spawn_id);
        }
    }
}

pub(crate) fn snapshot_from_creature(
    guid: ObjectGuid,
    creature: &Creature,
) -> CreatureTransformVitalsSnapshotLikeCpp {
    let world = creature.unit().world();
    CreatureTransformVitalsSnapshotLikeCpp {
        guid,
        map_id: world.map_id(),
        instance_id: world.instance_id(),
        position: world.position(),
        combat_reach: world.combat_reach(),
        health: creature.current_health(),
        max_health: creature.max_health(),
        is_alive: creature.is_alive(),
        is_in_world: world.object().is_in_world(),
    }
}
