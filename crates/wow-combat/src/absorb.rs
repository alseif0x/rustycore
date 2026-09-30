// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

pub use wow_data_model::aura_effects::{
    RepresentedAbsorbShieldLikeCpp, RepresentedHealAbsorbShieldLikeCpp,
    RepresentedManaShieldLikeCpp,
};

/// `Trinity::AbsorbAuraOrderPred`'s rank (`SpellAuraEffects.h:365-407`).
///
/// C++ sorts the victim's school-absorb effects so Fel Blossom (`28527`), the
/// Ice Barrier category (`471`) and Sacrifice (`7812`) are spent first, while
/// Cauterize (`86949`) and Spirit of Redemption (`20711`) are always last. The
/// predicate only orders those named ranks and returns false for every other
/// pair, so C++'s `std::sort` leaves equal-rank shields in an unspecified order;
/// this rank reproduces the named order and keeps equal ranks in the caller's
/// deterministic slot order.
pub fn represented_absorb_priority_like_cpp(shield: &RepresentedAbsorbShieldLikeCpp) -> u8 {
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
pub struct RepresentedAbsorbConsumptionLikeCpp {
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
pub struct RepresentedMeleeAbsorbLikeCpp {
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
/// These pure rules consume caller-resolved shields and ignore-absorb input.
/// World callers retain aura/catalog selection and canonical aura-amount and
/// mana-power writes. This extraction does not add script callbacks or
/// resistance-stage handling.
pub fn represented_melee_absorb_like_cpp(
    shields: &[RepresentedAbsorbShieldLikeCpp],
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
    let mut ordered: Vec<&RepresentedAbsorbShieldLikeCpp> = shields.iter().collect();
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

/// C++ `CalculatePct(damage, auraAbsorbMod)`: the damage portion the attacker's
/// ignore-absorb modifier removes from what a shield may take.
pub fn represented_melee_ignored_absorb_amount_like_cpp(damage: u32, pct: f32) -> u32 {
    if pct <= 0.0 {
        return 0;
    }
    ((damage as f32) * pct / 100.0) as u32
}

/// One mana shield's depletion, applied by the canonical aura owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedManaShieldConsumptionLikeCpp {
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
pub struct RepresentedMeleeManaAbsorbLikeCpp {
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
/// The spellmod half of `CalcValueMultiplier` remains unrepresented
/// (`mana_multiplier` is the data amplitude alone), and a zero drain resolves
/// to no absorb instead of C++'s `0 / 0` float division.
pub fn represented_melee_mana_absorb_like_cpp(
    shields: &[RepresentedManaShieldLikeCpp],
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
pub struct RepresentedHealAbsorbLikeCpp {
    /// C++ `HealInfo::GetAbsorb()`.
    pub absorbed: u32,
    /// C++ `HealInfo::GetHeal()` after the shields were spent.
    pub heal: u32,
    /// The per-shield depletion the canonical aura owner must commit.
    pub consumed: Vec<RepresentedAbsorbConsumptionLikeCpp>,
}

/// C++ `Unit::CalcHealAbsorb`'s loop (`Unit.cpp:2026-2068`) for one heal.
///
/// Each heal-absorb shield's amount is clamped to the heal left, an
/// amount-counting shield is depleted and removed at zero, and a negative
/// (infinite) amount is clamped to zero and never removed. Unlike the damage
/// absorb loop there is no priority sort and no ignore-absorb term in C++.
pub fn represented_heal_absorb_like_cpp(
    shields: &[RepresentedHealAbsorbShieldLikeCpp],
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
