//! Condition entry model and load reporting regression scenarios, part 4 of 4.
//!
//! Moved out of the conditions.rs root under #638; every test is unchanged.

use super::*;

#[test]
fn parse_condition_row_maps_positive_source_and_type_like_cpp() {
    let condition = parse_condition_row_like_cpp(
        ConditionDbRowLikeCpp {
            source_type_or_reference_id: ConditionSourceType::Phase as i32,
            source_group: 12,
            source_entry: 34,
            source_id: 0,
            else_group: 2,
            condition_type_or_reference: ConditionType::Aura as i32,
            condition_target: 1,
            condition_value1: 100,
            condition_value2: 2,
            condition_value3: 3,
            condition_string_value1: String::from("string"),
            negative_condition: true,
            error_type: 4,
            error_text_id: 5,
            script_name: String::from("script"),
        },
        |name| u32::from(name == "script") * 77,
    )
    .unwrap();

    assert_eq!(condition.source_type, ConditionSourceType::Phase);
    assert_eq!(condition.condition_type, ConditionType::Aura);
    assert_eq!(condition.source_group, 12);
    assert_eq!(condition.source_entry, 34);
    assert_eq!(condition.else_group, 2);
    assert_eq!(condition.condition_target, 1);
    assert_eq!(condition.condition_value1, 100);
    assert_eq!(condition.condition_value2, 2);
    assert_eq!(condition.condition_value3, 3);
    assert_eq!(condition.condition_string_value1, "string");
    assert!(condition.negative_condition);
    assert_eq!(condition.error_type, 4);
    assert_eq!(condition.error_text_id, 5);
    assert_eq!(condition.script_id, 77);
}

#[test]
fn parse_condition_row_maps_reference_template_like_cpp() {
    let condition = parse_condition_row_like_cpp(
        ConditionDbRowLikeCpp {
            source_type_or_reference_id: -500,
            source_group: 9,
            source_entry: 8,
            source_id: 7,
            else_group: 0,
            condition_type_or_reference: -(ConditionType::Aura as i32),
            condition_target: 0,
            condition_value1: 0,
            condition_value2: 0,
            condition_value3: 0,
            condition_string_value1: String::new(),
            negative_condition: false,
            error_type: 0,
            error_text_id: 0,
            script_name: String::new(),
        },
        |_| 0,
    )
    .unwrap();

    assert_eq!(
        condition.source_type,
        ConditionSourceType::ReferenceCondition
    );
    assert_eq!(condition.source_group, 500);
    assert_eq!(condition.source_entry, 8);
    assert_eq!(condition.source_id, 7);
    assert_eq!(condition.reference_id, ConditionType::Aura as u32);
    assert_eq!(condition.condition_type, ConditionType::None);
}

#[test]
fn parse_condition_rows_records_cpp_reference_useless_data_warnings_without_skipping() {
    let row = ConditionDbRowLikeCpp {
        source_type_or_reference_id: ConditionSourceType::Phase as i32,
        source_group: 0,
        source_entry: 0,
        source_id: 0,
        else_group: 0,
        condition_type_or_reference: -42,
        condition_target: 1,
        condition_value1: 10,
        condition_value2: 20,
        condition_value3: 30,
        condition_string_value1: String::new(),
        negative_condition: true,
        error_type: 0,
        error_text_id: 0,
        script_name: String::new(),
    };

    let report = parse_condition_rows_like_cpp([row], |_| 0);

    assert_eq!(report.conditions.len(), 1);
    assert!(report.skipped.is_empty());
    assert_eq!(
        report.warnings,
        vec![
            ConditionLoadWarningLikeCpp::ReferenceUselessConditionTarget {
                source_type_or_reference_id: ConditionSourceType::Phase as i32,
                condition_target: 1,
            },
            ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: ConditionSourceType::Phase as i32,
                field: 1,
                value: 10,
            },
            ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: ConditionSourceType::Phase as i32,
                field: 2,
                value: 20,
            },
            ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: ConditionSourceType::Phase as i32,
                field: 3,
                value: 30,
            },
            ConditionLoadWarningLikeCpp::ReferenceUselessNegativeCondition {
                source_type_or_reference_id: ConditionSourceType::Phase as i32,
            },
        ]
    );
}

#[test]
fn parse_condition_rows_records_cpp_reference_template_useless_data_warnings() {
    let row = ConditionDbRowLikeCpp {
        source_type_or_reference_id: -500,
        source_group: 9,
        source_entry: 8,
        source_id: 7,
        else_group: 0,
        condition_type_or_reference: ConditionType::Aura as i32,
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

    let report = parse_condition_rows_like_cpp([row], |_| 0);

    assert_eq!(
        report.warnings,
        vec![
            ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceGroup {
                source_type_or_reference_id: -500,
                source_group: 9,
            },
            ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceEntry {
                source_type_or_reference_id: -500,
                source_entry: 8,
            },
            ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceId {
                source_type_or_reference_id: -500,
                source_id: 7,
            },
        ]
    );
    assert_eq!(
        report.skipped[0].reason,
        ConditionRowSkipReason::SourceIdNotAllowed {
            source_type: ConditionSourceType::ReferenceCondition,
            source_id: 7,
        }
    );
}

#[test]
fn parse_condition_row_skips_self_reference_like_cpp() {
    let row = ConditionDbRowLikeCpp {
        source_type_or_reference_id: -42,
        source_group: 0,
        source_entry: 0,
        source_id: 0,
        else_group: 0,
        condition_type_or_reference: -42,
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

    let skipped = parse_condition_row_like_cpp(row, |_| 0).unwrap_err();

    assert_eq!(skipped.reason, ConditionRowSkipReason::SelfReference(-42));
}

#[test]
fn parse_condition_rows_applies_cpp_source_group_and_id_shape_validation() {
    let mut invalid_group = condition_row(ConditionSourceType::QuestAvailable, ConditionType::Aura);
    invalid_group.source_group = 5;
    let mut invalid_id = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
    invalid_id.source_id = 9;
    let report = parse_condition_rows_like_cpp([invalid_group, invalid_id], |_| 0);

    assert_eq!(report.conditions.len(), 0);
    assert_eq!(
        report.skipped[0].reason,
        ConditionRowSkipReason::SourceGroupNotAllowed {
            source_type: ConditionSourceType::QuestAvailable,
            source_group: 5,
        }
    );
    assert_eq!(
        report.skipped[1].reason,
        ConditionRowSkipReason::SourceIdNotAllowed {
            source_type: ConditionSourceType::Phase,
            source_id: 9,
        }
    );
}

#[test]
fn parse_condition_rows_validates_condition_target_for_non_references_like_cpp() {
    let mut invalid = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
    invalid.condition_target = 1;
    let mut valid = condition_row(ConditionSourceType::SpellClickEvent, ConditionType::Aura);
    valid.condition_target = 1;
    let mut reference = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
    reference.condition_type_or_reference = -77;
    reference.condition_target = 2;

    let report = parse_condition_rows_like_cpp([invalid, valid, reference], |_| 0);

    assert_eq!(report.conditions.len(), 2);
    assert_eq!(
        report.conditions[0].source_type,
        ConditionSourceType::SpellClickEvent
    );
    assert_eq!(report.conditions[1].reference_id, 77);
    assert_eq!(
        report.skipped[0].reason,
        ConditionRowSkipReason::ConditionTargetOutOfRange {
            source_type: ConditionSourceType::Phase,
            condition_target: 1,
            max_available_targets: 1,
        }
    );
}

#[test]
fn parse_condition_rows_records_cpp_useless_condition_value_warnings() {
    let mut alive = condition_row(ConditionSourceType::Phase, ConditionType::Alive);
    alive.condition_value1 = 10;
    alive.condition_value2 = 20;
    alive.condition_value3 = 30;
    alive.condition_string_value1 = "unused".to_string();
    let mut player_guid = condition_row(ConditionSourceType::Phase, ConditionType::ObjectEntryGuid);
    player_guid.condition_value1 = TypeId::Player as u32;
    player_guid.condition_value2 = 42;
    player_guid.condition_value3 = 77;

    let report = parse_condition_rows_like_cpp([alive, player_guid], |_| 0);

    assert_eq!(report.conditions.len(), 2);
    assert_eq!(
        report.warnings,
        vec![
            ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: ConditionType::Alive,
                field: 1,
                value: 10,
            },
            ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: ConditionType::Alive,
                field: 2,
                value: 20,
            },
            ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: ConditionType::Alive,
                field: 3,
                value: 30,
            },
            ConditionLoadWarningLikeCpp::UselessConditionStringValue {
                condition_type: ConditionType::Alive,
                field: 4,
                value: "unused".to_string(),
            },
            ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: ConditionType::ObjectEntryGuid,
                field: 2,
                value: 42,
            },
            ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: ConditionType::ObjectEntryGuid,
                field: 3,
                value: 77,
            },
        ]
    );
}

#[test]
fn parse_condition_rows_normalizes_error_fields_like_cpp() {
    let mut non_spell = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
    non_spell.error_type = 7;
    non_spell.error_text_id = 8;
    let mut spell_with_error = condition_row(ConditionSourceType::Spell, ConditionType::Aura);
    spell_with_error.error_type = 7;
    spell_with_error.error_text_id = 8;
    let mut spell_without_error = condition_row(ConditionSourceType::Spell, ConditionType::Aura);
    spell_without_error.error_text_id = 8;

    let report =
        parse_condition_rows_like_cpp([non_spell, spell_with_error, spell_without_error], |_| 0);

    assert_eq!(report.conditions.len(), 3);
    assert_eq!(report.conditions[0].error_type, 0);
    assert_eq!(report.conditions[0].error_text_id, 0);
    assert_eq!(report.conditions[1].error_type, 7);
    assert_eq!(report.conditions[1].error_text_id, 8);
    assert_eq!(report.conditions[2].error_type, 0);
    assert_eq!(report.conditions[2].error_text_id, 0);
    assert_eq!(
        report.warnings,
        vec![
            ConditionLoadWarningLikeCpp::ErrorTypeResetForNonSpell {
                source_type: ConditionSourceType::Phase,
                error_type: 7,
            },
            ConditionLoadWarningLikeCpp::ErrorTextIdResetWithoutErrorType {
                source_type: ConditionSourceType::Phase,
                error_text_id: 8,
            },
            ConditionLoadWarningLikeCpp::ErrorTextIdResetWithoutErrorType {
                source_type: ConditionSourceType::Spell,
                error_text_id: 8,
            },
        ]
    );
}

#[test]
fn condition_entries_store_groups_by_source_type_and_id_like_cpp() {
    let first = Condition {
        source_type: ConditionSourceType::Phase,
        source_group: 7,
        source_entry: 20,
        source_id: 0,
        condition_type: ConditionType::Aura,
        condition_value1: 100,
        ..Condition::default()
    };
    let second = Condition {
        condition_value1: 101,
        ..first.clone()
    };
    let other = Condition {
        source_type: ConditionSourceType::TerrainSwap,
        source_group: 7,
        source_entry: 20,
        source_id: 0,
        condition_type: ConditionType::MapId,
        ..Condition::default()
    };

    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        first.clone(),
        second.clone(),
        other,
    ]);

    assert_eq!(store.bucket_count(), 2);
    assert_eq!(store.condition_count(), 3);
    let phase_bucket = store
        .conditions_for_like_cpp(ConditionSourceType::Phase, first.id_like_cpp())
        .unwrap();
    assert_eq!(phase_bucket.as_slice(), &[first, second]);
}

#[test]
fn spell_click_aura_spell_index_matches_cpp_load_builder() {
    let aura_spell_click = Condition {
        source_type: ConditionSourceType::SpellClickEvent,
        source_group: 7,
        source_entry: 20,
        condition_type: ConditionType::Aura,
        condition_value1: 100,
        ..Condition::default()
    };
    let non_aura_spell_click = Condition {
        source_type: ConditionSourceType::SpellClickEvent,
        source_group: 7,
        source_entry: 21,
        condition_type: ConditionType::MapId,
        condition_value1: 571,
        ..Condition::default()
    };
    let aura_other_source = Condition {
        source_type: ConditionSourceType::Spell,
        source_group: 0,
        source_entry: 100,
        condition_type: ConditionType::Aura,
        condition_value1: 200,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
        aura_spell_click,
        non_aura_spell_click,
        aura_other_source,
    ]);

    assert_eq!(
        store.spells_used_in_spell_click_conditions_like_cpp(),
        std::collections::HashSet::from([100])
    );
    assert!(store.is_spell_used_in_spell_click_conditions_like_cpp(100));
    assert!(!store.is_spell_used_in_spell_click_conditions_like_cpp(200));
}

#[test]
fn conditions_reference_expires_after_store_reload_like_cpp() {
    let condition = Condition {
        source_type: ConditionSourceType::Phase,
        source_group: 7,
        source_entry: 20,
        condition_type: ConditionType::Aura,
        ..Condition::default()
    };
    let reference = {
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition.clone()]);
        store
            .reference_for_like_cpp(ConditionSourceType::Phase, condition.id_like_cpp())
            .unwrap()
    };

    assert!(reference.upgrade().is_none());
    assert!(reference.is_expired());
}
