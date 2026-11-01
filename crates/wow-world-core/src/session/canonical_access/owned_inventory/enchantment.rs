// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::OwnedInventoryAccessLikeCpp;

impl OwnedInventoryAccessLikeCpp<'_> {
    pub fn inventory_enchantment_durations_snapshot_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<Vec<wow_entities::PlayerEnchantDuration>> {
        self.core.canonical_player_snapshot_like_cpp(|player| {
            player
                .enchant_durations()
                .iter()
                .filter(|duration| duration.item_guid == item_guid)
                .copied()
                .collect::<Vec<_>>()
        })
    }
}
