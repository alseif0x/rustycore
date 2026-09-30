use wow_combat::{
    SpellDamagePctDoneInputsLikeCpp, spell_damage_bonus_done_like_cpp,
    spell_damage_pct_done_like_cpp,
};

#[test]
fn spell_damage_pct_applies_ordered_school_aura_mechanic_and_script_terms() {
    let school_percentages = [0.1, 0.2, 0.5, 0.4, 0.0, 0.0, 0.0];
    let damage_done_versus = [(0b010, 10), (0b011, 20), (0b001, 99)];
    let damage_done_versus_aura_state = [(2, 10), (2, -20), (1, 99), (0, 50), (-1, 50), (33, 50)];
    let damage_percent_by_target_mechanic = [(4, 10), (4, -5), (2, 99), (64, 50)];
    let damage_done_for_mechanic = [(7, 10), (7, -5), (6, 99)];

    let actual = spell_damage_pct_done_like_cpp(SpellDamagePctDoneInputsLikeCpp {
        school_mask: 0b1010,
        school_percentages: &school_percentages,
        creature_type_mask: 0b010,
        damage_done_versus: &damage_done_versus,
        target_aura_state_mask: 1 << 1,
        damage_done_versus_aura_state: &damage_done_versus_aura_state,
        target_mechanic_mask: 1 << 4,
        damage_percent_done_by_target_aura_mechanic: &damage_percent_by_target_mechanic,
        spell_mechanic: Some(7),
        damage_done_for_mechanic: Some(&damage_done_for_mechanic),
        scripted_factor: 3.0,
    });
    let expected = ((((((((0.4_f32 * 1.1) * 1.2) * 1.1) * 0.8) * 1.1) * 0.95) * 1.05) * 3.0);
    assert!((actual - expected).abs() < f32::EPSILON);
}

#[test]
fn spell_damage_pct_gate_and_invalid_bits_fail_closed() {
    let no_terms: [(i32, i32); 0] = [];
    assert_eq!(
        spell_damage_pct_done_like_cpp(SpellDamagePctDoneInputsLikeCpp {
            school_mask: 0,
            school_percentages: &[0.0; 7],
            creature_type_mask: 0,
            damage_done_versus: &no_terms,
            target_aura_state_mask: u32::MAX,
            damage_done_versus_aura_state: &[(-1, 50), (0, 50), (33, 50)],
            target_mechanic_mask: u64::MAX,
            damage_percent_done_by_target_aura_mechanic: &[(0, 50), (-1, 50), (64, 50)],
            spell_mechanic: Some(0),
            damage_done_for_mechanic: Some(&[(0, 100)]),
            scripted_factor: 1.0,
        }),
        0.0,
    );
}

#[test]
fn spell_damage_final_application_clamps_below_zero_and_at_u32_max() {
    assert_eq!(spell_damage_bonus_done_like_cpp(10, -20, 1.0), 0);
    assert_eq!(spell_damage_bonus_done_like_cpp(u32::MAX, 0, 1.0), u32::MAX);
}
