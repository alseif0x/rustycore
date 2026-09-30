// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

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

/// One represented `SPELL_AURA_SCHOOL_ABSORB` shield of a player victim.
///
/// C++ `Unit::CalcAbsorbResist` (`Unit.cpp:1813-1880`) copies
/// `GetAuraEffectsByType(SPELL_AURA_SCHOOL_ABSORB)`, sorts it with
/// `Trinity::AbsorbAuraOrderPred` and depletes each effect's amount. The
/// application slot and effect index are carried here because the depletion is
/// a canonical aura-amount write, not a recomputation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RepresentedAbsorbShieldLikeCpp {
    /// The aura application slot that owns the effect.
    pub slot: u8,
    /// C++ `AuraEffect::GetEffIndex()`.
    pub effect_index: u8,
    /// C++ `AuraEffect::GetId()`.
    pub spell_id: i32,
    /// C++ `SpellInfo::GetCategory()` (`SpellInfo.cpp:1361-1364`), the
    /// `AbsorbAuraOrderPred` Ice Barrier rank.
    pub category_id: u32,
    /// C++ `AuraEffect::GetAmount()`. A negative amount is an infinite-absorb
    /// script shield, which C++ clamps to zero before absorbing.
    pub amount: i32,
    /// C++ `SpellInfo::HasAttribute(SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE)`: an
    /// absorb that an attacker's `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` cannot
    /// reduce (`Unit.cpp:1830-1832`).
    pub cannot_be_ignored: bool,
}

/// One represented `SPELL_AURA_MANA_SHIELD` of a player victim.
///
/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop (`Unit.cpp:1886-1930`) reads
/// the effect's amount as the damage cap and
/// `SpellEffectInfo::CalcValueMultiplier` (`Amplitude`) as the mana drained per
/// absorbed point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RepresentedManaShieldLikeCpp {
    /// The aura application slot that owns the effect.
    pub slot: u8,
    /// C++ `AuraEffect::GetEffIndex()`.
    pub effect_index: u8,
    /// C++ `AuraEffect::GetId()`.
    pub spell_id: i32,
    /// C++ `AuraEffect::GetAmount()`. A negative amount is an infinite-absorb
    /// script shield, which C++ clamps to zero.
    pub amount: i32,
    /// C++ `SpellEffectInfo::CalcValueMultiplier(caster)`'s data term: the mana
    /// the shield drains per point of absorbed damage.
    pub mana_multiplier: f32,
    /// C++ `SpellInfo::HasAttribute(SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE)`.
    pub cannot_be_ignored: bool,
}

/// One represented `SPELL_AURA_SCHOOL_HEAL_ABSORB` of a player victim.
///
/// C++ `Unit::CalcHealAbsorb` (`Unit.cpp:2020-2084`) copies
/// `GetAuraEffectsByType(SPELL_AURA_SCHOOL_HEAL_ABSORB)` and depletes each
/// effect's amount by the heal it consumed; the application slot and effect
/// index are carried so the depletion is a canonical aura-amount write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RepresentedHealAbsorbShieldLikeCpp {
    /// The aura application slot that owns the effect.
    pub slot: u8,
    /// C++ `AuraEffect::GetEffIndex()`.
    pub effect_index: u8,
    /// C++ `AuraEffect::GetAmount()`. A negative amount is an infinite-absorb
    /// script shield, which C++ clamps to zero.
    pub amount: i32,
}
