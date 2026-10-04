// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{AuraRemovalCxLikeCpp, RemovedAuraLikeCpp};

impl AuraRemovalCxLikeCpp<'_> {
    pub(super) fn publish_aura_removal_like_cpp(&self, slot: u8) {
        self.spell.send_aura_update_removed_with_publication_like_cpp(
            self.player.player_guid_like_cpp(),
            &self.player.packet_publication_like_cpp(), slot,
        );
    }

    pub(super) fn update_total_stat_percentage_phase_like_cpp(
        &mut self,
        removed: &RemovedAuraLikeCpp,
        spell_store: Option<&wow_data::SpellStore>,
    ) {
        if self.player.is_logged_in_like_cpp()
            && self.spell.aura_has_total_stat_percentage_effect_with_store_like_cpp(
                spell_store, &removed.aura,
            )
        {
            let preserve_health_pct = self.spell
                .total_stat_percentage_aura_preserves_health_pct_with_store_like_cpp(
                    spell_store, &removed.aura,
                );
            let player = self.stats.reborrow_like_cpp(
                &self.player,
                #[cfg(any(test, feature = "test-fixtures"))] &*self.shapeshift_form,
            );
            let mut stats = crate::CharacterStatsApplicationCxLikeCpp::new(
                player, &*self.inventory, self.player.packet_publication_like_cpp(),
            );
            stats.send_total_stat_percentage_update_like_cpp(preserve_health_pct);
        }
    }

    pub(super) fn sync_attack_speed_phase_like_cpp(&self, spell_store: Option<&wow_data::SpellStore>) {
        self.spell.sync_attack_speed_with_access_like_cpp(&self.player, spell_store);
    }
}
