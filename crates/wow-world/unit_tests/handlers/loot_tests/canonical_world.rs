//! Canonical map-object fixtures shared by loot authority and visibility tests.

use std::sync::{Arc, Mutex};
use wow_constants::{TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    AccessorObjectKind, CORPSE_DYNFLAG_LOOTABLE, Corpse, CorpseType, Creature, GameObject,
    WorldObject,
};

use crate::session::WorldSession;

pub(super) fn canonical_world_object(
    guid: ObjectGuid,
    map_id: u32,
    position: Position,
) -> WorldObject {
    let (type_id, type_mask) = if guid.is_game_object() {
        (TypeId::GameObject, TypeMask::GAME_OBJECT)
    } else {
        (TypeId::Unit, TypeMask::UNIT)
    };
    let mut object = WorldObject::new(false, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(map_id, 0).unwrap();
    object.relocate(position);
    object.object_mut().add_to_world();
    object
}

pub(super) fn attach_canonical_map_object(
    session: &mut WorldSession,
    kind: AccessorObjectKind,
    object: WorldObject,
) {
    let map_id = object.map_id();
    let instance_id = object.instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .add_to_map_like_cpp(kind, object)
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

pub(super) fn attach_loot_guid_allocator_for_owner(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
) {
    let kind = if owner_guid.is_game_object() {
        AccessorObjectKind::GameObject
    } else {
        AccessorObjectKind::Creature
    };
    attach_canonical_map_object(
        session,
        kind,
        canonical_world_object(owner_guid, u32::from(owner_guid.map_id()), Position::ZERO),
    );
}

/// Re-seat the canonical record for `guid` on the session's registered legacy
/// creature, so the fixture keeps ONE admitted incarnation.
///
/// F6-7 R7a. `attach_loot_guid_allocator_for_owner` attaches a canonical map
/// instance *after* `register_world_creature` published the legacy
/// representation, and it installs a separately constructed object for the same
/// GUID (a plain `WorldObject` record, not a typed `Creature`). The two sides
/// therefore belong to different incarnations — or the canonical side is not a
/// creature at all — so the R7 gate correctly refuses every owner mutation on
/// that creature. This helper derives the canonical record from the registered
/// legacy incarnation (same health-state revision authority, same loot
/// allocation, same entity state) and asserts the shared ownership it
/// establishes.
///
/// Independently constructed canonical creatures remain the subject of the
/// rejection tests, which must not call this helper.
pub(super) fn adopt_registered_creature_as_canonical_incarnation_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) {
    let legacy_map_id = u16::try_from(map_id).expect("test map id fits the legacy key");
    let legacy = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .find_creature(legacy_map_id, instance_id, guid)
        .expect("the fixture registered the legacy representation")
        .creature
        .clone();
    let mut guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = guard
        .find_map_mut(map_id, instance_id)
        .expect("the fixture attached the canonical map instance");
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_creature(legacy.clone())
                .expect("the registered creature is a valid canonical record"),
        )
        .expect("the registered creature replaces the fixture's placeholder record");
    let owner = map
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("the canonical record is a typed creature");
    assert!(
        legacy
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &owner.unit().health_state_revision_authority_like_cpp()
            ),
        "the canonical record must share the registered health timeline"
    );
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(owner.loot_authority_like_cpp()),
        "the canonical record must share the registered loot allocation"
    );
}

pub(super) fn attach_canonical_gameobject(session: &mut WorldSession, game_object: GameObject) {
    let map_id = game_object.world().map_id();
    let instance_id = game_object.world().instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_game_object(game_object).unwrap(),
            )
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

pub(super) fn attach_canonical_creature(session: &mut WorldSession, creature: Creature) {
    let map_id = creature.unit().world().map_id();
    let instance_id = creature.unit().world().instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

pub(super) fn attach_canonical_corpse(session: &mut WorldSession, corpse: Corpse) {
    let map_id = corpse.world().map_id();
    let instance_id = corpse.world().instance_id();
    let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    {
        let mut manager = manager.lock().unwrap();
        manager
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_corpse(corpse).unwrap())
            .unwrap();
    }
    session.set_canonical_map_manager(manager);
}

pub(super) fn make_canonical_corpse_for_session(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Corpse {
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePvp, 1_000);
    corpse.world_mut().object_mut().create(guid);
    corpse
        .world_mut()
        .set_map(u32::from(session.core.player_map_id_like_cpp()), 0)
        .unwrap();
    corpse.world_mut().relocate(Position::ZERO);
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
    corpse.clear_corpse_data_changes();
    corpse
}

pub(super) fn canonical_corpse_snapshot(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<Corpse> {
    let manager = session.core.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.core.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_corpse(guid).cloned()
}

pub(super) fn make_canonical_creature_for_session(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Creature {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(u32::from(session.core.player_map_id_like_cpp()), 0)
        .unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature
}

pub(super) fn canonical_creature_snapshot(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<Creature> {
    let manager = session.core.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.core.player_map_id_like_cpp()), 0)?;
    map.map().with_creature_like_cpp(guid, Clone::clone)
}

pub(super) fn make_canonical_gameobject_for_session(
    session: &WorldSession,
    guid: ObjectGuid,
    go_type: u8,
) -> GameObject {
    let mut game_object = GameObject::new();
    game_object.world_mut().object_mut().create(guid);
    game_object
        .world_mut()
        .set_map(u32::from(session.core.player_map_id_like_cpp()), 0)
        .unwrap();
    game_object.world_mut().relocate(Position::ZERO);
    game_object.world_mut().object_mut().add_to_world();
    game_object.set_go_type(go_type);
    game_object
}

pub(super) fn canonical_gameobject_snapshot(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<GameObject> {
    let manager = session.core.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.core.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_game_object(guid).cloned()
}
