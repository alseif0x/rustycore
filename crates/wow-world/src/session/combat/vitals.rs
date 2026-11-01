//! Represented health and power changes and their regeneration.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Installs the process-owned DB2 `PowerType` catalog. C++
    /// `sDB2Manager.GetPowerTypeEntry(power)` (`Unit.cpp:6581-6585`); the
    /// spawn/regeneration paths read the same store through
    /// `CreatureSpawnCatalogsLikeCpp`, so this slot is the session's read-only
    /// handle for the spell-effect chain.
    pub fn set_power_type_store(&mut self, store: Arc<PowerTypeStore>) {
        self.catalogs.power_type_store = Some(store);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/vitals/f3_shims.rs"]
mod f3_shims;
