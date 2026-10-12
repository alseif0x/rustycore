//! Session-free creature lifecycle, movement and melee LOS fixtures.
//!
//! Moved from `wow-world` `session::tests::fixtures::lifecycle` by #1363 slice 3b,
//! so the creature runtime scenarios in `wow-world-creature` reach them too.

use wow_constants::TypeId;
use wow_core::{ObjectGuid, Position};

pub struct CreatureMeleeLosTestEnvironment {
    pub los: bool,
}

impl wow_entities::WorldObjectEnvironment for CreatureMeleeLosTestEnvironment {
    fn map_id(&self) -> u32 {
        0
    }

    fn instance_id(&self) -> u32 {
        0
    }

    fn visibility_range(&self) -> f32 {
        100.0
    }

    fn line_of_sight(&self, _query: wow_entities::LineOfSightQuery<'_>) -> bool {
        self.los
    }

    fn map_height(
        &self,
        _object: &wow_entities::WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: wow_entities::WorldObjectHeightQuery,
    ) -> f32 {
        wow_entities::INVALID_HEIGHT
    }

    fn floor_z(
        &self,
        _object: &wow_entities::WorldObject,
        _position: Position,
        _max_search_dist: f32,
    ) -> f32 {
        wow_entities::INVALID_HEIGHT
    }
}

pub fn melee_los_test_world_object(
    guid: ObjectGuid,
    type_id: TypeId,
    type_mask: wow_constants::TypeMask,
    position: Position,
) -> wow_entities::WorldObject {
    let mut object = wow_entities::WorldObject::new(true, type_id, type_mask);
    object.object_mut().create(guid);
    object.set_map(0, 0).unwrap();
    object.relocate(position);
    object.object_mut().add_to_world();
    object
}

pub fn lifecycle_test_map_store_like_cpp(
    map_id: u32,
    instance_type: i8,
    flags1: u32,
) -> wow_data::MapStore {
    wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1,
        flags2: 0,
    }])
}

// ── step_creature_movement_like_cpp unit tests (no WorldSession) ──────────

pub fn make_test_world_creature(
    guid: ObjectGuid,
) -> wow_entities::Creature {
    wow_world_core::creature_projection::new_runtime_creature_like_cpp(
        guid,
        9999,
        Position::new(10.0, 10.0, 0.0, 0.0),
        25,
        2,
        3,
        5,
        20.0,
        100,
        14,
        0,
        0,
    )
}
