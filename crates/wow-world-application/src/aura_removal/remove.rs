// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;

impl AuraRemovalCxLikeCpp<'_> {
    pub fn remove_aura_like_cpp(&mut self, slot: u8) -> Result<(), &'static str> {
        let removed = super::initial::remove_initial_phase_like_cpp(
            self.spell,
            &mut self.player,
            self.spell_store.map(|store| store.as_ref()),
            slot,
        )?;
        super::threat::remove_threat_phase_like_cpp(&mut self.player, &removed, self.consumer_test);
        self.remove_mount_control_phase_like_cpp(&removed);
        self.remove_movement_speed_phase_like_cpp(&removed);
        self.publish_aura_removal_like_cpp(slot);
        if removed.aura.spell_id == wow_world_core::session::SPELL_PVP_RULES_ENABLED_LIKE_CPP {
            let _ = self.item_scaling_phase_like_cpp();
        }
        self.update_total_stat_percentage_phase_like_cpp(
            &removed,
            self.spell_store.map(|store| store.as_ref()),
        );
        self.sync_attack_speed_phase_like_cpp(self.spell_store.map(|store| store.as_ref()));
        if let Some(mutation) = self.shapeshift_ownership_phase_like_cpp(removed.aura.spell_id) {
            self.sync_shapeshift_form_like_cpp(mutation);
        }
        self.display_power_phase_like_cpp(removed.aura.spell_id);
        Ok(())
    }
}
