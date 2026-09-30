//! Pure C++ Player stat and weapon-damage projections.

use super::{PlayerStatSystemInputLikeCpp, PlayerStatSystemProjectionLikeCpp};

/// C++ `Player::CalculateMinMaxDamage` (`StatSystem.cpp:428-478`) for the
/// represented player weapon ranges. Item ranges replace the base
/// MINDAMAGE/MAXDAMAGE values and the attack-power term uses the equipped
/// delay; an empty range retains the unarmed 1/2 values and the two-second C++
/// default multiplier.
///
/// `shapeshift_combat_round_time` is the active
/// `SpellShapeshiftFormEntry::CombatRoundTime` (`StatSystem.cpp:461-467`): a
/// feral form rescales the base weapon damage by
/// `CombatRoundTime / 1000 / GetAPMultiplier` before the attack-power term is
/// added back.
pub fn effective_weapon_damage_ranges_like_cpp(
    projection: PlayerStatSystemProjectionLikeCpp,
    weapon_damage: [[f32; 2]; 3],
    base_attack_time: [u32; 3],
    shapeshift_combat_round_time: Option<f32>,
) -> [[f32; 2]; 3] {
    let total_ap = projection.total_attack_power.max(0) as f32;
    let total_ranged_ap = projection.total_ranged_attack_power.max(0) as f32;
    std::array::from_fn(|index| {
        let attack =
            <wow_constants::WeaponAttackType as num_traits::FromPrimitive>::from_usize(index)
                .unwrap_or(wow_constants::WeaponAttackType::BaseAttack);
        let has_item_range = weapon_damage[index][0] > 0.0 && weapon_damage[index][1] > 0.0;
        if attack == wow_constants::WeaponAttackType::RangedAttack
            && !has_item_range
            && total_ranged_ap == 0.0
        {
            return [0.0, 0.0];
        }
        let attack_power = if attack == wow_constants::WeaponAttackType::RangedAttack {
            total_ranged_ap
        } else {
            total_ap
        };
        let attack_power_multiplier = if base_attack_time[index] > 0 {
            // C++ clamps `GetAPMultiplier` to 0.25 in
            // `Player::CalculateMinMaxDamage`, even when a malformed/custom
            // weapon delay is shorter than 250 ms.
            (base_attack_time[index] as f32 / 1000.0).max(0.25)
        } else {
            2.0
        };
        let [weapon_min, weapon_max] = if has_item_range {
            weapon_damage[index]
        } else {
            [1.0, 2.0]
        };
        let [weapon_min, weapon_max] =
            match shapeshift_combat_round_time.filter(|round_time| *round_time > 0.0) {
                Some(round_time) => {
                    let scale = round_time / 1000.0 / attack_power_multiplier;
                    [weapon_min * scale, weapon_max * scale]
                }
                None => [weapon_min, weapon_max],
            };
        let ap_component = attack_power / 14.0 * attack_power_multiplier;
        // C++ `Unit::CalculateMinMaxDamage`: `((weapon + baseValue) * basePct +
        // totalValue) * totalPct`; the represented `totalPct` is the
        // `UpdateDamagePctDoneMods` factor (offhand 0.5, aura 79 multiplier).
        let total_pct = projection
            .weapon_damage_pct
            .get(index)
            .copied()
            .unwrap_or(1.0);
        let total_value = projection
            .weapon_damage_flat
            .get(index)
            .copied()
            .unwrap_or(0.0);
        [
            ((weapon_min + ap_component + total_value) * total_pct).max(1.0),
            ((weapon_max + ap_component + total_value) * total_pct).max(1.0),
        ]
    })
}
