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
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
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
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
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
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
        .unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature
}

pub(super) fn canonical_creature_snapshot(
    session: &WorldSession,
    guid: ObjectGuid,
) -> Option<Creature> {
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
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
        .set_map(u32::from(session.player_map_id_like_cpp()), 0)
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
    let manager = session.canonical_map_manager.as_ref()?;
    let manager = manager.lock().ok()?;
    let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
    map.map().get_typed_game_object(guid).cloned()
}
