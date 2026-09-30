//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[tokio::test]
async fn represented_fishing_node_junk_loot_uses_default_zone_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_052);
    let item_id = 80_002;
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);
    install_limited_test_item_template(&mut session, item_id, 0);
    let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
    fishing_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 1,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_JUNK_FISH_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Fishing, fishing_store);
    session.set_loot_stores(Arc::new(stores));

    prepare_money_player_residence_for_test(&mut session);
    open_fishing_loot_cycle_for_test(&mut session, gameobject_guid, 77, true)
        .await;

    let loot = loot_for_test(&session, gameobject_guid).unwrap();
    assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_JUNK_LIKE_CPP);
    assert_eq!(loot.items.len(), 1);
    assert_eq!(loot.items[0].item_id, item_id);
    assert!(is_active_loot_guid_for_test(&session, gameobject_guid));
}

#[tokio::test]
async fn represented_fishing_node_loot_walks_parent_area_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_051);
    let item_id = 80_001;
    session.set_player_guid(Some(player_guid));
    wow_world::test_fixtures::insert_client_visible_guid_for_test(&mut session, gameobject_guid);
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        AreaTableEntry {
            id: 77,
            continent_id: 0,
            parent_area_id: 10,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        AreaTableEntry {
            id: 10,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    install_limited_test_item_template(&mut session, item_id, 0);
    let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
    fishing_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: 10,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Fishing, fishing_store);
    session.set_loot_stores(Arc::new(stores));

    prepare_money_player_residence_for_test(&mut session);
    open_fishing_loot_cycle_for_test(&mut session, gameobject_guid, 77, false)
        .await;

    let loot = loot_for_test(&session, gameobject_guid).unwrap();
    assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_LIKE_CPP);
    assert_eq!(loot.items.len(), 1);
    assert_eq!(loot.items[0].item_id, item_id);
    assert!(is_active_loot_guid_for_test(&session, gameobject_guid));
}

