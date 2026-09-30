use wow_combat::{
    spell_advertised_coefficient_benefit_like_cpp,
    spell_base_damage_bonus_fallback_like_cpp, spell_base_healing_bonus_fallback_like_cpp,
    spell_bonus_coefficient_from_ap_like_cpp, spell_done_flat_benefit_add_ap_like_cpp,
    spell_power_override_from_ap_like_cpp,
};

#[test]
fn spell_power_ap_override_keeps_positive_half_rounding_and_attack_power_order() {
    assert_eq!(spell_power_override_from_ap_like_cpp(101, 0, 0.0, 50.0), 51);
    assert_eq!(spell_power_override_from_ap_like_cpp(100, 20, 0.25, 50.0), 75);
    assert_eq!(spell_power_override_from_ap_like_cpp(-50, 0, 0.0, 100.0), 0);
}

#[test]
fn spell_power_coefficients_keep_truncation_and_nonpositive_guard() {
    assert_eq!(spell_bonus_coefficient_from_ap_like_cpp(0.25, 18.0), 4);
    assert_eq!(spell_bonus_coefficient_from_ap_like_cpp(0.0, 18.0), 0);
    assert_eq!(spell_bonus_coefficient_from_ap_like_cpp(-1.0, 18.0), 0);
    assert_eq!(spell_bonus_coefficient_from_ap_like_cpp(f32::NAN, 18.0), 0);
    assert_eq!(spell_advertised_coefficient_benefit_like_cpp(-9, 0.5), -4);
    assert_eq!(spell_done_flat_benefit_add_ap_like_cpp(-4, 3), -1);
}

#[test]
fn spell_base_damage_fallback_filters_schools_and_invalid_stat_indices() {
    let stats = [20, 30, 40, 50, 60];
    let damage_done = [(0b001, 5), (0b100, 99), (0b011, -2)];
    let damage_from_stats = [(0b001, 0, 50), (0b010, 4, 50), (0b001, -1, 100), (0b001, 5, 100)];

    assert_eq!(
        spell_base_damage_bonus_fallback_like_cpp(
            0b011,
            100,
            &stats,
            &damage_done,
            &damage_from_stats,
        ),
        143,
    );
}

#[test]
fn spell_base_healing_fallback_keeps_zero_misc_wildcard_and_mana_gate() {
    let stats = [5, 10, 15, 40, 25];
    let healing_done = [(0, 4), (0b010, 6), (0b001, 99)];
    let healing_from_stats = [(0, 9, 50), (1, 7, 25), (-1, 0, 100), (5, 0, 100)];

    assert_eq!(
        spell_base_healing_bonus_fallback_like_cpp(
            0b010,
            20,
            100,
            &stats,
            &healing_done,
            &healing_from_stats,
        ),
        74,
    );
    assert_eq!(
        spell_base_healing_bonus_fallback_like_cpp(
            0b010,
            20,
            0,
            &stats,
            &healing_done,
            &healing_from_stats,
        ),
        34,
    );
}
