// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The world-server `world.*` hook adapter (#1444 H2).

use std::sync::Mutex;

use wow_config::WorldConfigSet;

use wow_module_api::{ModuleDescriptor, ModuleHooks, ModuleVersion, WorldEffects, WorldModule};
use wow_world::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp,
    PlayerSessionRegistrationLikeCpp,
};

use super::*;

fn registration(is_in_world: bool) -> (PlayerSessionRegistrationLikeCpp, flume::Receiver<Vec<u8>>) {
    let (send_tx, send_rx) = flume::unbounded();
    let (command_tx, _command_rx) = flume::bounded(4);
    let info = PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp::new_with_bnet(
            "Tester", 1, 0, 0, 1, 1, 0, 2, "enUS", None,
        ),
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            is_in_world,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    };
    (info, send_rx)
}

/// Records each world hook; announces on startup and on every update.
#[derive(Default)]
struct Recorder {
    seen: Mutex<Vec<String>>,
}

impl WorldModule for Recorder {
    fn on_config_loaded(&self, s: &ConfigLoadedSnapshot, e: &mut WorldEffects<'_>) {
        self.seen.lock().unwrap().push(format!(
            "config {} {} {}",
            s.reload, s.realm_id, s.max_player_level
        ));
        e.log("config seen");
    }
    fn on_startup(&self, s: &WorldStartupSnapshot, e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("startup {} {}", s.realm_id, s.module_count));
        e.announce_world("Server up.\nHave fun.");
    }
    fn on_shutdown(&self, s: &WorldShutdownSnapshot, _e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("shutdown {}", s.exit_code));
    }
    fn on_update(&self, s: &WorldUpdateSnapshot, _e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("update {} {}", s.tick, s.diff_ms));
    }
}

fn hooks_with(recorder: Arc<Recorder>) -> WorldModuleHooksLikeCpp {
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "alpha.world",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "World",
            )
            .unwrap(),
            ModuleHooks::new().world(recorder),
        )
        .unwrap();
    WorldModuleHooksLikeCpp::new(Arc::new(registry))
}

fn system_text(bytes: &[u8], virtual_realm: u32, text: &str) -> bool {
    bytes
        == ChatPkt {
            msg_type: ChatMsg::System,
            language: 0,
            sender_guid: ObjectGuid::EMPTY,
            sender_name: String::new(),
            target_guid: ObjectGuid::EMPTY,
            target_name: String::new(),
            prefix: String::new(),
            channel: String::new(),
            text: text.to_owned(),
            virtual_realm,
        }
        .to_bytes()
}

#[test]
fn zero_modules_do_no_world_hook_work() {
    let mut hooks = WorldModuleHooksLikeCpp::after_config_load_like_cpp(
        Arc::new(ModuleRegistry::new()),
        &WorldConfigSet::default(),
        1,
    );
    assert_eq!(hooks.update_like_cpp(Instant::now(), 50, 1), None);
    assert_eq!(hooks.last_update, None, "no clock is kept without a module");
    hooks.startup_like_cpp("RustyCore", 1);
    hooks.shutdown_like_cpp(0);
}

#[test]
fn each_world_hook_reaches_the_module_with_its_snapshot() {
    let recorder = Arc::new(Recorder::default());
    let mut hooks = WorldModuleHooksLikeCpp::after_config_load_like_cpp(
        Arc::clone(&hooks_with(Arc::clone(&recorder)).registry),
        &WorldConfigSet::default(),
        7,
    );
    hooks.startup_like_cpp("RustyCore", 7);
    assert!(hooks.update_like_cpp(Instant::now(), 50, 3).is_some());
    hooks.shutdown_like_cpp(1);
    assert_eq!(
        *recorder.seen.lock().unwrap(),
        vec![
            "config false 7 80",
            "startup 7 1",
            "update 3 50",
            "shutdown 1"
        ]
    );
}

/// C++ `World::SendWorldText`: every line, as a system message, to every
/// session whose player is in the world, and to nobody else.
#[test]
fn announce_world_reaches_every_in_world_player_line_by_line() {
    let players = Arc::new(PlayerRegistry::new());
    let (first, first_rx) = registration(true);
    let (second, second_rx) = registration(true);
    let (loading, loading_rx) = registration(false);
    players.register_or_replace(ObjectGuid::create_player(1, 1), first, Default::default());
    players.register_or_replace(ObjectGuid::create_player(1, 2), second, Default::default());
    players.register_or_replace(ObjectGuid::create_player(1, 3), loading, Default::default());
    let virtual_realm = 0x0100_0001;
    let hooks = hooks_with(Arc::new(Recorder::default())).with_announcer(
        WorldAnnouncerLikeCpp::new(Arc::clone(&players), virtual_realm),
    );

    hooks.startup_like_cpp("RustyCore", 1);

    for rx in [&first_rx, &second_rx] {
        let packets: Vec<_> = rx.try_iter().collect();
        assert_eq!(packets.len(), 2, "one packet per line");
        assert!(system_text(&packets[0], virtual_realm, "Server up."));
        assert!(system_text(&packets[1], virtual_realm, "Have fun."));
    }
    assert!(
        loading_rx.try_recv().is_err(),
        "a player not in the world receives no world text"
    );
}

/// C++ `World::Update(diff)` runs `OnWorldUpdate` on every world tick, so a
/// module's `diff` sums to wall time. The canonical loop can skip a tick (the
/// map timer has not passed, a barrier is held), so each dispatch reports the
/// time since the previous dispatch, not only the last loop iteration.
#[test]
fn world_update_reports_the_time_since_the_previous_dispatch() {
    let recorder = Arc::new(Recorder::default());
    let mut hooks = hooks_with(Arc::clone(&recorder));
    hooks.startup_like_cpp("RustyCore", 1);
    let start = Instant::now();
    hooks.update_like_cpp(start, 50, 1);
    hooks.update_like_cpp(start + Duration::from_millis(120), 10, 3);
    hooks.update_like_cpp(start + Duration::from_millis(130), 10, 4);
    assert_eq!(
        *recorder.seen.lock().unwrap(),
        vec!["startup 1 1", "update 1 50", "update 3 120", "update 4 10"]
    );
}

/// C++ runs `OnStartup` before the world loop's first `OnUpdate`; the
/// canonical loop is spawned earlier, so its ticks before `world.startup`
/// dispatch nothing.
#[test]
fn no_world_update_is_dispatched_before_startup() {
    let recorder = Arc::new(Recorder::default());
    let mut hooks = hooks_with(Arc::clone(&recorder));
    let loop_copy = hooks.clone();
    let mut loop_copy = loop_copy;
    assert_eq!(loop_copy.update_like_cpp(Instant::now(), 50, 1), None);
    hooks.startup_like_cpp("RustyCore", 1);
    assert!(
        loop_copy.update_like_cpp(Instant::now(), 50, 2).is_some(),
        "startup on the composition root's copy releases the loop's copy"
    );
    assert_eq!(
        *recorder.seen.lock().unwrap(),
        vec!["startup 1 1", "update 2 50"]
    );
    let _ = &mut hooks;
}

/// A slow module is reported, and the dispatch still completes.
#[test]
fn world_update_measures_its_dispatch_against_the_budget() {
    struct Slow;
    impl WorldModule for Slow {
        fn on_update(&self, _s: &WorldUpdateSnapshot, _e: &mut WorldEffects<'_>) {
            std::thread::sleep(WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP + Duration::from_millis(1));
        }
    }
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "slow.world",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Slow",
            )
            .unwrap(),
            ModuleHooks::new().world(Arc::new(Slow)),
        )
        .unwrap();
    let mut hooks = WorldModuleHooksLikeCpp::new(Arc::new(registry));
    hooks.startup_like_cpp("RustyCore", 1);
    let (callbacks, _effects) = hooks
        .update_like_cpp(Instant::now(), 50, 1)
        .expect("a module offers world.update");
    assert!(callbacks > WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP);
    let metrics = hooks.world_update_metrics_like_cpp();
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].0.as_str(), "slow.world");
    assert_eq!(metrics[0].1.calls, 1);
    assert_eq!(metrics[0].1.over_budget, 1);
    assert!(metrics[0].1.max > WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP);
}

/// A steadily slow module is reported once, then summarized once per
/// window, not once per tick; callbacks and effects are counted apart.
#[test]
fn budget_overruns_are_throttled_and_attributed_per_module() {
    let mut hooks = WorldModuleHooksLikeCpp::with_budget(
        Arc::clone(&hooks_with(Arc::new(Recorder::default())).registry),
        Duration::ZERO,
    );
    hooks.startup_like_cpp("RustyCore", 1);
    let start = Instant::now();
    for tick in 1..=100 {
        hooks.update_like_cpp(start + Duration::from_millis(tick), 1, tick);
    }
    let first_window = hooks.budget.warnings;
    assert!(
        (1..=2).contains(&first_window),
        "one first warning for the module (and at most one for effects), not one per tick: {first_window}"
    );
    hooks.update_like_cpp(
        start + WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP + Duration::from_millis(200),
        1,
        101,
    );
    let summaries = hooks.budget.warnings - first_window;
    assert!(
        (1..=2).contains(&summaries),
        "one summary per source after the window: {summaries}"
    );
    let metrics = hooks.world_update_metrics_like_cpp();
    assert_eq!(metrics[0].0.as_str(), "alpha.world");
    assert_eq!(metrics[0].1.calls, 101);
    assert_eq!(metrics[0].1.over_budget, 101);
}

/// The announcer addresses this realm as the sessions do.
#[test]
fn the_realm_announcer_uses_the_session_virtual_realm_address() {
    let hooks = WorldModuleHooksLikeCpp::new(Arc::new(ModuleRegistry::new()))
        .with_realm_announcer_like_cpp(
            &Arc::new(PlayerRegistry::new()),
            &crate::RealmHandleLikeCpp::new_like_cpp(1, 2, 0x0304),
            0x0304,
        );
    assert_eq!(hooks.announcer.expect("bound").virtual_realm, 0x0102_0304);
}

/// TrinityCore `ChatHandler::LineFromMessage` skips empty lines, and one
/// dispatch sends at most `MAX_WORLD_TEXT_LINES_PER_DISPATCH_LIKE_CPP` lines
/// across all its announcements, so a module cannot flood the socket queues.
#[test]
fn announcements_skip_empty_lines_and_share_a_per_dispatch_line_cap() {
    struct Loud;
    impl WorldModule for Loud {
        fn on_startup(&self, _s: &WorldStartupSnapshot, e: &mut WorldEffects<'_>) {
            e.announce_world("\n\nfirst\n\nsecond\n");
            e.announce_world(vec!["x"; 20].join("\n"));
            e.announce_world("\n".repeat(200));
        }
    }
    let players = Arc::new(PlayerRegistry::new());
    let (player, player_rx) = registration(true);
    players.register_or_replace(ObjectGuid::create_player(1, 1), player, Default::default());
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "loud.world",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Loud",
            )
            .unwrap(),
            ModuleHooks::new().world(Arc::new(Loud)),
        )
        .unwrap();
    let hooks = WorldModuleHooksLikeCpp::new(Arc::new(registry))
        .with_announcer(WorldAnnouncerLikeCpp::new(Arc::clone(&players), 1));

    hooks.startup_like_cpp("RustyCore", 1);

    let packets: Vec<_> = player_rx.try_iter().collect();
    assert_eq!(packets.len(), MAX_WORLD_TEXT_LINES_PER_DISPATCH_LIKE_CPP);
    assert!(system_text(&packets[0], 1, "first"));
    assert!(system_text(&packets[1], 1, "second"));
    assert!(system_text(&packets[2], 1, "x"));
}

#[test]
fn an_announcement_before_the_registry_exists_is_dropped_without_failing() {
    let recorder = Arc::new(Recorder::default());
    let hooks = hooks_with(Arc::clone(&recorder));
    hooks.startup_like_cpp("RustyCore", 1);
    assert_eq!(recorder.seen.lock().unwrap().len(), 1);
}

/// The world tick dispatch runs after the loop released its map guard: the
/// placement is pinned in the source, and this proves the adapter itself
/// takes no lock a module could contend with.
#[test]
fn the_world_update_dispatch_sits_after_the_tick_with_no_guard_in_scope() {
    let source = include_str!("runtime/map/update_loop.rs");
    let dispatch = source
        .find("world_module_hooks.update_like_cpp(now, diff_ms, tick_epoch)")
        .expect("the loop dispatches world.update");
    let last_manager_drop = source
        .rfind("drop(manager);")
        .expect("the tick drops its guard");
    let game_events = source
        .find("if !stop_after_tick && game_event_scheduler.update(diff_ms)")
        .expect("game events run in the tick");
    let stop = source
        .rfind("if stop_after_tick {")
        .expect("the loop ends a final tick");
    assert!(last_manager_drop < dispatch && game_events < dispatch && dispatch < stop);
    assert_eq!(
        source
            .matches("world_module_hooks.update_like_cpp(")
            .count(),
        1,
        "one world.update per completed tick"
    );
    let tail = &source[dispatch..stop];
    assert!(
        !tail.contains(".lock()"),
        "no guard is taken between the dispatch and the end of the tick"
    );
}

// ── `creature.*` (#1444 H3b) and the per-tick warning throttle ─────────

#[derive(Default)]
struct CreatureProbe {
    seen: Mutex<Vec<String>>,
}

impl wow_module_api::CreatureModule for CreatureProbe {
    fn on_added_to_world(
        &self,
        s: &wow_module_api::CreatureAddedToWorldSnapshot,
        e: &mut WorldEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(format!(
            "added {} {} {} {}",
            s.entry, s.location.map_id, s.location.instance_id, s.location.x
        ));
        e.log("added");
    }
    fn on_died(&self, s: &wow_module_api::CreatureDiedSnapshot, e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("died {} {}", s.entry, s.location.map_id));
        e.announce_world(vec!["x"; 20].join("\n"));
    }
}

fn creature_hooks(probe: Arc<CreatureProbe>) -> WorldModuleHooksLikeCpp {
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "alpha.creatures",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Creatures",
            )
            .unwrap(),
            ModuleHooks::new().creature(probe),
        )
        .unwrap();
    WorldModuleHooksLikeCpp::new(Arc::new(registry)).with_announcer(WorldAnnouncerLikeCpp::new(
        Arc::new(PlayerRegistry::new()),
        1,
    ))
}

fn creature_event(kind: CreatureModuleEventKindLikeCpp, entry: u32) -> CreatureModuleEventLikeCpp {
    CreatureModuleEventLikeCpp {
        kind,
        guid: ObjectGuid::EMPTY,
        entry,
        map_id: 571,
        instance_id: 3,
        position: wow_core::Position::new(1.5, 2.0, 3.0, 0.0),
    }
}

#[test]
fn creature_events_dispatch_once_each_in_order_and_cost_nothing_without_modules() {
    let idle = WorldModuleHooksLikeCpp::new(Arc::new(ModuleRegistry::new()));
    assert!(!idle.creature_events_wanted_like_cpp());

    let probe = Arc::new(CreatureProbe::default());
    let mut hooks = creature_hooks(Arc::clone(&probe));
    assert!(hooks.creature_events_wanted_like_cpp());
    hooks.creature_events_like_cpp(
        Instant::now(),
        &[
            creature_event(CreatureModuleEventKindLikeCpp::AddedToWorld, 299),
            creature_event(CreatureModuleEventKindLikeCpp::Died, 299),
        ],
    );
    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec!["added 299 571 3 1.5", "died 299 571"]
    );
}

/// Review nit: a per-tick hook whose announcement is cut every tick, or whose
/// batch is rejected every tick, logs once and is then summarized.
#[test]
fn per_tick_effect_warnings_are_throttled_then_summarized() {
    let probe = Arc::new(CreatureProbe::default());
    let mut hooks = creature_hooks(probe);
    let start = Instant::now();
    for tick in 0..50 {
        hooks.creature_events_like_cpp(
            start + Duration::from_millis(tick),
            &[creature_event(CreatureModuleEventKindLikeCpp::Died, 1)],
        );
    }
    assert_eq!(
        hooks.warnings.logged, 1,
        "one cut warning, not one per tick"
    );
    hooks.creature_events_like_cpp(
        start + WORLD_UPDATE_BUDGET_SUMMARY_INTERVAL_LIKE_CPP + Duration::from_millis(100),
        &[],
    );
    assert_eq!(
        hooks.warnings.logged, 2,
        "one summary with the suppressed count, even on a quiet tick"
    );

    struct Invalid;
    impl WorldModule for Invalid {
        fn on_update(&self, _s: &WorldUpdateSnapshot, e: &mut WorldEffects<'_>) {
            e.log("x".repeat(10_000));
        }
    }
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "alpha.invalid",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Invalid",
            )
            .unwrap(),
            ModuleHooks::new().world(Arc::new(Invalid)),
        )
        .unwrap();
    let mut hooks = WorldModuleHooksLikeCpp::new(Arc::new(registry));
    hooks.startup_like_cpp("RustyCore", 1);
    for tick in 1..=50 {
        hooks.update_like_cpp(start + Duration::from_millis(tick), 1, tick);
    }
    assert_eq!(
        hooks.warnings.logged, 1,
        "one rejected-batch warning, not one per tick"
    );
}

/// The creature events are dispatched after the creature phases' deferred
/// delivery, which runs after every guard of the tick was released.
#[test]
fn the_creature_event_dispatch_sits_after_the_guards_of_the_tick() {
    let source = include_str!("runtime/map/update_loop.rs");
    let dispatch = source
        .find("world_module_hooks\n                    .creature_events_like_cpp(")
        .expect("the loop dispatches the creature events");
    let delivery = source
        .find("deliver_canonical_creature_phases_like_cpp(phases, &player_registry)")
        .expect("the creature phases are delivered");
    let guards_released = source
        .find("(tick_summary, creature_phases)\n            };")
        .expect("the tick's guarded block ends");
    assert!(guards_released < delivery && delivery < dispatch);
    assert_eq!(source.matches(".creature_events_like_cpp(").count(), 1);
}

/// The process hooks sit where C++ `main` calls them, relative to the steps
/// around them in `app.rs`:
/// - `OnAfterConfigLoad(false)` once the databases are started, the online
///   accounts cleared and the realm marked offline, as C++ reaches
///   `SetInitialWorldSettings` -> `LoadConfigSettings` only after `StartDB`
///   and the realm-offline update (AzerothCore `Main.cpp:285/291/315`,
///   `World.cpp:300`; TrinityCore `Main.cpp:293/304/337`, `World.cpp:2363`),
///   and before the realm is listed;
/// - `OnStartup` after both listeners are bound and the realm is marked
///   online (TrinityCore `Main.cpp:413`, AzerothCore `Main.cpp:395`), beside
///   the internal script lifecycle. The map loop is already spawned, but it
///   dispatches no `world.update` before this call (the started gate), so
///   modules see `OnStartup` before their first `OnUpdate`, as in C++;
/// - `OnShutdown` once the world loop stopped and before the realm is marked
///   offline (TrinityCore `Main.cpp:440`, AzerothCore `Main.cpp:419`). C++
///   runs it before `KickAll`/`UpdateSessions(1)`/`StopNetwork`, which happen
///   later in the socket manager's deleter; RustyCore runs it after the
///   session flush and the respawn-writer drain. That is an intentional
///   deviation: modules never run while sessions are still being finalized,
///   and an announcement reaches nobody in either order.
#[test]
fn the_process_hooks_follow_the_cpp_startup_and_shutdown_order() {
    let app = include_str!("app.rs");
    let at = |needle: &str| {
        app.find(needle)
            .unwrap_or_else(|| panic!("app.rs no longer contains {needle:?}"))
    };
    let once = |needle: &str| {
        assert_eq!(app.matches(needle).count(), 1, "{needle:?} appears once");
        at(needle)
    };
    let config = once("module_hooks::WorldModuleHooksLikeCpp::after_config_load_like_cpp(");
    let startup = once("world_module_hooks.startup_like_cpp(");
    let shutdown = once("world_module_hooks.shutdown_like_cpp(");

    assert!(at("let world_configs = wow_config::load_world_config_values();") < config);
    assert!(at("clear_online_accounts_like_cpp(&login_db") < config);
    assert!(at("verify_world_db_version_like_cpp(") < config);
    assert!(at("set_realm_offline(&login_db, realm_id).await?;") < config);
    assert!(config < at("update_realm_list_once_like_cpp("));

    assert!(at("if let Err(error) = set_realm_online(&login_db, realm_id)") < startup);
    assert!(at("let mut map_update_handle = spawn_canonical_map_update_loop(") < startup);
    assert!(at("wow_script::lifecycle::on_startup_like_cpp()") < startup);

    assert!(at("update_sessions_shutdown_flush_once_like_cpp(") < shutdown);
    assert!(at("if !drain_respawn_db_writer_like_cpp(") < shutdown);
    assert!(at("wow_script::lifecycle::on_shutdown_like_cpp()") < shutdown);
    assert!(shutdown < app.rfind("set_realm_offline(&login_db, realm_id)").unwrap());
}

/// #1444 acceptance: the tick cost of `world.update` with zero modules and
/// with one trivial module, measured on the adapter path the map loop runs
/// (interest query, started gate, diff, timed dispatch, effects, budget).
/// Run with `cargo test -p world-server --lib -- --ignored --nocapture
/// world_update_tick_cost`; the evidence records the figures and the profile.
#[test]
#[ignore = "measurement, run explicitly"]
fn world_update_tick_cost_with_zero_and_one_trivial_module() {
    struct Trivial;
    impl WorldModule for Trivial {
        fn on_update(&self, _s: &WorldUpdateSnapshot, _e: &mut WorldEffects<'_>) {}
    }
    const TICKS: u64 = 200_000;
    let measure = |hooks: &mut WorldModuleHooksLikeCpp| {
        hooks.startup_like_cpp("RustyCore", 1);
        let start = Instant::now();
        for tick in 1..=TICKS {
            std::hint::black_box(hooks.update_like_cpp(start, 10, tick));
        }
        start.elapsed().as_nanos() as f64 / TICKS as f64
    };
    let mut zero = WorldModuleHooksLikeCpp::new(Arc::new(ModuleRegistry::new()));
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "trivial.world",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Trivial",
            )
            .unwrap(),
            ModuleHooks::new().world(Arc::new(Trivial)),
        )
        .unwrap();
    let mut one = WorldModuleHooksLikeCpp::new(Arc::new(registry));
    let zero_ns = measure(&mut zero);
    let one_ns = measure(&mut one);
    println!(
        "world.update tick cost ({}): zero modules {zero_ns:.1} ns/tick, one trivial module {one_ns:.1} ns/tick, over {TICKS} ticks",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    assert!(zero_ns < one_ns);
}
