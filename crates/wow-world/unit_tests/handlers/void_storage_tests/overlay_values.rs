use super::*;

fn make_void_storage_session_with_detached_player() -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx, manager) = make_void_storage_session();
    let mut player = Box::new(wow_entities::Player::new(None, false));
    let guid = ObjectGuid::create_player(1, 42);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);
    let handle = manager
        .lock()
        .expect("canonical map manager lock")
        .install_detached_player_like_cpp(player)
        .expect("fresh detached Player installation");
    session.core.player_handle_like_cpp = Some(handle);
    assert!(session.core.with_owned_player_like_cpp(|_| ()).is_some());
    (session, send_rx, manager)
}

#[test]
fn direct_storage_overlay_replaces_a_vacated_slots_old_entry_like_cpp() {
    let (mut session, _, _) = make_void_storage_session_with_detached_player();
    let old_entry = 19_019;
    let replacement_entry = 19_020;
    install_void_test_item_template_with_stack(&mut session, replacement_entry, 20);
    let player_guid = ObjectGuid::create_player(1, 42);
    let slot = INVENTORY_SLOT_ITEM_START;
    let guid = ObjectGuid::create_item(1, 720);
    let mut old_item = session.make_inventory_item_object(
        guid,
        old_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    old_item.set_count(19);
    session.insert_inventory_item_object(old_item);
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid,
            entry_id: old_entry,
            db_guid: 720,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    session
        .core
        .with_owned_player_mut_like_cpp(|player| player.store_top_level_item(slot, guid))
        .expect("detached Player owner")
        .expect("valid top-level slot");

    let overlay = DirectInventoryStorageOverlayLikeCpp {
        bag: INVENTORY_SLOT_BAG_0,
        slot,
        entry_id: replacement_entry,
        count: 10,
    };
    let (result, destinations, _) =
        wow_world_application::plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            &session,
            replacement_entry,
            1,
            &[overlay],
            &[(INVENTORY_SLOT_BAG_0, slot)],
        )
        .expect("represented storage planner");

    assert_eq!(result, wow_constants::InventoryResult::Ok);
    assert_eq!(
        destinations,
        vec![wow_entities::ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(slot),
            1,
        )],
        "the replacement entry merges into its overlay stack at the vacated position"
    );
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .map(|item| (item.object().entry(), item.count()))
        }),
        Some(Some((old_entry, 19))),
        "query overlay values do not mutate the canonical Item's entry or count"
    );
}

#[test]
fn direct_storage_overlay_bag_placeholder_supplies_child_storage_like_cpp() {
    let (mut session, _, _) = make_void_storage_session_with_detached_player();
    let bag_entry = 21_841;
    let child_entry = 19_019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, child_entry);
    let player_guid = ObjectGuid::create_player(1, 42);
    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 730 + i64::from(offset));
        let item = session.make_inventory_item_object(
            guid,
            child_entry,
            player_guid,
            1,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid,
                entry_id: child_entry,
                db_guid: guid.counter() as u64,
                inventory_type: Some(InventoryType::NonEquip as u8),
            },
        );
        session
            .core
            .with_owned_player_mut_like_cpp(|player| player.store_top_level_item(slot, guid))
            .expect("detached Player owner")
            .expect("valid top-level slot");
    }

    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let overlay = DirectInventoryStorageOverlayLikeCpp {
        bag: INVENTORY_SLOT_BAG_0,
        slot: bag_slot,
        entry_id: bag_entry,
        count: 1,
    };
    let (result, destinations, _) =
        wow_world_application::plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            &session,
            child_entry,
            1,
            &[overlay],
            &[],
        )
        .expect("represented storage planner with overlay bag");

    assert_eq!(result, wow_constants::InventoryResult::Ok);
    assert_eq!(destinations.len(), 1);
    let [bag, slot] = destinations[0].pos.to_be_bytes();
    assert_eq!(
        bag, bag_slot,
        "the overlay's bag placeholder owns the result"
    );
    assert_eq!(slot, 0, "the child is stored in the first overlay bag slot");
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.top_level_item_guid(bag_slot),
            player.get_bag_by_pos(bag_slot),
            player.get_item_by_pos(bag_slot, 0),
        )),
        Some((None, None, None)),
        "the query-only placeholder never becomes canonical Player storage"
    );
}
