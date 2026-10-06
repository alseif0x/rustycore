// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-package choice and reward-inventory admission.

use super::*;

impl WorldSession {
    pub(in crate::handlers::quest) fn represented_reward_choice_template_exists_like_cpp(
        &self,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let hub = crate::session::hub_ref(self);
        wow_world_application::QuestRewardCx::reward_choice_template_exists_like_cpp(
            choice.loot_item_type,
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP,
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP,
            choice.item_id,
            hub.catalogs.item_store().map(Arc::as_ref),
            hub.catalogs.currency_types_store().map(Arc::as_ref),
        )
    }

    pub(in crate::handlers::quest) fn represented_can_select_quest_package_item_like_cpp(
        &self,
        quest_package_item: &QuestPackageItemEntry,
    ) -> bool {
        let hub = crate::session::hub_ref(self);
        wow_world_application::QuestRewardCx::can_select_quest_package_item_like_cpp(
            quest_package_item,
            hub.catalogs.item_store().map(Arc::as_ref),
            hub.catalogs.item_stats_store().map(Arc::as_ref),
            hub.player_race_like_cpp(),
        )
    }

    pub(in crate::handlers::quest) fn represented_quest_package_choice_matches_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let hub = crate::session::hub_ref(self);
        wow_world_application::QuestRewardCx::quest_package_choice_matches_like_cpp(
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

    pub(in crate::handlers::quest) fn send_quest_failed_like_cpp(
        &self,
        quest_id: u32,
        reason: InventoryResult,
    ) {
        if quest_id == 0 {
            return;
        }

        self.send_packet(&QuestGiverQuestFailed {
            quest_id,
            reason: reason as u32,
        });
    }

    fn represented_quest_reward_inventory_plan_result_like_cpp(
        &self,
        item_id: u32,
        count: u32,
    ) -> InventoryResult {
        self.plan_store_new_direct_inventory_item(item_id, count)
            .map(|(result, _, _)| result)
            .unwrap_or(InventoryResult::ItemNotFound)
    }

    pub(in crate::handlers::quest) fn send_quest_package_reward_inventory_error_like_cpp(
        &self,
        result: InventoryResult,
        item_id: u32,
    ) {
        let hub = crate::session::hub_ref(self);
        let limit_category = hub
            .catalogs
            .item_storage_template(item_id)
            .map(|template| u32::from(template.item_limit_category))
            .unwrap_or(0);
        hub.core
            .send_equip_error(result, None, None, 0, limit_category);
    }

    pub(in crate::handlers::quest) fn represented_can_reward_quest_inventory_like_cpp(
        &self,
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
                    self.represented_quest_reward_inventory_plan_result_like_cpp(*item_id, *count);
                if result != InventoryResult::Ok {
                    self.send_quest_failed_like_cpp(quest.id, result);
                    return false;
                }
            }
        }

        for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
            if *item_id == 0 {
                continue;
            }

            let result =
                self.represented_quest_reward_inventory_plan_result_like_cpp(*item_id, *count);
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
        }

        if quest.quest_package_id == 0
            || choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
        {
            return true;
        }

        let Some(store) = &self.catalogs.quests.package_item_store else {
            return true;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return true;
        };

        let mut has_filtered_quest_package_reward = false;
        for entry in store.quest_package_items_like_cpp(quest.quest_package_id) {
            if entry.item_id != choice_item_id
                || !self.represented_can_select_quest_package_item_like_cpp(entry)
            {
                continue;
            }

            has_filtered_quest_package_reward = true;
            let Ok(item_id) = u32::try_from(entry.item_id) else {
                self.send_quest_package_reward_inventory_error_like_cpp(
                    InventoryResult::ItemNotFound,
                    0,
                );
                return false;
            };
            let result = self.represented_quest_reward_inventory_plan_result_like_cpp(
                item_id,
                entry.item_quantity,
            );
            if result != InventoryResult::Ok {
                self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
                return false;
            }
        }

        if !has_filtered_quest_package_reward {
            for entry in store.quest_package_items_fallback_like_cpp(quest.quest_package_id) {
                if entry.item_id != choice_item_id {
                    continue;
                }

                let Ok(item_id) = u32::try_from(entry.item_id) else {
                    self.send_quest_package_reward_inventory_error_like_cpp(
                        InventoryResult::ItemNotFound,
                        0,
                    );
                    return false;
                };
                let result = self.represented_quest_reward_inventory_plan_result_like_cpp(
                    item_id,
                    entry.item_quantity,
                );
                if result != InventoryResult::Ok {
                    self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
                    return false;
                }
            }
        }

        true
    }
}
