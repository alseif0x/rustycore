// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Placement of the `player.login` hook port (#1445 H0).

const LOGIN: &str = include_str!("../../src/continue_login/login.rs");

fn position(needle: &str) -> usize {
    LOGIN
        .find(needle)
        .unwrap_or_else(|| panic!("login.rs must contain {needle:?}"))
}

/// C++ calls `OnPlayerLogin` last (`CharacterHandler.cpp:1390`): after the
/// login criteria and after the online mark. #169: the online mark persists
/// before any module observes the login.
#[test]
fn the_login_port_runs_last_after_the_online_mark() {
    let online = position("mark_player_online_like_cpp(");
    let port = position("host.player_login_hook_like_cpp(&event);");
    assert!(
        online < port,
        "the port must follow the persisted online mark"
    );
    let tail = &LOGIN[port..];
    assert_eq!(
        tail.matches(".await").count(),
        0,
        "nothing may be awaited after the module port: it is the last step"
    );
    assert_eq!(
        LOGIN.matches("player_login_hook_like_cpp(").count(),
        1,
        "the login operation raises the hook exactly once"
    );
}

/// With no interested module the login reads nothing for the hook (#1445
/// review M1): the event (canonical Player reads that take the map lock) is
/// built only inside the interest query's branch, in its own block whose split
/// borrow ends before the port call.
#[test]
fn the_login_event_is_read_only_when_a_module_wants_it() {
    let gate = position("if host.player_login_hook_wanted_like_cpp() {");
    let block = position("let login_hook_event = {");
    let split = position("let (_, hub) = host.split_lifecycle_ref();");
    let read = position("PlayerLoginHookEventLikeCpp::from_hub_like_cpp(hub, first_login)");
    let close = gate + LOGIN[gate..].find("        };\n").expect("event block end");
    let port = position("host.player_login_hook_like_cpp(&event);");
    assert!(gate < block && block < split && split < read && read < close && close < port);
    assert_eq!(
        LOGIN.matches("from_hub_like_cpp(").count(),
        1,
        "nothing else builds the event"
    );
    assert_eq!(
        LOGIN[..gate].matches("player_login_hook").count(),
        0,
        "no hook work happens before the interest query"
    );
}
