// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The committed relocation, swap and merged-removal publications of the
//! session-owned inventory (#1363 slice 12b1b; moved verbatim from the World
//! `session/player_items/persistence.rs`).

use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use crate::{TrainerSessionFieldsLikeCpp, TrainerSessionHostLikeCpp};

/// Apply a previously validated and committed C++ `RemoveItem` +
/// `StoreItem`/`BankItem` relocation to the session-owned inventory.
///
/// `character_inventory.bag` persistence is handled by the caller. This
/// method deliberately has no fallible database work so memory cannot move
/// ahead of the transaction.
pub fn apply_committed_inventory_item_relocation_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
    moved_count: u32,
) -> bool {
    let TrainerSessionFieldsLikeCpp {
        hub: HubMut { core, catalogs, .. },
        inventory,
        ..
    } = host.split_trainer_mut();
    crate::InventoryCommittedRelocationCxLikeCpp::new(
        inventory,
        core.owned_inventory_access_like_cpp(),
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
    )
    .apply_committed_inventory_item_relocation_like_cpp(
        source_bag,
        source_slot,
        destination_bag,
        destination_slot,
        moved_count,
    )
}

/// Publish a committed C++ real swap after both database positions were
/// replaced in one transaction. Both positions must still contain the
/// pre-commit items; all runtime mutation is therefore infallible after
/// this preflight.
pub fn apply_committed_inventory_item_swap_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    source_bag: u8,
    source_slot: u8,
    destination_bag: u8,
    destination_slot: u8,
) -> bool {
    let TrainerSessionFieldsLikeCpp {
        hub: HubMut { core, catalogs, .. },
        inventory,
        ..
    } = host.split_trainer_mut();
    crate::InventoryCommittedSwapCxLikeCpp::new(
        inventory,
        core.owned_inventory_access_like_cpp(),
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
    )
    .apply_committed_inventory_item_swap_like_cpp(
        source_bag,
        source_slot,
        destination_bag,
        destination_slot,
    )
}

/// Remove a source item after its complete stack was merged into existing
/// destination stacks by a committed storage transaction.
pub fn apply_committed_inventory_item_removal_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    source_bag: u8,
    source_slot: u8,
    item_guid: ObjectGuid,
) -> bool {
    let TrainerSessionFieldsLikeCpp {
        hub: HubMut { core, catalogs, .. },
        inventory,
        ..
    } = host.split_trainer_mut();
    crate::InventoryCommittedRelocationCxLikeCpp::new(
        inventory,
        core.owned_inventory_access_like_cpp(),
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
    )
    .apply_committed_inventory_item_removal_like_cpp(source_bag, source_slot, item_guid)
}
