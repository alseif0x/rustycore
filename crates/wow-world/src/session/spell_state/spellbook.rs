//! Known spells, learning and unlearning at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The login proficiency replay moved to
//! `wow_world_spell` (#1363 slice 12b3).

use super::*;

impl WorldSession {
    pub fn set_cast_unstuck_enabled_like_cpp(&mut self, enabled: bool) {
        self.config.represented_cast_unstuck_enabled_like_cpp = enabled;
    }

    pub fn set_offhand_check_at_spell_unlearn_like_cpp(&mut self, enabled: bool) {
        self.spell_state
            .set_offhand_check_at_spell_unlearn_like_cpp(enabled);
    }
}
