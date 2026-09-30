//! First occurrence loot preparation/allocation/install on the original ROOT slot.
//! No hook receipt, progress, implicit disposal, new reservation or runtime activation.
use super::*;
use crate::MapKey;
use crate::map::MapGuidSequenceErrorLikeCpp;
use std::collections::HashMap;
use wow_core::guid::HighGuid;
use wow_entities::{CreatureLoot, OwnedLootAuthority};

#[derive(Debug)]
pub enum MeleeLootError {
    Phase(MeleeKillPhaseError),
    Counter(MapGuidSequenceErrorLikeCpp),
    LifetimeConflict { captured: u64, current: u64 },
    InstallRejected,
    ManagerUnavailable,
    GuidEncoding,
    GuidUnavailable,
}

/// Owns the exact original ledger while APP performs generation without a Map guard.
pub struct PreparedMeleeLoot {
    pending: PendingMeleeKills,
}
impl PreparedMeleeLoot {
    pub fn pending(&self) -> &PendingMeleeKills {
        &self.pending
    }
    pub fn source(&self) -> &CreatureLootObservation {
        &self.pending.first_capture().source
    }
    pub fn target_guid(&self) -> ObjectGuid {
        self.pending.first_capture().target_guid
    }
    pub fn authority(&self) -> &OwnedLootAuthority {
        &self.pending.first_capture().authority
    }
    pub fn object_generation(&self) -> u64 {
        self.pending.first_capture().object_generation
    }
    pub fn into_pending(self) -> PendingMeleeKills {
        self.pending
    }
}

impl MapManager {
    pub fn prepare_melee_loot(
        &self,
        tick: &MapObjectTickContinuation,
        prepared: PreparedMeleeKill,
    ) -> Result<PreparedMeleeLoot, (MeleeLootError, PendingMeleeKills)> {
        let mut pending = prepared.into_pending();
        match self.validate_current_melee_loot(tick, &mut pending) {
            Ok(()) => Ok(PreparedMeleeLoot { pending }),
            Err(error) => Err((error, pending)),
        }
    }

    fn validate_current_melee_loot(
        &self,
        tick: &MapObjectTickContinuation,
        pending: &mut PendingMeleeKills,
    ) -> Result<(), MeleeLootError> {
        self.validate_current_melee_kill(tick, pending)
            .map_err(MeleeLootError::Phase)?;
        let captured = pending.first_capture();
        let actor = self
            .maps
            .get(&pending.token.key())
            .expect("current ROOT retains Map")
            .map()
            .creature_actor(captured.target_guid)
            .expect("same Map retains validated TARGET");
        let current = actor.creature.loot_lifecycle_revision_like_cpp();
        let lifetime = captured.source.loot_lifecycle_revision();
        if current != lifetime {
            return Err(MeleeLootError::LifetimeConflict {
                captured: lifetime,
                current,
            });
        }
        Ok(())
    }

    /// Called at each original pool GUID point, AFTER its money RNG/item IO.
    pub fn next_melee_loot_counter(
        &mut self,
        tick: &MapObjectTickContinuation,
        operation: &mut PreparedMeleeLoot,
    ) -> Result<(MapKey, i64), MeleeLootError> {
        self.validate_current_melee_loot(tick, &mut operation.pending)?;
        let key = operation.pending.token.key();
        let counter = self
            .maps
            .get_mut(&key)
            .expect("current ROOT retains Map")
            .map_mut()
            .generate_low_guid_like_cpp(HighGuid::LootObject)
            .map_err(MeleeLootError::Counter)?;
        Ok((key, counter))
    }

    /// The existing installer consumes pools on rejection. The original token,
    /// entire outcome/batch and cursor remain in the caller's operation.
    pub fn install_melee_loot(
        &mut self,
        tick: &MapObjectTickContinuation,
        operation: &mut PreparedMeleeLoot,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> Result<(), MeleeLootError> {
        self.validate_current_melee_loot(tick, &mut operation.pending)?;
        let captured = operation.pending.first_capture();
        let installed = self
            .maps
            .get_mut(&operation.pending.token.key())
            .expect("current ROOT retains Map")
            .map_mut()
            .creature_actor_mut(captured.target_guid)
            .expect("same Map retains validated TARGET")
            .install_kill_loot(
                &captured.authority,
                captured.object_generation,
                captured.source.loot_lifecycle_revision(),
                shared,
                personal,
            );
        if installed {
            Ok(())
        } else {
            Err(MeleeLootError::InstallRejected)
        }
    }
}

#[cfg(test)]
mod tests;
