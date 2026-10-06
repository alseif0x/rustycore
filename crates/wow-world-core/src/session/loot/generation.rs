// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::hub_support::RepresentedLootPlayerContext;
use wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;

impl crate::session::SessionCatalogs {
    pub fn remote_has_incomplete_quest_item_drop_for_item_like_cpp(
        &self,
        item_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };

        player_context
            .active_quest_statuses
            .iter()
            .any(|(quest_id, status)| {
                if *status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .item_drop
                    .iter()
                    .enumerate()
                    .any(|(index, drop_item_id)| {
                        if *drop_item_id != item_id {
                            return false;
                        }

                        let Some(template) = self.item_storage_template(item_id) else {
                            return false;
                        };

                        let quantity = quest.item_drop_quantity[index];
                        let mut max_allowed_count = if quantity != 0 {
                            quantity
                        } else {
                            template.max_stack_size
                        };
                        if template.max_count > 0 {
                            max_allowed_count = max_allowed_count.min(template.max_count as u32);
                        }

                        player_context.inventory_item_count(item_id) < max_allowed_count
                    })
            })
    }
}
