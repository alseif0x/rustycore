//! Canonical Player spell-hit aura-source authority and synchronization.
//! The authority-complete setter and the glyph and add-to-map source queries
//! moved to `wow_world_spell` (#1363 slice 12b3); the proof and its
//! publication to `wow_world_application::ContinueLoginHostLikeCpp` (#1363
//! final cleanup).

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_aura_subsystem_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::AuraSubsystem) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mutate_player_aura_subsystem_like_cpp(&mut hub, mutate)
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/session/spell_state/aura/spell_hit_authority/f3_shims.rs"]
mod f3_shims;
