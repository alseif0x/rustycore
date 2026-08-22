//! Focused hook test: the module registers, honours its configuration and
//! reacts to first login only.

use std::collections::BTreeMap;

use wow_module_api::{ModuleConfig, ModuleConfigValue, ModuleId, ModuleRegistry, PlayerLoginSnapshot};

fn snapshot(first_login: bool) -> PlayerLoginSnapshot {
    PlayerLoginSnapshot {
        guid: wow_core::ObjectGuid::create_player(1, 1),
        name: "Tester".to_owned(),
        race: 1,
        class: 1,
        level: 1,
        map_id: 0,
        first_login,
    }
}

fn config(pairs: &[(&str, ModuleConfigValue)]) -> ModuleConfig {
    ModuleConfig::new(
        &ModuleId::new("__MODULE_ID__").expect("valid id"),
        pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect::<BTreeMap<_, _>>(),
    )
}

fn configured() -> ModuleConfig {
    config(&[
        ("enabled", ModuleConfigValue::Bool(true)),
        ("welcome_text", ModuleConfigValue::Text("hello".to_owned())),
    ])
}

#[test]
fn greets_only_on_first_login() {
    let mut registry = ModuleRegistry::new();
    __MODULE_CRATE__::register(&mut registry, configured()).expect("registration");

    let first = registry.dispatch_player_login(&snapshot(true)).expect("valid");
    assert_eq!(first.len(), 1, "first login is greeted");

    let repeat = registry.dispatch_player_login(&snapshot(false)).expect("valid");
    assert!(repeat.is_empty(), "a returning player is not greeted");
}

#[test]
fn the_configured_text_is_the_only_thing_that_changes() {
    let mut registry = ModuleRegistry::new();
    __MODULE_CRATE__::register(
        &mut registry,
        config(&[("welcome_text", ModuleConfigValue::Text("custom line".to_owned()))]),
    )
    .expect("registration");
    let effects = registry.dispatch_player_login(&snapshot(true)).expect("valid");
    let rendered = format!("{:?}", effects.iter().collect::<Vec<_>>());
    assert!(rendered.contains("custom line"), "{rendered}");
}

#[test]
fn invalid_configuration_prevents_activation() {
    for bad in [
        vec![("welcome_text", ModuleConfigValue::Text(String::new()))],
        vec![("welcome_text", ModuleConfigValue::Bool(true))],
        vec![
            ("welcome_text", ModuleConfigValue::Text("ok".to_owned())),
            ("wecome_txet", ModuleConfigValue::Text("typo".to_owned())),
        ],
        vec![("enabled", ModuleConfigValue::Bool(true))],
    ] {
        let mut registry = ModuleRegistry::new();
        assert!(
            __MODULE_CRATE__::register(&mut registry, config(&bad)).is_err(),
            "{bad:?} must prevent activation"
        );
        assert!(registry.is_empty(), "a refused module must not be registered");
    }
}
