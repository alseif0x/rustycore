use super::*;

#[test]
fn represented_item_set_heirloom_max_level_guard_matches_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 918);
    let hands_guid = ObjectGuid::create_item(1, 919);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(19);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 709,
        name: "Heirloom Curve Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 112,
            1 => 113,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 33,
            chr_spec_id: 0,
            spell_id: 9033,
            threshold: 2,
            item_set_id: 709,
        },
    ])));
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([
        HeirloomEntry {
            id: 112,
            source_text: "test".to_string(),
            item_id: 112,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 0,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
        HeirloomEntry {
            id: 113,
            source_text: "test".to_string(),
            item_id: 113,
            legacy_upgraded_item_id: 0,
            static_upgraded_item_id: 0,
            source_type_enum: 0,
            flags: 0,
            legacy_item_id: 0,
            upgrade_item_id: [0; 6],
            upgrade_item_bonus_list_id: [0; 6],
        },
    ])));
    let sparse = ItemSparseTemplateEntry {
        flags: [0; 4],
        price_random_value: 0.0,
        content_tuning_id: 55,
        player_level_to_item_level_curve_id: 77,
        zone_bound: [0; 2],
        allowable_class: 0,
        ..inventory_sparse_template_for_test(InventoryType::Chest as i8)
    };
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(112, sparse), (113, sparse)],
            [],
        ),
    ));
    session.set_curve_store(Arc::new(CurveStore::from_entries([CurveEntry {
        id: 77,
        curve_type: 0,
        flags: 0,
    }])));
    session.set_curve_point_store(Arc::new(CurvePointStore::from_entries([
        CurvePointEntry {
            id: 1,
            pos: [1.0, 10.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: 77,
            order_index: 0,
        },
        CurvePointEntry {
            id: 2,
            pos: [20.0, 20.0],
            pre_sl_squish_pos: [0.0, 0.0],
            curve_id: 77,
            order_index: 1,
        },
    ])));
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        ContentTuningEntry {
            id: 55,
            min_level: 1,
            max_level: 18,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        112,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        113,
        InventoryType::Hands,
    );

    assert!(
        !session.record_represented_items_set_item_like_cpp(chest_guid, true),
        "C++ AddItemsSetItem returns before creating the set effect when player level exceeds heirloom max level"
    );
    assert!(session.represented_item_set_effect_like_cpp(709).is_none());

    session.set_player_level_like_cpp(18);
    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 709,
            spell_entry_id: 33,
            spell_id: 9033,
            threshold: 2,
            apply: true,
        }],
        "C++ only blocks heirloom item-set bonuses when player level is greater than the derived max level"
    );
}
#[test]
fn represented_item_set_uses_primary_spec_not_loot_spec_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 917);
    session.set_player_guid(Some(player_guid));
    session.set_loot_specialization_id_like_cpp(66);
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 708,
        name: "Primary Spec Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| if i == 0 { 111 } else { 0 }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 32,
            chr_spec_id: 66,
            spell_id: 9032,
            threshold: 1,
            item_set_id: 708,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
        111,
        InventoryType::Chest,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(item_guid, true));
    assert!(
        session
            .represented_item_set_spell_events_like_cpp()
            .is_empty(),
        "C++ HandleSetLootSpecialization changes LootSpecID only; item-set ChrSpecID uses GetPrimarySpecialization"
    );

    session.set_represented_primary_specialization_id_like_cpp(66);
    assert_eq!(
        session.record_represented_update_item_set_auras_like_cpp(false),
        2
    );
    assert_eq!(
        session.represented_item_set_aura_refresh_events_like_cpp(),
        &[
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 708,
                spell_entry_id: 32,
                spell_id: 9032,
                apply: false,
                form_change: false,
            },
            RepresentedItemSetAuraRefreshEventLikeCpp {
                item_set_id: 708,
                spell_entry_id: 32,
                spell_id: 9032,
                apply: true,
                form_change: false,
            },
        ],
        "C++ UpdateItemSetAuras uses the current primary specialization, not LootSpecID"
    );
}
