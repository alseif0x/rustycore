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
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: "Tester".to_string(),
            account_id: 1,
            battlenet_account_id: 0,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
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
    let start = Instant::now();
    hooks.update_like_cpp(start, 50, 1);
    hooks.update_like_cpp(start + Duration::from_millis(120), 10, 3);
    hooks.update_like_cpp(start + Duration::from_millis(130), 10, 4);
    assert_eq!(
        *recorder.seen.lock().unwrap(),
        vec!["update 1 50", "update 3 120", "update 4 10"]
    );
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
    let elapsed = hooks
        .update_like_cpp(Instant::now(), 50, 1)
        .expect("a module offers world.update");
    assert!(elapsed > WORLD_UPDATE_HOOK_BUDGET_LIKE_CPP);
}

/// The announcer addresses this realm as the sessions do.
#[test]
fn the_realm_announcer_uses_the_session_virtual_realm_address() {
    let announcer =
        WorldAnnouncerLikeCpp::for_realm_like_cpp(Arc::new(PlayerRegistry::new()), 1, 2, 0x0304);
    assert_eq!(announcer.virtual_realm, 0x0102_0304);
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

/// The process hooks sit where C++ `main` calls them, relative to the steps
/// around them in `app.rs`:
/// - `OnAfterConfigLoad(false)` once the configuration is read, before the
///   world databases are checked and the realm is listed (AzerothCore
///   `World.cpp:300`, the end of `LoadConfigSettings`);
/// - `OnStartup` after both listeners are bound and the realm is marked
///   online, once the map loop runs (TrinityCore `Main.cpp:413`, AzerothCore
///   `Main.cpp:395`), beside the internal script lifecycle;
/// - `OnShutdown` after the world loop stopped, the sessions were flushed and
///   the respawn writer drained, and before the realm is marked offline
///   (TrinityCore `Main.cpp:440`, AzerothCore `Main.cpp:419`).
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
    assert!(config < at("clear_online_accounts_like_cpp(&login_db"));
    assert!(config < at("verify_world_db_version_like_cpp("));

    assert!(at("if let Err(error) = set_realm_online(&login_db, realm_id)") < startup);
    assert!(at("let mut map_update_handle = spawn_canonical_map_update_loop(") < startup);
    assert!(at("wow_script::lifecycle::on_startup_like_cpp()") < startup);

    assert!(at("update_sessions_shutdown_flush_once_like_cpp(") < shutdown);
    assert!(at("if !drain_respawn_db_writer_like_cpp(") < shutdown);
    assert!(at("wow_script::lifecycle::on_shutdown_like_cpp()") < shutdown);
    assert!(shutdown < app.rfind("set_realm_offline(&login_db, realm_id)").unwrap());
}
