// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented melee attack-table rules (#29).
//!
//! C++ `Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2378`) and the outcome
//! switch in `Unit::CalculateMeleeDamage` (`Unit.cpp:1343-1440`), separated from
//! the `rules_3` damage-bonus family because they own a different transition.

/// C++ `MeleeHitOutcome` (`UnitDefines.h:389-403`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedMeleeOutcomeLikeCpp {
    /// C++ `MELEE_HIT_MISS`.
    Miss,
    /// C++ `MELEE_HIT_DODGE`.
    Dodge,
    /// C++ `MELEE_HIT_PARRY`.
    Parry,
    /// C++ `MELEE_HIT_GLANCING`.
    Glancing,
    /// C++ `MELEE_HIT_BLOCK`.
    Block,
    /// C++ `MELEE_HIT_CRIT`.
    Crit,
    /// C++ `MELEE_HIT_NORMAL`.
    Hit,
}

/// C++ `Unit::GetBlockPercent`'s base implementation (`Unit.h:947`): the flat
/// 30% every non-player victim blocks. `Player::GetBlockPercent`
/// (`Player.cpp:25288`) resolves shield block instead, but the represented table
/// only has creature victims.
pub(crate) const CREATURE_BLOCK_PERCENT_LIKE_CPP: f32 = 30.0;

/// The per-attack-type percentages C++ `Unit::RollMeleeOutcomeAgainst` reads,
/// already resolved by the swing owner.
///
/// C++ works in 1/10000 units and truncates each percentage with
/// `int32(chance * 100.0f)`; a band whose value is zero is skipped without
/// consuming probability, and a band is only offered when the matching
/// `canDodge`/`canParryOrBlock` gate holds.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RepresentedMeleeOutcomeInputsLikeCpp {
    /// `MeleeSpellMissChance(victim, attType, nullptr)`.
    pub miss_chance_pct: f32,
    /// `GetUnitDodgeChance(attType, victim)`.
    pub dodge_chance_pct: f32,
    /// `GetUnitParryChance(attType, victim)`.
    pub parry_chance_pct: f32,
    /// `GetUnitBlockChance(attType, victim)`.
    pub block_chance_pct: f32,
    /// `(10 + 10 * (victimLevel - attackerLevel))` when C++'s glancing
    /// eligibility holds, otherwise zero.
    pub glancing_chance_pct: f32,
    /// `GetUnitCriticalChanceAgainst(attType, victim)` plus the attacker's
    /// `SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE` sum.
    pub crit_chance_pct: f32,
    /// C++ `canDodge`: the victim can dodge this attack.
    pub can_dodge: bool,
    /// C++ `canParryOrBlock`: the victim faces the attacker.
    pub can_parry: bool,
}

impl RepresentedMeleeOutcomeInputsLikeCpp {
    /// No represented avoidance: every band is empty, so the table always
    /// returns `Hit` (the pre-table represented behaviour).
    pub(crate) const NONE: Self = Self {
        miss_chance_pct: 0.0,
        dodge_chance_pct: 0.0,
        parry_chance_pct: 0.0,
        block_chance_pct: 0.0,
        glancing_chance_pct: 0.0,
        crit_chance_pct: 0.0,
        can_dodge: false,
        can_parry: false,
    };
}

/// C++ `urand(0, 9999)`'s inclusive upper bound.
pub(crate) const MELEE_OUTCOME_ROLL_MAX_LIKE_CPP: u32 = 9_999;

/// C++ `int32(chance * 100.0f)`: a percentage in the table's 1/10000 units.
fn chance_units_like_cpp(percent: f32) -> i32 {
    (percent * 100.0) as i32
}

/// C++ `Unit::RollMeleeOutcomeAgainst`'s band order for one already-drawn roll:
/// `MISS > DODGE > PARRY > GLANCING > BLOCK > CRIT > HIT`.
///
/// `roll` is the caller's `urand(0, 9999)`; keeping it a parameter makes the
/// table deterministic for tests while the owners draw it per landed swing.
pub(crate) fn melee_outcome_like_cpp(
    inputs: &RepresentedMeleeOutcomeInputsLikeCpp,
    roll: i32,
) -> RepresentedMeleeOutcomeLikeCpp {
    let mut sum = 0_i32;
    let mut band = |percent: f32, allowed: bool, outcome: RepresentedMeleeOutcomeLikeCpp| {
        if !allowed {
            return None;
        }
        let units = chance_units_like_cpp(percent);
        if units > 0
            && roll < {
                sum += units;
                sum
            }
        {
            return Some(outcome);
        }
        None
    };

    // 1. MISS.
    if let Some(outcome) = band(
        inputs.miss_chance_pct,
        true,
        RepresentedMeleeOutcomeLikeCpp::Miss,
    ) {
        return outcome;
    }
    // 2. DODGE.
    if let Some(outcome) = band(
        inputs.dodge_chance_pct,
        inputs.can_dodge,
        RepresentedMeleeOutcomeLikeCpp::Dodge,
    ) {
        return outcome;
    }
    // 3. PARRY.
    if let Some(outcome) = band(
        inputs.parry_chance_pct,
        inputs.can_parry,
        RepresentedMeleeOutcomeLikeCpp::Parry,
    ) {
        return outcome;
    }
    // 4. GLANCING (eligibility is resolved by the owner into the percentage).
    if let Some(outcome) = band(
        inputs.glancing_chance_pct,
        true,
        RepresentedMeleeOutcomeLikeCpp::Glancing,
    ) {
        return outcome;
    }
    // 5. BLOCK.
    if let Some(outcome) = band(
        inputs.block_chance_pct,
        inputs.can_parry,
        RepresentedMeleeOutcomeLikeCpp::Block,
    ) {
        return outcome;
    }
    // 6. CRIT.
    if let Some(outcome) = band(
        inputs.crit_chance_pct,
        true,
        RepresentedMeleeOutcomeLikeCpp::Crit,
    ) {
        return outcome;
    }
    // 7. CRUSHING needs a creature attacker (`IsControlledByPlayer()` is false);
    // 8. HIT.
    RepresentedMeleeOutcomeLikeCpp::Hit
}

/// C++ `urand(0, 9999)` then [`melee_outcome_like_cpp`]; the owners call this
/// once per landed swing, never for a timer that is not ready.
pub(crate) fn rolled_melee_outcome_like_cpp(
    inputs: &RepresentedMeleeOutcomeInputsLikeCpp,
) -> RepresentedMeleeOutcomeLikeCpp {
    let roll = i32::try_from(wow_core::urand_like_cpp(0, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP))
        .unwrap_or_default();
    melee_outcome_like_cpp(inputs, roll)
}

/// C++ `Unit::CalculateMeleeDamage`'s outcome switch (`Unit.cpp:1343-1440`) for
/// the represented swing.
///
/// Boundaries: the critical-hit `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` multiplier
/// and the crushing 150% branch are not represented; those inputs are absent
/// from the represented table.
pub(crate) fn melee_outcome_damage_like_cpp(
    outcome: RepresentedMeleeOutcomeLikeCpp,
    damage: u32,
    attacker_level: u8,
    victim_level: u8,
) -> (u32, u32) {
    match outcome {
        RepresentedMeleeOutcomeLikeCpp::Miss
        | RepresentedMeleeOutcomeLikeCpp::Dodge
        | RepresentedMeleeOutcomeLikeCpp::Parry => (0, 0),
        RepresentedMeleeOutcomeLikeCpp::Glancing => {
            let mut level_difference = i32::from(victim_level) - i32::from(attacker_level);
            if level_difference > 3 {
                level_difference = 3;
            }
            let reduce_percent = 1.0 - level_difference as f32 * 0.1;
            ((reduce_percent * damage as f32) as u32, 0)
        }
        RepresentedMeleeOutcomeLikeCpp::Block => {
            // C++ `CalculatePct(damage, GetBlockPercent(attackerLevel))`
            // truncates; `IsBlockCritical` needs the victim's aura sum, which
            // has no represented producer, so the doubled block is absent.
            let blocked = (damage as f32 * CREATURE_BLOCK_PERCENT_LIKE_CPP / 100.0) as u32;
            (damage.saturating_sub(blocked), blocked)
        }
        RepresentedMeleeOutcomeLikeCpp::Crit => (damage.saturating_mul(2), 0),
        RepresentedMeleeOutcomeLikeCpp::Hit => (damage, 0),
    }
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
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_BLOCK, HIT_INFO_CRITICAL_HIT, HIT_INFO_GLANCING,
        HIT_INFO_MISS, HIT_INFO_OFFHAND, VICTIM_STATE_DODGE, VICTIM_STATE_HIT, VICTIM_STATE_INTACT,
        VICTIM_STATE_PARRY,
    };

    let mut hit_info = if offhand { HIT_INFO_OFFHAND } else { 0 };
    let victim_state = match outcome {
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
        RepresentedMeleeOutcomeLikeCpp::Hit => {
            hit_info |= HIT_INFO_AFFECTS_VICTIM;
            VICTIM_STATE_HIT
        }
    };
    (hit_info, victim_state)
}

/// Attacker-side facts the swing owner resolves once per swing, in the form
/// C++ `Unit::RollMeleeOutcomeAgainst` reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RepresentedMeleeAttackerFactsLikeCpp {
    /// `GetLevelForTarget(victim)`.
    pub level: u8,
    /// `haveOffhandWeapon() && !IsInFeralForm() && !HasAuraType(
    /// SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY)`.
    pub dual_wielding: bool,
    /// C++ `m_modMeleeHitChance`: `7.5 + GetRatingBonusValue(CR_HIT_MELEE)`.
    pub melee_hit_chance_pct: f32,
    /// The attacker's `SPELL_AURA_MOD_HIT_CHANCE` sum.
    pub hit_chance_aura_pct: f32,
    /// `GetUnitCriticalChanceDone`: `CritPercentage` for the base attack and
    /// `OffhandCritPercentage` for the offhand.
    pub crit_pct: [f32; 2],
    /// The attacker's `SPELL_AURA_MOD_AUTOATTACK_CRIT_CHANCE` sum.
    pub autoattack_crit_aura_pct: f32,
    /// `Player::GetExpertiseDodgeOrParryReduction(attType)`: the published
    /// `MainhandExpertise`/`OffhandExpertise` divided by four.
    pub expertise_reduction_pct: [f32; 2],
    /// `GetUnitDodgeChance`'s attacker-side reductions: the attacker's
    /// `SPELL_AURA_MOD_COMBAT_RESULT_CHANCE` sum for `VICTIMSTATE_DODGE` plus
    /// its `SPELL_AURA_MOD_ENEMY_DODGE` sum. They only affect dodge.
    pub dodge_reduction_pct: f32,
}

/// Victim-side facts the swing owner resolves once per swing. A represented
/// player victim is not supported by this unit: the session owner cannot read
/// another player's snapshot, so only creature victims contribute a table.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RepresentedMeleeVictimFactsLikeCpp {
    /// `victim->GetLevelForTarget(attacker)`.
    pub level: u8,
    /// Whether the victim is a creature (the only represented avoidance source).
    pub is_creature: bool,
    /// C++ `victim->IsTotem()`: totems have no dodge, parry or block.
    pub is_totem: bool,
    /// C++ `GetUnitDodgeChance`'s creature base (`CreatureBaseStats`-seeded),
    /// before the victim-level bonus and the attacker's expertise reduction.
    pub dodge_pct: f32,
    /// C++ `GetUnitParryChance`'s creature base; zero when the template carries
    /// `CREATURE_FLAG_EXTRA_NO_PARRY`.
    pub parry_pct: f32,
    /// C++ `GetUnitBlockChance`'s creature base; zero when the template carries
    /// `CREATURE_FLAG_EXTRA_NO_BLOCK`.
    pub block_pct: f32,
    /// The victim's `SPELL_AURA_MOD_DODGE_PERCENT` sum.
    pub dodge_aura_pct: f32,
    /// The victim's `SPELL_AURA_MOD_PARRY_PERCENT` sum.
    pub parry_aura_pct: f32,
    /// The victim's `SPELL_AURA_MOD_BLOCK_PERCENT` sum.
    pub block_aura_pct: f32,
    /// The victim's `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum, subtracted
    /// from the attacker's miss chance.
    pub attacker_melee_hit_chance_pct: f32,
    /// The victim's `SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE` plus
    /// `SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE` sums, added to the
    /// attacker's critical chance.
    pub attacker_melee_crit_chance_pct: f32,
    /// The victim's `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` sum, kept
    /// only for effects whose `MiscValueB` health threshold the victim is not
    /// below (the owner applies C++'s `!HealthBelowPct` predicate).
    pub crit_chance_vs_target_health_pct: f32,
    /// The victim's `SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER` sum, kept only for
    /// effects the attacker cast (the owner applies C++'s caster predicate).
    pub crit_chance_for_caster_pct: f32,
    /// C++ `canParryOrBlock`: `victim->HasInArc(M_PI, attacker)`. C++
    /// `canDodge` is true for every creature victim outside casting/control,
    /// which the represented model does not resolve yet.
    pub faces_attacker: bool,
}

/// C++ `Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2310`) chance assembly
/// for both represented melee attack types.
pub(crate) fn melee_outcome_inputs_like_cpp(
    attacker: &RepresentedMeleeAttackerFactsLikeCpp,
    victim: &RepresentedMeleeVictimFactsLikeCpp,
) -> [RepresentedMeleeOutcomeInputsLikeCpp; 2] {
    // The represented table needs a creature victim: a canonical-player victim's
    // avoidance lives in that player's session, so it keeps the pre-table
    // behaviour rather than inventing a band, exactly like the creature-only
    // armour rule.
    if !victim.is_creature {
        return [RepresentedMeleeOutcomeInputsLikeCpp::NONE; 2];
    }
    // C++ `GetUnitMissChance()` is a flat 5.0 for every unit.
    let mut miss_chance_pct = 5.0;
    if attacker.dual_wielding {
        miss_chance_pct += 19.0;
    }
    miss_chance_pct -= attacker.melee_hit_chance_pct;
    miss_chance_pct -= attacker.hit_chance_aura_pct;
    miss_chance_pct -= victim.attacker_melee_hit_chance_pct;
    // C++ `MeleeSpellMissChance` ends with `std::max(missChance, 0.f)`.
    let miss_chance_pct = miss_chance_pct.max(0.0);

    let level_difference = i32::from(victim.level) - i32::from(attacker.level);
    let level_bonus = if level_difference > 0 {
        1.5 * level_difference as f32
    } else {
        0.0
    };

    let mut dodge_chance_pct = 0.0;
    let mut parry_chance_pct = 0.0;
    let mut block_chance_pct = 0.0;
    if victim.is_creature && !victim.is_totem {
        // C++ `GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance`
        // creature branches, including the victim's percentage auras.
        dodge_chance_pct = victim.dodge_pct + victim.dodge_aura_pct + level_bonus;
        parry_chance_pct = victim.parry_pct + victim.parry_aura_pct + level_bonus;
        block_chance_pct = victim.block_pct + victim.block_aura_pct + level_bonus;
        // C++ `GetUnitDodgeChance` adds the attacker's combat-result and
        // enemy-dodge modifiers after the level bonus.
        dodge_chance_pct += attacker.dodge_reduction_pct;
    }

    // C++ glancing: player/pet attacker against a non-player victim more than
    // three levels higher.
    let glancing_chance_pct = if victim.is_creature && attacker.level + 3 < victim.level {
        (10 + 10 * (i32::from(victim.level) - i32::from(attacker.level))) as f32
    } else {
        0.0
    };

    std::array::from_fn(|index| {
        let expertise_reduction_pct = attacker.expertise_reduction_pct[index];
        RepresentedMeleeOutcomeInputsLikeCpp {
            miss_chance_pct,
            dodge_chance_pct: (dodge_chance_pct - expertise_reduction_pct).max(0.0),
            parry_chance_pct: (parry_chance_pct - expertise_reduction_pct).max(0.0),
            block_chance_pct,
            glancing_chance_pct,
            crit_chance_pct: attacker.crit_pct[index]
                + attacker.autoattack_crit_aura_pct
                + victim.attacker_melee_crit_chance_pct
                + victim.crit_chance_vs_target_health_pct
                + victim.crit_chance_for_caster_pct,
            can_dodge: victim.is_creature,
            can_parry: victim.is_creature && victim.faces_attacker,
        }
    })
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
