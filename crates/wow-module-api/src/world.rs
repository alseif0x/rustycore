// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `world.*` hook family (source API 2, #1444 H2).
//!
//! Process-level notifications, modelled on AzerothCore `WorldScript`
//! (`src/server/game/Scripting/ScriptDefines/WorldScript.h`). They run on the
//! world server's own tasks, never inside a session and never under the map,
//! respawn-order or any session lock. Their effects are process-wide: a log
//! line, or a system message to every player in the world.

use std::fmt;

use crate::effect::{EffectBatch, ModuleEffectError, ValidatedEffect, validate_system_message};
use crate::registry::ModuleId;

/// An effect a `world.*` hook may request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WorldEffect {
    /// Write one line to the server log, attributed to the module.
    Log { text: String },
    /// C++ `World::SendWorldText`: a system message to every player in the
    /// world.
    AnnounceWorld { text: String },
}

impl ValidatedEffect for WorldEffect {
    fn validate(&self, module: &ModuleId) -> Result<(), ModuleEffectError> {
        match self {
            Self::Log { text } | Self::AnnounceWorld { text } => {
                validate_system_message(module, text)
            }
        }
    }
}

/// The handle one module writes world effects through.
pub struct WorldEffects<'a> {
    batch: &'a mut EffectBatch<WorldEffect>,
    module: ModuleId,
}

impl<'a> WorldEffects<'a> {
    pub(crate) fn new(batch: &'a mut EffectBatch<WorldEffect>, module: ModuleId) -> Self {
        Self { batch, module }
    }

    /// Queue a log line.
    pub fn log(&mut self, text: impl Into<String>) {
        self.batch
            .push(self.module.clone(), WorldEffect::Log { text: text.into() });
    }

    /// Queue a system message to every player in the world.
    pub fn announce_world(&mut self, text: impl Into<String>) {
        self.batch.push(
            self.module.clone(),
            WorldEffect::AnnounceWorld { text: text.into() },
        );
    }
}

impl fmt::Debug for WorldEffects<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldEffects")
            .field("module", &self.module)
            .finish_non_exhaustive()
    }
}

/// `world.config_loaded`: the world configuration was read.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ConfigLoadedSnapshot {
    /// C++ `reload`. RustyCore has no configuration reload yet, so this is
    /// always `false` today.
    pub reload: bool,
    pub realm_id: u32,
    /// C++ `CONFIG_MAX_PLAYER_LEVEL`.
    pub max_player_level: u32,
    /// C++ `CONFIG_EXPANSION`.
    pub expansion: u8,
}

impl ConfigLoadedSnapshot {
    #[must_use]
    pub fn new(reload: bool, realm_id: u32, max_player_level: u32, expansion: u8) -> Self {
        Self {
            reload,
            realm_id,
            max_player_level,
            expansion,
        }
    }
}

/// `world.startup`: the world finished starting and accepts players.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct WorldStartupSnapshot {
    pub server_version: String,
    pub realm_id: u32,
    /// How many linked modules this server registered.
    pub module_count: usize,
}

impl WorldStartupSnapshot {
    #[must_use]
    pub fn new(server_version: impl Into<String>, realm_id: u32, module_count: usize) -> Self {
        Self {
            server_version: server_version.into(),
            realm_id,
            module_count,
        }
    }
}

/// `world.shutdown`: the world is stopping; sessions are already flushed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct WorldShutdownSnapshot {
    /// The process exit code the shutdown will report.
    pub exit_code: i32,
}

impl WorldShutdownSnapshot {
    #[must_use]
    pub fn new(exit_code: i32) -> Self {
        Self { exit_code }
    }
}

/// `world.update`: one world tick completed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct WorldUpdateSnapshot {
    /// C++ `diff`: milliseconds since the previous tick.
    pub diff_ms: u32,
    /// The tick number, counting from 1.
    pub tick: u64,
}

impl WorldUpdateSnapshot {
    #[must_use]
    pub fn new(diff_ms: u32, tick: u64) -> Self {
        Self { diff_ms, tick }
    }
}

/// The `world.*` family. Every method has an empty default.
pub trait WorldModule: Send + Sync {
    /// AzerothCore `WorldScript::OnAfterConfigLoad(bool reload)`
    /// (`WorldScript.h:53`, `World.cpp:300`).
    fn on_config_loaded(&self, _snapshot: &ConfigLoadedSnapshot, _effects: &mut WorldEffects<'_>) {}

    /// AzerothCore `WorldScript::OnStartup` (`WorldScript.h:74`, worldserver
    /// `Main.cpp:395`).
    fn on_startup(&self, _snapshot: &WorldStartupSnapshot, _effects: &mut WorldEffects<'_>) {}

    /// AzerothCore `WorldScript::OnShutdown` (`WorldScript.h:77`, worldserver
    /// `Main.cpp:419`).
    ///
    /// Deviation from C++: AzerothCore calls `OnShutdown` before
    /// `KickAll`/`UpdateSessions(1)`/`StopNetwork`. RustyCore calls it later,
    /// after the session flush and the respawn-writer drain and before the
    /// realm is marked offline, so no module runs while sessions are still
    /// being finalized; an `announce_world` here may reach no player.
    fn on_shutdown(&self, _snapshot: &WorldShutdownSnapshot, _effects: &mut WorldEffects<'_>) {}

    /// AzerothCore `WorldScript::OnUpdate(uint32 diff)` (`WorldScript.h:71`,
    /// `World.cpp:1342`). Runs every tick: keep it cheap.
    fn on_update(&self, _snapshot: &WorldUpdateSnapshot, _effects: &mut WorldEffects<'_>) {}
}
