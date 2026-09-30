use super::*;

#[test]
fn open_item_get_inventory_item_by_pos_resolves_top_level_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));

    let top_guid = ObjectGuid::create_item(1, 900);
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            23,
            InventoryItem {
                guid: top_guid,
                entry_id: 700,
                db_guid: 900,
                inventory_type: None,
            },
        );
    let top_item =
        session.make_inventory_item_object(top_guid, 700, player_guid, 1, 0, ItemContext::None, 23);
    session.insert_inventory_item_object(top_item);

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 23)
            .map(|i| i.guid),
        Some(top_guid)
    );
}

#[test]
fn open_item_get_inventory_item_by_pos_excludes_buyback_top_level_like_cpp() {
    let (mut session, _, _) = make_session();
    session
        .player_item_test_fixture_like_cpp
        .buyback_items
        .insert(
            BUYBACK_SLOT_START,
            InventoryItem {
                guid: ObjectGuid::create_item(1, 901),
                entry_id: 701,
                db_guid: 901,
                inventory_type: None,
            },
        );

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, BUYBACK_SLOT_START)
            .is_none()
    );
}

#[test]
fn open_item_get_inventory_item_by_pos_resolves_nested_carried_bag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (_, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 5)
            .map(|i| i.guid),
        Some(child_guid)
    );
}

#[test]
fn open_item_get_inventory_item_by_pos_resolves_nested_bank_bag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (_, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, BANK_SLOT_BAG_START, 5);

    assert_eq!(
        session
            .get_inventory_item_by_pos(BANK_SLOT_BAG_START, 5)
            .map(|i| i.guid),
        Some(child_guid)
    );
}

#[test]
fn open_item_get_inventory_item_by_pos_resolves_nested_reagent_bag_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (_, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, REAGENT_BAG_SLOT_START, 5);

    assert_eq!(
        session
            .get_inventory_item_by_pos(REAGENT_BAG_SLOT_START, 5)
            .map(|i| i.guid),
        Some(child_guid)
    );
}

#[test]
fn open_item_get_inventory_item_by_pos_missing_bag_or_empty_slot_is_missing() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START + 1, 0)
            .is_none()
    );
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 3)
            .is_none()
    );
}

#[test]
fn open_item_nested_item_preserves_top_level_bag_slot_and_inner_slot() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let (bag_guid, child_guid) =
        insert_open_item_bag_with_child(&mut session, player_guid, INVENTORY_SLOT_BAG_START, 5);

    let child = session.inventory_item_objects.get(&child_guid).unwrap();
    assert_eq!(child.container_guid(), bag_guid);
    assert_eq!(child.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(child.slot(), 5);
    assert_eq!(
        child.position(),
        u16::from(INVENTORY_SLOT_BAG_START) << 8 | 5
    );
}

#[tokio::test]
async fn open_item_nested_has_loot_opens_without_internal_bag_error() {
    assert_open_item_nested_has_loot_opens_without_internal_bag_error(INVENTORY_SLOT_BAG_START)
        .await;
}

#[tokio::test]
async fn open_item_nested_bank_bag_has_loot_opens_without_internal_bag_error() {
    assert_open_item_nested_has_loot_opens_without_internal_bag_error(BANK_SLOT_BAG_START).await;
}

#[tokio::test]
async fn open_item_nested_reagent_bag_has_loot_opens_without_internal_bag_error() {
    assert_open_item_nested_has_loot_opens_without_internal_bag_error(REAGENT_BAG_SLOT_START).await;
}

#[tokio::test]
async fn open_item_wrapped_without_has_loot_does_not_generate_loot_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    install_open_item_template_with_flags(&mut session, 700, ItemFlags::empty(), 0);
    insert_open_item_top_level(&mut session, player_guid, 23, item_guid, 700, true);
    session
        .inventory_item_objects
        .get_mut(&item_guid)
        .unwrap()
        .set_item_flag(ItemFieldFlags::WRAPPED);

    session
        .handle_open_item(WorldPacket::from_bytes(&[INVENTORY_SLOT_BAG_0, 23]))
        .await;

    assert!(!session.loot_table.contains_key(&item_guid));
    assert!(send_rx.try_recv().is_err());
    assert!(
        session
            .inventory_item_objects
            .get(&item_guid)
            .is_some_and(|item| !item.loot_generated() && item.is_wrapped())
    );
}
