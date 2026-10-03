//! Represented spell cooldown and recovery state.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn mark_represented_character_spell_cooldowns_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mark_represented_character_spell_cooldowns_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn record_loaded_character_spell_cooldown_like_cpp(
        &mut self,
        spell_id: u32,
        item_id: u32,
        cooldown_end_unix_secs: i64,
        category_id: u32,
        category_end_unix_secs: i64,
    ) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.record_loaded_character_spell_cooldown_like_cpp(
            &mut hub,
            spell_id,
            item_id,
            cooldown_end_unix_secs,
            category_id,
            category_end_unix_secs,
        )
    }
    /// Check if a spell is on cooldown (global or per-spell).
    ///
    /// Returns true if either the global cooldown (1500ms) or the spell-specific
    /// cooldown from SpellStore is still active.
    pub fn is_spell_on_cooldown(&self, spell_id: i32) -> bool {
        let Some(last_cast) = self.last_spell_cast_time_like_cpp() else {
            return true;
        };
        let Some(last_cast) = last_cast else {
            return false; // Never casted
        };

        let elapsed_ms = last_cast.elapsed().as_millis() as u32;

        // Global cooldown: 1500ms
        if elapsed_ms < 1500 {
            return true;
        }

        // Per-spell cooldown (if exists in SpellStore)
        if let Some(store) = &self.catalogs.spell_catalogs.spell_store {
            if let Some(spell_info) = store.get(spell_id) {
                if elapsed_ms < spell_info.cooldown_ms {
                    return true;
                }
            }
        }

        false
    }
    /// Login snapshot of spell-history + charge packet entries
    /// (see `record_login_spell_history_packets_like_cpp`).
    #[cfg(test)]
    pub(crate) fn spell_history_packets_like_cpp(
        &self,
    ) -> (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>) {
        self.spell_state.spell_history_packets_for_test_like_cpp()
    }
}
#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/cooldown/f3_shims.rs"]
mod f3_shims;
