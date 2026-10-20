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

/// The ledger itself: in every mode with a player, `Retirement` (and so the
/// logout hook) comes only after `CharacterSave` and `CharacterOffline` were
/// admitted and applied, and it is admitted exactly once.
#[test]
fn the_ledger_admits_retirement_only_after_the_save_and_the_offline_mark() {
    use wow_world_lifecycle::{
        FinalizationMode, FinalizationOutcome, FinalizationStep, SessionFinalization,
    };
    for mode in [
        FinalizationMode::CharacterSelection,
        FinalizationMode::TimedLogout,
        FinalizationMode::Disconnect,
    ] {
        let mut ledger = SessionFinalization::new(mode, true, None);
        let mut ran = Vec::new();
        while let Some(step) = ledger.next_step() {
            assert!(ledger.begin(step), "{mode:?}: {step:?} is admitted");
            assert!(ledger.finish(step, FinalizationOutcome::Applied));
            ran.push(step);
        }
        let at = |wanted| ran.iter().position(|step| *step == wanted);
        let retirement = at(FinalizationStep::Retirement).expect("a player retires");
        assert!(
            at(FinalizationStep::CharacterSave).expect("saved") < retirement,
            "{mode:?}"
        );
        assert!(
            at(FinalizationStep::CharacterOffline).expect("marked offline") < retirement,
            "{mode:?}"
        );
        assert_eq!(
            ran.iter()
                .filter(|step| **step == FinalizationStep::Retirement)
                .count(),
            1,
            "{mode:?}: retirement runs once"
        );
        assert!(!ledger.begin(FinalizationStep::Retirement), "no replay");
    }
}
