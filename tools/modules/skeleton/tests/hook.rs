//! Focused hook test: the module registers and reacts to first login only.

use wow_module_api::{ModuleRegistry, PlayerLoginSnapshot};

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

#[test]
fn greets_only_on_first_login() {
    let mut registry = ModuleRegistry::new();
    __MODULE_CRATE__::register(&mut registry).expect("registration");

    let first = registry.dispatch_player_login(&snapshot(true)).expect("valid");
    assert_eq!(first.len(), 1, "first login is greeted");

    let repeat = registry.dispatch_player_login(&snapshot(false)).expect("valid");
    assert!(repeat.is_empty(), "a returning player is not greeted");
}
