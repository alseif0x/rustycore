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

pub(super) fn item_turnin_persistence_rows_like_cpp(
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

mod creature_equipment;
mod storage_move;
mod turnin;
