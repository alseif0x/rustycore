//! Dormant quiescent ownership operation; no tick or producer is activated here.

use super::{MapKey, MapManager, MapTickCoordinationStateLikeCpp};
use crate::map::{CreatureActorTransportError, CreatureActorTransportSummary};
use crate::map_manager::MapInstance;

impl MapManager {
    /// APP holds its existing writer fence and both manager guards, and must
    /// invoke this only before startup admission or after permanent closure.
    /// Idle is necessary but is not evidence that external writers quiesced.
    pub fn transport_legacy_creature_ownership(
        &mut self,
        key: MapKey,
        incarnation: u64,
        source: &mut MapInstance,
    ) -> Result<CreatureActorTransportSummary, CreatureActorTransportError> {
        if self.tick_coordination_like_cpp() != MapTickCoordinationStateLikeCpp::Idle {
            return Err(CreatureActorTransportError::MapBusy);
        }
        let current_incarnation = self
            .map_incarnation_like_cpp(key)
            .ok_or(CreatureActorTransportError::MissingMap)?;
        if current_incarnation != incarnation {
            return Err(CreatureActorTransportError::StaleIncarnation);
        }
        let managed = self
            .maps
            .get_mut(&key)
            .ok_or(CreatureActorTransportError::MissingMap)?;
        managed
            .map_mut()
            .transport_legacy_creature_ownership(source)
    }
}

#[cfg(test)]
mod tests;
