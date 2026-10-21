// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Example Milestones — the #1444 example module.
//!
//! It exercises the hook families of source API 2 together:
//! - `player.login`: a greeting to the player who logged in;
//! - `player.level_changed`: `level_message` on every `level_step`-th level,
//!   and optionally a world announcement. A player hook can only message its
//!   own player, so the announcement is queued and the next `world.update`
//!   sends it;
//! - `player.quest_rewarded`: a per-player count of quests turned in this
//!   session, reported to the player;
//! - `player.creature_kill`: a per-player kill counter, a self message on
//!   every `kill_milestone`-th kill, and a server log line for it (queued,
//!   like the announcement, and written by the next `world.update`);
//! - `player.logout`: forgets the player's counters;
//! - `chat.message`: a pre-decision that denies any message containing one
//!   of `blocked_words`;
//! - `world.startup` and `world.update`: a log line, and the queued
//!   announcements;
//! - `creature.died`: a process-wide death counter, logged on shutdown.
//!
//! The counters live in memory only: they are lost on restart. Durable module
//! state is #583 work. The module declares exactly the `player.*` hooks it
//! implements, so the server builds no snapshot for the others, and it
//! registers its `chat.*` family only when there is a word to block.
//!
//! This is in-process code compiled into the server with full privileges.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use wow_module_api::{
    ChatDecision, ChatMessageSnapshot, ChatModule, CreatureDiedSnapshot, CreatureKillSnapshot,
    CreatureModule, ModuleConfig, ModuleDescriptor, ModuleHooks, ModuleRegistrationError,
    ModuleRegistry, ModuleVersion, ObjectGuid, PlayerHook, PlayerLevelChangedSnapshot,
    PlayerLoginModule, PlayerLoginSnapshot, PlayerLogoutSnapshot, PlayerModule,
    QuestRewardedSnapshot, ScopedEffects, WorldEffects, WorldModule, WorldShutdownSnapshot,
    WorldStartupSnapshot, WorldUpdateSnapshot,
};

/// The placeholder `level_message` replaces with the new level.
const LEVEL_PLACEHOLDER: &str = "{level}";
/// The default `level_message`.
const DEFAULT_LEVEL_MESSAGE: &str = "Milestone reached: level {level}!";
/// Longest `level_message` accepted, leaving room for the level within the
/// 255-byte system message limit.
const MAX_LEVEL_MESSAGE_BYTES: usize = 200;

/// The validated configuration, resolved once at registration.
#[derive(Clone, Debug)]
struct Settings {
    level_step: u8,
    level_message: String,
    kill_milestone: u32,
    announce_levels: bool,
    /// Lower-cased whole words; empty when nothing is blocked.
    blocked_words: BTreeSet<String>,
}

/// Per-player counters, forgotten when the player's logout is confirmed.
#[derive(Clone, Copy, Debug, Default)]
struct PlayerCounters {
    kills: u32,
    quests_rewarded: u32,
}

/// The module's in-memory state, shared by its families.
pub struct Milestones {
    settings: Settings,
    players: Mutex<HashMap<ObjectGuid, PlayerCounters>>,
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
            players: Mutex::new(HashMap::new()),
            pending_announcements: Mutex::new(Vec::new()),
            pending_logs: Mutex::new(Vec::new()),
            creature_deaths: AtomicU64::new(0),
        }
    }

    fn counters_of(&self, player: ObjectGuid) -> PlayerCounters {
        self.players.lock().map_or_else(
            |_| PlayerCounters::default(),
            |players| players.get(&player).copied().unwrap_or_default(),
        )
    }

    /// The kill count recorded for `player` (tests and diagnostics).
    #[must_use]
    pub fn kills_of(&self, player: ObjectGuid) -> u32 {
        self.counters_of(player).kills
    }

    /// The quests `player` turned in this session (tests and diagnostics).
    #[must_use]
    pub fn quests_rewarded_of(&self, player: ObjectGuid) -> u32 {
        self.counters_of(player).quests_rewarded
    }

    /// The creature deaths observed since the process started.
    #[must_use]
    pub fn creature_deaths(&self) -> u64 {
        self.creature_deaths.load(Ordering::Relaxed)
    }

    /// Whether `text` contains a blocked word. Words are compared whole and
    /// case-insensitively, so blocking `ass` does not block `class`.
    fn is_blocked(&self, text: &str) -> bool {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .any(|word| self.settings.blocked_words.contains(&word.to_lowercase()))
    }
}

impl PlayerLoginModule for Milestones {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        let welcome = if snapshot.first_login {
            "Welcome"
        } else {
            "Welcome back"
        };
        effects.send_system_message_self(format!(
            "{welcome}, {}! A milestone every {} kills.",
            snapshot.name, self.settings.kill_milestone
        ));
    }
}

impl PlayerModule for Milestones {
    fn enabled_hooks(&self) -> &'static [PlayerHook] {
        &[
            PlayerHook::LevelChanged,
            PlayerHook::QuestRewarded,
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
        effects.send_system_message_self(
            self.settings
                .level_message
                .replace(LEVEL_PLACEHOLDER, &level.to_string()),
        );
        if self.settings.announce_levels
            && let Ok(mut pending) = self.pending_announcements.lock()
        {
            pending.push(format!("A player has reached level {level}."));
        }
    }

    fn on_quest_rewarded(&self, snapshot: &QuestRewardedSnapshot, effects: &mut ScopedEffects<'_>) {
        let Ok(mut players) = self.players.lock() else {
            return;
        };
        let counters = players.entry(snapshot.guid).or_default();
        counters.quests_rewarded = counters.quests_rewarded.saturating_add(1);
        effects.send_system_message_self(format!(
            "Quest {} done: {} this session.",
            snapshot.quest_id, counters.quests_rewarded
        ));
    }

    fn on_creature_kill(&self, snapshot: &CreatureKillSnapshot, effects: &mut ScopedEffects<'_>) {
        let Ok(mut players) = self.players.lock() else {
            return;
        };
        let counters = players.entry(snapshot.guid).or_default();
        counters.kills = counters.kills.saturating_add(1);
        let count = counters.kills;
        if count % self.settings.kill_milestone == 0 {
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
        if let Ok(mut players) = self.players.lock() {
            players.remove(&snapshot.guid);
        }
    }
}

impl ChatModule for Milestones {
    fn on_chat_message(&self, snapshot: &ChatMessageSnapshot) -> ChatDecision {
        if self.is_blocked(&snapshot.text) {
            ChatDecision::Deny
        } else {
            ChatDecision::Allow
        }
    }
}

impl WorldModule for Milestones {
    fn on_startup(&self, _snapshot: &WorldStartupSnapshot, effects: &mut WorldEffects<'_>) {
        effects.log(format!(
            "milestones every {} levels and {} kills, {} blocked words",
            self.settings.level_step,
            self.settings.kill_milestone,
            self.settings.blocked_words.len()
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
    let level_step = config.integer("level_step")?.unwrap_or(1);
    let level_message = config
        .text("level_message")?
        .unwrap_or_else(|| DEFAULT_LEVEL_MESSAGE.to_owned());
    let kill_milestone = config.integer("kill_milestone")?.unwrap_or(100);
    let announce_levels = config.bool("announce_levels")?.unwrap_or(false);
    let blocked_words = config.text("blocked_words")?.unwrap_or_default();

    let level_step = u8::try_from(level_step)
        .ok()
        .filter(|step| *step > 0)
        .ok_or_else(|| config.invalid("level_step", "must be between 1 and 255"))?;
    if level_message.trim().is_empty()
        || level_message.len() > MAX_LEVEL_MESSAGE_BYTES
        || level_message.chars().any(char::is_control)
    {
        return Err(config
            .invalid(
                "level_message",
                format!("must be 1-{MAX_LEVEL_MESSAGE_BYTES} bytes without control characters"),
            )
            .into());
    }
    let kill_milestone = u32::try_from(kill_milestone)
        .ok()
        .filter(|step| *step > 0)
        .ok_or_else(|| config.invalid("kill_milestone", "must be a positive 32-bit integer"))?;
    let blocked_words = parse_blocked_words(&blocked_words)
        .map_err(|reason| config.invalid("blocked_words", reason))?;
    // Any key the module did not read is an operator typo, not a silent no-op.
    config.finish()?;
    if !enabled {
        return Ok(None);
    }
    Ok(Some(Arc::new(Milestones::new(Settings {
        level_step,
        level_message,
        kill_milestone,
        announce_levels,
        blocked_words,
    }))))
}

/// `blocked_words` is a comma-separated list of single words.
fn parse_blocked_words(raw: &str) -> Result<BTreeSet<String>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(|word| {
            if word.chars().all(char::is_alphanumeric) {
                Ok(word.to_lowercase())
            } else {
                Err(format!(
                    "`{word}` is not a single word (letters and digits only, separated by commas)"
                ))
            }
        })
        .collect()
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
    let mut hooks = ModuleHooks::new()
        .player_login(module.clone())
        .player(module.clone())
        .world(module.clone());
    // Ask for every chat message only when there is something to block.
    if !module.settings.blocked_words.is_empty() {
        hooks = hooks.chat(module.clone());
    }
    registry.register_module(descriptor, hooks.creature(module))
}

/// Build the module from a configuration without registering it (tests).
#[doc(hidden)]
pub fn build_for_test(
    config: ModuleConfig,
) -> Result<Option<Arc<Milestones>>, ModuleRegistrationError> {
    build(config)
}
