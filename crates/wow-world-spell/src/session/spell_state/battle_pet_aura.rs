// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented `SPELL_AURA_MOD_BATTLE_PET_XP_PCT` aura application.
//! Moved from the World `session/battle_pet_adapter.rs` (#1363 slice 7c); it
//! uses only this state's visible-aura slot, insert and publication.

use crate::SessionSpellState;
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectAmountLikeCpp,
    RepresentedAuraEffectLikeCpp,
};
use wow_world_core::session::HubMut;

pub fn represented_aura_effect_amounts_like_cpp(
    effect: &wow_data::SpellEffectInfo,
) -> Vec<RepresentedAuraEffectAmountLikeCpp> {
    let Some(effect_index) = u8::try_from(effect.effect_index).ok() else {
        return Vec::new();
    };
    vec![RepresentedAuraEffectAmountLikeCpp {
        effect_index,
        amount: effect.effect_base_points,
    }]
}

impl SessionSpellState {
    pub fn apply_represented_battle_pet_xp_pct_aura_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let slot = self
            .next_player_visible_aura_slot_like_cpp(hub.shared())
            .ok_or("No free aura slots or missing Player aura owner")?;

        let multiplier = 1.0 + (effect.effect_base_points as f32 / 100.0);
        let aura = AuraApplication {
            spell_id,
            difficulty_id: hub.core.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::ModBattlePetXpPct),
            represented_amount: effect.effect_base_points,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: None,
            represented_multiplier: multiplier,
            applied_at: Instant::now(),
        };

        if !self.insert_player_visible_aura_like_cpp(hub, aura) {
            return Err("Missing Player aura owner");
        }
        self.send_aura_update_applied(
            hub.shared(),
            spell_id,
            slot,
            caster_guid,
            30_000,
            0x0000_0001,
            0x1,
        );

        Ok(())
    }
}
