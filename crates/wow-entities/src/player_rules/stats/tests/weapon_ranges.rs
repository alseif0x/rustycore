//! Original weapon-range integration contract.

use super::super::calculate_derived_stats as calculate_player_stat_system_like_cpp;
use wow_data::effective_weapon_damage_ranges_like_cpp;
use wow_data_model::player_stats::{
    PlayerLevelStats, PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
};

#[test]
fn weapon_damage_ranges_apply_flat_before_the_total_pct_like_cpp() {
    let input = PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats::default(),
        class: 1,
        // Level 1 keeps the class-specific attack-power term negative, so
        // `GetTotalAttackPowerValue` clamps to zero and the range isolates
        // the flat/percentage arithmetic.
        level: 1,
        attack_power_per_strength: 0,
        attack_power_per_agility: 0,
        ranged_attack_power_per_agility: 0,
        stat_total_multipliers: [1.0; 5],
        stat_buff_total_multipliers: [1.0; 5],
        gear_stats: [0; 5],
        gear_health: 0,
        gear_mana: 0,
        gear_armor: 0,
        armor_base_pct: 1.0,
        armor_flat_aura: 0,
        armor_of_stat_percent: [0; 5],
        armor_total_pct: 1.0,
        armor_bonus_pct: 1.0,
        spell_dodge_pct: 0.0,
        spell_parry_pct: 0.0,
        spell_block_pct: 0.0,
        crit_mainhand_aura_pct: 0.0,
        crit_offhand_aura_pct: 0.0,
        crit_ranged_aura_pct: 0.0,
        spell_crit_aura_pct: 0.0,
        gear_attack_power: 0,
        gear_ranged_attack_power: 0,
        attack_power_flat_aura: 0,
        attack_power_total_pct: 1.0,
        ranged_attack_power_flat_aura: 0,
        ranged_attack_power_total_pct: 1.0,
        attack_power_override_by_spell_power_pct: None,
        spell_bonus: PlayerSpellBonusInputLikeCpp {
            weapon_damage_pct: [1.5, 0.75, 1.0],
            weapon_damage_flat: [20.0, 40.0, 0.0],
            ..Default::default()
        },
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    };
    let projection = calculate_player_stat_system_like_cpp(input);
    // `CalculateMinMaxDamage`: `((weapon 10 + ap 0) * 1.0 + flat) * pct`.
    let ranges =
        effective_weapon_damage_ranges_like_cpp(projection, [[10.0, 20.0]; 3], [2_000; 3], None);
    assert_eq!(ranges[0], [45.0, 60.0]);
    assert_eq!(ranges[1], [37.5, 45.0]);
    assert_eq!(ranges[2], [10.0, 20.0]);

    // C++ `Player::CalculateMinMaxDamage` (`StatSystem.cpp:461-467`) rescales
    // the base weapon damage by `CombatRoundTime / 1000 / GetAPMultiplier`
    // while a feral form is active. With a 2 s delay the multiplier is 2.0,
    // so a 1000 ms round time halves the weapon part before the
    // attack-power term is added back.
    let feral = effective_weapon_damage_ranges_like_cpp(
        projection,
        [[20.0, 40.0]; 3],
        [2_000; 3],
        Some(1_000.0),
    );
    assert_eq!(feral[0], [45.0, 60.0]);
    // A zero round time is the unshaped branch.
    let unchanged = effective_weapon_damage_ranges_like_cpp(
        projection,
        [[20.0, 40.0]; 3],
        [2_000; 3],
        Some(0.0),
    );
    assert_eq!(unchanged[0], [60.0, 90.0]);
}
