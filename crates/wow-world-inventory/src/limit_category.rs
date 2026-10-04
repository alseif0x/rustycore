// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_data::{
    ItemLimitCategoryConditionStore, ItemLimitCategoryEntry, ItemLimitCategoryStore,
    PlayerConditionContextLikeCpp, PlayerConditionStore, is_player_meeting_condition_like_cpp,
};
use wow_entities::ItemLimitCategoryTemplate;

impl crate::InventoryState {
    pub fn item_limit_category_template_with_context_like_cpp(
        &self,
        entry: &ItemLimitCategoryEntry,
        condition_store: Option<&ItemLimitCategoryConditionStore>,
        player_condition_store: Option<&PlayerConditionStore>,
        context: Option<PlayerConditionContextLikeCpp<'_>>,
    ) -> Option<ItemLimitCategoryTemplate> {
        let mut quantity = entry.quantity;
        if let Some(condition_store) = condition_store {
            let context = context?;
            for condition in condition_store.conditions_for_parent_like_cpp(entry.id) {
                let player_condition =
                    player_condition_store.and_then(|store| store.get(condition.player_condition_id));
                if player_condition.is_none_or(|condition| {
                    is_player_meeting_condition_like_cpp(condition, &context)
                }) {
                    quantity = (i16::from(quantity) + i16::from(condition.add_quantity)) as u8;
                }
            }
        }

        Some(ItemLimitCategoryTemplate {
            id: entry.id,
            quantity,
            flags: entry.flags,
        })
    }

    pub fn item_limit_category_entry_like_cpp<'a>(
        &self,
        limit_category_id: u32,
        store: Option<&'a ItemLimitCategoryStore>,
    ) -> Option<&'a ItemLimitCategoryEntry> {
        (limit_category_id != 0)
            .then(|| store.and_then(|store| store.get(limit_category_id)))
            .flatten()
    }
}
