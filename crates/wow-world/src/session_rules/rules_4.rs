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
    pub(crate) const NONE: Self = Self {
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
/// the represented swing, as the `(Damage, Blocked, OriginalDamage)` triple C++
/// publishes.
///
/// C++ assigns `OriginalDamage` inside each arm, not once before the switch:
/// the avoided arms, the glancing reduction and the block all keep the
/// pre-outcome value (`Unit.cpp:1345-1355`, `1415-1427`), while the critical arm
/// assigns it *after* doubling and the
/// `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` multiplier (`Unit.cpp:1362-1375`), so a
/// critical swing publishes the doubled value as its original too.
///
/// The crushing branch is retained with the target's source expression. It is
/// normally unreachable because `Unit.cpp:2371` produces a negative band for
/// ordinary levels; this is a fidelity boundary, not a Rust-side correction.
pub(crate) fn melee_outcome_damage_like_cpp(
    outcome: RepresentedMeleeOutcomeLikeCpp,
    damage: u32,
    attacker_level: u8,
    victim_level: u8,
    crit_damage_multiplier: f32,
    // C++ `victim->GetBlockPercent(attackerLevel)`: the flat `30.0` creature
    // base (`Unit.h:947`) or a player's `Player::GetBlockPercent`
    // (`Player.cpp:25288-25298`). `CalculatePct(damage, pct)` divides by 100,
    // so a player's returned *fraction* blocks at most `0.85%` of the damage,
    // exactly like the target build.
    block_percent_like_cpp: f32,
) -> (u32, u32, u32) {
    match outcome {
        RepresentedMeleeOutcomeLikeCpp::Immune
        | RepresentedMeleeOutcomeLikeCpp::Evade
        | RepresentedMeleeOutcomeLikeCpp::Miss
        | RepresentedMeleeOutcomeLikeCpp::Dodge
        | RepresentedMeleeOutcomeLikeCpp::Parry => (0, 0, damage),
        RepresentedMeleeOutcomeLikeCpp::Glancing => {
            let mut level_difference = i32::from(victim_level) - i32::from(attacker_level);
            if level_difference > 3 {
                level_difference = 3;
            }
            let reduce_percent = 1.0 - level_difference as f32 * 0.1;
            let reduced = (reduce_percent * damage as f32) as u32;
            (reduced, 0, damage)
        }
        RepresentedMeleeOutcomeLikeCpp::Block => {
            // C++ `CalculatePct(damage, GetBlockPercent(attackerLevel))`
            // truncates; `IsBlockCritical` needs the victim's aura sum, which
            // has no represented producer, so the doubled block is absent.
            let blocked = (damage as f32 * block_percent_like_cpp / 100.0) as u32;
            (damage.saturating_sub(blocked), blocked, damage)
        }
        RepresentedMeleeOutcomeLikeCpp::Crit => {
            // C++ doubles the damage and then applies
            // `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` (`Unit.cpp:1362-1375`).
            let doubled = (damage as f32 * 2.0 * crit_damage_multiplier).max(0.0) as u32;
            (doubled, 0, doubled)
        }
        RepresentedMeleeOutcomeLikeCpp::Crushing => {
            // C++ `Unit.cpp:1423-1429`: 150% normal damage, with the
            // post-multiplier value published as `OriginalDamage`.
            let crushing = damage.saturating_add(damage / 2);
            (crushing, 0, crushing)
        }
        RepresentedMeleeOutcomeLikeCpp::Hit => (damage, 0, damage),
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

/// Attacker-side facts the swing owner resolves once per swing, in the form
/// C++ `Unit::RollMeleeOutcomeAgainst` reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RepresentedMeleeAttackerFactsLikeCpp {
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

/// Victim-side facts the swing owner resolves once per swing. The session owner
/// cannot read another player's snapshot, so only the map-owned creature
/// runtime resolves a canonical-player victim; it contributes the miss, dodge,
/// parry and crit bands (its block band and armour mitigation stay boundaries).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RepresentedMeleeVictimFactsLikeCpp {
    /// `victim->GetLevelForTarget(attacker)`.
    pub level: u8,
    /// Whether the victim is a creature (the only represented avoidance source).
    pub is_creature: bool,
    /// Whether the victim is a player. A player victim has no represented
    /// avoidance yet, but C++ `MeleeSpellMissChance` reads its
    /// `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum and its `IsStandState`
    /// exactly like a creature's, so the miss band is representable.
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

/// C++ `Player::GetBlockPercent(attackerLevel)` (`Player.cpp:25288-25298`): the
/// published `ActivePlayerData::ShieldBlock` over itself plus
/// `DB2Manager::EvaluateExpectedStat(ExpectedStatType::ArmorConstant, ...)`,
/// capped at `0.85`, and `0` when both inputs are zero.
///
/// Boundary: the represented runtime does not load `ExpectedStat.db2`, so the
/// caller passes C++'s own empty-store fallback (`EvaluateExpectedStat` returns
/// `1.0f` when the level row is absent); a later unit can load the table and
/// pass its value instead.
pub(crate) fn player_block_percent_like_cpp(shield_block: i32, armor_constant: f32) -> f32 {
    let block_armor = shield_block.max(0) as f32;
    if block_armor + armor_constant == 0.0 {
        return 0.0;
    }
    (block_armor / (block_armor + armor_constant)).min(0.85)
}

/// C++ `Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2310`) chance assembly
/// for both represented melee attack types.
pub(crate) fn melee_outcome_inputs_like_cpp(
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
    //   * there is no glancing band (the C++ condition needs a player or pet
    //     attacker) and `GetUnitBlockChance`'s band is left out because the
    //     blocked *damage* needs C++ `Player::GetBlockPercent`'s DB2
    //     `ExpectedStatType::ArmorConstant` table, which the represented data
    //     does not load.
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
