// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Runtime Aura construction; catalog selection and commit are distinct phases.
//!
//! C++ Aura::BuildEffectMaskForOwner/TryCreate (SpellAuras.cpp:314-395),
//! AuraApplication::_InitFlags (69-154), and AuraEffect::CalculateAmount
//! (SpellAuraEffects.cpp:659). Existing raw/calculated amount distinctions,
//! simplified flags and unchecked single-effect shifts remain represented gaps.
//! Target reference a5f8da2e: src/server/game/Spells/Auras/SpellAuras.cpp
//! and SpellAuraEffects.cpp. This relocation does not claim full Aura execution.

use super::{
    AuraApplicationLikeCpp, AuraSubsystem, Instant, ObjectGuid, RepresentedAuraEffectAmountLikeCpp,
    RepresentedAuraEffectLikeCpp,
};

impl AuraSubsystem {
    pub fn install_runtime_application_with_provenance(
        &mut self,
        aura: AuraApplicationLikeCpp,
        provenance: super::AuraCastProvenanceLikeCpp,
    ) {
        let slot = aura.slot;
        self.insert_runtime_application_like_cpp(aura);
        self.set_aura_cast_provenance_like_cpp(slot, provenance);
    }

    pub fn single_effect_amounts(
        effect_index: u32,
        base_points: i32,
    ) -> Vec<RepresentedAuraEffectAmountLikeCpp> {
        let Some(effect_index) = u8::try_from(effect_index).ok() else {
            return Vec::new();
        };
        vec![RepresentedAuraEffectAmountLikeCpp {
            effect_index,
            amount: base_points,
        }]
    }
    /// Select and calculate before A0, then form the canonical record.
    /// The only amount-vector clone is retained at its original field point.
    /// Difficulty is evaluated before that clone; the clock is evaluated last.
    pub fn build_stat_runtime_application<'catalog, E: 'catalog>(
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        select: impl FnOnce(i32) -> Option<&'catalog [E]>,
        fields: impl Fn(&E) -> (u32, i32, i32, i32),
        mut base_amount: impl FnMut(&E) -> i32,
        is_ability: impl FnOnce(i32) -> bool,
        difficulty: impl FnOnce() -> u8,
        now: impl FnOnce() -> Instant,
    ) -> (
        AuraApplicationLikeCpp,
        Vec<RepresentedAuraEffectAmountLikeCpp>,
        bool,
        bool,
    ) {
        // Preserve the represented StatSystem-relevant multiplier on the same
        // AuraApplication. C++ AuraEffect::HandleModTotalPercentStat uses
        // MiscValueB as a per-stat bitmask (zero means all stats), while the
        // generic AuraApplication continues to own the visible slot.
        let total_stat_percentage_effects: Vec<_> = select(spell_id)
            .map(|effects| {
                effects.iter()
                    .filter(|effect| {
                        1u32.checked_shl(fields(effect).0)
                            .is_some_and(|bit| effect_mask & bit != 0)
                            && fields(effect).1
                                == wow_constants::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                    })
                    .map(|effect| (
                        fields(effect).0, base_amount(effect), fields(effect).2, fields(effect).3,
                    ))
                    .collect()
            })
            .unwrap_or_default();
        let is_ability = is_ability(spell_id);
        let modifies_total_stats = !total_stat_percentage_effects.is_empty();
        let preserve_health_pct = is_ability
            && total_stat_percentage_effects
                .iter()
                .any(|(_, _, _, stat_mask)| *stat_mask == 0 || *stat_mask & (1 << 2) != 0);
        let first_total_stat_percentage = total_stat_percentage_effects.first().copied();
        let (
            represented_effect,
            represented_amount,
            represented_misc_value,
            represented_multiplier,
        ) = if let Some((_, amount, _, stat_mask)) = first_total_stat_percentage {
            (
                Some(RepresentedAuraEffectLikeCpp::ModTotalStatPercentage),
                amount,
                Some(stat_mask),
                1.0 + amount as f32 / 100.0,
            )
        } else {
            (None, 0, None, 1.0)
        };
        let represented_effect_amounts: Vec<_> = total_stat_percentage_effects
            .iter()
            .filter_map(|(effect_index, amount, _, _)| {
                u8::try_from(*effect_index).ok().map(|effect_index| {
                    RepresentedAuraEffectAmountLikeCpp {
                        effect_index,
                        amount: *amount,
                    }
                })
            })
            .collect();
        let aura = AuraApplicationLikeCpp {
            spell_id,
            difficulty_id: difficulty(),
            caster_guid,
            slot,
            duration_total: duration_ms,
            duration_remaining: duration_ms,
            stack_count: 1,
            aura_flags,
            effect_mask,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect,
            represented_amount,
            represented_effect_amounts: represented_effect_amounts.clone(),
            represented_misc_value,
            represented_multiplier,
            applied_at: now(),
        };
        (
            aura,
            represented_effect_amounts,
            modifies_total_stats,
            preserve_health_pct,
        )
    }

    pub fn build_focus_runtime_application<E>(
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect: &E,
        fields: impl Fn(&E) -> (u32, i32, i32),
        difficulty: impl FnOnce() -> u8,
        now: impl FnOnce() -> Instant,
    ) -> AuraApplicationLikeCpp {
        AuraApplicationLikeCpp {
            spell_id,
            difficulty_id: difficulty(),
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << fields(effect).0,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::ProvideSpellFocus),
            represented_amount: fields(effect).1,
            represented_effect_amounts: Self::single_effect_amounts(
                fields(effect).0,
                fields(effect).1,
            ),
            represented_misc_value: Some(fields(effect).2),
            represented_multiplier: 1.0,
            applied_at: now(),
        }
    }

    pub fn build_modifier_runtime_application<E>(
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect: &E,
        represented_effect: RepresentedAuraEffectLikeCpp,
        duration_ms: u32,
        fields: impl Fn(&E) -> (u32, i32, i32),
        difficulty: impl FnOnce() -> u8,
        now: impl FnOnce() -> Instant,
    ) -> AuraApplicationLikeCpp {
        AuraApplicationLikeCpp {
            spell_id,
            difficulty_id: difficulty(),
            caster_guid,
            slot,
            duration_total: duration_ms,
            duration_remaining: duration_ms,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << fields(effect).0,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(represented_effect),
            represented_amount: fields(effect).1,
            represented_effect_amounts: Self::single_effect_amounts(
                fields(effect).0,
                fields(effect).1,
            ),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: now(),
        }
    }

    pub fn build_mounted_runtime_application<E>(
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect: &E,
        mounted_amount: i32,
        fields: impl Fn(&E) -> (u32, i32, i32),
        difficulty: impl FnOnce() -> u8,
        now: impl FnOnce() -> Instant,
    ) -> AuraApplicationLikeCpp {
        AuraApplicationLikeCpp {
            spell_id,
            difficulty_id: difficulty(),
            caster_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << fields(effect).0,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::Mounted),
            represented_amount: mounted_amount,
            represented_effect_amounts: Self::single_effect_amounts(
                fields(effect).0,
                fields(effect).1,
            ),
            represented_misc_value: Some(fields(effect).2),
            represented_multiplier: 1.0,
            applied_at: now(),
        }
    }

    pub fn build_xp_runtime_application<E>(
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect: &E,
        fields: impl Fn(&E) -> (u32, i32, i32),
        difficulty: impl FnOnce() -> u8,
        now: impl FnOnce() -> Instant,
    ) -> AuraApplicationLikeCpp {
        let multiplier = 1.0 + (fields(effect).1 as f32 / 100.0);
        AuraApplicationLikeCpp {
            spell_id,
            difficulty_id: difficulty(),
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << fields(effect).0,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::ModBattlePetXpPct),
            represented_amount: fields(effect).1,
            represented_effect_amounts: Self::single_effect_amounts(
                fields(effect).0,
                fields(effect).1,
            ),
            represented_misc_value: None,
            represented_multiplier: multiplier,
            applied_at: now(),
        }
    }
}
