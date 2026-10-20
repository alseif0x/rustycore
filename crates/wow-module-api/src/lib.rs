// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Public source API for trusted Rust modules compiled into the server.
//!
//! Source API 1 (#228) carried one vertical, `player.login ->
//! SendSystemMessageSelf`. It mirrors C++ `ScriptMgr::OnPlayerLogin`
//! (`ScriptMgr.cpp:2052-2055`), invoked after a completed login
//! (`CharacterHandler.cpp:1452`), against the `PlayerScript` hook shape
//! (`ScriptMgr.h:764`).
//!
//! Source API 2 (#1445) keeps that contract unchanged and adds the registry by
//! capability: a module registers a [`ModuleHooks`] set with one entry per
//! hook family it implements, modelled on AzerothCore's script classes
//! (`src/server/game/Scripting/ScriptDefines/*.h`). Dispatch stays in
//! `ModuleId` order, validates each batch as a whole, and isolates a panicking
//! callback by disabling its module for the rest of the process.
//!
//! What a module receives is an immutable snapshot; what it returns is a
//! validated batch of typed effects. It never sees a `WorldSession`, a
//! `Player`, a `Map`, a database pool, a packet writer or a raw pointer, and
//! this crate depends on no runtime, transport, storage or protocol crate.
//!
//! Nothing here promises a stable native ABI or hot reload. Modules are
//! compiled in, the types are expected to evolve, and official internal
//! scripts keep their own separate dispatch.

mod config;
mod decision;
mod effect;
mod hook;
mod player;
mod registry;
mod world;

pub use config::{ModuleConfig, ModuleConfigError, ModuleConfigValue};
pub use decision::ChatDecision;
pub use effect::{
    EffectBatch, HookOutcome, ModuleEffectError, NotificationEffect, PlayerLoginEffect,
    PlayerLoginEffects, ScopedEffects,
};
pub use hook::{PlayerLoginModule, PlayerLoginSnapshot};
pub use player::{
    CreatureKillSnapshot, LogoutReason, PlayerLevelChangedSnapshot, PlayerLogoutSnapshot,
    PlayerModule, PlayerMoneyChangedSnapshot, QuestAcceptedSnapshot, QuestRewardedSnapshot,
    SpellCastSnapshot,
};
pub use registry::{
    HookFamily, ModuleDescriptor, ModuleHooks, ModuleId, ModuleRegistrationError, ModuleRegistry,
    ModuleVersion, module_callback_in_progress,
};
pub use world::{
    ConfigLoadedSnapshot, WorldEffect, WorldEffects, WorldModule, WorldShutdownSnapshot,
    WorldStartupSnapshot, WorldUpdateSnapshot,
};

/// The player identity every snapshot carries. Re-exported so a module names
/// it through this crate alone: `tools/modules/compose.py` refuses a module
/// with a normal dependency on any other server crate, `wow-core` included.
pub use wow_core::ObjectGuid;

/// The source API this server provides. `tools/modules/compose.py` records it
/// in `modules.lock.toml` and its tests assert it matches this constant.
pub const SOURCE_API: &str = "2";

/// Every source API a module may declare. API 2 is additive over API 1, so a
/// module written against API 1 composes and runs unchanged.
pub const COMPATIBLE_SOURCE_APIS: &[&str] = &["1", "2"];

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
