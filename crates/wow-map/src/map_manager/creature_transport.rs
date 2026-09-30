//! Exact-slot, move-only extraction for the quiescent ownership transition.
//! These primitives neither register actors nor replay their lifecycle.

use super::{GridCoord, MapInstance, ObjectGuid, WorldCreature};
use std::collections::hash_map::Entry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacyCreatureTransportError {
    WrongInstance,
    MissingGrid,
    GridCoordinateMismatch,
    MissingActor,
    GuidMismatch,
    ActorMapMismatch,
    DuplicateGuid,
    Occupied,
}

/// Only this owner can construct a slot. Its metadata is not an ABA witness:
/// the same existing MapInstance stays exclusively borrowed through commit.
#[derive(Debug)]
pub(crate) struct LegacyCreatureTransportSlot {
    map_id: u16,
    instance_id: u32,
    grid: GridCoord,
    guid: ObjectGuid,
}

#[derive(Debug)]
pub(crate) struct TakenLegacyCreature {
    slot: LegacyCreatureTransportSlot,
    actor: WorldCreature,
}

impl TakenLegacyCreature {
    pub(crate) fn into_actor(self) -> WorldCreature {
        self.actor
    }
}

impl MapInstance {
    pub(crate) fn preflight_creature_transport_slot(
        &self,
        coord: GridCoord,
        guid: ObjectGuid,
    ) -> Result<LegacyCreatureTransportSlot, LegacyCreatureTransportError> {
        let grid = self
            .grids
            .get(&coord)
            .ok_or(LegacyCreatureTransportError::MissingGrid)?;
        if grid.coord != coord {
            return Err(LegacyCreatureTransportError::GridCoordinateMismatch);
        }
        let actor = grid
            .creatures
            .get(&guid)
            .ok_or(LegacyCreatureTransportError::MissingActor)?;
        if actor.guid() != guid {
            return Err(LegacyCreatureTransportError::GuidMismatch);
        }
        if actor.map_id() != u32::from(self.map_id) || actor.instance_id() != self.instance_id {
            return Err(LegacyCreatureTransportError::ActorMapMismatch);
        }
        if self.grids.iter().any(|(other_coord, other)| {
            *other_coord != coord && other.creatures.contains_key(&guid)
        }) {
            return Err(LegacyCreatureTransportError::DuplicateGuid);
        }
        Ok(LegacyCreatureTransportSlot {
            map_id: self.map_id,
            instance_id: self.instance_id,
            grid: coord,
            guid,
        })
    }

    pub(crate) fn take_creature_transport_slot(
        &mut self,
        slot: LegacyCreatureTransportSlot,
    ) -> Result<TakenLegacyCreature, (LegacyCreatureTransportError, LegacyCreatureTransportSlot)>
    {
        if self.map_id != slot.map_id || self.instance_id != slot.instance_id {
            return Err((LegacyCreatureTransportError::WrongInstance, slot));
        }
        if let Err(error) = self.preflight_creature_transport_slot(slot.grid, slot.guid) {
            return Err((error, slot));
        }
        // This second lookup cannot fail: no mutation/callback separates the
        // borrowed preflight from remove on this exclusive instance borrow.
        let actor = self
            .grids
            .get_mut(&slot.grid)
            .expect("preflighted source grid remains present")
            .creatures
            .remove(&slot.guid)
            .expect("preflighted source actor remains present");
        Ok(TakenLegacyCreature { slot, actor })
    }

    pub(crate) fn restore_creature_transport_slot(
        &mut self,
        taken: TakenLegacyCreature,
    ) -> Result<(), (LegacyCreatureTransportError, TakenLegacyCreature)> {
        if self.map_id != taken.slot.map_id || self.instance_id != taken.slot.instance_id {
            return Err((LegacyCreatureTransportError::WrongInstance, taken));
        }
        if taken.actor.guid() != taken.slot.guid {
            return Err((LegacyCreatureTransportError::GuidMismatch, taken));
        }
        if taken.actor.map_id() != u32::from(taken.slot.map_id)
            || taken.actor.instance_id() != taken.slot.instance_id
        {
            return Err((LegacyCreatureTransportError::ActorMapMismatch, taken));
        }
        if self.grids.iter().any(|(coord, grid)| {
            *coord != taken.slot.grid && grid.creatures.contains_key(&taken.slot.guid)
        }) {
            return Err((LegacyCreatureTransportError::DuplicateGuid, taken));
        }
        let Some(grid) = self.grids.get_mut(&taken.slot.grid) else {
            return Err((LegacyCreatureTransportError::MissingGrid, taken));
        };
        if grid.coord != taken.slot.grid {
            return Err((LegacyCreatureTransportError::GridCoordinateMismatch, taken));
        }
        match grid.creatures.entry(taken.slot.guid) {
            Entry::Occupied(_) => Err((LegacyCreatureTransportError::Occupied, taken)),
            Entry::Vacant(slot) => {
                slot.insert(taken.actor);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;
