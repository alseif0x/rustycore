//! Quest giver menus, status and offer details seen from the Session.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn invalidate_player_quest_status_authority_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.set_status_authority_complete_like_cpp(false);
        });
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/quest/giver/f3_shims.rs"]
mod f3_shims;
