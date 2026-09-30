//! Prepared move-only respawn handoff. This is not a producer activation switch.
use super::*;
use wow_map::spawn::RespawnTransferError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnOwnerTransferError {
    InvalidMapId,
    CanonicalUnavailable,
    LegacyUnavailable,
    MissingLegacyMap,
    Admission(RespawnTransferError),
}

impl super::super::WorldSession {
    /// APP must quiesce all producers and previously drained payload borrowers.
    /// Its existing writer-order guard outlives extraction/admission/restoration.
    /// Lock order is fence -> canonical manager -> legacy manager, matching the
    /// canonical projection path. No IO, mailbox submission or await under guards.
    /// Until this prepared operation is activated and legacy writers retired,
    /// there remain TWO production instances of the same tagged store definition.
    pub fn transfer_legacy_respawns_to_canonical(
        legacy: &crate::map_manager::SharedMapManager,
        canonical: &SharedCanonicalMapManager,
        key: wow_map::MapKey,
        incarnation: u64,
        writer_fence: &std::sync::MutexGuard<'_, ()>,
    ) -> Result<(), RespawnOwnerTransferError> {
        let map_id = u16::try_from(key.map_id).map_err(|_| RespawnOwnerTransferError::InvalidMapId)?;
        let mut canonical = canonical.lock().map_err(|_| RespawnOwnerTransferError::CanonicalUnavailable)?;
        if canonical.tick_coordination_like_cpp() != wow_map::MapTickCoordinationStateLikeCpp::Idle {
            return Err(RespawnOwnerTransferError::Admission(RespawnTransferError::MapBusy));
        }
        let Some(current_incarnation) = canonical.map_incarnation_like_cpp(key) else {
            return Err(RespawnOwnerTransferError::Admission(RespawnTransferError::MissingMap));
        };
        if current_incarnation != incarnation {
            return Err(RespawnOwnerTransferError::Admission(RespawnTransferError::StaleIncarnation));
        }
        if canonical.find_map(key.map_id, key.instance_id).is_none() {
            return Err(RespawnOwnerTransferError::Admission(RespawnTransferError::MissingMap));
        }
        let mut legacy = legacy.write().map_err(|_| RespawnOwnerTransferError::LegacyUnavailable)?;
        let source = legacy.get_map_mut(map_id, key.instance_id)
            .ok_or(RespawnOwnerTransferError::MissingLegacyMap)?;
        let transfer = source.take_respawn_transfer(incarnation, writer_fence);
        match canonical.accept_respawn_transfer(transfer) {
            Ok(()) => Ok(()),
            Err((error, transfer)) => {
                // Both exclusive guards are still held and no source operation
                // ran since mem::take: restoration cannot encounter new slots.
                source.restore_respawn_transfer(transfer)
                    .expect("exclusive quiescent source remains empty after rejected transfer");
                Err(RespawnOwnerTransferError::Admission(error))
            }
        }
    }
}

#[cfg(test)]
mod tests;
