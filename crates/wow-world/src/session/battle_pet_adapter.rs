// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle pet adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.
//! The outcome types, the stats application and the uncage `DestroyItem`
//! phase moved to `wow_world_inventory::battle_pet_session` (#1363 slice 7b).

#[cfg(test)]
use super::RepresentedBattlePetCageItemLikeCpp;
use super::represented_aura_effect_amounts_like_cpp;
use super::{AuraApplication, Instant, ObjectGuid, RepresentedAuraEffectLikeCpp, WorldSession};

#[cfg(test)]
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(test)]
pub(crate) static NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP: AtomicI64 = AtomicI64::new(1);

#[cfg(test)]
pub(crate) fn next_represented_battle_pet_guid_like_cpp() -> ObjectGuid {
    let counter = NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP.fetch_add(1, Ordering::Relaxed);
    ObjectGuid::create_global(wow_core::guid::HighGuid::BattlePet, 0, counter)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetCageOutcomeLikeCpp {
    Caged(RepresentedBattlePetCageItemLikeCpp),
    NoJournalLock,
    UnknownPet,
    NotTradable,
    InBattleSlot,
    Damaged,
    InventoryUnavailable,
    StoreFailed,
}

impl WorldSession {
    // ── Aura system ───────────────────────────────────────────────

    pub(in crate::session) fn apply_represented_battle_pet_xp_pct_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        let multiplier = 1.0 + (effect.effect_base_points as f32 / 100.0);
        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.core.current_map_difficulty_id_like_cpp(),
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

        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        self.send_aura_update_applied(spell_id, slot, caster_guid, 30_000, 0x0000_0001, 0x1);

        Ok(())
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/battle_pet_adapter/f3_shims.rs"]
mod f3_shims;
