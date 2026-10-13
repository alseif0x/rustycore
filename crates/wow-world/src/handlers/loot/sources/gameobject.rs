// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject loot-source interaction and runtime state publication.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn open_represented_fishing_hole_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        wow_world_loot_ops::open_represented_fishing_hole_with_catalogs_like_cpp(
            self,
            &item_valuation,
            gameobject_guid,
            gameobject_entry,
            loot_id,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn open_represented_fishing_node_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        wow_world_loot_ops::open_represented_fishing_node_loot_with_catalogs_like_cpp(
            self,
            &item_valuation,
            gameobject_guid,
            area_id,
            junk,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn open_represented_gathering_node_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        wow_world_loot_ops::open_represented_gathering_node_with_catalogs_like_cpp(
            self,
            &item_valuation,
            gameobject_guid,
            gameobject_entry,
            source,
        )
        .await;
    }
}

impl crate::session::LootCxRef<'_> {
    pub(crate) fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        wow_world_loot_ops::queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
            &self.hub.core.loot_release_access_like_cpp(),
            self.world_entities,
            gameobject_guid,
        )
    }
}
