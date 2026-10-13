// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory request contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

pub(crate) use wow_world_inventory::MAX_EQUIPMENT_SET_INDEX_LIKE_CPP;
pub(crate) use wow_world_inventory::RepresentedAutoUnequipOffhandReasonLikeCpp;

pub(crate) use wow_world_inventory::DirectInventoryStorageOverlayLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_interaction::VendorBuyItemTestOverrideLikeCpp;
