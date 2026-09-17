// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free Session rules, part 3.
//!
//! The aura-effect projection lives here so the session adapter and the
//! map-owned legacy runtime can resolve the same canonical Player auras with
//! one implementation instead of drifting apart.

use crate::session::*;
use std::collections::HashMap;
use wow_data::SpellStore;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp};

/// One resolved effect of a player's applied auras, the shape C++
/// `Unit::GetAuraEffectsByType` exposes to every predicate that also reads
/// `MiscValueB`, the effect's caster or its spell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlayerAuraEffectLikeCpp {
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
    pub(crate) fn as_applied_like_cpp(&self) -> AppliedAuraEffectLikeCpp {
        AppliedAuraEffectLikeCpp {
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
/// Auras are visited in ascending slot order so the projection is deterministic
/// (C++ walks its aura list in application order); the amount prefers the
/// represented `AuraEffect` value and falls back to the spell effect's
/// no-caster calculation, exactly like the session adapter used to do.
pub(crate) fn player_aura_effects_full_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<PlayerAuraEffectLikeCpp> {
    player_aura_effects_filtered_like_cpp(auras, spell_store, Some(aura_type))
}

/// Every active effect of a player's applied auras, the input C++
/// `MeleeDamageBonusTaken`'s `GetTotalAuraModifier*` chain reads across several
/// aura types at once.
pub(crate) fn player_aura_effects_all_like_cpp(
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
pub(crate) fn player_aura_effects_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32)> {
    player_aura_effects_full_by_spell_aura_type_like_cpp(auras, spell_store, aura_type)
        .into_iter()
        .map(|effect| (effect.misc_value, effect.amount))
        .collect()
}

/// One represented `SPELL_AURA_SCHOOL_ABSORB` shield of a player victim.
///
/// C++ `Unit::CalcAbsorbResist` (`Unit.cpp:1813-1880`) copies
/// `GetAuraEffectsByType(SPELL_AURA_SCHOOL_ABSORB)`, sorts it with
/// `Trinity::AbsorbAuraOrderPred` and depletes each effect's amount. The
/// application slot and effect index are carried here because the depletion is
/// a canonical aura-amount write, not a recomputation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RepresentedAbsorbShieldLikeCpp {
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
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_SCHOOL_ABSORB` selection
/// (`Unit.cpp:1812-1825`): every active absorb effect whose `MiscValue` covers
/// the incoming school mask.
///
/// Auras are visited in ascending slot order so the input to the priority sort
/// is deterministic; the amount prefers the represented `AuraEffect` value and
/// falls back to the spell effect's no-caster calculation, exactly like the
/// generic aura projection.
pub(crate) fn player_absorb_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedAbsorbShieldLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut shields = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        let category_id = spell_store
            .hit_metadata_for_difficulty_like_cpp(aura.spell_id, difficulty_id, difficulty_store)
            .map_or(0, |metadata| metadata.category_id);
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                // C++ `!(absorbAurEff->GetMiscValue() & damageInfo.GetSchoolMask())`.
                && (effect.effect_misc_value_1 as u32) & school_mask != 0
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            shields.push(RepresentedAbsorbShieldLikeCpp {
                slot,
                effect_index: u8::try_from(effect.effect_index).unwrap_or(0),
                spell_id: aura.spell_id,
                category_id,
                amount,
            });
        }
    }
    shields
}

/// One represented `SPELL_AURA_MANA_SHIELD` of a player victim.
///
/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop (`Unit.cpp:1886-1930`) reads
/// the effect's amount as the damage cap and
/// `SpellEffectInfo::CalcValueMultiplier` (`Amplitude`) as the mana drained per
/// absorbed point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RepresentedManaShieldLikeCpp {
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
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_MANA_SHIELD` selection
/// (`Unit.cpp:1886-1897`): every active mana-shield effect whose `MiscValue`
/// covers the incoming school mask.
///
/// C++ iterates `GetAuraEffectsByType` in application order; the represented
/// projection visits auras in ascending slot order so the loop is
/// deterministic.
pub(crate) fn player_mana_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    school_mask: u32,
) -> Vec<RepresentedManaShieldLikeCpp> {
    let mut slots: Vec<u8> = auras.keys().copied().collect();
    slots.sort_unstable();
    let mut shields = Vec::new();
    for slot in slots {
        let aura = &auras[&slot];
        let Some(spell) = spell_store.get(aura.spell_id) else {
            continue;
        };
        for effect in spell.effects().iter().filter(|effect| {
            effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MANA_SHIELD
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
                // C++ `!(absorbAurEff->GetMiscValue() & damageInfo.GetSchoolMask())`.
                && (effect.effect_misc_value_1 as u32) & school_mask != 0
        }) {
            let amount = aura
                .represented_effect_amounts
                .iter()
                .find(|represented| {
                    u8::try_from(effect.effect_index).ok() == Some(represented.effect_index)
                })
                .map(|represented| represented.amount)
                .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
            shields.push(RepresentedManaShieldLikeCpp {
                slot,
                effect_index: u8::try_from(effect.effect_index).unwrap_or(0),
                spell_id: aura.spell_id,
                amount,
                mana_multiplier: effect.calc_value_multiplier_like_cpp(),
            });
        }
    }
    shields
}

/// C++ `Unit::MeleeDamageBonusDone`'s auto-attack percentage term
/// (`Unit.cpp:7620-7627`): `AddPct(DoneTotalMod, amount)` for every active
/// `SPELL_AURA_MOD_AUTOATTACK_DAMAGE` effect. The represented white swing
/// multiplies its rolled damage by the returned factor; `1.0` when nothing is
/// active.
pub(crate) fn represented_autoattack_damage_multiplier_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
) -> f32 {
    player_aura_effects_by_spell_aura_type_like_cpp(
        auras,
        spell_store,
        wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE,
    )
    .into_iter()
    .fold(1.0_f32, |total, (_, amount)| {
        total * (1.0 + amount as f32 / 100.0)
    })
}

/// C++ `Unit::MeleeDamageBonusDone`'s white-swing terms (`Unit.cpp:7558-7650`)
/// resolved from one attacker's aura effects: the flat
/// `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` benefit, the
/// `SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS` bonus converted with
/// `GetAPMultiplier`, the `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` multiplier, plus
/// the victim's `HasAuraState` (`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE`)
/// and `HasAuraWithMechanic`
/// (`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC`) multipliers.
/// `(DoneFlatBenefit, DoneTotalMod)`.
///
/// `victim_attack_power_bonus` is the victim's
/// `SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS` (`165`) sum, which C++ reads
/// from the victim and the caller resolves because only a player victim's auras
/// carry represented effect amounts.
pub(crate) fn melee_damage_bonus_done_from_effects_like_cpp(
    attacker_effects: &[AppliedAuraEffectLikeCpp],
    victim_attack_power_bonus: i32,
    creature_type_mask: u32,
    victim_aura_state_mask: u32,
    victim_mechanic_mask: u64,
    is_ranged: bool,
    attack_power_multiplier: f32,
) -> (i32, f32) {
    let matches = |misc_value: i32| misc_value & creature_type_mask as i32 != 0;
    let flat_sum = |aura_type: i32| -> i32 {
        attacker_effects
            .iter()
            .filter(|effect| effect.aura_type == aura_type)
            .map(|effect| effect.amount)
            .sum()
    };
    let flat_sum_by_mask = |aura_type: i32| -> i32 {
        attacker_effects
            .iter()
            .filter(|effect| effect.aura_type == aura_type && matches(effect.misc_value))
            .map(|effect| effect.amount)
            .sum()
    };
    let pct_by_mask = |aura_type: i32| -> f32 {
        attacker_effects
            .iter()
            .filter(|effect| effect.aura_type == aura_type && matches(effect.misc_value))
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            })
    };
    let mut flat = victim_attack_power_bonus;
    if creature_type_mask != 0 {
        flat = flat.saturating_add(flat_sum_by_mask(
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
        ));
        let versus_aura_type = if is_ranged {
            wow_data::spell::aura_types::SPELL_AURA_MOD_RANGED_ATTACK_POWER_VERSUS
        } else {
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS
        };
        let ap_bonus = flat_sum_by_mask(versus_aura_type);
        if ap_bonus != 0 {
            flat = flat.saturating_add((ap_bonus as f32 / 3.5 * attack_power_multiplier) as i32);
        }
    }
    if victim_attack_power_bonus != 0 {
        // The victim's own AP-granting aura is converted with the same
        // `GetAPMultiplier` factor C++ uses for the attacker-side `APbonus`.
        flat = flat.saturating_add(
            (victim_attack_power_bonus as f32 / 3.5 * attack_power_multiplier) as i32,
        );
    }
    let mut done_total_mod = 1.0_f32;
    if creature_type_mask != 0 {
        done_total_mod *=
            pct_by_mask(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS);
    }
    // C++ `Unit::MeleeDamageBonusDone`'s auto-attack percentage branch
    // (`Unit.cpp:7620-7627`), read for every white swing.
    done_total_mod *= attacker_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE
        })
        .fold(1.0_f32, |total, effect| {
            total * (1.0 + effect.amount as f32 / 100.0)
        });
    // C++ `Unit::MeleeDamageBonusDone`'s "bonus against aurastate"
    // (`Unit.cpp:7634-7640`).
    if victim_aura_state_mask != 0 {
        done_total_mod *= attacker_effects
            .iter()
            .filter(|effect| {
                effect.aura_type
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE
                    && u32::try_from(effect.misc_value)
                        .ok()
                        .and_then(|state| state.checked_sub(1))
                        .and_then(|bit| 1_u32.checked_shl(bit))
                        .is_some_and(|bit| victim_aura_state_mask & bit != 0)
            })
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            });
    }
    // C++ `Unit::MeleeDamageBonusDone`'s "bonus against target aura mechanic"
    // (`Unit.cpp:7642-7648`).
    if victim_mechanic_mask != 0 {
        done_total_mod *= attacker_effects
            .iter()
            .filter(|effect| {
                effect.aura_type
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC
                    && (1..64).contains(&effect.misc_value)
                    && victim_mechanic_mask & (1_u64 << effect.misc_value) != 0
            })
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            });
    }
    (flat, done_total_mod)
}

/// C++ `Unit::MeleeDamageBonusDone`'s white-swing terms for a canonical
/// Player attacker, read across the represented aura-effect projection.
///
/// Boundary: a creature victim's `SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`
/// has no represented effect amount, so `victim_attack_power_bonus` is zero
/// here; a player victim resolves it through
/// [`melee_damage_bonus_done_from_effects_like_cpp`] directly.
pub(crate) fn melee_damage_bonus_done_like_cpp(
    attacker_auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    creature_type_mask: u32,
    victim_aura_state_mask: u32,
    victim_mechanic_mask: u64,
    is_ranged: bool,
    attack_power_multiplier: f32,
) -> (i32, f32) {
    let effects: Vec<AppliedAuraEffectLikeCpp> =
        player_aura_effects_all_like_cpp(attacker_auras, spell_store)
            .into_iter()
            .map(|effect| effect.as_applied_like_cpp())
            .collect();
    melee_damage_bonus_done_from_effects_like_cpp(
        &effects,
        0,
        creature_type_mask,
        victim_aura_state_mask,
        victim_mechanic_mask,
        is_ranged,
        attack_power_multiplier,
    )
}

/// C++ `Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`) over a canonical
/// Player's `AuraApplication` map: the union of each application's
/// `SpellInfo::Mechanic` and its applied `IsEffect()` slots' mechanics, read at
/// the application's own difficulty.
pub(crate) fn aura_application_mechanic_mask_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    auras.values().fold(0_u64, |mask, aura| {
        mask | spell_mechanic_mask_like_cpp(
            spell_store,
            difficulty_store,
            aura.spell_id,
            aura.difficulty_id,
            aura.effect_mask,
        )
    })
}

/// C++ `Unit::HasAuraWithMechanic` over a creature's `AppliedAuraRef` list.
/// Those refs do not retain a difficulty, so the caller supplies the one it
/// resolves the rest of the victim's state with.
pub(crate) fn applied_aura_mechanic_mask_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    applied_auras.iter().fold(0_u64, |mask, aura| {
        mask | spell_mechanic_mask_like_cpp(
            spell_store,
            difficulty_store,
            i32::try_from(aura.spell_id).unwrap_or(0),
            difficulty_id,
            aura.effect_mask,
        )
    })
}

/// The mechanic bits one `AuraApplication`/`AppliedAuraRef` contributes: its
/// spell's own `Mechanic` plus every applied `IsEffect()` slot that carries one.
fn spell_mechanic_mask_like_cpp(
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    spell_id: i32,
    difficulty_id: u8,
    effect_mask: u32,
) -> u64 {
    let Some(metadata) =
        spell_store.hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
    else {
        return 0;
    };
    let mut mask = mechanic_bit_like_cpp(i32::from(metadata.spell_mechanic)).unwrap_or(0);
    let effects =
        spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store);
    for (effect_index, mechanic) in metadata.effect_mechanics {
        if !(1..32).contains(&effect_index) || effect_mask & (1_u32 << effect_index) == 0 {
            continue;
        }
        let is_effect = effects.is_some_and(|effects| {
            effects
                .iter()
                .any(|effect| effect.effect_index == effect_index && effect.effect != 0)
        });
        if is_effect {
            mask |= mechanic_bit_like_cpp(mechanic).unwrap_or(0);
        }
    }
    mask
}

/// C++ `UI64LIT(1) << mechanic` for a positive mechanic; `None` for the unset
/// or out-of-range value a malformed row could carry.
fn mechanic_bit_like_cpp(mechanic: i32) -> Option<u64> {
    (1..64).contains(&mechanic).then(|| 1_u64 << mechanic)
}

/// C++ `Unit::CalculateDamage(attType, normalized = false, addTotalPct = true)`
/// (`Unit.cpp:2384-2435`): both `UnitData` bounds are clamped at zero, ordered,
/// truncated to `uint32` and resolved with one inclusive `urand`. The represented
/// `UnitData` range is already `Player::CalculateMinMaxDamage`'s published value,
/// so the roll is the only missing step; C++ draws it per landed swing, never
/// from a timer that is not ready.
pub(crate) fn white_swing_roll_like_cpp(min_damage: f32, max_damage: f32) -> u32 {
    let min_damage = min_damage.max(0.0);
    let max_damage = max_damage.max(0.0);
    let (min_damage, max_damage) = if min_damage > max_damage {
        (max_damage, min_damage)
    } else {
        (min_damage, max_damage)
    };
    wow_core::urand_like_cpp(min_damage as u32, max_damage as u32)
}

/// C++ `Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) for the represented
/// physical melee swing.
///
/// The represented inputs are the victim's `Unit::GetArmor()` (a creature's
/// `GenerateArmor` value or a player's published armour), the attacker's live
/// `GetRatingBonusValue(CR_ARMOR_PENETRATION)` percentage, the victim's
/// `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` (345) sum for effects the attacker cast,
/// the attacker's `SPELL_AURA_MOD_TARGET_RESISTANCE` (123) sum covering
/// `SPELL_SCHOOL_MASK_NORMAL`, and the attacker's
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269) sum covering the same school.
/// `GetArmorMultiplierForTarget` is `1.0` for every 3.4.3 unit (no override), so
/// it is not a term.
///
/// Boundaries: a spell's `SpellModOp::TargetResistance` adjustment cannot apply
/// to an auto-attack (`spellInfo == null`), and C++ truncates each
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effect separately
/// (`armor = std::floor(AddPct(armor, -amount))`) while the owner sums the
/// amounts first, so two concurrent effects differ from C++ by that per-step
/// truncation.
pub(crate) fn armor_reduced_damage_like_cpp(
    damage: u32,
    attacker_level: u8,
    victim_level: u8,
    victim_armor: i32,
    armor_penetration_pct: f32,
    target_resistance_normal_aura: i32,
    ignore_target_resist_normal_pct: f32,
    bypass_armor_pct_by_caster: f32,
) -> u32 {
    // `armor *= victim->GetArmorMultiplierForTarget(attacker)` is a no-op.
    let mut armor = victim_armor.max(0) as f32;
    // C++ `armor = CalculatePct(armor, 100 - std::min(armorBypassPct, 100))`
    // over the victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` effects that the
    // attacker cast (`Unit.cpp:1631-1637`).
    if bypass_armor_pct_by_caster != 0.0 {
        armor = armor * (100.0 - bypass_armor_pct_by_caster.min(100.0)) / 100.0;
    }
    // `armor += attacker->GetTotalAuraModifierByMiscMask(MOD_TARGET_RESISTANCE,
    // NORMAL)`; a negative sum is armour penetration.
    armor += target_resistance_normal_aura as f32;

    // `armor = std::floor(AddPct(armor, -amount))` over the attacker's
    // `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effects of the normal school.
    if ignore_target_resist_normal_pct != 0.0 {
        armor = (armor * (1.0 - ignore_target_resist_normal_pct / 100.0)).floor();
    }

    // `Player` CR_ARMOR_PENETRATION rating bonus, capped the way C++ caps it.
    let victim_level = victim_level as f32;
    let max_armor_pen = if victim_level < 60.0 {
        400.0 + 85.0 * victim_level
    } else {
        400.0 + 85.0 * victim_level + 4.5 * 85.0 * (victim_level - 59.0)
    };
    let max_armor_pen = ((armor + max_armor_pen) / 3.0).min(armor);
    armor -= max_armor_pen * (armor_penetration_pct.clamp(0.0, 100.0) / 100.0);

    if armor <= 0.0 {
        armor = 0.0;
    }

    // `levelModifier = attacker->GetLevel()`, extended above level 59.
    let mut level_modifier = attacker_level as f32;
    if level_modifier > 59.0 {
        level_modifier += 4.5 * (level_modifier - 59.0);
    }
    let mut damage_reduction = 0.1 * armor / (8.5 * level_modifier + 40.0);
    damage_reduction /= 1.0 + damage_reduction;
    let damage_reduction = damage_reduction.clamp(0.0, 0.75);

    (damage as f32 * (1.0 - damage_reduction)).max(0.0).ceil() as u32
}

/// One resolved effect of a creature's `AppliedAuraRef`, the shape every
/// victim-side C++ `GetTotalAuraModifier*` query reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AppliedAuraEffectLikeCpp {
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

/// C++ `Unit::GetAuraEffectsByType`/`GetTotalAuraModifier*` over a creature's
/// `AuraApplicationMap`: every active effect of the unit's applied auras,
/// resolved at the caller's difficulty.
///
/// The player-side counterpart is
/// [`player_aura_effects_by_spell_aura_type_like_cpp`]; a creature's
/// `AppliedAuraRef` carries no represented effect amounts, so the effect's
/// no-caster calculation is the value, exactly like the mechanic-mask
/// projection.
pub(crate) fn creature_aura_effects_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Vec<AppliedAuraEffectLikeCpp> {
    let mut effects = Vec::new();
    for aura in applied_auras {
        let spell_id = i32::try_from(aura.spell_id).unwrap_or(0);
        let Some(spell_effects) =
            spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
        else {
            continue;
        };
        for effect in spell_effects.iter().filter(|effect| {
            effect.effect_aura != 0
                && 1u32
                    .checked_shl(effect.effect_index)
                    .is_some_and(|bit| aura.effect_mask & bit != 0)
        }) {
            effects.push(AppliedAuraEffectLikeCpp {
                spell_id,
                caster_guid: aura.caster_guid,
                aura_type: effect.effect_aura,
                misc_value: effect.effect_misc_value_1,
                misc_value_b: effect.effect_misc_value_2,
                amount: effect.calc_value_no_caster_like_cpp(),
            });
        }
    }
    effects
}
