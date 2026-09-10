//! Creature template regressions, part 1 of 2.
//!
//! Moved out of the creature_template.rs root under #664; every test is unchanged.

use super::*;

#[test]
fn creature_classification_store_maps_template_entries_like_cpp() {
    let store = CreatureTemplateClassificationStoreLikeCpp::from_entries([(100, 0), (101, 4)]);
    assert_eq!(store.len(), 2);
    assert!(!store.is_empty());
    assert_eq!(store.classification_for_entry(100), Some(0));
    assert_eq!(store.classification_for_entry(101), Some(4));
    assert_eq!(store.classification_for_entry(999), None);
}

#[test]
fn creature_template_lifecycle_store_preserves_cpp_field_mapping_and_vehicle_id() {
    let store = CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        CreatureTemplateLifecycleRecordLikeCpp {
            entry: 42,
            name: "C++ Template".to_string(),
            ai_name: "SmartAI".to_string(),
            script_name: "npc_cpp_template".to_string(),
            required_expansion: 2,
            faction: 35,
            npc_flags: 0x1_0000_0040,
            speed_walk: 1.0,
            speed_run: 1.14286,
            scale: 1.25,
            classification: 4,
            damage_school: wow_constants::spell::SpellSchools::Fire as u8,
            unit_flags: 0x0000_0200,
            unit_flags2: 0x0000_0800,
            unit_flags3: 0x0000_0002,
            creature_type: 7,
            family: 0,
            trainer_class: 8,
            unit_class: 2,
            vehicle_id: 900,
            movement_type: 1,
            ground_movement_type: CreatureGroundMovementType::Hover as u8,
            swim_allowed: false,
            flight_movement_type: CreatureFlightMovementType::CanFly as u8,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: CreatureRandomMovementType::AlwaysRun as u8,
            interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            flags_extra: 0x20,
            string_id: "template_string".to_string(),
            regen_health: true,
            spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ]);

    let template = store.get(42).expect("template row retained");
    assert_eq!(template.name, "C++ Template");
    assert_eq!(template.ai_name, "SmartAI");
    assert_eq!(template.script_name, "npc_cpp_template");
    assert_eq!(template.required_expansion, 2);
    assert_eq!(template.faction, 35);
    assert_eq!(template.npc_flags, 0x1_0000_0040);
    assert_eq!(template.speed_walk, 1.0);
    assert_eq!(template.speed_run, 1.14286);
    assert_eq!(template.scale, 1.25);
    assert_eq!(template.classification, 4);
    assert_eq!(
        template.damage_school,
        wow_constants::spell::SpellSchools::Fire as u8
    );
    assert_eq!(template.unit_flags, 0x0000_0200);
    assert_eq!(template.unit_flags2, 0x0000_0800);
    assert_eq!(template.unit_flags3, 0x0000_0002);
    assert_eq!(template.creature_type, 7);
    assert_eq!(template.trainer_class, 8);
    assert_eq!(template.unit_class, 2);
    assert_eq!(template.vehicle_id, 900);
    assert_eq!(template.movement_type, 1);
    assert_eq!(
        template.ground_movement_type,
        CreatureGroundMovementType::Hover as u8
    );
    assert!(!template.swim_allowed);
    assert_eq!(
        template.flight_movement_type,
        CreatureFlightMovementType::CanFly as u8
    );
    assert_eq!(
        template.random_movement_type,
        CreatureRandomMovementType::AlwaysRun as u8
    );
    assert_eq!(template.flags_extra, 0x20);
    assert_eq!(template.string_id, "template_string");
    assert!(template.regen_health);
}

#[test]
fn creature_template_lifecycle_store_removes_npc_flag_for_entries_like_cpp() {
    let spellclick_flag = 0x0100_0000_u64;
    let other_flag = 0x2_u64;
    let mut with_spellclick = creature_template_lifecycle_record_for_test(100);
    with_spellclick.npc_flags = spellclick_flag | other_flag;
    let mut without_spellclick = creature_template_lifecycle_record_for_test(101);
    without_spellclick.npc_flags = other_flag;
    let mut untouched = creature_template_lifecycle_record_for_test(102);
    untouched.npc_flags = spellclick_flag | 0x4;

    let mut store = CreatureTemplateLifecycleStoreLikeCpp::from_templates([
        with_spellclick,
        without_spellclick,
        untouched,
    ]);

    assert_eq!(
        store.remove_npc_flag_for_entries_like_cpp([100, 101, 999], spellclick_flag),
        1
    );
    assert_eq!(store.get(100).unwrap().npc_flags, other_flag);
    assert_eq!(store.get(101).unwrap().npc_flags, other_flag);
    assert_eq!(store.get(102).unwrap().npc_flags, spellclick_flag | 0x4);
}

#[test]
fn creature_template_lifecycle_normalizes_invalid_flight_like_cpp() {
    let mut invalid = CreatureTemplateLifecycleRecordLikeCpp {
        entry: 43,
        name: "invalid flight".to_string(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: MAX_EXPANSIONS_LIKE_CPP,
        faction: 35,
        npc_flags: 0,
        speed_walk: 1.0,
        speed_run: 1.0,
        scale: 1.0,
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        creature_type: 0,
        family: 0,
        trainer_class: 0,
        unit_class: 1,
        vehicle_id: 0,
        movement_type: 0,
        ground_movement_type: 0,
        swim_allowed: true,
        flight_movement_type: CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        flags_extra: 0,
        string_id: String::new(),
        regen_health: true,
        spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
        models: Vec::new(),
    };
    invalid = invalid.normalize_like_cpp();

    assert_eq!(
        invalid.flight_movement_type,
        CreatureFlightMovementType::None as u8
    );
    assert_eq!(invalid.required_expansion, 0);
}

#[test]
fn creature_template_lifecycle_normalizes_invalid_random_like_cpp() {
    let mut invalid = creature_template_lifecycle_record_for_test(44);
    invalid.random_movement_type = CREATURE_RANDOM_MOVEMENT_TYPE_MAX_LIKE_CPP;

    invalid = invalid.normalize_like_cpp();

    assert_eq!(
        invalid.random_movement_type,
        CreatureRandomMovementType::Walk as u8
    );
}

#[test]
fn creature_template_lifecycle_normalizes_invalid_chase_like_cpp() {
    let mut invalid = creature_template_lifecycle_record_for_test(45);
    invalid.chase_movement_type = CREATURE_CHASE_MOVEMENT_TYPE_MAX_LIKE_CPP;

    invalid = invalid.normalize_like_cpp();

    assert_eq!(
        invalid.chase_movement_type,
        CreatureChaseMovementType::Run as u8
    );
}

#[test]
fn creature_template_lifecycle_normalizes_zero_speeds_like_cpp() {
    let template = CreatureTemplateLifecycleRecordLikeCpp {
        entry: 44,
        name: "zero speeds".to_string(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 0,
        faction: 35,
        npc_flags: 0,
        speed_walk: 0.0,
        speed_run: 0.0,
        scale: 1.0,
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        creature_type: 0,
        family: 0,
        trainer_class: 0,
        unit_class: 1,
        vehicle_id: 0,
        movement_type: 0,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: CreatureFlightMovementType::None as u8,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        flags_extra: 0,
        string_id: String::new(),
        regen_health: true,
        spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
        models: Vec::new(),
    };

    let store = CreatureTemplateLifecycleStoreLikeCpp::from_templates([template]);
    let normalized = store.get(44).expect("template row retained");

    assert_eq!(
        normalized.speed_walk, 1.0,
        "C++ ObjectMgr::CheckCreatureTemplate forces zero speed_walk to 1.0"
    );
    assert_eq!(
        normalized.speed_run, 1.14286,
        "C++ ObjectMgr::CheckCreatureTemplate forces zero speed_run to 1.14286"
    );
}

#[test]
fn creature_template_lifecycle_spells_skip_oob_and_missing_template_like_cpp() {
    let mut present = CreatureTemplateLifecycleRecordLikeCpp {
        entry: 7,
        name: String::new(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 0,
        faction: 0,
        npc_flags: 0,
        speed_walk: 0.0,
        speed_run: 0.0,
        scale: 1.0,
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        creature_type: 0,
        family: 0,
        trainer_class: 0,
        unit_class: 0,
        vehicle_id: 0,
        movement_type: 0,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        flags_extra: 0,
        string_id: String::new(),
        regen_health: false,
        spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
        models: Vec::new(),
    };
    present.apply_spell_row_like_cpp(0, 100);
    present.apply_spell_row_like_cpp(7, 700);
    present.apply_spell_row_like_cpp(8, 800);

    let store = CreatureTemplateLifecycleStoreLikeCpp::from_templates([present]);
    let template = store.get(7).expect("present template");
    assert_eq!(template.spells, [100, 0, 0, 0, 0, 0, 0, 700]);
    assert!(store.get(999).is_none());
}

#[test]
fn creature_template_lifecycle_models_preserve_order_and_first_valid_like_cpp() {
    let mut template = CreatureTemplateLifecycleRecordLikeCpp {
        entry: 8,
        name: String::new(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 0,
        faction: 0,
        npc_flags: 0,
        speed_walk: 0.0,
        speed_run: 0.0,
        scale: 1.0,
        classification: 0,
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        creature_type: 0,
        family: 0,
        trainer_class: 0,
        unit_class: 0,
        vehicle_id: 0,
        movement_type: 0,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        flags_extra: 0,
        string_id: String::new(),
        regen_health: false,
        spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
        models: Vec::new(),
    };
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 0,
        display_scale: 9.9,
        probability: 100.0,
    });
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 111,
        display_scale: 1.0,
        probability: 25.0,
    });
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 222,
        display_scale: 2.0,
        probability: 75.0,
    });

    assert_eq!(template.models.len(), 2);
    assert_eq!(template.models[0].creature_display_id, 111);
    assert_eq!(template.models[1].creature_display_id, 222);
    assert_eq!(
        template.first_model_like_cpp(),
        Some(CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: 111,
            display_scale: 1.0,
            probability: 25.0,
        })
    );
}

#[test]
fn creature_template_choose_display_model_uses_weighted_model_like_cpp() {
    let mut template = creature_template_lifecycle_record_for_test(80);
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 111,
        display_scale: 1.0,
        probability: 25.0,
    });
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 222,
        display_scale: 2.0,
        probability: 75.0,
    });
    let model_info_store = model_info_store_for_display_selection_tests_like_cpp([
        model_info_like_cpp(111, 0, false),
        model_info_like_cpp(222, 0, false),
    ]);
    let mut random = FixedCreatureModelRandomLikeCpp {
        weighted_roll: 25.01,
        other_gender_zero: false,
    };

    let model = template
        .choose_display_model_like_cpp(&model_info_store, None, &mut random)
        .expect("weighted C++ model should resolve");

    assert_eq!(model.creature_display_id, 222);
    assert_eq!(model.display_scale, 2.0);
}

#[test]
fn creature_template_choose_display_model_uses_invisible_trigger_fallback_like_cpp() {
    let mut template = creature_template_lifecycle_record_for_test(81);
    template.flags_extra = CreatureFlagsExtra::TRIGGER.bits();
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 21_955,
        display_scale: 1.0,
        probability: 100.0,
    });
    let model_info_store = model_info_store_for_display_selection_tests_like_cpp([
        model_info_like_cpp(21_955, 0, false),
        model_info_like_cpp(DEFAULT_INVISIBLE_CREATURE_DISPLAY_ID_LIKE_CPP, 0, true),
    ]);
    let mut random = FixedCreatureModelRandomLikeCpp {
        weighted_roll: 0.0,
        other_gender_zero: false,
    };

    let model = template
        .choose_display_model_like_cpp(&model_info_store, None, &mut random)
        .expect("trigger template should resolve the C++ invisible fallback");

    assert_eq!(
        model.creature_display_id,
        DEFAULT_INVISIBLE_CREATURE_DISPLAY_ID_LIKE_CPP
    );
    assert_eq!(model.display_scale, 1.0);
}

#[test]
fn creature_template_choose_display_model_applies_other_gender_like_cpp() {
    let mut template = creature_template_lifecycle_record_for_test(82);
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 111,
        display_scale: 1.0,
        probability: 100.0,
    });
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 222,
        display_scale: 2.0,
        probability: 100.0,
    });
    let model_info_store = model_info_store_for_display_selection_tests_like_cpp([
        model_info_like_cpp(111, 222, false),
        model_info_like_cpp(222, 0, false),
    ]);
    let mut random = FixedCreatureModelRandomLikeCpp {
        weighted_roll: 0.0,
        other_gender_zero: true,
    };

    let model = template
        .choose_display_model_like_cpp(&model_info_store, None, &mut random)
        .expect("other-gender model should resolve");

    assert_eq!(model.creature_display_id, 222);
    assert_eq!(
        model.display_scale, 2.0,
        "C++ replaces the full CreatureModel when the other-gender display exists in the template"
    );
}

#[test]
fn creature_template_lifecycle_models_normalize_non_positive_display_scale_like_cpp() {
    let mut template = CreatureTemplateLifecycleRecordLikeCpp {
        entry: 9,
        name: String::new(),
        ai_name: String::new(),
        script_name: String::new(),
        required_expansion: 0,
        faction: 0,
        npc_flags: 0,
        speed_walk: 0.0,
        speed_run: 0.0,
        scale: 1.0,
        classification: 0,
        damage_school: MAX_SPELL_SCHOOL_LIKE_CPP,
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        creature_type: 0,
        family: 0,
        trainer_class: 0,
        unit_class: 0,
        vehicle_id: 0,
        movement_type: 0,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: 0,
        rooted: false,
        chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        flags_extra: 0,
        string_id: String::new(),
        regen_health: false,
        spells: [0; MAX_CREATURE_SPELLS_LIKE_CPP],
        models: vec![CreatureTemplateLifecycleModelLikeCpp {
            creature_display_id: 333,
            display_scale: -2.0,
            probability: 1.0,
        }],
    };
    let normalized_store =
        CreatureTemplateLifecycleStoreLikeCpp::from_templates([template.clone()]);
    assert_eq!(
        normalized_store.get(9).expect("template").models[0].display_scale,
        1.0
    );
    assert_eq!(
        normalized_store.get(9).expect("template").damage_school,
        wow_constants::spell::SpellSchools::Normal as u8,
        "C++ ObjectMgr clamps invalid creature_template.dmgschool to SPELL_SCHOOL_NORMAL"
    );

    template.models.clear();
    template.push_model_like_cpp(CreatureTemplateLifecycleModelLikeCpp {
        creature_display_id: 444,
        display_scale: 0.0,
        probability: 1.0,
    });
    assert_eq!(template.models[0].display_scale, 1.0);
}

#[test]
fn creature_template_sparring_store_validates_rows_like_cpp() {
    let store = CreatureTemplateSparringStoreLikeCpp::from_rows_like_cpp(
        [
            (10, 35.5),
            (10, 75.0),
            (11, 20.0),
            (12, 0.0),
            (13, -5.0),
            (14, 100.1),
        ],
        |entry| matches!(entry, 10 | 12 | 13 | 14),
    );

    assert_eq!(store.len(), 2);
    assert_eq!(store.values_for_entry_like_cpp(10), Some(&[35.5, 75.0][..]));
    assert_eq!(store.values_for_entry_like_cpp(11), None);
    assert_eq!(store.values_for_entry_like_cpp(12), None);
    assert_eq!(store.values_for_entry_like_cpp(13), None);
    assert_eq!(store.values_for_entry_like_cpp(14), None);
}

#[test]
fn creature_template_sparring_selection_preserves_float_percent_like_cpp() {
    let store = CreatureTemplateSparringStoreLikeCpp::from_rows_like_cpp(
        [(10, 35.5), (10, 75.25)],
        |entry| entry == 10,
    );

    assert_eq!(store.select_for_entry_by_index_like_cpp(10, 0), Some(35.5));
    assert_eq!(store.select_for_entry_by_index_like_cpp(10, 1), Some(75.25));
    assert_eq!(store.select_for_entry_by_index_like_cpp(10, 2), Some(35.5));
    assert_eq!(store.select_for_entry_by_index_like_cpp(999, 0), None);
}

#[test]
fn creature_classification_damage_rates_match_cpp_switch_and_default_elite() {
    let rates = CreatureClassificationDamageRatesLikeCpp {
        normal: 1.0,
        elite: 2.0,
        rare_elite: 3.0,
        obsolete: 4.0,
        rare: 5.0,
        trivial: 6.0,
        minus_mob: 7.0,
    };

    assert_eq!(rates.modifier_for_classification_like_cpp(0), 1.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(1), 2.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(2), 3.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(3), 4.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(4), 5.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(5), 6.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(6), 7.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(99), 2.0);
}

#[test]
fn creature_classification_health_rates_match_cpp_switch_and_default_elite() {
    let rates = CreatureClassificationHealthRatesLikeCpp {
        normal: 1.0,
        elite: 2.0,
        rare_elite: 3.0,
        obsolete: 4.0,
        rare: 5.0,
        trivial: 6.0,
        minus_mob: 7.0,
    };

    assert_eq!(rates.modifier_for_classification_like_cpp(0), 1.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(1), 2.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(2), 3.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(3), 4.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(4), 5.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(5), 6.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(6), 7.0);
    assert_eq!(rates.modifier_for_classification_like_cpp(99), 2.0);
}

#[test]
fn creature_difficulty_health_scaling_current_and_invalid_match_cpp() {
    let current = CreatureDifficultyRecordLikeCpp {
        health_scaling_expansion: CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP,
        ..base_difficulty_record()
    }
    .normalize_like_cpp(1.0);
    assert_eq!(
        current.health_scaling_expansion_index_like_cpp(),
        CREATURE_CURRENT_EXPANSION_LIKE_CPP
    );

    let invalid_low = CreatureDifficultyRecordLikeCpp {
        health_scaling_expansion: CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP - 1,
        ..base_difficulty_record()
    }
    .normalize_like_cpp(1.0);
    assert_eq!(invalid_low.health_scaling_expansion, 0);
    assert_eq!(invalid_low.health_scaling_expansion_index_like_cpp(), 0);

    let invalid_high = CreatureDifficultyRecordLikeCpp {
        health_scaling_expansion: CREATURE_CURRENT_EXPANSION_LIKE_CPP as i32 + 1,
        ..base_difficulty_record()
    }
    .normalize_like_cpp(1.0);
    assert_eq!(invalid_high.health_scaling_expansion, 0);
}

#[test]
fn creature_difficulty_normalizes_min_max_gold_and_damage_modifier_like_cpp() {
    let normalized = CreatureDifficultyRecordLikeCpp {
        min_level: 0,
        max_level: 0,
        health_scaling_expansion: 1,
        damage_modifier: 3.0,
        gold_min: 50,
        gold_max: 10,
        ..base_difficulty_record()
    }
    .normalize_like_cpp(2.0);
    assert_eq!(normalized.min_level, 1);
    assert_eq!(normalized.max_level, 1);
    assert_eq!(normalized.gold_max, 50);
    assert_eq!(normalized.damage_modifier, 6.0);

    let inverted = CreatureDifficultyRecordLikeCpp {
        min_level: 60,
        max_level: 55,
        ..base_difficulty_record()
    }
    .normalize_like_cpp(1.0);
    assert_eq!(inverted.min_level, 55);
    assert_eq!(inverted.max_level, 55);
}

#[test]
fn creature_base_stats_normalize_loaded_rows_but_missing_fallback_stays_zero_like_cpp() {
    let store = CreatureBaseStatsStoreLikeCpp::from_records([(
        10,
        2,
        CreatureBaseStatsRecordLikeCpp {
            base_health: [0, 25, 0],
            base_mana: 30,
            base_armor: 40,
            attack_power: 50,
            ranged_attack_power: 60,
            base_damage: [-1.0, 2.5, -0.25],
        },
    )]);

    let loaded = store.get_like_cpp(10, 2);
    assert_eq!(loaded.base_health, [1, 25, 1]);
    assert_eq!(loaded.base_damage, [0.0, 2.5, 0.0]);
    assert_eq!(loaded.base_mana, 30);
    assert_eq!(loaded.attack_power, 50);
    assert_eq!(loaded.ranged_attack_power, 60);

    let missing = store.get_like_cpp(99, 2);
    assert_eq!(missing.base_health, [0, 0, 0]);
    assert_eq!(missing.base_damage, [0.0, 0.0, 0.0]);
    assert_eq!(missing.base_mana, 0);
    assert_eq!(missing.attack_power, 0);
    assert_eq!(missing.ranged_attack_power, 0);
}

#[test]
fn creature_base_stats_generate_helpers_match_cpp_ceil_and_expansion_index() {
    let stats = CreatureBaseStatsRecordLikeCpp {
        base_health: [100, 200, 300],
        base_mana: 50,
        base_armor: 80,
        attack_power: 0,
        ranged_attack_power: 0,
        base_damage: [1.25, 2.5, 3.75],
    };
    let difficulty = CreatureDifficultyRecordLikeCpp {
        health_scaling_expansion: CREATURE_EXPANSION_LEVEL_CURRENT_LIKE_CPP,
        health_modifier: 1.25,
        mana_modifier: 1.01,
        armor_modifier: 1.1,
        ..base_difficulty_record()
    }
    .normalize_like_cpp(1.0);

    assert_eq!(stats.generate_health_like_cpp(&difficulty), 375);
    assert_eq!(stats.generate_mana_like_cpp(&difficulty), 51);
    assert_eq!(stats.generate_armor_like_cpp(&difficulty), 88);
    assert_eq!(stats.generate_base_damage_like_cpp(&difficulty), 3.75);

    let no_mana = CreatureBaseStatsRecordLikeCpp {
        base_mana: 0,
        ..stats
    };
    assert_eq!(no_mana.generate_mana_like_cpp(&difficulty), 0);
}

#[test]
fn creature_difficulty_store_keys_by_entry_and_difficulty_after_normalization() {
    let store = CreatureDifficultyStoreLikeCpp::from_records(
        [CreatureDifficultyRecordLikeCpp {
            entry: 7,
            difficulty_id: 3,
            min_level: 4,
            max_level: 5,
            damage_modifier: 2.0,
            ..base_difficulty_record()
        }],
        |entry| if entry == 7 { 1.5 } else { 1.0 },
    );

    let record = store.get_like_cpp(7, 3);
    assert_eq!(record.min_level, 4);
    assert_eq!(record.max_level, 5);
    assert_eq!(record.damage_modifier, 3.0);
    assert_eq!(store.get_like_cpp(7, 0).min_level, 1);
}

#[test]
fn creature_difficulty_store_follows_difficulty_fallbacks_then_default_like_cpp() {
    let store = CreatureDifficultyStoreLikeCpp::from_records_with_difficulty_fallbacks(
        [CreatureDifficultyRecordLikeCpp {
            entry: 7,
            difficulty_id: 0,
            min_level: 12,
            max_level: 12,
            ..base_difficulty_record()
        }],
        |_| 1.0,
        [(2, 1), (1, 0)],
    );

    assert_eq!(store.get_like_cpp(7, 2).min_level, 12);
    assert_eq!(store.get_like_cpp(8, 2).min_level, 1);
}

#[test]
fn creature_template_mount_model_selection_matches_cpp_shape() {
    let entry = CreatureTemplateMountEntryLikeCpp {
        entry: 10,
        vehicle_id: 77,
        models: vec![CreatureTemplateMountModelLikeCpp {
            display_id: 1234,
            display_scale: 1.0,
            probability: 0.0,
        }],
    };

    assert_eq!(
        entry.choose_display_id_like_cpp(&mut StdRng::seed_from_u64(1)),
        Some(1234)
    );

    let entry = CreatureTemplateMountEntryLikeCpp {
        entry: 11,
        vehicle_id: 0,
        models: vec![
            CreatureTemplateMountModelLikeCpp {
                display_id: 1,
                display_scale: 1.0,
                probability: 0.0,
            },
            CreatureTemplateMountModelLikeCpp {
                display_id: 2,
                display_scale: 1.0,
                probability: 100.0,
            },
        ],
    };

    assert_eq!(
        entry.choose_display_id_like_cpp(&mut StdRng::seed_from_u64(2)),
        Some(2)
    );
}
