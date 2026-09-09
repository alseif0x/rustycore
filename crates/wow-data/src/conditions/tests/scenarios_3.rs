//! Condition entry model and load reporting regression scenarios, part 3 of 4.
//!
//! Moved out of the conditions.rs root under #638; every test is unchanged.

use super::*;

#[test]
fn condition_external_source_validation_uses_loaded_stores_like_cpp() {
    let item_store = crate::ItemStore::from_records([crate::ItemRecord {
        id: 100,
        class_id: 0,
        subclass_id: 0,
        material: 0,
        inventory_type: 0,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }]);
    let area_store = crate::AreaTableStore::from_entries([
        crate::AreaTableEntry {
            id: 7,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        crate::AreaTableEntry {
            id: 8,
            continent_id: 0,
            parent_area_id: 7,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: crate::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP,
        },
    ]);
    let map_store = crate::MapStore::from_entries([crate::MapEntry {
        id: 571,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        instance_type: 0,
        expansion_id: 0,
        flags1: 0,
        flags2: 0,
    }]);
    let quest_store =
        crate::quest::QuestStore::from_quests_like_cpp([quest_template(700, Vec::new())]);
    let mut spell_store = crate::SpellStore::new();
    spell_store.insert(200, spell_info(200));
    spell_store.insert(
        201,
        crate::SpellInfo {
            spell_id: 201,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                crate::SpellEffectInfo {
                    effect_index: 0,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 6,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                crate::SpellEffectInfo {
                    effect_index: 1,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 7,
                    implicit_target_2: 0,
                    ..Default::default()
                },
            ],
        },
    );
    let area_trigger_store =
        crate::AreaTriggerDb2Store::from_entries([crate::AreaTriggerDb2Entry {
            id: 300,
            message: String::new(),
            pos: crate::maps_world::Db2Position3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            continent_id: 571,
            phase_use_flags: 0,
            phase_id: 0,
            phase_group_id: 0,
            radius: 1.0,
            box_length: 0.0,
            box_width: 0.0,
            box_height: 0.0,
            box_yaw: 0.0,
            shape_type: 0,
            shape_id: 0,
            area_trigger_action_set_id: 0,
            flags: 0,
        }]);
    let mut graveyard_store = crate::GraveyardStore::default();
    graveyard_store.load_graveyard_zones_from_rows_like_cpp(
        [crate::GraveyardZoneRow {
            safe_loc_id: 400,
            ghost_zone_id: 7,
        }],
        |_| true,
        |_| true,
    );
    let (spawn_group_store, _) = crate::SpawnGroupTemplateStore::from_rows_like_cpp([
        crate::SpawnGroupTemplateRow {
            group_id: 500,
            name: "manual".to_string(),
            flags: 0,
        },
        crate::SpawnGroupTemplateRow {
            group_id: 501,
            name: "system".to_string(),
            flags: crate::spawn_group::SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP,
        },
    ]);
    let creature_template_store = crate::WorldIdStore::from_ids("creature_template", [600]);
    let gameobject_template_store = crate::WorldIdStore::from_ids("gameobject_template", [601]);
    let trainer_store = crate::WorldIdStore::from_ids("trainer", [700]);
    let conversation_line_template_store =
        crate::WorldIdStore::from_ids("conversation_line_template", [800]);
    let area_trigger_template_store =
        crate::AreaTriggerTemplateStore::from_keys([(900, false), (901, true)]);
    let loot_template_exists = |source_type: ConditionSourceType, source_group: u32| {
        source_type == ConditionSourceType::CreatureLootTemplate && source_group == 123
    };
    let loot_source_entry_exists =
        |source_type: ConditionSourceType, source_group: u32, source_entry: i32| {
            source_type == ConditionSourceType::CreatureLootTemplate
                && source_group == 123
                && (source_entry == 456 || source_entry == 900)
        };
    let stores = ConditionExternalValidationStoresLikeCpp {
        item_store: Some(&item_store),
        area_table_store: Some(&area_store),
        map_store: Some(&map_store),
        quest_store: Some(&quest_store),
        spell_store: Some(&spell_store),
        area_trigger_db2_store: Some(&area_trigger_store),
        graveyard_store: Some(&graveyard_store),
        spawn_group_store: Some(&spawn_group_store),
        creature_template_store: Some(&creature_template_store),
        gameobject_template_store: Some(&gameobject_template_store),
        trainer_store: Some(&trainer_store),
        conversation_line_template_store: Some(&conversation_line_template_store),
        area_trigger_template_store: Some(&area_trigger_template_store),
        loot_template_exists: Some(&loot_template_exists),
        loot_source_entry_exists: Some(&loot_source_entry_exists),
        ..ConditionExternalValidationStoresLikeCpp::default()
    };

    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: 700,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::QuestAvailable,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingQuestAvailable(999))
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::NpcVendor,
                source_group: 600,
                source_entry: 100,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::NpcVendor,
                source_group: 600,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingNpcVendorItem(999))
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::Spell,
                source_entry: 200,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::GossipMenu,
                source_group: 800,
                source_entry: 900,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::GossipMenuOption,
                source_group: 800,
                source_entry: 1,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    let mut spell_implicit = Condition {
        source_type: ConditionSourceType::SpellImplicitTarget,
        source_group: 0b11,
        source_entry: 201,
        ..Condition::default()
    };
    assert_eq!(
        validate_and_normalize_condition_source_external_like_cpp(&mut spell_implicit, stores),
        Ok(())
    );
    assert_eq!(spell_implicit.source_group, 0b10);
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::SpellImplicitTarget,
                source_group: 0b1,
                source_entry: 201,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::InvalidSpellImplicitTargetEffectMask(1))
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::SpellClickEvent,
                source_group: 600,
                source_entry: 200,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::SpellClickEvent,
                source_group: 600,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingSourceSpell {
                source_type: ConditionSourceType::SpellClickEvent,
                spell_id: 999
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::TrainerSpell,
                source_group: 700,
                source_entry: 200,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::TrainerSpell,
                source_group: 999,
                source_entry: 200,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingTrainer(
            999
        ))
    ));
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::TrainerSpell,
                source_group: 700,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingSourceSpell {
                source_type: ConditionSourceType::TrainerSpell,
                spell_id: 999
            }
        )
    ));
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::VehicleSpell,
                source_group: 999,
                source_entry: 200,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingSourceCreatureTemplate {
                source_type: ConditionSourceType::VehicleSpell,
                entry: 999
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::ConversationLine,
                source_entry: 800,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::ConversationLine,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingConversationLineTemplate(999))
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::AreaTrigger,
                source_group: 900,
                source_entry: 0,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::AreaTrigger,
                source_group: 901,
                source_entry: 1,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::AreaTrigger,
                source_group: 900,
                source_entry: 1,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingAreaTriggerTemplate {
                id: 900,
                is_custom: true
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::CreatureTemplateVehicle,
                source_entry: 600,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::CreatureTemplateVehicle,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingSourceCreatureTemplate {
                source_type: ConditionSourceType::CreatureTemplateVehicle,
                entry: 999
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::AreaTriggerClientTriggered,
                source_entry: 300,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::AreaTriggerClientTriggered,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingClientAreaTrigger(999))
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::Graveyard,
                source_group: 7,
                source_entry: 400,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::Graveyard,
                source_group: 7,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingGraveyard {
                safe_loc_id: 999,
                zone_id: 7
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::SpawnGroup,
                source_entry: 500,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::SpawnGroup,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingSpawnGroup(999))
    ));
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::SpawnGroup,
                source_entry: 501,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::SystemSpawnGroup(501))
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::ObjectIdVisibility,
                source_group: TypeId::Unit as u32,
                source_entry: 600,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::ObjectIdVisibility,
                source_group: TypeId::GameObject as u32,
                source_entry: 601,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::ObjectIdVisibility,
                source_group: TypeId::GameObject as u32,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingSourceGameObjectTemplate {
                source_type: ConditionSourceType::ObjectIdVisibility,
                entry: 999
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::CreatureLootTemplate,
                source_group: 123,
                source_entry: 456,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::CreatureLootTemplate,
                source_group: 999,
                source_entry: 456,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingLootTemplate {
                source_type: ConditionSourceType::CreatureLootTemplate,
                source_group: 999
            }
        )
    ));
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::CreatureLootTemplate,
                source_group: 123,
                source_entry: 777,
                ..Condition::default()
            },
            stores
        ),
        Err(
            ConditionSourceValidationErrorLikeCpp::NonExistingLootSourceEntry {
                source_type: ConditionSourceType::CreatureLootTemplate,
                source_group: 123,
                source_entry: 777
            }
        )
    ));
    assert_eq!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::TerrainSwap,
                source_entry: 571,
                ..Condition::default()
            },
            stores
        ),
        Ok(())
    );
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::TerrainSwap,
                source_entry: 999,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingTerrainSwapMap(999))
    ));
    assert!(matches!(
        validate_condition_source_external_like_cpp(
            &Condition {
                source_type: ConditionSourceType::Phase,
                source_entry: 99,
                ..Condition::default()
            },
            stores
        ),
        Err(ConditionSourceValidationErrorLikeCpp::NonExistingPhaseArea(
            99
        ))
    ));
}

#[test]
fn parse_condition_rows_applies_static_condition_type_validation_like_cpp() {
    let max_condition_type = ConditionDbRowLikeCpp {
        condition_type_or_reference: ConditionType::Max as i32,
        ..condition_row(ConditionSourceType::SpellClickEvent, ConditionType::None)
    };
    let mut deprecated = condition_row(
        ConditionSourceType::SpellClickEvent,
        ConditionType::SpawnMaskDeprecated,
    );
    deprecated.condition_target = 1;
    let mut legacy = condition_row(
        ConditionSourceType::SpellClickEvent,
        ConditionType::TypeMaskLegacy,
    );
    legacy.condition_target = 1;
    legacy.condition_value1 = 1 << 4;

    let report = parse_condition_rows_like_cpp([max_condition_type, deprecated, legacy], |_| 0);

    assert_eq!(report.conditions.len(), 1);
    assert_eq!(report.conditions[0].condition_type, ConditionType::TypeMask);
    assert_eq!(
        report.conditions[0].condition_value1,
        TypeMask::PLAYER.bits()
    );
    assert_eq!(
        report.skipped[0].reason,
        ConditionRowSkipReason::InvalidConditionType(ConditionType::Max as i32)
    );
    assert_eq!(
        report.skipped[1].reason,
        ConditionRowSkipReason::ConditionTypeValidationFailed(
            ConditionTypeValidationErrorLikeCpp::DeprecatedSpawnMask
        )
    );
}

#[test]
fn condition_source_static_validation_matches_cpp_pure_rejections() {
    let mut spell_implicit = condition_row(
        ConditionSourceType::SpellImplicitTarget,
        ConditionType::Aura,
    );
    spell_implicit.source_group = 0;
    let mut area_trigger = condition_row(ConditionSourceType::AreaTrigger, ConditionType::Aura);
    area_trigger.source_group = 77;
    area_trigger.source_entry = 2;
    let mut object_visibility =
        condition_row(ConditionSourceType::ObjectIdVisibility, ConditionType::Aura);
    object_visibility.source_group = TypeId::Player as u32;

    let report =
        parse_condition_rows_like_cpp([spell_implicit, area_trigger, object_visibility], |_| 0);

    assert_eq!(report.conditions.len(), 0);
    assert_eq!(
        report.skipped[0].reason,
        ConditionRowSkipReason::ConditionSourceValidationFailed(
            ConditionSourceValidationErrorLikeCpp::InvalidSpellImplicitTargetEffectMask(0)
        )
    );
    assert_eq!(
        report.skipped[1].reason,
        ConditionRowSkipReason::ConditionSourceValidationFailed(
            ConditionSourceValidationErrorLikeCpp::InvalidAreaTriggerSourceEntry(2)
        )
    );
    assert_eq!(
        report.skipped[2].reason,
        ConditionRowSkipReason::ConditionSourceValidationFailed(
            ConditionSourceValidationErrorLikeCpp::UncheckedObjectIdVisibilityObjectType(
                TypeId::Player as u32,
            )
        )
    );
}

#[test]
fn condition_source_static_validation_keeps_reference_templates_internal_like_cpp() {
    let mut positive_internal =
        condition_row(ConditionSourceType::ReferenceCondition, ConditionType::Aura);
    positive_internal.source_group = 7;
    let mut negative_template = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
    negative_template.source_type_or_reference_id = -7;
    negative_template.source_group = 0;

    let report = parse_condition_rows_like_cpp([positive_internal, negative_template], |_| 0);

    assert_eq!(report.conditions.len(), 1);
    assert_eq!(
        report.conditions[0].source_type,
        ConditionSourceType::ReferenceCondition
    );
    assert_eq!(report.conditions[0].source_group, 7);
    assert_eq!(
        report.skipped[0].reason,
        ConditionRowSkipReason::ConditionSourceValidationFailed(
            ConditionSourceValidationErrorLikeCpp::InvalidSourceType(
                ConditionSourceType::ReferenceCondition,
            )
        )
    );
}

#[test]
fn condition_searcher_type_mask_matches_cpp_direct_cases() {
    assert_eq!(
        Condition {
            condition_type: ConditionType::None,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_ALL
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::Aura,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_CREATURE | GRID_MAP_TYPE_MASK_PLAYER
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::Item,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_PLAYER
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::CreatureType,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_CREATURE
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::PrivateObject,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_ALL & !GRID_MAP_TYPE_MASK_PLAYER
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::Team,
            negative_condition: true,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_ALL
    );
}

#[test]
fn condition_searcher_type_mask_object_entry_and_type_mask_match_cpp() {
    assert_eq!(
        Condition {
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::Unit as u32,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_CREATURE
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::Player as u32,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_PLAYER
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::GameObject as u32,
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_GAME_OBJECT
    );
    assert_eq!(
        Condition {
            condition_type: ConditionType::TypeMask,
            condition_value1: (TypeMask::UNIT | TypeMask::GAME_OBJECT).bits(),
            ..Condition::default()
        }
        .get_searcher_type_mask_for_condition_like_cpp(),
        GRID_MAP_TYPE_MASK_CREATURE | GRID_MAP_TYPE_MASK_PLAYER | GRID_MAP_TYPE_MASK_GAME_OBJECT
    );
}

#[test]
fn condition_searcher_type_mask_list_ands_groups_ors_else_groups_and_expands_refs() {
    let reference_condition = Condition {
        source_type: ConditionSourceType::ReferenceCondition,
        source_group: 77,
        condition_type: ConditionType::Item,
        ..Condition::default()
    };
    let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([reference_condition]);
    let conditions = vec![
        Condition {
            else_group: 0,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        },
        Condition {
            else_group: 0,
            reference_id: 77,
            ..Condition::default()
        },
        Condition {
            else_group: 1,
            condition_type: ConditionType::CreatureType,
            ..Condition::default()
        },
    ];

    assert_eq!(
        store.get_searcher_type_mask_for_condition_list_like_cpp(&conditions),
        GRID_MAP_TYPE_MASK_PLAYER | GRID_MAP_TYPE_MASK_CREATURE
    );
}

#[test]
fn condition_id_matches_cpp_key_fields() {
    let condition = Condition {
        source_group: 12,
        source_entry: -45,
        source_id: 67,
        ..Condition::default()
    };

    assert_eq!(condition.id_like_cpp(), ConditionId::new(12, -45, 67));
}
