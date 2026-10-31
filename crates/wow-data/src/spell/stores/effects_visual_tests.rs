use super::*;

fn override_row(target_spell_id: u32) -> crate::spell_db2::SpellEffectDb2Entry {
    crate::spell_db2::SpellEffectDb2Entry {
        id: 10,
        difficulty_id: 3,
        effect_index: 0,
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 403,
        effect_aura_period: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [target_spell_id as i32, 0],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id: 90_001,
    }
}

#[test]
fn visual_metadata_query_keeps_a_retained_nonbase_projection_after_rehydration() {
    let target_spell_id = 8_326;
    let mut store = SpellStore::new();
    store.hydrate_spell_effect_rows(&crate::spell_db2::SpellEffectDb2Store::from_entries([
        override_row(target_spell_id),
    ]));
    assert!(store.spells.is_empty(), "the source row is nonbase-only");
    assert!(store.may_override_spell_visual(target_spell_id));

    store.hydrate_spell_effect_rows(&crate::spell_db2::SpellEffectDb2Store::from_entries([]));
    assert!(store.regular_visual_override_targets.is_empty());
    assert!(store.spells.is_empty());
    assert!(
        store.may_override_spell_visual(target_spell_id),
        "the existing canonical projection still owns the retained nonbase slot"
    );
}
