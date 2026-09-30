//! Literal catalog rows shared by the Character item fixtures.
//! Prices and storage defaults match the inventory fixtures; Appearance
//! retains its differing fields as explicit caller overrides.

use wow_constants::ItemBondingType;
use wow_data::item::ItemRecord;
use wow_data::item::stats::ItemSparseTemplateEntry;

pub(super) fn basic_item_record(
    entry_id: u32,
    class_id: u8,
    subclass_id: u8,
    inventory_type: i8,
) -> ItemRecord {
    ItemRecord {
        id: entry_id,
        class_id: class_id,
        subclass_id: subclass_id,
        material: 0,
        inventory_type: inventory_type,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }
}

pub(super) fn inventory_sparse_template(inventory_type: i8) -> ItemSparseTemplateEntry {
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
        inventory_type: inventory_type,
    }
}
