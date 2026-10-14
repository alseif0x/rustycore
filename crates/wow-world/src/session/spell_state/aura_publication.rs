//! Aura slot updates and packets published to the client.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_war_mode_update_zone_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        self.represented_player_flags_value_like_cpp().is_some()
            && !self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP)
    }
    pub(in crate::session) fn represented_update_zone_script_aura_source_is_hit_inert_like_cpp(
        &self,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_update_zone_script_aura_source_is_hit_inert_like_cpp(hub)
    }
    pub(in crate::session) fn represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(hub)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_publication/f3_shims.rs"]
mod f3_shims;
