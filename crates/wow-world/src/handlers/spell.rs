// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast handlers — CMSG_CAST_SPELL, CMSG_CANCEL_CAST, CMSG_CANCEL_CHANNELLING.
//!
//! Normal requests decode/adapt into `player_cast`; canonical state and
//! publication adapters live under `session/player_cast`. Immediate and queued
//! requests share preparation, while the existing driver consumes timed casts.
//! Other represented spell/item handlers retain their explicit operation paths.
//! Reference: Classic Game/Handlers/SpellHandler.cpp, Player.cpp and Spell.cpp.
//!
//! `#1263 F5 remaining families`: the four registrations that lived here
//! (`CMSG_CAST_SPELL`, `CMSG_OPEN_ITEM`, `CMSG_SELF_RES`, `CMSG_SPELL_CLICK`)
//! moved unchanged to the application crate's `crate::spell_handlers` area
//! registrar, which already owns the `SpellHandler.cpp` cancellation surface.
//! `crates/wow-world/src/session/spell_handler_contexts.rs` lends that registrar
//! the same `WorldSession` operations and catalog destructuring the legacy
//! closures used.

use std::collections::HashMap;

use tracing::{debug, warn};

use wow_constants::{BagFamilyMask, ClientOpcodes, InventoryResult, ItemFlags, TypeId};
use wow_core::ObjectGuid;
use wow_data::{DISABLE_TYPE_SPELL, DisableWorldObjectRefLikeCpp};
use wow_entities::INVENTORY_SLOT_BAG_0;

use wow_loot::{
    LootConditionRowLikeCpp, condition_compare_values_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
};
use wow_packet::ClientPacket;
use wow_packet::packets::item::{ItemExpirePurchaseRefund, ItemInstance};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags, LootItemData, LootResponse,
};
use wow_packet::packets::spell::{
    CancelModSpeedNoControlAuras, CastSpellRequest, OpenItem, SelfRes, SpellClick,
};

use crate::session::{
    AreaTriggerCatalogsLikeCpp, RepresentedPendingSpellCastRequestLikeCpp, WorldSession,
};
use wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;

mod ops_1;
mod ops_2;
mod state;
#[cfg(test)]
mod test_shims;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

// ── Handler implementations ───────────────────────────────────────

#[cfg(test)]
#[path = "../../unit_tests/handlers/spell/tests/mod.rs"]
mod tests;
