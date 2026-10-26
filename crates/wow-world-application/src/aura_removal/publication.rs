// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::stat_application::select_aura_stat_application;
use super::{AuraRemovalCxLikeCpp, RemovedAuraLikeCpp};

impl AuraRemovalCxLikeCpp<'_> {
    pub(super) fn publish_aura_removal_like_cpp(
        &self,
        removed: &RemovedAuraLikeCpp,
        selected_update: bool,
    ) {
        self.spell.send_aura_removal_for_spell(
            self.player.player_guid_like_cpp(),
            &self.player.packet_publication_like_cpp(),
            removed.aura.slot,
            selected_update,
        );
    }

    pub(super) fn update_character_stats_phase(
        &mut self,
        removed: &RemovedAuraLikeCpp,
        spell_store: Option<&wow_data::SpellStore>,
    ) -> Result<(), &'static str> {
        if !self.player.is_logged_in_like_cpp() {
            return Ok(());
        }

        let required_sickness =
            self.is_current_race_sickness_spell(removed.aura.spell_id) == Some(true);
        let selection = if required_sickness {
            super::stat_application::select_selected_aura_stat_application(
                spell_store,
                removed.aura.spell_id,
                removed.aura.difficulty_id,
                self.difficulty_store,
                removed.aura.effect_mask,
            )?
        } else {
            select_aura_stat_application(
                spell_store,
                removed.aura.spell_id,
                removed.aura.effect_mask,
                false,
            )?
        };
        let Some(selection) = selection else {
            return Ok(());
        };
        let Some(cause) = selection.cause else {
            return Ok(());
        };
        let preserve_health_pct = matches!(
            cause,
            wow_world_core::session::PlayerStatApplicationCause::StatAura {
                preserve_health_pct: true,
                ..
            }
        );
        let use_values_application = self.stat_application_uses_values(required_sickness);
        let player = self.stats.reborrow_like_cpp(
            &self.player,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.shapeshift_form,
        );
        let mut stats = wow_world_inventory::CharacterStatsApplicationCxLikeCpp::new(
            player,
            &*self.inventory,
            self.player.packet_publication_like_cpp(),
        );
        if use_values_application {
            if stats.apply_stats_for_values(cause).is_none() {
                #[cfg(any(test, feature = "test-fixtures"))]
                if self.player.player_handle_absent_like_cpp() {
                    stats.send_total_stat_percentage_update_like_cpp(preserve_health_pct);
                    return Ok(());
                }
                return Err("Unable to apply calculated Player stats after aura removal");
            }
        } else {
            // Unrelated auras on an Alive Player keep the existing immediate
            // stat packet instead of opting into global VALUES publication.
            stats.send_total_stat_percentage_update_like_cpp(preserve_health_pct);
        }
        Ok(())
    }

    pub(super) fn sync_attack_speed_phase_like_cpp(
        &self,
        spell_store: Option<&wow_data::SpellStore>,
    ) {
        self.spell
            .sync_attack_speed_with_access_like_cpp(&self.player, spell_store);
    }
}
