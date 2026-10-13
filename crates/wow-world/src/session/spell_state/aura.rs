//! Represented aura state owned at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
use wow_world_spell::shapeshift_form_of_spell_like_cpp;

#[path = "aura/effect_queries.rs"]
mod effect_queries;
#[path = "aura/spell_hit_authority.rs"]
mod spell_hit_authority;

impl WorldSession {
    pub(in crate::session) fn represented_cast_speed_multiplier_like_cpp(&self) -> f32 {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_cast_speed_multiplier_like_cpp(hub)
    }

    pub(crate) fn represented_player_is_polymorphed_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_player_is_polymorphed_like_cpp(hub)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura/f3_shims.rs"]
mod f3_shims;
