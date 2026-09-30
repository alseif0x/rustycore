//! Void-storage regressions.
//!
//! Separated from the void_storage_tests.rs root under #683.

use super::*;

use std::sync::{Arc, Mutex};

use crate::session::{PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP, SessionPlayerController, SessionState};
use wow_constants::{
    Gender, InventoryType, ItemBondingType, ItemClass, ItemFieldFlags, ItemFlags, ItemQuality,
    ItemSubClassWeapon, ServerOpcodes,
};
use wow_core::{ObjectGuid, Position, VoidStorageItemIdGeneratorLikeCpp, guid::HighGuid};
use wow_data::{
    ItemModifiedAppearanceEntry, ItemModifiedAppearanceStore, ItemRandomPropertiesEntry,
    ItemRandomPropertiesStore, ItemRandomPropertyTemplateEntry, ItemRandomSuffixEntry,
    ItemRandomSuffixStore, ItemRecord, ItemSearchNameEntry, ItemSearchNameStore,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore,
};
use wow_entities::{INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START};
use wow_packet::ServerPacket;
use wow_packet::packets::loot::{CreatureLoot, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags};

#[derive(Debug)]
struct RecordingVoidStoragePersistencePortLikeCpp {
    outcome: wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp,
    unlocks: Mutex<Vec<wow_persistence::VoidStorageUnlockWriteRequestLikeCpp>>,
    swaps: Mutex<Vec<wow_persistence::VoidStorageSwapWriteRequestLikeCpp>>,
    transfers: Mutex<Vec<wow_persistence::VoidStorageTransferWriteRequestLikeCpp>>,
}

impl RecordingVoidStoragePersistencePortLikeCpp {
    fn new(outcome: wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp) -> Self {
        Self {
            outcome,
            unlocks: Mutex::new(Vec::new()),
            swaps: Mutex::new(Vec::new()),
            transfers: Mutex::new(Vec::new()),
        }
    }
}

impl wow_persistence::VoidStoragePersistencePortLikeCpp
    for RecordingVoidStoragePersistencePortLikeCpp
{
    fn persist_void_storage_unlock_like_cpp<'a>(
        &'a self,
        request: wow_persistence::VoidStorageUnlockWriteRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp,
    > {
        self.unlocks.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_void_storage_swap_like_cpp<'a>(
        &'a self,
        request: wow_persistence::VoidStorageSwapWriteRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp,
    > {
        self.swaps.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_void_storage_transfer_like_cpp<'a>(
        &'a self,
        request: wow_persistence::VoidStorageTransferWriteRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        'a,
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp,
    > {
        self.transfers.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

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
    session.set_player_faction_template_like_cpp(35);
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
    creature.unit_mut().set_faction(35);
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
    install_void_test_item_template_with_stack(session, entry, 1);
}

fn install_void_test_item_template_with_stack(
    session: &mut WorldSession,
    entry: u32,
    max_stack_size: i32,
) {
    install_void_test_item_template_with_stack_and_flags(
        session,
        entry,
        max_stack_size,
        ItemFlags::empty(),
    );
}

fn install_void_test_item_template_with_stack_and_flags(
    session: &mut WorldSession,
    entry: u32,
    max_stack_size: i32,
    flags: ItemFlags,
) {
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
            flags: [flags.bits() as u32, 0, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: max_stack_size,
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

async fn run_one_void_deposit_with_outcome_like_cpp(
    outcome: wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<RecordingVoidStoragePersistencePortLikeCpp>,
    ObjectGuid,
) {
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
    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(outcome));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_uint32(1);
    packet.write_uint32(0);
    packet.write_packed_guid(&item_guid);
    session.handle_void_storage_transfer(packet).await;
    (session, send_rx, port, item_guid)
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

mod scenarios_1;
mod scenarios_2;
