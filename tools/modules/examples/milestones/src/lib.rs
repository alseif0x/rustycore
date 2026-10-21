// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Example Milestones — the #1444 example module.
//!
//! It exercises the hook families of source API 2 together:
//! - `player.level_changed`: a self message on every `level_step`-th level,
//!   and optionally a world announcement. A player hook can only message its
//!   own player, so the announcement is queued and the next `world.update`
//!   sends it;
//! - `player.creature_kill`: a per-player kill counter, a self message on
//!   every `kill_step`-th kill, and a server log line for it (queued, like the
//!   announcement, and written by the next `world.update`);
//! - `player.logout`: forgets the player's counter;
//! - `world.startup` and `world.update`: a log line, and the queued
//!   announcements;
//! - `creature.died`: a process-wide death counter, logged on shutdown.
//!
//! The counters live in memory only: they are lost on restart. Durable module
//! state is #583 work. The module declares exactly the `player.*` hooks it
//! implements, so the server builds no snapshot for the others.
//!
//! This is in-process code compiled into the server with full privileges.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use wow_module_api::{
    CreatureDiedSnapshot, CreatureKillSnapshot, CreatureModule, ModuleConfig, ModuleDescriptor,
    ModuleHooks, ModuleRegistrationError, ModuleRegistry, ModuleVersion, ObjectGuid, PlayerHook,
    PlayerLevelChangedSnapshot, PlayerLogoutSnapshot, PlayerModule, ScopedEffects, WorldEffects,
    WorldModule, WorldShutdownSnapshot, WorldStartupSnapshot, WorldUpdateSnapshot,
};

/// The validated configuration, resolved once at registration.
#[derive(Clone, Copy, Debug)]
struct Settings {
    level_step: u8,
    kill_step: u32,
    announce_levels: bool,
}

/// The module's in-memory state, shared by its three families.
pub struct Milestones {
    settings: Settings,
    kills: Mutex<HashMap<ObjectGuid, u32>>,
    /// Level milestones waiting for the next `world.update` announcement.
    pending_announcements: Mutex<Vec<String>>,
    /// Kill milestones waiting for the next `world.update` log line.
    pending_logs: Mutex<Vec<String>>,
    creature_deaths: AtomicU64,
}

impl Milestones {
    fn new(settings: Settings) -> Self {
        Self {
            settings,
            kills: Mutex::new(HashMap::new()),
            pending_announcements: Mutex::new(Vec::new()),
            pending_logs: Mutex::new(Vec::new()),
            creature_deaths: AtomicU64::new(0),
        }
    }

    /// The kill count recorded for `player` (tests and diagnostics).
    #[must_use]
    pub fn kills_of(&self, player: ObjectGuid) -> u32 {
        self.kills
            .lock()
            .map_or(0, |kills| kills.get(&player).copied().unwrap_or(0))
    }

    /// The creature deaths observed since the process started.
    #[must_use]
    pub fn creature_deaths(&self) -> u64 {
        self.creature_deaths.load(Ordering::Relaxed)
    }
}

impl PlayerModule for Milestones {
    fn enabled_hooks(&self) -> &'static [PlayerHook] {
        &[
            PlayerHook::LevelChanged,
            PlayerHook::CreatureKill,
            PlayerHook::Logout,
        ]
    }

    fn on_level_changed(
        &self,
        snapshot: &PlayerLevelChangedSnapshot,
        effects: &mut ScopedEffects<'_>,
    ) {
        let level = snapshot.new_level;
        if level == 0 || level % self.settings.level_step != 0 {
            return;
        }
        effects.send_system_message_self(format!("Milestone reached: level {level}!"));
        if self.settings.announce_levels
            && let Ok(mut pending) = self.pending_announcements.lock()
        {
            pending.push(format!("A player has reached level {level}."));
        }
    }

    fn on_creature_kill(&self, snapshot: &CreatureKillSnapshot, effects: &mut ScopedEffects<'_>) {
        let Ok(mut kills) = self.kills.lock() else {
            return;
        };
        let count = kills.entry(snapshot.guid).or_insert(0);
        *count += 1;
        if *count % self.settings.kill_step == 0 {
            effects.send_system_message_self(format!("Milestone reached: {count} kills!"));
            if let Ok(mut logs) = self.pending_logs.lock() {
                logs.push(format!(
                    "player {:?} reached {count} kills (last: creature entry {})",
                    snapshot.guid, snapshot.creature_entry
                ));
            }
        }
    }

    fn on_logout(&self, snapshot: &PlayerLogoutSnapshot) {
        if let Ok(mut kills) = self.kills.lock() {
            kills.remove(&snapshot.guid);
        }
    }
}

impl WorldModule for Milestones {
    fn on_startup(&self, _snapshot: &WorldStartupSnapshot, effects: &mut WorldEffects<'_>) {
        effects.log(format!(
            "milestones every {} levels and {} kills",
            self.settings.level_step, self.settings.kill_step
        ));
    }

    fn on_update(&self, _snapshot: &WorldUpdateSnapshot, effects: &mut WorldEffects<'_>) {
        if let Ok(mut logs) = self.pending_logs.lock() {
            for text in logs.drain(..) {
                effects.log(text);
            }
        }
        if let Ok(mut pending) = self.pending_announcements.lock() {
            for text in pending.drain(..) {
                effects.announce_world(text);
            }
        }
    }

    fn on_shutdown(&self, _snapshot: &WorldShutdownSnapshot, effects: &mut WorldEffects<'_>) {
        effects.log(format!(
            "{} creature deaths observed",
            self.creature_deaths()
        ));
    }
}

impl CreatureModule for Milestones {
    fn on_died(&self, _snapshot: &CreatureDiedSnapshot, _effects: &mut WorldEffects<'_>) {
        self.creature_deaths.fetch_add(1, Ordering::Relaxed);
    }
}

/// Registrar called by the generated compositor. Keep the name in sync with
/// `registrar` in `module.toml`.
pub fn register(
    registry: &mut ModuleRegistry,
    config: ModuleConfig,
) -> Result<(), ModuleRegistrationError> {
    let Some(module) = build(config)? else {
        return Ok(());
    };
    register_with(registry, module)
}

/// Validate the configuration. `None` when the operator disabled the module.
fn build(mut config: ModuleConfig) -> Result<Option<Arc<Milestones>>, ModuleRegistrationError> {
    let enabled = config.bool("enabled")?.unwrap_or(true);
    let level_step = config.integer("level_step")?.unwrap_or(10);
    let kill_step = config.integer("kill_step")?.unwrap_or(100);
    let announce_levels = config.bool("announce_levels")?.unwrap_or(false);
    let level_step = u8::try_from(level_step)
        .ok()
        .filter(|step| *step > 0)
        .ok_or_else(|| config.invalid("level_step", "must be between 1 and 255"))?;
    let kill_step = u32::try_from(kill_step)
        .ok()
        .filter(|step| *step > 0)
        .ok_or_else(|| config.invalid("kill_step", "must be a positive 32-bit integer"))?;
    // Any key the module did not read is an operator typo, not a silent no-op.
    config.finish()?;
    if !enabled {
        return Ok(None);
    }
    Ok(Some(Arc::new(Milestones::new(Settings {
        level_step,
        kill_step,
        announce_levels,
    }))))
}

/// Register an already built module value (tests keep a handle on it).
pub fn register_with(
    registry: &mut ModuleRegistry,
    module: Arc<Milestones>,
) -> Result<(), ModuleRegistrationError> {
    let descriptor = ModuleDescriptor::new(
        "example.milestones",
        ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Example Milestones",
    )?;
    registry.register_module(
        descriptor,
        ModuleHooks::new()
            .player(module.clone())
            .world(module.clone())
            .creature(module),
    )
}

/// Build the module from a configuration without registering it (tests).
pub fn build_for_test(config: ModuleConfig) -> Result<Option<Arc<Milestones>>, ModuleRegistrationError> {
    build(config)
}
