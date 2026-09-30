//! Narrow owned publication facts captured after the terminal Step.

use super::*;
use crate::{MapKey, map_manager::WorldCreature};
use wow_core::Position;
use wow_entities::{CreatureAiState, UnitValuesUpdate};

#[derive(Debug)]
pub struct ActorMovementTraceFacts {
    pub entry: u32,
    pub map_id: u32,
    pub state: CreatureAiState,
}

/// Contains no actor, generator, RNG, borrowed entity or publication authority.
/// The application retains the original Health-before-Move event ordering.
#[derive(Debug)]
pub struct ActorMovementCompletion {
    pub guid: ObjectGuid,
    pub key: MapKey,
    pub incarnation: u64,
    pub position: Position,
    pub visibility_range: f32,
    pub movement: Option<CreatureMovementStep>,
    pub home_health_update: Option<UnitValuesUpdate>,
    pub trace: ActorMovementTraceFacts,
}

pub(super) fn capture(
    actor: &mut WorldCreature,
    guid: ObjectGuid,
    key: MapKey,
    incarnation: u64,
    movement: Option<CreatureMovementStep>,
) -> ActorMovementCompletion {
    let position = actor.position();
    let home_health_update = actor.take_home_health_restored_pending_like_cpp()
        .then(|| actor.creature.unit().values_update());
    ActorMovementCompletion {
        guid, key, incarnation, position,
        visibility_range: actor.visibility_range_like_cpp(),
        movement, home_health_update,
        trace: ActorMovementTraceFacts {
            entry: actor.entry(), map_id: actor.map_id(), state: actor.state(),
        },
    }
}
