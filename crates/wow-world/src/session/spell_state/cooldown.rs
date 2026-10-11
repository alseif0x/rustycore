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
}
#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/cooldown/f3_shims.rs"]
mod f3_shims;
