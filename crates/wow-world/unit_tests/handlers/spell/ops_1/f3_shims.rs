// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.
//! #1363 slice 4c: they forward to the bodies moved to `wow_world_loot_ops`.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::handlers::spell) async fn load_item_template_addon_money_loot_like_cpp(
        &self,
        item_entry: u32,
    ) -> (u32, u32) {
        wow_world_loot_ops::load_item_template_addon_money_loot_like_cpp(self, item_entry).await
    }
    pub(in crate::handlers::spell) async fn load_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_entry: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        wow_world_loot_ops::load_item_template_addon_loot_metadata_like_cpp(self, item_entry).await
    }
}
