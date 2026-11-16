use super::super::tests::bonus_source_test_templates;
use super::{
    ITEM_FLAG_NO_DISENCHANT, ITEM_FLAG2_NO_TRADE_BIND_ON_ACQUIRE, ITEM_FLAG4_NO_SALVAGE,
    ItemBonusSources,
};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{
        item_quantities::ItemEffectRelationRecord,
        item_records::{
            AzeriteEmpoweredItemRecord, ITEM_SQUISH_ERA_HASH, ItemBonusRecord, ItemEffectRecord,
            ItemOffsetCurveRecord, ItemScalingConfigRecord, ItemSquishEraRecord,
        },
    },
};
use wow_entities::{
    ItemBonusData, ItemBonusDataError, ItemBonusEntry, ItemBonusSource, ItemOffsetCurveFacts,
    ItemScalingConfigFacts, ItemSquishEraFacts,
};

const ITEM_ID: u32 = 0x8000_0005;

fn removals(rows: impl IntoIterator<Item = (u32, i32, u8)>) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(rows)
}

fn bonus(id: u32, parent: u16, bonus_type: u8, values: [i32; 4], order: u8) -> ItemBonusRecord {
    ItemBonusRecord {
        id,
        values,
        parent,
        bonus_type,
        order,
    }
}

fn effect(id: u32, legacy_slot: u8) -> ItemEffectRecord {
    ItemEffectRecord {
        id,
        legacy_slot,
        trigger: 0,
        charges: 0,
        cooldown: 0,
        category_cooldown: 0,
        spell_category: 0,
        spell: 0,
        specialization: 0,
        player_condition: 0,
    }
}

fn relation(id: u32, effect: i32) -> ItemEffectRelationRecord {
    ItemEffectRelationRecord {
        id,
        effect,
        item: ITEM_ID,
    }
}

#[test]
fn source_borrows_templates_and_preserves_item_facts_and_lookup_widths() {
    let rows_to_remove = removals([(ITEM_SQUISH_ERA_HASH, 8, 2)]);
    let templates = bonus_source_test_templates(ITEM_ID, &rows_to_remove, |records| {
        records.items[0].class = 0;
        records.items[0].content_tuning = 0x1234;
        let template = &mut records.sparse[0];
        template.quality = -7;
        template.item_level = 0xabcd;
        template.required_level = -9;
        template.stat_bonus = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        template.stat_percent = [-10, -9, -8, -7, -6, -5, -4, -3, -2, -1];
        template.socket_percentage = [
            f32::from_bits(0x7fc0_1234),
            -0.0,
            1.25,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        template.socket_type = [0, 127, 255];
        template.bonding = 15;
        template.content_tuning = i32::MIN;
        template.player_level_curve = -3;
        template.item_level_offset_curve = -4;
        template.item_level_offset = -5;
        template.squish_era = -6;
        template.limit_category = -7;
        template.flags[0] = ITEM_FLAG_NO_DISENCHANT;
        template.flags[1] = ITEM_FLAG2_NO_TRADE_BIND_ON_ACQUIRE;
        template.flags[3] = 0x20 | ITEM_FLAG4_NO_SALVAGE | 0x0100_0000;

        records.effects = vec![effect(55, 3), effect(0, 1), effect(67, 3)];
        records.relations = vec![
            relation(1, 55),
            relation(2, 0),
            relation(3, 55),
            relation(4, 67),
        ];
        records.azerite_empowered_items = vec![AzeriteEmpoweredItemRecord {
            id: 1,
            item: ITEM_ID as i32,
            tier_unlock_set: u32::MAX,
            power_set: 0x8000_0001,
        }];
        records.item_bonuses = vec![
            bonus(90, 7, 11, [90, 91, 92, 93], 0),
            bonus(10, 7, 12, [10, 11, 12, 13], 255),
            bonus(11, 7, 13, [11, 12, 13, 14], 1),
        ];
        records.item_scaling_configs = vec![ItemScalingConfigRecord {
            id: 0x8000_0001,
            offset_curve: -2,
            item_level: -3,
            required_level: -4,
            squish_era: -5,
            flags: i32::MIN,
        }];
        records.item_offset_curves = vec![ItemOffsetCurveRecord {
            id: 0x8000_0002,
            curve: i32::MIN,
            offset: i32::MAX,
        }];
        records.item_squish_eras = vec![
            ItemSquishEraRecord {
                id: 2,
                patch: 0,
                curve: -8,
                flags: 0,
            },
            ItemSquishEraRecord {
                id: 8,
                patch: 5,
                curve: 6,
                flags: 1,
            },
        ];
        records.item_squish_era_file_last_index = Some(9);
    });
    let source = ItemBonusSources::new(&templates, Some(0));

    assert!(std::ptr::eq(source.templates, &templates));
    assert!(std::ptr::eq(
        source.templates.data.as_ref(),
        templates.data.as_ref()
    ));
    assert_eq!(source.actual_realm_patch(), Some(0));

    let facts = source.template_facts(ITEM_ID).unwrap().unwrap();
    assert_eq!(facts.item_id, ITEM_ID);
    assert_eq!(facts.quality, (-7i32) as u32);
    assert_eq!(facts.item_level, 0xabcd);
    assert_eq!(facts.required_level, -9);
    assert_eq!(facts.item_stat_type, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(
        facts.stat_percent_editor,
        [-10, -9, -8, -7, -6, -5, -4, -3, -2, -1]
    );
    assert_eq!(
        facts.item_stat_socket_cost_multiplier.map(f32::to_bits),
        [
            0x7fc0_1234,
            0x8000_0000,
            1.25f32.to_bits(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ]
    );
    assert_eq!(facts.socket_color, [0, 127, 255]);
    assert_eq!(facts.bonding, 15);
    assert_eq!(facts.content_tuning_id, i32::MIN as u32);
    assert_eq!(facts.player_level_to_item_level_curve_id, (-3i32) as u32);
    assert_eq!(facts.item_level_offset_curve_id, (-4i32) as u32);
    assert_eq!(facts.item_level_offset_item_level, (-5i32) as u32);
    assert_eq!(facts.item_squish_era_id, (-6i32) as u32);
    assert_eq!(facts.limit_category, (-7i32) as u32);
    assert_eq!(facts.effect_ids, [0, 67, 55, 55]);
    assert!(!facts.can_disenchant);
    assert!(facts.can_scrap);
    assert!(!facts.can_salvage);
    assert!(facts.can_recraft);
    assert!(facts.cannot_trade_bind_on_pickup);

    assert_eq!(source.template_facts(0).unwrap(), None);
    assert_eq!(
        source.azerite_tier_unlock_set_id(ITEM_ID).unwrap(),
        Some(u32::MAX)
    );
    assert_eq!(source.azerite_tier_unlock_set_id(0).unwrap(), None);
    assert!(source.has_item_effect(0).unwrap());
    assert!(source.has_item_effect(55).unwrap());
    assert!(!source.has_item_effect(56).unwrap());
    assert_eq!(
        source.scaling_config(0x8000_0001).unwrap(),
        Some(ItemScalingConfigFacts {
            item_offset_curve_id: -2,
            item_level: -3,
            required_level: -4,
            item_squish_era_id: -5,
            flags: i32::MIN,
        })
    );
    assert_eq!(source.scaling_config(0).unwrap(), None);
    assert_eq!(
        source.offset_curve(0x8000_0002).unwrap(),
        Some(ItemOffsetCurveFacts {
            curve_id: i32::MIN,
            offset: i32::MAX,
        })
    );
    assert_eq!(source.offset_curve(0).unwrap(), None);
    assert_eq!(source.item_squish_era_index_bound().unwrap(), 10);
    assert_eq!(
        source.item_squish_era(2).unwrap(),
        Some(ItemSquishEraFacts { patch: 0, flags: 0 })
    );
    assert_eq!(source.item_squish_era(8).unwrap(), None);
    assert_eq!(source.item_squish_era(10).unwrap(), None);
}

#[test]
fn loaded_zero_keys_are_present_and_zero_flags_select_inverse_capabilities() {
    let templates = bonus_source_test_templates(0, &removals([]), |records| {
        records.items[0].class = 0;
        records.azerite_empowered_items = vec![AzeriteEmpoweredItemRecord {
            id: 0,
            item: 0,
            tier_unlock_set: 0,
            power_set: 0,
        }];
        records.item_scaling_configs = vec![ItemScalingConfigRecord {
            id: 0,
            offset_curve: 0,
            item_level: 0,
            required_level: 0,
            squish_era: 0,
            flags: 0,
        }];
        records.item_offset_curves = vec![ItemOffsetCurveRecord {
            id: 0,
            curve: 0,
            offset: 0,
        }];
        records.item_squish_eras = vec![ItemSquishEraRecord {
            id: 0,
            patch: 0,
            curve: 0,
            flags: 0,
        }];
        records.item_squish_era_file_last_index = Some(0);
    });
    let source = ItemBonusSources::new(&templates, None);
    let facts = source.template_facts(0).unwrap().unwrap();

    assert_eq!(facts.item_id, 0);
    assert!(facts.can_disenchant);
    assert!(!facts.can_scrap);
    assert!(facts.can_salvage);
    assert!(!facts.can_recraft);
    assert!(!facts.cannot_trade_bind_on_pickup);
    assert_eq!(source.azerite_tier_unlock_set_id(0).unwrap(), Some(0));
    assert_eq!(
        source.scaling_config(0).unwrap(),
        Some(ItemScalingConfigFacts {
            item_offset_curve_id: 0,
            item_level: 0,
            required_level: 0,
            item_squish_era_id: 0,
            flags: 0,
        })
    );
    assert_eq!(
        source.offset_curve(0).unwrap(),
        Some(ItemOffsetCurveFacts {
            curve_id: 0,
            offset: 0,
        })
    );
    assert_eq!(source.item_squish_era_index_bound().unwrap(), 1);
    assert_eq!(
        source.item_squish_era(0).unwrap(),
        Some(ItemSquishEraFacts { patch: 0, flags: 0 })
    );
}

#[test]
fn fourteen_effects_are_projected_and_entity_enforces_capacity() {
    let templates = bonus_source_test_templates(ITEM_ID, &removals([]), |records| {
        records.items[0].class = 0;
        records.effects = (0..14).map(|id| effect(id, id as u8)).collect();
        records.relations = (0..14).map(|id| relation(id + 1, id as i32)).collect();
    });
    let source = ItemBonusSources::new(&templates, None);
    let facts = source.template_facts(ITEM_ID).unwrap().unwrap();

    assert_eq!(facts.effect_ids, (0..14).collect::<Vec<_>>());
    let mut data = ItemBonusData::new();
    assert_eq!(
        data.initialize_from_instance(ITEM_ID, &[], &source),
        Err(ItemBonusDataError::EffectCapacityExceeded { effect_id: 13 })
    );
}

#[test]
fn bonus_rows_keep_id_order_and_missing_sources_are_loaded_absence() {
    let no_removals = removals([]);
    let templates = bonus_source_test_templates(ITEM_ID, &no_removals, |records| {
        records.items[0].class = 0;
        records.item_bonuses = vec![
            bonus(90, 7, 11, [90, 91, 92, 93], 0),
            bonus(10, 7, 12, [10, 11, 12, 13], 255),
            bonus(11, 7, 13, [11, 12, 13, 14], 1),
        ];
    });
    let source = ItemBonusSources::new(&templates, None);
    assert_eq!(source.actual_realm_patch(), None);
    let rows = source.bonus_list(7).unwrap().collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            ItemBonusEntry {
                bonus_type: 12,
                values: [10, 11, 12, 13],
            },
            ItemBonusEntry {
                bonus_type: 13,
                values: [11, 12, 13, 14],
            },
            ItemBonusEntry {
                bonus_type: 11,
                values: [90, 91, 92, 93],
            },
        ]
    );
    assert_eq!(source.bonus_list(0x8000_0007).unwrap().count(), 0);
    assert_eq!(source.template_facts(0x8000_0004).unwrap(), None);
    assert_eq!(source.item_squish_era(0).unwrap(), None);
}

#[test]
fn entity_bonus_data_uses_nested_scaling_and_crafted_level_sources() {
    let rows_to_remove = removals([(ITEM_SQUISH_ERA_HASH, 8, 2)]);
    let templates = bonus_source_test_templates(ITEM_ID, &rows_to_remove, |records| {
        records.items[0].class = 0;
        records.item_bonuses = vec![
            bonus(1, 300, 50, [301, 0, 0, 0], 0),
            bonus(2, 301, 51, [500, 4, 0, 0], 0),
            bonus(3, 301, 52, [100, 1, 80, 3], 0),
        ];
        records.item_scaling_configs = vec![ItemScalingConfigRecord {
            id: 500,
            offset_curve: 600,
            item_level: 170,
            required_level: 80,
            squish_era: 8,
            flags: 3,
        }];
        records.item_offset_curves = vec![ItemOffsetCurveRecord {
            id: 600,
            curve: i32::MIN,
            offset: -12,
        }];
        records.item_squish_eras = vec![ItemSquishEraRecord {
            id: 2,
            patch: 0,
            curve: 0,
            flags: 0,
        }];
        records.item_squish_era_file_last_index = Some(9);
    });

    let realm_source = ItemBonusSources::new(&templates, Some(0));
    let mut realm_data = ItemBonusData::new();
    realm_data
        .initialize_from_instance(ITEM_ID, &[300], &realm_source)
        .unwrap();
    assert_eq!(realm_data.scaling_config_uses_player_level(), true);
    assert_eq!(realm_data.item_level_offset_curve_id(), i32::MIN as u32);
    assert_eq!(realm_data.item_level_offset(), -12);
    assert_eq!(realm_data.item_squish_era_id(), 8);
    assert_eq!(
        realm_data.scaling_config_crafting_quality_item_level_bonus(),
        80
    );
    assert_eq!(realm_data.ignore_squish(), true);
    assert_eq!(realm_data.restrict_scaling_to_content_tuning(), true);

    let no_realm_source = ItemBonusSources::new(&templates, None);
    let mut no_realm_data = ItemBonusData::new();
    no_realm_data
        .initialize_from_instance(ITEM_ID, &[300], &no_realm_source)
        .unwrap();
    assert_eq!(
        no_realm_data.scaling_config_crafting_quality_item_level_bonus(),
        100
    );
}
