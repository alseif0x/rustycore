//! Item and inventory fixtures shared by quest source-item scenarios.

use super::quest_template;
use crate::session::{InventoryItem, WorldSession};
use std::sync::Arc;
use wow_constants::{InventoryType, ItemBondingType, ItemClass, ItemContext};
use wow_core::ObjectGuid;
use wow_data::quest::{QuestStore, QuestTemplate};
use wow_data::{
    ItemLimitCategoryEntry, ItemLimitCategoryStore, ItemRecord, ItemSparseTemplateEntry,
    ItemStatsStore, ItemStore,
};
use wow_entities::ITEM_LIMIT_CATEGORY_MODE_HAVE;

pub(crate) fn quest_template_with_source_item(
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

pub(crate) fn store_with_source_item_quest(
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

pub(crate) fn install_source_item_template(
    session: &mut WorldSession,
    entry: u32,
    stackable: i32,
    max_count: u32,
) {
    install_source_item_template_with_start_quest_limit_category_and_flags3(
        session, entry, stackable, max_count, 0, 0, 0,
    );
}

pub(crate) fn install_source_item_template_with_flags3(
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

pub(crate) fn install_source_item_template_with_start_quest(
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

pub(crate) fn install_source_item_template_with_limit_category(
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

pub(crate) fn install_source_item_template_with_start_quest_and_limit_category(
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

pub(super) fn install_source_item_template_with_start_quest_limit_category_and_flags3(
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

pub(crate) fn install_source_item_template_with_start_quest_limit_category_flags3_and_bonding(
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

pub(crate) fn insert_direct_inventory_item(
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

pub(crate) fn install_have_limit_category_like_cpp(
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
