// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-package choice and reward-inventory admission, moved from the World
//! `handlers/quest/rewards/validation.rs` (#1363 slice 6b).

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestChoiceItemLikeCpp {
    pub loot_item_type: u8,
    pub item_id: u32,
    pub quantity: i32,
}

pub fn represented_reward_choice_template_exists_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    choice: QuestChoiceItemLikeCpp,
) -> bool {
    let hub = host.hub_ref();
    crate::QuestRewardCx::reward_choice_template_exists_like_cpp(
        choice.loot_item_type,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
        choice.item_id,
        hub.catalogs.item_store().map(Arc::as_ref),
        hub.catalogs.currency_types_store().map(Arc::as_ref),
    )
}

pub(crate) fn represented_can_select_quest_package_item_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_package_item: &QuestPackageItemEntry,
) -> bool {
    let hub = host.hub_ref();
    crate::QuestRewardCx::can_select_quest_package_item_like_cpp(
        quest_package_item,
        hub.catalogs.item_store().map(Arc::as_ref),
        hub.catalogs.item_stats_store().map(Arc::as_ref),
        hub.player_race_like_cpp(),
    )
}

pub fn represented_quest_package_choice_matches_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
    choice: QuestChoiceItemLikeCpp,
) -> bool {
    let hub = host.hub_ref();
    crate::QuestRewardCx::quest_package_choice_matches_like_cpp(
        quest.quest_package_id,
        choice.loot_item_type,
        QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
        choice.item_id,
        hub.catalogs.quests.package_item_store.as_deref(),
        hub.catalogs.item_store().map(Arc::as_ref),
        hub.catalogs.item_stats_store().map(Arc::as_ref),
        hub.player_race_like_cpp(),
    )
}

pub(crate) fn send_quest_failed_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_id: u32,
    reason: InventoryResult,
) {
    if quest_id == 0 {
        return;
    }

    host.send_packet(&QuestGiverQuestFailed {
        quest_id,
        reason: reason as u32,
    });
}

pub(crate) fn represented_quest_reward_inventory_plan_result_like_cpp<
    H: QuestItemStorageHostLikeCpp,
>(
    host: &H,
    item_id: u32,
    count: u32,
) -> InventoryResult {
    host.plan_store_new_direct_inventory_item(item_id, count)
        .map(|(result, _, _)| result)
        .unwrap_or(InventoryResult::ItemNotFound)
}

pub(crate) fn send_quest_package_reward_inventory_error_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    result: InventoryResult,
    item_id: u32,
) {
    let hub = host.hub_ref();
    let limit_category = hub
        .catalogs
        .item_storage_template(item_id)
        .map(|template| u32::from(template.item_limit_category))
        .unwrap_or(0);
    hub.core
        .send_equip_error(result, None, None, 0, limit_category);
}

pub fn represented_can_reward_quest_inventory_like_cpp<H: QuestItemStorageHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
    choice: QuestChoiceItemLikeCpp,
) -> bool {
    // C++ `Player::CanRewardQuest(quest, rewardType, rewardId, true)`.
    if choice.loot_item_type == QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP {
        for ((item_id, count), item_type) in quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
        {
            if *item_id == 0
                || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                || *item_id != choice.item_id
            {
                continue;
            }

            let result =
                represented_quest_reward_inventory_plan_result_like_cpp(host, *item_id, *count);
            if result != InventoryResult::Ok {
                send_quest_failed_like_cpp(host, quest.id, result);
                return false;
            }
        }
    }

    for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
        if *item_id == 0 {
            continue;
        }

        let result =
            represented_quest_reward_inventory_plan_result_like_cpp(host, *item_id, *count);
        if result != InventoryResult::Ok {
            send_quest_failed_like_cpp(host, quest.id, result);
            return false;
        }
    }

    if quest.quest_package_id == 0
        || choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
    {
        return true;
    }

    let Some(store) = &host.hub_ref().catalogs.quests.package_item_store else {
        return true;
    };
    let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
        return true;
    };

    let mut has_filtered_quest_package_reward = false;
    for entry in store.quest_package_items_like_cpp(quest.quest_package_id) {
        if entry.item_id != choice_item_id
            || !represented_can_select_quest_package_item_like_cpp(host, entry)
        {
            continue;
        }

        has_filtered_quest_package_reward = true;
        let Ok(item_id) = u32::try_from(entry.item_id) else {
            send_quest_package_reward_inventory_error_like_cpp(
                host,
                InventoryResult::ItemNotFound,
                0,
            );
            return false;
        };
        let result = represented_quest_reward_inventory_plan_result_like_cpp(
            host,
            item_id,
            entry.item_quantity,
        );
        if result != InventoryResult::Ok {
            send_quest_package_reward_inventory_error_like_cpp(host, result, item_id);
            return false;
        }
    }

    if !has_filtered_quest_package_reward {
        for entry in store.quest_package_items_fallback_like_cpp(quest.quest_package_id) {
            if entry.item_id != choice_item_id {
                continue;
            }

            let Ok(item_id) = u32::try_from(entry.item_id) else {
                send_quest_package_reward_inventory_error_like_cpp(
                    host,
                    InventoryResult::ItemNotFound,
                    0,
                );
                return false;
            };
            let result = represented_quest_reward_inventory_plan_result_like_cpp(
                host,
                item_id,
                entry.item_quantity,
            );
            if result != InventoryResult::Ok {
                send_quest_package_reward_inventory_error_like_cpp(host, result, item_id);
                return false;
            }
        }
    }

    true
}
