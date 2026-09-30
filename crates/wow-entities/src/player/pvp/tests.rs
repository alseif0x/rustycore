// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::cell::Cell;

use crate::Player;

#[test]
fn hostile_area_branch_order_skips_faction_lookup_when_not_needed_like_cpp() {
    let unused_lookup = || panic!("faction resolver must stay behind its original branch");

    assert!(!Player::represented_hostile_area_state_like_cpp(
        true, true, true, true, false, false, true, unused_lookup,
    ));
    assert!(Player::represented_hostile_area_state_like_cpp(
        false, true, false, true, false, false, false, unused_lookup,
    ));
    assert!(Player::represented_hostile_area_state_like_cpp(
        false, false, true, true, false, false, false, unused_lookup,
    ));
    assert!(!Player::represented_hostile_area_state_like_cpp(
        false, false, false, true, true, false, true, unused_lookup,
    ));
    assert!(Player::represented_hostile_area_state_like_cpp(
        false, false, false, true, true, true, false, unused_lookup,
    ));
    assert!(!Player::represented_hostile_area_state_like_cpp(
        false, false, false, false, false, false, true, unused_lookup,
    ));
}

#[test]
fn uncontested_enemy_area_resolves_faction_groups_once_like_cpp() {
    let calls = Cell::new(0);

    let hostile = Player::represented_hostile_area_state_like_cpp(
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        || {
            calls.set(calls.get() + 1);
            (0b0010, Some((0, 0b0010)))
        },
    );

    assert!(hostile);
    assert_eq!(calls.get(), 1);
}

#[test]
fn friendly_group_precedes_enemy_and_unmatched_groups_use_realm_like_cpp() {
    let friend_wins = Player::represented_hostile_area_state_like_cpp(
        false,
        false,
        false,
        true,
        false,
        false,
        false,
        || (0b0011, Some((0b0001, 0b0010))),
    );
    assert!(!friend_wins, "the friend mask is checked before the enemy mask");

    let realm_fallback = Player::represented_hostile_area_state_like_cpp(
        false,
        false,
        false,
        true,
        false,
        false,
        true,
        || (0b0100, Some((0b0001, 0b0010))),
    );
    assert!(realm_fallback);
}

#[test]
fn missing_template_stays_nonhostile_but_war_mode_is_the_final_override_like_cpp() {
    let missing_template = Player::represented_hostile_area_state_like_cpp(
        false,
        false,
        false,
        true,
        false,
        false,
        true,
        || (0b0100, None),
    );
    assert!(!missing_template, "a missing template does not use realm fallback");

    let war_mode_without_area_hostility = Player::represented_hostile_area_state_like_cpp(
        false,
        false,
        false,
        false,
        false,
        true,
        false,
        || panic!("no area branch should resolve factions"),
    );
    assert!(war_mode_without_area_hostility);

    let war_mode_over_sanctuary = Player::represented_hostile_area_state_like_cpp(
        true,
        false,
        false,
        true,
        false,
        true,
        false,
        || panic!("Sanctuary should skip faction resolution"),
    );
    assert!(war_mode_over_sanctuary);
}
