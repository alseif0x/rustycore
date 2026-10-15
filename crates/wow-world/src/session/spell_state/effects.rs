//! Represented spell effect application seen from the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn apply_represented_login_spell_reset_if_needed_like_cpp(&mut self) -> bool {
        const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

        if !self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_RESET_SPELLS_LIKE_CPP) != 0)
        {
            return false;
        }

        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_SPELLS_LIKE_CPP, true);
        let spells = wow_world_spell::known_spells_like_cpp(self).to_vec();
        for spell_id in spells {
            wow_world_spell::remove_known_spell_like_cpp(self, spell_id);
        }
        self.apply_represented_quest_rewarded_spells_like_cpp();
        self.core
            .send_notification_like_cpp(self.reset_spells_notification_text_like_cpp());
        true
    }
}
