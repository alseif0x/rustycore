//! Behaviour tests for [`super`].
//!
//! Extracted from `quest.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

use super::*;
use crate::player_inventory_persistence_test_fixture::PlayerInventoryPersistencePortFixtureLikeCpp;
use crate::player_quest_persistence_test_fixture::{
    PlayerQuestLoadStageFixtureLikeCpp, PlayerQuestPersistencePortFixtureLikeCpp,
};
use crate::session::InventoryItem;
use crate::session::directory::PlayerRegistry;
use wow_constants::{
    ComparisonType, ConditionSourceType, ConditionType, InventoryType, ItemBondingType, ItemClass,
    ItemContext,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::quest::{
    QUEST_FLAGS_DAILY_LIKE_CPP, QUEST_FLAGS_WEEKLY_LIKE_CPP, QUEST_ITEM_DROP_COUNT,
    QUEST_REWARD_CHOICES_COUNT, QUEST_REWARD_CURRENCY_COUNT, QUEST_REWARD_DISPLAY_SPELL_COUNT,
    QUEST_REWARD_ITEM_COUNT, QUEST_REWARD_REPUTATIONS_COUNT, QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP, QuestObjective, QuestPoolMemberRowLikeCpp,
    QuestPoolSavedActiveRowLikeCpp, QuestPoolStoreLikeCpp, QuestStore, QuestTemplate,
};
use wow_data::{
    AdventureMapPoiEntry, AdventureMapPoiStore, Condition, ConditionEntriesByTypeStore,
    CurrencyTypesEntry, CurrencyTypesStore, ItemLimitCategoryEntry, ItemLimitCategoryStore,
    ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore,
    progression_rewards::{
        FactionEntry, FactionStore, QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        QuestFactionRewardEntry, QuestFactionRewardStore, QuestInfoEntry, QuestInfoStore,
        QuestPackageItemEntry, QuestPackageItemStore,
    },
    reputation::{ReputationRewardRateEntryLikeCpp, ReputationRewardRateStoreLikeCpp},
};
use wow_entities::{ITEM_LIMIT_CATEGORY_MODE_HAVE, Player, PlayerReputationRecord};
use wow_packet::packets::item::InventoryChangeFailure;
use wow_packet::packets::quest::QuestGiverQuestFailed;
use wow_packet::{ClientPacket, WorldPacket};
use wow_persistence::{
    ItemTemplateAddonCatalogPersistencePortLikeCpp, ItemTemplateAddonCatalogRequestLikeCpp,
    ItemTemplateAddonLootMetadataOutcomeLikeCpp, ItemTemplateAddonMoneyOutcomeLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerQuestActivePersistenceRowLikeCpp,
    PlayerQuestDailyPersistenceRowLikeCpp, PlayerQuestIdPersistenceRowLikeCpp,
    PlayerQuestObjectivePersistenceRowLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
    QuestPoiBlobLoadRowLikeCpp, QuestPoiLoadOutcomeLikeCpp, QuestPoiLoadStageLikeCpp,
    QuestPoiPersistencePortLikeCpp, QuestPoiPointLoadRowLikeCpp,
};
use wow_social::group::{GroupInfo, GroupRegistry, PendingInvites};

/// The quest opcode registrations.
///
/// #359 retired the dispatcher's match arms: an opcode is declared once, in
/// its `PacketHandlerEntry`, which now carries the call as well as the
/// admission metadata. These tests used to assert the arm and the registration
/// separately; there is one side left to assert.
const QUEST_HANDLER_REGISTRATIONS: &str = include_str!("quest/handlers.rs");

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    let mut session = WorldSession::new(
        1,
        "QuestStatusTest".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    );
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    // Reward tests model successful persistence. Production composition
    // installs the typed ports; these narrow unit fixtures retain the
    // explicit no-I/O success seam for unrelated reward assertions.
    session.set_loot_money_persistence_test_result_like_cpp(true);
    (session, send_rx)
}

fn quest_giver_cmsg_packet(guid: ObjectGuid, quest_id: u32, bit_byte: u8) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&guid);
    packet.write_uint32(quest_id);
    packet.write_uint8(bit_byte);
    packet.reset_read();
    packet
}

#[test]
fn quest_giver_query_quest_reads_respond_to_giver_as_bit_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7001, bit_byte);
        let (parsed_guid, quest_id, respond_to_giver) =
            read_quest_giver_query_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7001);
        assert_eq!(
            respond_to_giver, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}

#[test]
fn quest_giver_accept_quest_reads_start_cheat_as_bit_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 456);

    for (bit_byte, expected) in [(0x80, true), (0x00, false), (0x01, false)] {
        let mut packet = quest_giver_cmsg_packet(guid, 7002, bit_byte);
        let (parsed_guid, quest_id, start_cheat) =
            read_quest_giver_accept_quest_like_cpp(&mut packet).unwrap();

        assert_eq!(parsed_guid, guid);
        assert_eq!(quest_id, 7002);
        assert_eq!(
            start_cheat, expected,
            "C++ ReadBit reads the high bit; byte {bit_byte:#04x} must not be treated as bool"
        );
    }
}

#[test]
fn quest_giver_creature_id_is_zero_for_gameobject_sources_like_cpp() {
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(creature_guid(15513, 27)),
        15513
    );
    assert_eq!(
        quest_giver_creature_id_from_source_like_cpp(gameobject_guid(180516, 301)),
        0
    );
}

#[tokio::test]
async fn quest_giver_accept_emits_player_quest_log_update_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 7201;
    let gameobject_entry = 9301;
    let source_guid = gameobject_guid(gameobject_entry, 301);
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(gameobject_entry, quest_id));
    session.set_quest_store(Arc::new(store));
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, source_guid, gameobject_entry);
    attach_map_manager(&mut session, manager);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        source_guid,
        gameobject_entry,
        Position::new(10.0, 0.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8,
    );

    session
        .handle_quest_giver_accept_quest(quest_giver_cmsg_packet(source_guid, quest_id, 0x00))
        .await;

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("accepted quest should enter the represented quest log");
    assert_eq!(status.slot, 0);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);

    let update = send_rx
        .try_recv()
        .expect("C++ SetQuestSlot must become an immediate player UpdateObject");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    assert!(
        update
            .windows(std::mem::size_of::<u32>())
            .any(|window| window == quest_id.to_le_bytes()),
        "quest-log UpdateObject should carry the accepted QuestID"
    );

    let complete = send_rx
        .try_recv()
        .expect("legacy represented accept confirmation should still be sent");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_accept_rejected_source_sends_no_quest_log_update_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 7202;
    let gameobject_entry = 9302;
    let source_guid = gameobject_guid(gameobject_entry, 302);
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, source_guid, gameobject_entry);
    attach_map_manager(&mut session, manager);

    session
        .handle_quest_giver_accept_quest(quest_giver_cmsg_packet(source_guid, quest_id, 0x00))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(send_rx.try_recv().is_err());
}

fn quest_template(id: u32) -> QuestTemplate {
    QuestTemplate {
        id,
        quest_type: 2,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; QUEST_ITEM_DROP_COUNT],
        log_title: format!("Quest {id}"),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
    }
}

fn quest_info_entry_like_cpp(id: u32, quest_type: i8, modifiers: i32) -> QuestInfoEntry {
    QuestInfoEntry {
        id,
        info_name: String::new(),
        quest_type,
        modifiers,
        profession: 0,
    }
}

fn store_with_quests(ids: &[u32]) -> QuestStore {
    QuestStore::from_quests_like_cpp(ids.iter().copied().map(quest_template))
}

fn adventure_map_poi(id: u32, quest_id: u32, player_condition_id: u32) -> AdventureMapPoiEntry {
    AdventureMapPoiEntry {
        id,
        title: String::new(),
        description: String::new(),
        world_position: [0.0, 0.0],
        poi_type: 0,
        player_condition_id,
        quest_id,
        lfg_dungeon_id: 0,
        reward_item_id: 0,
        ui_texture_atlas_member_id: 0,
        ui_texture_kit_id: 0,
        map_id: 0,
        area_table_id: 0,
    }
}

fn adventure_map_start_quest_packet(quest_id: i32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(quest_id);
    pkt
}

#[tokio::test]
async fn adventure_map_start_quest_records_request_after_cpp_gates() {
    let (mut session, _send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7001])));
    session.set_adventure_map_poi_store(Arc::new(AdventureMapPoiStore::from_entries([
        adventure_map_poi(10, 7002, 0),
        adventure_map_poi(20, 7001, 0),
    ])));

    session
        .handle_adventure_map_start_quest(adventure_map_start_quest_packet(7001))
        .await;

    assert_eq!(
        session.represented_adventure_map_start_quest_requests_like_cpp(),
        &[RepresentedAdventureMapStartQuestLikeCpp {
            quest_id: 7001,
            adventure_map_poi_id: 20,
            player_condition_id: 0,
        }]
    );
}

#[tokio::test]
async fn adventure_map_start_quest_unknown_quest_returns_silently_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7001])));
    session.set_adventure_map_poi_store(Arc::new(AdventureMapPoiStore::from_entries([
        adventure_map_poi(20, 7002, 0),
    ])));

    session
        .handle_adventure_map_start_quest(adventure_map_start_quest_packet(7002))
        .await;

    assert!(
        session
            .represented_adventure_map_start_quest_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn adventure_map_start_quest_missing_player_condition_store_returns_silently_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7001])));
    session.set_adventure_map_poi_store(Arc::new(AdventureMapPoiStore::from_entries([
        adventure_map_poi(20, 7001, 42),
    ])));

    session
        .handle_adventure_map_start_quest(adventure_map_start_quest_packet(7001))
        .await;

    assert!(
        session
            .represented_adventure_map_start_quest_requests_like_cpp()
            .is_empty()
    );
}

fn quest_template_with_objective_count(id: u32, objective_count: usize) -> QuestTemplate {
    let mut quest = quest_template(id);
    quest.objectives = (0..objective_count)
        .map(|index| QuestObjective {
            id: id * 10 + index as u32,
            quest_id: id,
            obj_type: 0,
            order: index as u8,
            storage_index: index as i8,
            object_id: 1000 + index as i32,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    quest
}

fn store_with_sharable_quest_objectives(id: u32, objective_count: usize) -> QuestStore {
    let mut quest = quest_template_with_objective_count(id, objective_count);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    QuestStore::from_quests_like_cpp([quest])
}

fn store_with_sharable_timed_quest_objectives(
    id: u32,
    objective_count: usize,
    limit_time_secs: i64,
) -> QuestStore {
    let mut quest = quest_template_with_objective_count(id, objective_count);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.limit_time_secs = limit_time_secs;
    QuestStore::from_quests_like_cpp([quest])
}

fn quest_template_with_source_item(
    id: u32,
    source_item_id: u32,
    source_item_count: u32,
    source_spell_id: u32,
) -> QuestTemplate {
    let mut quest = quest_template(id);
    quest.source_item_id = source_item_id;
    quest.source_item_count = source_item_count;
    quest.source_spell_id = source_spell_id;
    quest
}

fn store_with_source_item_quest(
    quest_id: u32,
    source_item_id: u32,
    source_item_count: u32,
    source_spell_id: u32,
) -> QuestStore {
    QuestStore::from_quests_like_cpp([quest_template_with_source_item(
        quest_id,
        source_item_id,
        source_item_count,
        source_spell_id,
    )])
}

fn install_source_item_template(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
) {
    install_source_item_template_with_start_quest_limit_category_and_flags3(
        session, entry, stackable, max_count, 0, 0, 0,
    );
}

fn install_source_item_template_with_flags3(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
    flags3: u32,
) {
    install_source_item_template_with_start_quest_limit_category_and_flags3(
        session, entry, stackable, max_count, 0, 0, flags3,
    );
}

fn install_source_item_template_with_start_quest(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
    start_quest_id: i32,
) {
    install_source_item_template_with_start_quest_and_limit_category(
        session,
        entry,
        stackable,
        max_count,
        start_quest_id,
        0,
    );
}

fn install_source_item_template_with_limit_category(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
    limit_category: u16,
) {
    install_source_item_template_with_start_quest_and_limit_category(
        session,
        entry,
        stackable,
        max_count,
        0,
        limit_category,
    );
}

fn install_source_item_template_with_start_quest_and_limit_category(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
    start_quest_id: i32,
    limit_category: u16,
) {
    install_source_item_template_with_start_quest_limit_category_and_flags3(
        session,
        entry,
        stackable,
        max_count,
        start_quest_id,
        limit_category,
        0,
    );
}

fn install_source_item_template_with_start_quest_limit_category_and_flags3(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
    start_quest_id: i32,
    limit_category: u16,
    flags3: u32,
) {
    install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
        session,
        entry,
        stackable,
        max_count,
        start_quest_id,
        limit_category,
        flags3,
        ItemBondingType::None,
    );
}

fn install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
    start_quest_id: i32,
    limit_category: u16,
    flags3: u32,
    bonding: ItemBondingType,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Consumable as u8,
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
            flags: [0, 0, flags3, 0],
            bag_family: 0,
            start_quest_id,
            stackable,
            max_count: i32::try_from(max_count).unwrap_or(i32::MAX),
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
            limit_category,
            instance_bound: 0,
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: bonding as u8,
            container_slots: 0,
            inventory_type: InventoryType::NonEquip as i8,
        },
    )])));
}

fn insert_direct_inventory_item(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    entry: u32,
    count: u32,
    db_guid: u64,
) {
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    session.insert_inventory_item_like_cpp(
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: entry,
            db_guid,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry,
        player_guid,
        count,
        0,
        ItemContext::None,
        slot,
    );
    session.insert_inventory_item_object(item);
}

fn install_have_limit_category_like_cpp(
    session: &mut WorldSession,
    category_id: u32,
    quantity: u8,
) {
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: category_id,
            name: format!("Have Limit {category_id}"),
            quantity,
            flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
        },
    ])));
}

#[test]
fn query_quest_completion_builds_creature_then_masked_go_entries_like_cpp() {
    let mut store = store_with_quests(&[77]);
    store.ender_quests.entry(1234).or_default().push(77);
    store.ender_quests.entry(12).or_default().push(77);
    store
        .gameobject_ender_quests
        .entry(0x5678)
        .or_default()
        .push(77);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[77]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 77);
    assert_eq!(response[0].npcs, vec![12, 1234, 0x8000_5678u32 as i32]);
}

#[test]
fn query_quest_completion_skips_negative_missing_and_oversized_creature_entries_like_cpp() {
    let mut store = store_with_quests(&[5]);
    store
        .ender_quests
        .entry(i32::MAX as u32 + 1)
        .or_default()
        .push(5);
    store
        .gameobject_ender_quests
        .entry(u32::MAX)
        .or_default()
        .push(5);

    let response = represented_quest_completion_npc_response_like_cpp(&store, &[-1, 999, 5]);

    assert_eq!(response.len(), 1);
    assert_eq!(response[0].quest_id, 5);
    assert_eq!(response[0].npcs, vec![-1]);
}

#[tokio::test]
async fn quest_poi_query_filters_to_active_quest_slots_like_cpp() {
    let (mut session, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    add_active_quest(&mut session, 77);
    session.quest_poi_store_like_cpp = Some(Arc::new(HashMap::from([
        (
            77,
            wow_packet::packets::query::QuestPoiData {
                quest_id: 77,
                blobs: vec![wow_packet::packets::query::QuestPoiBlobData {
                    blob_index: 1,
                    objective_index: -1,
                    quest_objective_id: 2,
                    quest_object_id: 3,
                    map_id: 571,
                    ui_map_id: 486,
                    priority: 4,
                    flags: 5,
                    world_effect_id: 6,
                    player_condition_id: 7,
                    navigation_player_condition_id: 8,
                    spawn_tracking_id: 9,
                    points: vec![wow_packet::packets::query::QuestPoiBlobPoint {
                        x: 10,
                        y: 11,
                        z: 12,
                    }],
                    always_allow_merging_blobs: false,
                }],
            },
        ),
        (
            88,
            wow_packet::packets::query::QuestPoiData {
                quest_id: 88,
                blobs: Vec::new(),
            },
        ),
    ])));

    let mut missing_quest_pois =
        [0; wow_packet::packets::query::QUEST_POI_QUERY_MISSING_QUEST_POIS_LIKE_CPP];
    missing_quest_pois[0] = 77;
    missing_quest_pois[1] = 88;
    missing_quest_pois[2] = 77;

    session
        .handle_quest_poi_query(wow_packet::packets::query::QuestPoiQuery {
            missing_quest_count: 3,
            missing_quest_pois,
        })
        .await;

    let bytes = realm_rx.try_recv().expect("quest POI response");
    assert!(send_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::QuestPoiQueryResponse as u16
    );
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 77);
    assert_eq!(packet.read_int32().unwrap(), 1);
}

struct QuestPoiPortFixtureLikeCpp(QuestPoiLoadOutcomeLikeCpp);

impl QuestPoiPersistencePortLikeCpp for QuestPoiPortFixtureLikeCpp {
    fn load_quest_poi_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestPoiLoadOutcomeLikeCpp> {
        let outcome = self.0.clone();
        Box::pin(async move { outcome })
    }
}

struct ItemTemplateAddonCatalogPortFixtureLikeCpp {
    requests: std::sync::Mutex<Vec<ItemTemplateAddonCatalogRequestLikeCpp>>,
    outcomes:
        std::sync::Mutex<std::collections::VecDeque<ItemTemplateAddonLootMetadataOutcomeLikeCpp>>,
}

impl ItemTemplateAddonCatalogPortFixtureLikeCpp {
    fn new(
        outcomes: impl IntoIterator<Item = ItemTemplateAddonLootMetadataOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }
}

impl ItemTemplateAddonCatalogPersistencePortLikeCpp for ItemTemplateAddonCatalogPortFixtureLikeCpp {
    fn load_item_template_addon_money_like_cpp<'a>(
        &'a self,
        _request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonMoneyOutcomeLikeCpp> {
        panic!("quest source-item lookup never requests item-addon money")
    }

    fn load_item_template_addon_loot_metadata_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonLootMetadataOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one item-addon metadata outcome per uncached request");
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn quest_source_item_addon_port_preserves_lookup_and_zero_cache_like_cpp() {
    let port = ItemTemplateAddonCatalogPortFixtureLikeCpp::new([
        ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(
            wow_persistence::ItemTemplateAddonLootMetadataRowLikeCpp {
                flags_cu: 0x55,
                quest_log_item_id: 9101,
            },
        ),
        ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed {
            reason: "world read failed".into(),
        },
    ]);
    let (mut session, _send_rx) = make_session();
    session.set_item_template_addon_catalog_persistence_port_like_cpp(port.clone());

    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1001)
            .await,
        9101
    );
    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1001)
            .await,
        9101
    );
    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1002)
            .await,
        0
    );
    assert_eq!(
        session
            .quest_source_item_quest_log_item_id_like_cpp(1002)
            .await,
        0
    );
    assert_eq!(
        *port.requests.lock().unwrap(),
        [
            ItemTemplateAddonCatalogRequestLikeCpp { item_entry: 1001 },
            ItemTemplateAddonCatalogRequestLikeCpp { item_entry: 1002 },
        ]
    );
}

fn quest_poi_blob_row_like_cpp(quest_id: i32, idx1: i32) -> QuestPoiBlobLoadRowLikeCpp {
    QuestPoiBlobLoadRowLikeCpp {
        quest_id,
        blob_index: 1,
        idx1,
        objective_index: -1,
        quest_objective_id: 2,
        quest_object_id: 3,
        map_id: 571,
        ui_map_id: 486,
        priority: 4,
        flags: 5,
        world_effect_id: 6,
        player_condition_id: 7,
        navigation_player_condition_id: 8,
        spawn_tracking_id: 9,
        always_allow_merging_blobs: false,
    }
}

#[test]
fn quest_poi_typed_rows_join_points_and_skip_unknown_groups_like_cpp() {
    let store = build_quest_poi_store_like_cpp(
        vec![QuestPoiPointLoadRowLikeCpp {
            quest_id: 77,
            idx1: 3,
            x: 10,
            y: 11,
            z: 12,
        }],
        vec![
            quest_poi_blob_row_like_cpp(77, 3),
            quest_poi_blob_row_like_cpp(88, 9),
        ],
    );

    assert_eq!(store.len(), 1);
    assert_eq!(store[&77].blobs[0].points[0].x, 10);
    assert!(!store.contains_key(&88));
}

#[tokio::test]
async fn quest_poi_cache_consumes_typed_port_rows_and_caches_the_result() {
    let (mut session, _) = make_session();
    session.set_quest_poi_persistence_port_like_cpp(Arc::new(QuestPoiPortFixtureLikeCpp(
        QuestPoiLoadOutcomeLikeCpp::Loaded {
            points: vec![QuestPoiPointLoadRowLikeCpp {
                quest_id: 77,
                idx1: 3,
                x: 10,
                y: 11,
                z: 12,
            }],
            blobs: vec![quest_poi_blob_row_like_cpp(77, 3)],
        },
    )));

    let first = session.quest_poi_store_like_cpp().await;
    let second = session.quest_poi_store_like_cpp().await;
    assert_eq!(first[&77].blobs.len(), 1);
    assert!(Arc::ptr_eq(&first, &second));
}

#[tokio::test]
async fn missing_or_failed_quest_poi_port_caches_the_existing_empty_result() {
    let (mut missing, _) = make_session();
    let missing_first = missing.quest_poi_store_like_cpp().await;
    let missing_second = missing.quest_poi_store_like_cpp().await;
    assert!(missing_first.is_empty());
    assert!(Arc::ptr_eq(&missing_first, &missing_second));

    let (mut failed, _) = make_session();
    failed.set_quest_poi_persistence_port_like_cpp(Arc::new(QuestPoiPortFixtureLikeCpp(
        QuestPoiLoadOutcomeLikeCpp::Failed {
            stage: QuestPoiLoadStageLikeCpp::Points,
            reason: "world DB unavailable".to_owned(),
        },
    )));
    let failed_store = failed.quest_poi_store_like_cpp().await;
    assert!(failed_store.is_empty());
}

fn creature_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, entry, counter)
}

fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, entry, counter)
}

fn insert_creature(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
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
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    creature.unit_mut().set_level(80);
    creature.set_ai_identity_runtime(1, 35, NPCFlags1::QUEST_GIVER.bits(), 0);
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn insert_gameobject(manager: &mut wow_map::MapManager, guid: ObjectGuid, entry: u32) {
    let mut gameobject = wow_entities::GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    gameobject.world_mut().object_mut().add_to_world();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
}

fn insert_player_with_reputation(
    manager: &mut wow_map::MapManager,
    guid: ObjectGuid,
    faction_id: u32,
    standing: i32,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));
    player
        .gameplay_state_mut()
        .reputations
        .push(PlayerReputationRecord {
            faction_id,
            standing,
            flags: 0,
        });
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn attach_map_manager(session: &mut WorldSession, manager: wow_map::MapManager) {
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(manager)));
}

async fn run_status_query(session: &mut WorldSession, guid: ObjectGuid) {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    session.handle_quest_giver_status_query(pkt).await;
}

fn add_active_quest(session: &mut WorldSession, quest_id: u32) {
    let slot = session.first_free_quest_slot_like_cpp().unwrap_or(0);
    add_active_quest_in_slot(session, quest_id, slot);
}

fn add_active_quest_in_slot(session: &mut WorldSession, quest_id: u32, slot: u8) {
    add_active_quest_in_slot_with_status(session, quest_id, slot, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
}

fn add_active_quest_in_slot_with_status(
    session: &mut WorldSession,
    quest_id: u32,
    slot: u8,
    status: u8,
) {
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.statuses.insert(
                quest_id,
                PlayerQuestStatus {
                    quest_id,
                    status,
                    explored: false,
                    accept_time_secs: 0,
                    end_time_secs: 0,
                    objective_counts: Vec::new(),
                    slot,
                },
            );
        })
        .expect("test Player quest owner");
}

fn add_rewarded_quest(session: &mut WorldSession, quest_id: u32) {
    session
        .mutate_player_quest_gameplay_like_cpp(|quests| {
            quests.rewarded_quest_ids.insert(quest_id);
        })
        .expect("test Player quest owner");
}

#[test]
fn aggregate_item_removal_plan_persists_all_objectives_in_one_status() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_400;
    let first_item_id = 19_900;
    let second_item_id = 19_901;
    let mut quest = quest_template(quest_id);
    quest.objectives = [first_item_id, second_item_id]
        .into_iter()
        .enumerate()
        .map(|(index, item_id)| QuestObjective {
            id: quest_id * 10 + index as u32,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: index as u8,
            storage_index: index as i8,
            object_id: item_id,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    session
        .player_quests
        .get_mut(&quest_id)
        .expect("active quest")
        .objective_counts = vec![1, 1];

    let removed_entries = [first_item_id as u32, second_item_id as u32];
    let planned = session.plan_item_transfer_quest_persistence_like_cpp(
        &removed_entries,
        &[(first_item_id as u32, 0), (second_item_id as u32, 0)],
        &[],
    );

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].quest_id, quest_id);
    assert_eq!(planned[0].status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![0, 0]);
}

#[test]
fn mixed_item_transfer_quest_plan_applies_withdrawal_after_deposit() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_403;
    let item_id = 19_903;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot_with_status(&mut session, quest_id, 0, QUEST_STATUS_COMPLETE_LIKE_CPP);
    session
        .player_quests
        .get_mut(&quest_id)
        .expect("active quest")
        .objective_counts = vec![1];

    let planned = session.plan_item_transfer_quest_persistence_like_cpp(
        &[item_id as u32],
        &[(item_id as u32, 0)],
        &[(item_id as u32, 0, 1)],
    );

    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![1]);
}

#[test]
fn quest_bound_withdrawal_plan_consumes_credit_without_physical_item() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 7_404;
    let item_id = 19_904;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 1,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot(&mut session, quest_id, 0);
    let mut plan = session.begin_item_transfer_quest_persistence_like_cpp(&[], &[]);

    assert!(
        session.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
            &mut plan,
            item_id as u32,
            0,
            1,
        )
    );
    let planned = session.finish_item_transfer_quest_persistence_like_cpp(plan);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(planned[0].objective_counts, vec![1]);
}

#[tokio::test]
async fn bank_withdrawal_credits_only_first_matching_bound_item_objective_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let item_id = 19_901;
    let bound_quest = |quest_id: u32| {
        let mut quest = quest_template(quest_id);
        quest.objectives = vec![QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: item_id,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        }];
        quest
    };
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        bound_quest(7_401),
        bound_quest(7_402),
    ])));
    add_active_quest_in_slot(&mut session, 7_401, 0);
    add_active_quest_in_slot(&mut session, 7_402, 1);

    let planned = session.plan_bank_item_quest_persistence_like_cpp(item_id as u32, 0, false, 1, 1);
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].objective_counts, vec![1]);
    let planned_quest_id = planned[0].quest_id;

    let changed = session
        .apply_quest_item_added_objective_progress_like_cpp(item_id as u32, 0, 1)
        .await;
    assert_eq!(changed, vec![planned_quest_id]);
    assert_eq!(
        session
            .player_quests
            .values()
            .flat_map(|status| status.objective_counts.iter())
            .copied()
            .sum::<i32>(),
        1,
        "C++ UpdateQuestObjectiveProgress breaks after the first credited quest-bound item objective"
    );

    let bytes = send_rx
        .try_recv()
        .expect("single quest-bound item objective should send ItemPushResult");
    let mut packet = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn bound_item_durable_plan_and_apply_use_the_same_quest_log_order_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let item_id = 19_950;
    let early_slot_quest_id = 7_452;
    let late_slot_quest_id = 7_451;
    let bound_quest = |quest_id: u32| {
        let mut quest = quest_template(quest_id);
        quest.objectives = vec![QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: item_id,
            amount: 2,
            flags: 0,
            flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
            progress_bar_weight: 0.0,
            description: String::new(),
        }];
        quest
    };
    let quest_store = Arc::new(QuestStore::from_quests_like_cpp([
        bound_quest(late_slot_quest_id),
        bound_quest(early_slot_quest_id),
    ]));
    session.set_quest_store(Arc::clone(&quest_store));
    // Insert the numerically smaller quest first, but give it the later
    // quest-log slot. HashMap bucket/insertion order must affect neither
    // the pre-SQL plan nor the post-COMMIT mutation.
    add_active_quest_in_slot(&mut session, late_slot_quest_id, 9);
    add_active_quest_in_slot(&mut session, early_slot_quest_id, 2);

    let planned = session
        .plan_quest_source_item_bound_objective_persistence_like_cpp(item_id as u32, 0, 1)
        .expect("one bound objective should be planned");
    assert_eq!(planned.statuses.len(), 1);
    assert_eq!(planned.statuses[0].quest_id, early_slot_quest_id);

    let applied = session
        .apply_quest_source_item_bound_objective_progress_for_object_like_cpp(
            quest_store.as_ref(),
            item_id,
            1,
        )
        .await;
    assert_eq!(applied, vec![(early_slot_quest_id, 1)]);
    assert_eq!(
        session.player_quests[&early_slot_quest_id].objective_counts,
        vec![1]
    );
    assert!(
        session.player_quests[&late_slot_quest_id]
            .objective_counts
            .is_empty()
    );
}

#[tokio::test]
async fn bank_withdrawal_item_objective_never_sends_generic_credit_like_cpp() {
    let (mut session, send_rx) = make_session();
    let item_id = 19_902;
    let quest_id = 7_403;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: item_id,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    }];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest(&mut session, quest_id);

    let changed = session
        .apply_quest_item_added_objective_progress_like_cpp(item_id as u32, 0, 1)
        .await;

    assert_eq!(changed, vec![quest_id]);
    assert_eq!(session.player_quests[&quest_id].objective_counts, vec![1]);
    assert!(
        send_rx.try_recv().is_err(),
        "C++ suppresses QuestUpdateAddCredit for ITEM objectives"
    );
}

#[test]
fn represented_quest_objective_completable_accepts_cpp_storing_value_previous_types() {
    let quest_id = 7100;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![
        QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: 44,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        },
        QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: 55,
            amount: 1,
            flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        },
    ];
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![1, 0],
        slot: 0,
    };

    assert!(WorldSession::represented_quest_objective_completable_like_cpp(&status, &quest, 1));
}

#[test]
fn represented_objective_negative_storage_index_does_not_alias_slot_zero_like_cpp() {
    let quest_id = 7101;
    let mut quest = quest_template(quest_id);
    let objective = QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: -1,
        object_id: 55,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    };
    quest.objectives = vec![objective.clone()];
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![1],
        slot: 0,
    };

    assert!(
        !WorldSession::represented_quest_objective_complete_like_cpp(&status, &quest, &objective)
    );
}

async fn run_close_quest(session: &mut WorldSession, quest_id: u32) {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(quest_id);
    session.handle_quest_giver_close_quest(pkt).await;
}

async fn run_remove_quest_slot(session: &mut WorldSession, slot: u8) {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(slot);
    session.handle_quest_log_remove_quest(pkt).await;
}

async fn run_request_world_quest_update(session: &mut WorldSession) {
    session
        .handle_request_world_quest_update(WorldPacket::new_empty())
        .await;
}

async fn run_quest_confirm_accept(session: &mut WorldSession, quest_id: i32) {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(quest_id);
    session.handle_quest_confirm_accept(pkt).await;
}

fn write_cpp_item_instance_like_cpp(
    pkt: &mut WorldPacket,
    item_id: i32,
    random_properties_seed: i32,
    random_properties_id: i32,
    item_mods: &[(i32, u8)],
    item_bonus_ids: Option<&[u32]>,
) {
    pkt.write_int32(item_id);
    pkt.write_int32(random_properties_seed);
    pkt.write_int32(random_properties_id);
    pkt.write_bit(item_bonus_ids.is_some());
    pkt.flush_bits();
    pkt.write_bits(item_mods.len() as u32, 6);
    pkt.flush_bits();
    for (value, modifier_type) in item_mods {
        pkt.write_int32(*value);
        pkt.write_uint8(*modifier_type);
    }
    if let Some(item_bonus_ids) = item_bonus_ids {
        pkt.write_uint8(0);
        pkt.write_uint32(item_bonus_ids.len() as u32);
        for bonus_id in item_bonus_ids {
            pkt.write_uint32(*bonus_id);
        }
    }
}

fn write_cpp_quest_choice_item_like_cpp(
    pkt: &mut WorldPacket,
    loot_item_type: u8,
    item_id: i32,
    quantity: i32,
) {
    pkt.reset_bits();
    pkt.write_bits(u32::from(loot_item_type), 2);
    write_cpp_item_instance_like_cpp(pkt, item_id, 0, 0, &[], None);
    pkt.write_int32(quantity);
}

fn quest_giver_choose_reward_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
    loot_item_type: u8,
    item_id: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    write_cpp_quest_choice_item_like_cpp(
        &mut pkt,
        loot_item_type,
        item_id as i32,
        if item_id == 0 { 0 } else { 1 },
    );
    pkt
}

fn quest_giver_request_reward_packet_like_cpp(
    source_guid: ObjectGuid,
    quest_id: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&source_guid);
    pkt.write_uint32(quest_id);
    pkt
}

fn currency_entry_like_cpp(id: u32) -> CurrencyTypesEntry {
    CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }
}

fn install_test_item_template_with_flags2_like_cpp(
    session: &mut WorldSession,
    entry: u32,
    flags2: u32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: entry,
        class_id: ItemClass::Consumable as u8,
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
            flags: [0, flags2, 0, 0],
            bag_family: 0,
            start_quest_id: 0,
            stackable: 1,
            max_count: 0,
            lock_id: 0,
            required_reputation_rank: 0,
            sell_price: 0,
            buy_price: 0,
            vendor_stack_count: 1,
            price_variance: 0.0,
            price_random_value: 0.0,
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

#[test]
fn quest_giver_choose_reward_choice_parser_reads_cpp_wire_item_choice() {
    let guid = ObjectGuid::create_player(1, 42);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.write_uint32(7001);
    write_cpp_quest_choice_item_like_cpp(
        &mut pkt,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        19019,
        3,
    );

    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint32().unwrap(), 7001);
    assert_eq!(
        WorldSession::read_quest_choice_item_like_cpp(&mut pkt).unwrap(),
        QuestChoiceItemLikeCpp {
            loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            item_id: 19019,
            quantity: 3,
        }
    );
    assert!(pkt.is_empty());
}

#[test]
fn quest_giver_choose_reward_choice_parser_skips_cpp_item_mods_and_bonus() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP), 2);
    write_cpp_item_instance_like_cpp(&mut pkt, 392, 11, 22, &[(7, 1), (8, 2)], Some(&[91, 92]));
    pkt.write_int32(5);

    let choice = WorldSession::read_quest_choice_item_like_cpp(&mut pkt).unwrap();

    assert_eq!(
        choice,
        QuestChoiceItemLikeCpp {
            loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            item_id: 392,
            quantity: 5,
        }
    );
    assert!(pkt.is_empty());
}

#[test]
fn quest_giver_choose_reward_choice_parser_rejects_truncated_cpp_wire() {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(u32::from(QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP), 2);
    write_cpp_item_instance_like_cpp(&mut pkt, 19019, 0, 0, &[], None);

    assert!(WorldSession::read_quest_choice_item_like_cpp(&mut pkt).is_err());
}

#[test]
fn quest_giver_choose_reward_choice_validation_matches_loaded_cpp_type() {
    let mut quest = quest_template(7002);
    quest.reward_choice_items[0] = (19019, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    quest.reward_choice_items[1] = (392, 5);
    quest.reward_choice_item_types[1] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;

    assert!(
        WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
            &quest,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 19019,
                quantity: 1,
            }
        )
    );
    assert!(
        WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
            &quest,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
                item_id: 392,
                quantity: 5,
            }
        )
    );
    assert!(
        !WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
            &quest,
            QuestChoiceItemLikeCpp {
                loot_item_type: QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                item_id: 392,
                quantity: 5,
            }
        )
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_rejects_missing_reward_item_template_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7003;
    let reward_item_id = 19019;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_choose_reward_accepts_existing_reward_currency_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7004;
    let currency_id = 392;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (currency_id, 5);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            currency_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(session.player_currency_quantity(currency_id), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 5,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(5),
            quantity_gain_source: Some(CurrencyGainSourceLikeCpp::QuestReward as i32),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            Some(wow_constants::ServerOpcodes::UpdateObject),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete),
            Some(wow_constants::ServerOpcodes::QuestUpdateComplete),
        ]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_fixed_currency_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7014;
    let currency_id = 393;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_currencies[0] = currency_id;
    quest.reward_currency_amounts[0] = 7;
    session.set_player_gold_like_cpp(5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(session.player_currency_quantity(currency_id), 7);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 7,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(7),
            quantity_gain_source: Some(CurrencyGainSourceLikeCpp::DailyQuestReward as i32),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            Some(wow_constants::ServerOpcodes::UpdateObject),
            Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete),
            Some(wow_constants::ServerOpcodes::QuestUpdateComplete),
        ]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_removes_timed_quest_before_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7020;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    session.set_player_gold_like_cpp(5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 100,
            end_time_secs: 700,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_timed_quest_removals_like_cpp(),
        &[quest_id]
    );
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
}

#[tokio::test]
async fn quest_giver_choose_reward_non_timed_quest_records_no_timed_removal_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7021;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 100,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_timed_quest_removals_like_cpp()
            .is_empty()
    );
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
}

#[tokio::test]
async fn quest_giver_choose_reward_emits_reward_skill_fields_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7022;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_skill_line_id = 333;
    quest.reward_skill_points = 5;
    session.set_player_gold_like_cpp(5);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_skill_updates_like_cpp(),
        &[(333, 5)]
    );
    let update = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    let complete = send_rx.try_recv().unwrap();
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
    );
    assert_eq!(&complete[18..22], &333u32.to_le_bytes());
    assert_eq!(&complete[22..26], &5u32.to_le_bytes());
    assert_eq!(session.player_gold_like_cpp(), 42);
}

#[tokio::test]
async fn quest_giver_choose_reward_records_title_and_talent_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7025;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_title_id = 77;
    quest.reward_skill_points = 3;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_titles_like_cpp(),
        &[RepresentedQuestRewardTitleLikeCpp {
            quest_id,
            title_id: 77,
            char_title_lookup_unrepresented: true,
            set_title_runtime_unrepresented: true,
        }]
    );
    assert_eq!(
        session.represented_quest_reward_talent_points_like_cpp(),
        &[RepresentedQuestRewardTalentPointsLikeCpp {
            quest_id,
            points: 3,
            init_talent_for_level_unrepresented: true,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_mail_sender_entry_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7026;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_mail_template_id = 55;
    quest.reward_mail_delay_secs = 900;
    quest.reward_mail_sender_entry = 1234;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_mails_like_cpp(),
        &[RepresentedQuestRewardMailLikeCpp {
            quest_id,
            mail_template_id: 55,
            delay_secs: 900,
            sender_entry: Some(1234),
            quest_giver_guid: None,
            mail_template_lookup_unrepresented: true,
            mail_draft_runtime_unrepresented: true,
            character_db_transaction_unrepresented: true,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_mail_quest_giver_sender_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7027;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_mail_template_id = 56;
    quest.reward_mail_delay_secs = 30;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_mails_like_cpp(),
        &[RepresentedQuestRewardMailLikeCpp {
            quest_id,
            mail_template_id: 56,
            delay_secs: 30,
            sender_entry: None,
            quest_giver_guid: Some(player_guid),
            mail_template_lookup_unrepresented: true,
            mail_draft_runtime_unrepresented: true,
            character_db_transaction_unrepresented: true,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_reputation_override_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7028;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_faction_ids[2] = 930;
    quest.reward_faction_values[2] = 7;
    quest.reward_faction_overrides[2] = 1200;
    quest.reward_faction_cap_in[2] = 5;
    quest.reward_faction_flags = 1 << 2;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 2,
            faction_id: 930,
            reward_faction_value: 7,
            reward_faction_override: 1200,
            reward_faction_cap_in: 5,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 12,
            reputation_after_reward_rate_like_cpp: 12,
            no_quest_bonus: true,
            no_spillover: true,
            source: RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
            faction_store_lookup_unrepresented: true,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: true,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: true,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_reputation_db2_lookup_gap_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7029;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_values[0] = -4;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: -4,
            reward_faction_override: 0,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 0,
            reputation_after_low_level_rate_like_cpp: 0,
            reputation_after_reward_rate_like_cpp: 0,
            no_quest_bonus: false,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: true,
            quest_faction_reward_store_lookup_unrepresented: true,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: true,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_resolves_reward_reputation_db2_value_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7030;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_values[0] = -4;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.set_quest_faction_reward_store(Arc::new(QuestFactionRewardStore::from_entries([
        QuestFactionRewardEntry {
            id: 2,
            difficulty: [0, 5, 10, 15, 250, 350, 500, 750, 1000, 1500],
        },
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: -4,
            reward_faction_override: 0,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 250,
            reputation_after_low_level_rate_like_cpp: 250,
            reputation_after_reward_rate_like_cpp: 250,
            no_quest_bonus: false,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .expect("quest reward faction state")
            .standing,
        250
    );
    let mut pkt = loop {
        let bytes = send_rx
            .try_recv()
            .expect("set faction standing packet from quest reputation");
        let pkt = wow_packet::WorldPacket::from_bytes(&bytes);
        if pkt.server_opcode() == Some(wow_constants::ServerOpcodes::SetFactionStanding) {
            break pkt;
        }
    };
    pkt.skip_opcode();
    assert_eq!(pkt.read_float().expect("achievement bonus"), 0.0);
    assert_eq!(pkt.read_uint32().expect("faction count"), 1);
    assert_eq!(pkt.read_int32().expect("reputation list id"), 5);
    assert_eq!(pkt.read_int32().expect("standing"), 250);
    assert!(!pkt.read_bit().expect("show visual"));
}

#[tokio::test]
async fn quest_giver_choose_reward_skips_missing_reward_reputation_faction_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7031;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 999_999;
    quest.reward_faction_overrides[0] = 1200;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_quest_reward_reputations_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_skips_reward_reputation_at_rank_cap_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7032;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    quest.reward_faction_cap_in[0] = 5;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    let mut manager = wow_map::MapManager::default();
    insert_player_with_reputation(&mut manager, player_guid, 76, 9000);
    attach_map_manager(&mut session, manager);
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_quest_reward_reputations_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_reputation_below_rank_cap_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7033;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    quest.reward_faction_cap_in[0] = 6;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    let mut manager = wow_map::MapManager::default();
    insert_player_with_reputation(&mut manager, player_guid, 76, 9000);
    attach_map_manager(&mut session, manager);
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: 0,
            reward_faction_override: 1200,
            reward_faction_cap_in: 6,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 12,
            reputation_after_reward_rate_like_cpp: 12,
            no_quest_bonus: true,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_applies_reputation_reward_rate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7034;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.set_reputation_reward_rate_store(Arc::new(
        ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                faction_id: 76,
                rates: ReputationRewardRateEntryLikeCpp {
                    quest_rate: 1.0,
                    quest_daily_rate: 1.5,
                    quest_weekly_rate: 1.0,
                    quest_monthly_rate: 1.0,
                    quest_repeatable_rate: 1.0,
                    creature_rate: 1.0,
                    spell_rate: 1.0,
                },
            }],
            session.faction_store().unwrap(),
        )
        .0,
    ));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: 0,
            reward_faction_override: 1200,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 12,
            reputation_after_reward_rate_like_cpp: 18,
            no_quest_bonus: true,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: false,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_applies_low_level_quest_reputation_rate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7036;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.quest_level = 20;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    session.set_player_level_like_cpp(80);
    session.set_reputation_rates_like_cpp(crate::ReputationRatesLikeCpp {
        low_level_quest: 0.5,
        ..crate::ReputationRatesLikeCpp::default()
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_reputations_like_cpp(),
        &[RepresentedQuestRewardReputationLikeCpp {
            quest_id,
            slot: 0,
            faction_id: 76,
            reward_faction_value: 0,
            reward_faction_override: 1200,
            reward_faction_cap_in: 0,
            base_reputation_before_gain: 12,
            reputation_after_low_level_rate_like_cpp: 6,
            reputation_after_reward_rate_like_cpp: 6,
            no_quest_bonus: true,
            no_spillover: false,
            source: RepresentedQuestRewardReputationSourceLikeCpp::Quest,
            faction_store_lookup_unrepresented: false,
            quest_faction_reward_store_lookup_unrepresented: false,
            reputation_reward_rate_lookup_unrepresented: true,
            gray_level_script_hook_unrepresented: true,
            reputation_rank_cap_check_unrepresented: false,
            calculate_reputation_gain_unrepresented: true,
            modify_reputation_runtime_unrepresented: false,
        }]
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_skips_zero_reputation_reward_rate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7035;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_faction_ids[0] = 76;
    quest.reward_faction_overrides[0] = 1200;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(76, 5),
    ])));
    session.set_reputation_reward_rate_store(Arc::new(
        ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                faction_id: 76,
                rates: ReputationRewardRateEntryLikeCpp {
                    quest_rate: 0.0,
                    quest_daily_rate: 1.0,
                    quest_weekly_rate: 1.0,
                    quest_monthly_rate: 1.0,
                    quest_repeatable_rate: 1.0,
                    creature_rate: 1.0,
                    spell_rate: 1.0,
                },
            }],
            session.faction_store().unwrap(),
        )
        .0,
    ));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .represented_quest_reward_reputations_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_records_reward_spell_cast_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7023;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_spell = 12_345;
    quest.reward_display_spell = [22_001, 22_002, 0];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_spell_casts_like_cpp(),
        &[RepresentedQuestRewardSpellCastLikeCpp {
            quest_id,
            spell_id: 12_345,
            kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
            can_delay_teleport_like_cpp: true,
            spell_info_lookup_unrepresented: true,
            caster_selection_unrepresented: true,
            cast_spell_runtime_unrepresented: true,
        }]
    );
    assert!(!session.represented_can_delay_teleport_like_cpp());
}

#[tokio::test]
async fn quest_giver_choose_reward_records_display_spells_only_without_reward_spell_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7024;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP;
    quest.reward_display_spell = [22_001, 0, 22_003];
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session.represented_quest_reward_spell_casts_like_cpp(),
        &[
            RepresentedQuestRewardSpellCastLikeCpp {
                quest_id,
                spell_id: 22_001,
                kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell { index: 0 },
                can_delay_teleport_like_cpp: true,
                spell_info_lookup_unrepresented: true,
                caster_selection_unrepresented: false,
                cast_spell_runtime_unrepresented: true,
            },
            RepresentedQuestRewardSpellCastLikeCpp {
                quest_id,
                spell_id: 22_003,
                kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell { index: 2 },
                can_delay_teleport_like_cpp: true,
                spell_info_lookup_unrepresented: true,
                caster_selection_unrepresented: false,
                cast_spell_runtime_unrepresented: true,
            },
        ]
    );
    assert!(!session.represented_can_delay_teleport_like_cpp());
}

#[tokio::test]
async fn quest_giver_choose_reward_sets_daily_lockout_status_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7015;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(session.daily_quests_completed_like_cpp.contains(&quest_id));
    assert!(!session.df_quests_like_cpp.contains(&quest_id));
    assert!(session.last_daily_quest_time_like_cpp > 0);
}

#[tokio::test]
async fn quest_giver_choose_reward_sets_df_lockout_in_daily_table_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7016;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.daily_quests_completed_like_cpp.contains(&quest_id));
    assert!(session.df_quests_like_cpp.contains(&quest_id));
    assert!(session.last_daily_quest_time_like_cpp > 0);
}

#[tokio::test]
async fn quest_giver_choose_reward_sets_weekly_and_monthly_lockouts_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let weekly_id = 7017;
    let monthly_id = 7018;
    let mut weekly = quest_template(weekly_id);
    weekly.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP | 0x0000_8000;
    let mut monthly = quest_template(monthly_id);
    monthly.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    monthly.special_flags = 0x0000_0010;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        weekly, monthly,
    ])));
    for quest_id in [weekly_id, monthly_id] {
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;
    }

    assert!(
        session
            .weekly_quests_completed_like_cpp
            .contains(&weekly_id)
    );
    assert!(
        !session
            .weekly_quests_completed_like_cpp
            .contains(&monthly_id)
    );
    assert!(
        session
            .monthly_quests_completed_like_cpp
            .contains(&monthly_id)
    );
    assert!(
        !session
            .monthly_quests_completed_like_cpp
            .contains(&weekly_id)
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_sets_seasonal_lockout_status_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7019;
    let event_id = 9;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.quest_sort_id = -376;
    quest.event_id_for_quest = event_id;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(
        session
            .seasonal_quests_like_cpp
            .get(&event_id)
            .is_some_and(|quests| quests.contains_key(&quest_id))
    );
    assert!(session.seasonal_quest_changed_like_cpp);
}

#[tokio::test]
async fn quest_giver_choose_reward_removes_item_objective_before_rewards_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7015;
    let required_item_id = 19_028;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: required_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_gold_like_cpp(5);
    install_source_item_template(&mut session, required_item_id, 20, 0);
    insert_direct_inventory_item(&mut session, player_guid, 23, required_item_id, 5, 9911);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == required_item_id)
        .expect("partial objective item stack should remain");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .map(|item| item.count()),
        Some(3)
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_removes_currency_objective_before_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7016;
    let currency_id = 394;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: currency_id as i32,
        amount: 4,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_gold_like_cpp(5);
    session.set_currency_types_store(Arc::new(CurrencyTypesStore::from_entries([
        currency_entry_like_cpp(currency_id),
    ])));
    assert!(
        session
            .add_currency_quest_reward_like_cpp(
                currency_id,
                10,
                CurrencyGainSourceLikeCpp::QuestReward,
            )
            .unwrap()
            .is_some()
    );
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    assert_eq!(session.player_currency_quantity(currency_id), 6);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::misc::SetCurrency {
            type_id: currency_id as i32,
            quantity: 6,
            flags: 0,
            weekly_quantity: None,
            tracked_quantity: None,
            max_quantity: None,
            total_earned: None,
            suppress_chat_log: false,
            quantity_change: Some(-4),
            quantity_gain_source: None,
            quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        }
        .to_bytes()
    );
}

#[tokio::test]
async fn quest_giver_choose_reward_accepts_quest_package_primary_everyone_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7005;
    let reward_item_id = 19_019;
    let package_id = 77;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_test_item_template_with_flags2_like_cpp(&mut session, reward_item_id, 0);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("primary package reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(1)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 1);
                assert_eq!(packet.read_int32().unwrap(), 1);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}

#[tokio::test]
async fn quest_giver_choose_reward_accepts_quest_package_fallback_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7006;
    let reward_item_id = 19_020;
    let package_id = 78;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_test_item_template_with_flags2_like_cpp(&mut session, reward_item_id, 0);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("fallback package reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(1)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 1);
                assert_eq!(packet.read_int32().unwrap(), 1);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}

#[tokio::test]
async fn quest_giver_choose_reward_rejects_quest_package_wrong_faction_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7007;
    let reward_item_id = 19_021;
    let package_id = 79;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_test_item_template_with_flags2_like_cpp(
        &mut session,
        reward_item_id,
        ItemFlags2::FactionHorde as u32,
    );
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_choose_reward_direct_choice_inventory_failure_sends_quest_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7008;
    let reward_item_id = 19_022;
    let limit_category = 44;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 1);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9907);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QuestGiverQuestFailed {
            quest_id,
            reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_choose_reward_fixed_reward_inventory_failure_sends_quest_failed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7009;
    let reward_item_id = 19_023;
    let limit_category = 45;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_items[0] = reward_item_id;
    quest.reward_amounts[0] = 1;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9908);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        QuestGiverQuestFailed {
            quest_id,
            reason: InventoryResult::ItemMaxLimitCategoryCountExceededIs as u32,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_choose_reward_fixed_reward_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7012;
    let reward_item_id = 19_026;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_items[0] = reward_item_id;
    quest.reward_amounts[0] = 2;
    session.set_player_gold_like_cpp(5);
    install_source_item_template(&mut session, reward_item_id, 20, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            0,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("fixed reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(2)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 2);
                assert_eq!(packet.read_int32().unwrap(), 2);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}

#[tokio::test]
async fn quest_reward_item_definite_and_unknown_commit_fail_closed_before_publication_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "fixture rollback".into(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "fixture unknown commit".into(),
        },
    ] {
        let (mut session, _send_rx) = make_session();
        let player_guid = session.player_guid().unwrap();
        let quest_id = 70_120;
        let reward_item_id = 19_126;
        let mut quest = quest_template(quest_id);
        quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
        quest.reward_money_difficulty = 37;
        quest.reward_items[0] = reward_item_id;
        quest.reward_amounts[0] = 2;
        session.set_player_gold_like_cpp(5);
        install_source_item_template(&mut session, reward_item_id, 20, 0);
        session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
        session.player_quests.insert(
            quest_id,
            PlayerQuestStatus {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        let (port, requests) =
            PlayerInventoryPersistencePortFixtureLikeCpp::with_outcomes_like_cpp([
                PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                outcome,
            ]);
        session.set_player_inventory_persistence_port_like_cpp(port);

        session
            .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
                player_guid,
                quest_id,
                QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
                0,
            ))
            .await;

        assert_eq!(
            session
                .player_quests
                .get(&quest_id)
                .map(|status| status.status),
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
        );
        assert!(!session.rewarded_quests.contains(&quest_id));
        assert_eq!(session.player_gold_like_cpp(), 5);
        assert!(
            session
                .inventory_items_like_cpp()
                .values()
                .all(|item| item.entry_id != reward_item_id)
        );
        let requests = requests.lock().unwrap();
        assert!(matches!(
            requests.as_slice(),
            [
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(_),
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestItemGrant(_),
            ]
        ));
    }
}

#[tokio::test]
async fn quest_giver_choose_reward_chosen_item_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7013;
    let reward_item_id = 19_027;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.reward_choice_items[0] = (reward_item_id, 3);
    quest.reward_choice_item_types[0] = QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
    session.set_player_gold_like_cpp(5);
    install_source_item_template(&mut session, reward_item_id, 20, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 42);
    let reward_item = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == reward_item_id)
        .expect("chosen reward item should be in direct inventory");
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&reward_item.guid)
            .map(|item| item.count()),
        Some(3)
    );

    let mut saw_item_push = false;
    let mut saw_quest_complete = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        match packet.read_uint16().unwrap() {
            opcode if opcode == wow_constants::ServerOpcodes::ItemPushResult as u16 => {
                saw_item_push = true;
                assert_eq!(packet.read_packed_guid().unwrap(), player_guid);
                assert_eq!(
                    packet.read_uint8().unwrap(),
                    u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                );
                let slot_in_bag = packet.read_int32().unwrap();
                assert!(slot_in_bag >= 0);
                assert_eq!(packet.read_int32().unwrap(), 0);
                assert_eq!(packet.read_int32().unwrap(), 3);
                assert_eq!(packet.read_int32().unwrap(), 3);
            }
            opcode if opcode == wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16 => {
                saw_quest_complete = true;
            }
            _ => {}
        }
    }
    assert!(saw_item_push);
    assert!(saw_quest_complete);
}

#[tokio::test]
async fn quest_giver_choose_reward_package_primary_inventory_failure_sends_equip_error_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7010;
    let reward_item_id = 19_024;
    let package_id = 80;
    let limit_category = 46;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9909);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
            .with_limit_category(limit_category)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_choose_reward_package_fallback_inventory_failure_sends_equip_error_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().unwrap();
    let quest_id = 7011;
    let reward_item_id = 19_025;
    let package_id = 81;
    let limit_category = 47;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    quest.reward_money_difficulty = 37;
    quest.quest_package_id = package_id;
    session.set_player_gold_like_cpp(5);
    install_source_item_template_with_limit_category(
        &mut session,
        reward_item_id,
        20,
        0,
        limit_category as u16,
    );
    install_have_limit_category_like_cpp(&mut session, limit_category, 1);
    insert_direct_inventory_item(&mut session, player_guid, 23, reward_item_id, 1, 9910);
    session.set_quest_package_item_store(Arc::new(QuestPackageItemStore::from_entries([
        QuestPackageItemEntry {
            id: 1,
            package_id: package_id as u16,
            item_id: reward_item_id as i32,
            item_quantity: 1,
            display_type: QUEST_PACKAGE_FILTER_UNMATCHED_LIKE_CPP,
        },
    ])));
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );

    session
        .handle_quest_giver_choose_reward(quest_giver_choose_reward_packet_like_cpp(
            player_guid,
            quest_id,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            reward_item_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .map(|status| status.status),
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP)
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(session.player_gold_like_cpp(), 5);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
            .with_limit_category(limit_category)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}

async fn run_quest_push_result(
    session: &mut WorldSession,
    sender_guid: ObjectGuid,
    quest_id: u32,
    result: u8,
) {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&sender_guid);
    pkt.write_uint32(quest_id);
    pkt.write_uint8(result);
    session.handle_quest_push_result(pkt).await;
}

async fn run_push_quest_to_party(session: &mut WorldSession, quest_id: u32) {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(quest_id);
    session.handle_push_quest_to_party(pkt).await;
}

fn store_with_sharable_quest(id: u32) -> QuestStore {
    let mut quest = quest_template(id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    QuestStore::from_quests_like_cpp([quest])
}

fn store_with_sharable_quest_levels(id: u32, min_level: i32, max_level: u8) -> QuestStore {
    let mut quest = quest_template(id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.min_level = min_level;
    quest.max_level = max_level;
    QuestStore::from_quests_like_cpp([quest])
}

fn store_with_sharable_quest_class_race(
    id: u32,
    allowable_classes: u32,
    allowable_races: u64,
) -> QuestStore {
    let mut quest = quest_template(id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.allowable_classes = allowable_classes;
    quest.allowable_races = allowable_races;
    QuestStore::from_quests_like_cpp([quest])
}

fn store_with_sharable_quest_reputation(
    id: u32,
    min_faction: u32,
    min_value: i32,
    max_faction: u32,
    max_value: i32,
) -> QuestStore {
    let mut quest = quest_template(id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.required_min_rep_faction = min_faction;
    quest.required_min_rep_value = min_value;
    quest.required_max_rep_faction = max_faction;
    quest.required_max_rep_value = max_value;
    QuestStore::from_quests_like_cpp([quest])
}

fn store_with_sharable_quest_previous(id: u32, prev_quest_id: i32) -> QuestStore {
    let mut quest = quest_template(id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.prev_quest_id = prev_quest_id;
    QuestStore::from_quests_like_cpp([quest])
}

fn store_with_daily_sharable_quests(ids: &[u32]) -> QuestStore {
    let quests = ids.iter().map(|id| {
        let mut quest = quest_template(*id);
        quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
        quest
    });
    QuestStore::from_quests_like_cpp(quests)
}

fn store_with_df_sharable_quest(id: u32) -> QuestStore {
    let mut quest = quest_template(id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.special_flags |= QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    QuestStore::from_quests_like_cpp([quest])
}

fn quest_pool_store_with_active_saved(
    quest_store: &QuestStore,
    pool_id: u32,
    member_quest_ids: &[u32],
    active_quest_ids: &[u32],
) -> QuestPoolStoreLikeCpp {
    QuestPoolStoreLikeCpp::from_rows_like_cpp(
        quest_store,
        member_quest_ids
            .iter()
            .enumerate()
            .map(|(pool_index, quest_id)| QuestPoolMemberRowLikeCpp {
                quest_id: *quest_id,
                pool_id,
                pool_index: pool_index as u32,
                num_active: Some(active_quest_ids.len() as u32),
            }),
        active_quest_ids
            .iter()
            .map(|quest_id| QuestPoolSavedActiveRowLikeCpp {
                pool_id,
                quest_id: *quest_id,
            }),
    )
}

fn recv_world_quest_update_count(send_rx: &flume::Receiver<Vec<u8>>) -> u32 {
    let bytes = send_rx
        .try_recv()
        .expect("world quest update response packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::WorldQuestUpdateResponse as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    pkt.read_uint32().unwrap()
}

fn recv_push_quest_result_response(send_rx: &flume::Receiver<Vec<u8>>) -> (ObjectGuid, u8, String) {
    let bytes = send_rx
        .try_recv()
        .expect("quest push result response packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QuestPushResult as u16
    );
    let result = read_push_quest_result_response_bytes(&bytes);
    assert!(send_rx.try_recv().is_err());
    result
}

fn recv_push_quest_result_response_after_death_sync(
    send_rx: &flume::Receiver<Vec<u8>>,
) -> (ObjectGuid, u8, String) {
    loop {
        let bytes = send_rx
            .try_recv()
            .expect("quest push result response packet after death sync");
        let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
        if opcode == wow_constants::ServerOpcodes::QuestPushResult as u16 {
            return read_push_quest_result_response_bytes(&bytes);
        }
        assert_eq!(
            opcode,
            wow_constants::ServerOpcodes::UpdateObject as u16,
            "only UpdateObject death-state sync may precede QuestPushResult"
        );
    }
}

fn read_push_quest_result_response_bytes(bytes: &[u8]) -> (ObjectGuid, u8, String) {
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let sender_guid = pkt.read_packed_guid().unwrap();
    let result = pkt.read_uint8().unwrap();
    let title_len = pkt.read_bits(9).unwrap() as usize;
    let quest_title = pkt.read_string(title_len).unwrap();
    assert_eq!(pkt.remaining(), 0);
    (sender_guid, result, quest_title)
}

fn recv_quest_giver_quest_details_contains_quest_id(
    send_rx: &flume::Receiver<Vec<u8>>,
    quest_id: u32,
) {
    let bytes = send_rx
        .try_recv()
        .expect("quest giver quest details packet");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestDetails)
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == quest_id.to_le_bytes())
    );
    assert!(send_rx.try_recv().is_err());
}

fn recv_quest_giver_request_items_like_cpp(
    send_rx: &flume::Receiver<Vec<u8>>,
    quest_id: u32,
) -> (Vec<(i32, i32, u32)>, bool) {
    let bytes = send_rx
        .try_recv()
        .expect("quest giver request items packet");
    let mut pkt = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverRequestItems)
    );
    pkt.skip_opcode();
    let _giver_guid = pkt.read_packed_guid().expect("giver guid");
    let giver_creature_id = pkt.read_int32().expect("giver creature id");
    assert_eq!(pkt.read_int32().expect("quest id"), quest_id as i32);
    let _comp_emote_delay = pkt.read_int32().expect("comp emote delay");
    let _comp_emote_type = pkt.read_int32().expect("comp emote type");
    for _ in 0..3 {
        let _ = pkt.read_uint32().expect("quest flags");
    }
    let _suggested_party_members = pkt.read_int32().expect("suggested party members");
    let _money_to_get = pkt.read_int32().expect("money to get");
    let collect_count = pkt.read_int32().expect("collect count");
    let currency_count = pkt.read_int32().expect("currency count");
    let _status_flags = pkt.read_int32().expect("status flags");
    let mut collect = Vec::new();
    for _ in 0..collect_count {
        collect.push((
            pkt.read_int32().expect("collect object id"),
            pkt.read_int32().expect("collect amount"),
            pkt.read_uint32().expect("collect flags"),
        ));
    }
    for _ in 0..currency_count {
        let _currency_id = pkt.read_int32().expect("currency id");
        let _currency_amount = pkt.read_int32().expect("currency amount");
    }
    let auto_launched = pkt.read_bit().expect("auto launched bit");
    assert_eq!(
        pkt.read_int32().expect("repeated giver creature id"),
        giver_creature_id
    );
    assert_eq!(
        pkt.read_uint32()
            .expect("conditional completion text count"),
        0
    );
    assert!(send_rx.try_recv().is_err());
    (collect, auto_launched)
}

#[tokio::test]
async fn quest_giver_request_reward_completes_ready_quest_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = session.player_guid().expect("player guid");
    let quest_id = 9021;
    let mut quest = quest_template(quest_id);
    quest.flags = QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest(&mut session, quest_id);

    session
        .handle_quest_giver_request_reward(quest_giver_request_reward_packet_like_cpp(
            player_guid,
            quest_id,
        ))
        .await;

    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("quest should still be active before choose-reward")
            .status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    recv_quest_giver_offer_reward_contains_quest_id(&send_rx, quest_id);
}

fn recv_quest_giver_offer_reward_contains_quest_id(
    send_rx: &flume::Receiver<Vec<u8>>,
    quest_id: u32,
) {
    let bytes = send_rx.try_recv().expect("quest giver offer reward packet");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverOfferRewardMessage)
    );
    assert!(
        bytes
            .windows(4)
            .any(|window| window == quest_id.to_le_bytes())
    );
    assert!(send_rx.try_recv().is_err());
}

fn assert_success_command_queued_like_cpp(
    sender_rx: &flume::Receiver<Vec<u8>>,
    receiver_rx: &flume::Receiver<Vec<u8>>,
    receiver_session: &mut WorldSession,
    receiver_guid: ObjectGuid,
    sender_guid: ObjectGuid,
    quest_id: u32,
) {
    assert_eq!(
        recv_push_quest_result_response(sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
            String::new()
        )
    );
    assert!(receiver_rx.try_recv().is_err());
    let commands = receiver_session.drain_session_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
            assert_eq!(command.sender_guid, sender_guid);
            assert_eq!(command.quest.id, quest_id);
        }
        other => panic!("unexpected session command: {other:?}"),
    }
}

fn recv_status(send_rx: &flume::Receiver<Vec<u8>>) -> (ObjectGuid, u64) {
    let bytes = send_rx.try_recv().expect("quest giver status packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QuestGiverStatus as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let guid = pkt.read_packed_guid().unwrap();
    let status = pkt.read_uint64().unwrap();
    (guid, status)
}

fn recv_status_multiple(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<(ObjectGuid, u64)> {
    let bytes = send_rx
        .try_recv()
        .expect("quest giver status multiple packet");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QuestGiverStatusMultiple as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let count = pkt.read_int32().unwrap();
    assert!(count >= 0);
    let mut statuses = Vec::new();
    for _ in 0..count {
        statuses.push((pkt.read_packed_guid().unwrap(), pkt.read_uint64().unwrap()));
    }
    statuses
}

fn mark_visible(session: &mut WorldSession, guid: ObjectGuid) {
    session.client_visible_guids_like_cpp.insert(guid);
}

fn mark_visible_gameobject_questgiver(session: &mut WorldSession, guid: ObjectGuid) {
    let mut state = crate::session::RepresentedGameObjectUseState::default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER as u8);
    session
        .represented_gameobject_use_states
        .insert(guid, state);
    mark_visible(session, guid);
}

fn assert_confirm_accept_outcome(
    session: &WorldSession,
    receiver_guid: Option<ObjectGuid>,
    sender_guid: ObjectGuid,
    quest_id: u32,
    raw_quest_id: i32,
    reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
) {
    let success_boundary = matches!(
        reason,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::AddQuestRuntimeUnrepresented
    );
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid,
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id,
            reason,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: None,
            add_quest_runtime_unrepresented: success_boundary,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
}

fn assert_complete_status_update_like_cpp(
    session: &WorldSession,
    quest_id: u32,
    tracking_event_auto_reward_unrepresented: bool,
) {
    assert_eq!(
        session.represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        }]
    );
}

fn install_confirm_accept_sender_snapshot(
    session: &mut WorldSession,
    sender_guid: ObjectGuid,
    quest_id: u32,
    same_group: bool,
    sender_active_status: Option<u8>,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_loaded_player_name_like_cpp("Receiver".to_string());
    session.register_in_player_registry();

    let (mut sender_session, sender_rx) = make_session();
    sender_session.set_player_guid(Some(sender_guid));
    sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
    sender_session.set_player_registry(player_registry);
    sender_session.register_in_player_registry();
    assert!(sender_session.adopt_registered_canonical_player_fixture_like_cpp());
    if let Some(status) = sender_active_status {
        add_active_quest_in_slot_with_status(&mut sender_session, quest_id, 0, status);
    }
    sender_session.sync_player_registry_state_like_cpp();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    if same_group {
        if let Some(receiver_guid) = session.player_guid() {
            group.add_member(receiver_guid);
        }
    }
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    (sender_session, sender_rx)
}

#[tokio::test]
async fn quest_confirm_accept_short_packet_does_not_clear_pending_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 81);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7001);

    session
        .handle_quest_confirm_accept(WorldPacket::from_bytes(&[0x59, 0x1B, 0x00]))
        .await;

    assert_eq!(
        session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: 7001,
        })
    );
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_no_pending_valid_packet_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7002])));

    run_quest_confirm_accept(&mut session, 7002).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_mismatch_preserves_pending_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 82);
    session.set_quest_store(Arc::new(store_with_quests(&[7003])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7003);

    run_quest_confirm_accept(&mut session, 7004).await;

    assert_eq!(
        session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: 7003,
        })
    );
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_match_missing_template_clears_without_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 83);
    session.set_quest_store(Arc::new(store_with_quests(&[7005])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7006);

    run_quest_confirm_accept(&mut session, 7006).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_confirm_accepts_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_match_template_records_original_player_missing_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 84);
    let receiver_guid = ObjectGuid::create_player(1, 42);
    session.set_quest_store(Arc::new(store_with_quests(&[7007])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7007);

    run_quest_confirm_accept(&mut session, 7007).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(receiver_guid),
        sender_guid,
        7007,
        7007,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_negative_raw_id_compares_as_u32_bit_pattern_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 85);
    let quest_id = u32::MAX;
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);

    run_quest_confirm_accept(&mut session, -1).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        -1,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_sender_exists_not_same_group_records_not_in_same_raid_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 86);
    session.set_quest_store(Arc::new(store_with_quests(&[7008])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7008);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        7008,
        false,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, 7008).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        7008,
        7008,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_same_group_sender_not_active_records_original_not_active_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 87);
    session.set_quest_store(Arc::new(store_with_quests(&[7009])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7009);
    let (_sender_session, sender_rx) =
        install_confirm_accept_sender_snapshot(&mut session, sender_guid, 7009, true, None);

    run_quest_confirm_accept(&mut session, 7009).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        7009,
        7009,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerNotActiveQuest,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_same_group_sender_active_can_take_failed_records_receiver_gate_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 88);
    let quest_id = 7010;
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    session.rewarded_quests.insert(quest_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanTakeQuestFailed,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_can_take_ok_log_full_records_can_add_log_full_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 89);
    let quest_id = 7011;
    session.set_quest_store(Arc::new(store_with_quests(&[quest_id])));
    for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
        add_active_quest_in_slot_with_status(
            &mut session,
            80_000 + u32::from(slot),
            slot,
            QUEST_STATUS_COMPLETE_LIKE_CPP,
        );
    }
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_no_source_side_effects_adds_local_quest_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 90);
    let quest_id = 7012;
    session.set_quest_store(Arc::new(store_with_sharable_timed_quest_objectives(
        quest_id, 3, 600,
    )));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("receiver quest log should receive bounded local AddQuest state");
    assert_eq!(status.quest_id, quest_id);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(!status.explored);
    assert!(status.accept_time_secs > 0);
    assert_eq!(status.end_time_secs, status.accept_time_secs + 600);
    assert_eq!(status.objective_counts, vec![0, 0, 0]);
    assert_eq!(status.slot, 0);
    let registry = session.player_registry().expect("test installs registry");
    let snapshot = registry
        .loot_player_context(receiver_guid)
        .expect("receiver canonical state should sync after quest insertion");
    assert_eq!(
        snapshot.active_quest_statuses.get(&quest_id),
        Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_eq!(
        snapshot.active_quest_objective_counts.get(&quest_id),
        Some(&vec![0, 0, 0])
    );
    assert_confirm_accept_outcome(
        &session,
        Some(receiver_guid),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_first_free_slot_skips_occupied_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 190);
    let occupied_quest_id = 8000;
    let quest_id = 70120;
    session.set_quest_store(Arc::new(store_with_sharable_quest_objectives(quest_id, 1)));
    add_active_quest_in_slot_with_status(
        &mut session,
        occupied_quest_id,
        0,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let occupied_status = session
        .player_quests
        .get(&occupied_quest_id)
        .expect("pre-existing quest should remain in slot 0");
    assert_eq!(occupied_status.slot, 0);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("accepted quest should be inserted into first free slot");
    assert_eq!(status.slot, 1);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    let registry = session.player_registry().expect("test installs registry");
    let snapshot = registry
        .loot_player_context(receiver_guid)
        .expect("receiver canonical state should sync after quest insertion");
    assert_eq!(
        snapshot.active_quest_statuses.get(&quest_id),
        Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_confirm_accept_outcome(
        &session,
        Some(receiver_guid),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_spell_records_two_self_casts_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 191);
    let quest_id = 70121;
    let mut quest = quest_template_with_objective_count(quest_id, 2);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.source_spell_id = 12_345;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-spell-only quest should still insert represented local AddQuest state");
    assert_eq!(status.quest_id, quest_id);
    assert_eq!(status.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(!status.explored);
    assert_eq!(status.objective_counts, vec![0, 0]);
    assert_eq!(status.slot, 0);
    let registry = session.player_registry().expect("test installs registry");
    let snapshot = registry
        .loot_player_context(receiver_guid)
        .expect("receiver canonical state should sync after source-spell quest insertion");
    assert_eq!(
        snapshot.active_quest_statuses.get(&quest_id),
        Some(&QUEST_STATUS_INCOMPLETE_LIKE_CPP)
    );
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: None,
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(12_345),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_start_quest_no_grant_adds_local_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 90);
    let quest_id = 7012;
    let source_item_id = 9000;
    let source_spell_id = 12_344;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        2,
        source_spell_id,
    )));
    install_source_item_template_with_start_quest(
        &mut session,
        source_item_id,
        20,
        0,
        quest_id as i32,
    );
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session
            .represented_inventory_item_counts_like_cpp()
            .expect("fixture canonical inventory owner")
            .get(&source_item_id)
            .copied()
            .unwrap_or(0),
        0
    );
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStartQuestNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(source_spell_id),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_with_space_stores_and_pushes_item_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 91);
    let quest_id = 7013;
    let source_item_id = 9001;
    let source_spell_id = 12_346;
    let quest_log_item_id = 9101;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, source_spell_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should still add local quest state")
            .status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("source-item quest should still add local quest state")
            .objective_counts,
        vec![2]
    );
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    let stored_source_item_slot = session
        .inventory_items_like_cpp()
        .iter()
        .find_map(|(&slot, item)| (item.entry_id == source_item_id).then_some(slot))
        .expect("source item should have a direct inventory slot");
    assert_eq!(stored_source_item_count, 2);
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(source_spell_id),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    let mut sent = Vec::new();
    while let Ok(packet) = send_rx.try_recv() {
        sent.push(packet);
    }
    assert!(
        sent.len() >= 3,
        "StoreNewItem(update=true) should create item/update player and SendNewItem"
    );
    let mut saw_item_push = false;
    for bytes in &sent {
        let mut packet = WorldPacket::from_bytes(bytes);
        if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16 {
            continue;
        }
        saw_item_push = true;
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
        );
        assert_eq!(
            packet.read_int32().unwrap(),
            i32::from(stored_source_item_slot)
        );
        assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 2);
    }
    assert!(
        saw_item_push,
        "source item grant should send ItemPushResult"
    );
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_full_backpack_stores_in_represented_bag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 98);
    let quest_id = 7020;
    let source_item_id = 9007;
    let bag_item_id = 9107;
    let filler_item_id = 9108;
    let bag_guid = ObjectGuid::create_item(1, 9_107);

    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        3,
        0,
    )));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: source_item_id,
            class_id: ItemClass::Consumable as u8,
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
            id: bag_item_id,
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
            id: filler_item_id,
            class_id: ItemClass::Consumable as u8,
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
    let sparse = |inventory_type: InventoryType, stackable: i32, container_slots: u8| {
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
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
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots,
            inventory_type: inventory_type as i8,
        }
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (source_item_id, sparse(InventoryType::NonEquip, 20, 0)),
        (bag_item_id, sparse(InventoryType::Bag, 1, 4)),
        (filler_item_id, sparse(InventoryType::NonEquip, 1, 0)),
    ])));

    session.insert_inventory_item_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: bag_item_id,
            db_guid: 9_107,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_item_id,
        receiver_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag_item);
    for slot_offset in 0..wow_entities::INVENTORY_DEFAULT_SIZE {
        insert_direct_inventory_item(
            &mut session,
            receiver_guid,
            wow_entities::INVENTORY_SLOT_ITEM_START + slot_offset,
            filler_item_id,
            1,
            9_200 + u64::from(slot_offset),
        );
    }

    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert!(session.player_quests.contains_key(&quest_id));
    assert!(
        session
            .inventory_items_like_cpp()
            .values()
            .all(|item| item.entry_id != source_item_id)
    );
    let child = session
        .inventory_item_objects_like_cpp()
        .values()
        .find(|item| item.object().entry() == source_item_id)
        .expect("source item should be created inside represented bag");
    assert_eq!(child.container_guid(), bag_guid);
    assert_eq!(child.bag_slot(), wow_entities::INVENTORY_SLOT_BAG_START);
    assert_eq!(child.slot(), 0);
    assert_eq!(child.count(), 3);
    assert_eq!(
        session
            .represented_inventory_item_counts_like_cpp()
            .expect("fixture canonical inventory owner")
            .get(&source_item_id)
            .copied(),
        Some(3)
    );

    let mut saw_item_push = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16 {
            continue;
        }
        saw_item_push = true;
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            wow_entities::INVENTORY_SLOT_BAG_START
        );
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 3);
        assert_eq!(packet.read_int32().unwrap(), 3);
    }
    assert!(saw_item_push);
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_merges_existing_stack_inside_represented_bag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 99);
    let quest_id = 7021;
    let source_item_id = 9008;
    let bag_item_id = 9109;
    let filler_item_id = 9110;
    let bag_guid = ObjectGuid::create_item(1, 9_109);
    let child_guid = ObjectGuid::create_item(1, 9_110);

    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        2,
        0,
    )));
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: source_item_id,
            class_id: ItemClass::Consumable as u8,
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
            id: bag_item_id,
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
            id: filler_item_id,
            class_id: ItemClass::Consumable as u8,
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
    let sparse = |inventory_type: InventoryType, stackable: i32, container_slots: u8| {
        ItemSparseTemplateEntry {
            flags: [0, 0, 0, 0],
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
            zone_bound: [0, 0],
            required_reputation_faction: 0,
            allowable_class: -1,
            required_expansion: 0,
            bonding: ItemBondingType::None as u8,
            container_slots,
            inventory_type: inventory_type as i8,
        }
    };
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (source_item_id, sparse(InventoryType::NonEquip, 20, 0)),
        (bag_item_id, sparse(InventoryType::Bag, 1, 4)),
        (filler_item_id, sparse(InventoryType::NonEquip, 1, 0)),
    ])));

    session.insert_inventory_item_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: bag_item_id,
            db_guid: 9_109,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        bag_item_id,
        receiver_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::INVENTORY_SLOT_BAG_START,
    );
    session.insert_inventory_item_object(bag_item);
    let mut child = session.make_inventory_item_object(
        child_guid,
        source_item_id,
        receiver_guid,
        18,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, wow_entities::INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(child);
    for slot_offset in 0..wow_entities::INVENTORY_DEFAULT_SIZE {
        insert_direct_inventory_item(
            &mut session,
            receiver_guid,
            wow_entities::INVENTORY_SLOT_ITEM_START + slot_offset,
            filler_item_id,
            1,
            9_300 + u64::from(slot_offset),
        );
    }

    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let child = session
        .inventory_item_objects_like_cpp()
        .get(&child_guid)
        .expect("existing represented bag stack should remain");
    assert_eq!(child.count(), 20);
    assert_eq!(child.container_guid(), bag_guid);
    assert_eq!(child.bag_slot(), wow_entities::INVENTORY_SLOT_BAG_START);
    assert_eq!(child.slot(), 0);
    assert_eq!(
        session
            .represented_inventory_item_counts_like_cpp()
            .expect("fixture canonical inventory owner")
            .get(&source_item_id)
            .copied(),
        Some(20)
    );

    let mut saw_item_push = false;
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        if packet.read_uint16().unwrap() != wow_constants::ServerOpcodes::ItemPushResult as u16 {
            continue;
        }
        saw_item_push = true;
        assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
        assert_eq!(
            packet.read_uint8().unwrap(),
            wow_entities::INVENTORY_SLOT_BAG_START
        );
        assert_eq!(packet.read_int32().unwrap(), -1);
        assert_eq!(packet.read_int32().unwrap(), 0);
        assert_eq!(packet.read_int32().unwrap(), 2);
        assert_eq!(packet.read_int32().unwrap(), 20);
    }
    assert!(saw_item_push);
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_binds_on_acquire_like_cpp_store_item() {
    let (mut session, _send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 203);
    let quest_id = 7122;
    let source_item_id = 9210;
    let quest = quest_template_with_source_item(quest_id, source_item_id, 1, 0);
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
        &mut session,
        source_item_id,
        20,
        0,
        0,
        0,
        0,
        ItemBondingType::OnAcquire,
    );
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let stored = session
        .inventory_items_like_cpp()
        .values()
        .find(|item| item.entry_id == source_item_id)
        .and_then(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .expect("source item should be stored as runtime item object");
    assert_eq!(stored.bonding(), ItemBondingType::OnAcquire);
    assert!(stored.is_soul_bound());
    assert_eq!(
        stored.item_flags_bits() & ItemFieldFlags::SOULBOUND.bits(),
        ItemFieldFlags::SOULBOUND.bits()
    );
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_bound_objective_updates_quest_without_creating_item_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 191);
    let quest_id = 7113;
    let source_item_id = 9201;
    let source_spell_id = 12_347;
    let quest_log_item_id = 9301;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, source_spell_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    let status = session
        .player_quests
        .get(&quest_id)
        .expect("bound source-item quest should still add local quest state");
    assert_eq!(status.status, QUEST_STATUS_COMPLETE_LIKE_CPP);
    assert_eq!(status.objective_counts, vec![2]);
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    assert_eq!(stored_source_item_count, 0);
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: Some(source_spell_id),
            represented_source_spell_self_casts: 2,
        }]
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);

    let sent = send_rx.try_recv().expect("bound item update packet");
    assert!(send_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&sent);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_uint32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(!packet.read_bit().unwrap());
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(3).unwrap(), 3);
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_tracking_event_source_item_objective_auto_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 194);
    let quest_id = 7119;
    let source_item_id = 9204;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 1, 0);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: source_item_id as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_complete_status_update_like_cpp(&session, quest_id, false);

    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::QuestGiverQuestComplete));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::QuestUpdateComplete));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::ItemPushResult));
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_bound_objective_broadcasts_to_group_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 193);
    let other_guid = ObjectGuid::create_player(1, 194);
    let quest_id = 7115;
    let source_item_id = 9203;
    let quest_log_item_id = 9303;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_loaded_player_name_like_cpp("Receiver".to_string());
    session.register_in_player_registry();

    let (mut sender_session, sender_rx) = make_session();
    sender_session.set_player_guid(Some(sender_guid));
    sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
    sender_session.set_player_registry(Arc::clone(&player_registry));
    sender_session.register_in_player_registry();
    assert!(sender_session.adopt_registered_canonical_player_fixture_like_cpp());
    add_active_quest_in_slot_with_status(
        &mut sender_session,
        quest_id,
        0,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    sender_session.sync_player_registry_state_like_cpp();

    let (mut other_session, other_rx) = make_session();
    other_session.set_player_guid(Some(other_guid));
    other_session.set_loaded_player_name_like_cpp("Other".to_string());
    other_session.set_player_registry(player_registry);
    other_session.register_in_player_registry();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    group.add_member(receiver_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let self_packet = send_rx.try_recv().expect("receiver group packet");
    assert_eq!(sender_rx.try_recv().unwrap(), self_packet);
    assert_eq!(other_rx.try_recv().unwrap(), self_packet);
    assert!(send_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&self_packet);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_uint32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(!packet.read_bit().unwrap());
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(3).unwrap(), 3);
}

#[tokio::test]
async fn quest_confirm_accept_source_item_bound_objective_dont_report_flag_sends_direct_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 195);
    let other_guid = ObjectGuid::create_player(1, 196);
    let quest_id = 7116;
    let source_item_id = 9204;
    let quest_log_item_id = 9304;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: quest_log_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template_with_flags3(
        &mut session,
        source_item_id,
        20,
        0,
        ItemFlags3::DontReportLootLogToParty as u32,
    );
    session.cache_item_template_addon_quest_log_item_id_like_cpp(source_item_id, quest_log_item_id);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_loaded_player_name_like_cpp("Receiver".to_string());
    session.register_in_player_registry();

    let (mut sender_session, sender_rx) = make_session();
    sender_session.set_player_guid(Some(sender_guid));
    sender_session.set_loaded_player_name_like_cpp("Sender".to_string());
    sender_session.set_player_registry(Arc::clone(&player_registry));
    sender_session.register_in_player_registry();
    assert!(sender_session.adopt_registered_canonical_player_fixture_like_cpp());
    add_active_quest_in_slot_with_status(
        &mut sender_session,
        quest_id,
        0,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    sender_session.sync_player_registry_state_like_cpp();

    let (mut other_session, other_rx) = make_session();
    other_session.set_player_guid(Some(other_guid));
    other_session.set_loaded_player_name_like_cpp("Other".to_string());
    other_session.set_player_registry(player_registry);
    other_session.register_in_player_registry();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    group.add_member(receiver_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let sent = send_rx.try_recv().expect("direct bound item update packet");
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
    assert!(other_rx.try_recv().is_err());
    let mut packet = WorldPacket::from_bytes(&sent);
    assert_eq!(
        packet.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::ItemPushResult as u16
    );
    assert_eq!(packet.read_packed_guid().unwrap(), receiver_guid);
    assert_eq!(
        packet.read_uint8().unwrap(),
        u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
    );
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), quest_log_item_id as i32);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 2);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_uint32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(!packet.read_bit().unwrap());
    assert!(!packet.read_bit().unwrap());
    assert_eq!(packet.read_bits(3).unwrap(), 3);
}

#[tokio::test]
async fn quest_confirm_accept_source_item_multiple_bound_objectives_stops_after_first_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 192);
    let quest_id = 7114;
    let source_item_id = 9202;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: source_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: source_item_id as i32,
        amount: 2,
        flags: 0,
        flags2: QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![2, 0]);
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    assert_eq!(stored_source_item_count, 0);
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::Ok),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    let sent: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|bytes| {
            let mut packet = WorldPacket::from_bytes(bytes);
            packet.read_uint16().ok() == Some(wow_constants::ServerOpcodes::ItemPushResult as u16)
        }),
        "C++ sends the bound-objective ItemPushResult without materializing an inventory Item"
    );
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_sequenced_objective_waits_for_previous_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 197);
    let quest_id = 7117;
    let source_item_id = 9205;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 9901,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: source_item_id as i32,
        amount: 2,
        flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![0, 0]);
    let stored_source_item_count: u32 = session
        .inventory_items_like_cpp()
        .values()
        .filter(|item| item.entry_id == source_item_id)
        .filter_map(|item| session.inventory_item_objects_like_cpp().get(&item.guid))
        .map(|item| item.count())
        .sum();
    assert_eq!(stored_source_item_count, 2);
    let sent: Vec<_> = std::iter::from_fn(|| send_rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|bytes| {
            let mut packet = WorldPacket::from_bytes(bytes);
            packet.read_uint16().ok() == Some(wow_constants::ServerOpcodes::ItemPushResult as u16)
        }),
        "source item is still granted; only sequenced objective progress is blocked"
    );
    assert!(sender_rx.try_recv().is_err());
    assert_eq!(session.player_guid(), Some(receiver_guid));
}

#[tokio::test]
async fn quest_confirm_accept_source_item_optional_previous_allows_sequenced_objective_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 198);
    let quest_id = 7118;
    let source_item_id = 9206;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 9902,
        amount: 1,
        flags: QUEST_OBJECTIVE_FLAG_OPTIONAL_LIKE_CPP_LOCAL,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: source_item_id as i32,
        amount: 2,
        flags: QUEST_OBJECTIVE_FLAG_SEQUENCED_LIKE_CPP_LOCAL,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![0, 2]);
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_progress_bar_part_objective_progresses_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 199);
    let quest_id = 7119;
    let source_item_id = 9207;
    let mut quest = quest_template_with_source_item(quest_id, source_item_id, 2, 0);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: source_item_id as i32,
        amount: 2,
        flags: QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
        flags2: 0,
        progress_bar_weight: 50.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: 0,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    install_source_item_template(&mut session, source_item_id, 20, 0);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("source-item quest should add local quest state");
    assert_eq!(status.objective_counts, vec![2, 0]);
    assert!(sender_rx.try_recv().is_err());
}

#[test]
fn represented_progress_bar_part_objective_stops_when_progress_bar_complete_like_cpp() {
    let quest_id = 7120;
    let mut quest = quest_template(quest_id);
    quest.objectives = vec![
        QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL,
            order: 0,
            storage_index: 0,
            object_id: 99,
            amount: 2,
            flags: QUEST_OBJECTIVE_FLAG_PART_OF_PROGRESS_BAR_LIKE_CPP_LOCAL,
            flags2: 0,
            progress_bar_weight: 50.0,
            description: String::new(),
        },
        QuestObjective {
            id: quest_id * 10 + 1,
            quest_id,
            obj_type: QUEST_OBJECTIVE_PROGRESS_BAR_LIKE_CPP_LOCAL,
            order: 1,
            storage_index: 1,
            object_id: 0,
            amount: 100,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        },
    ];
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs: 0,
        end_time_secs: 0,
        objective_counts: vec![2, 0],
        slot: 0,
    };

    assert!(!WorldSession::represented_quest_objective_completable_like_cpp(&status, &quest, 0));
}

#[tokio::test]
async fn quest_confirm_accept_source_item_zero_count_normalizes_to_one_and_fails_full_inventory_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 96);
    let quest_id = 7018;
    let source_item_id = 9005;
    let filler_item_id = 9105;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        0,
        0,
    )));
    install_source_item_template(&mut session, source_item_id, 1, 0);
    for slot in 35..59 {
        insert_direct_inventory_item(
            &mut session,
            receiver_guid,
            slot,
            filler_item_id,
            1,
            91_000 + u64::from(slot),
        );
    }
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(!session.player_quests.contains_key(&quest_id));
    let outcomes = session.represented_quest_confirm_accepts_like_cpp();
    assert_eq!(outcomes.len(), 1);
    let outcome = &outcomes[0];
    assert_eq!(outcome.receiver_guid, Some(receiver_guid));
    assert_eq!(outcome.sender_guid_before_clear, sender_guid);
    assert_eq!(outcome.quest_id, quest_id);
    assert_eq!(outcome.raw_quest_id, quest_id as i32);
    assert_eq!(
        outcome.reason,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed
    );
    assert!(!outcome.add_quest_runtime_unrepresented);
    let source_item_result = outcome
        .can_add_source_item_result
        .expect("zero ProvidedItemCount must normalize to one and reach planner failure");
    assert_ne!(source_item_result, InventoryResult::Ok);
    assert_ne!(source_item_result, InventoryResult::ItemMaxCount);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(source_item_result).to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_at_max_count_allows_can_add_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 92);
    let quest_id = 7014;
    let source_item_id = 9002;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    install_source_item_template(&mut session, source_item_id, 20, 1);
    insert_direct_inventory_item(&mut session, receiver_guid, 23, source_item_id, 1, 9002);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert!(session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemMaxCountNoGrant,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::ItemMaxCount),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_missing_source_item_proto_fails_can_add_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 93);
    let quest_id = 7015;
    let source_item_id = 9003;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason:
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::ItemNotFound),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemNotFound).to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_limit_category_missing_db2_entry_fails_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 95);
    let quest_id = 7017;
    let source_item_id = 9004;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    install_source_item_template_with_limit_category(&mut session, source_item_id, 20, 0, 44);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(!session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason:
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(InventoryResult::NotEquippable),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::NotEquippable).to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_source_item_start_quest_still_respects_limit_category_like_cpp() {
    let (mut session, send_rx) = make_session();
    let receiver_guid = session.player_guid().unwrap();
    let sender_guid = ObjectGuid::create_player(1, 97);
    let quest_id = 7019;
    let source_item_id = 9006;
    session.set_quest_store(Arc::new(store_with_source_item_quest(
        quest_id,
        source_item_id,
        1,
        0,
    )));
    install_source_item_template_with_start_quest_and_limit_category(
        &mut session,
        source_item_id,
        20,
        0,
        quest_id as i32,
        44,
    );
    session.set_item_limit_category_store(Arc::new(ItemLimitCategoryStore::from_entries([
        ItemLimitCategoryEntry {
            id: 44,
            name: "Quest Source Have Limit".into(),
            quantity: 1,
            flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
        },
    ])));
    insert_direct_inventory_item(&mut session, receiver_guid, 23, source_item_id, 1, 9906);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(!session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session.represented_quest_confirm_accepts_like_cpp(),
        &[RepresentedQuestConfirmAcceptLikeCpp {
            receiver_guid: Some(receiver_guid),
            sender_guid_before_clear: sender_guid,
            quest_id,
            raw_quest_id: quest_id as i32,
            reason:
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
            object_accessor_unrepresented: true,
            party_runtime_unrepresented: true,
            can_add_source_item_unrepresented: false,
            can_add_source_item_result: Some(
                InventoryResult::ItemMaxLimitCategoryCountExceededIs
            ),
            add_quest_runtime_unrepresented: false,
            source_spell_unrepresented: false,
            represented_source_spell_id: None,
            represented_source_spell_self_casts: 0,
        }]
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
            .with_limit_category(44)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_without_source_item_does_not_overclaim_source_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 94);
    let quest_id = 7016;
    session.set_quest_store(Arc::new(store_with_source_item_quest(quest_id, 0, 0, 0)));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(session.player_quests.contains_key(&quest_id));
    assert_eq!(
        session
            .player_quests
            .get(&quest_id)
            .expect("no-objective shared quest should be locally tracked")
            .status,
        QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_confirm_accept_tracking_event_auto_rewards_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 201);
    let quest_id = 7121;
    let mut quest = quest_template(quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
    let (_sender_session, sender_rx) = install_confirm_accept_sender_snapshot(
        &mut session,
        sender_guid,
        quest_id,
        true,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
    );

    run_quest_confirm_accept(&mut session, quest_id as i32).await;

    assert_confirm_accept_outcome(
        &session,
        Some(ObjectGuid::create_player(1, 42)),
        sender_guid,
        quest_id,
        quest_id as i32,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
    );
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_complete_status_update_like_cpp(&session, quest_id, false);
    let slot_update = send_rx
        .try_recv()
        .expect("tracking event auto reward should clear quest log slot");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&slot_update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    let complete = send_rx
        .try_recv()
        .expect("tracking event auto reward should send quest complete");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&complete).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestGiverQuestComplete)
    );
    let update = send_rx
        .try_recv()
        .expect("tracking event auto reward should send quest update complete");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::QuestUpdateComplete)
    );
    assert!(send_rx.try_recv().is_err());
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_push_short_packet_does_not_clear_pending_state_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 77);
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7001);

    session
        .handle_quest_push_result(WorldPacket::from_bytes(&[0x00]))
        .await;

    assert_eq!(
        session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: 7001,
        })
    );
    assert!(
        session
            .represented_quest_push_result_responses_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_push_no_pending_valid_packet_is_noop_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 78);

    run_quest_push_result(&mut session, sender_guid, 7002, 3).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_push_result_responses_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_push_pending_sender_match_clears_and_records_response_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = ObjectGuid::create_player(1, 79);
    let receiver_guid = session.player_guid().unwrap();
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, 7003);

    run_quest_push_result(&mut session, sender_guid, 8003, 6).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert_eq!(
        session.represented_quest_push_result_responses_like_cpp(),
        &[RepresentedQuestPushResultResponseLikeCpp {
            receiver_guid,
            sender_guid,
            parsed_quest_id: 8003,
            pending_quest_id: 7003,
            result: 6,
        }]
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_push_pending_sender_mismatch_clears_without_response_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    let pending_sender_guid = ObjectGuid::create_player(1, 80);
    let packet_sender_guid = ObjectGuid::create_player(1, 81);
    session.set_represented_pending_quest_sharing_like_cpp(pending_sender_guid, 7004);

    run_quest_push_result(&mut session, packet_sender_guid, 7004, 4).await;

    assert_eq!(session.represented_pending_quest_sharing_like_cpp(), None);
    assert!(
        session
            .represented_quest_push_result_responses_like_cpp()
            .is_empty()
    );
    assert_eq!(
        session.represented_quest_push_result_sender_mismatch_count_like_cpp(),
        1
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_push_inventory_registration_and_dispatcher_contract_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QuestPushResult)
        .expect("QuestPushResult handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_quest_push_result");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains("session.handle_quest_push_result(pkt).await"),
        "the QuestPushResult registration must carry the call itself"
    );
}

#[test]
fn quest_push_packet_parser_reads_sender_quest_id_result_in_cpp_order() {
    let sender_guid = ObjectGuid::create_player(1, 82);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&sender_guid);
    pkt.write_uint32(7005);
    pkt.write_uint8(9);

    let parsed = QuestPushResult::read(&mut pkt).expect("valid QuestPushResult");

    assert_eq!(parsed.sender_guid, sender_guid);
    assert_eq!(parsed.quest_id, 7005);
    assert_eq!(parsed.result, 9);
}

#[tokio::test]
async fn push_quest_to_party_malformed_packet_records_no_evidence_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_sharable_quest(7101)));
    add_active_quest(&mut session, 7101);

    session
        .handle_push_quest_to_party(WorldPacket::from_bytes(&[0x9F, 0x34, 0x00]))
        .await;

    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .is_empty()
    );
    assert!(
        session
            .represented_pending_quest_sharing_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn push_quest_to_party_missing_quest_template_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[7102])));

    run_push_quest_to_party(&mut session, 7103).await;

    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn push_quest_to_party_unshareable_or_not_in_log_records_not_allowed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    session.set_quest_store(Arc::new(store_with_sharable_quest(7104)));

    run_push_quest_to_party(&mut session, 7104).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7104,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotAllowed,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert_eq!(
        recv_push_quest_result_response(&send_rx),
        (
            sender_guid.expect("test session has player guid"),
            quest_push_reason::NOT_ALLOWED,
            String::new()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_shareable_sender_without_pool_store_still_blocks_before_group_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    session.set_quest_store(Arc::new(store_with_sharable_quest(7105)));
    add_active_quest(&mut session, 7105);

    run_push_quest_to_party(&mut session, 7105).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7105,
            target_guid: sender_guid,
            reason:
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::QuestPoolActiveCheckUnrepresented,
            quest_pool_active_check_unrepresented: true,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert!(
        session
            .represented_pending_quest_sharing_like_cpp()
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn push_quest_to_party_inactive_pooled_quest_records_not_daily_before_group_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_daily_sharable_quests(&[7106, 7107]);
    let quest_pool_store =
        quest_pool_store_with_active_saved(&quest_store, 77, &[7106, 7107], &[7107]);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7106);
    session.group_guid = Some(99);

    run_push_quest_to_party(&mut session, 7106).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7106,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotDaily,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert_eq!(
        recv_push_quest_result_response(&send_rx),
        (
            sender_guid.expect("test session has player guid"),
            quest_push_reason::NOT_DAILY,
            String::new()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_active_pooled_quest_passes_pool_check_to_not_in_party_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_daily_sharable_quests(&[7108, 7109]);
    let quest_pool_store =
        quest_pool_store_with_active_saved(&quest_store, 78, &[7108, 7109], &[7108]);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7108);

    run_push_quest_to_party(&mut session, 7108).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7108,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::NotInParty,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: false,
            receiver_fanout_unrepresented: false,
        }]
    );
    assert_eq!(
        recv_push_quest_result_response(&send_rx),
        (
            sender_guid.expect("test session has player guid"),
            quest_push_reason::NOT_IN_PARTY,
            String::new()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_non_pooled_quest_passes_pool_check_to_group_boundary_like_cpp() {
    let (mut session, send_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_sharable_quest(7110);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7110);
    session.group_guid = Some(99);

    run_push_quest_to_party(&mut session, 7110).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7110,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: true,
            receiver_fanout_unrepresented: true,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

/// Put one party member's canonical `Player` on the shared map.
///
/// Mirrors what a live session does at world entry; the quest-share gates read
/// reputation off this owner since #252.
///
/// Takes the three values it needs rather than `&WorldSession`: the session type
/// carries database handles, so accepting it here would register this fixture as
/// a direct persistence accessor in the ownership inventory for no reason.
fn insert_canonical_party_player_like_cpp(
    account_id: u32,
    player_guid: ObjectGuid,
    position: Position,
    canonical: &crate::session::SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
) {
    let mut player = wow_entities::Player::new(Some(u64::from(account_id)), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player
        .unit_mut()
        .world_mut()
        .set_map(map_id, instance_id)
        .unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player.unit_mut().set_faction(1);
    canonical
        .lock()
        .unwrap()
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

/// Set one faction standing on a party member's canonical `Player`.
fn set_canonical_party_reputation_like_cpp(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    faction_id: u32,
    standing: i32,
) {
    let mut guard = canonical.lock().unwrap();
    let player = guard
        .find_map_mut(571, 0)
        .expect("resident party map")
        .map_mut()
        .get_typed_player_mut(guid)
        .expect("canonical party member");
    player
        .gameplay_state_mut()
        .reputations
        .retain(|record| record.faction_id != faction_id);
    player
        .gameplay_state_mut()
        .reputations
        .push(wow_entities::PlayerReputationRecord {
            faction_id,
            standing,
            flags: 0,
        });
}

fn install_represented_party(
    session: &mut WorldSession,
    sender_guid: ObjectGuid,
    receiver_guid: ObjectGuid,
) -> (Arc<PlayerRegistry>, WorldSession, flume::Receiver<Vec<u8>>) {
    let player_registry = Arc::new(PlayerRegistry::default());
    let (mut receiver_session, receiver_rx) = make_session();
    receiver_session.set_player_guid(Some(receiver_guid));
    receiver_session.set_loaded_player_name_like_cpp("Receiver".to_string());
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.set_player_position_like_cpp(Position::new(11.0, 0.0, 0.0, 0.0));
    receiver_session.set_player_registry(Arc::clone(&player_registry));

    // Production keeps every in-world player on the shared canonical map, and
    // #252 reads the receiver's reputation off that owner instead of a mirrored
    // copy. Install it here so the harness exercises the same path.
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    insert_canonical_party_player_like_cpp(
        receiver_session.account_id,
        receiver_guid,
        receiver_session
            .player_position_like_cpp()
            .expect("party member position"),
        &canonical,
        571,
        0,
    );
    receiver_session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_canonical_map_manager(Arc::clone(&canonical));

    receiver_session.register_in_player_registry();
    assert!(receiver_session.adopt_registered_canonical_player_fixture_like_cpp());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(sender_guid);
    group.add_member(receiver_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry.clone());
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    (player_registry, receiver_session, receiver_rx)
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_on_quest_emits_on_quest_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 43);
    let quest_store = store_with_sharable_quest(7111);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7111);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest(&mut receiver_session, 7111);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, 7111).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP,
            "Quest 7111".to_string()
        )
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_rewarded_emits_already_done_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 44);
    let quest_store = store_with_sharable_quest(7112);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7112);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, 7112);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, 7112).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7112".to_string()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_log_full_emits_log_full_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 144);
    let shared_quest_id = 7116;
    let quest_store = store_with_sharable_quest(shared_quest_id);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
        add_active_quest_in_slot(&mut receiver_session, 8000 + u32::from(slot), slot);
    }
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7116".to_string()
        )
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_daily_completed_emits_already_done_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 244);
    let shared_quest_id = 7117;
    let quest_store = store_with_daily_sharable_quests(&[shared_quest_id]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7117".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_df_completed_emits_already_done_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 245);
    let shared_quest_id = 7118;
    let quest_store = store_with_df_sharable_quest(shared_quest_id);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_represented_df_quest_like_cpp_for_test(shared_quest_id, true);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7118".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_non_daily_non_df_ignores_unrelated_daily_snapshot_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 246);
    let shared_quest_id = 7119;
    let quest_store = store_with_sharable_quest(shared_quest_id);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(9001, true);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
            outcome.reason,
            RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
        ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_low_level_emits_low_level_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 248);
    let shared_quest_id = 7121;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 10, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_level_like_cpp(4);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7121".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_high_level_emits_high_level_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 249);
    let shared_quest_id = 7122;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 1, 40);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_level_like_cpp(80);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7122".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_receiver_max_level_zero_does_not_block_high_level_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 250);
    let shared_quest_id = 7123;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 1, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_level_like_cpp(80);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMaxLevelHighLevel
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_wrong_class_emits_class_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 251);
    let shared_quest_id = 7124;
    let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 1 << (2 - 1), 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();
    assert_eq!(
        player_registry
            .quest_sharing_snapshot(receiver_guid, None)
            .expect("receiver snapshot")
            .class,
        1
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_CLASS_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
            "Quest 7124".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_wrong_race_emits_race_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 252);
    let shared_quest_id = 7125;
    let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 0, 1 << (2 - 1));
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();
    assert_eq!(
        player_registry
            .quest_sharing_snapshot(receiver_guid, None)
            .expect("receiver snapshot")
            .race,
        1
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_RACE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7125".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_receiver_class_precedes_race_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 253);
    let shared_quest_id = 7126;
    let quest_store =
        store_with_sharable_quest_class_race(shared_quest_id, 1 << (2 - 1), 1 << (2 - 1));
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_CLASS_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
            "Quest 7126".to_string()
        )
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_zero_class_and_race_masks_do_not_block_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 254);
    let shared_quest_id = 7127;
    let quest_store = store_with_sharable_quest_class_race(shared_quest_id, 0, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass
                    | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestRaceWrongRace
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_low_min_reputation_emits_low_faction_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 255);
    let shared_quest_id = 7128;
    let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 72, 100, 0, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
        receiver_guid,
        72,
        99,
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
            "Quest 7128".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_equal_max_reputation_emits_low_faction_pair_like_cpp()
{
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 256);
    let shared_quest_id = 7129;
    let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 0, 0, 72, 100);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
        receiver_guid,
        72,
        100,
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
            "Quest 7129".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
}

#[tokio::test]
async fn push_quest_to_party_zero_reputation_factions_do_not_block_with_missing_snapshot_like_cpp()
{
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 257);
    let shared_quest_id = 7130;
    let quest_store = store_with_sharable_quest_reputation(shared_quest_id, 0, 999, 0, -1);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationHighFaction)));
}

#[tokio::test]
async fn push_quest_to_party_positive_prev_missing_rewarded_emits_prerequisite_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 259);
    let shared_quest_id = 7132;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, 9001);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7132".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}

#[tokio::test]
async fn push_quest_to_party_positive_prev_rewarded_passes_to_unrepresented_boundary_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 260);
    let shared_quest_id = 7133;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, 9002);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, 9002);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_negative_prev_missing_active_incomplete_emits_prerequisite_pair_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 261);
    let shared_quest_id = 7134;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9003);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7134".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_negative_prev_active_incomplete_passes_to_unrepresented_boundary_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 262);
    let shared_quest_id = 7135;
    let quest_store = store_with_sharable_quest_previous(shared_quest_id, -9004);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest(&mut receiver_session, 9004);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_previous_missing_rewarded_emits_prerequisite_pair_like_cpp()
{
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 470);
    let shared_quest_id = 7607;
    let prev_id = 9607;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = 0;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, previous_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7607".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_previous_rewarded_nonnegative_group_passes_to_unrepresented_boundary_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 471);
    let shared_quest_id = 7608;
    let prev_id = 9608;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = 0;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, previous_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, prev_id);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_previous_negative_exclusive_group_requires_all_other_members_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 472);
    let shared_quest_id = 7609;
    let prev_id = 9609;
    let sibling_id = 9610;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = -90;
    let mut sibling_quest = quest_template(sibling_id);
    sibling_quest.exclusive_group = -90;
    let quest_store =
        QuestStore::from_quests_like_cpp([shared_quest, previous_quest, sibling_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, prev_id);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7609".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));

    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut previous_quest = quest_template(prev_id);
    previous_quest.next_quest_id = shared_quest_id;
    previous_quest.exclusive_group = -90;
    let mut sibling_quest = quest_template(sibling_id);
    sibling_quest.exclusive_group = -90;
    let quest_store =
        QuestStore::from_quests_like_cpp([shared_quest, previous_quest, sibling_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_rewarded_quest(&mut receiver_session, prev_id);
    add_rewarded_quest(&mut receiver_session, sibling_id);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_dependent_breadcrumb_active_status_emits_prerequisite_and_absent_passes_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 473);
    let shared_quest_id = 7610;
    let breadcrumb_id = 9611;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut breadcrumb_quest = quest_template(breadcrumb_id);
    breadcrumb_quest.breadcrumb_for_quest_id = shared_quest_id as i32;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, breadcrumb_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest_in_slot_with_status(
        &mut receiver_session,
        breadcrumb_id,
        2,
        QUEST_STATUS_FAILED_LIKE_CPP,
    );
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7610".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));

    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    let mut breadcrumb_quest = quest_template(breadcrumb_id);
    breadcrumb_quest.breadcrumb_for_quest_id = shared_quest_id as i32;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, breadcrumb_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_success_command_queued_like_cpp(
        &sender_rx,
        &receiver_rx,
        &mut receiver_session,
        receiver_guid,
        sender_guid,
        shared_quest_id,
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_breadcrumb_for_quest_remains_unrepresented_without_prerequisite_pair_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 474);
    let shared_quest_id = 7611;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    shared_quest.breadcrumb_for_quest_id = 9991;
    let target_quest = quest_template(9991);
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, target_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert!(sender_rx.try_recv().is_err());
    assert!(receiver_rx.try_recv().is_err());
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ))
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_reputation_precedes_previous_prerequisite_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 263);
    let shared_quest_id = 7136;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.required_min_rep_faction = 72;
    quest.required_min_rep_value = 100;
    quest.prev_quest_id = 9005;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
        receiver_guid,
        72,
        99,
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP,
            "Quest 7136".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite)));
}

#[tokio::test]
async fn push_quest_to_party_class_precedes_reputation_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 258);
    let shared_quest_id = 7131;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.allowable_classes = 1 << (2 - 1);
    quest.required_min_rep_faction = 72;
    quest.required_min_rep_value = 100;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    receiver_session.sync_player_registry_state_like_cpp();
    set_canonical_party_reputation_like_cpp(
        receiver_session
            .canonical_map_manager
            .as_ref()
            .expect("canonical map manager"),
        receiver_guid,
        72,
        -42000,
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_CLASS_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP,
            "Quest 7131".to_string()
        )
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestClassWrongClass)));
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestReputationLowFaction)));
}

#[tokio::test]
async fn push_quest_to_party_daily_precedes_low_level_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 251);
    let shared_quest_id = 7124;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP | QUEST_FLAGS_DAILY_LIKE_CPP;
    quest.min_level = 80;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_level_like_cpp(1);
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7124".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestMinLevelLowLevel
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_receiver_level_snapshot_syncs_from_world_session_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 252);
    let shared_quest_id = 7125;
    let quest_store = store_with_sharable_quest_levels(shared_quest_id, 20, 0);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    assert_eq!(
        player_registry
            .group_presence(receiver_guid)
            .map(|presence| presence.level),
        Some(80)
    );
    receiver_session.set_player_level_like_cpp(19);
    receiver_session.sync_player_registry_state_like_cpp();
    assert_eq!(
        player_registry
            .group_presence(receiver_guid)
            .map(|presence| presence.level),
        Some(19)
    );

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7125".to_string()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_log_full_precedes_daily_completed_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 247);
    let shared_quest_id = 7120;
    let quest_store = store_with_daily_sharable_quests(&[shared_quest_id]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    for slot in 0..MAX_QUEST_LOG_SIZE_LIKE_CPP {
        add_active_quest_in_slot(&mut receiver_session, 8100 + u32::from(slot), slot);
    }
    receiver_session.set_represented_daily_quest_completed_like_cpp_for_test(shared_quest_id, true);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP,
            "Quest 7120".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverLogFull
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestDayAlreadyDone
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_low_receiver_expansion_emits_expansion_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 248);
    let shared_quest_id = 7121;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.expansion = 2;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.unregister_from_player_registry();
    receiver_session.expansion = 1;
    receiver_session.register_in_player_registry();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_EXPANSION_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP,
            "Quest 7121".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_success_prompts_receiver_details_and_sets_pending_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 249);
    let shared_quest_id = 7122;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.expansion = 2;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
            String::new()
        )
    );
    assert!(receiver_rx.try_recv().is_err());
    let commands = receiver_session.drain_session_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
            assert_eq!(command.sender_guid, sender_guid);
            assert_eq!(command.quest.id, shared_quest_id);
        }
        other => panic!("unexpected session command: {other:?}"),
    }
    receiver_session
        .session_command_tx()
        .try_send(commands.into_iter().next().expect("command"))
        .expect("requeue command for processing");
    receiver_session
        .process_represented_session_commands_like_cpp()
        .await;
    assert_eq!(
        receiver_session.represented_pending_quest_sharing_like_cpp(),
        Some(crate::session::RepresentedPendingQuestSharingLikeCpp {
            sender_guid,
            quest_id: shared_quest_id,
        })
    );
    recv_quest_giver_quest_details_contains_quest_id(&receiver_rx, shared_quest_id);
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
                )
                && !outcome.receiver_fanout_unrepresented)
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
                    | RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_repeatable_turn_in_success_prompts_request_items_without_pending_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 611);
    let shared_quest_id = 76110;
    let mut quest = quest_template(shared_quest_id);
    quest.quest_type = 0;
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.special_flags |= 0x0000_0001;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id: shared_quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 49211,
        amount: 3,
        flags: 0xA5,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let quest_for_assertion = quest.clone();
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_SUCCESS_LIKE_CPP,
            String::new()
        )
    );
    assert!(receiver_rx.try_recv().is_err());
    let commands = receiver_session.drain_session_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(command) => {
            assert_eq!(command.sender_guid, sender_guid);
            assert_eq!(command.quest.id, shared_quest_id);
        }
        other => panic!("unexpected session command: {other:?}"),
    }
    receiver_session
        .session_command_tx()
        .try_send(commands.into_iter().next().expect("command"))
        .expect("requeue command for processing");
    receiver_session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        receiver_session.represented_pending_quest_sharing_like_cpp(),
        None
    );
    let (collect, auto_launched) =
        recv_quest_giver_request_items_like_cpp(&receiver_rx, shared_quest_id);
    assert_eq!(collect, vec![(49211, 3, 0xA5)]);
    assert!(auto_launched);
    assert!(
        !receiver_session
            .can_complete_repeatable_quest_represented_bounded_like_cpp(&quest_for_assertion)
    );
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted
            )
            && !outcome.receiver_fanout_unrepresented
    ));
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_repeatable_turn_in_command_queue_failure_sends_no_success_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 612);
    let shared_quest_id = 76120;
    let mut quest = quest_template(shared_quest_id);
    quest.quest_type = 0;
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.special_flags |= 0x0000_0001;
    quest.objectives.push(QuestObjective {
        id: 1,
        quest_id: shared_quest_id,
        obj_type: 1,
        order: 0,
        storage_index: 0,
        object_id: 49212,
        amount: 3,
        flags: 0xA5,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let quest_for_dummy_commands = quest.clone();
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.sync_player_registry_state_like_cpp();

    for _ in 0..256 {
        receiver_session
            .session_command_tx()
            .try_send(SessionCommand::SetQuestSharingInfoAndSendDetails(
                SetQuestSharingInfoAndSendDetailsCommand {
                    sender_guid,
                    quest: quest_for_dummy_commands.clone(),
                },
            ))
            .expect("fill receiver command queue fixture");
    }

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert!(sender_rx.try_recv().is_err());
    assert!(receiver_rx.try_recv().is_err());
    assert_eq!(receiver_session.drain_session_commands().len(), 256);
    assert!(session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPromptCommandFailed
            )
            && outcome.receiver_fanout_unrepresented
    ));
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(sender_guid)
                && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverEligibilityUnrepresented
            ) && outcome.receiver_fanout_unrepresented)
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(
        |outcome| outcome.target_guid == Some(receiver_guid)
            && matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverRepeatableTurnInRequestItemsPrompted
            )
    ));
}

#[tokio::test]
async fn push_quest_to_party_receiver_unknown_status_after_expansion_emits_invalid_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 609);
    let shared_quest_id = 76090;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.expansion = 2;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest_in_slot_with_status(&mut receiver_session, shared_quest_id, 2, 0xFE);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_INVALID_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
            "Quest 76090".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid
                ))
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}

#[tokio::test]
async fn push_quest_to_party_receiver_positive_exclusive_group_active_peer_emits_invalid_pair_like_cpp()
 {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 610);
    let shared_quest_id = 76091;
    let peer_quest_id = 76092;
    let mut shared_quest = quest_template(shared_quest_id);
    shared_quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    shared_quest.exclusive_group = 609;
    let mut peer_quest = quest_template(peer_quest_id);
    peer_quest.exclusive_group = 609;
    let quest_store = QuestStore::from_quests_like_cpp([shared_quest, peer_quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    add_active_quest(&mut receiver_session, peer_quest_id);
    receiver_session.sync_player_registry_state_like_cpp();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_INVALID_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP,
            "Quest 76091".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| outcome.target_guid == Some(receiver_guid)
                && matches!(
                    outcome.reason,
                    RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverCanTakeQuestInvalid
                ))
    );
    assert!(!session.represented_push_quest_to_party_outcomes_like_cpp().iter().any(|outcome| outcome.target_guid == Some(receiver_guid) && matches!(outcome.reason, RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSuccessQuestDetailsPrompted)));
}

#[tokio::test]
async fn push_quest_to_party_prerequisite_precedes_expansion_gate_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 250);
    let shared_quest_id = 7123;
    let mut quest = quest_template(shared_quest_id);
    quest.flags |= QUEST_FLAGS_SHARABLE_LIKE_CPP;
    quest.prev_quest_id = 9001;
    quest.expansion = 2;
    let quest_store = QuestStore::from_quests_like_cpp([quest]);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, shared_quest_id);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.unregister_from_player_registry();
    receiver_session.expansion = 1;
    receiver_session.register_in_player_registry();

    run_push_quest_to_party(&mut session, shared_quest_id).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP,
            "Quest 7123".to_string()
        )
    );
    assert!(
        session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestPreviousQuestPrerequisite
            ))
    );
    assert!(
        !session
            .represented_push_quest_to_party_outcomes_like_cpp()
            .iter()
            .any(|outcome| matches!(
                outcome.reason,
                RepresentedPushQuestToPartyOutcomeReasonLikeCpp::ReceiverSatisfyQuestExpansionRequiredExpansion
            ))
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_busy_emits_sender_only_busy_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 45);
    let quest_store = store_with_sharable_quest(7113);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7113);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session
        .set_represented_pending_quest_sharing_like_cpp(ObjectGuid::create_player(1, 77), 9000);

    run_push_quest_to_party(&mut session, 7113).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_BUSY_LIKE_CPP,
            String::new()
        )
    );
    assert!(receiver_rx.try_recv().is_err());
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_dead_emits_dead_pair_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 46);
    let quest_store = store_with_sharable_quest(7114);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7114);
    let (_player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_alive_like_cpp(false);

    run_push_quest_to_party(&mut session, 7114).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_DEAD_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
            "Quest 7114".to_string()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_grouped_receiver_dead_observes_runtime_under_map_sync_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid().expect("test sender guid");
    let receiver_guid = ObjectGuid::create_player(1, 146);
    let quest_store = store_with_sharable_quest(7114);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7114);
    let (player_registry, mut receiver_session, receiver_rx) =
        install_represented_party(&mut session, sender_guid, receiver_guid);
    receiver_session.set_player_health_like_cpp(1_000, 1_000);
    let mut movement_info = wow_packet::packets::movement::MovementInfo::default();
    movement_info.position.z = -501.0;

    let event = receiver_session.handle_under_map_like_cpp(&movement_info);

    assert!(event.is_some());
    assert!(!receiver_session.player_is_alive_like_cpp());
    assert!(
        !player_registry
            .group_presence(receiver_guid)
            .expect("receiver registry snapshot")
            .is_alive
    );
    let health_update = receiver_rx.try_recv().expect("void health update");
    assert_eq!(
        u16::from_le_bytes([health_update[0], health_update[1]]),
        wow_constants::ServerOpcodes::HealthUpdate as u16
    );
    let damage_log = receiver_rx
        .try_recv()
        .expect("void environmental damage log");
    assert_eq!(
        u16::from_le_bytes([damage_log[0], damage_log[1]]),
        wow_constants::ServerOpcodes::EnvironmentalDamageLog as u16
    );

    run_push_quest_to_party(&mut session, 7114).await;

    assert_eq!(
        recv_push_quest_result_response(&sender_rx),
        (
            receiver_guid,
            QUEST_PUSH_REASON_DEAD_LIKE_CPP,
            String::new()
        )
    );
    assert_eq!(
        recv_push_quest_result_response_after_death_sync(&receiver_rx),
        (
            sender_guid,
            QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP,
            "Quest 7114".to_string()
        )
    );
}

#[tokio::test]
async fn push_quest_to_party_missing_group_registry_keeps_explicit_blocker_like_cpp() {
    let (mut session, sender_rx) = make_session();
    let sender_guid = session.player_guid();
    let quest_store = store_with_sharable_quest(7115);
    let quest_pool_store = QuestPoolStoreLikeCpp::from_rows_like_cpp(&quest_store, [], []);
    session.set_quest_store(Arc::new(quest_store));
    session.set_quest_pool_store(Arc::new(quest_pool_store));
    add_active_quest(&mut session, 7115);
    session.group_guid = Some(1234);

    run_push_quest_to_party(&mut session, 7115).await;

    assert_eq!(
        session.represented_push_quest_to_party_outcomes_like_cpp(),
        &[RepresentedPushQuestToPartyOutcomeLikeCpp {
            sender_guid,
            quest_id: 7115,
            target_guid: sender_guid,
            reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp::GroupRuntimeUnrepresented,
            quest_pool_active_check_unrepresented: false,
            group_runtime_unrepresented: true,
            receiver_fanout_unrepresented: true,
        }]
    );
    assert!(sender_rx.try_recv().is_err());
}

#[test]
fn push_quest_to_party_registration_and_dispatch_are_wired_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::PushQuestToParty)
        .expect("PushQuestToParty handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_push_quest_to_party");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains("session.handle_push_quest_to_party(pkt).await"),
        "the PushQuestToParty registration must carry the call itself"
    );
}

#[test]
fn quest_packet_registration_and_dispatch_are_wired_like_cpp() {
    let cases = [
        (
            ClientOpcodes::QuestGiverQueryQuest,
            "handle_quest_giver_query_quest",
            "session.handle_quest_giver_query_quest(pkt).await",
        ),
        (
            ClientOpcodes::QuestGiverAcceptQuest,
            "handle_quest_giver_accept_quest",
            "session.handle_quest_giver_accept_quest(pkt).await",
        ),
        (
            ClientOpcodes::QuestGiverRequestReward,
            "handle_quest_giver_request_reward",
            "session.handle_quest_giver_request_reward(pkt).await",
        ),
        (
            ClientOpcodes::QuestGiverCompleteQuest,
            "handle_quest_giver_complete_quest",
            "session.handle_quest_giver_complete_quest(pkt).await",
        ),
        (
            ClientOpcodes::QuestGiverChooseReward,
            "handle_quest_giver_choose_reward",
            "session.handle_quest_giver_choose_reward(pkt).await",
        ),
        (
            ClientOpcodes::QueryQuestInfo,
            "handle_query_quest_info",
            "session.handle_query_quest_info(pkt).await",
        ),
    ];
    for (opcode, handler_name, call) in cases {
        let entry = inventory::iter::<PacketHandlerEntry>
            .into_iter()
            .find(|entry| entry.opcode == opcode)
            .unwrap_or_else(|| panic!("{opcode:?} handler registration"));

        assert_eq!(entry.status, SessionStatus::LoggedIn, "{opcode:?}");
        assert_eq!(entry.processing, PacketProcessing::Inplace, "{opcode:?}");
        assert_eq!(entry.handler_name, handler_name, "{opcode:?}");
        assert!(
            QUEST_HANDLER_REGISTRATIONS.contains(call),
            "{opcode:?} must reach its handler from its own registration"
        );
    }
}

#[tokio::test]
async fn request_world_quest_update_empty_payload_sends_empty_response_like_cpp() {
    let (mut session, send_rx) = make_session();

    run_request_world_quest_update(&mut session).await;

    assert_eq!(recv_world_quest_update_count(&send_rx), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn request_world_quest_update_with_payload_ignores_bytes_and_sends_empty_response_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(1);

    session.handle_request_world_quest_update(pkt).await;

    assert_eq!(recv_world_quest_update_count(&send_rx), 0);
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn request_world_quest_update_inventory_entry_matches_cpp_status_and_processing() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::RequestWorldQuestUpdate)
        .expect("RequestWorldQuestUpdate handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_request_world_quest_update");
}

#[tokio::test]
async fn quest_giver_status_query_missing_noncanonical_guid_sends_no_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[1001])));
    attach_map_manager(&mut session, wow_map::MapManager::default());

    run_status_query(&mut session, creature_guid(9001, 1)).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_status_query_unsupported_player_or_item_guid_sends_no_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[1001])));
    attach_map_manager(&mut session, wow_map::MapManager::default());

    run_status_query(&mut session, ObjectGuid::create_player(1, 99)).await;
    run_status_query(&mut session, ObjectGuid::create_item(1, 100)).await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_status_query_canonical_creature_starter_sends_available_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1001]);
    store.starter_quests.entry(9001).or_default().push(1001);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9001, 1);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9001);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::TRIVIAL));
}

#[tokio::test]
async fn quest_giver_status_query_canonical_creature_starter_sends_quest_when_not_trivial_like_cpp()
{
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(1006);
    quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9006).or_default().push(1006);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9006, 6);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9006);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}

#[tokio::test]
async fn quest_giver_status_query_uses_configured_low_level_hide_diff_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_low_level_hide_diff_like_cpp(5);
    let mut quest = quest_template(1013);
    quest.quest_level = 75;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9013).or_default().push(1013);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9013, 13);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9013);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}

#[tokio::test]
async fn quest_giver_status_query_canonical_creature_starter_sends_future_when_visible_but_low_level_like_cpp()
 {
    let (mut session, send_rx) = make_session();
    let mut quest = quest_template(1007);
    quest.quest_level = 85;
    quest.min_level = 85;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9007).or_default().push(1007);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9007, 7);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9007);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::FUTURE));
}

#[tokio::test]
async fn quest_giver_status_query_uses_configured_high_level_hide_diff_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_high_level_hide_diff_like_cpp(2);
    let mut quest = quest_template(1014);
    quest.quest_level = 85;
    quest.min_level = 83;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9014).or_default().push(1014);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9014, 14);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9014);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}

#[tokio::test]
async fn quest_giver_status_query_starter_respects_quest_available_conditions_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1008;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9008).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 90,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9008, 8);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9008);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}

#[tokio::test]
async fn quest_giver_status_query_starter_allows_passing_quest_available_conditions_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1009;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9009).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 80,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9009, 9);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9009);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}

#[tokio::test]
async fn quest_giver_status_query_starter_allows_objective_progress_condition_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_quest_id = 1015;
    let starter_quest_id = 1016;
    let active_quest = quest_template_with_objective_count(active_quest_id, 1);
    let objective_id = active_quest.objectives[0].id;
    let mut starter_quest = quest_template(starter_quest_id);
    starter_quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([active_quest, starter_quest]);
    store
        .starter_quests
        .entry(9016)
        .or_default()
        .push(starter_quest_id);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        active_quest_id,
        PlayerQuestStatus {
            quest_id: active_quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![2],
            slot: 0,
        },
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: starter_quest_id as i32,
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: objective_id,
            condition_value3: 2,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9016, 16);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9016);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::QUEST));
}

#[tokio::test]
async fn quest_giver_status_query_starter_rejects_objective_progress_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_quest_id = 1017;
    let starter_quest_id = 1018;
    let active_quest = quest_template_with_objective_count(active_quest_id, 1);
    let objective_id = active_quest.objectives[0].id;
    let mut starter_quest = quest_template(starter_quest_id);
    starter_quest.quest_level = 80;
    let mut store = QuestStore::from_quests_like_cpp([active_quest, starter_quest]);
    store
        .starter_quests
        .entry(9018)
        .or_default()
        .push(starter_quest_id);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        active_quest_id,
        PlayerQuestStatus {
            quest_id: active_quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![2],
            slot: 0,
        },
    );
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: starter_quest_id as i32,
            condition_type: ConditionType::QuestObjectiveProgress,
            condition_value1: objective_id,
            condition_value3: 3,
            ..Condition::default()
        }]),
    ));
    let guid = creature_guid(9018, 18);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9018);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}

#[tokio::test]
async fn quest_giver_status_query_canonical_creature_completed_ender_sends_can_reward_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1002]);
    store.ender_quests.entry(9002).or_default().push(1002);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        1002,
        PlayerQuestStatus {
            quest_id: 1002,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = creature_guid(9002, 2);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9002);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::CAN_REWARD)
    );
}

#[tokio::test]
async fn quest_giver_status_query_important_starter_uses_quest_info_modifiers_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1010;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 80;
    quest.quest_info_id = 710;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9010).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
        quest_info_entry_like_cpp(710, 2, 0x400),
    ])));
    let guid = creature_guid(9010, 10);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9010);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::IMPORTANT_QUEST)
    );
}

#[tokio::test]
async fn quest_giver_status_query_important_low_level_uses_future_important_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1011;
    let mut quest = quest_template(quest_id);
    quest.quest_level = 85;
    quest.min_level = 85;
    quest.quest_info_id = 711;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.starter_quests.entry(9011).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
        quest_info_entry_like_cpp(711, 2, 0x400),
    ])));
    let guid = creature_guid(9011, 11);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9011);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::FUTURE_IMPORTANT_QUEST)
    );
}

#[tokio::test]
async fn quest_giver_status_query_covenant_completed_ender_uses_quest_info_tag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let quest_id = 1012;
    let mut quest = quest_template(quest_id);
    quest.quest_info_id = 712;
    let mut store = QuestStore::from_quests_like_cpp([quest]);
    store.ender_quests.entry(9012).or_default().push(quest_id);
    session.set_quest_store(Arc::new(store));
    session.set_quest_info_store(Arc::new(QuestInfoStore::from_entries([
        quest_info_entry_like_cpp(712, 15, 0),
    ])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = creature_guid(9012, 12);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9012);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (
            guid,
            quest_giver_status::COVENANT_CALLING_REWARD_COMPLETE_POI
        )
    );
}

#[tokio::test]
async fn quest_giver_status_query_canonical_gameobject_starter_uses_go_relation_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1003]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9103, 1003));
    session.set_quest_store(Arc::new(store));
    let guid = gameobject_guid(9103, 3);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9103);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::TRIVIAL));
}

#[tokio::test]
async fn quest_giver_status_query_canonical_gameobject_completed_ender_uses_go_relation_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1004]);
    assert!(store.insert_gameobject_ender_relation_like_cpp(9104, 1004));
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        1004,
        PlayerQuestStatus {
            quest_id: 1004,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = gameobject_guid(9104, 4);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9104);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(
        recv_status(&send_rx),
        (guid, quest_giver_status::CAN_REWARD)
    );
}

#[tokio::test]
async fn quest_giver_status_query_gameobject_ignores_creature_relation_for_same_entry_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[1005]);
    store.starter_quests.entry(9105).or_default().push(1005);
    store.ender_quests.entry(9105).or_default().push(1005);
    session.set_quest_store(Arc::new(store));
    session.player_quests.insert(
        1005,
        PlayerQuestStatus {
            quest_id: 1005,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    let guid = gameobject_guid(9105, 5);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9105);
    attach_map_manager(&mut session, manager);

    run_status_query(&mut session, guid).await;

    assert_eq!(recv_status(&send_rx), (guid, quest_giver_status::NONE));
}

#[tokio::test]
async fn quest_giver_status_multiple_empty_visible_set_sends_zero_count_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[2001])));
    attach_map_manager(&mut session, wow_map::MapManager::default());

    session.handle_quest_giver_status_multiple_query().await;

    assert!(recv_status_multiple(&send_rx).is_empty());
}

#[tokio::test]
async fn quest_giver_status_multiple_visible_canonical_creature_starter_sends_available_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[2002]);
    store.starter_quests.entry(9202).or_default().push(2002);
    session.set_quest_store(Arc::new(store));
    let guid = creature_guid(9202, 202);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, guid, 9202);
    attach_map_manager(&mut session, manager);
    mark_visible(&mut session, guid);

    session.handle_quest_giver_status_multiple_query().await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}

#[tokio::test]
async fn quest_giver_status_multiple_visible_gameobject_starter_uses_go_relation_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[2003]);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9203, 2003));
    store.starter_quests.entry(9203).or_default().push(2999);
    session.set_quest_store(Arc::new(store));
    let guid = gameobject_guid(9203, 203);
    let mut manager = wow_map::MapManager::default();
    insert_gameobject(&mut manager, guid, 9203);
    attach_map_manager(&mut session, manager);
    mark_visible_gameobject_questgiver(&mut session, guid);

    session.handle_quest_giver_status_multiple_query().await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(guid, quest_giver_status::TRIVIAL)]
    );
}

#[tokio::test]
async fn quest_giver_status_multiple_skips_missing_player_item_and_non_questgiver_go_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut store = store_with_quests(&[2004]);
    store.starter_quests.entry(9204).or_default().push(2004);
    assert!(store.insert_gameobject_starter_relation_like_cpp(9204, 2004));
    session.set_quest_store(Arc::new(store));
    let accepted_guid = creature_guid(9204, 204);
    let missing_guid = creature_guid(9204, 205);
    let player_guid = ObjectGuid::create_player(1, 204);
    let item_guid = ObjectGuid::create_item(1, 204);
    let non_questgiver_go = gameobject_guid(9204, 206);
    let mut manager = wow_map::MapManager::default();
    insert_creature(&mut manager, accepted_guid, 9204);
    insert_gameobject(&mut manager, non_questgiver_go, 9204);
    attach_map_manager(&mut session, manager);
    for guid in [
        accepted_guid,
        missing_guid,
        player_guid,
        item_guid,
        non_questgiver_go,
    ] {
        mark_visible(&mut session, guid);
    }
    let mut state = crate::session::RepresentedGameObjectUseState::default();
    state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
    session
        .represented_gameobject_use_states
        .insert(non_questgiver_go, state);

    session.handle_quest_giver_status_multiple_query().await;

    assert_eq!(
        recv_status_multiple(&send_rx),
        vec![(accepted_guid, quest_giver_status::TRIVIAL)]
    );
}

#[tokio::test]
async fn quest_giver_close_active_existing_template_records_acknowledge_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5901])));
    add_active_quest(&mut session, 5901);

    run_close_quest(&mut session, 5901).await;

    assert_eq!(
        session.represented_auto_accept_acknowledged_quests_like_cpp,
        vec![5901]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_close_missing_active_quest_records_no_acknowledge_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5902])));

    run_close_quest(&mut session, 5902).await;

    assert!(
        session
            .represented_auto_accept_acknowledged_quests_like_cpp
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_close_missing_template_records_no_acknowledge_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5904])));
    add_active_quest(&mut session, 5903);

    run_close_quest(&mut session, 5903).await;

    assert!(
        session
            .represented_auto_accept_acknowledged_quests_like_cpp
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_giver_close_short_packet_records_no_acknowledge_and_sends_no_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_quest_store(Arc::new(store_with_quests(&[5905])));
    add_active_quest(&mut session, 5905);

    session
        .handle_quest_giver_close_quest(WorldPacket::from_bytes(&[0x05, 0x17]))
        .await;

    assert!(
        session
            .represented_auto_accept_acknowledged_quests_like_cpp
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_giver_close_inventory_registration_matches_dispatch_contract_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QuestGiverCloseQuest)
        .expect("QuestGiverCloseQuest handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_quest_giver_close_quest");
}

#[tokio::test]
async fn quest_log_remove_short_packet_does_not_remove_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5911, 0);

    session
        .handle_quest_log_remove_quest(WorldPacket::from_bytes(&[]))
        .await;

    assert!(session.player_quests.contains_key(&5911));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), Some(5911));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_slot_outside_max_does_not_remove_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5912, 0);

    run_remove_quest_slot(&mut session, 25).await;

    assert!(session.player_quests.contains_key(&5912));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(0), Some(5912));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_valid_slot_removes_only_that_slot_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 880_001, 7);
    add_active_quest_in_slot(&mut session, 17, 3);

    run_remove_quest_slot(&mut session, 7).await;

    assert!(!session.player_quests.contains_key(&880_001));
    assert!(session.player_quests.contains_key(&17));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(7), None);
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(3), Some(17));

    let update = send_rx
        .try_recv()
        .expect("C++ SetQuestSlot(slot, 0) must become an immediate player UpdateObject");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&update).server_opcode(),
        Some(wow_constants::ServerOpcodes::UpdateObject)
    );
    assert!(
        !update
            .windows(std::mem::size_of::<u32>())
            .any(|window| window == 880_001_u32.to_le_bytes()),
        "quest-log abandon UpdateObject should clear the removed QuestID"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_empty_valid_slot_does_not_remove_other_quest_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5914, 4);

    run_remove_quest_slot(&mut session, 3).await;

    assert!(session.player_quests.contains_key(&5914));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(4), Some(5914));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(3), None);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn quest_log_remove_duplicate_slot_fails_closed_and_removes_none_like_cpp() {
    let (mut session, send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    run_remove_quest_slot(&mut session, 2).await;

    assert!(session.player_quests.contains_key(&5915));
    assert!(session.player_quests.contains_key(&5916));
    assert_eq!(session.get_quest_slot_quest_id_like_cpp(2), None);
    assert_eq!(session.first_free_quest_slot_like_cpp(), Some(0));
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn quest_log_create_entries_preserve_explicit_slot_holes_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5916, 9);
    add_active_quest_in_slot(&mut session, 5915, 2);

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[0], (0, 0, 0, [0; 24]));
    assert_eq!(entries[2].0, 5915);
    assert_eq!(entries[9].0, 5916);
}

#[test]
fn quest_log_create_entries_preserve_end_time_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5917, 4);
    session
        .player_quests
        .get_mut(&5917)
        .expect("active quest")
        .end_time_secs = 123_456;

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries[4].2, 123_456);
}

#[test]
fn save_to_db_quest_status_list_includes_active_quests_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot_with_status(&mut session, 5920, 2, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    add_active_quest_in_slot_with_status(&mut session, 5919, 3, QUEST_STATUS_COMPLETE_LIKE_CPP);

    assert_eq!(
        session.represented_quest_statuses_for_save_like_cpp(),
        vec![
            (5919, QUEST_STATUS_COMPLETE_LIKE_CPP),
            (5920, QUEST_STATUS_INCOMPLETE_LIKE_CPP)
        ],
        "C++ Player::SaveToDB reaches _SaveQuestStatus for represented active quests"
    );
}

#[test]
fn quest_status_projection_persists_only_nonzero_storage_objectives_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5925;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 1,
        storage_index: 1,
        object_id: 45,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 2,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL,
        order: 2,
        storage_index: -1,
        object_id: 0,
        amount: 20,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: true,
            accept_time_secs: 12,
            end_time_secs: 34,
            objective_counts: vec![3, 0, 9],
            slot: 0,
        },
    );

    let projected = session.represented_quest_status_persistence_like_cpp(
        session.player_quests.get(&quest_id).unwrap(),
    );
    assert_eq!(projected.quest_id, quest_id);
    assert_eq!(projected.status, QUEST_STATUS_INCOMPLETE_LIKE_CPP);
    assert!(projected.explored);
    assert_eq!(projected.accept_time_secs, 12);
    assert_eq!(projected.end_time_secs, 34);
    assert_eq!(
        projected.objectives,
        vec![wow_persistence::QuestObjectiveCountPersistenceLikeCpp {
            objective_index: 0,
            count: 3,
        }]
    );
}

#[tokio::test]
async fn quest_status_save_uses_the_sqlx_free_player_quest_port_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5928;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    session.player_quests.insert(
        quest_id,
        PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_COMPLETE_LIKE_CPP,
            explored: true,
            accept_time_secs: 12,
            end_time_secs: 34,
            objective_counts: vec![5],
            slot: 0,
        },
    );
    let fixture = PlayerQuestPersistencePortFixtureLikeCpp::default();
    let requests = Arc::clone(&fixture.status_requests);
    session.set_player_quest_persistence_port_like_cpp(Arc::new(fixture));

    session
        .save_quest_to_db(quest_id, QUEST_STATUS_COMPLETE_LIKE_CPP)
        .await;

    assert_eq!(
        requests.lock().unwrap().as_slice(),
        &[PlayerQuestStatusPersistenceRequestLikeCpp::Save {
            owner_guid: 42,
            status: wow_persistence::QuestStatusPersistenceLikeCpp {
                quest_id,
                status: QUEST_STATUS_COMPLETE_LIKE_CPP,
                explored: true,
                accept_time_secs: 12,
                end_time_secs: 34,
                objectives: vec![wow_persistence::QuestObjectiveCountPersistenceLikeCpp {
                    objective_index: 0,
                    count: 5,
                }],
            },
        }]
    );
}

#[tokio::test]
async fn quest_load_keeps_the_seven_stage_order_behind_the_typed_port_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let active_id = 5930;
    let rewarded_id = 5931;
    let daily_id = 5932;
    let weekly_id = 5933;
    let monthly_id = 5934;
    let mut active = quest_template(active_id);
    active.objectives.push(QuestObjective {
        id: active_id * 10,
        quest_id: active_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    let rewarded = quest_template(rewarded_id);
    let mut daily = quest_template(daily_id);
    daily.flags |= QUEST_FLAGS_DAILY_LIKE_CPP;
    let mut weekly = quest_template(weekly_id);
    weekly.flags |= QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let mut monthly = quest_template(monthly_id);
    monthly.special_flags |= QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([
        active, rewarded, daily, weekly, monthly,
    ])));

    let fixture = PlayerQuestPersistencePortFixtureLikeCpp {
        active: vec![PlayerQuestActivePersistenceRowLikeCpp {
            quest_id: Some(active_id),
            status: Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP),
            explored: Some(1),
            accept_time_secs: Some(12),
            end_time_secs: Some(34),
        }],
        objectives: vec![PlayerQuestObjectivePersistenceRowLikeCpp {
            quest_id: Some(active_id),
            storage_index: Some(0),
            count: Some(3),
        }],
        rewarded: vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(rewarded_id),
        }],
        daily: vec![PlayerQuestDailyPersistenceRowLikeCpp {
            quest_id: Some(daily_id),
            completed_time: Some(45),
        }],
        weekly: vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(weekly_id),
        }],
        monthly: vec![PlayerQuestIdPersistenceRowLikeCpp {
            quest_id: Some(monthly_id),
        }],
        ..Default::default()
    };
    let stages = Arc::clone(&fixture.stages);
    session.set_player_quest_persistence_port_like_cpp(Arc::new(fixture));

    session.load_player_quests().await;

    assert_eq!(
        stages.lock().unwrap().as_slice(),
        &[
            PlayerQuestLoadStageFixtureLikeCpp::Active,
            PlayerQuestLoadStageFixtureLikeCpp::Objectives,
            PlayerQuestLoadStageFixtureLikeCpp::Rewarded,
            PlayerQuestLoadStageFixtureLikeCpp::Daily,
            PlayerQuestLoadStageFixtureLikeCpp::Weekly,
            PlayerQuestLoadStageFixtureLikeCpp::Monthly,
            PlayerQuestLoadStageFixtureLikeCpp::Seasonal,
        ]
    );
    assert_eq!(session.player_quests[&active_id].objective_counts, vec![3]);
    assert!(session.rewarded_quests.contains(&rewarded_id));
    assert!(session.daily_quests_completed_like_cpp.contains(&daily_id));
    assert!(
        session
            .weekly_quests_completed_like_cpp
            .contains(&weekly_id)
    );
    assert!(
        session
            .monthly_quests_completed_like_cpp
            .contains(&monthly_id)
    );
}

#[test]
fn save_to_db_quest_status_list_skips_rewarded_non_repeatable_active_duplicate_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let active_rewarded_quest_id = 5921;
    let active_quest_id = 5922;
    let quest_store = QuestStore::from_quests_like_cpp([
        quest_template(active_rewarded_quest_id),
        quest_template(active_quest_id),
    ]);
    session.quest_store = Some(Arc::new(quest_store));
    add_active_quest_in_slot_with_status(
        &mut session,
        active_rewarded_quest_id,
        0,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
    );
    add_active_quest_in_slot_with_status(
        &mut session,
        active_quest_id,
        1,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    session.rewarded_quests.insert(active_rewarded_quest_id);

    assert_eq!(
        session.represented_quest_statuses_for_save_like_cpp(),
        vec![(active_quest_id, QUEST_STATUS_INCOMPLETE_LIKE_CPP)],
        "C++ reward save deletes active quest status and stores rewarded separately"
    );
}

#[test]
fn quest_load_removes_active_rewarded_duplicate_and_compacts_slots_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let duplicate_quest_id = 5923;
    let active_quest_id = 5924;
    let quest_store = QuestStore::from_quests_like_cpp([
        quest_template(duplicate_quest_id),
        quest_template(active_quest_id),
    ]);
    session.quest_store = Some(Arc::new(quest_store));
    add_active_quest_in_slot_with_status(
        &mut session,
        duplicate_quest_id,
        0,
        QUEST_STATUS_COMPLETE_LIKE_CPP,
    );
    add_active_quest_in_slot_with_status(
        &mut session,
        active_quest_id,
        3,
        QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    );
    session.rewarded_quests.insert(duplicate_quest_id);

    assert_eq!(
        session.remove_represented_active_rewarded_duplicates_like_cpp(),
        vec![duplicate_quest_id]
    );
    assert!(!session.player_quests.contains_key(&duplicate_quest_id));
    assert_eq!(
        session
            .player_quests
            .get(&active_quest_id)
            .map(|status| status.slot),
        Some(0)
    );
}

#[test]
fn save_to_db_quest_status_list_is_empty_without_active_quests_like_cpp() {
    let (session, _send_rx) = make_session();

    assert!(
        session
            .represented_quest_statuses_for_save_like_cpp()
            .is_empty()
    );
}

#[test]
fn quest_log_create_entries_store_flag_objectives_in_state_flags_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let quest_id = 5918;
    let mut quest = quest_template(quest_id);
    quest.objectives.push(QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL,
        order: 0,
        storage_index: 0,
        object_id: 44,
        amount: 5,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    quest.objectives.push(QuestObjective {
        id: quest_id * 10 + 1,
        quest_id,
        obj_type: 10,
        order: 1,
        storage_index: 1,
        object_id: 45,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
    add_active_quest_in_slot(&mut session, quest_id, 3);
    session
        .player_quests
        .get_mut(&quest_id)
        .expect("active quest")
        .objective_counts = vec![3, 1];

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries[3].1, 256 << 1);
    assert_eq!(entries[3].3[0], 3);
    assert_eq!(
        entries[3].3[1], 0,
        "C++ stores flag objectives in QuestLog.StateFlags, not ObjectiveProgress"
    );
}

#[test]
fn quest_log_create_entries_preserve_failed_state_flag_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot_with_status(&mut session, 5919, 5, QUEST_STATUS_FAILED_LIKE_CPP);

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries[5].1, 2);
}

#[test]
fn quest_log_create_entries_duplicate_slot_is_empty_fail_closed_like_cpp() {
    let (mut session, _send_rx) = make_session();
    add_active_quest_in_slot(&mut session, 5915, 2);
    add_active_quest_in_slot(&mut session, 5916, 2);

    let entries = session.quest_log_create_entries_like_cpp();

    assert_eq!(entries.len(), MAX_QUEST_LOG_SIZE_LIKE_CPP as usize);
    assert_eq!(entries[2], (0, 0, 0, [0; 24]));
    assert!(session.player_quests.contains_key(&5915));
    assert!(session.player_quests.contains_key(&5916));
}

#[test]
fn quest_log_remove_inventory_registration_and_dispatcher_contract_like_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QuestLogRemoveQuest)
        .expect("QuestLogRemoveQuest handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_quest_log_remove_quest");
    assert!(
        QUEST_HANDLER_REGISTRATIONS.contains("session.handle_quest_log_remove_quest(pkt).await"),
        "the QuestLogRemoveQuest registration must carry the call itself"
    );
}

#[test]
fn can_take_quest_blocks_when_quest_available_condition_not_met_like_cpp() {
    // NEGATIVA: nivel requerido 90, jugador nivel 80 → condición no se cumple.
    let (mut session, _send_rx) = make_session();
    let quest_id = 7570u32;
    let quest = quest_template(quest_id);
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id as i32,
            condition_type: ConditionType::Level,
            condition_value1: 90,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));

    // Sin la condición la quest sería aceptable (nivel 1, min_level 1, raza/clase sin filtro).
    // Con la condición de nivel 90, el jugador (80) no la cumple.
    assert!(!session.can_take_quest(&quest));

    // POSITIVA: nivel requerido 80 (alcanzable para el jugador en nivel 80).
    let quest_id2 = 7571u32;
    let quest2 = quest_template(quest_id2);
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session.set_quest_store(Arc::new(store2));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::QuestAvailable,
            source_entry: quest_id2 as i32,
            condition_type: ConditionType::Level,
            condition_value1: 80,
            condition_value2: ComparisonType::HighEq as u32,
            ..Condition::default()
        }]),
    ));

    assert!(session.can_take_quest(&quest2));
}

#[test]
fn can_take_quest_blocks_when_session_expansion_below_required_like_cpp() {
    // NEGATIVA: expansión de sesión 1 < expansión requerida 2 → rechaza.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(9900u32);
    quest.expansion = 2;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.expansion = 1;
    assert!(!session.can_take_quest(&quest));

    // POSITIVA límite: expansión de sesión == expansión requerida → acepta.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(9901u32);
    quest2.expansion = 2;
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));
    session2.expansion = 2;
    assert!(session2.can_take_quest(&quest2));
}

// ── SatisfyQuestDay / Week / Month tests ────────────────────────────────
// C++ Player::CanTakeQuest (Player.cpp:14093-14102) gates on
// SatisfyQuestDay && SatisfyQuestWeek && SatisfyQuestMonth. A daily/DF/weekly/
// monthly quest already on cooldown must not be re-acceptable.

#[test]
fn can_take_quest_blocks_daily_already_completed_like_cpp() {
    // NEGATIVE: a daily quest already in DailyQuestsCompleted is blocked
    // (C++ SatisfyQuestDay, Player.cpp:15393-15407).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7600u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.daily_quests_completed_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_allows_daily_not_yet_completed_like_cpp() {
    // POSITIVE: a daily quest not yet completed today is acceptable.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7601u32);
    quest.flags = QUEST_FLAGS_DAILY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_df_quest_already_completed_like_cpp() {
    // NEGATIVE: a DF (dungeon-finder) quest already in DFQuests is blocked
    // (C++ SatisfyQuestDay DFQuest branch, Player.cpp:15393-15407).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7602u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.df_quests_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_weekly_already_completed_like_cpp() {
    // NEGATIVE: a weekly quest already in the weekly cooldown set is blocked
    // (C++ SatisfyQuestWeek, Player.cpp:15409-15418).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7603u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.weekly_quests_completed_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_allows_weekly_not_yet_completed_like_cpp() {
    // POSITIVE: a weekly quest not on cooldown is acceptable.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7604u32);
    quest.flags = QUEST_FLAGS_WEEKLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_blocks_monthly_already_completed_like_cpp() {
    // NEGATIVE: a monthly quest already in the monthly cooldown set is blocked
    // (C++ SatisfyQuestMonth, Player.cpp:15445-15454).
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7605u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    session.monthly_quests_completed_like_cpp.insert(quest.id);

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_allows_monthly_not_yet_completed_like_cpp() {
    // POSITIVE: a monthly quest not on cooldown is acceptable.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(7606u32);
    quest.special_flags = QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

// ── SatisfyQuestExclusiveGroup tests ────────────────────────────────────

#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_rewarded_non_repeatable_like_cpp() {
    // NEGATIVA: peer del mismo exclusive_group (>0) ya rewarded (no repetible)
    // → Player.cpp:15379 segundo término: !(repeatable && repeatable) && rewarded → false.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9910u32);
    peer.exclusive_group = 5;
    // is_repeatable() false por defecto (quest_type=2, special_flags=0)

    let mut quest = quest_template(9911u32);
    quest.exclusive_group = 5;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // El peer ya fue recompensado (no repetible).
    session.rewarded_quests.insert(peer.id);

    // La quest objetivo no está ni activa ni rewarded.
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_blocks_when_peer_active_like_cpp() {
    // NEGATIVA-2: peer del mismo exclusive_group activo en player_quests (status != NONE)
    // → Player.cpp:15379 primer término: GetQuestStatus(peer) != NONE → false.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9912u32);
    peer.exclusive_group = 7;

    let mut quest = quest_template(9913u32);
    quest.exclusive_group = 7;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // El peer está activo (Incomplete).
    add_active_quest(&mut session, peer.id);

    // La quest objetivo aún no ha sido aceptada.
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_positive_no_conflicting_peer_allows_like_cpp() {
    // POSITIVA: exclusive_group > 0 pero ningún peer está activo ni rewarded → true.
    let (mut session, _send_rx) = make_session();

    let mut peer = quest_template(9914u32);
    peer.exclusive_group = 9;

    let mut quest = quest_template(9915u32);
    quest.exclusive_group = 9;

    let store = QuestStore::from_quests_like_cpp([peer.clone(), quest.clone()]);
    session.set_quest_store(Arc::new(store));

    // Ningún peer activo ni rewarded → debe permitir.
    assert!(session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_exclusive_group_zero_never_blocks_like_cpp() {
    // POSITIVA: exclusive_group <= 0 → Player.cpp:15351 → siempre true.
    let (mut session, _send_rx) = make_session();

    let mut quest = quest_template(9916u32);
    quest.exclusive_group = 0;

    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));

    // También verifica group negativo.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(9917u32);
    quest2.exclusive_group = -3;

    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));

    assert!(session2.can_take_quest(&quest2));
}

// ── SatisfyQuestDependentPreviousQuests tests ────────────────────────────

#[test]
fn can_take_quest_dependent_previous_not_rewarded_blocks_like_cpp() {
    // NEGATIVA: quest objetivo con dependent_previous_quests=[prev] (exclusive_group=0 >= 0),
    // prev NO en rewarded_quests → Player.cpp:15121-15177 → false.
    let (mut session, _send_rx) = make_session();

    let quest_id = 9920u32;
    let prev_id = 9921u32;

    // Construir quest previa con next_quest_id → quest objetivo, exclusive_group=0 (>= 0).
    let mut prev_quest = quest_template(prev_id);
    prev_quest.next_quest_id = quest_id;
    prev_quest.exclusive_group = 0;

    // from_quests_like_cpp normaliza dependent_previous_quests automáticamente.
    let store = Arc::new(QuestStore::from_quests_like_cpp([
        quest_template(quest_id),
        prev_quest,
    ]));
    // Obtener la quest del store ya normalizado (con dependent_previous_quests populado).
    let quest = store.get(quest_id).expect("quest in store").clone();
    session.set_quest_store(Arc::clone(&store));

    // prev NO en rewarded_quests → el gate debe bloquear.
    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_dependent_previous_rewarded_allows_like_cpp() {
    // POSITIVA: mismo setup pero prev en rewarded_quests (exclusive_group=0 >= 0)
    // → Player.cpp:15134 → false (no blocked) → can_take_quest sigue adelante.
    let (mut session, _send_rx) = make_session();

    let quest_id = 9922u32;
    let prev_id = 9923u32;

    let mut prev_quest = quest_template(prev_id);
    prev_quest.next_quest_id = quest_id;
    prev_quest.exclusive_group = 0;

    let store = Arc::new(QuestStore::from_quests_like_cpp([
        quest_template(quest_id),
        prev_quest,
    ]));
    let quest = store.get(quest_id).expect("quest in store").clone();
    session.set_quest_store(Arc::clone(&store));

    // prev en rewarded_quests → el gate no bloquea.
    session.rewarded_quests.insert(prev_id);
    assert!(session.can_take_quest(&quest));
}

// ── SatisfyQuestReputation tests ─────────────────────────────────────────

#[test]
fn can_take_quest_reputation_blocks_when_below_min_rep_like_cpp() {
    // NEGATIVA min: required_min_rep_faction != 0, jugador con standing < required_min_rep_value.
    // → Player.cpp:15265 → false.
    let (mut session, _send_rx) = make_session();

    // Facción 76 con reputation_index 5.
    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Jugador standing 100 — por debajo del mínimo requerido (500).
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 100;

    let mut quest = quest_template(9920u32);
    quest.required_min_rep_faction = faction_id;
    quest.required_min_rep_value = 500;
    // Aislar el gate: sin restricciones de raza/clase/level/conditions/expansion/exclusive_group.
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_reputation_blocks_when_at_or_above_max_rep_like_cpp() {
    // NEGATIVA max: required_max_rep_faction != 0, jugador con standing >= required_max_rep_value.
    // → Player.cpp:15277 → false.
    let (mut session, _send_rx) = make_session();

    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Jugador standing 1000 — igual al máximo requerido (1000) → bloqueado.
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 1000;

    let mut quest = quest_template(9921u32);
    quest.required_max_rep_faction = faction_id;
    quest.required_max_rep_value = 1000;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_reputation_allows_when_in_valid_range_like_cpp() {
    // POSITIVA: standing >= min y < max → el gate de reputación no bloquea.
    let (mut session, _send_rx) = make_session();

    let rep_list_id: u32 = 5;
    let faction_id: u32 = 76;
    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16),
    ])));
    // Standing 500 — exactamente en el mínimo (500) y por debajo del máximo (1000).
    session
        .reputation_mgr_like_cpp_mut()
        .get_state_mut(rep_list_id)
        .expect("reputation state")
        .standing = 500;

    let mut quest = quest_template(9922u32);
    quest.required_min_rep_faction = faction_id;
    quest.required_min_rep_value = 500;
    quest.required_max_rep_faction = faction_id;
    quest.required_max_rep_value = 1000;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

// ── SatisfyQuestSkill tests ──────────────────────────────────────────────

#[test]
fn can_take_quest_skill_blocks_when_skill_below_required_like_cpp() {
    // NEGATIVA: required_skill_id != 0, skill del jugador < required_skill_points → false.
    // Player.cpp:15015-15037.
    let skill_id: u16 = 1_000;
    let required_points: u32 = 300;

    let (mut session, _send_rx) = make_session();
    // Jugador con skill 1000 en valor 299 — por debajo del requisito.
    session
        .set_player_skill_values_like_cpp(std::collections::HashMap::from([(skill_id, 299_u16)]));

    let mut quest = quest_template(9940u32);
    quest.required_skill_id = u32::from(skill_id);
    quest.required_skill_points = required_points;
    // Aislar el gate: sin restricciones de raza/clase/level/rep/conditions/expansion.
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(!session.can_take_quest(&quest));
}

#[test]
fn can_take_quest_skill_allows_when_skill_at_or_above_required_like_cpp() {
    // POSITIVA: skill del jugador >= required_skill_points → el gate no bloquea.
    let skill_id: u16 = 1_000;
    let required_points: u32 = 300;

    let (mut session, _send_rx) = make_session();
    // Jugador con skill 1000 exactamente en 300.
    session
        .set_player_skill_values_like_cpp(std::collections::HashMap::from([(skill_id, 300_u16)]));

    let mut quest = quest_template(9941u32);
    quest.required_skill_id = u32::from(skill_id);
    quest.required_skill_points = required_points;
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));

    assert!(session.can_take_quest(&quest));
}

// ── SatisfyQuestDependentBreadcrumbQuests tests ──────────────────────────

#[test]
fn can_take_quest_blocks_when_dependent_breadcrumb_in_log_like_cpp() {
    let breadcrumb_quest_id = 9930u32;
    let quest_id = 9931u32;

    // NEGATIVA: breadcrumb B (9930) está en player_quests con status INCOMPLETE
    // → Player.cpp:15203-15222 → false.
    let (mut session, _send_rx) = make_session();
    let mut quest = quest_template(quest_id);
    quest.dependent_breadcrumb_quests = vec![breadcrumb_quest_id];
    let store = QuestStore::from_quests_like_cpp([quest.clone()]);
    session.set_quest_store(Arc::new(store));
    add_active_quest(&mut session, breadcrumb_quest_id);
    assert!(!session.can_take_quest(&quest));

    // POSITIVA: breadcrumb no está en el log → no bloquea.
    let (mut session2, _send_rx2) = make_session();
    let mut quest2 = quest_template(quest_id);
    quest2.dependent_breadcrumb_quests = vec![breadcrumb_quest_id];
    let store2 = QuestStore::from_quests_like_cpp([quest2.clone()]);
    session2.set_quest_store(Arc::new(store2));
    // breadcrumb_quest_id no insertado en player_quests
    assert!(session2.can_take_quest(&quest2));
}
