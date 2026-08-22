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
