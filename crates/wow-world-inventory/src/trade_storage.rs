// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Storage planning of a completed trade (#38).
//!
//! The plan consumes a disposable snapshot of each participant's native and
//! runtime inventory storage. The canonical Players remain the application
//! owners for the eventual trade transition.

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;
use wow_entities::{
    Item, ItemLimitCategoryTemplate, ItemStorageTemplate, Player, PlayerInventoryRuntime,
    PlayerInventoryStorage,
};

mod simulation;

/// Where one incoming item ends up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeStoreActionLikeCpp {
    /// `_StoreItem` into an existing stack.
    Merge {
        target_guid: ObjectGuid,
        added: u32,
        new_count: u32,
    },
    /// The traded item itself is stored with `count` at an empty position.
    Store { bag: u8, slot: u8, count: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeStorePlanLikeCpp {
    pub item_guid: ObjectGuid,
    pub actions: Vec<TradeStoreActionLikeCpp>,
}

impl TradeStorePlanLikeCpp {
    /// The item merged completely and its own row goes away.
    #[must_use]
    pub fn fully_merged(&self) -> bool {
        !self
            .actions
            .iter()
            .any(|action| matches!(action, TradeStoreActionLikeCpp::Store { .. }))
    }
}

/// Catalog lookups the plan needs.
pub struct TradeStoreCatalogsLikeCpp<'a> {
    pub template: &'a dyn Fn(u32) -> Option<ItemStorageTemplate>,
    pub limit_category: &'a dyn Fn(u32) -> Option<ItemLimitCategoryTemplate>,
}

/// Disposable native/runtime storage used by one receiving-side simulation.
///
/// Only the inventory substates needed for planning are copied. The snapshot
/// has no path back to the canonical Player.
pub struct TradeStorageSnapshot {
    owner_guid: ObjectGuid,
    backpack_slots: u8,
    inventory: PlayerInventoryStorage,
    inventory_runtime: PlayerInventoryRuntime,
}

impl TradeStorageSnapshot {
    /// Capture only the identity, backpack capacity and inventory substates
    /// needed by trade storage planning.
    pub fn capture(player: &Player) -> Self {
        Self {
            owner_guid: player.guid(),
            backpack_slots: player.inventory_slot_count(),
            inventory: player.inventory().clone(),
            inventory_runtime: player.inventory_runtime_like_cpp().clone(),
        }
    }
}

/// Plan the represented receiving inventory: `outgoing` leave first and
/// `incoming` are stored in order. `Err((result, entry))` identifies the first
/// incoming item whose placement fails.
pub fn plan_trade_receive_like_cpp(
    receiver: &Player,
    outgoing: &[ObjectGuid],
    incoming: &[Item],
    catalogs: &TradeStoreCatalogsLikeCpp<'_>,
) -> Result<Vec<TradeStorePlanLikeCpp>, (InventoryResult, u32)> {
    plan_trade_receive_snapshot_like_cpp(
        TradeStorageSnapshot::capture(receiver),
        outgoing,
        incoming,
        catalogs,
    )
}

/// Consume one receiving-side inventory snapshot to plan all incoming items.
pub fn plan_trade_receive_snapshot_like_cpp(
    snapshot: TradeStorageSnapshot,
    outgoing: &[ObjectGuid],
    incoming: &[Item],
    catalogs: &TradeStoreCatalogsLikeCpp<'_>,
) -> Result<Vec<TradeStorePlanLikeCpp>, (InventoryResult, u32)> {
    simulation::plan_trade_receive_snapshot_like_cpp(snapshot, outgoing, incoming, catalogs)
}

#[cfg(test)]
#[path = "../unit_tests/trade_storage_tests.rs"]
mod tests;
