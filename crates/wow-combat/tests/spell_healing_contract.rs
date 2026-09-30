use wow_combat::{
    SpellHealingPctDoneInputsLikeCpp, spell_healing_bonus_done_like_cpp,
    spell_healing_bonus_from_victim_aura_effects_like_cpp, spell_healing_bonus_taken_like_cpp,
    spell_healing_pct_done_like_cpp,
};

#[test]
fn spell_healing_victim_aura_filter_is_strict_while_base_healing_owns_wildcard() {
    assert_eq!(
        spell_healing_bonus_from_victim_aura_effects_like_cpp(
            20,
            0b010,
            &[(0, 30), (0b010, 7), (0b001, 100)],
        ),
        27,
    );
}

#[test]
fn spell_healing_pct_keeps_state_and_missing_health_multipliers_ordered() {
    let actual = spell_healing_pct_done_like_cpp(SpellHealingPctDoneInputsLikeCpp {
        gated: false,
        healing_done_percent: 1.25,
        target_aura_state_mask: 1 << 1,
        damage_done_versus_aura_state: &[(2, 20), (1, 99), (-1, 99), (33, 99)],
        healing_done_pct_versus_target_health: &[40, -20],
        target_health_pct: Some(25.0),
    });
    let expected = ((1.25_f32 * 1.2) * 1.3) * 0.85;
    assert!((actual - expected).abs() < f32::EPSILON);

    assert_eq!(
        spell_healing_pct_done_like_cpp(SpellHealingPctDoneInputsLikeCpp {
            gated: false,
            healing_done_percent: 0.8,
            target_aura_state_mask: 0,
            damage_done_versus_aura_state: &[],
            healing_done_pct_versus_target_health: &[100],
            target_health_pct: None,
        }),
        0.8,
    );
    assert_eq!(
        spell_healing_pct_done_like_cpp(SpellHealingPctDoneInputsLikeCpp {
            gated: false,
            healing_done_percent: 1.0,
            target_aura_state_mask: 0,
            damage_done_versus_aura_state: &[],
            healing_done_pct_versus_target_health: &[100],
            target_health_pct: Some(125.0),
        }),
        1.0,
    );
}

#[test]
fn spell_healing_pct_gate_short_circuits_all_modifier_inputs() {
    assert_eq!(
        spell_healing_pct_done_like_cpp(SpellHealingPctDoneInputsLikeCpp {
            gated: true,
            healing_done_percent: 4.0,
            target_aura_state_mask: u32::MAX,
            damage_done_versus_aura_state: &[(1, 500)],
            healing_done_pct_versus_target_health: &[500],
            target_health_pct: Some(0.0),
        }),
        1.0,
    );
}

#[test]
fn spell_healing_taken_uses_min_negative_then_max_positive_and_clamps() {
    assert_eq!(
        spell_healing_bonus_taken_like_cpp(100, &[-30, -10, 20, 5]),
        84
    );
    assert_eq!(spell_healing_bonus_taken_like_cpp(10, &[-200, -150]), 0);
    assert_eq!(spell_healing_bonus_taken_like_cpp(u32::MAX, &[]), u32::MAX);
}

#[test]
fn spell_healing_final_application_clamps_below_zero_and_at_u32_max() {
    assert_eq!(spell_healing_bonus_done_like_cpp(10, -20, 1.0), 0);
    assert_eq!(
        spell_healing_bonus_done_like_cpp(u32::MAX, 0, 1.0),
        u32::MAX
    );
}
