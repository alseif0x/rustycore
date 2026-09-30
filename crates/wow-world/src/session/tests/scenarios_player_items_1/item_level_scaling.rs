use super::*;

#[test]
fn represented_item_level_uses_player_level_curve_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_158_u32;
    session.set_player_level_like_cpp(45);
    install_represented_item_level_curve_fixture_like_cpp(&mut session, item_id, 0, 9_001);

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(145),
        "C++ Item::GetItemLevel replaces template item level with DB2Manager::GetCurveValueAt(PlayerLevelToItemLevelCurveId, owner level) before bonus/caps"
    );
}
#[test]
fn represented_item_level_curve_clamps_owner_level_by_content_tuning_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_159_u32;
    session.set_player_level_like_cpp(80);
    install_represented_item_level_curve_fixture_like_cpp(&mut session, item_id, 55, 9_002);
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 55,
            min_level: 10,
            max_level: 40,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(140),
        "C++ clamps owner level through GetContentTuningData(contentTuningId, true) before evaluating the item-level curve"
    );
}
#[test]
fn represented_item_level_curve_fixed_level_overrides_content_tuning_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_160_u32;
    let item_guid = ObjectGuid::create_item(1, 30_160);
    session.set_player_level_like_cpp(80);
    install_represented_item_level_curve_fixture_like_cpp(&mut session, item_id, 56, 9_003);
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 56,
            min_level: 10,
            max_level: 40,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));
    let owner = session.player_guid().unwrap_or(ObjectGuid::EMPTY);
    let mut item = session.make_inventory_item_object(
        item_guid,
        item_id,
        owner,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_modifier(ItemModifier::TimewalkerLevel, 50);

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, Some(&item)),
        Some(150),
        "C++ fixedLevel/ITEM_MODIFIER_TIMEWALKER_LEVEL bypasses ContentTuning clamp for Item::GetItemLevel"
    );
}
#[test]
fn represented_item_level_curve_missing_data_does_not_fall_back_to_template_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_161_u32;
    session.set_player_level_like_cpp(45);
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_with_scaling_like_cpp(InventoryType::Chest, 0, 0, 9_004),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(WorldSession::MIN_ITEM_LEVEL_LIKE_CPP),
        "C++ GetCurveValueAt returns 0 for missing curve data; Item::GetItemLevel then clamps to MIN_ITEM_LEVEL instead of falling back to proto ItemLevel"
    );
}
#[test]
fn represented_item_level_applies_min_cap_to_equipable_item_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_049_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level: 150,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(150),
        "C++ Item::GetItemLevel(owner) raises equipable items below MinItemLevel"
    );
}
#[test]
fn represented_item_level_respects_min_cap_cutoff_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_050_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 100,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level_cutoff: 120,
        min_item_level: 150,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(100),
        "C++ only applies MinItemLevel when itemLevelBeforeUpgrades reaches MinItemLevelCutoff"
    );

    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        min_item_level_cutoff: 90,
        min_item_level: 150,
        ..Default::default()
    });
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(150),
        "C++ cutoff uses itemLevelBeforeUpgrades, not the capped item level"
    );
}
#[test]
fn represented_item_level_applies_max_cap_to_equipable_item_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_051_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 200,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        max_item_level: 120,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(120),
        "C++ Item::GetItemLevel(owner) caps equipable items above MaxItemLevel"
    );
}
#[test]
fn represented_item_level_ignores_max_cap_with_pvp_cap_flag_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_052_u32;
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                item_id,
                sparse_template_for_inventory_type_like_cpp(
                    InventoryType::Chest,
                    ItemFlags3::IgnoreItemLevelCapInPvp as u32,
                ),
            )],
            [(
                item_id,
                ItemRandomPropertyTemplateEntry {
                    item_level: 200,
                    quality: ItemQuality::Epic as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    session.set_represented_item_level_caps_like_cpp(RepresentedItemLevelCapsLikeCpp {
        max_item_level: 120,
        ..Default::default()
    });

    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(200),
        "C++ clears maxItemLevel when ITEM_FLAG3_IGNORE_ITEM_LEVEL_CAP_IN_PVP is present"
    );
}
#[test]
fn represented_item_level_area_scaling_activates_on_battleground_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let item_id = 30_154_u32;
    install_represented_pvp_item_level_fixture_like_cpp(&mut session, item_id, 25);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        represented_item_level_area_map_like_cpp(489, wow_data::map::MAP_BATTLEGROUND, 0),
    ])));
    session.set_player_map_position_like_cpp(489, Position::ZERO);

    assert!(session.update_represented_item_level_area_based_scaling_like_cpp());
    assert!(session.represented_using_pvp_item_levels_like_cpp());
    assert_eq!(
        session.represented_item_level_like_cpp(item_id, None),
        Some(125)
    );
}
