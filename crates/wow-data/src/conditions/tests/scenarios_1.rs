//! Condition entry model and load reporting regression scenarios, part 1 of 4.
//!
//! Moved out of the conditions.rs root under #638; every test is unchanged.

use super::*;

#[test]
fn condition_row_preserves_normalized_signed_domains() {
    let row = ConditionDbRowLikeCpp {
        source_type_or_reference_id: ConditionSourceType::SmartEvent as i32,
        source_group: 1,
        source_entry: -5,
        source_id: (-1_i32) as u32,
        else_group: 0,
        condition_type_or_reference: ConditionType::None as i32,
        condition_target: 0,
        condition_value1: 0,
        condition_value2: 0,
        condition_value3: 0,
        condition_string_value1: String::new(),
        negative_condition: false,
        error_type: 0,
        error_text_id: 0,
        script_name: String::new(),
    };
    let condition = parse_condition_row_like_cpp(row, |_| 0).unwrap();
    assert_eq!(condition.source_id, u32::MAX);
}

#[test]
fn condition_default_matches_cpp_constructor() {
    let condition = Condition::default();

    assert_eq!(condition.source_type, ConditionSourceType::None);
    assert_eq!(condition.source_group, 0);
    assert_eq!(condition.source_entry, 0);
    assert_eq!(condition.source_id, 0);
    assert_eq!(condition.else_group, 0);
    assert_eq!(condition.condition_type, ConditionType::None);
    assert_eq!(condition.condition_value1, 0);
    assert_eq!(condition.condition_value2, 0);
    assert_eq!(condition.condition_value3, 0);
    assert!(condition.condition_string_value1.is_empty());
    assert_eq!(condition.error_type, 0);
    assert_eq!(condition.error_text_id, 0);
    assert_eq!(condition.reference_id, 0);
    assert_eq!(condition.script_id, 0);
    assert_eq!(condition.condition_target, 0);
    assert!(!condition.negative_condition);
    assert!(!condition.is_loaded_like_cpp());
}

#[test]
fn condition_is_loaded_matches_cpp() {
    let mut condition = Condition {
        condition_type: ConditionType::Aura,
        ..Condition::default()
    };
    assert!(condition.is_loaded_like_cpp());

    condition.condition_type = ConditionType::None;
    condition.reference_id = 42;
    assert!(condition.is_loaded_like_cpp());

    condition.reference_id = 0;
    condition.script_id = 7;
    assert!(condition.is_loaded_like_cpp());
}

#[test]
fn source_group_and_id_flags_match_cpp() {
    assert!(condition_source_can_have_group_set_like_cpp(
        ConditionSourceType::CreatureLootTemplate,
    ));
    assert!(condition_source_can_have_group_set_like_cpp(
        ConditionSourceType::SpellClickEvent,
    ));
    assert!(condition_source_can_have_group_set_like_cpp(
        ConditionSourceType::ReferenceCondition,
    ));
    assert!(!condition_source_can_have_group_set_like_cpp(
        ConditionSourceType::QuestAvailable,
    ));

    assert!(condition_source_can_have_id_set_like_cpp(
        ConditionSourceType::SmartEvent,
    ));
    assert!(!condition_source_can_have_id_set_like_cpp(
        ConditionSourceType::SpellClickEvent,
    ));
}

#[test]
fn source_condition_type_allowance_matches_cpp_spawn_group_special_case() {
    assert!(condition_source_can_have_condition_type_like_cpp(
        ConditionSourceType::SpawnGroup,
        ConditionType::MapId,
    ));
    assert!(condition_source_can_have_condition_type_like_cpp(
        ConditionSourceType::SpawnGroup,
        ConditionType::ScenarioStep,
    ));
    assert!(!condition_source_can_have_condition_type_like_cpp(
        ConditionSourceType::SpawnGroup,
        ConditionType::Aura,
    ));
    assert!(condition_source_can_have_condition_type_like_cpp(
        ConditionSourceType::NpcVendor,
        ConditionType::Aura,
    ));
}

#[test]
fn max_available_condition_targets_matches_cpp_source_groups() {
    assert_eq!(
        Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            ..Condition::default()
        }
        .max_available_condition_targets_like_cpp(),
        2
    );
    assert_eq!(
        Condition {
            source_type: ConditionSourceType::SmartEvent,
            ..Condition::default()
        }
        .max_available_condition_targets_like_cpp(),
        2
    );
    assert_eq!(
        Condition {
            source_type: ConditionSourceType::Phase,
            ..Condition::default()
        }
        .max_available_condition_targets_like_cpp(),
        1
    );
}

#[test]
fn condition_to_string_matches_cpp_shape() {
    let condition = Condition {
        source_type: ConditionSourceType::SmartEvent,
        source_group: 8,
        source_entry: -7,
        source_id: 9,
        condition_type: ConditionType::ObjectEntryGuid,
        ..Condition::default()
    };

    assert_eq!(
        condition.to_string_like_cpp(false),
        "[Condition SourceType: 22 (SmartScript), SourceGroup: 8, SourceEntry: -7, SourceId: 9]"
    );
    assert_eq!(
        condition.to_string_like_cpp(true),
        "[Condition SourceType: 22 (SmartScript), SourceGroup: 8, SourceEntry: -7, SourceId: 9, ConditionType: 51 (Object Entry or Guid)]"
    );
}

#[test]
fn condition_to_string_handles_reference_and_private_object_name() {
    let condition = Condition {
        source_type: ConditionSourceType::ReferenceCondition,
        source_group: 55,
        condition_type: ConditionType::PrivateObject,
        ..Condition::default()
    };

    assert_eq!(
        condition.to_string_like_cpp(true),
        "[Condition SourceType: 34 (Reference), SourceGroup: 55, SourceEntry: 0, ConditionType: 57 (Private Object)]"
    );
}

#[test]
fn condition_type_info_matches_cpp_value_slot_metadata() {
    let aura = condition_type_info_like_cpp(ConditionType::Aura);
    assert_eq!(aura.name, "Aura");
    assert!(aura.has_condition_value1);
    assert!(aura.has_condition_value2);
    assert!(aura.has_condition_value3);
    assert!(!aura.has_condition_string_value1);

    let alive = condition_type_info_like_cpp(ConditionType::Alive);
    assert_eq!(alive.name, "Alive");
    assert!(!alive.has_condition_value1);
    assert!(!alive.has_condition_value2);
    assert!(!alive.has_condition_value3);
    assert!(!alive.has_condition_string_value1);

    let objective = condition_type_info_like_cpp(ConditionType::QuestObjectiveProgress);
    assert_eq!(objective.name, "Quest objective progress");
    assert!(objective.has_condition_value1);
    assert!(!objective.has_condition_value2);
    assert!(objective.has_condition_value3);
    assert!(!objective.has_condition_string_value1);

    let string_id = condition_type_info_like_cpp(ConditionType::StringId);
    assert_eq!(string_id.name, "String ID");
    assert!(!string_id.has_condition_value1);
    assert!(!string_id.has_condition_value2);
    assert!(!string_id.has_condition_value3);
    assert!(string_id.has_condition_string_value1);
}

#[test]
fn condition_type_info_keeps_private_object_slot_explicit() {
    let private_object = condition_type_info_like_cpp(ConditionType::PrivateObject);

    assert_eq!(private_object.name, "Private Object");
    assert!(!private_object.has_condition_value1);
    assert!(!private_object.has_condition_value2);
    assert!(!private_object.has_condition_value3);
    assert!(!private_object.has_condition_string_value1);
}

#[test]
fn useless_condition_value_fields_match_cpp_static_metadata() {
    let alive = Condition {
        condition_type: ConditionType::Alive,
        condition_value1: 1,
        condition_value2: 2,
        condition_value3: 3,
        condition_string_value1: String::from("unused"),
        ..Condition::default()
    };
    assert_eq!(
        useless_condition_value_fields_like_cpp(&alive),
        vec![1, 2, 3, 4]
    );

    let aura = Condition {
        condition_type: ConditionType::Aura,
        condition_value1: 1,
        condition_value2: 2,
        condition_value3: 3,
        condition_string_value1: String::from("unused"),
        ..Condition::default()
    };
    assert_eq!(useless_condition_value_fields_like_cpp(&aura), vec![4]);

    let player_object_entry_guid = Condition {
        condition_type: ConditionType::ObjectEntryGuid,
        condition_value1: TypeId::Player as u32,
        condition_value2: 42,
        condition_value3: 77,
        ..Condition::default()
    };
    assert_eq!(
        useless_condition_value_fields_like_cpp(&player_object_entry_guid),
        vec![2, 3]
    );
}

#[test]
fn condition_type_static_validation_matches_cpp_pure_rejections() {
    let mut invalid_aura = Condition {
        condition_type: ConditionType::Aura,
        condition_value2: MAX_SPELL_EFFECTS_LIKE_CPP,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut invalid_aura).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::InvalidSpellEffectIndex(MAX_SPELL_EFFECTS_LIKE_CPP,)
    );

    let mut zero_item_count = Condition {
        condition_type: ConditionType::Item,
        condition_value1: 25,
        condition_value2: 0,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut zero_item_count).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::ZeroItemCount
    );

    let mut invalid_team = Condition {
        condition_type: ConditionType::Team,
        condition_value1: 123,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut invalid_team).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::InvalidTeam(123)
    );

    let mut invalid_level = Condition {
        condition_type: ConditionType::Level,
        condition_value2: ComparisonType::Max as u32,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut invalid_level).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::InvalidComparisonType {
            field: 2,
            value: ComparisonType::Max as u32,
        }
    );

    let mut invalid_stand_state = Condition {
        condition_type: ConditionType::StandState,
        condition_value1: 0,
        condition_value2: UnitStandStateType::Max as u32,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut invalid_stand_state).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::InvalidStandState {
            value1: 0,
            value2: UnitStandStateType::Max as u32,
        }
    );

    let mut invalid_battle_pet_count = Condition {
        condition_type: ConditionType::BattlePetCount,
        condition_value2: DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP + 1,
        condition_value3: ComparisonType::Eq as u32,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut invalid_battle_pet_count).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::InvalidBattlePetCount(
            DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP + 1,
        )
    );

    let mut string_id = Condition {
        condition_type: ConditionType::StringId,
        condition_string_value1: "spawn-id".to_string(),
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut string_id),
        Ok(ConditionTypeValidationReportLikeCpp {
            useless_value_fields: Vec::new(),
        })
    );
}

#[test]
fn condition_type_static_validation_matches_cpp_target_selector_rules() {
    let mut self_relation = Condition {
        source_type: ConditionSourceType::SpellClickEvent,
        condition_target: 1,
        condition_type: ConditionType::RelationTo,
        condition_value1: 1,
        condition_value2: RelationType::InParty as u32,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut self_relation).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::SelfTargetSelector { field: 1, value: 1 }
    );

    let mut invalid_distance = Condition {
        source_type: ConditionSourceType::SpellClickEvent,
        condition_target: 0,
        condition_type: ConditionType::DistanceTo,
        condition_value1: 2,
        condition_value3: ComparisonType::Eq as u32,
        ..Condition::default()
    };
    assert_eq!(
        validate_condition_type_static_like_cpp(&mut invalid_distance).unwrap_err(),
        ConditionTypeValidationErrorLikeCpp::InvalidTargetSelector {
            field: 1,
            value: 2,
            max: 2,
        }
    );

    let mut valid_reaction = Condition {
        source_type: ConditionSourceType::SpellClickEvent,
        condition_target: 0,
        condition_type: ConditionType::ReactionTo,
        condition_value1: 1,
        condition_value2: 1,
        ..Condition::default()
    };
    assert!(validate_condition_type_static_like_cpp(&mut valid_reaction).is_ok());
}

#[test]
fn condition_type_static_validation_normalizes_legacy_object_types_like_cpp() {
    let mut legacy_object_entry = Condition {
        condition_type: ConditionType::ObjectEntryGuidLegacy,
        condition_value1: 3,
        ..Condition::default()
    };

    validate_condition_type_static_like_cpp(&mut legacy_object_entry).unwrap();

    assert_eq!(
        legacy_object_entry.condition_type,
        ConditionType::ObjectEntryGuid
    );
    assert_eq!(legacy_object_entry.condition_value1, TypeId::Unit as u32);

    let mut legacy_type_mask = Condition {
        condition_type: ConditionType::TypeMaskLegacy,
        condition_value1: (1 << 3) | (1 << 5),
        ..Condition::default()
    };

    validate_condition_type_static_like_cpp(&mut legacy_type_mask).unwrap();

    assert_eq!(legacy_type_mask.condition_type, ConditionType::TypeMask);
    assert_eq!(
        legacy_type_mask.condition_value1,
        (TypeMask::UNIT | TypeMask::GAME_OBJECT).bits()
    );
}
