// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory storage, equip/swap, destroy, durability and item modification.

use super::*;
mod destruction;
mod equipment_sets;
mod handlers;
mod inventory_moves;
pub(super) mod login_load;

pub(crate) fn item_turnin_persistence_rows_like_cpp(
    player_guid: ObjectGuid,
    changes: &[ExtendedCostItemTurninChange],
) -> Vec<wow_persistence::VendorItemTurninPersistenceLikeCpp> {
    changes
        .iter()
        .map(|change| match *change {
            ExtendedCostItemTurninChange::Update {
                db_guid, new_count, ..
            } => wow_persistence::VendorItemTurninPersistenceLikeCpp::Update {
                item_guid: db_guid,
                new_count,
            },
            ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                wow_persistence::VendorItemTurninPersistenceLikeCpp::Delete {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: db_guid,
                }
            }
        })
        .collect()
}

impl WorldSession {
    pub(crate) fn inventory_container_db_guid_like_cpp(&self, bag: u8) -> Option<u64> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.inventory_container_db_guid_like_cpp(hub, bag)
    }

    pub(super) fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.has_item_count_direct_inventory(hub, item_entry, count)
    }

    pub(crate) fn plan_destroy_item_count_direct_inventory(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.plan_destroy_item_count_direct_inventory(hub, item_entry, count)
    }

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
