// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Assemble represented melee chances from caller-resolved value facts.

use crate::RepresentedMeleeOutcomeInputsLikeCpp;

/// Attacker-side facts the swing owner resolves once per swing, in the form
/// C++ `Unit::RollMeleeOutcomeAgainst` reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RepresentedMeleeAttackerFactsLikeCpp {
    /// `GetLevelForTarget(victim)`.
    pub level: u8,
    /// `haveOffhandWeapon() && !IsInFeralForm()`.
    pub dual_wielding: bool,
    /// C++ `GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_CRIT_DAMAGE_BONUS,
    /// NORMAL)`: the attacker's critical-damage multiplier for physical damage.
    /// `1.0` when nothing is active.
    pub crit_damage_multiplier: f32,
    /// C++ `HasAuraType(SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY)`: an active
    /// aura of that type removes the dual-wield miss penalty regardless of its
    /// amount.
    pub ignores_dual_wield_hit_penalty: bool,
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
    /// C++ `Unit::IsControlledByPlayer()` gate for crushing blows.
    pub is_controlled_by_player: bool,
    /// C++ `CREATURE_FLAG_EXTRA_NO_CRUSHING_BLOWS` gate.
    pub no_crushing_blows: bool,
}

/// Victim-side facts resolved by the swing owner from its canonical authority.
/// This value type does not grant access to another entity or resolve catalogs.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RepresentedMeleeVictimFactsLikeCpp {
    /// `victim->GetLevelForTarget(attacker)`.
    pub level: u8,
    /// Whether the victim is a creature.
    pub is_creature: bool,
    /// Whether the victim is a player. Published avoidance percentages are
    /// resolved by the caller, alongside its aura and stand-state facts.
    pub is_player: bool,
    /// C++ `victim->IsTotem()`: totems have no dodge, parry or block.
    pub is_totem: bool,
    /// C++ `victim->ToCreature()->IsEvadingAttacks()`.
    pub is_evading_attacks: bool,
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
    /// C++ `canParryOrBlock`: `victim->HasInArc(M_PI, attacker)`.
    pub faces_attacker: bool,
    /// C++ `victim->IsImmunedToDamage(SPELL_SCHOOL_MASK_NORMAL)`
    /// (`Unit.cpp:7320-7336`): the victim's school-immunity mask covers the
    /// whole swing school, so the swing ends before any band.
    pub is_immune_to_damage: bool,
    /// C++ `victim->HasUnitState(UNIT_STATE_CONTROLLED)`: a controlled victim
    /// can neither dodge nor parry/block (`Unit.cpp:2296-2304`).
    pub is_controlled: bool,
    /// C++ `victim->IsStandState()` (`Unit.cpp:9960-9964`): a player victim that
    /// is sitting, sleeping or kneeling is always crit. Owners leave this `true`
    /// for a creature victim, which the sitting rule never reads.
    pub is_stand_state: bool,
}

/// C++ `Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2310`) chance assembly
/// for both represented melee attack types.
pub fn melee_outcome_inputs_like_cpp(
    attacker: &RepresentedMeleeAttackerFactsLikeCpp,
    victim: &RepresentedMeleeVictimFactsLikeCpp,
) -> [RepresentedMeleeOutcomeInputsLikeCpp; 2] {
    // The represented table needs a victim whose state the caller can read: a
    // creature victim for the session owner, or a canonical-player victim for
    // the map-owned creature runtime. Anything else keeps the pre-table
    // behaviour rather than inventing a band.
    if !victim.is_creature && !victim.is_player {
        return [RepresentedMeleeOutcomeInputsLikeCpp::NONE; 2];
    }
    // C++ `GetUnitMissChance()` is a flat 5.0 for every unit, so the miss band
    // is identical for both represented victim kinds.
    let mut miss_chance_pct = 5.0;
    if attacker.dual_wielding && !attacker.ignores_dual_wield_hit_penalty {
        miss_chance_pct += 19.0;
    }
    miss_chance_pct -= attacker.melee_hit_chance_pct;
    miss_chance_pct -= attacker.hit_chance_aura_pct;
    miss_chance_pct -= victim.attacker_melee_hit_chance_pct;
    // C++ `MeleeSpellMissChance` ends with `std::max(missChance, 0.f)`.
    let miss_chance_pct = miss_chance_pct.max(0.0);

    // C++ `Unit.cpp:2364-2378` only offers crushing to a non-player-controlled
    // creature at least four levels above its victim and without the template
    // flag. Preserve the target source's raw expression (`Unit.cpp:2371`)
    // instead of silently repairing its negative result.
    let crushing_chance_units = if i32::from(attacker.level) >= i32::from(victim.level) + 4
        && !attacker.is_controlled_by_player
        && !attacker.no_crushing_blows
    {
        i32::from(attacker.level) - i32::from(victim.level) * 1000 - 1500
    } else {
        0
    };

    // C++ `RollMeleeOutcomeAgainst`'s player-victim branch
    // (`Unit.cpp:2284-2360`):
    //   * `canParryOrBlock = victim->HasInArc(M_PI, attacker)` and
    //     `canDodge = victim->GetTypeId() != TYPEID_PLAYER || canParryOrBlock`,
    //     so a player victim dodges only while facing the attacker;
    //   * `GetUnitDodgeChance`/`GetUnitParryChance` read the published
    //     `DodgePercentage`/`ParryPercentage`, which already fold the victim's
    //     ratings, attribute contribution and `MOD_*_PERCENT` auras, and add no
    //     victim-level bonus; `GetUnitParryChance` publishes zero while
    //     `CanParry()` is false;
    //   * the attacker's expertise and dodge reductions still apply;
    //   * there is no glancing band against a player victim; blocking reads
    //     the published BlockPercentage. Blocked damage is a separate rule,
    //     with its armor constant resolved by the caller.
    if victim.is_player {
        let can_avoid = victim.faces_attacker && !victim.is_controlled;
        let dodge_chance_pct = (victim.dodge_pct + attacker.dodge_reduction_pct).max(0.0);
        let parry_chance_pct = victim.parry_pct.max(0.0);
        return std::array::from_fn(|index| {
            let expertise_reduction_pct = attacker.expertise_reduction_pct[index];
            let crit_chance_pct = attacker.crit_pct[index]
                + attacker.autoattack_crit_aura_pct
                + victim.attacker_melee_crit_chance_pct
                + victim.crit_chance_vs_target_health_pct
                + victim.crit_chance_for_caster_pct;
            RepresentedMeleeOutcomeInputsLikeCpp {
                miss_chance_pct,
                dodge_chance_pct: (dodge_chance_pct - expertise_reduction_pct).max(0.0),
                parry_chance_pct: (parry_chance_pct - expertise_reduction_pct).max(0.0),
                // C++ `GetUnitBlockChance`'s player branch reads the published
                // `BlockPercentage` (non-zero only while `CanBlock()` and a
                // shield are present).
                block_chance_pct: victim.block_pct,
                glancing_chance_pct: 0.0,
                crit_chance_pct,
                can_dodge: can_avoid,
                can_parry: can_avoid,
                is_evading_attacks: false,
                always_crits: !victim.is_stand_state && crit_chance_pct > 0.0,
                is_immune_to_damage: victim.is_immune_to_damage,
                crushing_chance_units,
            }
        });
    }

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
            // C++ clears both gates when the victim is casting a non-melee spell
            // or is `UNIT_STATE_CONTROLLED`; the represented creature runtime
            // does not track its current spell slots, so only the unit-state
            // gate is resolved.
            can_dodge: victim.is_creature && !victim.is_controlled,
            can_parry: victim.is_creature && victim.faces_attacker && !victim.is_controlled,
            is_evading_attacks: victim.is_evading_attacks,
            is_immune_to_damage: victim.is_immune_to_damage,
            // The sitting-target rule only applies to a player victim.
            always_crits: false,
            crushing_chance_units,
        }
    })
}
