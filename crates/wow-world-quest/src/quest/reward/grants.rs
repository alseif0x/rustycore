// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered fixed, chosen and package item grants for one quest reward.

use super::super::QuestRewardDurablePlanLikeCpp;
use super::QuestRewardCx;
#[cfg(any(test, feature = "test-fixtures"))]
use super::item_planning::QuestRewardItemPlanningFixtureRefsLikeCpp;
use wow_constants::InventoryResult;
use wow_data::progression_rewards::QuestPackageItemEntry;

impl QuestRewardCx<'_> {
    pub(super) fn send_quest_failed_like_cpp(&self, quest_id: u32, reason: InventoryResult) {
        if quest_id == 0 {
            return;
        }
        self.send_packet_like_cpp(&wow_packet::packets::quest::QuestGiverQuestFailed {
            quest_id,
            reason: reason as u32,
        });
    }

    pub(super) fn send_quest_package_reward_inventory_error_like_cpp(
        &self,
        result: InventoryResult,
        item_id: u32,
    ) {
        let limit_category = self
            .catalogs
            .item_storage_template(item_id)
            .map(|template| u32::from(template.item_limit_category))
            .unwrap_or(0);
        self.player
            .send_equip_error_like_cpp(result, None, None, 0, limit_category);
    }

    pub async fn store_fixed_quest_reward_items_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> bool {
        for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
            if *item_id == 0 {
                continue;
            }
            let (result, dest, _) = self
                .plan_store_new_direct_inventory_item_like_cpp(
                    *item_id,
                    *count,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    vitals,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    reputation,
                )
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
            if !self
                .store_quest_reward_item_like_cpp(
                    plan,
                    item_guid_generator,
                    *item_id,
                    *count,
                    &dest,
                )
                .await
            {
                return false;
            }
        }
        true
    }

    pub async fn store_chosen_quest_reward_item_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        choice_item_id: u32,
        choice_loot_item_type: u8,
        item_loot_item_type: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> bool {
        if choice_loot_item_type != item_loot_item_type || choice_item_id == 0 {
            return true;
        }
        if self
            .catalogs
            .item_store()
            .is_none_or(|store| store.get(choice_item_id).is_none())
        {
            return true;
        }
        for ((item_id, count), item_type) in quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
        {
            if *item_id == 0 || *item_type != item_loot_item_type || *item_id != choice_item_id {
                continue;
            }
            let (result, dest, _) = self
                .plan_store_new_direct_inventory_item_like_cpp(
                    *item_id,
                    *count,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    vitals,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    reputation,
                )
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
            if !self
                .store_quest_reward_item_like_cpp(
                    plan,
                    item_guid_generator,
                    *item_id,
                    *count,
                    &dest,
                )
                .await
            {
                return false;
            }
        }
        true
    }

    async fn store_quest_package_reward_entry_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry: &QuestPackageItemEntry,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> bool {
        let Ok(item_id) = u32::try_from(entry.item_id) else {
            self.send_quest_package_reward_inventory_error_like_cpp(
                InventoryResult::ItemNotFound,
                0,
            );
            return false;
        };
        let (result, dest, _) = self
            .plan_store_new_direct_inventory_item_like_cpp(
                item_id,
                entry.item_quantity,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixtures,
                #[cfg(any(test, feature = "test-fixtures"))]
                vitals,
                #[cfg(any(test, feature = "test-fixtures"))]
                reputation,
            )
            .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
        if result != InventoryResult::Ok {
            self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
            return false;
        }
        self.store_quest_reward_item_like_cpp(
            plan,
            item_guid_generator,
            item_id,
            entry.item_quantity,
            &dest,
        )
        .await
    }

    pub async fn store_quest_package_reward_items_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        choice_item_id: u32,
        choice_loot_item_type: u8,
        item_loot_item_type: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))] vitals: (&u32, &u32, &bool),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation: &wow_entities::PlayerReputationStateLikeCpp,
    ) -> bool {
        if quest.quest_package_id == 0
            || choice_loot_item_type != item_loot_item_type
            || choice_item_id == 0
        {
            return true;
        }
        // C++ gates `RewardQuestPackage` behind a non-null selected reward item template.
        if self
            .catalogs
            .item_store()
            .is_none_or(|store| store.get(choice_item_id).is_none())
        {
            return true;
        }
        let Some(store) = &self.catalogs.quests.package_item_store else {
            return true;
        };
        let Ok(choice_item_id) = i32::try_from(choice_item_id) else {
            return true;
        };
        let primary_entries = store
            .quest_package_items_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .cloned()
            .collect::<Vec<_>>();
        let fallback_entries = store
            .quest_package_items_fallback_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut has_filtered_quest_package_reward = false;
        for entry in primary_entries {
            if !Self::can_select_quest_package_item_like_cpp(
                &entry,
                self.catalogs.item_store().map(std::sync::Arc::as_ref),
                self.catalogs.item_stats_store().map(std::sync::Arc::as_ref),
                self.player.player_race_like_cpp(),
            ) {
                continue;
            }
            has_filtered_quest_package_reward = true;
            if !self
                .store_quest_package_reward_entry_like_cpp(
                    plan,
                    item_guid_generator,
                    &entry,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    fixtures,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    vitals,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    reputation,
                )
                .await
            {
                return false;
            }
        }
        if !has_filtered_quest_package_reward {
            for entry in fallback_entries {
                if !self
                    .store_quest_package_reward_entry_like_cpp(
                        plan,
                        item_guid_generator,
                        &entry,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        fixtures,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        vitals,
                        #[cfg(any(test, feature = "test-fixtures"))]
                        reputation,
                    )
                    .await
                {
                    return false;
                }
            }
        }
        true
    }
}
