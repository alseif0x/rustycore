// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

pub fn represented_player_stat_changes_like_cpp(
    state: &wow_entities::PlayerItemBonusStateLikeCpp,
) -> wow_packet::packets::update::PlayerStatChanges {
    let mut changes = wow_packet::packets::update::PlayerStatChanges {
        base_mana: state.mana_base,
        base_health: state.health_base,
        attack_power: state.attack_power_total,
        ranged_attack_power: state.ranged_attack_power_total,
        stats: state.stats_base,
        stat_pos_buff: state.stats_base,
        armor: state.armor_base + state.armor_total + state.resistances_base[0],
        combat_ratings: state.combat_ratings,
        // This fixture has no aura/stat producers, so the item accumulator is
        // the whole represented `SpellBaseDamageBonusDone`/`HealingBonusDone`.
        mod_damage_done_pos: std::array::from_fn(|school| {
            if school == 0 {
                0
            } else {
                state.spell_power_bonus
            }
        }),
        mod_damage_done_neg: [0; 7],
        mod_healing_done_pos: state.spell_power_bonus,
        mod_damage_done_percent: [1.0; 7],
        shield_block: i32::try_from(state.shield_block_value).unwrap_or(i32::MAX),
        ..Default::default()
    };

    changes.min_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][0];
    changes.max_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][1];
    changes.min_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][0];
    changes.max_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][1];
    changes
}
