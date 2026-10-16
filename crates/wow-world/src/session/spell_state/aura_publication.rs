//! Aura slot updates and packets published to the client.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The zone-script and area-PvP
//! source queries moved to `wow_world_spell` (#1363 slice 12b3).

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_war_mode_update_zone_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        wow_world_entities::represented_player_flags_value_like_cpp(self).is_some()
            && !wow_world_entities::represented_player_has_flag_like_cpp(
                self,
                PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP,
            )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_publication/f3_shims.rs"]
mod f3_shims;
