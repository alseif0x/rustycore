//! Shared player and applied-aura effect projections.

use std::collections::HashMap;
use wow_data::SpellStore;
use wow_entities::AuraApplicationLikeCpp;

/// One resolved effect of a player's applied auras, the shape C++
/// `Unit::GetAuraEffectsByType` exposes to every predicate that also reads
/// `MiscValueB`, the effect's caster or its spell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerAuraEffectLikeCpp {
    /// C++ `AuraApplication::GetSlot()`: identifies the owning application
    /// when callers also need base-aura metadata such as cast provenance.
    pub slot: u8,
    /// C++ `AuraEffect::GetId()`: the owning spell.
    pub spell_id: i32,
    /// C++ `AuraEffect::GetAuraType()`.
    pub aura_type: i32,
    /// C++ `AuraEffect::GetMiscValue()`.
    pub misc_value: i32,
    /// C++ `AuraEffect::GetMiscValueB()`, read by predicates such as
    /// `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH`'s health threshold.
    pub misc_value_b: i32,
    pub amount: i32,
    /// C++ `AuraEffect::GetCasterGUID()`.
    pub caster_guid: wow_core::ObjectGuid,
}

impl PlayerAuraEffectLikeCpp {
    /// The creature-aura projection's shape, which the shared
    /// `MeleeDamageBonusTaken` chain consumes.
    pub fn as_applied_like_cpp(&self) -> AppliedAuraEffectLikeCpp {
        AppliedAuraEffectLikeCpp {
            slot: self.slot,
            spell_id: self.spell_id,
            caster_guid: self.caster_guid,
            aura_type: self.aura_type,
            misc_value: self.misc_value,
            misc_value_b: self.misc_value_b,
            amount: self.amount,
        }
    }
}

/// C++ `Unit::GetAuraEffectsByType(auraType)` for one player's applied auras.
///
/// Rust retains the inherited ascending-slot traversal. C++
/// `GetAuraEffectsByType` returns `m_modAuras[type]`; this comment makes no
/// claim that the Rust slot order matches C++ list order. The amount prefers
/// the represented `AuraEffect` value and falls back to the spell effect's
/// no-caster calculation, exactly like the session adapter used to do.
pub fn player_aura_effects_full_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<PlayerAuraEffectLikeCpp> {
    player_aura_effects_filtered_like_cpp(auras, spell_store, Some(aura_type))
}

/// Every active effect of a player's applied auras, the input C++
/// `MeleeDamageBonusTaken`'s `GetTotalAuraModifier*` chain reads across several
/// aura types at once.
pub fn player_aura_effects_all_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
) -> Vec<PlayerAuraEffectLikeCpp> {
    player_aura_effects_filtered_like_cpp(auras, spell_store, None)
}

fn player_aura_effects_filtered_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: Option<i32>,
) -> Vec<PlayerAuraEffectLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut effects = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        for effect in spell.effects().iter().filter(|effect| {
            aura_type.is_none_or(|aura_type| effect.effect_aura == aura_type)
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            effects.push(PlayerAuraEffectLikeCpp {
                slot,
                spell_id: aura.spell_id,
                aura_type: effect.effect_aura,
                misc_value: effect.effect_misc_value_1,
                misc_value_b: effect.effect_misc_value_2,
                amount,
                caster_guid: aura.caster_guid,
            });
        }
    }
    effects
}

/// The `(MiscValue, amount)` projection every `GetTotalAuraModifier*` caller
/// reads.
pub fn player_aura_effects_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32)> {
    player_aura_effects_full_by_spell_aura_type_like_cpp(auras, spell_store, aura_type)
        .into_iter()
        .map(|effect| (effect.misc_value, effect.amount))
        .collect()
}

/// One resolved effect of a creature's `AppliedAuraRef`, the shape every
/// victim-side C++ `GetTotalAuraModifier*` query reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppliedAuraEffectLikeCpp {
    /// C++ `AuraApplication::GetSlot()` for the effect's owning application.
    pub slot: u8,
    pub spell_id: i32,
    pub caster_guid: wow_core::ObjectGuid,
    /// The effect's `AuraType`.
    pub aura_type: i32,
    /// The effect's `MiscValue`, matched against a school mask where C++ uses
    /// `GetTotalAuraModifierByMiscMask`.
    pub misc_value: i32,
    /// The effect's `MiscValueB`, read by C++ predicates such as
    /// `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH`'s health threshold.
    pub misc_value_b: i32,
    pub amount: i32,
}
