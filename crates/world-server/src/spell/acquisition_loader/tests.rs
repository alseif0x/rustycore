use super::*;
use wow_data::DifficultyEntry;

#[test]
fn typed_spell_effect_overlay_preserves_raw_integer_and_float_domains() {
    let row = wow_persistence::SpellEffectHotfixPersistenceRowLikeCpp {
        record_id: Some(7),
        difficulty_id: Some(-1),
        effect_index: None,
        effect: Some(36),
        effect_base_points: Some(i64::MIN),
        effect_die_sides: Some(1),
        effect_trigger_spell: Some(42),
        effect_misc_value: [Some(-9), None],
        implicit_target: [Some(1), Some(21)],
        coefficient_bits: Some(f32::NAN.to_bits()),
        variance_bits: Some((-0.0_f32).to_bits()),
        spell_id: Some(100),
        effect_chain_targets: Some(3),
        effect_points_per_resource_bits: Some(1.5_f32.to_bits()),
        effect_real_points_per_level_bits: None,
        effect_item_type: Some(-2),
        effect_aura: Some(4),
        effect_mechanic: Some(5),
        effect_attributes: Some(-1),
    };
    let bridged = overlay_row_like_cpp(
        SpellAcquisitionTableLikeCpp::SpellEffect,
        SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellEffect(row),
    )
    .unwrap();

    assert_eq!(bridged.integer_columns[0], Some(7));
    assert_eq!(bridged.integer_columns[2], None);
    assert_eq!(bridged.integer_columns[17], Some(-2));
    assert_eq!(bridged.float_columns_bits[11], Some(f32::NAN.to_bits()));
    assert_eq!(bridged.float_columns_bits[12], Some((-0.0_f32).to_bits()));
    assert_eq!(bridged.float_columns_bits[16], None);
}

#[test]
fn typed_overlay_rejects_a_row_from_the_wrong_source_family() {
    let row = wow_persistence::SpellLearnSpellHotfixPersistenceRowLikeCpp {
        record_id: Some(1),
        spell_id: Some(2),
        learn_spell_id: Some(3),
        overrides_spell_id: Some(0),
    };
    assert!(
        overlay_row_like_cpp(
            SpellAcquisitionTableLikeCpp::SpellEffect,
            SpellAcquisitionHotfixPersistenceRowLikeCpp::SpellLearnSpell(row),
        )
        .is_err()
    );
}

#[test]
fn startup_outcome_keeps_empty_success_distinct_from_failure() {
    let empty = loaded_startup_like_cpp::<Vec<u32>>(
        SpellAcquisitionStartupLoadOutcomeLikeCpp::Loaded(Vec::new()),
    )
    .unwrap();
    assert!(empty.is_empty());
    assert!(
        loaded_startup_like_cpp::<Vec<u32>>(SpellAcquisitionStartupLoadOutcomeLikeCpp::Failed {
            reason: "world read failed".to_string(),
        },)
        .is_err()
    );
}

fn difficulty(id: u32, fallback_difficulty_id: u8) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

fn acquisition_effect(
    record_id: u32,
    effect_index: i64,
    effect_type: u32,
    misc_value: i64,
    trigger_spell: i64,
) -> SpellAcquisitionEffectLikeCpp {
    SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: 100,
        difficulty_id_raw: 0,
        effect_index_raw: effect_index,
        effect_type_raw: i64::from(effect_type),
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0f32.to_bits(),
        effect_real_points_per_level_bits: 0.0f32.to_bits(),
        effect_coefficient_bits: 0.0f32.to_bits(),
        effect_variance_bits: 0.0f32.to_bits(),
        effect_trigger_spell_raw: trigger_spell,
        effect_item_type_raw: 0,
        effect_misc_value_raw: [misc_value, 0],
        implicit_target_raw: [0, 0],
    }
}

#[test]
fn fallback_chain_keeps_final_missing_lookup_like_cpp() {
    let store = DifficultyStore::from_entries([difficulty(5, 4), difficulty(4, 3)]);

    assert_eq!(difficulty_chain_like_cpp(&store, 5), vec![5, 4, 3]);
}

#[test]
fn fallback_chain_stops_invalid_cycle() {
    let store = DifficultyStore::from_entries([difficulty(5, 4), difficulty(4, 5)]);

    assert_eq!(difficulty_chain_like_cpp(&store, 5), vec![5, 4]);
}

#[test]
fn proven_talent_wins_over_unknown_custom_variant() {
    assert_eq!(
        effective_talent_like_cpp(false, true, SpellAcquisitionTalentLookupLikeCpp::Talent,),
        Some(true)
    );
    assert_eq!(
        effective_talent_like_cpp(false, true, SpellAcquisitionTalentLookupLikeCpp::NotTalent,),
        None
    );
    assert_eq!(
        effective_talent_like_cpp(true, true, SpellAcquisitionTalentLookupLikeCpp::NotTalent,),
        Some(true)
    );
}

#[test]
fn custom_talent_uncertainty_uses_rejected_attribute_bits() {
    let errors = [
        wow_data::SpellCustomAttributeLoadErrorLikeCpp {
            spell_id: 100,
            difficulty: Some(0),
            attributes: wow_data::SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
            kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
        },
        wow_data::SpellCustomAttributeLoadErrorLikeCpp {
            spell_id: 100,
            difficulty: Some(2),
            attributes: wow_data::SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP
                | SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
            kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
        },
    ];

    assert_eq!(
        custom_talent_unknown_keys_like_cpp(&errors),
        BTreeSet::from([(100, 2)])
    );
}

#[test]
fn proven_talent_is_published_for_every_exact_variant_like_cpp() {
    let mut store = SpellCustomAttributeStoreLikeCpp::default();

    assert_eq!(
        apply_proven_talent_to_variants_like_cpp(&mut store, 100, [0, 2]),
        2
    );
    assert_ne!(
        store.attributes_for_spell_difficulty_like_cpp(100, 0) & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
        0
    );
    assert_ne!(
        store.attributes_for_spell_difficulty_like_cpp(100, 2) & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
        0
    );
}

#[test]
fn learn_skill_projection_stops_at_first_matching_effect_like_cpp() {
    let unrelated = acquisition_effect(1, 0, 0, 0, 0);
    let dual_wield = acquisition_effect(2, 1, SPELL_EFFECT_DUAL_WIELD, 0, 0);
    let mut skill = acquisition_effect(3, 2, SPELL_EFFECT_SKILL, -1, 0);
    skill.effect_base_points_raw = i64::from(i32::MAX) + 1;

    let LearnSkillProjectionLikeCpp::Covered(source) =
        project_learn_skill_source_like_cpp(100, &[&unrelated, &dual_wield, &skill])
    else {
        panic!("the first qualifying dual-wield effect must remain covered");
    };

    assert_eq!(
        source.effects,
        vec![SpellLearnSkillEffectLikeCpp {
            effect: SPELL_EFFECT_DUAL_WIELD,
            misc_value: 0,
            calc_value: 0,
        }]
    );
}

#[test]
fn learn_skill_projection_rejects_zero_skill_id() {
    let skill = acquisition_effect(1, 0, SPELL_EFFECT_SKILL, 0, 0);

    assert_eq!(
        project_learn_skill_source_like_cpp(100, &[&skill]),
        LearnSkillProjectionLikeCpp::Indeterminate(
            SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                record_id: 1,
                field: "SpellEffect.EffectMiscValue",
                raw: 0,
            }
        )
    );
}

#[test]
fn rng_dependent_first_skill_is_explicit_and_does_not_fall_through() {
    let mut ranged_skill = acquisition_effect(1, 0, SPELL_EFFECT_SKILL, 755, 0);
    ranged_skill.effect_base_points_raw = 4;
    ranged_skill.effect_die_sides_raw = 3;
    let dual_wield = acquisition_effect(2, 1, SPELL_EFFECT_DUAL_WIELD, 0, 0);

    assert_eq!(
        project_learn_skill_source_like_cpp(100, &[&ranged_skill, &dual_wield]),
        LearnSkillProjectionLikeCpp::Indeterminate(
            SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                record_id: 1,
                domain: wow_data::AcquisitionValueDomainLikeCpp {
                    minimum: 5,
                    maximum: 7,
                },
            }
        )
    );
}

#[test]
fn covered_spell_without_qualifying_effect_stays_distinct_from_indeterminate() {
    let unrelated = acquisition_effect(1, 0, 0, 0, 0);

    assert_eq!(
        project_learn_skill_source_like_cpp(100, &[&unrelated]),
        LearnSkillProjectionLikeCpp::Covered(SpellLearnSkillSourceSpellInfoLikeCpp {
            spell_id: 100,
            difficulty_none: true,
            effects: Vec::new(),
        })
    );
}

#[test]
fn learn_spell_projection_keeps_skill_step_and_checked_trigger_like_cpp() {
    let skill_step = acquisition_effect(1, 0, SPELL_EFFECT_SKILL_STEP, 0, 0);
    let learn_spell = acquisition_effect(2, 1, SPELL_EFFECT_LEARN_SPELL, 0, 200);

    let source =
        project_learn_spell_source_like_cpp(100, false, true, &[&skill_step, &learn_spell])
            .unwrap();

    assert!(source.has_skill_step_effect);
    assert!(source.is_passive);
    assert_eq!(
        source.learn_spell_effects,
        vec![SpellLearnSpellEffectLikeCpp {
            trigger_spell: 200,
            target_unit_pet: false,
        }]
    );
}
