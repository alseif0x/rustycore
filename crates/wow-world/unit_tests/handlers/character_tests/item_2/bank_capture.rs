use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START, Item, ItemPosCount,
    ItemValuesMapSnapshot, ItemValuesSnapshot, Player, PlayerInventoryItem,
};
use wow_map::{MapKey, MapManager, PlayerHandle};

const MAP: MapKey = MapKey::new(571, 0);

fn player_guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 42)
}

fn bag_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 95_001)
}

fn source_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 95_002)
}

fn raw_nested_key() -> ObjectGuid {
    ObjectGuid::new(0xAA, 0x95_003)
}

fn record(guid: ObjectGuid, entry_id: u32, inventory_type: InventoryType) -> PlayerInventoryItem {
    PlayerInventoryItem {
        guid,
        entry_id,
        db_guid: guid.counter() as u64,
        inventory_type: Some(inventory_type as u8),
    }
}

fn item(guid: ObjectGuid, entry_id: u32, owner_guid: ObjectGuid, slot: u8, count: u32) -> Item {
    let mut item = Item::new(0);
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry_id);
    item.set_owner_guid(owner_guid);
    item.set_contained_in(owner_guid);
    item.set_slot(slot);
    item.set_count(count);
    item
}

fn install_bank_catalog(session: &mut WorldSession) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([
        ItemRecord {
            id: 120,
            class_id: ItemClass::Miscellaneous as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 200,
            class_id: ItemClass::Container as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    let sparse = |entry_id, stackable, container_slots, inventory_type| {
        (
            entry_id,
            ItemSparseTemplateEntry {
                flags: [0; 4],
                bag_family: 0,
                start_quest_id: 0,
                stackable,
                max_count: 0,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                other_faction_item_id: 0,
                content_tuning_id: 0,
                player_level_to_item_level_curve_id: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0; 2],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: ItemBondingType::None as u8,
                container_slots,
                inventory_type: inventory_type as i8,
            },
        )
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        sparse(120, 20, 0, InventoryType::NonEquip),
        sparse(200, 1, 4, InventoryType::Bag),
    ])));
}

fn install_owner(
    session: &mut WorldSession,
    player: Player,
    active: bool,
) -> (Arc<Mutex<MapManager>>, PlayerHandle) {
    let guid = player.guid();
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    if active {
        manager.create_world_map(MAP.map_id, MAP.instance_id);
    }
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .expect("real Player should install in the owner manager");
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .expect("real Player should attach to the current map");
    }
    let manager = Arc::new(Mutex::new(manager));
    session.set_player_guid(Some(guid));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.core.current_map_id = MAP.map_id as u16;
    session.core.player_handle_like_cpp = Some(handle);
    (manager, handle)
}

fn empty_player(guid: ObjectGuid) -> Player {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.set_inventory_slot_count(16);
    player
}

fn seed_source(player: &mut Player, guid: ObjectGuid, count: u32) {
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, guid)
        .expect("source should occupy a valid top-level slot");
    let owner_guid = player.guid();
    let runtime = player.inventory_runtime_mut_like_cpp();
    runtime.store_item_in_slot_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        record(guid, 120, InventoryType::NonEquip),
    );
    runtime.store_item_object_like_cpp(item(
        guid,
        120,
        owner_guid,
        INVENTORY_SLOT_ITEM_START,
        count,
    ));
}

fn owner_facts(
    session: &WorldSession,
) -> (
    HashMap<u8, PlayerInventoryItem>,
    Vec<(ObjectGuid, ItemValuesSnapshot)>,
    [Option<ObjectGuid>; 4],
) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            let records = runtime.inventory_items().clone();
            let values = ItemValuesMapSnapshot::capture(runtime.item_objects());
            let ordered_values = values
                .iter()
                .map(|(&guid, item)| (guid, item.clone()))
                .collect();
            let positions = [
                player.top_level_item_guid(INVENTORY_SLOT_ITEM_START),
                player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                player.get_bag_by_pos(INVENTORY_SLOT_BAG_START),
                player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
            ];
            (records, ordered_values, positions)
        })
        .expect("current Player owner should remain bound")
}

fn bank_plan(
    session: &WorldSession,
    source_bag: u8,
    source_slot: u8,
) -> Option<(InventoryResult, Vec<ItemPosCount>)> {
    session.plan_bank_existing_inventory_item_at_like_cpp(
        source_bag,
        source_slot,
        INVENTORY_SLOT_BAG_0,
        wow_entities::BANK_SLOT_ITEM_START,
        false,
    )
}

fn add_fixture_source(session: &mut WorldSession, db_guid: u64, count: u32) -> ObjectGuid {
    session.set_player_guid(Some(player_guid()));
    install_bank_move_item_fixture(session, 120, 20);
    insert_bank_move_test_item(session, INVENTORY_SLOT_ITEM_START, 120, db_guid, count)
}

#[test]
fn canonical_bank_planning_uses_frozen_owner_facts_for_active_and_detached_players() {
    for active in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let fixture_guid = add_fixture_source(&mut session, 95_010, 9);
        let mut player = empty_player(player_guid());
        seed_source(&mut player, source_guid(), 3);
        let _ = install_owner(&mut session, player, active);

        let before = owner_facts(&session);
        let (result, destinations) =
            bank_plan(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
                .expect("canonical source should produce a bank plan");
        assert_eq!(result, InventoryResult::Ok);
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].count, 3);
        assert_ne!(fixture_guid, source_guid());
        assert_eq!(owner_facts(&session), before, "planning is read-only");
        assert!(send_rx.try_recv().is_err(), "planning sends no packet");
    }
}

#[test]
fn canonical_empty_and_stale_handles_do_not_use_poisoned_fixture_facts() {
    let (mut empty_session, send_rx) = make_session_with_send_capacity(1);
    let _fixture_guid = add_fixture_source(&mut empty_session, 95_020, 9);
    let empty = empty_player(player_guid());
    let _ = install_owner(&mut empty_session, empty, true);
    assert!(
        bank_plan(
            &empty_session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .is_none()
    );
    assert!(send_rx.try_recv().is_err());

    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let _fixture_guid = add_fixture_source(&mut session, 95_021, 9);
    let mut original = empty_player(player_guid());
    seed_source(&mut original, source_guid(), 3);
    let (manager, stale_handle) = install_owner(&mut session, original, false);
    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(stale_handle)
        .expect("old Player generation should retire");
    let mut replacement = empty_player(player_guid());
    let replacement_guid = ObjectGuid::create_item(1, 95_022);
    seed_source(&mut replacement, replacement_guid, 2);
    let current_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("same-GUID replacement should install");
    assert_ne!(current_handle, stale_handle);
    assert!(
        bank_plan(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START,).is_none(),
        "stale binding must not fall back to the poisoned fixture"
    );

    session.core.player_handle_like_cpp = Some(current_handle);
    let (result, destinations) =
        bank_plan(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .expect("current replacement generation should resolve");
    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].count, 2);
    assert!(send_rx.try_recv().is_err());
}

fn seed_bag_and_nested(
    session: &mut WorldSession,
    bag_item_guid: ObjectGuid,
    nested: &[(ObjectGuid, ObjectGuid)],
) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_item_guid)
                .unwrap();
            player
                .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_item_guid, 4)
                .unwrap();
            let owner_guid = player.guid();
            let runtime = player.inventory_runtime_mut_like_cpp();
            runtime.store_item_in_slot_like_cpp(
                INVENTORY_SLOT_BAG_START,
                record(bag_item_guid, 200, InventoryType::Bag),
            );
            runtime.store_item_object_like_cpp(item(
                bag_item_guid,
                200,
                owner_guid,
                INVENTORY_SLOT_BAG_START,
                1,
            ));
            for &(map_key, embedded_guid) in nested {
                let mut child = item(embedded_guid, 120, owner_guid, 2, 1);
                child.set_container_guid_and_slot(bag_item_guid, INVENTORY_SLOT_BAG_START);
                runtime.item_objects_mut().insert(map_key, child);
            }
        })
        .expect("canonical owner should accept the real bag topology");
}

fn old_two_clone_nested_winner(session: &WorldSession) -> (ObjectGuid, u32) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp().clone();
            let item_objects = runtime.item_objects().clone();
            item_objects
                .values()
                .find(|item| item.container_guid() == bag_guid() && item.slot() == 2)
                .map(|item| (item.object().guid(), item.object().entry()))
        })
        .expect("current Player owner should exist")
        .expect("collision fixture has a matching nested item")
}

#[test]
fn nested_lookup_keeps_first_old_clone_collision_and_embedded_key_failure() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(player_guid()));
    install_bank_catalog(&mut session);
    let player = empty_player(player_guid());
    let _ = install_owner(&mut session, player, false);
    let first_guid = ObjectGuid::create_item(1, 95_031);
    let second_guid = ObjectGuid::create_item(1, 95_032);
    seed_bag_and_nested(
        &mut session,
        bag_guid(),
        &[(first_guid, first_guid), (second_guid, second_guid)],
    );
    let expected = old_two_clone_nested_winner(&session);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_bag_item(INVENTORY_SLOT_BAG_START, 2, expected.0)
                .unwrap();
        })
        .expect("bag slot should match the old traversal winner");
    let nested = session
        .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 2)
        .expect("nested item should resolve from the ordered values read");
    assert_eq!((nested.guid, nested.entry_id), expected);
    let before_plan = owner_facts(&session);
    let (result, destinations) = bank_plan(&session, INVENTORY_SLOT_BAG_START, 2)
        .expect("valid nested source should produce a bank plan");
    assert_eq!(result, InventoryResult::Ok);
    assert_eq!(destinations.len(), 1);
    assert_eq!(destinations[0].count, 1);
    assert_eq!(owner_facts(&session), before_plan);
    assert!(send_rx.try_recv().is_err());

    let raw_key = raw_nested_key();
    let embedded_guid = ObjectGuid::create_item(1, 95_033);
    let (mut malformed_session, _) = make_session_with_send_capacity(1);
    malformed_session.set_player_guid(Some(player_guid()));
    install_bank_catalog(&mut malformed_session);
    let player = empty_player(player_guid());
    let _ = install_owner(&mut malformed_session, player, false);
    seed_bag_and_nested(
        &mut malformed_session,
        bag_guid(),
        &[(raw_key, embedded_guid)],
    );
    malformed_session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_bag_item(INVENTORY_SLOT_BAG_START, 2, embedded_guid)
                .unwrap();
        })
        .expect("adversarial fixture keeps the embedded GUID in topology");
    assert_eq!(
        malformed_session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 2)
            .map(|item| item.guid),
        Some(embedded_guid),
        "nested record projection uses the embedded Item GUID"
    );
    assert!(
        bank_plan(&malformed_session, INVENTORY_SLOT_BAG_START, 2,).is_none(),
        "the subsequent selected-key read fails when only the raw map key exists"
    );
    assert_ne!(raw_key, embedded_guid);
}
