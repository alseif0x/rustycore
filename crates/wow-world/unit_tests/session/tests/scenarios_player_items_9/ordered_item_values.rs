use super::*;
use std::sync::{Arc, Mutex};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    EQUIPMENT_SLOT_CHEST, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START, Item, Player,
    PlayerInventoryItem,
};
use wow_map::{MIN_GRID_DELAY_MS, MapKey, MapManager, PlayerHandle};
use wow_world_core::session::SessionHubHostLikeCpp;

const MAP: MapKey = MapKey::new(571, 0);

fn player_guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 42)
}

fn equipment_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 94_001)
}

fn bag_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 94_002)
}

fn backpack_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 94_003)
}

fn nested_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 94_004)
}

fn nested_raw_key() -> ObjectGuid {
    ObjectGuid::new(0xAA, 0x94_005)
}

fn poison_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 94_006)
}

type SharedManager = Arc<Mutex<MapManager>>;

fn item(guid: ObjectGuid, entry: u32, slot: u8, durability: u32, maximum: u32) -> Item {
    let mut item = Item::new(0);
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry);
    item.set_slot(slot);
    item.set_count(1);
    item.set_durability(durability);
    item.set_max_durability(maximum);
    item
}

fn record(guid: ObjectGuid, entry_id: u32) -> PlayerInventoryItem {
    PlayerInventoryItem {
        guid,
        entry_id,
        db_guid: guid.counter() as u64,
        inventory_type: None,
    }
}

fn install_poisoned_fixture(session: &mut WorldSession) {
    session
        .inventory
        .insert_inventory_item_for_test_like_cpp(EQUIPMENT_SLOT_CHEST, record(poison_guid(), 100));
    session.insert_inventory_item_object(item(poison_guid(), 100, EQUIPMENT_SLOT_CHEST, 20, 40));
}

fn install_canonical_owner(
    session: &mut WorldSession,
    active: bool,
) -> (SharedManager, PlayerHandle) {
    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid());
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 10);
    if active {
        manager.create_world_map(MAP.map_id, MAP.instance_id);
    }
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .expect("canonical Player should install in the real manager");
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .expect("canonical Player should attach to its current map");
    }
    let manager = Arc::new(Mutex::new(manager));
    session.set_player_guid(Some(player_guid()));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.core.current_map_id = MAP.map_id as u16;
    session.core.player_handle_like_cpp = Some(handle);
    (manager, handle)
}

fn seed_canonical_inventory(session: &mut WorldSession) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_inventory_slot_count(16);
            player
                .store_top_level_item(EQUIPMENT_SLOT_CHEST, equipment_guid())
                .unwrap();
            player
                .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid())
                .unwrap();
            player
                .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid(), 4)
                .unwrap();
            player
                .store_bag_item(INVENTORY_SLOT_BAG_START, 3, nested_guid())
                .unwrap();
            player
                .store_top_level_item(INVENTORY_SLOT_ITEM_START, backpack_guid())
                .unwrap();

            let runtime = player.inventory_runtime_mut_like_cpp();
            runtime
                .store_item_in_slot_like_cpp(EQUIPMENT_SLOT_CHEST, record(equipment_guid(), 100));
            runtime.store_item_in_slot_like_cpp(INVENTORY_SLOT_BAG_START, record(bag_guid(), 200));
            runtime.store_item_in_slot_like_cpp(
                INVENTORY_SLOT_ITEM_START,
                record(backpack_guid(), 100),
            );
            runtime.store_item_object_like_cpp(item(
                equipment_guid(),
                100,
                EQUIPMENT_SLOT_CHEST,
                20,
                40,
            ));
            runtime.store_item_object_like_cpp(item(
                bag_guid(),
                200,
                INVENTORY_SLOT_BAG_START,
                0,
                0,
            ));
            runtime.store_item_object_like_cpp(item(
                backpack_guid(),
                100,
                INVENTORY_SLOT_ITEM_START,
                20,
                40,
            ));
            let mut nested = item(nested_guid(), 100, 3, 10, 30);
            nested.set_container_guid_and_slot(bag_guid(), INVENTORY_SLOT_BAG_START);
            runtime.item_objects_mut().insert(nested_raw_key(), nested);
        })
        .expect("current Player generation should accept inventory facts");
}

fn install_repair_catalog(session: &mut WorldSession) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 100,
        class_id: ItemClass::Armor as u8,
        subclass_id: 4,
        material: 0,
        inventory_type: InventoryType::Chest as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    let sparse = ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 1.0,
        price_random_value: 0.0,
        max_durability: 40,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: 0,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: InventoryType::Chest as i8,
    };
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [(
                100,
                ItemStatEntry {
                    stats: [(-1, 0); 10],
                    resistances: [0; 7],
                    armor: 0,
                },
            )],
            [(100, sparse)],
            [(
                100,
                ItemRandomPropertyTemplateEntry {
                    item_level: 50,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            )],
        ),
    ));
    let mut armor_cost = [0; 8];
    armor_cost[4] = 10;
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 50,
            weapon_sub_class_cost: [0; 21],
            armor_sub_class_cost: armor_cost,
        },
    ])));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry { id: 8, data: 1.0 },
    ])));
}

fn query_targets(session: &WorldSession) -> Vec<(ObjectGuid, u8, bool)> {
    let hub = session.hub_ref();
    session
        .inventory
        .represented_durability_targets_like_cpp(hub, true)
}

fn query_repair_costs(session: &WorldSession) -> Option<Vec<(ObjectGuid, u64)>> {
    let hub = session.hub_ref();
    session
        .inventory
        .repairable_inventory_item_costs_like_cpp(hub, 1.0, 1.0)
}

#[test]
fn durability_and_repair_queries_use_canonical_values_and_records() {
    for active in [false, true] {
        let (mut session, _, _) = make_session();
        install_poisoned_fixture(&mut session);
        install_repair_catalog(&mut session);
        install_canonical_owner(&mut session, active);
        seed_canonical_inventory(&mut session);

        let ordered_targets = query_targets(&session);
        assert_eq!(
            ordered_targets,
            vec![
                (equipment_guid(), EQUIPMENT_SLOT_CHEST, true),
                (bag_guid(), INVENTORY_SLOT_BAG_START, true),
                (backpack_guid(), INVENTORY_SLOT_ITEM_START, false),
                (nested_raw_key(), 3, false),
            ],
            "equipment, backpack, and carried-bag targets retain production ordering"
        );
        let mut targets = ordered_targets;
        targets.sort_unstable_by_key(|target| target.0);
        let mut expected_targets = vec![
            (equipment_guid(), EQUIPMENT_SLOT_CHEST, true),
            (bag_guid(), INVENTORY_SLOT_BAG_START, true),
            (nested_raw_key(), 3, false),
            (backpack_guid(), INVENTORY_SLOT_ITEM_START, false),
        ];
        expected_targets.sort_unstable_by_key(|target| target.0);
        assert_eq!(targets, expected_targets);
        assert!(!targets.iter().any(|(guid, _, _)| *guid == poison_guid()));

        let mut costs = query_repair_costs(&session).expect("present Player yields costs");
        costs.sort_unstable_by_key(|(guid, _)| *guid);
        let mut expected_costs = vec![
            (equipment_guid(), 200),
            (backpack_guid(), 200),
            (nested_guid(), 200),
        ];
        expected_costs.sort_unstable_by_key(|(guid, _)| *guid);
        assert_eq!(costs, expected_costs);
        assert!(!costs.iter().any(|(guid, _)| *guid == poison_guid()));
        assert!(
            targets.iter().any(|(guid, _, _)| *guid == nested_raw_key()),
            "DurabilityLossAll retains the original Item-map key"
        );
        assert!(
            costs.iter().any(|(guid, _)| *guid == nested_guid()),
            "repair traversal returns the nested Item's embedded GUID"
        );
    }
}

#[test]
fn empty_and_stale_canonical_owners_do_not_fall_back_but_absent_owner_does() {
    let (mut fixture_session, _, _) = make_session();
    install_poisoned_fixture(&mut fixture_session);
    install_repair_catalog(&mut fixture_session);
    assert_eq!(
        query_targets(&fixture_session),
        vec![(poison_guid(), EQUIPMENT_SLOT_CHEST, true)]
    );
    assert_eq!(
        query_repair_costs(&fixture_session),
        Some(vec![(poison_guid(), 200)])
    );

    let (mut empty_session, _, _) = make_session();
    install_poisoned_fixture(&mut empty_session);
    install_repair_catalog(&mut empty_session);
    install_canonical_owner(&mut empty_session, true);
    assert!(query_targets(&empty_session).is_empty());
    assert_eq!(query_repair_costs(&empty_session), Some(Vec::new()));

    let (mut detached_session, _, _) = make_session();
    install_poisoned_fixture(&mut detached_session);
    install_repair_catalog(&mut detached_session);
    let (manager, stale_handle) = install_canonical_owner(&mut detached_session, false);
    seed_canonical_inventory(&mut detached_session);
    let expected = query_targets(&detached_session);
    assert!(
        !expected.is_empty(),
        "detached current generation remains readable"
    );
    assert!(
        !query_repair_costs(&detached_session)
            .expect("detached owner should remain readable")
            .is_empty()
    );
    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(stale_handle)
        .expect("captured generation should retire");
    let mut replacement = Player::new(Some(1), false);
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid());
    let current_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("same-GUID replacement should have a new generation");
    assert_ne!(current_handle, stale_handle);
    assert!(query_targets(&detached_session).is_empty());
    assert!(query_repair_costs(&detached_session).is_none());
    detached_session.core.player_handle_like_cpp = Some(current_handle);
    assert!(query_targets(&detached_session).is_empty());
    assert_eq!(query_repair_costs(&detached_session), Some(Vec::new()));
}
