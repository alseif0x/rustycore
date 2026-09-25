// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deterministic represented melee attack table; no RNG or runtime state.

/// C++ `MeleeHitOutcome` (`UnitDefines.h:389-403`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedMeleeOutcomeLikeCpp {
    /// C++ `CalculateMeleeDamage`'s physical immunity return
    /// (`Unit.cpp:1315-1324`): the victim is immune to the swing's school, so
    /// the swing ends before any damage roll or band with
    /// `HITINFO_NORMALSWING`/`VICTIMSTATE_IS_IMMUNE` and zero damage.
    Immune,
    /// C++ `MELEE_HIT_EVADE`: an evading creature victim returns this before any
    /// band is rolled (`Unit.cpp:2274-2275`).
    Evade,
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
    /// C++ `MELEE_HIT_CRUSHING`.
    Crushing,
    /// C++ `MELEE_HIT_NORMAL`.
    Hit,
}

/// The per-attack-type percentages C++ `Unit::RollMeleeOutcomeAgainst` reads,
/// already resolved by the swing owner.
///
/// C++ works in 1/10000 units and truncates each percentage with
/// `int32(chance * 100.0f)`; a band whose value is zero is skipped without
/// consuming probability, and a band is only offered when the matching
/// `canDodge`/`canParryOrBlock` gate holds.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RepresentedMeleeOutcomeInputsLikeCpp {
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
    /// C++ `victim->IsImmunedToDamage(schoolMask)`
    /// (`Unit.cpp:7320-7336`): a physical immune check that precedes the whole
    /// table.
    pub is_immune_to_damage: bool,
    /// C++ `victim->ToCreature()->IsEvadingAttacks()`: the table returns
    /// `MELEE_HIT_EVADE` before rolling.
    pub is_evading_attacks: bool,
    /// C++ `RollMeleeOutcomeAgainst`'s sitting-target rule
    /// (`Unit.cpp:2312-2314`): a player victim that is not in a stand state is
    /// always crit while the attacker's critical chance is non-zero.
    pub always_crits: bool,
    /// C++ `Unit.cpp:2371`'s raw crushing band in 1/10000 units. The target
    /// source uses `attackerLevel - victimLevel * 1000 - 1500` verbatim.
    pub crushing_chance_units: i32,
}

impl RepresentedMeleeOutcomeInputsLikeCpp {
    /// No represented avoidance: every band is empty, so the table always
    /// returns `Hit` (the pre-table represented behaviour).
    pub const NONE: Self = Self {
        miss_chance_pct: 0.0,
        dodge_chance_pct: 0.0,
        parry_chance_pct: 0.0,
        block_chance_pct: 0.0,
        glancing_chance_pct: 0.0,
        crit_chance_pct: 0.0,
        can_dodge: false,
        can_parry: false,
        is_evading_attacks: false,
        always_crits: false,
        is_immune_to_damage: false,
        crushing_chance_units: 0,
    };
}

/// C++ `urand(0, 9999)`'s inclusive upper bound.
pub const MELEE_OUTCOME_ROLL_MAX_LIKE_CPP: u32 = 9_999;

/// C++ `int32(chance * 100.0f)`: a percentage in the table's 1/10000 units.
fn chance_units_like_cpp(percent: f32) -> i32 {
    (percent * 100.0) as i32
}

/// C++ `Unit::RollMeleeOutcomeAgainst`'s band order for one already-drawn roll:
/// `MISS > sitting CRIT > DODGE > PARRY > GLANCING > BLOCK > CRIT > CRUSHING > HIT`.
///
/// `roll` is the caller's `urand(0, 9999)`; keeping it a parameter makes the
/// table deterministic for tests while the owners draw it per landed swing.
pub fn melee_outcome_like_cpp(
    inputs: &RepresentedMeleeOutcomeInputsLikeCpp,
    roll: i32,
) -> RepresentedMeleeOutcomeLikeCpp {
    // C++ ends the swing before every band when the victim is immune to the
    // attack's school.
    if inputs.is_immune_to_damage {
        return RepresentedMeleeOutcomeLikeCpp::Immune;
    }
    // C++ returns `MELEE_HIT_EVADE` before the bands when the victim is an
    // evading creature.
    if inputs.is_evading_attacks {
        return RepresentedMeleeOutcomeLikeCpp::Evade;
    }
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
    // C++ returns `MELEE_HIT_CRIT` before the avoidance bands when a player
    // victim is not in a stand state and the attacker's critical chance is
    // non-zero (`Unit.cpp:2312-2314`).
    if inputs.always_crits {
        return RepresentedMeleeOutcomeLikeCpp::Crit;
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
    // 7. CRUSHING. This value is already in C++'s 1/10000 units. The target
    // source expression is intentionally preserved verbatim; for ordinary
    // levels it is negative and therefore cannot win a roll.
    sum += inputs.crushing_chance_units;
    if roll < sum {
        return RepresentedMeleeOutcomeLikeCpp::Crushing;
    }
    // 8. HIT.
    RepresentedMeleeOutcomeLikeCpp::Hit
}
