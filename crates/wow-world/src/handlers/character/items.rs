// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item turn-in application. The item handler bodies moved to
//! `wow_world_inventory::item_session` (#1363 slice 8c); `host` is their World
//! adapter.

use super::*;
mod equipment_sets;
mod handlers;
mod host;

impl WorldSession {
    pub(crate) fn apply_item_turnin_changes(
        &mut self,
        _player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        let access = self.core.owned_inventory_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        let send_stat_update = self
            .inventory
            .apply_item_turnin_changes_with_access_like_cpp(
                &access,
                &publication,
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                _player_guid,
                map_id,
                changes,
            );
        if send_stat_update {
            self.send_stat_update();
        }
    }
}
