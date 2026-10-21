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
//! one system message per non-empty line, to every player in the world.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};
use wow_config::WorldConfigSet;
use wow_core::ObjectGuid;
use wow_module_api::{
    ConfigLoadedSnapshot, CreatureAddedToWorldSnapshot, CreatureDiedSnapshot, CreatureLocation,
    HookFamily, HookOutcome, ModuleId, ModuleRegistry, WorldEffect, WorldShutdownSnapshot,
    WorldStartupSnapshot, WorldUpdateSnapshot,
};
use wow_packet::ServerPacket;
use wow_packet::packets::chat::{ChatMsg, ChatPkt};
use wow_world::session::directory::PlayerRegistry;
use wow_world::session::{CreatureModuleEventKindLikeCpp, CreatureModuleEventLikeCpp};

/// Above this, one module's `world.update` callback, or the effects of one
/// dispatch, is reported: module code runs on the map loop, and a slow module
/// delays every map.
pub(crate) const WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP: Duration = Duration::from_millis(5);

/// How often repeated budget overruns are summarized, after the first one of
/// each module was reported.
pub(crate) const WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP: Duration = Duration::from_secs(10);

/// The most `AnnounceWorld` lines one dispatch sends. Each line is one packet
/// to every player in the world, whose socket queue is bounded; announcements
/// beyond it are cut, and the cut is logged.
pub(crate) const MAX_WORLD_TEXT_LINES_PER_DISPATCH_LIKE_CPP: usize = 8;

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

    /// C++ `World::SendWorldText` for an already-localized text: each
    /// non-empty line (TrinityCore `ChatHandler::LineFromMessage` tokenizes
    /// with `strtok`, which skips empty lines, `Chat.cpp:43`) as a
    /// `CHAT_MSG_SYSTEM` to every session whose player is in the world, at most
    /// `max_lines` of them. Returns the lines sent and the packets queued.
    pub(crate) fn send_world_text_like_cpp(&self, text: &str, max_lines: usize) -> (usize, usize) {
        let packets: Vec<Vec<u8>> = text
            .split('\n')
            .filter(|line| !line.is_empty())
            .take(max_lines)
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
        if packets.is_empty() {
            return (0, 0);
        }
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
        (packets.len(), queued)
    }
}

/// One module's `world.update` cost since the process started.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct WorldUpdateModuleMetricLikeCpp {
    pub(crate) calls: u64,
    pub(crate) total: Duration,
    pub(crate) max: Duration,
    pub(crate) over_budget: u64,
}

/// The per-module `world.update` budget and metric (#1444, plan §4.3 row 15).
///
/// The first overrun of each module, and the first of effect application,
/// is logged at once. Later ones are counted and summarized at most once per
/// [`WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP`], so a steadily slow
/// module costs one log line per window, not one per tick.
#[derive(Clone, Debug)]
struct WorldUpdateBudgetLikeCpp {
    budget: Duration,
    modules: BTreeMap<ModuleId, WorldUpdateModuleMetricLikeCpp>,
    /// Overruns not yet logged, per module; `None` stands for effects.
    unreported: BTreeMap<Option<ModuleId>, u64>,
    effects_warned: bool,
    window_started: Option<Instant>,
    /// How many warnings this tracker emitted: the throttle's observable.
    warnings: u64,
}

impl WorldUpdateBudgetLikeCpp {
    fn new(budget: Duration) -> Self {
        Self {
            budget,
            modules: BTreeMap::new(),
            unreported: BTreeMap::new(),
            effects_warned: false,
            window_started: None,
            warnings: 0,
        }
    }

    fn record_like_cpp(
        &mut self,
        now: Instant,
        tick: u64,
        timings: &[(ModuleId, Duration)],
        effects: Duration,
    ) {
        let window_started = *self.window_started.get_or_insert(now);
        for (module, elapsed) in timings {
            let metric = self.modules.entry(module.clone()).or_default();
            metric.calls += 1;
            metric.total += *elapsed;
            metric.max = metric.max.max(*elapsed);
            if *elapsed <= self.budget {
                continue;
            }
            metric.over_budget += 1;
            if metric.over_budget == 1 {
                self.warnings += 1;
                warn!(
                    %module,
                    tick,
                    elapsed_us = elapsed.as_micros() as u64,
                    budget_us = self.budget.as_micros() as u64,
                    "linked module's world.update exceeded its per-tick budget; further overruns are summarized"
                );
            } else {
                *self.unreported.entry(Some(module.clone())).or_default() += 1;
            }
        }
        if effects > self.budget {
            if self.effects_warned {
                *self.unreported.entry(None).or_default() += 1;
            } else {
                self.effects_warned = true;
                self.warnings += 1;
                warn!(
                    tick,
                    elapsed_us = effects.as_micros() as u64,
                    budget_us = self.budget.as_micros() as u64,
                    "applying world.update effects exceeded the per-tick budget; further overruns are summarized"
                );
            }
        }
        if now.saturating_duration_since(window_started)
            >= WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP
        {
            self.summarize_like_cpp();
            self.window_started = Some(now);
        }
    }

    /// Log the overruns counted since the last summary, one line per module.
    fn summarize_like_cpp(&mut self) {
        for (module, suppressed) in std::mem::take(&mut self.unreported) {
            self.warnings += 1;
            match module {
                Some(module) => {
                    let metric = self.modules.get(&module).copied().unwrap_or_default();
                    warn!(
                        %module,
                        suppressed,
                        over_budget = metric.over_budget,
                        calls = metric.calls,
                        max_us = metric.max.as_micros() as u64,
                        mean_us = (metric.total.as_micros() / u128::from(metric.calls.max(1))) as u64,
                        "linked module's world.update keeps exceeding its per-tick budget"
                    );
                }
                None => warn!(
                    suppressed,
                    "applying world.update effects keeps exceeding the per-tick budget"
                ),
            }
        }
    }
}

/// A problem met while applying one dispatch's effects.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ApplyWarningLikeCpp {
    /// The batch failed validation as a whole; nothing was applied.
    BatchRejected { error: String },
    /// An announcement lost lines to the per-dispatch line limit.
    AnnouncementCut {
        module: ModuleId,
        dropped_lines: usize,
    },
}

impl ApplyWarningLikeCpp {
    /// What the throttle keys the warning on: its kind and module.
    fn key(&self) -> (&'static str, Option<ModuleId>) {
        match self {
            Self::BatchRejected { .. } => ("batch_rejected", None),
            Self::AnnouncementCut { module, .. } => ("announcement_cut", Some(module.clone())),
        }
    }

    fn log(&self, hook: &'static str) {
        match self {
            Self::BatchRejected { error } => {
                warn!(%error, hook, "module effect batch rejected; no effect applied");
            }
            Self::AnnouncementCut {
                module,
                dropped_lines,
            } => warn!(
                %module,
                hook,
                dropped_lines,
                limit = MAX_WORLD_TEXT_LINES_PER_DISPATCH_LIKE_CPP,
                "world announcement cut at the per-dispatch line limit"
            ),
        }
    }
}

/// The throttle for the warnings a per-tick hook (`world.update`,
/// `creature.*`) can raise on every tick: the first occurrence of each
/// (hook, kind, module) is logged at once, later ones are counted and
/// summarized at most once per [`WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP`]
/// with the suppressed count, as the budget warning is.
#[derive(Clone, Debug, Default)]
struct ApplyWarningThrottleLikeCpp {
    warned: std::collections::BTreeSet<(&'static str, &'static str, Option<ModuleId>)>,
    unreported: BTreeMap<(&'static str, &'static str, Option<ModuleId>), u64>,
    window_started: Option<Instant>,
    /// How many lines this throttle logged: its observable.
    logged: u64,
}

impl ApplyWarningThrottleLikeCpp {
    fn record_like_cpp(
        &mut self,
        now: Instant,
        hook: &'static str,
        warnings: Vec<ApplyWarningLikeCpp>,
    ) {
        let window_started = *self.window_started.get_or_insert(now);
        for warning in warnings {
            let (kind, module) = warning.key();
            let key = (hook, kind, module);
            if self.warned.insert(key.clone()) {
                self.logged += 1;
                warning.log(hook);
            } else {
                *self.unreported.entry(key).or_default() += 1;
            }
        }
        if now.saturating_duration_since(window_started)
            >= WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP
        {
            for ((hook, kind, module), suppressed) in std::mem::take(&mut self.unreported) {
                self.logged += 1;
                match module {
                    Some(module) => warn!(
                        %module,
                        hook,
                        kind,
                        suppressed,
                        "linked module effect warnings repeated; summarized"
                    ),
                    None => warn!(
                        hook,
                        kind, suppressed, "module effect warnings repeated; summarized"
                    ),
                }
            }
            self.window_started = Some(now);
        }
    }
}

/// The process-level module hooks.
#[derive(Clone)]
pub(crate) struct WorldModuleHooksLikeCpp {
    registry: Arc<ModuleRegistry>,
    announcer: Option<WorldAnnouncerLikeCpp>,
    /// Set by `world.startup`. C++ runs `OnStartup` before the world loop's
    /// first `OnUpdate`; the canonical loop is spawned a little earlier, so a
    /// tick before startup dispatches nothing. Shared by every clone.
    started: Arc<AtomicBool>,
    /// When the previous `world.update` ran, so each dispatch reports the time
    /// since the previous one, as C++ `World::Update(diff)` does, even when
    /// the loop skipped a tick in between. Only the map loop's copy moves it.
    last_update: Option<Instant>,
    /// The per-module tick budget and metric. Only the map loop's copy
    /// records into it.
    budget: WorldUpdateBudgetLikeCpp,
    /// The throttle for the effect warnings of the per-tick hooks. Only the
    /// map loop's copy records into it.
    warnings: ApplyWarningThrottleLikeCpp,
}

impl WorldModuleHooksLikeCpp {
    pub(crate) fn new(registry: Arc<ModuleRegistry>) -> Self {
        Self::with_budget(registry, WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP)
    }

    fn with_budget(registry: Arc<ModuleRegistry>, budget: Duration) -> Self {
        Self {
            registry,
            announcer: None,
            started: Arc::new(AtomicBool::new(false)),
            last_update: None,
            budget: WorldUpdateBudgetLikeCpp::new(budget),
            warnings: ApplyWarningThrottleLikeCpp::default(),
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

    /// Bind the world announcer once the player registry exists, addressed
    /// with this realm's C++ `RealmHandle::GetAddress()`, as the sessions do.
    #[must_use]
    pub(crate) fn with_realm_announcer_like_cpp(
        mut self,
        players: &Arc<PlayerRegistry>,
        realm: &crate::RealmHandleLikeCpp,
        realm_id: u16,
    ) -> Self {
        let virtual_realm =
            wow_world::session::virtual_realm_address_like_cpp(realm.region, realm.site, realm_id);
        self.announcer = Some(WorldAnnouncerLikeCpp::new(
            Arc::clone(players),
            virtual_realm,
        ));
        self
    }

    /// Bind a given announcer (tests).
    #[cfg(test)]
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
        self.apply_and_log_like_cpp("world.config_loaded", outcome);
    }

    /// `world.startup`: AzerothCore `OnStartup` (worldserver `Main.cpp:395`).
    /// From here on, the map loop's ticks dispatch `world.update`.
    pub(crate) fn startup_like_cpp(&self, server_version: &str, realm_id: u32) {
        if self.wanted() {
            let module_count = self.registry.descriptors().len();
            let snapshot = WorldStartupSnapshot::new(server_version, realm_id, module_count);
            let outcome = self.registry.dispatch_world_startup(&snapshot);
            self.apply_and_log_like_cpp("world.startup", outcome);
        }
        self.started.store(true, Ordering::Release);
    }

    /// `world.shutdown`: AzerothCore `OnShutdown` (worldserver `Main.cpp:419`).
    pub(crate) fn shutdown_like_cpp(&self, exit_code: i32) {
        if !self.wanted() {
            return;
        }
        let outcome = self
            .registry
            .dispatch_world_shutdown(&WorldShutdownSnapshot::new(exit_code));
        self.apply_and_log_like_cpp("world.shutdown", outcome);
    }

    /// `world.update`: AzerothCore `OnWorldUpdate(diff)` (`World.cpp:1342`),
    /// once per completed world tick after `world.startup`. `now` is the
    /// tick's game time and `tick_diff_ms` the tick's own diff, used for the
    /// first dispatch only; later ones report the time since the previous
    /// dispatch. Returns how long the module callbacks and the effect
    /// application took, in that order.
    pub(crate) fn update_like_cpp(
        &mut self,
        now: Instant,
        tick_diff_ms: u32,
        tick: u64,
    ) -> Option<(Duration, Duration)> {
        if !self.wanted() || !self.started.load(Ordering::Acquire) {
            return None;
        }
        let diff_ms = self.last_update.replace(now).map_or(tick_diff_ms, |last| {
            now.saturating_duration_since(last)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32
        });
        let (outcome, timings) = self
            .registry
            .dispatch_world_update_timed(&WorldUpdateSnapshot::new(diff_ms, tick));
        let callbacks = timings.iter().map(|(_, elapsed)| *elapsed).sum();
        let applying = Instant::now();
        let warnings = self.apply_like_cpp("world.update", outcome);
        let effects = applying.elapsed();
        self.budget.record_like_cpp(now, tick, &timings, effects);
        self.warnings.record_like_cpp(now, "world.update", warnings);
        Some((callbacks, effects))
    }

    /// Whether a linked module wants the `creature.*` family: the map tick's
    /// interest query, asked before the lifecycle phase copies any event.
    pub(crate) fn creature_events_wanted_like_cpp(&self) -> bool {
        self.registry.handles(HookFamily::Creature)
    }

    /// `creature.added_to_world` and `creature.died`: AzerothCore
    /// `OnCreatureAddWorld` (`AllCreatureScript.h:43`) and `OnUnitDeath`
    /// (`UnitScript.h:111`), one dispatch per event in the order the
    /// lifecycle phase observed them, after the tick's guards were released.
    /// Not gated on `world.startup`: C++ adds creatures to the world before
    /// `OnStartup` too.
    pub(crate) fn creature_events_like_cpp(
        &mut self,
        now: Instant,
        events: &[CreatureModuleEventLikeCpp],
    ) {
        if events.is_empty() || !self.creature_events_wanted_like_cpp() {
            return;
        }
        for event in events {
            let location = CreatureLocation::new(
                u32::from(event.map_id),
                event.instance_id,
                event.position.x,
                event.position.y,
                event.position.z,
                event.position.orientation,
            );
            let (hook, outcome) = match event.kind {
                CreatureModuleEventKindLikeCpp::AddedToWorld => (
                    "creature.added_to_world",
                    self.registry.dispatch_creature_added_to_world(
                        &CreatureAddedToWorldSnapshot::new(event.guid, event.entry, location),
                    ),
                ),
                CreatureModuleEventKindLikeCpp::Died => (
                    "creature.died",
                    self.registry
                        .dispatch_creature_died(&CreatureDiedSnapshot::new(
                            event.guid,
                            event.entry,
                            location,
                        )),
                ),
            };
            let warnings = self.apply_like_cpp(hook, outcome);
            self.warnings.record_like_cpp(now, hook, warnings);
        }
    }

    /// The per-module `world.update` metric: calls, total and maximum
    /// callback time and budget overruns since the process started.
    #[cfg(test)]
    pub(crate) fn world_update_metrics_like_cpp(
        &self,
    ) -> Vec<(ModuleId, WorldUpdateModuleMetricLikeCpp)> {
        self.budget
            .modules
            .iter()
            .map(|(module, metric)| (module.clone(), *metric))
            .collect()
    }

    /// [`Self::apply_like_cpp`] for a hook that runs once (configuration,
    /// startup, shutdown): its warnings cannot repeat, so they are logged.
    fn apply_and_log_like_cpp(&self, hook: &'static str, outcome: HookOutcome<WorldEffect>) {
        for warning in self.apply_like_cpp(hook, outcome) {
            warning.log(hook);
        }
    }

    /// Apply a validated `world.*` batch, all or nothing. Announcements share
    /// one line budget per dispatch. A disabled module is logged at once (it
    /// happens once per module); the other problems are returned so a
    /// per-tick hook can throttle them.
    fn apply_like_cpp(
        &self,
        hook: &'static str,
        outcome: HookOutcome<WorldEffect>,
    ) -> Vec<ApplyWarningLikeCpp> {
        for module in &outcome.disabled {
            warn!(%module, hook, "linked module panicked; disabled for the rest of the process");
        }
        let mut warnings = Vec::new();
        let effects = match outcome.effects {
            Ok(effects) => effects,
            Err(error) => {
                warnings.push(ApplyWarningLikeCpp::BatchRejected {
                    error: error.to_string(),
                });
                return warnings;
            }
        };
        let mut lines_left = MAX_WORLD_TEXT_LINES_PER_DISPATCH_LIKE_CPP;
        for (module, effect) in effects.iter() {
            match effect {
                WorldEffect::Log { text } => info!(%module, hook, "{text}"),
                WorldEffect::AnnounceWorld { text } => match &self.announcer {
                    Some(announcer) => {
                        let wanted = text.split('\n').filter(|line| !line.is_empty()).count();
                        let (sent, queued) = announcer.send_world_text_like_cpp(text, lines_left);
                        lines_left -= sent;
                        if sent < wanted {
                            warnings.push(ApplyWarningLikeCpp::AnnouncementCut {
                                module: module.clone(),
                                dropped_lines: wanted - sent,
                            });
                        }
                        debug!(%module, hook, sent, queued, "module world announcement");
                    }
                    None => {
                        debug!(%module, hook, "module world announcement before any player can be in the world");
                    }
                },
                _ => warn!(%module, hook, "unsupported world effect ignored"),
            }
        }
        warnings
    }
}

#[cfg(test)]
#[path = "module_hooks_tests.rs"]
mod tests;
