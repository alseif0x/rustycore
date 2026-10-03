// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_world_core::session::HubRef;

impl crate::InventoryState {
    /// C++ `Player::GetItemByEntry(entry, ItemSearchLocation::Default)`.
    pub fn represented_player_has_default_item_entry_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_id: u32,
    ) -> bool {
        let Some(player) = self.direct_inventory_player_snapshot(hub) else {
            return false;
        };
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp(hub) else {
            return false;
        };
        let mut found = false;
        player.for_each_item_guid_like_cpp(wow_entities::ItemSearchLocation::DEFAULT, |item_guid| {
            if item_objects
                .get(&item_guid)
                .is_some_and(|item| item.object().entry() == item_id)
            {
                found = true;
                wow_entities::ItemSearchCallbackResult::Stop
            } else {
                wow_entities::ItemSearchCallbackResult::Continue
            }
        });
        found
    }
}
