// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;

impl AuraRemovalCxLikeCpp<'_> {
    pub fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool {
        let has_died_state = self.player.has_canonical_died_state_like_cpp().unwrap_or(false);
        if !has_died_state { return false; }
        let Some(visible_auras) = self.player.visible_auras_snapshot_like_cpp() else { return false; };
        let slots: Vec<u8> = visible_auras.iter().filter_map(|(slot, aura)| {
            (aura.represented_effect == Some(wow_entities::RepresentedAuraEffectLikeCpp::FeignDeath))
                .then_some(*slot)
        }).collect();
        if slots.is_empty() { return false; }
        for slot in slots {
            let _ = self.remove_aura_like_cpp(slot);
        }
        let _ = self.player.clear_canonical_died_state_like_cpp();
        true
    }
}
