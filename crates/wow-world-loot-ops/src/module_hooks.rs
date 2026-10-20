// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hook ports of the loot handlers (#1444 H4).
//!
//! This crate never sees the public module API. A handler raises a plain
//! domain event through a provided host-trait method whose default does
//! nothing; the World overrides it and dispatches to the linked modules
//! (`wow-world` `session/module_hooks.rs`). Each port is asked first whether
//! a module wants the event, and is called only after the loot's durable
//! commit, with no loot claim, map guard, money guard or await in scope.

use wow_core::ObjectGuid;

/// `player.loot_item`: AzerothCore `OnPlayerLootItem`, the last step of
/// `Player::StoreLootItem` (`Player.cpp:13918`, `PlayerScript.h:431`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LootItemHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub item_id: u32,
    pub count: u32,
    /// The creature, game object or item looted (C++ `GetLootGUID()`).
    pub loot_source: ObjectGuid,
}

/// `player.loot_money`: AzerothCore `LootScript::OnLootMoney`
/// (`LootHandler.cpp:273`, `LootScript.h:44`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LootMoneyHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub loot_source: ObjectGuid,
    /// C++ `loot->gold`: the whole amount, before any group share.
    pub money: u64,
    /// What this player's committed balance received.
    pub share: u64,
}
