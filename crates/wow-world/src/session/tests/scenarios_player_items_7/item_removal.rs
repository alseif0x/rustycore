use super::*;

#[test]
fn destroyed_inventory_item_mod_remove_matches_cpp_destroy_item_equipment_branch() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            103,
            ItemStatEntry {
                stats: [
                    (ItemModType::Strength as i8, 12),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                ],
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_CHEST,
            InventoryItem {
                guid: item_guid,
                entry_id: 103,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(InventoryType::Chest as u8),
            },
        );
    let item = session.make_inventory_item_object(
        item_guid,
        103,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_CHEST, true);
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        12
    );
    let actions_before_destroy = session.represented_item_bonus_actions_like_cpp().len();

    assert!(session.record_destroyed_inventory_item_mod_remove_like_cpp(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        item_guid,
    ));

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        0,
        "C++ Player::DestroyItem calls _ApplyItemMods(pItem, slot, false) for bag 0 slots below INVENTORY_SLOT_BAG_END"
    );
    assert!(session.represented_item_bonus_actions_like_cpp().len() > actions_before_destroy);
}
#[test]
fn destroyed_inventory_item_mod_remove_skips_backpack_and_broken_items_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let backpack_item_guid = ObjectGuid::create_item(1, 904);
    let broken_item_guid = ObjectGuid::create_item(1, 905);
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            104,
            ItemStatEntry {
                stats: [
                    (ItemModType::Strength as i8, 12),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                ],
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));

    let backpack_item = session.make_inventory_item_object(
        backpack_item_guid,
        104,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(backpack_item);

    let mut broken_item = session.make_inventory_item_object(
        broken_item_guid,
        104,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    broken_item.set_max_durability(10);
    broken_item.set_durability(0);
    session.insert_inventory_item_object(broken_item);

    let actions_before = session.represented_item_bonus_actions_like_cpp().len();
    assert!(
        !session.record_destroyed_inventory_item_mod_remove_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            backpack_item_guid,
        )
    );
    assert!(
        !session.record_destroyed_inventory_item_mod_remove_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_CHEST,
            broken_item_guid,
        )
    );
    assert_eq!(
        session.represented_item_bonus_actions_like_cpp().len(),
        actions_before,
        "C++ _ApplyItemMods skips non-applied inventory slots and broken equipped items"
    );
}
