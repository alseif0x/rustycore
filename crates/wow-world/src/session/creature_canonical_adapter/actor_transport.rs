//! Prepared ownership transport. Composition must establish real quiescence;
//! this operation does not stop producers or reopen a permanently closed rail.

use super::super::{SharedCanonicalMapManager, WorldSession};
use crate::map_manager::SharedMapManager;
use wow_map::{CreatureActorTransportError, CreatureActorTransportSummary, MapKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureOwnershipTransferError {
    InvalidMapId,
    CanonicalUnavailable,
    LegacyUnavailable,
    Admission(CreatureActorTransportError),
}

impl WorldSession {
    /// Only before any startup admission, or after the existing permanent
    /// closure has actually settled all producers/borrowers. Idle alone does
    /// not prove that condition. Lock order remains fence -> canonical -> legacy.
    /// No I/O, await, delivery or legacy snapshot reconciliation occurs here.
    pub fn transfer_legacy_creature_ownership(
        legacy: &SharedMapManager,
        canonical: &SharedCanonicalMapManager,
        key: MapKey,
        incarnation: u64,
        _writer_fence: &std::sync::MutexGuard<'_, ()>,
    ) -> Result<CreatureActorTransportSummary, CreatureOwnershipTransferError> {
        let map_id = u16::try_from(key.map_id)
            .map_err(|_| CreatureOwnershipTransferError::InvalidMapId)?;
        let mut canonical = canonical.lock()
            .map_err(|_| CreatureOwnershipTransferError::CanonicalUnavailable)?;
        // Reject a stale/busy target before even borrowing the legacy source.
        if canonical.tick_coordination_like_cpp() != wow_map::MapTickCoordinationStateLikeCpp::Idle {
            return Err(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::MapBusy));
        }
        let actual = canonical.map_incarnation_like_cpp(key)
            .ok_or(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::MissingMap))?;
        if actual != incarnation {
            return Err(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::StaleIncarnation));
        }
        let mut legacy = legacy.write()
            .map_err(|_| CreatureOwnershipTransferError::LegacyUnavailable)?;
        let source = legacy.get_map_mut(map_id, key.instance_id)
            .ok_or(CreatureOwnershipTransferError::Admission(CreatureActorTransportError::MissingLegacyMap))?;
        canonical.transport_legacy_creature_ownership(key, incarnation, source)
            .map_err(CreatureOwnershipTransferError::Admission)
    }
}

#[cfg(test)]
mod tests;
