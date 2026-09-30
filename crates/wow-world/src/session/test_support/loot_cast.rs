//! Borrowed observations of the canonical cast and aura owners for loot fixtures.
use super::*;

impl WorldSession {
    pub(crate) fn loot_cast_pending(&self) -> bool {
        self.with_owned_player_like_cpp(|player| player.unit().subsystems().spells.execution.active.is_some()).unwrap_or(false)
    }

    pub(crate) fn loot_aura_slot_present(&self, slot: u8) -> bool {
        self.with_owned_player_like_cpp(|player| player.unit().subsystems().auras.runtime_applications_like_cpp().contains_key(&slot)).unwrap_or(false)
    }
}
