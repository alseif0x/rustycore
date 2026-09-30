// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical represented absorb-pool commits.
//!
//! C++ AuraEffect::ChangeAmount (SpellAuraEffects.cpp:923-969) is called by
//! Unit::CalcAbsorbResist (Unit.cpp:1852-1859, 1931-1937) and
//! Unit::CalcHealAbsorb (Unit.cpp:2059-2066). These represented storage writes
//! retain the existing clamps and removal semantics; they do not implement
//! ChangeAmount's script/reapply or client-update handling. Calculation,
//! health/mana writes, and publication remain with the application owner.

use super::{AppliedAuraRef, AuraSubsystem, RepresentedAuraEffectAmountLikeCpp};

impl AuraSubsystem {
    /// Commit the remaining runtime shield pool, replacing only the first
    /// represented effect match or appending at the end when it is absent.
    /// The existing runtime mutation accessor retains authority invalidation,
    /// even when the clamped amount is unchanged.
    pub fn set_runtime_absorb_amount(&mut self, slot: u8, effect_index: u8, remaining: i32) {
        let Some(aura) = self.runtime_application_mut_like_cpp(slot) else {
            return;
        };
        match aura
            .represented_effect_amounts
            .iter_mut()
            .find(|represented| represented.effect_index == effect_index)
        {
            Some(represented) => represented.amount = remaining.max(0),
            None => aura
                .represented_effect_amounts
                .push(RepresentedAuraEffectAmountLikeCpp {
                    effect_index,
                    amount: remaining.max(0),
                }),
        }
    }

    /// Preserve the first applied reference in collection order whose slot
    /// and range-checked effect bit match the consuming shield.
    pub fn find_applied_absorb_effect(&self, slot: u8, effect_index: u8) -> Option<AppliedAuraRef> {
        self.applied_auras
            .iter()
            .find(|aura| {
                aura.slot == slot
                    && 1_u32
                        .checked_shl(u32::from(effect_index))
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            })
            .copied()
    }

    /// Commit a caller-calculated remainder. Removal is supplied by the
    /// calculation owner, never inferred from the remainder here.
    ///
    /// Exhaustion retains the existing spell/caster grouping, copied work
    /// list, unapply marker zero, and clearing of only the original slot.
    /// A surviving shield updates only an existing amount entry: the
    /// established missing-entry fallback is not repaired by inserting one.
    pub fn commit_applied_absorb_amount(
        &mut self,
        applied: AppliedAuraRef,
        remaining: i32,
        removed: bool,
    ) {
        if removed {
            let aura_ref = applied.aura_ref();
            let covered: Vec<_> = self
                .applied_auras
                .iter()
                .filter(|candidate| candidate.aura_ref() == aura_ref)
                .copied()
                .collect();
            for covered in covered {
                self.unapply_aura(covered, 0);
            }
            let _ = self.clear_visible(applied.slot);
        } else if let Some(amount) = self.applied_aura_amounts.get_mut(&applied) {
            *amount = remaining.max(0);
        }
    }
}
