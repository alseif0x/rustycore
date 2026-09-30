//! Original derived-stat contract cases, moved without assertion changes.

mod weapon_ranges;

use super::calculate_derived_stats;
use wow_data_model::player_stats::{
    PlayerLevelStats, PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
};

#[test]
fn stat_system_uses_create_health_zero_base_mp_and_chrclasses_ap_coefficients() {
    let projection = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 12,
            stamina: 30,
            intellect: 40,
            spirit: 20,
            base_mana: 155,
        },
        class: 5,
        level: 80,
        attack_power_per_strength: 0,
        attack_power_per_agility: 0,
        ranged_attack_power_per_agility: 0,
        stat_total_multipliers: [1.0; 5],
        stat_buff_total_multipliers: [1.0; 5],
        gear_stats: [0, 0, 5, 3, 0],
        gear_health: 100,
        gear_mana: 50,
        gear_armor: 25,
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
        gear_attack_power: 17,
        gear_ranged_attack_power: 4,
        attack_power_flat_aura: 0,
        attack_power_total_pct: 1.0,
        ranged_attack_power_flat_aura: 0,
        ranged_attack_power_total_pct: 1.0,
        attack_power_override_by_spell_power_pct: None,
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    });

    assert_eq!(projection.create_health, 0);
    assert_eq!(projection.max_health, 100 + 20 + 15 * 10);
    assert_eq!(projection.base_mana, 155);
    assert_eq!(projection.max_mana, 155 + 50 + 20 + 23 * 15);
    assert_eq!(projection.armor, 12 * 2 + 25);
    assert_eq!(projection.attack_power, -20);
    assert_eq!(projection.attack_power_mod_pos, 17);
    // C++ `Unit::GetTotalAttackPowerValue` clamps the base plus modifier at
    // zero before the multiplier, so -20 + 17 yields zero.
    assert_eq!(projection.total_attack_power, 0);
    assert_eq!(projection.attack_power_multiplier, 0.0);
    assert_eq!(projection.ranged_attack_power, -10);
    assert_eq!(projection.ranged_attack_power_mod_pos, 21);
    assert_eq!(projection.total_ranged_attack_power, 11);
    assert_eq!(projection.ranged_attack_power_multiplier, 0.0);
}

#[test]
fn stat_system_applies_cpp_attack_power_aura_producers_like_cpp() {
    // C++ `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:333-403`)
    // and `Unit::GetTotalAttackPowerValue`: gear plus the
    // `MOD_ATTACK_POWER`/`MOD_RANGED_ATTACK_POWER` flats form the modifier,
    // the `..._PCT` auras form `TOTAL_PCT - 1.0`, and the total clamps the
    // base plus modifier at zero before multiplying.
    let input = PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
        class: 1,
        level: 80,
        attack_power_per_strength: 2,
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
        gear_attack_power: 50,
        gear_ranged_attack_power: 20,
        attack_power_flat_aura: 100,
        attack_power_total_pct: 1.5,
        ranged_attack_power_flat_aura: 40,
        ranged_attack_power_total_pct: 2.0,
        attack_power_override_by_spell_power_pct: None,
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    };
    let projection = calculate_derived_stats(input);

    // Class 1 (warrior): 10 Strength * 2 + (80 * 3 - 20) = 240 base AP,
    // and (80 + 10) * 0 - 10 = -10 ranged.
    assert_eq!(projection.attack_power, 240);
    assert_eq!(projection.attack_power_mod_pos, 150);
    assert_eq!(projection.attack_power_multiplier, 0.5);
    assert_eq!(projection.total_attack_power, 585);
    assert_eq!(projection.ranged_attack_power, -10);
    assert_eq!(projection.ranged_attack_power_mod_pos, 110);
    assert_eq!(projection.ranged_attack_power_multiplier, 1.0);
    assert_eq!(projection.total_ranged_attack_power, 200);
}

#[test]
fn stat_system_applies_cpp_armor_aura_producers_order_like_cpp() {
    // C++ `Player::UpdateArmor` (`StatSystem.cpp:251-276`): item BASE_VALUE
    // scaled by BASE_PCT, plus agility, plus the aura flat TOTAL_VALUE and
    // `MOD_RESISTANCE_OF_STAT_PERCENT` terms, then TOTAL_PCT and
    // `MOD_BONUS_ARMOR_PCT`, truncated by `int32(value)`.
    let projection = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 12,
            stamina: 30,
            intellect: 40,
            spirit: 20,
            base_mana: 155,
        },
        class: 5,
        level: 80,
        attack_power_per_strength: 0,
        attack_power_per_agility: 0,
        ranged_attack_power_per_agility: 0,
        stat_total_multipliers: [1.0; 5],
        stat_buff_total_multipliers: [1.0; 5],
        gear_stats: [0; 5],
        gear_health: 0,
        gear_mana: 0,
        gear_armor: 100,
        armor_base_pct: 1.5,
        armor_flat_aura: 40,
        // 50% of the final Agility (12) is added before the multipliers.
        armor_of_stat_percent: [0, 50, 0, 0, 0],
        armor_total_pct: 1.25,
        armor_bonus_pct: 1.1,
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
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    });

    // ((100 * 1.5) + 12 * 2 + 40 + 12 * 50 / 100) * 1.25 * 1.1 = 302.5
    assert_eq!(projection.armor, 302);
}

#[test]
fn stat_system_applies_cpp_avoidance_aura_percentages_like_cpp() {
    // C++ `Player::UpdateBlockPercentage`/`UpdateParryPercentage`/
    // `UpdateDodgePercentage` (`StatSystem.cpp:483-499`, `659-679`,
    // `700-717`): the aura `GetTotalAuraModifier` terms are flat
    // percentages added to the non-diminishing side.
    let warrior = PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 12,
            stamina: 30,
            intellect: 40,
            spirit: 20,
            base_mana: 0,
        },
        class: 1,
        level: 80,
        attack_power_per_strength: 2,
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
        spell_dodge_pct: 10.0,
        spell_parry_pct: 3.0,
        spell_block_pct: 7.0,
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
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: true,
        can_block: true,
    };
    let projection = calculate_derived_stats(warrior);

    // With no rating bonus the diminishing term is zero, so the result is
    // exactly the non-diminishing side.
    assert_eq!(projection.dodge_pct, 10.0);
    assert_eq!(projection.parry_pct, 8.0);
    assert_eq!(projection.block_pct, 12.0);

    // A class whose parry cap is zero keeps parry at zero even with the
    // aura and `can_parry` set, matching `UpdateParryPercentage`.
    let priest = PlayerStatSystemInputLikeCpp {
        class: 5,
        can_block: false,
        ..warrior
    };
    let projection = calculate_derived_stats(priest);
    assert_eq!(projection.parry_pct, 0.0);
}

#[test]
fn stat_system_applies_cpp_crit_aura_percentages_like_cpp() {
    // C++ `Player::UpdateAllCritPercentages`/`UpdateCritPercentage`
    // (`StatSystem.cpp:502-538`) and `UpdateSpellCritChance` (`718-731`):
    // every group starts at 5% and adds its own aura flat sum.
    let input = PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 12,
            stamina: 30,
            intellect: 40,
            spirit: 20,
            base_mana: 0,
        },
        class: 1,
        level: 80,
        attack_power_per_strength: 2,
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
        crit_mainhand_aura_pct: 2.0,
        crit_offhand_aura_pct: 3.0,
        crit_ranged_aura_pct: 4.0,
        spell_crit_aura_pct: 5.0,
        gear_attack_power: 0,
        gear_ranged_attack_power: 0,
        attack_power_flat_aura: 0,
        attack_power_total_pct: 1.0,
        ranged_attack_power_flat_aura: 0,
        ranged_attack_power_total_pct: 1.0,
        attack_power_override_by_spell_power_pct: None,
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    };
    let projection = calculate_derived_stats(input);

    assert_eq!(projection.crit_pct, 7.0);
    assert_eq!(projection.offhand_crit_pct, 8.0);
    assert_eq!(projection.ranged_crit_pct, 9.0);
    assert_eq!(projection.spell_crit_pct, [10.0; 7]);
}

#[test]
fn stat_system_uses_cpp_rating_and_diminishing_return_branches() {
    let mut rating_bonuses = [0.0; 32];
    rating_bonuses[2] = 10.0;
    rating_bonuses[3] = 10.0;
    rating_bonuses[4] = 2.0;
    rating_bonuses[8] = 3.0;
    rating_bonuses[9] = 4.0;
    rating_bonuses[10] = 5.0;

    let projection = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats::default(),
        class: 1,
        level: 80,
        attack_power_per_strength: 2,
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
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses,
        can_parry: true,
        can_block: true,
    });

    let expected_dodge = 65.631440 * 10.0 / (10.0 + 65.631440 * 0.9560);
    let expected_parry = expected_dodge + 5.0;
    assert!((projection.dodge_pct - expected_dodge).abs() < 0.00001);
    assert!((projection.parry_pct - expected_parry).abs() < 0.00001);
    assert_eq!(projection.block_pct, 7.0);
    assert_eq!(projection.crit_pct, 8.0);
    assert_eq!(projection.ranged_crit_pct, 9.0);
    assert_eq!(projection.spell_crit_pct, [10.0; 7]);
    assert_eq!(projection.dodge_from_attr, 0.0);
    assert_eq!(projection.parry_from_attr, 0.0);
}

#[test]
fn stat_system_applies_total_stat_percentage_before_dependent_stats_like_cpp() {
    let projection = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 173,
            agility: 90,
            stamina: 160,
            intellect: 98,
            spirit: 108,
            base_mana: 4_880,
        },
        class: 2,
        level: 80,
        attack_power_per_strength: 2,
        attack_power_per_agility: 0,
        ranged_attack_power_per_agility: 0,
        stat_total_multipliers: [1.03; 5],
        stat_buff_total_multipliers: [1.03; 5],
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
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    });

    assert_eq!(projection.stats, [178, 92, 164, 100, 111]);
    assert_eq!(projection.max_health, 1_460);
    assert_eq!(projection.max_mana, 6_100);
    assert_eq!(projection.armor, 184);
    assert_eq!(projection.attack_power, 576);
}

#[test]
fn stat_system_scales_positive_and_negative_client_buffs_like_cpp() {
    let projection = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 200,
            stamina: 10,
            intellect: 10,
            spirit: 10,
            base_mana: 0,
        },
        class: 1,
        level: 1,
        attack_power_per_strength: 2,
        attack_power_per_agility: 0,
        ranged_attack_power_per_agility: 0,
        stat_total_multipliers: [1.03; 5],
        stat_buff_total_multipliers: [1.03; 5],
        gear_stats: [100, -100, 0, 0, 0],
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
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    });

    assert_eq!(projection.stats[..2], [113, 103]);
    assert_eq!(projection.stat_pos_buff[..2], [103, 0]);
    assert_eq!(projection.stat_neg_buff[..2], [0, -103]);
}

#[test]
fn stat_system_overrides_attack_power_by_spell_power_like_cpp() {
    let input = PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 10,
            spirit: 10,
            base_mana: 0,
        },
        class: 1,
        level: 80,
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
        spell_bonus: PlayerSpellBonusInputLikeCpp::default(),
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    };
    let baseline = calculate_derived_stats(input);
    assert_eq!(baseline.attack_power, 220);
    assert_eq!(baseline.ranged_attack_power, -10);

    // C++ `CalculatePct(1234.0f, 12.5f)` = 154.25, truncated by the
    // `int32(base_attPower)` store; both attack mods share the base.
    let overridden = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        attack_power_override_by_spell_power_pct: Some(12.5),
        spell_bonus: PlayerSpellBonusInputLikeCpp {
            base_spell_power: 1_234,
            ..Default::default()
        },
        ..input
    });
    assert_eq!(overridden.attack_power, 154);
    assert_eq!(overridden.ranged_attack_power, 154);

    // `HasAuraType` presence alone overrides: an active effect whose summed
    // percent is zero yields `CalculatePct(spellPower, 0.0) == 0`.
    let zeroed = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        attack_power_override_by_spell_power_pct: Some(0.0),
        spell_bonus: PlayerSpellBonusInputLikeCpp {
            base_spell_power: 1_234,
            ..Default::default()
        },
        ..input
    });
    assert_eq!(zeroed.attack_power, 0);
    assert_eq!(zeroed.ranged_attack_power, 0);
}

#[test]
fn stat_system_publishes_spell_damage_and_healing_bonuses_like_cpp() {
    let input = PlayerStatSystemInputLikeCpp {
        base: PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
        class: 5,
        level: 80,
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
            base_spell_power: 100,
            // School 1 (holy) has a +30 aura; school 2 (fire) has a +20
            // aura and a -50 aura, so the C++ net sum is -30 while the
            // negative field is -50.
            damage_done_flat: [0, 30, -30, 0, 0, 0, 0],
            damage_done_neg: [0, 0, -50, 0, 0, 0, 0],
            damage_of_stat_percent: [[0; 5]; 7],
            healing_done_flat: 40,
            healing_of_stat_percent: [0; 5],
            override_spell_power_by_ap_pct: 0.0,
            // School 1 has (1 + 0.5) * (1 + 1.0) = 3.0, school 2 1.25.
            damage_done_percent: [1.0, 3.0, 1.25, 1.0, 1.0, 1.0, 1.0],
            // `UpdateHealingDonePercentMod` starts from 1.0.
            healing_done_percent: 2.0,
            // Mainhand/ranged 1.0 and offhand 0.5 with a +50% physical aura.
            weapon_damage_pct: [1.5, 0.75, 1.5],
            weapon_damage_flat: [20.0, 20.0, 0.0],
            versatility_bonus_aura: 200,
            // `ModTargetResistance = aura - item penetration`.
            target_resistance_aura: 20,
            item_spell_penetration: 15,
            target_physical_resistance_aura: 30,
        },
        rating_bonuses: [0.0; 32],
        can_parry: false,
        can_block: false,
    };
    let projection = calculate_derived_stats(input);
    assert_eq!(projection.mod_damage_done_pos[0], 0);
    // Holy: 100 base + 30 aura.
    assert_eq!(projection.mod_damage_done_pos[1], 130);
    // Fire: (100 - 30) - (-50) leaves the +20 aura.
    assert_eq!(projection.mod_damage_done_pos[2], 120);
    assert_eq!(projection.mod_damage_done_neg[2], -50);
    // Healing: 100 base + 40 aura + max(0, intellect 40).
    assert_eq!(projection.mod_healing_done_pos, 180);
    assert_eq!(
        projection.mod_damage_done_percent,
        [1.0, 3.0, 1.25, 1.0, 1.0, 1.0, 1.0]
    );
    assert_eq!(projection.mod_healing_done_percent, 2.0);
    assert_eq!(projection.mod_target_resistance, 5);
    assert_eq!(projection.mod_target_physical_resistance, 30);
    assert_eq!(projection.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(projection.override_ap_by_spell_power_percent, 0.0);
    assert_eq!(projection.versatility_bonus, 200.0);
    assert_eq!(projection.weapon_damage_pct, [1.5, 0.75, 1.5]);
    assert_eq!(projection.weapon_damage_flat, [20.0, 20.0, 0.0]);

    // `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` replaces both bonuses with
    // `int32(CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK), pct) + 0.5)`:
    // melee AP is `max(0, -20 + 500) = 480`, so 50% rounds to 240.
    // `SetUpdateFieldStatValue` clamps a negative aura sum at zero.
    let negative_versatility = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        spell_bonus: PlayerSpellBonusInputLikeCpp {
            versatility_bonus_aura: -50,
            ..input.spell_bonus
        },
        ..input
    });
    assert_eq!(negative_versatility.versatility_bonus, 0.0);

    let overridden = calculate_derived_stats(PlayerStatSystemInputLikeCpp {
        attack_power_flat_aura: 500,
        spell_bonus: PlayerSpellBonusInputLikeCpp {
            override_spell_power_by_ap_pct: 50.0,
            ..input.spell_bonus
        },
        ..input
    });
    assert_eq!(overridden.mod_damage_done_pos[1], 240);
    assert_eq!(overridden.mod_damage_done_pos[2], 290);
    assert_eq!(overridden.mod_healing_done_pos, 240);
    assert_eq!(overridden.override_spell_power_by_ap_percent, 50.0);
}
