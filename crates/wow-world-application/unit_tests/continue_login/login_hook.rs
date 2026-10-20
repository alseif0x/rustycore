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

/// The port is called after the split borrow that read the event has ended,
/// so the World adapter is free to borrow the whole session.
#[test]
fn the_login_port_is_called_outside_the_split_borrow() {
    let read = position("PlayerLoginHookEventLikeCpp::from_hub_like_cpp(hub, first_login)");
    let port = position("host.player_login_hook_like_cpp(&event);");
    let between = &LOGIN[read..port];
    assert!(
        between.contains("};"),
        "the event is read inside its own block, closed before the port call"
    );
}
