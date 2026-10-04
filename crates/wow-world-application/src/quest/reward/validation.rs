// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest package and reward-choice validation.

use wow_data::{
    CurrencyTypesStore, ItemStatsStore, ItemStore, QuestPackageItemEntry,
    QuestPackageItemStore,
};

use super::QuestRewardCx;

impl QuestRewardCx<'_> {
    pub fn reward_choice_template_exists_like_cpp(
        choice_loot_item_type: u8,
        item_loot_item_type: u8,
        currency_loot_item_type: u8,
        item_id: u32,
        item_store: Option<&ItemStore>,
        currency_types: Option<&CurrencyTypesStore>,
    ) -> bool {
        match choice_loot_item_type {
            item_type if item_type == item_loot_item_type => {
                item_store.is_some_and(|store| store.get(item_id).is_some())
            }
            currency_type if currency_type == currency_loot_item_type => {
                currency_types.is_some_and(|store| store.has_record(item_id))
            }
            _ => false,
        }
    }

    pub fn can_select_quest_package_item_like_cpp(
        quest_package_item: &QuestPackageItemEntry,
        item_store: Option<&ItemStore>,
        item_stats_store: Option<&ItemStatsStore>,
        player_race: u8,
    ) -> bool {
        let Ok(item_id) = u32::try_from(quest_package_item.item_id) else {
            return false;
        };
        if item_store.is_none_or(|store| store.get(item_id).is_none()) {
            return false;
        }

        let Some(sparse) = item_stats_store.and_then(|store| store.sparse_template(item_id)) else {
            return false;
        };

        let player_team =
            wow_world_core::session::state::hub_support::player_team_for_race_cpp(player_race);
        if ((sparse.flags[1] & wow_constants::item::ItemFlags2::FactionAlliance as u32) != 0
            && player_team != wow_constants::unit::Team::Alliance)
            || ((sparse.flags[1] & wow_constants::item::ItemFlags2::FactionHorde as u32) != 0
                && player_team != wow_constants::unit::Team::Horde)
        {
            return false;
        }

        matches!(
            quest_package_item.display_type,
            wow_data::QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP
        )
    }

    pub fn quest_package_choice_matches_like_cpp(
        quest_package_id: u32,
        choice_loot_item_type: u8,
        item_loot_item_type: u8,
        choice_item_id: u32,
        store: Option<&QuestPackageItemStore>,
        item_store: Option<&ItemStore>,
        item_stats_store: Option<&ItemStatsStore>,
        player_race: u8,
    ) -> bool {
        if choice_loot_item_type != item_loot_item_type || quest_package_id == 0 {
            return false;
        }

        let Some(store) = store else {
            return false;
        };
        let Ok(choice_item_id) = i32::try_from(choice_item_id) else {
            return false;
        };

        let primary_valid = store
            .quest_package_items_like_cpp(quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .any(|entry| {
                Self::can_select_quest_package_item_like_cpp(
                    entry,
                    item_store,
                    item_stats_store,
                    player_race,
                )
            });
        if primary_valid {
            return true;
        }

        store
            .quest_package_items_fallback_like_cpp(quest_package_id)
            .any(|entry| entry.item_id == choice_item_id)
    }
}
