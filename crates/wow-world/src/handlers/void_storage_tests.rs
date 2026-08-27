// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, CharacterDatabase, SqlParam};

use super::*;

use std::sync::{Arc, Mutex};
use std::time::Duration;

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
use wow_database::StatementDef;
use wow_entities::{INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START};
use wow_packet::ServerPacket;
use wow_packet::packets::loot::{CreatureLoot, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags};

#[derive(Debug)]
struct RecordingVoidStoragePersistencePortLikeCpp {
    outcome: wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp,
    unlocks: Mutex<Vec<wow_persistence::VoidStorageUnlockWriteRequestLikeCpp>>,
    swaps: Mutex<Vec<wow_persistence::VoidStorageSwapWriteRequestLikeCpp>>,
}

impl RecordingVoidStoragePersistencePortLikeCpp {
    fn new(outcome: wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp) -> Self {
        Self {
            outcome,
            unlocks: Mutex::new(Vec::new()),
            swaps: Mutex::new(Vec::new()),
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

#[test]
fn void_item_packet_uses_cpp_void_instance_fields_only() {
    let (session, _, _) = make_void_storage_session();
    let item = represented_void_item(77, 19019);
    let packet = session.represented_void_storage_item_packet_like_cpp(3, &item);

    assert_eq!(packet.item.item_id, 19019);
    assert_eq!(packet.item.random_properties_id, 0);
    assert_eq!(packet.item.random_properties_seed, 0);
    assert!(packet.item.item_bonus.is_none());
    assert_eq!(
        packet.item.modifications.values,
        vec![wow_packet::packets::item::ItemMod::new(
            80,
            ItemModifier::TimewalkerLevel as u8,
        )]
    );
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
fn login_load_adds_default_void_item_appearance_like_cpp() {
    let (mut session, _, canonical) = make_void_storage_session();
    let entry = 19019;
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    let mut canonical_player = wow_entities::Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical_player.set_race_class_gender(1, 1, Gender::Male);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
        )
        .unwrap();
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Sword as u8,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: entry,
            allowable_race: 0,
            display: String::new(),
            overall_quality_id: ItemQuality::Uncommon as u8,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(
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
                    allowable_class: 0,
                    required_expansion: 0,
                    bonding: ItemBondingType::None as u8,
                    container_slots: 0,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
            [(
                entry,
                ItemRandomPropertyTemplateEntry {
                    item_level: 1,
                    quality: ItemQuality::Uncommon as i8,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
        ),
    ));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: entry as i32,
            item_appearance_modifier_id: 0,
            item_appearance_id: 1000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    assert!(session.can_add_item_appearance_represented_like_cpp(65));

    session.clear_represented_void_storage_like_cpp();
    assert!(
        session.load_represented_void_storage_row_like_cpp(0, represented_void_item(77, entry),)
    );
    assert!(session.represented_item_appearances_like_cpp.contains(&65));
}

#[test]
fn locked_login_discards_residual_void_rows_and_initializes_empty_storage_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    install_void_test_item_template(&mut session, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(represented_void_item(77, 19019)),
        Some(0)
    );

    session.set_loaded_player_flags_like_cpp(0);
    assert!(!session.prepare_represented_void_storage_login_load_like_cpp());
    assert!(session.represented_void_storage_loaded_like_cpp());
    assert_eq!(session.represented_void_storage_free_slots_like_cpp(), 160);

    session.apply_committed_void_storage_unlock_like_cpp();
    assert!(session.void_storage_is_unlocked_like_cpp());
    assert!(session.represented_void_storage_loaded_like_cpp());
    assert_eq!(session.represented_void_storage_free_slots_like_cpp(), 160);
}

#[tokio::test]
async fn unlock_submits_one_semantic_write_before_runtime_publication_like_cpp() {
    let (mut session, _, canonical) = make_void_storage_session();
    session.set_loaded_player_flags_like_cpp(0);
    session.set_player_gold_like_cpp(2_000_000);
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed,
    ));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    session.handle_void_storage_unlock(packet).await;

    assert!(session.void_storage_is_unlocked_like_cpp());
    assert_eq!(port.unlocks.lock().unwrap().len(), 1);
    let request = port.unlocks.lock().unwrap()[0].clone();
    assert_eq!(request.player_guid, 42);
    assert_eq!(request.money_before, 2_000_000);
    assert_eq!(request.money_after, 1_000_000);
    assert_ne!(
        request.player_flags_after & PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP,
        0
    );
}

#[test]
fn unlock_and_swap_paths_have_no_concrete_persistence_after_port_cut() {
    let source = include_str!("void_storage.rs");
    for (start, end) in [
        (
            "pub async fn handle_void_storage_unlock",
            "pub async fn handle_void_storage_query",
        ),
        (
            "pub async fn handle_void_storage_swap_item",
            "#[path = \"void_storage_tests.rs\"]",
        ),
    ] {
        let body = source
            .split_once(start)
            .and_then(|(_, tail)| tail.split_once(end).map(|(body, _)| body))
            .expect("audited void-storage handler body");
        for forbidden in ["CharStatements", "SqlTransaction", ".prepare(", "char_db"] {
            assert!(
                !body.contains(forbidden),
                "{start} regained concrete persistence syntax: {forbidden}"
            );
        }
    }
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
fn new_void_withdrawal_create_carries_committed_item_state_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    install_void_test_item_template(&mut session, 19019);
    let owner = ObjectGuid::create_player(1, 42);
    let bag = ObjectGuid::create_item(1, 500);
    let item_guid = ObjectGuid::create_item(1, 501);
    let mut item = session.make_inventory_item_object(
        item_guid,
        19019,
        owner,
        2,
        37,
        ItemContext::Timewalking,
        4,
    );
    item.set_contained_in(bag);
    item.set_container_guid_and_slot(bag, 19);
    item.set_property_seed(29);
    item.set_random_properties_id(-13);
    item.set_creator(ObjectGuid::create_player(1, 7));
    item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    item.set_binding(true);
    item.set_enchantment(EnchantmentSlot::Property0, 201, 60_000, 2);

    let create_dynamic_flags = ItemFieldFlags::NEW_ITEM.bits();
    let create = void_withdrawal_item_create_data_like_cpp(&item, create_dynamic_flags, 0);
    assert_eq!(create.item_guid, item_guid);
    assert_eq!(create.entry_id, 19019);
    assert_eq!(create.owner_guid, owner);
    assert_eq!(create.contained_in, bag);
    assert_eq!(create.stack_count, 2);
    assert_eq!(create.dynamic_flags, ItemFieldFlags::NEW_ITEM.bits());
    assert_eq!(create.durability, 37);
    assert_eq!(create.random_properties_seed, 0);
    assert_eq!(create.random_properties_id, 0);
    assert_eq!(create.context, ItemContext::Timewalking as u8);
    assert_eq!(create.container_slots, 0);
    assert_eq!(
        create.enchantments[EnchantmentSlot::Property0 as usize].id,
        0
    );
    assert_eq!(
        create.enchantments[EnchantmentSlot::Property0 as usize].duration,
        0
    );
    assert_eq!(
        create.enchantments[EnchantmentSlot::Property0 as usize].charges,
        0
    );

    let post_store_update = WorldSession::void_withdrawal_post_store_item_values_update_like_cpp(
        &item,
        create_dynamic_flags,
    )
    .expect("post-store item update");
    let item_data = post_store_update
        .item_data
        .as_ref()
        .expect("post-store update owns ItemData");
    assert!(item_data.mask.is_set(wow_entities::ITEM_DATA_PARENT_BIT));
    assert!(item_data.mask.is_set(wow_entities::ITEM_DATA_CREATOR_BIT));
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS_BIT)
    );
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_PROPERTY_SEED_BIT)
    );
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_RANDOM_PROPERTIES_ID_BIT)
    );
    assert!(
        item_data
            .mask
            .is_set(wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT)
    );
    assert!(item_data.mask.is_set(
        wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT + EnchantmentSlot::Property0 as usize
    ));
    assert_eq!(item_data.values.creator, ObjectGuid::create_player(1, 7));
    assert_eq!(
        item_data.values.dynamic_flags,
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );
    assert_eq!(item_data.values.property_seed, 29);
    assert_eq!(item_data.values.random_properties_id, -13);
    assert_eq!(
        item_data.values.enchantments[EnchantmentSlot::Property0 as usize].id,
        201
    );

    let packet = crate::entity_update_bridge::item_values_update_to_update_object(
        item_guid,
        571,
        &post_store_update,
    )
    .expect("creator VALUES packet");
    let bytes = packet.to_bytes();
    let mut packed_creator = WorldPacket::new_empty();
    packed_creator.write_packed_guid(&ObjectGuid::create_player(1, 7));
    let packed_creator = packed_creator.into_data();
    assert!(
        bytes
            .windows(packed_creator.len())
            .any(|window| window == packed_creator),
        "the creator GUID must reach the serialized VALUES update"
    );
    assert!(bytes.windows(4).any(|window| {
        window
            == (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND)
                .bits()
                .to_le_bytes()
    }));
}

#[test]
fn withdrawn_bag_create_preserves_template_container_slots_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let bag_entry = 21841;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, 19019);
    let bag = session.make_inventory_item_object(
        ObjectGuid::create_item(1, 501),
        bag_entry,
        ObjectGuid::create_player(1, 42),
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    let container_slots = session
        .item_storage_template(bag_entry)
        .map_or(0, |template| u32::from(template.container_slots));

    let create = void_withdrawal_item_create_data_like_cpp(
        &bag,
        ItemFieldFlags::NEW_ITEM.bits(),
        container_slots,
    );

    assert_eq!(create.container_slots, 8);
    assert!(create.container_item_guids.iter().all(ObjectGuid::is_empty));
}

#[test]
fn committed_withdrawn_bag_registers_canonical_storage_before_child_like_cpp() {
    let (mut session, _, canonical) = make_void_storage_session();
    let bag_entry = 21841;
    let child_entry = 19019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, child_entry);
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut canonical_player = wow_entities::Player::new(Some(1), false);
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    canonical_player.set_race_class_gender(1, 1, Gender::Male);
    canonical_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    canonical_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
        )
        .unwrap();
    let bag_guid = ObjectGuid::create_item(1, 501);
    let child_guid = ObjectGuid::create_item(1, 502);
    let bag_object = session.make_inventory_item_object(
        bag_guid,
        bag_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    assert!(session.apply_committed_new_inventory_item_at_like_cpp(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: bag_entry,
            db_guid: 501,
            inventory_type: Some(InventoryType::Bag as u8),
        },
        bag_object,
    ));

    let child_object = session.make_inventory_item_object(
        child_guid,
        child_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    assert!(session.apply_committed_new_inventory_item_at_like_cpp(
        INVENTORY_SLOT_BAG_START,
        0,
        InventoryItem {
            guid: child_guid,
            entry_id: child_entry,
            db_guid: 502,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
        child_object,
    ));

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 0)
            })
            .flatten(),
        Some(child_guid)
    );
}

#[test]
fn mixed_transfer_publishes_deposit_destroy_before_withdrawal_create_like_cpp() {
    let (mut session, send_rx, _) = make_void_storage_session();
    install_void_test_item_template(&mut session, 19019);
    let owner = ObjectGuid::create_player(1, 42);
    let deposited_guid = ObjectGuid::create_item(1, 500);
    let withdrawn_guid = ObjectGuid::create_item(1, 501);
    let mut withdrawn_item = session.make_inventory_item_object(
        withdrawn_guid,
        19019,
        owner,
        1,
        37,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    withdrawn_item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    session.insert_inventory_item_object(withdrawn_item.clone());
    let mut post_store_item = withdrawn_item.clone();
    WorldSession::clear_item_publication_changes_like_cpp(&mut post_store_item);

    let create_dynamic_flags = ItemFieldFlags::NEW_ITEM.bits();
    let expected_destroy = UpdateObject::destroy_objects(vec![deposited_guid], 571).to_bytes();
    let expected_create = UpdateObject::create_stored_items(
        vec![void_withdrawal_item_create_data_like_cpp(
            &withdrawn_item,
            create_dynamic_flags,
            0,
        )],
        571,
    )
    .to_bytes();

    session.publish_void_storage_item_lifecycle_like_cpp(
        571,
        vec![(
            (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            vec![deposited_guid],
        )],
        vec![WorldSession::new_void_withdrawal_item_publication_like_cpp(
            &withdrawn_item,
            &post_store_item,
            create_dynamic_flags,
            0,
            571,
        )],
    );

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(packets.first(), Some(&expected_destroy));
    let create_index = packets
        .iter()
        .position(|packet| packet == &expected_create)
        .expect("withdrawal CREATE_OBJECT packet");
    assert!(
        create_index > 0,
        "C++ publishes every deposit destroy before withdrawal creates"
    );
}

#[test]
fn planned_stack_merge_publishes_store_then_post_store_values_like_cpp() {
    let (session, send_rx, _) = make_void_storage_session();
    let owner = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 501);
    let mut create_item = session.make_inventory_item_object(
        item_guid,
        19019,
        owner,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    create_item.set_item_flag(ItemFieldFlags::NEW_ITEM);
    let create_dynamic_flags = ItemFieldFlags::NEW_ITEM.bits();

    let mut first_post_store_item = create_item.clone();
    WorldSession::clear_item_publication_changes_like_cpp(&mut first_post_store_item);
    first_post_store_item.set_creator(ObjectGuid::create_player(1, 7));
    first_post_store_item.set_binding(true);

    let mut merged_post_store_item = first_post_store_item.clone();
    WorldSession::clear_item_publication_changes_like_cpp(&mut merged_post_store_item);
    merged_post_store_item.set_creator(ObjectGuid::create_player(1, 8));

    let expected_create = UpdateObject::create_stored_items(
        vec![void_withdrawal_item_create_data_like_cpp(
            &create_item,
            create_dynamic_flags,
            0,
        )],
        571,
    )
    .to_bytes();
    let expected_store_merge = UpdateObject::item_stack_count_update(item_guid, 571, 2).to_bytes();
    let expected_post_store_merge =
        crate::entity_update_bridge::item_values_update_to_update_object(
            item_guid,
            571,
            &merged_post_store_item.values_update(),
        )
        .expect("planned-stack post-store VALUES update")
        .to_bytes();

    let mut publications = vec![WorldSession::new_void_withdrawal_item_publication_like_cpp(
        &create_item,
        &first_post_store_item,
        create_dynamic_flags,
        0,
        571,
    )];
    publications.extend(
        WorldSession::merged_void_withdrawal_item_publications_like_cpp(
            item_guid,
            2,
            None,
            &merged_post_store_item,
            571,
        ),
    );
    session.publish_void_storage_item_lifecycle_like_cpp(571, Vec::new(), publications);

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let create_index = packets
        .iter()
        .position(|packet| packet == &expected_create)
        .expect("count-one CREATE_OBJECT packet");
    let store_merge_index = packets
        .iter()
        .position(|packet| packet == &expected_store_merge)
        .expect("count-two StoreItem VALUES packet");
    let post_store_merge_index = packets
        .iter()
        .position(|packet| packet == &expected_post_store_merge)
        .expect("post-StoreNewItem creator VALUES packet");
    assert!(
        create_index < store_merge_index && store_merge_index < post_store_merge_index,
        "C++ publishes CREATE, then the merge count, then post-store field changes"
    );
}

#[test]
fn nested_withdrawal_resolves_planned_bag_database_and_item_guids_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let runtime_bag_guid = ObjectGuid::create_item(1, 600);
    session.insert_inventory_item_like_cpp(
        bag_slot,
        InventoryItem {
            guid: runtime_bag_guid,
            entry_id: 21841,
            db_guid: 600,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let planned = std::collections::HashMap::from([(bag_slot, 700)]);
    let planned_bag_guid = ObjectGuid::create_item(1, 700);
    let planned_item_guids = std::collections::HashMap::from([(bag_slot, planned_bag_guid)]);

    assert_eq!(
        session.void_storage_withdrawal_container_db_guid_like_cpp(INVENTORY_SLOT_BAG_0, &planned,),
        Some(0)
    );
    assert_eq!(
        session.void_storage_withdrawal_container_db_guid_like_cpp(bag_slot, &planned,),
        Some(700),
        "the planned bag must beat the stale runtime bag being replaced in the transaction"
    );
    assert_eq!(
        session.void_storage_withdrawal_container_db_guid_like_cpp(
            wow_entities::INVENTORY_SLOT_BAG_START + 1,
            &planned,
        ),
        None
    );
    assert_eq!(
        session.void_storage_withdrawal_container_item_guid_like_cpp(
            INVENTORY_SLOT_BAG_0,
            &planned_item_guids,
        ),
        session.player_guid()
    );
    assert_eq!(
        session
            .void_storage_withdrawal_container_item_guid_like_cpp(bag_slot, &planned_item_guids,),
        Some(planned_bag_guid),
        "the planned bag object must beat the stale runtime bag in the same slot"
    );
    assert_ne!(planned_bag_guid, runtime_bag_guid);
}

#[test]
fn withdrawal_restores_and_persists_effective_random_property_enchantments_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    session.set_item_random_properties_store(Arc::new(ItemRandomPropertiesStore::from_entries([
        ItemRandomPropertiesEntry {
            id: 17,
            enchantments: [101, 102, 103, 104, 105],
        },
    ])));
    session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
        ItemRandomSuffixEntry {
            id: 13,
            enchantments: [201, 202, 203, 204, 205],
            allocation_pct: [0; 5],
        },
    ])));

    let positive = session.effective_void_storage_random_properties_like_cpp(17, 29);
    assert_eq!(positive.id, 17);
    assert_eq!(positive.seed, 0);
    assert_eq!(
        positive.enchantment_ids[EnchantmentSlot::Property2 as usize],
        101
    );
    assert_eq!(
        positive.enchantment_ids[EnchantmentSlot::Property3 as usize],
        102
    );
    assert_eq!(
        positive.enchantment_ids[EnchantmentSlot::Property4 as usize],
        103
    );

    let suffix = session.effective_void_storage_random_properties_like_cpp(-13, 29);
    assert_eq!(suffix.id, -13);
    assert_eq!(suffix.seed, 29);
    assert_eq!(
        suffix.enchantment_ids[EnchantmentSlot::Property0 as usize],
        201
    );
    assert_eq!(
        suffix.enchantment_ids[EnchantmentSlot::Property1 as usize],
        202
    );
    assert_eq!(
        suffix.enchantment_ids[EnchantmentSlot::Property2 as usize],
        203
    );
    assert_eq!(
        session.effective_void_storage_random_properties_like_cpp(-999, 29),
        EffectiveVoidStorageRandomPropertiesLikeCpp::default()
    );

    let enchantments =
        WorldSession::void_storage_enchantments_db_string_like_cpp(&suffix.enchantment_ids);
    let item = represented_void_item(77, 19019);
    let statement = WorldSession::build_void_storage_withdrawal_item_insert_statement_like_cpp(
        501,
        42,
        &item,
        1,
        83,
        900,
        suffix.id,
        suffix.seed,
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits(),
        &enchantments,
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
            wow_database::SqlParam::String(enchantments),
            wow_database::SqlParam::U32(
                (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
            ),
            wow_database::SqlParam::U32(83),
            wow_database::SqlParam::U32(900),
            wow_database::SqlParam::I32(-13),
            wow_database::SqlParam::I32(29),
            wow_database::SqlParam::U8(ItemContext::Timewalking as u8),
        ]
    );

    let mut runtime_item = wow_entities::Item::default();
    runtime_item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 999, 60_000, 2);
    WorldSession::apply_effective_void_storage_random_properties_like_cpp(
        &mut runtime_item,
        &suffix,
    );
    assert_eq!(runtime_item.data().random_properties_id, -13);
    assert_eq!(runtime_item.data().property_seed, 29);
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::Property0 as usize].id,
        201
    );
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::Property1 as usize].id,
        202
    );
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::Property2 as usize].id,
        203
    );
    assert_eq!(
        runtime_item.data().enchantments[EnchantmentSlot::EnhancementPermanent as usize].id,
        999,
        "SetItemRandomProperties must not clear unrelated destination enchantments"
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

#[test]
fn swap_destination_slot_truncates_to_uint8_before_range_check_like_cpp() {
    assert_eq!(
        WorldSession::void_storage_swap_destination_slot_like_cpp(256),
        0
    );
    assert_eq!(
        WorldSession::void_storage_swap_destination_slot_like_cpp(415),
        159
    );
    assert_eq!(
        WorldSession::void_storage_swap_destination_slot_like_cpp(416),
        160,
        "truncation occurs before the handler's 160-slot range check"
    );
}

#[tokio::test]
async fn withdrawal_store_plan_merges_before_empty_slots_with_atomic_overlays_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    install_void_test_item_template_with_stack(&mut session, 19019, 20);
    let player_guid = ObjectGuid::create_player(1, 42);
    let existing_guid = ObjectGuid::create_item(1, 501);
    let mut existing_item = session.make_inventory_item_object(
        existing_guid,
        19019,
        player_guid,
        19,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    existing_item.set_count(19);
    session.insert_inventory_item_object(existing_item);
    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: existing_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );

    let (result, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(19019, 1, &[], &[])
        .expect("represented inventory planner");
    assert_eq!(result, wow_constants::InventoryResult::Ok);
    assert_eq!(
        destinations,
        vec![wow_entities::ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START),
            1,
        )]
    );

    let overlays = [
        DirectInventoryStorageOverlayLikeCpp {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
            entry_id: 19019,
            count: 20,
        },
        DirectInventoryStorageOverlayLikeCpp {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START + 1,
            entry_id: 19019,
            count: 1,
        },
    ];
    let (result, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(19019, 1, &overlays, &[])
        .expect("represented inventory planner with detached reservations");
    assert_eq!(result, wow_constants::InventoryResult::Ok);
    assert_eq!(
        destinations,
        vec![wow_entities::ItemPosCount::new(
            (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START + 1),
            1,
        )]
    );

    let mut merged_item = session
        .inventory_item_objects_like_cpp()
        .get(&existing_guid)
        .cloned()
        .expect("existing merge target");
    merged_item.set_count(20);
    merged_item.set_creator(ObjectGuid::create_player(1, 7));
    merged_item.set_binding(true);
    let enchantments = WorldSession::void_storage_enchantments_db_string_like_cpp(&[0; 13]);
    let lazy_pool = sqlx::mysql::MySqlPoolOptions::new()
        .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
        .expect("syntactically valid lazy CharacterDB pool");
    let char_db = wow_database::CharacterDatabase::from_pool(lazy_pool);
    let statement = session.build_void_storage_merged_item_update_statement_like_cpp(
        &char_db,
        &InventoryItem {
            guid: existing_guid,
            entry_id: 19019,
            db_guid: 501,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
        &merged_item,
        &enchantments,
    );
    assert_eq!(statement.sql(), CharStatements::UPD_ITEM_INSTANCE.sql());
    assert_eq!(statement.params()[4], wow_database::SqlParam::U32(20));
    assert_eq!(
        statement.params()[8],
        wow_database::SqlParam::String(enchantments)
    );
    assert_eq!(statement.params()[19], wow_database::SqlParam::U64(501));

    session.update_inventory_item_object_like_cpp(existing_guid, |item| item.set_count(20));
    let (_, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(19019, 1, &[], &[])
        .expect("full stack must force the next empty slot");
    assert_eq!(
        destinations[0].pos,
        (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START + 1)
    );
    let (_, destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            19019,
            1,
            &[],
            &[(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
        )
        .expect("a stack planned for deposit must be absent from withdrawal planning");
    assert_eq!(
        destinations[0].pos,
        (u16::from(INVENTORY_SLOT_BAG_0) << 8) | u16::from(INVENTORY_SLOT_ITEM_START),
        "C++ destroys deposits before CanStoreNewItem scans withdrawal destinations"
    );
}

#[test]
fn withdrawal_planner_excludes_slots_from_a_deposited_equipped_bag_like_cpp() {
    let (mut session, _, _) = make_void_storage_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_entry = 21841;
    let item_entry = 19019;
    install_void_test_bag_and_child_templates(&mut session, bag_entry, item_entry);

    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 600 + i64::from(offset));
        let item = session.make_inventory_item_object(
            guid,
            item_entry,
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
                entry_id: item_entry,
                db_guid: guid.counter() as u64,
                inventory_type: Some(InventoryType::NonEquip as u8),
            },
        );
    }

    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let bag_guid = ObjectGuid::create_item(1, 700);
    let bag_inventory = InventoryItem {
        guid: bag_guid,
        entry_id: bag_entry,
        db_guid: 700,
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

    let child_guid = ObjectGuid::create_item(1, 701);
    let mut child_item = session.make_inventory_item_object(
        child_guid,
        item_entry,
        player_guid,
        1,
        0,
        ItemContext::None,
        5,
    );
    child_item.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(child_item);

    let (_, child_only_destinations, _) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            item_entry,
            1,
            &[],
            &[(bag_slot, 5)],
        )
        .expect("child-only snapshot still has the equipped bag");
    let [child_only_bag, _] = child_only_destinations[0].pos.to_be_bytes();
    assert_eq!(
        child_only_bag, bag_slot,
        "the adversarial fixture must expose the orphan-container risk"
    );

    let destroyed = session.plan_void_storage_destroyed_items_like_cpp(
        INVENTORY_SLOT_BAG_0,
        bag_slot,
        bag_inventory,
        Vec::new(),
    );
    let vacated_positions = destroyed
        .iter()
        .map(|destroyed| (destroyed.bag, destroyed.slot))
        .collect::<Vec<_>>();
    assert_eq!(
        vacated_positions,
        vec![(bag_slot, 5), (INVENTORY_SLOT_BAG_0, bag_slot)]
    );

    let (result, destinations, no_space_count) = session
        .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
            item_entry,
            1,
            &[],
            &vacated_positions,
        )
        .expect("represented inventory planner");
    assert_ne!(result, wow_constants::InventoryResult::Ok);
    assert!(destinations.is_empty());
    assert_eq!(no_space_count, Some(1));
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

    let (destroyed_guids, changed_quest_ids) =
        session.apply_committed_void_storage_destroyed_items_like_cpp(&destroyed);
    assert_eq!(destroyed_guids, vec![child_guid, bag_guid]);
    assert!(changed_quest_ids.is_empty());
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

    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "write failed".to_string(),
        },
    ));
    session.set_void_storage_persistence_port_like_cpp(port.clone());

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
    assert_eq!(port.swaps.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn swap_unknown_commit_with_unchanged_money_quarantines_session() {
    let (mut session, _, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    let item = represented_void_item(77, 19019);
    assert_eq!(
        session.add_represented_void_storage_item_like_cpp(item.clone()),
        Some(0)
    );
    let port = Arc::new(RecordingVoidStoragePersistencePortLikeCpp::new(
        wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
            reason: "commit reply lost".to_string(),
            observed_money: Some(0),
        },
    ));
    session.set_void_storage_persistence_port_like_cpp(port);

    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&vault_keeper);
    packet.write_packed_guid(&ObjectGuid::create_item(1, 77));
    packet.write_uint32(4);
    session.handle_void_storage_swap_item(packet).await;

    assert_eq!(session.state(), SessionState::Disconnecting);
    assert!(
        session
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
    );
    assert_eq!(
        session.represented_void_storage_item_at_like_cpp(0),
        Some(item)
    );
    assert!(
        session
            .represented_void_storage_item_at_like_cpp(4)
            .is_none()
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
async fn deposit_definite_rollback_retains_active_item_loot_view_atomically() {
    let (mut session, send_rx, canonical) = make_void_storage_session();
    let vault_keeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1918, 43);
    insert_vault_keeper(&canonical, vault_keeper, 1918);
    install_void_test_item_template_with_stack_and_flags(
        &mut session,
        19019,
        1,
        ItemFlags::HAS_LOOT,
    );
    session.set_player_gold_like_cpp(500_000);
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 501);
    let item = session.make_inventory_item_object(
        item_guid,
        19019,
        player_guid,
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
    session.loot_table.insert(
        item_guid,
        CreatureLoot {
            loot_guid: item_guid,
            coins: 0,
            unlooted_count: 1,
            loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: vec![player_guid],
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 1,
                item_id: 19019,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: ItemContext::None as u8,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );
    session.set_active_loot_guid(item_guid);

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

    assert!(session.has_active_loot_views_like_cpp());
    assert!(session.loot_table.contains_key(&item_guid));
    assert!(
        session
            .get_inventory_item_by_guid_like_cpp(item_guid)
            .is_some(),
        "the release runs before planning, while definite DB rollback still preserves inventory"
    );
    assert_eq!(session.represented_void_storage_free_slots_like_cpp(), 160);
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::VoidTransferResult]
    );
}

#[test]
fn committed_void_deposit_retires_only_its_destroyed_item_loot_like_cpp() {
    let (mut session, send_rx, _) = make_void_storage_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let destroyed_item = ObjectGuid::create_item(1, 501);
    let unrelated_item = ObjectGuid::create_item(1, 502);
    for item_guid in [destroyed_item, unrelated_item] {
        session.loot_table.insert(
            item_guid,
            CreatureLoot {
                loot_guid: item_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );
        session.add_active_loot_view_owner_like_cpp(item_guid);
    }

    session.retire_committed_destroyed_item_loot_like_cpp(destroyed_item, player_guid);

    assert!(!session.active_loot_view_owners.contains(&destroyed_item));
    assert!(!session.loot_table.contains_key(&destroyed_item));
    assert!(session.active_loot_view_owners.contains(&unrelated_item));
    assert!(session.loot_table.contains_key(&unrelated_item));
    assert_eq!(
        send_rx
            .try_iter()
            .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::LootRelease]
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
