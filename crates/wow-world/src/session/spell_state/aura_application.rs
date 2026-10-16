//! Applying, refreshing, stacking and expiring represented auras.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn player_aura_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_application::PlayerAuraApplicationCxLikeCpp<'_> {
        let (spell_state, inventory, loot, quest_state, hub) =
            crate::session::state::split_aura_application_mut(self);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest_state;
        wow_world_application::player_aura_application_cx_like_cpp(
            hub,
            spell_state,
            inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            cfg!(test),
        )
    }

    #[cfg(test)]
    pub(in crate::session) fn apply_aura_with_effect_mask_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        // The admitted application aura owner holds this transition; the World
        // shell only builds the borrowed context (#1263 F6).
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
            )
    }
    pub(in crate::session) fn apply_aura_with_effect_mask_and_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            provenance,
            true,
        )
    }
    /// Test-only entry point for a represented multi-effect application, needed
    /// by acceptance cases (for example the transform/`IsPolymorphed` path)
    /// that must mark more than the first effect active.
    #[cfg(test)]
    pub(crate) fn apply_aura_with_effect_mask_for_test_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            0x0000_0001,
            effect_mask,
        )
    }
    fn apply_aura_with_effect_mask_provenance_and_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
        send_update: bool,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_provenance_and_update_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
                provenance,
                send_update,
            )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_application/f3_shims.rs"]
mod f3_shims;
