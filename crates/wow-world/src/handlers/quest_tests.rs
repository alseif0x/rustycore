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
use crate::player::inventory_persistence_test_fixture::PlayerInventoryPersistencePortFixtureLikeCpp;
use crate::player::quest_persistence_test_fixture::{
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
            ..Default::default()
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
            ..Default::default()
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

// ── SatisfyQuestDay / Week / Month tests ────────────────────────────────
// C++ Player::CanTakeQuest (Player.cpp:14093-14102) gates on
// SatisfyQuestDay && SatisfyQuestWeek && SatisfyQuestMonth. A daily/DF/weekly/
// monthly quest already on cooldown must not be re-acceptable.

// ── SatisfyQuestExclusiveGroup tests ────────────────────────────────────

// ── SatisfyQuestDependentPreviousQuests tests ────────────────────────────

// ── SatisfyQuestReputation tests ─────────────────────────────────────────

// ── SatisfyQuestSkill tests ──────────────────────────────────────────────

// ── SatisfyQuestDependentBreadcrumbQuests tests ──────────────────────────

#[path = "quest_tests/creature.rs"]
mod creature;
#[path = "quest_tests/gameobject.rs"]
mod gameobject;
#[path = "quest_tests/item_1.rs"]
mod item_1;
#[path = "quest_tests/item_2.rs"]
mod item_2;
#[path = "quest_tests/item_3.rs"]
mod item_3;
#[path = "quest_tests/item_4.rs"]
mod item_4;
#[path = "quest_tests/misc.rs"]
mod misc;
#[path = "quest_tests/quest_1.rs"]
mod quest_1;
#[path = "quest_tests/quest_2.rs"]
mod quest_2;
#[path = "quest_tests/quest_3.rs"]
mod quest_3;
#[path = "quest_tests/quest_4.rs"]
mod quest_4;
#[path = "quest_tests/quest_5.rs"]
mod quest_5;
#[path = "quest_tests/quest_6.rs"]
mod quest_6;
#[path = "quest_tests/quest_7.rs"]
mod quest_7;
#[path = "quest_tests/reward_transaction.rs"]
mod reward_transaction;
#[path = "quest_tests/spell.rs"]
mod spell;
