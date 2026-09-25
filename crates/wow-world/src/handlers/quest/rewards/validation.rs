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
        match choice.loot_item_type {
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP => self
                .item_store()
                .is_some_and(|store| store.get(choice.item_id).is_some()),
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP => self
                .currency_types_store()
                .is_some_and(|store| store.has_record(choice.item_id)),
            _ => false,
        }
    }

    pub(in crate::handlers::quest) fn represented_can_select_quest_package_item_like_cpp(
        &self,
        quest_package_item: &QuestPackageItemEntry,
    ) -> bool {
        let Ok(item_id) = u32::try_from(quest_package_item.item_id) else {
            return false;
        };
        if self
            .item_store()
            .is_none_or(|store| store.get(item_id).is_none())
        {
            return false;
        }

        let Some(sparse) = self
            .item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
        else {
            return false;
        };

        let player_team = crate::session::player_team_for_race_cpp(self.player_race_like_cpp());
        if ((sparse.flags[1] & ItemFlags2::FactionAlliance as u32) != 0
            && player_team != wow_constants::unit::Team::Alliance)
            || ((sparse.flags[1] & ItemFlags2::FactionHorde as u32) != 0
                && player_team != wow_constants::unit::Team::Horde)
        {
            return false;
        }

        match quest_package_item.display_type {
            QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP => true,
            QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP => false,
            QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP => false,
            _ => false,
        }
    }

    pub(in crate::handlers::quest) fn represented_quest_package_choice_matches_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
            || quest.quest_package_id == 0
        {
            return false;
        }

        let Some(store) = &self.quests.package_item_store else {
            return false;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return false;
        };

        let primary_valid = store
            .quest_package_items_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .any(|entry| self.represented_can_select_quest_package_item_like_cpp(entry));
        if primary_valid {
            return true;
        }

        store
            .quest_package_items_fallback_like_cpp(quest.quest_package_id)
            .any(|entry| entry.item_id == choice_item_id)
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
        let limit_category = self
            .item_storage_template(item_id)
            .map(|template| u32::from(template.item_limit_category))
            .unwrap_or(0);
        self.send_equip_error(result, None, None, 0, limit_category);
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

        let Some(store) = &self.quests.package_item_store else {
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
