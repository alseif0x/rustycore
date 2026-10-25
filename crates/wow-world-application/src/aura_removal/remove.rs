// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;

impl AuraRemovalCxLikeCpp<'_> {
    pub fn remove_aura_like_cpp(&mut self, slot: u8) -> Result<(), &'static str> {
        let removal_target = self
            .player
            .aura_subsystem_snapshot_like_cpp()
            .and_then(|auras| auras.runtime_application_like_cpp(slot).cloned());
        let selected_sickness = removal_target
            .as_ref()
            .is_some_and(|aura| self.is_current_race_sickness_spell(aura.spell_id) == Some(true));
        if selected_sickness {
            let aura = removal_target
                .as_ref()
                .expect("selected sickness came from the removal target");
            let spell_info = self
                .spell_store
                .and_then(|store| store.get(aura.spell_id))
                .ok_or("Missing SpellInfo for the race-selected sickness removal")?;
            super::sickness::validate_selected_sickness_data(
                aura.spell_id,
                spell_info,
                aura.effect_mask,
            )?;
            self.preflight_selected_sickness_stats()?;
        }
        let prepared_ghost_wisp = self.prepare_ghost_wisp_removal(slot)?;
        let removed = super::initial::remove_initial_phase_like_cpp(
            self.spell,
            &mut self.player,
            self.spell_store.map(|store| store.as_ref()),
            slot,
        )?;
        let selected_update = matches!(removed.aura.spell_id, 8_326 | 20_584) || selected_sickness;
        if selected_update {
            // C++ AuraApplication::_Remove publishes AuraUpdate before
            // Unit::_UnapplyAura applies its effect-removal handlers. The
            // race catalog selected the sickness spell before removal.
            self.publish_aura_removal_like_cpp(&removed, selected_update);
        }
        super::threat::remove_threat_phase_like_cpp(&mut self.player, &removed, self.consumer_test);
        self.remove_ghost_wisp_effects(&removed, prepared_ghost_wisp)?;
        self.remove_mount_control_phase_like_cpp(&removed);
        self.remove_movement_speed_phase_like_cpp(&removed);
        if !selected_update {
            self.publish_aura_removal_like_cpp(&removed, false);
        }
        if removed.aura.spell_id == wow_world_core::session::SPELL_PVP_RULES_ENABLED_LIKE_CPP {
            let _ = self.item_scaling_phase_like_cpp();
        }
        self.update_character_stats_phase(&removed, self.spell_store.map(|store| store.as_ref()))?;
        self.sync_attack_speed_phase_like_cpp(self.spell_store.map(|store| store.as_ref()));
        if let Some(mutation) = self.shapeshift_ownership_phase_like_cpp(removed.aura.spell_id) {
            self.sync_shapeshift_form_like_cpp(mutation);
        }
        self.display_power_phase_like_cpp(removed.aura.spell_id);
        Ok(())
    }
}
