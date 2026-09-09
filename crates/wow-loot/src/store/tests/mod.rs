//! Loot store definitions and templates regression scenarios.
//!
//! Separated from the lib.rs root under #642.

use super::{
    GeneratedLootItem, LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP, LOOT_METHOD_GROUP_LIKE_CPP,
    LOOT_METHOD_MASTER_LIKE_CPP, LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP,
    LOOT_METHOD_PERSONAL_LIKE_CPP, LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
    LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP,
    LOOT_SLOT_TYPE_MASTER_LIKE_CPP, LOOT_SLOT_TYPE_OWNER_LIKE_CPP,
    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootConditionId, LootConditionLinkReport,
    LootConditionReferenceUseLikeCpp, LootConditionRowLikeCpp, LootFillError, LootFillOptions,
    LootItemRandomProperties, LootItemTemplateMetadata, LootReferenceCheckReport, LootReferenceUse,
    LootStore, LootStoreItem, LootStoreItemContext, LootStoreKind, LootStoreLoadError, LootStores,
    LootTemplate, LootTemplateRow, MissingLootConditionItemTemplate, MissingLootConditionTemplate,
    MissingLootConditionTemplateItem, check_loot_condition_links_like_cpp,
    check_loot_condition_references_like_cpp, check_loot_references_like_cpp,
    condition_compare_values_like_cpp, generate_money_loot_with_rate_like_cpp,
    loot_condition_reference_ids_like_cpp, loot_condition_reference_self_references_like_cpp,
    loot_condition_row_is_loadable_without_external_stores_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_like_cpp_representable,
    loot_conditions_allow_player_with_references_like_cpp_representable,
    loot_item_ui_type_for_player_like_cpp,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use wow_core::ObjectGuid;

const DEFAULT_LOOT_MODE: u16 = 0x01;

fn item(item_id: u32, reference: u32, chance: f32, group_id: u8) -> LootStoreItem {
    LootStoreItem {
        item_id,
        reference,
        chance,
        needs_quest: false,
        loot_mode: DEFAULT_LOOT_MODE,
        group_id,
        min_count: 1,
        max_count: 1,
    }
}

fn item_metadata(max_stack: u32) -> LootItemTemplateMetadata {
    LootItemTemplateMetadata {
        max_stack,
        has_multi_drop_flag: false,
        has_follow_loot_rules_flag: false,
    }
}

fn guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

#[allow(clippy::too_many_arguments)]
fn ui_type(
    player: ObjectGuid,
    allowed_looters: &[ObjectGuid],
    is_looted_for_player: bool,
    free_for_all: bool,
    player_has_unlooted_ffa_item: bool,
    needs_quest: bool,
    follow_loot_rules: bool,
    loot_method: u8,
    round_robin_player: ObjectGuid,
    loot_master_guid: ObjectGuid,
    is_under_threshold: bool,
    is_blocked: bool,
    roll_winner_guid: ObjectGuid,
) -> Option<u8> {
    loot_item_ui_type_for_player_like_cpp(
        player,
        allowed_looters,
        is_looted_for_player,
        free_for_all,
        player_has_unlooted_ffa_item,
        needs_quest,
        follow_loot_rules,
        loot_method,
        round_robin_player,
        loot_master_guid,
        is_under_threshold,
        is_blocked,
        roll_winner_guid,
    )
}

fn generated_item(
    store_item: LootStoreItem,
    count: u32,
    loot_list_id: u32,
    store_kind: LootStoreKind,
    entry: u32,
) -> GeneratedLootItem {
    let item_id = store_item.item_id;
    GeneratedLootItem {
        item_id,
        count,
        loot_list_id,
        random_properties_id: item_id as i32 + 1000,
        random_properties_seed: item_id as i32 + 2000,
        context: 0,
        store_item_context: LootStoreItemContext {
            store_kind,
            entry,
            item: store_item,
        },
        free_for_all: false,
        follow_loot_rules: true,
        needs_quest: false,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    }
}

fn random_properties<R: Rng + ?Sized>(item_id: u32, _rng: &mut R) -> LootItemRandomProperties {
    LootItemRandomProperties {
        id: item_id as i32 + 1000,
        seed: item_id as i32 + 2000,
    }
}

fn condition(
    else_group: u32,
    condition_type_or_reference: i32,
    value1: u32,
    negative: bool,
) -> LootConditionRowLikeCpp {
    LootConditionRowLikeCpp {
        else_group,
        condition_type_or_reference,
        condition_target: 0,
        value1,
        value2: 0,
        value3: 0,
        string_value1: String::new(),
        negative,
        script_name: String::new(),
    }
}

mod scenarios_1;
mod scenarios_2;
