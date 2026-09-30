//! Local movement fixtures without application packet, config or worker resources.

use crate::map_manager::WorldCreature;
use super::CreatureMovementStep;
use wow_core::{ObjectGuid, Position};

pub(super) fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

pub(super) fn make_test_world_creature(guid: ObjectGuid) -> WorldCreature {
    WorldCreature::new(
        guid, 9999, Position::new(10.0, 10.0, 0.0, 0.0),
        25, 2, 3, 5, 20.0, 100, 14, 0, 0,
    )
}

pub(super) fn step(
    creature: &mut WorldCreature,
    diff_ms: u32,
    pathfinding_enabled: bool,
) -> Option<CreatureMovementStep> {
    creature.step_movement(
        diff_ms, None, None,
        |_, ignore| pathfinding_enabled && !ignore,
        |_, _, _, _| None,
    )
}
