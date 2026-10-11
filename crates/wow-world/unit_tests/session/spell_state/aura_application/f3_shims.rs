// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.catalogs
            .same_effect_stack_rule_aura_types_like_cpp(group_id)
    }
}

impl WorldSession {
    pub(in crate::session) fn remove_player_visible_aura_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<AuraApplication> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.remove_player_visible_aura_like_cpp(&mut hub, slot)
    }
    /// Apply an aura to the player and send SMSG_AURA_UPDATE.
    pub(crate) fn apply_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            0x0000_0001,
        )
    }
    pub(crate) fn remove_represented_mod_speed_no_control_auras_cancelable_like_cpp(
        &mut self,
    ) -> usize {
        self.remove_represented_cancelable_auras_by_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::ModSpeedNoControl,
        )
    }
    pub(in crate::session) fn remove_represented_cancelable_auras_by_effect_like_cpp(
        &mut self,
        represented_effect: RepresentedAuraEffectLikeCpp,
    ) -> usize {
        let (spell_state, inventory, loot, quest_state, hub) =
            crate::session::state::split_aura_application_mut(self);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = quest_state;
        wow_world_application::remove_represented_cancelable_auras_by_effect_like_cpp(
            hub,
            spell_state,
            inventory,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            quest_state,
            cfg!(test),
            represented_effect,
        )
    }
}
