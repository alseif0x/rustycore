use super::*;

#[tokio::test]
async fn repair_inventory_item_durability_spends_money_and_restores_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(2_000);
    session.set_item_store(Arc::new(ItemStore::from_records([represented_test_item_record_like_cpp(
        100,
        InventoryType::Weapon,
        ItemClass::Weapon,
        7,
    )])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
                100,
                ItemSparseTemplateEntry {
                    flags: [0; 4],
                    price_random_value: 0.0,
                    max_durability: 50,
                    zone_bound: [0; 2],
                    allowable_class: 0,
                    ..inventory_sparse_template_for_test(InventoryType::Weapon as i8)
                },
            )],
            [(
                100,
                ItemRandomPropertyTemplateEntry {
                    item_level: 57,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
        ),
    ));
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 57,
            weapon_sub_class_cost: std::array::from_fn(|i| if i == 7 { 13 } else { 0 }),
            armor_sub_class_cost: [0; 8],
        },
    ])));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry {
            id: (ItemQuality::Rare as u32 + 1) * 2,
            data: 1.25,
        },
    ])));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryItem {
                guid: item_guid,
                entry_id: 100,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(InventoryType::Weapon as u8),
            },
        );
    let item = session.make_inventory_item_object(
        item_guid,
        100,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(item);

    assert!(
        session
            .repair_inventory_item_durability_like_cpp(item_guid, true, 0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 700);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid,
            slot: EQUIPMENT_SLOT_MAINHAND,
            apply: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject, ServerOpcodes::UpdateObject],
        "C++ DurabilityRepair sets item durability and reapplies equipped broken-item mods, both visible through update fields"
    );

    session.set_player_gold_like_cpp(10);
    let item = session.inventory_item_objects.get_mut(&item_guid).unwrap();
    item.set_durability(40);
    assert!(
        !session
            .repair_inventory_item_durability_like_cpp(item_guid, true, 0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 10);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .durability,
        40
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
