// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented melee outcome arithmetic; no packet presentation or mutation.

use crate::RepresentedMeleeOutcomeLikeCpp;

/// C++ `Unit::GetBlockPercent`'s base implementation (`Unit.h:947`): the flat
/// 30% every non-player victim blocks. `Player::GetBlockPercent`
/// (`Player.cpp:25288`) resolves shield block instead; callers select the
/// appropriate value for the victim.
pub const CREATURE_BLOCK_PERCENT_LIKE_CPP: f32 = 30.0;

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
pub fn melee_outcome_damage_like_cpp(
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

/// C++ `Player::GetBlockPercent(attackerLevel)` (`Player.cpp:25288-25298`): the
/// published `ActivePlayerData::ShieldBlock` over itself plus
/// `DB2Manager::EvaluateExpectedStat(ExpectedStatType::ArmorConstant, ...)`,
/// capped at `0.85`, and `0` when both inputs are zero.
///
/// The caller resolves the armor constant from its catalog (including any
/// empty-store fallback); this arithmetic neither loads data nor owns state.
pub fn player_block_percent_like_cpp(shield_block: i32, armor_constant: f32) -> f32 {
    let block_armor = shield_block.max(0) as f32;
    if block_armor + armor_constant == 0.0 {
        return 0.0;
    }
    (block_armor / (block_armor + armor_constant)).min(0.85)
}
