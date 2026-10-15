// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest objective progress and completion.

use super::*;

impl WorldSession {
    pub(crate) async fn quest_source_item_quest_log_item_id_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> u32 {
        if let Some(quest_log_item_id) = self
            .catalogs
            .item_template_addon_quest_log_item_id_like_cpp(entry_id)
        {
            return quest_log_item_id;
        }

        let Some(port) = crate::session::cx_inventory_ref(self)
            .item_template_addon_catalog_persistence_port_like_cpp()
        else {
            return 0;
        };

        let quest_log_item_id = match port
            .load_item_template_addon_loot_metadata_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp {
                    item_entry: entry_id,
                },
            )
            .await
        {
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(row) => {
                row.quest_log_item_id.try_into().unwrap_or(0)
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing => 0,
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.core.account_id,
                    entry_id,
                    error = %reason,
                    "QuestConfirmAccept: failed to load item_template_addon QuestLogItemId"
                );
                0
            }
        };
        self.catalogs
            .cache_item_template_addon_quest_log_item_id_like_cpp(entry_id, quest_log_item_id);
        quest_log_item_id
    }
}
