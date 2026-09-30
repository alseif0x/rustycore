use super::*;

/// Source values for MessageToSet; membership and Player reads remain in APP.
#[derive(Debug)]
pub struct CreatureMessageSourceFacts {
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    position: Position,
    phase_shift: PhaseShift,
    visibility_range: f32,
}

impl CreatureMessageSourceFacts {
    pub(super) fn capture(creature: &Creature) -> Self {
        let world = creature.unit().world();
        Self {
            guid: creature.guid(),
            map_id: world.map_id(),
            instance_id: world.instance_id(),
            position: creature.position(),
            phase_shift: world.phase_shift().clone(),
            visibility_range: world
                .visibility_distance_override_like_cpp()
                .unwrap_or(crate::map_manager::VISIBILITY_RADIUS),
        }
    }

    pub fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub fn instance_id(&self) -> u32 {
        self.instance_id
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn phase_shift(&self) -> &PhaseShift {
        &self.phase_shift
    }

    pub fn visibility_range(&self) -> f32 {
        self.visibility_range
    }
}
