// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selection and admission for represented runtime Aura removal.
//!
//! Target reference a5f8da2e: src/server/game/Handlers/SpellHandler.cpp:272-354,
//! Entities/Unit/Unit.cpp:3604-3615,3965,4076-4108,2836-2864 and
//! Spells/Auras/SpellAuras.cpp:778-845. The application retains its snapshots,
//! removal phases, logs and publication. Positivity, remove-mode, interruption
//! exclusions/restarts and wall-clock versus C++ diff remain represented gaps.

use std::collections::HashMap;
use super::{AuraApplicationLikeCpp, AuraSubsystem, Instant, ObjectGuid, RepresentedAuraEffectLikeCpp};

impl AuraSubsystem {
    pub fn runtime_slots_for_spell(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        spell_id: i32,
    ) -> Vec<u8> {
        auras.values()
            .filter_map(|aura| (aura.spell_id == spell_id).then_some(aura.slot))
            .collect()
    }

    pub fn runtime_slots_with_attribute(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        attribute: u32,
        mut has_attribute: impl FnMut(i32, u32) -> bool,
    ) -> Vec<u8> {
        auras.values()
            .filter_map(|aura| has_attribute(aura.spell_id, attribute).then_some(aura.slot))
            .collect()
    }

    pub fn runtime_cancelable_slots_for_effect(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        represented_effect: RepresentedAuraEffectLikeCpp,
        mut no_aura_cancel: impl FnMut(i32) -> bool,
    ) -> Vec<u8> {
        auras.values()
            .filter_map(|aura| {
                // C++ removes SPELL_AURA_MOUNTED only when its SpellInfo is
                // cancelable, positive, and non-passive; the same predicate is
                // used for SPELL_AURA_MOD_SCALE in CancelGrowthAura. These
                // represented effects model positive player-cancelable paths;
                // SpellMisc attributes preserve the C++ no-player-cancel gate.
                if no_aura_cancel(aura.spell_id) {
                    return None;
                }
                (aura.represented_effect == Some(represented_effect)).then_some(aura.slot)
            })
            .collect()
    }

    /// Each catalog operation remains lazy and distinct. The application
    /// invokes this gate before obtaining the runtime Aura snapshot.
    pub fn owned_spell_is_cancelable(
        spell_id: i32,
        exists: impl FnOnce(i32) -> bool,
        no_aura_cancel: impl FnOnce(i32) -> bool,
        channeled: impl FnOnce(i32) -> bool,
        passive: impl FnOnce(i32) -> bool,
    ) -> bool {
        exists(spell_id)
            && !no_aura_cancel(spell_id)
            && !channeled(spell_id)
            && !passive(spell_id)
    }

    pub fn runtime_cancelable_owned_slots(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        spell_id: i32,
        caster_guid: ObjectGuid,
    ) -> Vec<u8> {
        auras.values()
            .filter_map(|aura| {
                if aura.spell_id != spell_id {
                    return None;
                }
                if !caster_guid.is_empty() && aura.caster_guid != caster_guid {
                    return None;
                }
                // C++ checks SpellInfo before RemoveOwnedAura: no
                // SPELL_ATTR0_NO_AURA_CANCEL, positive, and non-passive.
                // Full SpellInfo::IsPositive is not represented yet; allow
                // the locally materialized positive/cancelable aura shapes,
                // including the single-effect generic represented aura.
                (aura.represented_effect.is_none()
                    || matches!(
                        aura.represented_effect,
                        Some(
                            RepresentedAuraEffectLikeCpp::Mounted
                                | RepresentedAuraEffectLikeCpp::ModScale
                                | RepresentedAuraEffectLikeCpp::ModSpeedNoControl
                        )
                    ))
                .then_some(aura.slot)
            })
            .collect()
    }

    pub fn runtime_slots_with_interrupt_flags(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        flags: u32,
        flags2: u32,
    ) -> Vec<u8> {
        auras.values()
            .filter(|aura| {
                (flags != 0 && aura.aura_interrupt_flags & flags != 0)
                    || (flags2 != 0 && aura.aura_interrupt_flags2 & flags2 != 0)
            })
            .map(|aura| aura.slot)
            .collect()
    }

    pub fn expired_runtime_slots(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
        mut elapsed_millis: impl FnMut(&Instant) -> u128,
    ) -> Vec<u8> {
        auras.values()
            .filter(|aura| {
                // Permanent auras (duration_total == 0) never expire
                aura.duration_total > 0
                    && elapsed_millis(&aura.applied_at) as u32 >= aura.duration_total
            })
            .map(|aura| aura.slot)
            .collect()
    }

    pub fn runtime_stealth_or_invisibility_slots(
        auras: &HashMap<u8, AuraApplicationLikeCpp>,
    ) -> Vec<u8> {
        auras.iter()
            .filter_map(|(slot, aura)| {
                matches!(
                    aura.represented_effect,
                    Some(RepresentedAuraEffectLikeCpp::Stealth)
                        | Some(RepresentedAuraEffectLikeCpp::Invisibility)
                )
                .then_some(*slot)
            })
            .collect()
    }
}

