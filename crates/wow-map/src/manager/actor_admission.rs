// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Fresh owned admission into an existing canonical map; no producer activation.

use super::{MapKey, MapManager};
use crate::map::{FreshCreatureActorAdmission, FreshCreatureActorAdmissionError};
use crate::map_manager::WorldCreature;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreshCreatureActorMapAdmissionError {
    MissingMap { key: MapKey },
    Admission(FreshCreatureActorAdmissionError),
}

impl MapManager {
    /// Admit only into the requested existing map. Return the complete incoming
    /// motor on any failure; never create a map, promote a Record or displace it.
    pub fn admit_fresh_creature_actor(
        &mut self,
        key: MapKey,
        incoming: WorldCreature,
    ) -> Result<FreshCreatureActorAdmission, (FreshCreatureActorMapAdmissionError, WorldCreature)>
    {
        let Some(managed) = self.maps.get_mut(&key) else {
            return Err((
                FreshCreatureActorMapAdmissionError::MissingMap { key },
                incoming,
            ));
        };
        managed
            .map_mut()
            .admit_fresh_creature_actor(incoming)
            .map_err(|(error, incoming)| {
                (
                    FreshCreatureActorMapAdmissionError::Admission(error),
                    incoming,
                )
            })
    }
}

#[cfg(test)]
mod tests;
