// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

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
pub fn armor_reduced_damage_like_cpp(
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
