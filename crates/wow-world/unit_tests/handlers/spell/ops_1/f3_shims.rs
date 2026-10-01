// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::handlers::spell) async fn load_item_template_addon_money_loot_like_cpp(
        &self,
        item_entry: u32,
    ) -> (u32, u32) {
        crate::session::cx_inventory_ref(self)
            .load_item_template_addon_money_loot_like_cpp(item_entry)
            .await
    }
    pub(in crate::handlers::spell) async fn load_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_entry: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        crate::session::cx_inventory_ref(self)
            .load_item_template_addon_loot_metadata_like_cpp(item_entry)
            .await
    }
}
