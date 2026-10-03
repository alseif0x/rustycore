//! Aura slot updates and packets published to the client.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(crate) use wow_world_spell::player_aura_info_like_cpp;

impl WorldSession {
    pub(crate) fn insert_player_visible_aura_like_cpp(&mut self, aura: AuraApplication) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.insert_player_visible_aura_like_cpp(&mut hub, aura)
    }
    pub(crate) fn insert_player_visible_aura_with_provenance_like_cpp(
        &mut self,
        aura: AuraApplication,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> bool {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.insert_player_visible_aura_with_provenance_like_cpp(&mut hub, aura, provenance)
    }
    pub(in crate::session) fn next_player_visible_aura_slot_like_cpp(&self) -> Option<u8> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.next_player_visible_aura_slot_like_cpp(hub)
    }
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
    pub(in crate::session) fn send_aura_update_applied(
        &self,
        spell_id: i32,
        slot: u8,
        caster: ObjectGuid,
        duration: u32,
        flags: u32,
        effect_mask: u32,
    ) {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.send_aura_update_applied(hub, spell_id, slot, caster, duration, flags, effect_mask)
    }
    pub(in crate::session) fn update_represented_flight_flags_for_flight_aura_like_cpp(
        &mut self,
        apply: bool,
    ) {
        let should_enable = if apply {
            true
        } else {
            let Some(has_fly) = self
                .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Fly)
            else {
                return;
            };
            let Some(has_mounted_flight_speed) = self
                .resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                )
            else {
                return;
            };
            has_fly || has_mounted_flight_speed
        };
        crate::session::hub_mut(self)
            .set_represented_can_swim_to_fly_transition_like_cpp(should_enable);
        let can_fly_changed =
            crate::session::hub_mut(self).set_represented_can_fly_like_cpp(should_enable);
        if !should_enable && can_fly_changed {
            crate::session::hub_mut(self).move_represented_player_fall_like_cpp();
        }
    }
}


#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_publication/f3_shims.rs"]
mod f3_shims;
