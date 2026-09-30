//! Original LOS delegation contract and its entity-only fixtures.
use super::super::commit::is_creature_melee_los_clear_like_cpp;
use super::*;
use wow_constants::TypeId;

struct CreatureMeleeLosTestEnvironment {
    los: bool,
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

fn melee_los_test_world_object(
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

fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

#[test]
fn legacy_creature_melee_los_gate_delegates_to_map_environment_like_cpp() {
    let attacker = melee_los_test_world_object(
        test_creature_guid(91_026),
        TypeId::Unit,
        wow_constants::TypeMask::UNIT,
        Position::new(10.0, 10.0, 0.0, 0.0),
    );
    let victim = melee_los_test_world_object(
        ObjectGuid::create_player(1, 91_027),
        TypeId::Player,
        wow_constants::TypeMask::PLAYER,
        Position::new(11.0, 10.0, 0.0, 0.0),
    );

    assert!(!is_creature_melee_los_clear_like_cpp(
        &attacker,
        &victim,
        &CreatureMeleeLosTestEnvironment { los: false },
    ));
    assert!(is_creature_melee_los_clear_like_cpp(
        &attacker,
        &victim,
        &CreatureMeleeLosTestEnvironment { los: true },
    ));
}
