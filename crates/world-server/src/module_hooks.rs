// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The world-server adapter for the `world.*` module hooks (#1444 H2).
//!
//! Process-level hooks have no session to borrow, so they dispatch here, on
//! the composition root's own tasks: configuration, startup and shutdown from
//! `app.rs`, and the world tick from the canonical map loop after every map,
//! spawn-metadata and respawn-order guard of the tick was released. Every
//! entry asks `registry.handles(World)` first, so a zero-module server builds
//! no snapshot.
//!
//! Effects are a log line or C++ `World::SendWorldText` (`World.cpp:2982`):
//! one system message, split per line, to every player in the world.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wow_config::WorldConfigSet;

use tracing::{debug, info, warn};
use wow_core::ObjectGuid;
use wow_module_api::{
    ConfigLoadedSnapshot, HookFamily, HookOutcome, ModuleRegistry, WorldEffect,
    WorldShutdownSnapshot, WorldStartupSnapshot, WorldUpdateSnapshot,
};
use wow_packet::ServerPacket;
use wow_packet::packets::chat::{ChatMsg, ChatPkt};
use wow_world::session::directory::PlayerRegistry;

/// Above this, one world-tick dispatch is reported: module code runs on the
/// map loop, and a slow module delays every map.
pub(crate) const WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP: Duration = Duration::from_millis(5);

/// Where `AnnounceWorld` goes: the in-world players of the registry.
#[derive(Clone)]
pub(crate) struct WorldAnnouncerLikeCpp {
    players: Arc<PlayerRegistry>,
    /// C++ `RealmHandle::GetAddress()` of this realm, the `ChatPkt` virtual realm.
    virtual_realm: u32,
}

impl WorldAnnouncerLikeCpp {
    pub(crate) fn new(players: Arc<PlayerRegistry>, virtual_realm: u32) -> Self {
        Self {
            players,
            virtual_realm,
        }
    }

    /// The announcer of this realm: C++ `RealmHandle::GetAddress()`, region,
    /// site (battlegroup) and realm id, as the sessions build it
    /// (`connection_identity.rs` `virtual_realm_address`).
    pub(crate) fn for_realm_like_cpp(
        players: Arc<PlayerRegistry>,
        region: u8,
        site: u8,
        realm_id: u16,
    ) -> Self {
        let virtual_realm =
            (u32::from(region) << 24) | (u32::from(site) << 16) | u32::from(realm_id);
        Self::new(players, virtual_realm)
    }

    /// C++ `World::SendWorldText` for an already-localized text: every line as
    /// a `CHAT_MSG_SYSTEM` to every session whose player is in the world.
    /// Returns how many packets were queued.
    pub(crate) fn send_world_text_like_cpp(&self, text: &str) -> usize {
        let packets: Vec<Vec<u8>> = text
            .split('\n')
            .map(|line| {
                ChatPkt {
                    msg_type: ChatMsg::System,
                    language: 0,
                    sender_guid: ObjectGuid::EMPTY,
                    sender_name: String::new(),
                    target_guid: ObjectGuid::EMPTY,
                    target_name: String::new(),
                    prefix: String::new(),
                    channel: String::new(),
                    text: line.to_owned(),
                    virtual_realm: self.virtual_realm,
                }
                .to_bytes()
            })
            .collect();
        let mut queued = 0;
        for registration in self.players.in_world_registrations_like_cpp() {
            for packet in &packets {
                if self
                    .players
                    .try_send_current_packet(registration, packet.clone())
                    .is_ok()
                {
                    queued += 1;
                }
            }
        }
        queued
    }
}

/// The process-level module hooks.
#[derive(Clone)]
pub(crate) struct WorldModuleHooksLikeCpp {
    registry: Arc<ModuleRegistry>,
    announcer: Option<WorldAnnouncerLikeCpp>,
    /// When the previous `world.update` ran, so each dispatch reports the time
    /// since the previous one, as C++ `World::Update(diff)` does, even when
    /// the loop skipped a tick in between. Only the map loop's copy moves it.
    last_update: Option<Instant>,
}

impl WorldModuleHooksLikeCpp {
    pub(crate) fn new(registry: Arc<ModuleRegistry>) -> Self {
        Self {
            registry,
            announcer: None,
            last_update: None,
        }
    }

    /// Build the hooks once the world configuration is read, and raise
    /// `world.config_loaded` (AzerothCore `OnAfterConfigLoad(false)`,
    /// `World.cpp:300`, at the end of `World::LoadConfigSettings`).
    pub(crate) fn after_config_load_like_cpp(
        registry: Arc<ModuleRegistry>,
        configs: &WorldConfigSet,
        realm_id: u16,
    ) -> Self {
        let hooks = Self::new(registry);
        if hooks.wanted() {
            hooks.config_loaded_like_cpp(
                u32::from(realm_id),
                u32::from(crate::world_config_u8(
                    configs,
                    "CONFIG_MAX_PLAYER_LEVEL",
                    80,
                )),
                crate::world_config_u8(configs, "CONFIG_EXPANSION", 2),
            );
        }
        hooks
    }

    /// Bind the world announcer once the player registry exists.
    #[must_use]
    pub(crate) fn with_announcer(mut self, announcer: WorldAnnouncerLikeCpp) -> Self {
        self.announcer = Some(announcer);
        self
    }

    fn wanted(&self) -> bool {
        self.registry.handles(HookFamily::World)
    }

    /// `world.config_loaded`: AzerothCore `OnAfterConfigLoad(false)`
    /// (`World.cpp:300`). RustyCore has no configuration reload, so `reload`
    /// is always false.
    pub(crate) fn config_loaded_like_cpp(
        &self,
        realm_id: u32,
        max_player_level: u32,
        expansion: u8,
    ) {
        if !self.wanted() {
            return;
        }
        let snapshot = ConfigLoadedSnapshot::new(false, realm_id, max_player_level, expansion);
        let outcome = self.registry.dispatch_config_loaded(&snapshot);
        self.apply_like_cpp("world.config_loaded", outcome);
    }

    /// `world.startup`: AzerothCore `OnStartup` (worldserver `Main.cpp:395`).
    pub(crate) fn startup_like_cpp(&self, server_version: &str, realm_id: u32) {
        if !self.wanted() {
            return;
        }
        let module_count = self.registry.descriptors().len();
        let snapshot = WorldStartupSnapshot::new(server_version, realm_id, module_count);
        let outcome = self.registry.dispatch_world_startup(&snapshot);
        self.apply_like_cpp("world.startup", outcome);
    }

    /// `world.shutdown`: AzerothCore `OnShutdown` (worldserver `Main.cpp:419`).
    pub(crate) fn shutdown_like_cpp(&self, exit_code: i32) {
        if !self.wanted() {
            return;
        }
        let outcome = self
            .registry
            .dispatch_world_shutdown(&WorldShutdownSnapshot::new(exit_code));
        self.apply_like_cpp("world.shutdown", outcome);
    }

    /// `world.update`: AzerothCore `OnWorldUpdate(diff)` (`World.cpp:1342`),
    /// once per completed world tick. `now` is the tick's game time and
    /// `tick_diff_ms` the tick's own diff, used for the first dispatch only;
    /// later ones report the time since the previous dispatch. Returns how
    /// long the dispatch took.
    pub(crate) fn update_like_cpp(
        &mut self,
        now: Instant,
        tick_diff_ms: u32,
        tick: u64,
    ) -> Option<Duration> {
        if !self.wanted() {
            return None;
        }
        let diff_ms = self.last_update.replace(now).map_or(tick_diff_ms, |last| {
            now.saturating_duration_since(last)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32
        });
        let started = Instant::now();
        let outcome = self
            .registry
            .dispatch_world_update(&WorldUpdateSnapshot::new(diff_ms, tick));
        self.apply_like_cpp("world.update", outcome);
        let elapsed = started.elapsed();
        if elapsed > WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP {
            warn!(
                tick,
                elapsed_us = elapsed.as_micros() as u64,
                budget_us = WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP.as_micros() as u64,
                "linked world.update hooks exceeded their per-tick budget"
            );
        }
        Some(elapsed)
    }

    /// Apply a validated `world.*` batch, all or nothing.
    fn apply_like_cpp(&self, hook: &'static str, outcome: HookOutcome<WorldEffect>) {
        for module in &outcome.disabled {
            warn!(%module, hook, "linked module panicked; disabled for the rest of the process");
        }
        let effects = match outcome.effects {
            Ok(effects) => effects,
            Err(error) => {
                warn!(%error, hook, "module effect batch rejected; no effect applied");
                return;
            }
        };
        for (module, effect) in effects.iter() {
            match effect {
                WorldEffect::Log { text } => info!(%module, hook, "{text}"),
                WorldEffect::AnnounceWorld { text } => match &self.announcer {
                    Some(announcer) => {
                        let queued = announcer.send_world_text_like_cpp(text);
                        debug!(%module, hook, queued, "module world announcement");
                    }
                    None => {
                        debug!(%module, hook, "module world announcement before any player can be in the world");
                    }
                },
                _ => warn!(%module, hook, "unsupported world effect ignored"),
            }
        }
    }
}

#[cfg(test)]
#[path = "module_hooks_tests.rs"]
mod tests;
