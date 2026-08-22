// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Public source API for trusted Rust modules compiled into the server.
//!
//! This crate is deliberately small: it exists only to carry the one vertical
//! issue #228 earned, `player.login -> SendSystemMessageSelf`. It mirrors C++
//! `ScriptMgr::OnPlayerLogin` (`ScriptMgr.cpp:2052-2055`), invoked after a
//! completed login (`CharacterHandler.cpp:1452`), against the `PlayerScript`
//! hook shape (`ScriptMgr.h:764`).
//!
//! What a module receives is an immutable snapshot; what it returns is a
//! validated batch of typed effects. It never sees a `WorldSession`, a
//! `Player`, a `Map`, a database pool, a packet writer or a raw pointer, and
//! this crate depends on no runtime, transport, storage or protocol crate.
//!
//! Nothing here promises a stable native ABI or hot reload. Modules are
//! compiled in, the types are expected to evolve, and official internal
//! scripts keep their own separate dispatch.

mod effect;
mod hook;
mod registry;

pub use effect::{ModuleEffectError, PlayerLoginEffect, PlayerLoginEffects, ScopedEffects};
pub use hook::{PlayerLoginModule, PlayerLoginSnapshot};
pub use registry::{
    ModuleDescriptor, ModuleId, ModuleRegistrationError, ModuleRegistry, ModuleVersion,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
