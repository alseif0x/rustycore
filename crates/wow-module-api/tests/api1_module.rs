// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! A module the way an external repository writes it: a separate crate that
//! depends on `wow-module-api` alone (an integration test links nothing else).
//! It is a source API 1 module, so it also proves API 2 kept API 1 working,
//! and it names `ObjectGuid` through the re-export, never through `wow-core`
//! (#1445 review).

use std::collections::HashMap;
use std::sync::Mutex;

use wow_module_api::{
    ModuleConfig, ModuleDescriptor, ModuleId, ModuleRegistrationError, ModuleRegistry,
    ModuleVersion, ObjectGuid, PlayerLoginEffect, PlayerLoginModule, PlayerLoginSnapshot,
    ScopedEffects,
};

/// Per-player state keyed by the re-exported guid.
#[derive(Default)]
struct LoginCounter {
    logins: Mutex<HashMap<ObjectGuid, u32>>,
}

impl PlayerLoginModule for LoginCounter {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        let mut logins = self.logins.lock().unwrap();
        let count = logins.entry(snapshot.guid).or_default();
        *count += 1;
        effects.send_system_message_self(format!("Login number {count}."));
    }
}

fn register(
    registry: &mut ModuleRegistry,
    config: ModuleConfig,
) -> Result<(), ModuleRegistrationError> {
    config.finish()?;
    let descriptor = ModuleDescriptor::new(
        "example.login_counter",
        ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        "Login Counter",
    )?;
    registry.register_player_login_module(descriptor, Box::new(LoginCounter::default()))
}

fn snapshot(counter: i64) -> PlayerLoginSnapshot {
    PlayerLoginSnapshot {
        guid: ObjectGuid::create_player(1, counter),
        name: "Tester".to_owned(),
        race: 1,
        class: 1,
        level: 1,
        map_id: 0,
        first_login: false,
    }
}

#[test]
fn an_api_one_module_keyed_by_object_guid_registers_and_counts_per_player() {
    let mut registry = ModuleRegistry::new();
    let config = ModuleConfig::new(
        &ModuleId::new("example.login_counter").unwrap(),
        Default::default(),
    );
    register(&mut registry, config).expect("registration");

    let texts = |counter| {
        registry
            .dispatch_player_login(&snapshot(counter))
            .expect("valid")
            .iter()
            .map(|(_, effect)| match effect {
                PlayerLoginEffect::SendSystemMessageSelf { text } => text.clone(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(texts(1), vec!["Login number 1."]);
    assert_eq!(texts(1), vec!["Login number 2."]);
    assert_eq!(texts(2), vec!["Login number 1."]);
}
