// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Module API tests for [`super`].

#![cfg(test)]

use super::*;
use wow_core::ObjectGuid;

fn version() -> ModuleVersion {
    ModuleVersion {
        major: 1,
        minor: 0,
        patch: 0,
    }
}

fn snapshot(first_login: bool) -> PlayerLoginSnapshot {
    PlayerLoginSnapshot {
        guid: ObjectGuid::create_player(1, 42),
        name: "Tester".to_owned(),
        race: 1,
        class: 1,
        level: 1,
        map_id: 0,
        first_login,
    }
}

struct Greeter {
    text: String,
}

impl PlayerLoginModule for Greeter {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        if snapshot.first_login {
            effects.send_system_message_self(format!("{} welcome", self.text));
        } else {
            effects.send_system_message_self(self.text.clone());
        }
    }
}

fn registry_with(ids: &[(&str, &str)]) -> ModuleRegistry {
    let mut registry = ModuleRegistry::new();
    for (id, text) in ids {
        let descriptor = ModuleDescriptor::new(id, version(), "Greeter").expect("descriptor");
        registry
            .register_player_login_module(
                descriptor,
                Box::new(Greeter {
                    text: (*text).to_owned(),
                }),
            )
            .expect("registration");
    }
    registry
}

#[test]
fn zero_modules_dispatch_to_an_empty_batch() {
    let registry = ModuleRegistry::new();
    assert!(registry.is_empty());
    let effects = registry
        .dispatch_player_login(&snapshot(false))
        .expect("no-op");
    assert!(
        effects.is_empty(),
        "the zero-module path must produce nothing to apply"
    );
}

#[test]
fn login_dispatch_runs_each_module_once_in_module_id_order() {
    // Registered out of order on purpose: dispatch must not depend on it.
    let registry = registry_with(&[("zulu.greeter", "z"), ("alpha.greeter", "a")]);
    let effects = registry
        .dispatch_player_login(&snapshot(false))
        .expect("valid batch");
    let seen: Vec<_> = effects
        .iter()
        .map(|(module, effect)| match effect {
            PlayerLoginEffect::SendSystemMessageSelf { text } => (module.as_str(), text.as_str()),
        })
        .collect();
    assert_eq!(seen, vec![("alpha.greeter", "a"), ("zulu.greeter", "z")]);
    assert_eq!(effects.len(), 2, "each module contributes exactly once");
}

#[test]
fn first_login_state_reaches_the_module() {
    let registry = registry_with(&[("alpha.greeter", "hello")]);
    let first = registry
        .dispatch_player_login(&snapshot(true))
        .expect("valid");
    let repeat = registry
        .dispatch_player_login(&snapshot(false))
        .expect("valid");
    let text = |e: &PlayerLoginEffects| {
        e.iter()
            .map(|(_, effect)| match effect {
                PlayerLoginEffect::SendSystemMessageSelf { text } => text.clone(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(text(&first), vec!["hello welcome".to_owned()]);
    assert_eq!(text(&repeat), vec!["hello".to_owned()]);
}

struct Invalid(PlayerLoginEffect);

impl PlayerLoginModule for Invalid {
    fn on_player_login(&self, _snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        match &self.0 {
            PlayerLoginEffect::SendSystemMessageSelf { text } => {
                effects.send_system_message_self(text.clone())
            }
        }
    }
}

#[test]
fn one_invalid_effect_discards_the_whole_batch() {
    for bad in [String::new(), "x".repeat(256), "bad\u{7}bell".to_owned()] {
        let mut registry = registry_with(&[("alpha.greeter", "good")]);
        registry
            .register_player_login_module(
                ModuleDescriptor::new("zulu.broken", version(), "Broken").expect("descriptor"),
                Box::new(Invalid(PlayerLoginEffect::SendSystemMessageSelf {
                    text: bad.clone(),
                })),
            )
            .expect("registration");
        let error = registry
            .dispatch_player_login(&snapshot(false))
            .expect_err("an invalid effect must reject the batch");
        assert!(
            error.to_string().contains("zulu.broken"),
            "the error must name the offending module: {error}"
        );
    }
}

#[test]
fn a_newline_stays_legal_because_cpp_splits_on_it() {
    let mut registry = ModuleRegistry::new();
    registry
        .register_player_login_module(
            ModuleDescriptor::new("alpha.multiline", version(), "Multiline").expect("descriptor"),
            Box::new(Invalid(PlayerLoginEffect::SendSystemMessageSelf {
                text: "line one\nline two".to_owned(),
            })),
        )
        .expect("registration");
    assert_eq!(
        registry
            .dispatch_player_login(&snapshot(false))
            .expect("valid")
            .len(),
        1
    );
}

#[test]
fn duplicate_ids_and_invalid_descriptors_fail_clearly() {
    let mut registry = registry_with(&[("alpha.greeter", "a")]);
    let duplicate = registry.register_player_login_module(
        ModuleDescriptor::new("alpha.greeter", version(), "Greeter").expect("descriptor"),
        Box::new(Greeter {
            text: "b".to_owned(),
        }),
    );
    assert_eq!(
        duplicate,
        Err(ModuleRegistrationError::DuplicateId {
            id: "alpha.greeter".to_owned()
        })
    );

    for bad in [
        "",
        "Alpha",
        "1alpha",
        "alpha-greeter",
        "alpha greeter",
        &"a".repeat(65),
    ] {
        assert!(
            matches!(
                ModuleDescriptor::new(bad, version(), "Greeter"),
                Err(ModuleRegistrationError::InvalidId { .. })
            ),
            "id {bad:?} must be rejected"
        );
    }
    assert!(matches!(
        ModuleDescriptor::new("alpha.greeter", version(), "   "),
        Err(ModuleRegistrationError::InvalidDescriptor { .. })
    ));
}

/// The whole point of this crate is what it cannot reach. A dependency on the
/// runtime, transport, storage or protocol layers would let a module obtain a
/// `WorldSession`, a `Player`, a pool or a packet writer through a type, so
/// the manifest itself is the guard and is asserted here rather than left to
/// review.
#[test]
fn the_module_api_depends_on_nothing_that_could_leak_the_server() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "wow-world",
        "wow-map",
        "wow-network",
        "wow-packet",
        "wow-database",
        "wow-entities",
        "wow-instances",
        "world-server",
        "sqlx",
        "tokio",
        "flume",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "wow-module-api must not depend on {forbidden}"
        );
    }
    assert!(
        manifest.contains("wow-core"),
        "the snapshot needs ObjectGuid"
    );
}

/// A module must not be able to address anyone but the player who logged in.
#[test]
fn the_self_message_effect_carries_no_target() {
    let effect = PlayerLoginEffect::SendSystemMessageSelf {
        text: "hi".to_owned(),
    };
    let rendered = format!("{effect:?}");
    assert!(
        !rendered.contains("guid") && !rendered.contains("target"),
        "the effect must not carry a target: {rendered}"
    );
}

/// `compose.py` recomputes this digest to record it in `modules.lock.toml`.
/// Pinning the exact string here means the two implementations cannot drift
/// apart silently: `tools/modules/test_rustycore_module.py` asserts the same
/// literal from the Python side.
#[test]
fn the_config_digest_is_deterministic_and_matches_the_composer() {
    use std::collections::BTreeMap;
    let id = ModuleId::new("demo.greeter").expect("valid id");
    let empty = ModuleConfig::new(&id, BTreeMap::new());
    assert_eq!(empty.digest(), "fnv1a64:cbf29ce484222325");

    let populated = ModuleConfig::new(
        &id,
        BTreeMap::from([
            ("enabled".to_owned(), ModuleConfigValue::Bool(true)),
            (
                "welcome_text".to_owned(),
                ModuleConfigValue::Text("Overridden by the operator".to_owned()),
            ),
        ]),
    );
    assert_eq!(populated.digest(), "fnv1a64:1144d896121f347b");

    // Insertion order must not matter.
    let reordered = ModuleConfig::new(
        &id,
        BTreeMap::from([
            (
                "welcome_text".to_owned(),
                ModuleConfigValue::Text("Overridden by the operator".to_owned()),
            ),
            ("enabled".to_owned(), ModuleConfigValue::Bool(true)),
        ]),
    );
    assert_eq!(reordered.digest(), populated.digest());
}

/// Two modules cannot collide: each is constructed with its own id and only
/// ever receives the tree built for that namespace.
#[test]
fn configuration_is_namespaced_per_module() {
    use std::collections::BTreeMap;
    let mut alpha = ModuleConfig::new(
        &ModuleId::new("alpha.mod").expect("id"),
        BTreeMap::from([("text".to_owned(), ModuleConfigValue::Text("a".to_owned()))]),
    );
    let mut zulu = ModuleConfig::new(
        &ModuleId::new("zulu.mod").expect("id"),
        BTreeMap::from([("text".to_owned(), ModuleConfigValue::Text("z".to_owned()))]),
    );
    assert_eq!(alpha.text("text").expect("read"), Some("a".to_owned()));
    assert_eq!(zulu.text("text").expect("read"), Some("z".to_owned()));
    assert_ne!(alpha.digest(), zulu.digest());
}

#[test]
fn unknown_keys_wrong_types_and_blank_values_are_rejected_with_the_module_named() {
    use std::collections::BTreeMap;
    let id = ModuleId::new("demo.greeter").expect("id");

    let unknown = ModuleConfig::new(
        &id,
        BTreeMap::from([("typo".to_owned(), ModuleConfigValue::Bool(true))]),
    );
    let error = unknown
        .finish()
        .expect_err("an unread key is a typo, not a no-op");
    assert!(error.to_string().contains("demo.greeter"), "{error}");
    assert!(error.to_string().contains("typo"), "{error}");

    let mut wrong = ModuleConfig::new(
        &id,
        BTreeMap::from([(
            "enabled".to_owned(),
            ModuleConfigValue::Text("yes".to_owned()),
        )]),
    );
    let error = wrong
        .bool("enabled")
        .expect_err("a string is not a boolean");
    assert!(error.to_string().contains("must be a boolean"), "{error}");
}

// ── Source API 2: the registry by capability (#1445) ─────────────────────

#[test]
fn the_source_api_constants_are_additive() {
    assert_eq!(SOURCE_API, "2");
    assert_eq!(COMPATIBLE_SOURCE_APIS, &["1", "2"]);
}

#[test]
fn api_one_registration_is_the_login_family_of_api_two() {
    let mut registry = ModuleRegistry::new();
    assert!(!registry.handles(HookFamily::PlayerLogin));
    registry
        .register_player_login_module(
            ModuleDescriptor::new("alpha.greeter", version(), "Greeter").expect("descriptor"),
            Box::new(Greeter {
                text: "api one".to_owned(),
            }),
        )
        .expect("api 1 registration");
    registry
        .register_module(
            ModuleDescriptor::new("beta.greeter", version(), "Greeter").expect("descriptor"),
            ModuleHooks::new().player_login(std::sync::Arc::new(Greeter {
                text: "api two".to_owned(),
            })),
        )
        .expect("api 2 registration");
    assert!(registry.handles(HookFamily::PlayerLogin));
    let effects = registry
        .dispatch_player_login(&snapshot(false))
        .expect("valid");
    let texts: Vec<_> = effects
        .iter()
        .map(|(module, effect)| match effect {
            NotificationEffect::SendSystemMessageSelf { text } => {
                (module.as_str().to_owned(), text.clone())
            }
        })
        .collect();
    assert_eq!(
        texts,
        vec![
            ("alpha.greeter".to_owned(), "api one".to_owned()),
            ("beta.greeter".to_owned(), "api two".to_owned()),
        ]
    );
}

#[test]
fn a_module_with_no_hook_family_is_refused() {
    let mut registry = ModuleRegistry::new();
    let refused = registry.register_module(
        ModuleDescriptor::new("alpha.empty", version(), "Empty").expect("descriptor"),
        ModuleHooks::new(),
    );
    assert_eq!(
        refused,
        Err(ModuleRegistrationError::NoHooks {
            id: "alpha.empty".to_owned()
        })
    );
    assert!(registry.is_empty());
}

/// Queues a message, then panics: the queued message must not survive.
struct Panicking;

impl PlayerLoginModule for Panicking {
    fn on_player_login(&self, _snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        effects.send_system_message_self("queued before the panic");
        panic!("module bug");
    }
}

#[test]
fn a_panicking_module_is_isolated_and_disabled_for_the_rest_of_the_process() {
    let mut registry = registry_with(&[("alpha.greeter", "a"), ("zulu.greeter", "z")]);
    registry
        .register_player_login_module(
            ModuleDescriptor::new("mike.panics", version(), "Panics").expect("descriptor"),
            Box::new(Panicking),
        )
        .expect("registration");
    let id = ModuleId::new("mike.panics").expect("id");

    let outcome = registry.dispatch_player_login_outcome(&snapshot(false));
    assert_eq!(outcome.disabled, vec![id.clone()]);
    let effects = outcome
        .effects
        .expect("the healthy modules' batch is valid");
    let authors: Vec<_> = effects.iter().map(|(m, _)| m.as_str()).collect();
    assert_eq!(
        authors,
        vec!["alpha.greeter", "zulu.greeter"],
        "the panicking module's queued effect is discarded; the others still run in order"
    );
    assert!(registry.is_disabled(&id));

    let again = registry.dispatch_player_login_outcome(&snapshot(false));
    assert!(
        again.disabled.is_empty(),
        "a disabled module is not called again"
    );
    assert_eq!(again.effects.expect("valid").len(), 2);
}

#[test]
fn a_family_whose_only_module_was_disabled_is_no_longer_handled() {
    let mut registry = ModuleRegistry::new();
    registry
        .register_player_login_module(
            ModuleDescriptor::new("mike.panics", version(), "Panics").expect("descriptor"),
            Box::new(Panicking),
        )
        .expect("registration");
    assert!(registry.handles(HookFamily::PlayerLogin));
    let _ = registry.dispatch_player_login_outcome(&snapshot(true));
    assert!(
        !registry.handles(HookFamily::PlayerLogin),
        "owners skip building snapshots nobody can receive"
    );
    assert!(!registry.is_empty(), "the descriptor stays registered");
}

#[test]
fn the_chat_decision_defaults_to_allow() {
    assert_eq!(ChatDecision::default(), ChatDecision::Allow);
}

#[test]
fn the_callback_scope_is_visible_inside_and_cleared_after_a_panic() {
    struct Observe(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl PlayerLoginModule for Observe {
        fn on_player_login(&self, _s: &PlayerLoginSnapshot, _e: &mut ScopedEffects<'_>) {
            self.0.store(
                module_callback_in_progress(),
                std::sync::atomic::Ordering::SeqCst,
            );
        }
    }
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut registry = ModuleRegistry::new();
    registry
        .register_player_login_module(
            ModuleDescriptor::new("alpha.observe", version(), "Observe").expect("descriptor"),
            Box::new(Observe(std::sync::Arc::clone(&seen))),
        )
        .expect("registration");
    registry
        .register_player_login_module(
            ModuleDescriptor::new("mike.panics", version(), "Panics").expect("descriptor"),
            Box::new(Panicking),
        )
        .expect("registration");
    assert!(!module_callback_in_progress());
    let _ = registry.dispatch_player_login_outcome(&snapshot(false));
    assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
    assert!(
        !module_callback_in_progress(),
        "the scope must be cleared on unwind too"
    );
}

/// `catch_unwind` isolates a module only while panics unwind. No profile of
/// the workspace may switch to `panic = "abort"` (#1445 review).
#[test]
fn every_profile_keeps_panics_unwinding() {
    let manifest = include_str!("../../../Cargo.toml");
    let compact: String = manifest.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        !compact.contains("panic=\"abort\""),
        "a profile sets panic = \"abort\"; module failure isolation needs unwinding"
    );
}

// ── The `player.*` family (#1444 H1) ─────────────────────────────────────

/// Records every `player.*` hook it sees and answers each with one message.
#[derive(Default)]
struct PlayerRecorder {
    seen: std::sync::Mutex<Vec<String>>,
}

impl PlayerModule for PlayerRecorder {
    fn on_logout(&self, snapshot: &PlayerLogoutSnapshot) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("logout {:?}", snapshot.reason));
    }
    fn on_level_changed(
        &self,
        snapshot: &PlayerLevelChangedSnapshot,
        effects: &mut ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(format!(
            "level {} -> {}",
            snapshot.old_level, snapshot.new_level
        ));
        effects.send_system_message_self(format!("Level {}!", snapshot.new_level));
    }
    fn on_money_changed(
        &self,
        snapshot: &PlayerMoneyChangedSnapshot,
        effects: &mut ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(format!(
            "money {} -> {}",
            snapshot.old_money, snapshot.new_money
        ));
        effects.send_system_message_self("money");
    }
    fn on_quest_accepted(&self, snapshot: &QuestAcceptedSnapshot, effects: &mut ScopedEffects<'_>) {
        self.seen.lock().unwrap().push(format!(
            "accepted {} in {}",
            snapshot.quest_id, snapshot.slot
        ));
        effects.send_system_message_self("accepted");
    }
    fn on_quest_rewarded(&self, snapshot: &QuestRewardedSnapshot, effects: &mut ScopedEffects<'_>) {
        self.seen.lock().unwrap().push(format!(
            "rewarded {} xp {}",
            snapshot.quest_id, snapshot.reward_xp
        ));
        effects.send_system_message_self("rewarded");
    }
}

/// Overrides nothing: every `player.*` method keeps its empty default.
struct SilentPlayer;

impl PlayerModule for SilentPlayer {}

fn guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 42)
}

#[test]
fn the_player_family_dispatches_each_hook_in_module_id_order() {
    let zulu = std::sync::Arc::new(PlayerRecorder::default());
    let alpha = std::sync::Arc::new(PlayerRecorder::default());
    let mut registry = ModuleRegistry::new();
    for (id, module) in [("zulu.player", &zulu), ("alpha.player", &alpha)] {
        registry
            .register_module(
                ModuleDescriptor::new(id, version(), "Player").expect("descriptor"),
                ModuleHooks::new().player(std::sync::Arc::clone(module) as _),
            )
            .expect("registration");
    }
    assert!(registry.handles(HookFamily::Player));
    assert!(!registry.handles(HookFamily::PlayerLogin));

    let level = registry
        .dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(guid(), 9, 10))
        .effects
        .expect("valid");
    let authors: Vec<_> = level.iter().map(|(m, _)| m.as_str()).collect();
    assert_eq!(authors, vec!["alpha.player", "zulu.player"]);

    for outcome in [
        registry.dispatch_player_money_changed(&PlayerMoneyChangedSnapshot::new(guid(), 1, 2)),
        registry.dispatch_quest_accepted(&QuestAcceptedSnapshot::new(guid(), 7, 3)),
        registry.dispatch_quest_rewarded(&QuestRewardedSnapshot::new(
            guid(),
            7,
            guid(),
            0,
            120,
            10,
            60,
        )),
    ] {
        assert_eq!(outcome.effects.expect("valid").len(), 2);
    }
    let logout = registry.dispatch_player_logout(&PlayerLogoutSnapshot::new(
        guid(),
        5,
        LogoutReason::Disconnect,
    ));
    assert!(
        logout.effects.expect("valid").is_empty(),
        "a logging-out session receives no effect"
    );
    assert_eq!(
        *alpha.seen.lock().unwrap(),
        vec![
            "level 9 -> 10",
            "money 1 -> 2",
            "accepted 7 in 3",
            "rewarded 7 xp 120",
            "logout Disconnect"
        ]
    );
}

#[test]
fn every_player_hook_defaults_to_nothing() {
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.silent", version(), "Silent").expect("descriptor"),
            ModuleHooks::new().player(std::sync::Arc::new(SilentPlayer)),
        )
        .expect("registration");
    let level = registry
        .dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(guid(), 1, 2))
        .effects
        .expect("valid");
    assert!(level.is_empty());
}

#[test]
fn one_value_serves_two_families_with_one_state() {
    struct Both(std::sync::atomic::AtomicUsize);
    impl PlayerLoginModule for Both {
        fn on_player_login(&self, _s: &PlayerLoginSnapshot, _e: &mut ScopedEffects<'_>) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    impl PlayerModule for Both {
        fn on_level_changed(&self, _s: &PlayerLevelChangedSnapshot, _e: &mut ScopedEffects<'_>) {
            self.0.fetch_add(10, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let both = std::sync::Arc::new(Both(std::sync::atomic::AtomicUsize::new(0)));
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.both", version(), "Both").expect("descriptor"),
            ModuleHooks::new()
                .player_login(std::sync::Arc::clone(&both) as _)
                .player(std::sync::Arc::clone(&both) as _),
        )
        .expect("registration");
    let _ = registry.dispatch_player_login(&snapshot(false));
    let _ = registry.dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(guid(), 1, 2));
    assert_eq!(both.0.load(std::sync::atomic::Ordering::SeqCst), 11);
}

#[test]
fn a_player_hook_panic_disables_the_module_for_login_too() {
    struct Boom;
    impl PlayerLoginModule for Boom {
        fn on_player_login(&self, _s: &PlayerLoginSnapshot, e: &mut ScopedEffects<'_>) {
            e.send_system_message_self("never after the panic");
        }
    }
    impl PlayerModule for Boom {
        fn on_money_changed(&self, _s: &PlayerMoneyChangedSnapshot, _e: &mut ScopedEffects<'_>) {
            panic!("module bug");
        }
    }
    let boom = std::sync::Arc::new(Boom);
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.boom", version(), "Boom").expect("descriptor"),
            ModuleHooks::new()
                .player_login(std::sync::Arc::clone(&boom) as _)
                .player(boom as _),
        )
        .expect("registration");
    let outcome =
        registry.dispatch_player_money_changed(&PlayerMoneyChangedSnapshot::new(guid(), 1, 2));
    assert_eq!(outcome.disabled.len(), 1);
    assert!(
        registry
            .dispatch_player_login(&snapshot(true))
            .expect("valid")
            .is_empty()
    );
}

// ── The `world.*` family (#1444 H2) ──────────────────────────────────────

#[derive(Default)]
struct WorldRecorder {
    seen: std::sync::Mutex<Vec<String>>,
}

impl WorldModule for WorldRecorder {
    fn on_config_loaded(&self, s: &ConfigLoadedSnapshot, e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("config {}", s.reload));
        e.log("configured");
    }
    fn on_startup(&self, s: &WorldStartupSnapshot, e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("startup {}", s.module_count));
        e.announce_world("The world is up.");
    }
    fn on_shutdown(&self, s: &WorldShutdownSnapshot, e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("shutdown {}", s.exit_code));
        e.log("bye");
    }
    fn on_update(&self, s: &WorldUpdateSnapshot, _e: &mut WorldEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("update {} {}", s.tick, s.diff_ms));
    }
}

fn world_registry(modules: &[(&str, std::sync::Arc<dyn WorldModule>)]) -> ModuleRegistry {
    let mut registry = ModuleRegistry::new();
    for (id, module) in modules {
        registry
            .register_module(
                ModuleDescriptor::new(id, version(), "World").expect("descriptor"),
                ModuleHooks::new().world(std::sync::Arc::clone(module)),
            )
            .expect("registration");
    }
    registry
}

#[test]
fn the_world_family_dispatches_each_hook_with_typed_world_effects() {
    let zulu = std::sync::Arc::new(WorldRecorder::default());
    let alpha = std::sync::Arc::new(WorldRecorder::default());
    let registry = world_registry(&[
        ("zulu.world", zulu.clone() as _),
        ("alpha.world", alpha.clone() as _),
    ]);
    assert!(registry.handles(HookFamily::World));
    assert!(!registry.handles(HookFamily::Player));

    let config = registry
        .dispatch_config_loaded(&ConfigLoadedSnapshot::new(false, 1, 80, 2))
        .effects
        .expect("valid");
    let seen: Vec<_> = config
        .iter()
        .map(|(m, e)| (m.as_str(), e.clone()))
        .collect();
    assert_eq!(
        seen,
        vec![
            (
                "alpha.world",
                WorldEffect::Log {
                    text: "configured".to_owned()
                }
            ),
            (
                "zulu.world",
                WorldEffect::Log {
                    text: "configured".to_owned()
                }
            ),
        ]
    );
    let startup = registry
        .dispatch_world_startup(&WorldStartupSnapshot::new("RustyCore", 1, 2))
        .effects
        .expect("valid");
    assert!(
        startup
            .iter()
            .all(|(_, e)| matches!(e, WorldEffect::AnnounceWorld { .. }))
    );
    assert!(
        registry
            .dispatch_world_update(&WorldUpdateSnapshot::new(50, 7))
            .effects
            .expect("valid")
            .is_empty()
    );
    let _ = registry.dispatch_world_shutdown(&WorldShutdownSnapshot::new(0));
    assert_eq!(
        *alpha.seen.lock().unwrap(),
        vec!["config false", "startup 2", "update 7 50", "shutdown 0"]
    );
}

#[test]
fn an_invalid_world_announcement_rejects_the_whole_batch() {
    struct Bad;
    impl WorldModule for Bad {
        fn on_startup(&self, _s: &WorldStartupSnapshot, e: &mut WorldEffects<'_>) {
            e.announce_world("x".repeat(300));
        }
    }
    let good = std::sync::Arc::new(WorldRecorder::default());
    let registry = world_registry(&[
        ("alpha.world", good as _),
        ("zulu.bad", std::sync::Arc::new(Bad) as _),
    ]);
    let outcome = registry.dispatch_world_startup(&WorldStartupSnapshot::new("RustyCore", 1, 2));
    let error = outcome
        .effects
        .expect_err("an over-long announcement rejects the batch");
    assert!(error.to_string().contains("zulu.bad"), "{error}");
}

#[test]
fn a_panicking_world_update_disables_only_that_module() {
    struct Boom;
    impl WorldModule for Boom {
        fn on_update(&self, _s: &WorldUpdateSnapshot, _e: &mut WorldEffects<'_>) {
            panic!("tick bug");
        }
    }
    let good = std::sync::Arc::new(WorldRecorder::default());
    let registry = world_registry(&[
        ("alpha.world", good.clone() as _),
        ("mike.boom", std::sync::Arc::new(Boom) as _),
    ]);
    let first = registry.dispatch_world_update(&WorldUpdateSnapshot::new(50, 1));
    assert_eq!(first.disabled.len(), 1);
    let second = registry.dispatch_world_update(&WorldUpdateSnapshot::new(50, 2));
    assert!(second.disabled.is_empty());
    assert_eq!(
        *good.seen.lock().unwrap(),
        vec!["update 1 50", "update 2 50"]
    );
}

#[test]
fn the_timed_world_update_reports_each_module_callback_in_order() {
    struct Slow;
    impl WorldModule for Slow {
        fn on_update(&self, _s: &WorldUpdateSnapshot, _e: &mut WorldEffects<'_>) {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    let quick = std::sync::Arc::new(WorldRecorder::default());
    let registry = world_registry(&[
        ("zulu.slow", std::sync::Arc::new(Slow) as _),
        ("alpha.quick", quick as _),
    ]);
    let (outcome, timings) = registry.dispatch_world_update_timed(&WorldUpdateSnapshot::new(50, 1));
    assert!(outcome.effects.expect("valid").is_empty());
    let ids: Vec<_> = timings.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["alpha.quick", "zulu.slow"]);
    assert!(timings[1].1 >= std::time::Duration::from_millis(2));
}

/// #1444 acceptance: the registry's share of the `world.update` tick cost
/// with zero modules and with one trivial module. Run in release with
/// `cargo test --release -p wow-module-api -- --ignored --nocapture
/// world_update_registry_cost`.
#[test]
#[ignore = "measurement, run explicitly"]
fn world_update_registry_cost_with_zero_and_one_trivial_module() {
    struct Trivial;
    impl WorldModule for Trivial {}
    const TICKS: u64 = 1_000_000;
    let measure = |registry: &ModuleRegistry| {
        let start = std::time::Instant::now();
        for tick in 1..=TICKS {
            if registry.handles(HookFamily::World) {
                std::hint::black_box(
                    registry.dispatch_world_update_timed(&WorldUpdateSnapshot::new(10, tick)),
                );
            }
        }
        start.elapsed().as_nanos() as f64 / TICKS as f64
    };
    let zero = ModuleRegistry::new();
    let one = world_registry(&[("trivial.world", std::sync::Arc::new(Trivial) as _)]);
    let zero_ns = measure(&zero);
    let one_ns = measure(&one);
    println!(
        "world.update registry cost ({}): zero modules {zero_ns:.1} ns/tick, one trivial module {one_ns:.1} ns/tick, over {TICKS} ticks",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    assert!(zero_ns < one_ns);
}

// ── `player.creature_kill` and `player.spell_cast` (#1444 H3) ─────────────

#[derive(Default)]
struct CombatRecorder {
    seen: std::sync::Mutex<Vec<String>>,
}

impl PlayerModule for CombatRecorder {
    fn on_creature_kill(&self, s: &CreatureKillSnapshot, e: &mut ScopedEffects<'_>) {
        self.seen.lock().unwrap().push(format!(
            "kill {} lvl {} map {}",
            s.creature_entry, s.creature_level, s.map_id
        ));
        e.send_system_message_self("kill");
    }
    fn on_spell_cast(&self, s: &SpellCastSnapshot, _e: &mut ScopedEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("cast {} triggered {}", s.spell_id, s.triggered));
    }
}

#[test]
fn creature_kill_and_spell_cast_dispatch_through_the_player_family() {
    let recorder = std::sync::Arc::new(CombatRecorder::default());
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.combat", version(), "Combat").unwrap(),
            ModuleHooks::new().player(recorder.clone()),
        )
        .unwrap();
    let player = ObjectGuid::create_player(1, 42);
    let kill = registry
        .dispatch_creature_kill(&CreatureKillSnapshot::new(
            player,
            ObjectGuid::EMPTY,
            299,
            5,
            0,
        ))
        .effects
        .expect("valid");
    assert_eq!(kill.len(), 1);
    assert!(
        registry
            .dispatch_spell_cast(&SpellCastSnapshot::new(
                player,
                133,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
            ))
            .effects
            .expect("valid")
            .is_empty()
    );
    assert_eq!(
        *recorder.seen.lock().unwrap(),
        vec!["kill 299 lvl 5 map 0", "cast 133 triggered false"]
    );
}

#[test]
fn creature_kill_and_spell_cast_default_to_no_effect() {
    struct Silent;
    impl PlayerModule for Silent {}
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.silent", version(), "Silent").unwrap(),
            ModuleHooks::new().player(std::sync::Arc::new(Silent)),
        )
        .unwrap();
    let player = ObjectGuid::create_player(1, 42);
    assert!(
        registry
            .dispatch_creature_kill(&CreatureKillSnapshot::new(player, player, 1, 1, 0))
            .effects
            .expect("valid")
            .is_empty()
    );
}

// ── `item.equipped`, `player.loot_item`, `player.loot_money` (#1444 H4) ──

#[derive(Default)]
struct ItemRecorder {
    seen: std::sync::Mutex<Vec<String>>,
}

impl PlayerModule for ItemRecorder {
    fn on_equip(&self, s: &ItemEquippedSnapshot, e: &mut ScopedEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("equip {} {} {}", s.item_entry, s.bag, s.slot));
        e.send_system_message_self("equipped");
    }
    fn on_loot_item(&self, s: &LootItemSnapshot, e: &mut ScopedEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("loot {} x{}", s.item_id, s.count));
        e.send_system_message_self("looted");
    }
    fn on_loot_money(&self, s: &LootMoneySnapshot, _e: &mut ScopedEffects<'_>) {
        self.seen
            .lock()
            .unwrap()
            .push(format!("money {} {}", s.money, s.share));
    }
}

#[test]
fn equip_and_loot_hooks_dispatch_through_the_player_family() {
    let recorder = std::sync::Arc::new(ItemRecorder::default());
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.items", version(), "Items").unwrap(),
            ModuleHooks::new().player(recorder.clone()),
        )
        .unwrap();
    let player = ObjectGuid::create_player(1, 42);
    let item = ObjectGuid::create_item(1, 7);
    let creature =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 6, 9);
    let equip = registry
        .dispatch_item_equipped(&ItemEquippedSnapshot::new(player, item, 25, 255, 15))
        .effects
        .expect("valid");
    assert_eq!(equip.len(), 1);
    let loot = registry
        .dispatch_loot_item(&LootItemSnapshot::new(player, 2589, 3, creature))
        .effects
        .expect("valid");
    assert_eq!(loot.len(), 1);
    assert!(
        registry
            .dispatch_loot_money(&LootMoneySnapshot::new(player, creature, 90, 30))
            .effects
            .expect("valid")
            .is_empty()
    );
    assert_eq!(
        *recorder.seen.lock().unwrap(),
        vec!["equip 25 255 15", "loot 2589 x3", "money 90 30"]
    );
    assert!(!registry.handles(HookFamily::Chat));
}

// ── `chat.message`, a pre-decision (#1444 H4) ────────────────────────────

struct Chat(fn(&ChatMessageSnapshot) -> ChatDecision);

impl ChatModule for Chat {
    fn on_chat_message(&self, snapshot: &ChatMessageSnapshot) -> ChatDecision {
        (self.0)(snapshot)
    }
}

fn chat_registry(modules: &[(&str, fn(&ChatMessageSnapshot) -> ChatDecision)]) -> ModuleRegistry {
    let mut registry = ModuleRegistry::new();
    for (id, decide) in modules {
        registry
            .register_module(
                ModuleDescriptor::new(id, version(), "Chat").unwrap(),
                ModuleHooks::new().chat(std::sync::Arc::new(Chat(*decide))),
            )
            .unwrap();
    }
    registry
}

fn say(text: &str) -> ChatMessageSnapshot {
    ChatMessageSnapshot::new(
        ObjectGuid::create_player(1, 42),
        ChatKind::Say,
        7,
        text,
        None,
    )
}

#[test]
fn chat_replacements_chain_in_module_id_order() {
    // Registered out of order: `ModuleId` order decides, not registration.
    let registry = chat_registry(&[
        ("zulu.shout", |s| {
            ChatDecision::ReplaceText(s.text.to_uppercase())
        }),
        ("alpha.censor", |s| {
            ChatDecision::ReplaceText(s.text.replace("darn", "****"))
        }),
    ]);
    assert!(registry.handles(HookFamily::Chat));
    assert!(!registry.handles(HookFamily::Player));
    let outcome = registry.dispatch_chat_message(&say("darn it"));
    assert_eq!(
        outcome.decision,
        ChatDecision::ReplaceText("**** IT".to_owned())
    );
    assert_eq!(outcome.denied_by, None);
}

#[test]
fn the_first_chat_deny_wins_and_stops_the_dispatch() {
    let registry = chat_registry(&[
        ("alpha.block", |s| {
            if s.text.contains("gold") {
                ChatDecision::Deny
            } else {
                ChatDecision::Allow
            }
        }),
        ("zulu.boom", |_| panic!("must not run after a deny")),
    ]);
    let denied = registry.dispatch_chat_message(&say("cheap gold here"));
    assert_eq!(denied.decision, ChatDecision::Deny);
    assert_eq!(
        denied.denied_by.as_ref().map(ModuleId::as_str),
        Some("alpha.block")
    );
    assert!(
        denied.disabled.is_empty(),
        "the module after the deny never ran"
    );
}

#[test]
fn an_unchanged_chat_message_is_allowed() {
    let registry = chat_registry(&[("alpha.quiet", |_| ChatDecision::Allow)]);
    assert_eq!(
        registry.dispatch_chat_message(&say("hello")).decision,
        ChatDecision::Allow
    );
    assert_eq!(
        ModuleRegistry::new()
            .dispatch_chat_message(&say("hello"))
            .decision,
        ChatDecision::Allow
    );
}

#[test]
fn a_panicking_chat_module_is_disabled_and_its_decision_discarded() {
    let registry = chat_registry(&[
        ("alpha.boom", |_| panic!("chat bug")),
        ("zulu.tag", |s| {
            ChatDecision::ReplaceText(format!("{} (ok)", s.text))
        }),
    ]);
    let first = registry.dispatch_chat_message(&say("hi"));
    assert_eq!(
        first.decision,
        ChatDecision::ReplaceText("hi (ok)".to_owned())
    );
    assert_eq!(first.disabled.len(), 1);
    assert!(registry.is_disabled(&ModuleId::new("alpha.boom").unwrap()));
    let second = registry.dispatch_chat_message(&say("hi"));
    assert!(second.disabled.is_empty());
}

// ── Per-hook interest (#1444 H4, AzerothCore `CALL_ENABLED_HOOKS`) ────────

struct KillsOnly {
    seen: std::sync::Mutex<Vec<&'static str>>,
}

impl PlayerModule for KillsOnly {
    fn enabled_hooks(&self) -> &'static [PlayerHook] {
        &[PlayerHook::CreatureKill]
    }
    fn on_creature_kill(&self, _s: &CreatureKillSnapshot, _e: &mut ScopedEffects<'_>) {
        self.seen.lock().unwrap().push("kill");
    }
    fn on_spell_cast(&self, _s: &SpellCastSnapshot, _e: &mut ScopedEffects<'_>) {
        self.seen.lock().unwrap().push("cast");
    }
}

#[test]
fn the_player_interest_query_and_dispatch_follow_the_enabled_hooks() {
    let module = std::sync::Arc::new(KillsOnly {
        seen: std::sync::Mutex::new(Vec::new()),
    });
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.kills", version(), "Kills").unwrap(),
            ModuleHooks::new().player(module.clone()),
        )
        .unwrap();
    assert!(registry.handles(HookFamily::Player));
    assert!(registry.handles_player_hook(PlayerHook::CreatureKill));
    assert!(!registry.handles_player_hook(PlayerHook::SpellCast));
    assert!(!registry.handles_player_hook(PlayerHook::LootMoney));

    let player = ObjectGuid::create_player(1, 42);
    let _ = registry.dispatch_spell_cast(&SpellCastSnapshot::new(
        player,
        133,
        ObjectGuid::EMPTY,
        ObjectGuid::EMPTY,
        false,
    ));
    let _ = registry.dispatch_creature_kill(&CreatureKillSnapshot::new(
        player,
        ObjectGuid::EMPTY,
        299,
        5,
        0,
    ));
    assert_eq!(
        *module.seen.lock().unwrap(),
        vec!["kill"],
        "a hook the module did not enable is never called"
    );
}

#[test]
fn an_undeclared_player_module_enables_every_hook() {
    struct Undeclared;
    impl PlayerModule for Undeclared {}
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new("alpha.all", version(), "All").unwrap(),
            ModuleHooks::new().player(std::sync::Arc::new(Undeclared)),
        )
        .unwrap();
    for hook in PlayerHook::ALL {
        assert!(registry.handles_player_hook(*hook), "{hook:?}");
    }
}
