//! Owned immutable reads for the current Creature visibility adapters.
//!
//! This compatible observation intentionally derives CREATE data from the
//! inner Creature and supplies no canonical runtime spline, including for an
//! Actor. Reading an Actor's own presentation/runtime is a separate future
//! operation. No producers, precedence, busy gates or visibility clocks change.
//! C++ anchors: Map.cpp:3444, ObjectAccessor.cpp:206/217/241,
//! Object.cpp:1516-1742, SpellAuras.cpp:229-291 (a5f8da2e).

mod auras;
mod capture;
mod create;
mod message;

pub use auras::{CreatureAuraSlotFacts, CreatureInitialAuraFacts};
pub use create::CreatureCreateFacts;
pub use message::CreatureMessageSourceFacts;

use super::{GridLifecycle, Map, TerrainGridLoader};
use crate::map_manager::WorldCreature;
use wow_core::{ObjectGuid, Position};
use wow_entities::{Creature, PhaseShift, UnitVisibilityTargetFacts};

/// All target values from the same read window; the caller observes Player later.
#[derive(Debug)]
pub struct CreatureVisibilityCandidate {
    target: UnitVisibilityTargetFacts,
    create: CreatureCreateFacts,
    initial_auras: CreatureInitialAuraFacts,
}

impl CreatureVisibilityCandidate {
    pub fn visibility_target(&self) -> &UnitVisibilityTargetFacts {
        &self.target
    }

    pub fn create(&self) -> &CreatureCreateFacts {
        &self.create
    }

    pub fn initial_auras(&self) -> &CreatureInitialAuraFacts {
        &self.initial_auras
    }

    pub fn guid(&self) -> ObjectGuid {
        self.create.guid()
    }

    fn compatible(creature: &Creature) -> Self {
        Self {
            target: creature.unit().capture_visibility_target(),
            create: CreatureCreateFacts::compatible(creature),
            initial_auras: CreatureInitialAuraFacts::capture(
                creature.guid(),
                creature.level(),
                &creature.unit().subsystems().auras,
            ),
        }
    }
}

#[cfg(test)]
mod tests;
