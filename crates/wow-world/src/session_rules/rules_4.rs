// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters for melee RNG, packet presentation and aura mitigation.
//!
//! Pure attack-table arithmetic lives in `wow-combat`; these adapters retain
//! the original random-draw call, packet mapping and aura/catalog dependencies.

pub(crate) use wow_combat::{
    CREATURE_BLOCK_PERCENT_LIKE_CPP, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP,
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeOutcomeInputsLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    melee_outcome_damage_like_cpp, melee_outcome_inputs_like_cpp, melee_outcome_like_cpp,
    player_block_percent_like_cpp,
};

/// C++ `urand(0, 9999)` then [`melee_outcome_like_cpp`]; the owners call this
/// once per landed swing, never for a timer that is not ready.
pub(crate) fn rolled_melee_outcome_like_cpp(
    inputs: &RepresentedMeleeOutcomeInputsLikeCpp,
) -> RepresentedMeleeOutcomeLikeCpp {
    let roll = i32::try_from(wow_core::urand_like_cpp(0, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP))
        .unwrap_or_default();
    melee_outcome_like_cpp(inputs, roll)
}

/// C++ `CalcDamageInfo::HitInfo` and `TargetState` for one represented outcome
/// (`UnitDefines.h:440-465`, `Unit.h:45-55`), including the `HITINFO_OFFHAND`
/// the offhand branch sets before the table and the `HITINFO_AFFECTS_VICTIM`
/// C++ adds to every non-miss outcome.
pub(crate) fn melee_outcome_presentation_like_cpp(
    outcome: RepresentedMeleeOutcomeLikeCpp,
    offhand: bool,
) -> (u32, u8) {
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_BLOCK, HIT_INFO_CRITICAL_HIT, HIT_INFO_CRUSHING,
        HIT_INFO_GLANCING, HIT_INFO_MISS, HIT_INFO_NORMALSWING, HIT_INFO_OFFHAND,
        HIT_INFO_SWING_NO_HIT_SOUND, VICTIM_STATE_DODGE, VICTIM_STATE_EVADES, VICTIM_STATE_HIT,
        VICTIM_STATE_INTACT, VICTIM_STATE_IS_IMMUNE, VICTIM_STATE_PARRY,
    };

    let mut hit_info = if offhand { HIT_INFO_OFFHAND } else { 0 };
    let victim_state = match outcome {
        RepresentedMeleeOutcomeLikeCpp::Immune => {
            // C++ ORs `HITINFO_NORMALSWING` (`0x0`) and returns before the
            // `HITINFO_AFFECTS_VICTIM` line, so a main-hand immune swing
            // publishes a zero `hitInfo` with `VICTIMSTATE_IS_IMMUNE`.
            hit_info |= HIT_INFO_NORMALSWING;
            VICTIM_STATE_IS_IMMUNE
        }
        RepresentedMeleeOutcomeLikeCpp::Evade => {
            // C++ `CalculateMeleeDamage`'s `MELEE_HIT_EVADE` branch sets both
            // `HITINFO_MISS` and `HITINFO_SWINGNOHITSOUND` (`Unit.cpp:1345-1355`).
            hit_info |= HIT_INFO_MISS | HIT_INFO_SWING_NO_HIT_SOUND;
            VICTIM_STATE_EVADES
        }
        RepresentedMeleeOutcomeLikeCpp::Miss => {
            hit_info |= HIT_INFO_MISS;
            VICTIM_STATE_INTACT
        }
        RepresentedMeleeOutcomeLikeCpp::Dodge => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM;
            VICTIM_STATE_DODGE
        }
        RepresentedMeleeOutcomeLikeCpp::Parry => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM;
            VICTIM_STATE_PARRY
        }
        RepresentedMeleeOutcomeLikeCpp::Glancing => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM | HIT_INFO_GLANCING;
            VICTIM_STATE_HIT
        }
        RepresentedMeleeOutcomeLikeCpp::Block => {
            // C++ keeps `VICTIMSTATE_HIT` for a blocked hit and marks the block
            // through `HITINFO_BLOCK` (`Unit.cpp:1399-1407`).
            hit_info |= HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK;
            VICTIM_STATE_HIT
        }
        RepresentedMeleeOutcomeLikeCpp::Crit => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT;
            VICTIM_STATE_HIT
        }
        RepresentedMeleeOutcomeLikeCpp::Crushing => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRUSHING;
            VICTIM_STATE_HIT
        }
        RepresentedMeleeOutcomeLikeCpp::Hit => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM;
            VICTIM_STATE_HIT
        }
    };
    (hit_info, victim_state)
}

/// C++ `Unit::MeleeDamageBonusTaken`'s `(TakenFlatBenefit, TakenTotalMod)` for a
/// white swing, already resolved from the victim's and attacker's auras by the
/// swing owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RepresentedMeleeDamageTakenLikeCpp {
    pub flat: i32,
    pub pct: f32,
}

impl RepresentedMeleeDamageTakenLikeCpp {
    /// No represented taken modifiers: the damage passes through unchanged.
    pub(crate) const NONE: Self = Self { flat: 0, pct: 1.0 };
}

/// C++ `Unit::MeleeDamageBonusTaken` for `spellProto == null` and a melee attack
/// type (`Unit.cpp:1687-1756`): the victim's `SPELL_AURA_MOD_DAMAGE_TAKEN` sum
/// for the attacker's melee school, its `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` sum,
/// the `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` multiplier for that school, the
/// `SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER` multiplier restricted to auras the
/// attacker cast, and the `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` multiplier.
///
/// The Sanctified Wrath bypass C++ applies afterwards is summed from the
/// attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effects covering the same
/// school. Boundaries: the fixed cheat-death aura (45182), the ranged variants
/// and every `spellProto` branch cannot apply to a represented white swing; the
/// versatility term is commented out in the 3.4.3 source itself.
pub(crate) fn melee_damage_taken_flat_pct_like_cpp(
    victim_effects: &[crate::session_rules::AppliedAuraEffectLikeCpp],
    attacker_ignore_resist: &[(i32, i32)],
    attacker_guid: wow_core::ObjectGuid,
    school_mask: i32,
) -> RepresentedMeleeDamageTakenLikeCpp {
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN, SPELL_AURA_MOD_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
        SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT,
    };

    let flat = victim_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == SPELL_AURA_MOD_DAMAGE_TAKEN && effect.misc_value & school_mask != 0
        })
        .map(|effect| effect.amount)
        .sum::<i32>()
        + victim_effects
            .iter()
            .filter(|effect| effect.aura_type == SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN)
            .map(|effect| effect.amount)
            .sum::<i32>();

    let multiplier = |aura_type: i32, caster: Option<wow_core::ObjectGuid>| {
        victim_effects
            .iter()
            .filter(|effect| {
                effect.aura_type == aura_type
                    && caster.is_none_or(|guid| effect.caster_guid == guid)
            })
            .fold(1.0_f32, |total, effect| {
                total * (1.0 + effect.amount as f32 / 100.0)
            })
    };
    let mut pct = 1.0_f32;
    pct *= victim_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN
                && effect.misc_value & school_mask != 0
        })
        .fold(1.0_f32, |total, effect| {
            total * (1.0 + effect.amount as f32 / 100.0)
        });
    pct *= multiplier(SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER, Some(attacker_guid));
    pct *= multiplier(SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT, None);

    // C++ `Unit::MeleeDamageBonusTaken`'s Sanctified Wrath bypass: while the
    // victim's total modifier reduces damage, the attacker's
    // `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` shrinks that reduction.
    if pct < 1.0 {
        let mut damage_reduction = 1.0 - pct;
        for (misc_value, amount) in attacker_ignore_resist {
            if misc_value & school_mask == 0 {
                continue;
            }
            damage_reduction *= 1.0 - *amount as f32 / 100.0;
        }
        pct = 1.0 - damage_reduction;
    }

    RepresentedMeleeDamageTakenLikeCpp { flat, pct }
}

/// C++ `Unit::MeleeDamageBonusTaken`'s tail (`Unit.cpp:1758-1759`): the flat
/// benefit is added, the total modifier multiplies and the result truncates at
/// zero. C++ returns zero before the arithmetic when the flat benefit is
/// negative enough to absorb the whole hit.
pub(crate) fn melee_damage_taken_apply_like_cpp(
    taken: RepresentedMeleeDamageTakenLikeCpp,
    damage: u32,
) -> u32 {
    if damage == 0 {
        return 0;
    }
    if taken.flat < 0 && (damage as i32) < -taken.flat {
        return 0;
    }
    (((damage as i32 + taken.flat) as f32) * taken.pct).max(0.0) as u32
}

/// `Trinity::AbsorbAuraOrderPred`'s rank (`SpellAuraEffects.h:365-407`).
///
/// C++ sorts the victim's school-absorb effects so Fel Blossom (`28527`), the
/// Ice Barrier category (`471`) and Sacrifice (`7812`) are spent first, while
/// Cauterize (`86949`) and Spirit of Redemption (`20711`) are always last. The
/// predicate only orders those named ranks and returns false for every other
/// pair, so C++'s `std::sort` leaves equal-rank shields in an unspecified order;
/// this rank reproduces the named order and keeps equal ranks in the caller's
/// deterministic slot order.
pub(crate) fn represented_absorb_priority_like_cpp(
    shield: &crate::session_rules::RepresentedAbsorbShieldLikeCpp,
) -> u8 {
    // Lowest spends first.
    if shield.spell_id == 28527 {
        return 0; // Fel Blossom
    }
    if shield.category_id == 471 {
        return 1; // Ice Barrier
    }
    if shield.spell_id == 7812 {
        return 2; // Sacrifice
    }
    if shield.spell_id == 86949 {
        return 4; // Cauterize (must be last)
    }
    if shield.spell_id == 20711 {
        return 5; // Spirit of Redemption (must be last)
    }
    3
}

/// One shield's depletion, applied by the canonical aura owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAbsorbConsumptionLikeCpp {
    pub slot: u8,
    pub effect_index: u8,
    /// C++ `currentAbsorb`, after the `[0, damage]` clamp.
    pub consumed: i32,
    /// C++ `AuraEffect::GetAmount() - currentAbsorb`.
    pub remaining: i32,
    /// C++ `if (absorbAurEff->GetAmount() <= 0) Remove(AURA_REMOVE_BY_ENEMY_SPELL)`.
    pub removed: bool,
}

/// C++ `Unit::CalcAbsorbResist`'s school-absorb result for one hit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RepresentedMeleeAbsorbLikeCpp {
    /// C++ `DamageInfo::GetAbsorb()`.
    pub absorbed: u32,
    /// C++ `DamageInfo::GetDamage()` after the shields were spent.
    pub damage: u32,
    /// The per-shield depletion the canonical aura owner must commit.
    pub consumed: Vec<RepresentedAbsorbConsumptionLikeCpp>,
}

/// C++ `Unit::CalcAbsorbResist`'s school-absorb loop
/// (`Unit.cpp:1791-1880`) for one physical melee hit.
///
/// The incoming damage is offered to each shield in
/// `Trinity::AbsorbAuraOrderPred` order. A negative amount is an infinite
/// absorb C++ clamps to zero for safety, the amount is clamped to the damage
/// left, a fully spent shield is removed, and a shield that consumes nothing
/// still reports a zero consumption so the loop's clamps stay observable.
///
/// Boundaries: C++'s `absorbIgnoringDamage` term (an attacker's
/// `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` reduced by
/// `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`) has no represented producer or spell
/// attribute projection, `SPELL_AURA_MANA_SHIELD` needs a power write the melee
/// path does not own, and `Unit::CalcSpellResistedDamage` returns zero for a
/// non-magic school mask (`Unit.cpp:2058-2060`), so physical melee never resists.
/// C++ `Unit::CalcAbsorbResist`'s `auraAbsorbMod`
/// (`Unit.cpp:1803-1811`): the attacker's
/// `GetMaxPositiveAuraModifierByMiscMask(SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL,
/// schoolMask)` clamped to `[0, 100]`.
pub(crate) fn represented_melee_ignore_absorb_like_cpp(
    attacker_effects: &[crate::session_rules::AppliedAuraEffectLikeCpp],
    school_mask: u32,
) -> f32 {
    attacker_effects
        .iter()
        .filter(|effect| {
            effect.aura_type == wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL
                && (effect.misc_value as u32) & school_mask != 0
        })
        .map(|effect| effect.amount as f32)
        .fold(0.0_f32, f32::max)
        .clamp(0.0, 100.0)
}

/// C++ `CalculatePct(damage, auraAbsorbMod)`: the damage portion the attacker's
/// ignore-absorb modifier removes from what a shield may take.
pub(crate) fn represented_melee_ignored_absorb_amount_like_cpp(damage: u32, pct: f32) -> u32 {
    if pct <= 0.0 {
        return 0;
    }
    ((damage as f32) * pct / 100.0) as u32
}

pub(crate) fn represented_melee_absorb_like_cpp(
    shields: &[crate::session_rules::RepresentedAbsorbShieldLikeCpp],
    damage: u32,
    ignore_absorb_pct: f32,
) -> RepresentedMeleeAbsorbLikeCpp {
    let mut result = RepresentedMeleeAbsorbLikeCpp {
        absorbed: 0,
        damage,
        consumed: Vec::new(),
    };
    if damage == 0 || shields.is_empty() {
        return result;
    }
    let ignore = represented_melee_ignored_absorb_amount_like_cpp(damage, ignore_absorb_pct);
    let mut ordered: Vec<&crate::session_rules::RepresentedAbsorbShieldLikeCpp> =
        shields.iter().collect();
    ordered.sort_by_key(|shield| represented_absorb_priority_like_cpp(shield));
    let mut remaining_damage = damage;
    for shield in ordered {
        if remaining_damage == 0 {
            break;
        }
        // C++ `damageInfo.ModifyDamage(-absorbIgnoringDamage)` for every shield
        // whose spell lacks `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`, then the
        // `[0, damage]` clamp. C++ restores the reduction after the shield; the
        // temporary damage is floored at zero here instead of reproducing the
        // negative clamp C++ can reach when the ignoring amount exceeds what is
        // left.
        let absorbable_damage = if shield.cannot_be_ignored {
            remaining_damage
        } else {
            remaining_damage.saturating_sub(ignore)
        };
        // C++ `if (currentAbsorb < 0) currentAbsorb = 0;`
        let available = shield.amount.max(0);
        let consumed = available.min(i32::try_from(absorbable_damage).unwrap_or(i32::MAX));
        if consumed > 0 {
            remaining_damage -= consumed as u32;
            result.absorbed += consumed as u32;
        }
        // C++ only changes an amount-counting shield; a negative (infinite)
        // shield keeps its amount and is never removed here.
        let remaining = if shield.amount >= 0 {
            shield.amount - consumed
        } else {
            shield.amount
        };
        result.consumed.push(RepresentedAbsorbConsumptionLikeCpp {
            slot: shield.slot,
            effect_index: shield.effect_index,
            consumed,
            remaining,
            // C++ only removes an amount-counting shield.
            removed: shield.amount >= 0 && remaining <= 0,
        });
    }
    result.damage = remaining_damage;
    result
}

/// One mana shield's depletion, applied by the canonical aura owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedManaShieldConsumptionLikeCpp {
    pub slot: u8,
    pub effect_index: u8,
    /// C++ `currentAbsorb` after the mana scaling: the damage the shield
    /// actually took.
    pub consumed: i32,
    /// C++ `AuraEffect::GetAmount() - currentAbsorb`.
    pub remaining: i32,
    /// C++ `if (absorbAurEff->GetAmount() <= 0) Remove(AURA_REMOVE_BY_ENEMY_SPELL)`.
    pub removed: bool,
}

/// C++ `Unit::CalcAbsorbResist`'s mana-shield result for one hit.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RepresentedMeleeManaAbsorbLikeCpp {
    /// C++ `DamageInfo::GetAbsorb()` after the mana scaling.
    pub absorbed: u32,
    /// C++ `DamageInfo::GetDamage()` after the shields were spent.
    pub damage: u32,
    /// The mana C++ `ModifyPower(POWER_MANA, -manaReduction)` actually removed.
    pub mana_spent: u32,
    /// The per-shield depletion the canonical owner must commit.
    pub consumed: Vec<RepresentedManaShieldConsumptionLikeCpp>,
}

/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop (`Unit.cpp:1886-1930`) for
/// one physical melee hit.
///
/// Each shield's amount caps the damage it may take, the drain is
/// `amount * SpellEffectInfo::CalcValueMultiplier` (the data `Amplitude`), and
/// the absorbed damage scales down by the fraction of that drain the victim's
/// mana could pay (`manaTaken / manaReduction`). A negative amount is clamped
/// to zero, an amount-counting shield is depleted and removed at zero, and once
/// the damage is gone the remaining shields are not visited.
///
/// Boundaries: C++'s `absorbIgnoringDamage` term and the spellmod half of
/// `CalcValueMultiplier` stay unrepresented (`mana_multiplier` is the data
/// amplitude alone), and a zero drain resolves to no absorb instead of C++'s
/// `0 / 0` float division.
pub(crate) fn represented_melee_mana_absorb_like_cpp(
    shields: &[crate::session_rules::RepresentedManaShieldLikeCpp],
    damage: u32,
    available_mana: u32,
    ignore_absorb_pct: f32,
) -> RepresentedMeleeManaAbsorbLikeCpp {
    let mut result = RepresentedMeleeManaAbsorbLikeCpp {
        absorbed: 0,
        damage,
        mana_spent: 0,
        consumed: Vec::new(),
    };
    if damage == 0 || shields.is_empty() {
        return result;
    }
    let ignore = represented_melee_ignored_absorb_amount_like_cpp(damage, ignore_absorb_pct);
    let mut remaining_damage = damage;
    let mut remaining_mana = available_mana;
    for shield in shields {
        if remaining_damage == 0 {
            break;
        }
        // C++ `damageInfo.ModifyDamage(-absorbIgnoringDamage)` for a mana shield
        // without `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`, then the `[0, damage]`
        // clamp.
        let absorbable_damage = if shield.cannot_be_ignored {
            remaining_damage
        } else {
            remaining_damage.saturating_sub(ignore)
        };
        // C++ `if (currentAbsorb < 0) currentAbsorb = 0;` then the `[0, damage]`
        // clamp.
        let current = shield.amount.max(0) as u32;
        let current = current.min(absorbable_damage);
        let base_reduction = i32::try_from(current).unwrap_or(i32::MAX);
        // C++ `if (float manaMultiplier = CalcValueMultiplier(caster))
        // manaReduction = int32(float(manaReduction) * manaMultiplier);`
        let mana_reduction = if shield.mana_multiplier != 0.0 {
            (base_reduction as f32 * shield.mana_multiplier) as i32
        } else {
            base_reduction
        };
        let mana_taken = u32::try_from(mana_reduction.max(0))
            .unwrap_or(0)
            .min(remaining_mana);
        // C++ `currentAbsorb = currentAbsorb ? int32(float(currentAbsorb) *
        // (float(manaTaken) / float(manaReduction))) : 0;`
        let current_absorb = if current != 0 && mana_reduction > 0 {
            (current as f32 * (mana_taken as f32 / mana_reduction as f32)) as i32
        } else {
            0
        };
        let current_absorb = current_absorb.max(0);
        if current_absorb > 0 {
            remaining_damage = remaining_damage.saturating_sub(current_absorb as u32);
            result.absorbed += current_absorb as u32;
        }
        remaining_mana = remaining_mana.saturating_sub(mana_taken);
        result.mana_spent += mana_taken;
        let remaining = if shield.amount >= 0 {
            shield.amount - current_absorb
        } else {
            shield.amount
        };
        result
            .consumed
            .push(RepresentedManaShieldConsumptionLikeCpp {
                slot: shield.slot,
                effect_index: shield.effect_index,
                consumed: current_absorb,
                remaining,
                removed: shield.amount >= 0 && remaining <= 0,
            });
    }
    result.damage = remaining_damage;
    result
}

/// C++ `Unit::CalcHealAbsorb`'s result for one heal.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RepresentedHealAbsorbLikeCpp {
    /// C++ `HealInfo::GetAbsorb()`.
    pub absorbed: u32,
    /// C++ `HealInfo::GetHeal()` after the shields were spent.
    pub heal: u32,
    /// The per-shield depletion the canonical owner must commit.
    pub consumed: Vec<RepresentedAbsorbConsumptionLikeCpp>,
}

/// C++ `Unit::CalcHealAbsorb`'s loop (`Unit.cpp:2026-2068`) for one heal.
///
/// Each heal-absorb shield's amount is clamped to the heal left, an
/// amount-counting shield is depleted and removed at zero, and a negative
/// (infinite) amount is clamped to zero and never removed. Unlike the damage
/// absorb loop there is no priority sort and no ignore-absorb term in C++.
pub(crate) fn represented_heal_absorb_like_cpp(
    shields: &[crate::session_rules::RepresentedHealAbsorbShieldLikeCpp],
    heal: u32,
) -> RepresentedHealAbsorbLikeCpp {
    let mut result = RepresentedHealAbsorbLikeCpp {
        absorbed: 0,
        heal,
        consumed: Vec::new(),
    };
    if heal == 0 || shields.is_empty() {
        return result;
    }
    let mut remaining_heal = heal;
    for shield in shields {
        if remaining_heal == 0 {
            break;
        }
        // C++ `if (currentAbsorb < 0) currentAbsorb = 0;` then the `[0, heal]`
        // clamp.
        let available = shield.amount.max(0);
        let consumed = available.min(i32::try_from(remaining_heal).unwrap_or(i32::MAX));
        if consumed > 0 {
            remaining_heal -= consumed as u32;
            result.absorbed += consumed as u32;
        }
        let remaining = if shield.amount >= 0 {
            shield.amount - consumed
        } else {
            shield.amount
        };
        result.consumed.push(RepresentedAbsorbConsumptionLikeCpp {
            slot: shield.slot,
            effect_index: shield.effect_index,
            consumed,
            remaining,
            removed: shield.amount >= 0 && remaining <= 0,
        });
    }
    result.heal = remaining_heal;
    result
}
