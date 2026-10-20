// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World module adapter (#1445 H0), driven through the real hook port.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use wow_core::ObjectGuid;
use wow_module_api::{
    ModuleDescriptor, ModuleHooks, ModuleRegistry, ModuleVersion, PlayerLoginModule,
    PlayerLoginSnapshot, ScopedEffects,
};
use wow_world_application::{ContinueLoginHostLikeCpp, PlayerLoginHookEventLikeCpp};

use crate::session::{SharedCanonicalMapManager, WorldSession};

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_packet_tx, packet_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::unbounded();
    let session = WorldSession::new(
        1,
        "ModuleHooks".to_string(),
        0,
        2,
        2,
        54261,
        vec![0; 40],
        "enUS".to_string(),
        packet_rx,
        send_tx,
        crate::session::registry::build_dispatch_table(),
    );
    (session, send_rx)
}

fn event(first_login: bool) -> PlayerLoginHookEventLikeCpp {
    PlayerLoginHookEventLikeCpp {
        guid: ObjectGuid::create_player(1, 42),
        name: "Hooked".to_owned(),
        race: 1,
        class: 1,
        level: 10,
        map_id: 0,
        first_login,
    }
}

fn version() -> ModuleVersion {
    ModuleVersion {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

/// Records what it saw and probes the map lock from inside the callback.
struct Probe {
    text: &'static str,
    calls: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<PlayerLoginSnapshot>>>,
    map: Option<SharedCanonicalMapManager>,
    map_was_free: Arc<AtomicBool>,
}

impl PlayerLoginModule for Probe {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().expect("probe").push(snapshot.clone());
        if let Some(map) = &self.map {
            self.map_was_free
                .store(map.try_lock().is_ok(), Ordering::SeqCst);
        }
        effects.send_system_message_self(self.text);
    }
}

struct Probes {
    calls: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<PlayerLoginSnapshot>>>,
    map_was_free: Arc<AtomicBool>,
}

fn probe(text: &'static str, map: Option<SharedCanonicalMapManager>) -> (Probe, Probes) {
    let probes = Probes {
        calls: Arc::new(AtomicUsize::new(0)),
        seen: Arc::new(Mutex::new(Vec::new())),
        map_was_free: Arc::new(AtomicBool::new(false)),
    };
    (
        Probe {
            text,
            calls: Arc::clone(&probes.calls),
            seen: Arc::clone(&probes.seen),
            map,
            map_was_free: Arc::clone(&probes.map_was_free),
        },
        probes,
    )
}

fn register(registry: &mut ModuleRegistry, id: &str, module: impl PlayerLoginModule + 'static) {
    registry
        .register_module(
            ModuleDescriptor::new(id, version(), "Probe").expect("descriptor"),
            ModuleHooks::new().player_login(Arc::new(module)),
        )
        .expect("registration");
}

fn system_texts(send_rx: &flume::Receiver<Vec<u8>>, session: &WorldSession) -> Vec<String> {
    send_rx
        .try_iter()
        .map(|bytes| {
            for candidate in ["first", "second", "ok"] {
                let expected = wow_packet::packets::chat::ChatPkt {
                    msg_type: wow_packet::packets::chat::ChatMsg::System,
                    language: 0,
                    sender_guid: ObjectGuid::EMPTY,
                    sender_name: String::new(),
                    target_guid: ObjectGuid::EMPTY,
                    target_name: String::new(),
                    prefix: String::new(),
                    channel: String::new(),
                    text: candidate.to_owned(),
                    virtual_realm: session.core.virtual_realm_address(),
                };
                if wow_packet::ServerPacket::to_bytes(&expected) == bytes {
                    return candidate.to_owned();
                }
            }
            panic!("unexpected packet from the module adapter: {bytes:?}");
        })
        .collect()
}

#[test]
fn zero_modules_send_nothing_through_the_login_port() {
    let (mut session, send_rx) = make_session();
    session.player_login_hook_like_cpp(&event(true));
    assert!(send_rx.is_empty(), "the zero-module port must be a no-op");
}

#[test]
fn the_login_port_dispatches_in_module_id_order_and_applies_messages() {
    let (mut session, send_rx) = make_session();
    let (zulu, zulu_probes) = probe("second", None);
    let (alpha, alpha_probes) = probe("first", None);
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "zulu.probe", zulu);
    register(&mut registry, "alpha.probe", alpha);
    session.install_module_hooks_like_cpp(Arc::new(registry));

    session.player_login_hook_like_cpp(&event(true));

    assert_eq!(system_texts(&send_rx, &session), vec!["first", "second"]);
    assert_eq!(alpha_probes.calls.load(Ordering::SeqCst), 1);
    assert_eq!(zulu_probes.calls.load(Ordering::SeqCst), 1);
    let seen = alpha_probes.seen.lock().expect("probe").clone();
    assert_eq!(
        seen,
        vec![PlayerLoginSnapshot {
            guid: ObjectGuid::create_player(1, 42),
            name: "Hooked".to_owned(),
            race: 1,
            class: 1,
            level: 10,
            map_id: 0,
            first_login: true,
        }],
        "the snapshot carries the event one to one"
    );
}

/// The adapter takes no lock of its own: the canonical map manager is free
/// while a module runs.
#[test]
fn no_map_lock_is_held_while_a_module_runs() {
    let (mut session, _send_rx) = make_session();
    let canonical: SharedCanonicalMapManager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let (module, probes) = probe("ok", Some(Arc::clone(&canonical)));
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "alpha.probe", module);
    session.install_module_hooks_like_cpp(Arc::new(registry));

    session.player_login_hook_like_cpp(&event(false));

    assert_eq!(probes.calls.load(Ordering::SeqCst), 1);
    assert!(
        probes.map_was_free.load(Ordering::SeqCst),
        "a module must never run under the canonical map lock"
    );
}

struct Invalid;

impl PlayerLoginModule for Invalid {
    fn on_player_login(&self, _snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        effects.send_system_message_self("");
    }
}

#[test]
fn an_invalid_batch_applies_nothing_and_the_login_continues() {
    let (mut session, send_rx) = make_session();
    let (good, probes) = probe("ok", None);
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "alpha.probe", good);
    register(&mut registry, "zulu.invalid", Invalid);
    session.install_module_hooks_like_cpp(Arc::new(registry));

    session.player_login_hook_like_cpp(&event(false));

    assert_eq!(probes.calls.load(Ordering::SeqCst), 1);
    assert!(
        send_rx.is_empty(),
        "one invalid effect discards the whole batch, the good one included"
    );
}

struct Panics;

impl PlayerLoginModule for Panics {
    fn on_player_login(&self, _snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        effects.send_system_message_self("never sent");
        panic!("module bug");
    }
}

#[test]
fn a_panicking_module_is_disabled_and_the_others_still_apply() {
    let (mut session, send_rx) = make_session();
    let (good, probes) = probe("ok", None);
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "alpha.probe", good);
    register(&mut registry, "mike.panics", Panics);
    let registry = Arc::new(registry);
    session.install_module_hooks_like_cpp(Arc::clone(&registry));

    session.player_login_hook_like_cpp(&event(false));
    session.player_login_hook_like_cpp(&event(false));

    assert_eq!(system_texts(&send_rx, &session), vec!["ok", "ok"]);
    assert_eq!(probes.calls.load(Ordering::SeqCst), 2);
    assert!(registry.is_disabled(&wow_module_api::ModuleId::new("mike.panics").expect("id")));
}

/// The interest query the login asks before it reads anything (#1445 review
/// M1): nothing with zero modules, yes with a login module, and nothing again
/// once a panic disabled the only one.
#[test]
fn the_login_interest_query_follows_the_registry() {
    let (session, _send_rx) = make_session();
    assert!(!session.player_login_hook_wanted_like_cpp());

    let (mut session, _send_rx) = make_session();
    let (module, _probes) = probe("ok", None);
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "alpha.probe", module);
    session.install_module_hooks_like_cpp(Arc::new(registry));
    assert!(session.player_login_hook_wanted_like_cpp());

    let (mut session, _send_rx) = make_session();
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "mike.panics", Panics);
    session.install_module_hooks_like_cpp(Arc::new(registry));
    session.player_login_hook_like_cpp(&event(false));
    assert!(!session.player_login_hook_wanted_like_cpp());
}

/// The registry is installed once per session (#1445 review L4).
#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(expected = "installed once per session")
)]
fn a_second_registry_install_is_refused() {
    let (mut session, send_rx) = make_session();
    let (module, _probes) = probe("ok", None);
    let mut registry = ModuleRegistry::new();
    register(&mut registry, "alpha.probe", module);
    session.install_module_hooks_like_cpp(Arc::new(registry));
    session.install_module_hooks_like_cpp(Arc::new(ModuleRegistry::new()));
    session.player_login_hook_like_cpp(&event(false));
    assert_eq!(system_texts(&send_rx, &session), vec!["ok"]);
}

// ── The `player.*` ports (#1444 H1) ───────────────────────────────────────

use wow_module_api::{
    LogoutReason, PlayerLevelChangedSnapshot, PlayerLogoutSnapshot, PlayerModule,
    PlayerMoneyChangedSnapshot, QuestAcceptedSnapshot, QuestRewardedSnapshot,
};
use wow_world_application::{PlayerLogoutHookEventLikeCpp, SessionLifecycleHostLikeCpp};
use wow_world_quest::{
    PlayerLevelChangedHookEventLikeCpp, PlayerMoneyChangedHookEventLikeCpp,
    QuestAcceptedHookEventLikeCpp, QuestModuleHookPortLikeCpp, QuestRewardedHookEventLikeCpp,
    QuestSessionHostLikeCpp,
};

/// Records each `player.*` hook with whether the canonical map lock was free.
struct PlayerProbe {
    seen: Arc<Mutex<Vec<(String, bool)>>>,
    map: SharedCanonicalMapManager,
}

impl PlayerProbe {
    fn record(&self, what: String) {
        let free = self.map.try_lock().is_ok();
        self.seen.lock().expect("probe").push((what, free));
    }
}

impl PlayerModule for PlayerProbe {
    fn on_logout(&self, s: &PlayerLogoutSnapshot) {
        self.record(format!("logout {:?} {}", s.reason, s.account_id));
    }
    fn on_level_changed(&self, s: &PlayerLevelChangedSnapshot, e: &mut ScopedEffects<'_>) {
        self.record(format!("level {} {}", s.old_level, s.new_level));
        e.send_system_message_self("first");
    }
    fn on_money_changed(&self, s: &PlayerMoneyChangedSnapshot, e: &mut ScopedEffects<'_>) {
        self.record(format!("money {} {}", s.old_money, s.new_money));
        e.send_system_message_self("second");
    }
    fn on_quest_accepted(&self, s: &QuestAcceptedSnapshot, e: &mut ScopedEffects<'_>) {
        self.record(format!("accepted {} {}", s.quest_id, s.slot));
        e.send_system_message_self("ok");
    }
    fn on_quest_rewarded(&self, s: &QuestRewardedSnapshot, e: &mut ScopedEffects<'_>) {
        self.record(format!(
            "rewarded {} {} {} {} {}",
            s.quest_id, s.choice_item_id, s.reward_xp, s.money_before, s.money_after
        ));
        e.send_system_message_self("ok");
    }
}

fn player_probe_session() -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<Mutex<Vec<(String, bool)>>>,
) {
    let (mut session, send_rx) = make_session();
    let canonical: SharedCanonicalMapManager = Arc::new(Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.player", version(), "Player").expect("descriptor"),
            ModuleHooks::new().player(Arc::new(PlayerProbe {
                seen: Arc::clone(&seen),
                map: canonical,
            })),
        )
        .expect("registration");
    session.install_module_hooks_like_cpp(Arc::new(registry));
    (session, send_rx, seen)
}

#[test]
fn every_player_port_converts_its_event_and_runs_with_no_map_lock() {
    let (mut session, send_rx, seen) = player_probe_session();
    let guid = ObjectGuid::create_player(1, 42);

    QuestModuleHookPortLikeCpp::player_level_changed_like_cpp(
        &session.module_hooks,
        session.core.packet_publication_access_like_cpp(),
        &PlayerLevelChangedHookEventLikeCpp {
            guid,
            old_level: 9,
            new_level: 10,
        },
    );
    QuestModuleHookPortLikeCpp::player_money_changed_like_cpp(
        &session.module_hooks,
        session.core.packet_publication_access_like_cpp(),
        &PlayerMoneyChangedHookEventLikeCpp {
            guid,
            old_money: 5,
            new_money: 7,
        },
    );
    assert_eq!(system_texts(&send_rx, &session), vec!["first", "second"]);
    session.quest_accepted_hook_like_cpp(&QuestAcceptedHookEventLikeCpp {
        guid,
        quest_id: 7201,
        slot: 0,
    });
    QuestModuleHookPortLikeCpp::quest_rewarded_like_cpp(
        &session.module_hooks,
        session.core.packet_publication_access_like_cpp(),
        &QuestRewardedHookEventLikeCpp {
            guid,
            quest_id: 7201,
            quest_giver: guid,
            choice_item_id: 3,
            reward_xp: 120,
            money_before: 10,
            money_after: 60,
        },
    );
    assert_eq!(system_texts(&send_rx, &session), vec!["ok", "ok"]);
    session.player_logout_hook_like_cpp(&PlayerLogoutHookEventLikeCpp {
        guid,
        account_id: 1,
        mode: wow_world_lifecycle::FinalizationMode::TimedLogout,
    });
    assert!(send_rx.is_empty(), "logout carries no effect");

    let seen = seen.lock().expect("probe").clone();
    assert_eq!(
        seen.iter()
            .map(|(what, _)| what.as_str())
            .collect::<Vec<_>>(),
        vec![
            "level 9 10",
            "money 5 7",
            "accepted 7201 0",
            "rewarded 7201 3 120 10 60",
            "logout TimedLogout 1"
        ]
    );
    assert!(
        seen.iter().all(|(_, free)| *free),
        "no player hook may run under the canonical map lock: {seen:?}"
    );
}

#[test]
fn logout_reasons_map_one_to_one() {
    for (mode, reason) in [
        (
            wow_world_lifecycle::FinalizationMode::CharacterSelection,
            LogoutReason::CharacterSelection,
        ),
        (
            wow_world_lifecycle::FinalizationMode::TimedLogout,
            LogoutReason::TimedLogout,
        ),
        (
            wow_world_lifecycle::FinalizationMode::Disconnect,
            LogoutReason::Disconnect,
        ),
    ] {
        let (mut session, _send_rx, seen) = player_probe_session();
        session.player_logout_hook_like_cpp(&PlayerLogoutHookEventLikeCpp {
            guid: ObjectGuid::create_player(1, 42),
            account_id: 9,
            mode,
        });
        assert_eq!(
            seen.lock().expect("probe")[0].0,
            format!("logout {reason:?} 9")
        );
    }
}

#[test]
fn zero_modules_make_every_player_port_a_no_op() {
    let (mut session, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 42);
    QuestModuleHookPortLikeCpp::player_level_changed_like_cpp(
        &session.module_hooks,
        session.core.packet_publication_access_like_cpp(),
        &PlayerLevelChangedHookEventLikeCpp {
            guid,
            old_level: 1,
            new_level: 2,
        },
    );
    session.quest_accepted_hook_like_cpp(&QuestAcceptedHookEventLikeCpp {
        guid,
        quest_id: 1,
        slot: 0,
    });
    session.player_logout_hook_like_cpp(&PlayerLogoutHookEventLikeCpp {
        guid,
        account_id: 1,
        mode: wow_world_lifecycle::FinalizationMode::Disconnect,
    });
    assert!(send_rx.is_empty());
}
