use super::*;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::session::{PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP, SessionPlayerController};
use wow_constants::{InventoryType, ItemBondingType, ItemClass, ItemFieldFlags, ServerOpcodes};
use wow_core::{ObjectGuid, Position, VoidStorageItemIdGeneratorLikeCpp, guid::HighGuid};
use wow_data::{ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore};
use wow_database::StatementDef;
use wow_entities::{INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_ITEM_START};

fn make_void_storage_session() -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<Mutex<wow_map::MapManager>>,
) {
    let (_packet_tx, packet_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(16);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        packet_rx,
        send_tx,
    );
    let player_guid = ObjectGuid::create_player(1, 42);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_loaded_player_flags_like_cpp(PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP);
    session.set_void_storage_item_id_generator_like_cpp(Arc::new(
        VoidStorageItemIdGeneratorLikeCpp::new(100),
    ));
    session.mark_represented_void_storage_loaded_like_cpp();
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::new(60_000, 10)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    (session, send_rx, canonical)
}

fn insert_vault_keeper(manager: &Arc<Mutex<wow_map::MapManager>>, guid: ObjectGuid, entry: u32) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(5.0, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(1, 35, NPCFlags1::VAULT_KEEPER.bits(), 0);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn represented_void_item(item_id: u64, entry: u32) -> RepresentedVoidStorageItemLikeCpp {
    RepresentedVoidStorageItemLikeCpp {
        item_id,
        item_entry: entry,
        creator_guid: ObjectGuid::create_player(1, 7),
        fixed_scaling_level: 80,
        random_properties_id: -13,
        random_properties_seed: 29,
        context: ItemContext::Timewalking as u8,
    }
}

fn install_void_test_item_template(session: &mut WorldSession, entry: u32) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Miscellaneous as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry,
        ItemSparseTemplateEntry {
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
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

fn install_void_test_bag_and_child_templates(
    session: &mut WorldSession,
    bag_entry: u32,
    child_entry: u32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: bag_entry,
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
        ItemRecord {
            id: child_entry,
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
    ])));
    let sparse = |inventory_type: InventoryType, container_slots| ItemSparseTemplateEntry {
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
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (bag_entry, sparse(InventoryType::Bag, 8)),
        (child_entry, sparse(InventoryType::NonEquip, 0)),
    ])));
}

#[test]
fn login_load_rejects_invalid_rows_and_identity_collisions() {
    let (mut session, _, _) = make_void_storage_session();
    session.clear_represented_void_storage_like_cpp();
    install_void_test_item_template(&mut session, 19019);
    let item = represented_void_item(77, 19019);
    assert!(session.load_represented_void_storage_row_like_cpp(3, item.clone()));
    assert!(
        !session.load_represented_void_storage_row_like_cpp(3, represented_void_item(78, 19019),)
    );
    assert!(!session.load_represented_void_storage_row_like_cpp(4, item.clone(),));
    assert!(
        !session.load_represented_void_storage_row_like_cpp(4, represented_void_item(0, 19019),)
    );
    assert!(
        !session
            .load_represented_void_storage_row_like_cpp(u8::MAX, represented_void_item(79, 19019),)
    );
    assert!(
        !session.load_represented_void_storage_row_like_cpp(4, represented_void_item(80, 99999),)
    );
    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(3),
        Some(item)
    );
    assert_eq!(session.represented_void_storage_free_slots_like_cpp(), 159);
}

#[test]
fn full_save_rewrites_all_160_void_slots_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let item = represented_void_item(77, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(item.clone()),
        Some(0)
    );

    let statements = session
        .character_void_storage_save_statements_like_cpp(42)
        .expect("coherently loaded void storage");
    assert_eq!(statements.len(), 160);
    assert_eq!(
        statements[0].sql(),
        CharStatements::REP_CHAR_VOID_STORAGE_ITEM.sql()
    );
    assert_eq!(
        statements[0].params(),
        &[
            wow_database::SqlParam::U64(77),
            wow_database::SqlParam::U64(42),
            wow_database::SqlParam::U32(19019),
            wow_database::SqlParam::U8(0),
            wow_database::SqlParam::U64(7),
            wow_database::SqlParam::U32(80),
            wow_database::SqlParam::I32(-13),
            wow_database::SqlParam::I32(29),
            wow_database::SqlParam::U8(ItemContext::Timewalking as u8),
        ]
    );
    assert!(statements[1..].iter().all(|statement| {
        statement.sql() == CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT.sql()
    }));
}

#[test]
fn withdrawal_insert_persists_required_enchantments_column() {
    let item = represented_void_item(77, 19019);
    let statement = WorldSession::build_void_storage_withdrawal_item_insert_statement_like_cpp(
        501, 42, &item, 83, 900,
    );
    assert_eq!(
        statement.sql(),
        CharStatements::INS_ITEM_INSTANCE_CLONE.sql()
    );
    assert!(statement.sql().contains("charges, enchantments, flags"));
    assert_eq!(
        statement.params(),
        &[
            wow_database::SqlParam::U64(501),
            wow_database::SqlParam::U32(19019),
            wow_database::SqlParam::U64(42),
            wow_database::SqlParam::U64(7),
            wow_database::SqlParam::U64(0),
            wow_database::SqlParam::U32(1),
            wow_database::SqlParam::U32(0),
            wow_database::SqlParam::String(String::new()),
            wow_database::SqlParam::String(String::new()),
            wow_database::SqlParam::U32(ItemFieldFlags::SOULBOUND.bits()),
            wow_database::SqlParam::U32(83),
            wow_database::SqlParam::U32(900),
            wow_database::SqlParam::I32(-13),
            wow_database::SqlParam::I32(29),
            wow_database::SqlParam::U8(ItemContext::Timewalking as u8),
        ]
    );
}

#[test]
fn empty_inventory_positions_use_active_backpack_slot_count_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();

    session.set_player_inventory_slot_count_like_cpp(INVENTORY_DEFAULT_SIZE);
    let default_positions = session.represented_empty_inventory_positions_like_cpp();
    assert_eq!(default_positions.len(), usize::from(INVENTORY_DEFAULT_SIZE));
    assert_eq!(
        default_positions.last(),
        Some(&(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15))
    );

    session.set_player_inventory_slot_count_like_cpp(24);
    let expanded_positions = session.represented_empty_inventory_positions_like_cpp();
    assert_eq!(expanded_positions.len(), 24);
    assert_eq!(
        expanded_positions.last(),
        Some(&(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 23))
    );
}

#[tokio::test]
async fn nonempty_bag_deposit_plan_destroys_children_before_parent_atomically() {
    let (mut session, _, _) = make_void_storage_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_entry = 21841;
    let child_entry = 19019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, child_entry);

    let bag_guid = ObjectGuid::create_item(1, 501);
    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let bag_inventory = InventoryItem {
        guid: bag_guid,
        entry_id: bag_entry,
        db_guid: 501,
        inventory_type: Some(InventoryType::Bag as u8),
    };
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        bag_slot,
    );
    session.insert_inventory_item_object(bag_item);
    session.insert_inventory_item_like_cpp(bag_slot, bag_inventory.clone());

    let child_guid = ObjectGuid::create_item(1, 502);
    let mut child_item = session.make_inventory_item_object(
        child_guid,
        child_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        5,
    );
    child_item.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(child_item);

    let destroyed = session.plan_void_storage_destroyed_items_like_cpp(
        INVENTORY_SLOT_BAG_0,
        bag_slot,
        bag_inventory,
        Vec::new(),
    );
    assert_eq!(
        destroyed
            .iter()
            .map(|item| item.inventory_item.guid)
            .collect::<Vec<_>>(),
        vec![child_guid, bag_guid]
    );

    let lazy_pool = sqlx::mysql::MySqlPoolOptions::new()
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy CharacterDB pool");
    let char_db = wow_database::CharacterDatabase::from_pool(lazy_pool);
    let statements = destroyed
        .iter()
        .flat_map(|item| {
            WorldSession::void_storage_destroy_item_statements_like_cpp(
                &char_db,
                42,
                item.inventory_item.db_guid,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(statements.len(), 18);
    assert_eq!(
        statements
            .iter()
            .filter(|statement| {
                statement.sql() == CharStatements::DEL_CHAR_INVENTORY_ITEM.sql()
            })
            .map(|statement| statement.params().to_vec())
            .collect::<Vec<_>>(),
        vec![
            vec![
                wow_database::SqlParam::U64(42),
                wow_database::SqlParam::U64(502),
            ],
            vec![
                wow_database::SqlParam::U64(42),
                wow_database::SqlParam::U64(501),
            ],
        ]
    );

    assert_eq!(
        session.apply_committed_void_storage_destroyed_items_like_cpp(&destroyed),
        vec![child_guid, bag_guid]
    );
    assert!(session.get_inventory_item_by_pos(bag_slot, 5).is_none());
    assert!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, bag_slot)
            .is_none()
    );
    assert!(
        !session
            .inventory_item_objects_like_cpp()
            .contains_key(&child_guid)
    );
    assert!(
        !session
            .inventory_item_objects_like_cpp()
            .contains_key(&bag_guid)
    );
}

#[tokio::test]
async fn swap_definite_rollback_keeps_void_slots_unchanged() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    let item = represented_void_item(77, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(item.clone()),
        Some(0)
    );

    let failing_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy CharacterDB pool");
    session.set_char_db(Arc::new(wow_database::CharacterDatabase::from_pool(
        failing_pool,
    )));

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_packed_guid(&ObjectGuid::create_item(1, 77));
    packet.write_uint32(4);
    session.handle_void_storage_swap_item(packet).await;

    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(0),
        Some(item)
    );
    assert!(
        session
            .represented_void_storage_item_at_like_cpp(4)
            .is_none()
    );
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "definite rollback must reopen payout/save admission"
    );
}

#[tokio::test]
async fn deposit_definite_rollback_keeps_money_inventory_and_void_state_unchanged() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    install_void_test_item_template(&mut session, 19019);
    session.set_player_gold_like_cpp(500_000);
    let item_guid = ObjectGuid::create_item(1, 501);
    let item = session.make_inventory_item_object(
        item_guid,
        19019,
        ObjectGuid::create_player(1, 42),
        1,
        0,
        ItemContext::None,
        35,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: item_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );

    let failing_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy CharacterDB pool");
    session.set_char_db(Arc::new(wow_database::CharacterDatabase::from_pool(
        failing_pool,
    )));

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_uint32(1);
    packet.write_uint32(0);
    packet.write_packed_guid(&item_guid);
    session.handle_void_storage_transfer(packet).await;

    assert_eq!(session.player_gold_like_cpp(), 500_000);
    let (bag, slot, inventory_item) = session
        .get_inventory_item_by_guid_like_cpp(item_guid)
        .expect("rolled-back deposit must remain in inventory");
    assert_eq!((bag, slot), (INVENTORY_SLOT_BAG_0, 35));
    assert_eq!(inventory_item.guid, item_guid);
    assert_eq!(inventory_item.entry_id, 19019);
    assert_eq!(inventory_item.db_guid, 501);
    assert_eq!(session.represented_void_storage_free_slots_like_cpp(), 160);
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .begin_like_cpp()
            .is_ok(),
        "definite rollback must reopen payout/save admission"
    );
}

#[tokio::test]
async fn mixed_transfer_validation_failure_publishes_no_partial_deposit() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    install_void_test_item_template(&mut session, 19019);
    session.set_player_gold_like_cpp(500_000);

    let deposit_guid = ObjectGuid::create_item(1, 501);
    let deposit_item = session.make_inventory_item_object(
        deposit_guid,
        19019,
        ObjectGuid::create_player(1, 42),
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    session.insert_inventory_item_object(deposit_item);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: deposit_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );

    let unstoreable_void_item = represented_void_item(77, 99999);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(unstoreable_void_item.clone()),
        Some(0)
    );
    let lazy_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(100))
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy CharacterDB pool");
    session.set_char_db(Arc::new(wow_database::CharacterDatabase::from_pool(
        lazy_pool,
    )));

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_uint32(1);
    packet.write_uint32(1);
    packet.write_packed_guid(&deposit_guid);
    packet.write_packed_guid(&ObjectGuid::create_item(1, 77));
    session.handle_void_storage_transfer(packet).await;

    assert_eq!(session.player_gold_like_cpp(), 500_000);
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(deposit_guid)
            .is_some(),
        "the deposit remains visible when a later withdrawal cannot be planned"
    );
    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(0),
        Some(unstoreable_void_item)
    );
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
}
