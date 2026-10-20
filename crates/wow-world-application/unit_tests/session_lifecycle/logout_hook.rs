// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Placement of the `player.logout` hook port (#1444 H1).

const FINALIZATION: &str = include_str!("../../src/session_lifecycle/finalization.rs");

fn position(needle: &str) -> usize {
    FINALIZATION
        .find(needle)
        .unwrap_or_else(|| panic!("finalization.rs must contain {needle:?}"))
}

/// C++ `LogoutPlayer` order (`WorldSession.cpp:633-670`): `SaveToDB`, the
/// social cleanup, `OnPlayerLogout`, then `RemovePlayerFromMap`. The ledger
/// runs `CharacterSave` and `CharacterOffline` before `Retirement`.
#[test]
fn the_logout_port_runs_inside_retirement_between_the_registry_and_the_map() {
    let retirement = position("        Retirement => {");
    let registry = position("state.unregister_from_player_registry(hub)");
    let port = position("host.player_logout_hook_like_cpp(&event);");
    let map = position("s.unregister_canonical_player_from_map_like_cpp(&mut h)");
    let publication = position("        LogoutPublication => {");
    assert!(retirement < registry && registry < port && port < map && map < publication);
    assert_eq!(
        FINALIZATION.matches("player_logout_hook_like_cpp(").count(),
        1,
        "the logout operation raises the hook exactly once"
    );
    assert_eq!(
        FINALIZATION[retirement..publication]
            .matches(".await")
            .count(),
        0,
        "Retirement is synchronous, so the hook cannot be cancelled midway"
    );
}

#[test]
fn the_ledger_admits_retirement_only_after_the_save_and_the_offline_mark() {
    let ledger = include_str!("../../../wow-world-lifecycle/src/finalization.rs");
    for plan in ledger.split("=> &[").skip(1) {
        let plan = &plan[..plan.find(']').expect("plan end")];
        if let Some(retirement) = plan.find("Retirement") {
            let save = plan
                .find("CharacterSave")
                .expect("a plan with Retirement saves");
            let offline = plan
                .find("CharacterOffline")
                .expect("a plan with Retirement marks the character offline");
            assert!(save < retirement && offline < retirement, "{plan}");
        }
    }
}
