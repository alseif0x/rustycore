// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters for melee RNG, packet presentation and aura mitigation.
//!
//! Pure attack-table arithmetic lives in `wow-combat`; these adapters retain
//! the original random-draw call, packet mapping and aura/catalog dependencies.

pub(crate) use wow_combat::{
    CREATURE_BLOCK_PERCENT_LIKE_CPP, MELEE_OUTCOME_ROLL_MAX_LIKE_CPP,
    RepresentedAbsorbConsumptionLikeCpp, RepresentedHealAbsorbLikeCpp,
    RepresentedManaShieldConsumptionLikeCpp, RepresentedMeleeAbsorbLikeCpp,
    RepresentedMeleeManaAbsorbLikeCpp,
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeOutcomeInputsLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    represented_absorb_priority_like_cpp, represented_heal_absorb_like_cpp,
    represented_melee_absorb_like_cpp, represented_melee_ignored_absorb_amount_like_cpp,
    represented_melee_mana_absorb_like_cpp,
    melee_outcome_damage_like_cpp, melee_outcome_inputs_like_cpp, melee_outcome_like_cpp,
    player_block_percent_like_cpp,
};

pub(crate) use wow_combat::{
    RepresentedMeleeDamageTakenLikeCpp, melee_damage_taken_apply_like_cpp,
    melee_damage_taken_flat_pct_like_cpp, represented_melee_ignore_absorb_like_cpp,
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
