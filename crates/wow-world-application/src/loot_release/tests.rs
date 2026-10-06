// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application-owned unit tests for the loot-release helpers (#1263 F4).
//!
//! The session-integrated release and fanout paths are covered from
//! `wow-world` (the `handlers::loot` suites exercise the application context),
//! because they need a composed `WorldSession`. These tests pin the pure
//! helpers the application crate owns directly, so a change in their
//! arithmetic or cohort selection fails in the crate that defines it.

use super::*;
use wow_core::ObjectGuid;

#[test]
fn direct_item_count_release_without_maximum_destroys_the_whole_stack() {
    assert_eq!(direct_item_count_after_loot_release_like_cpp(10, None), 0);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(1, None), 0);
}

#[test]
fn direct_item_count_keeps_the_remainder_for_a_partial_release() {
    assert_eq!(
        direct_item_count_after_loot_release_like_cpp(10, Some(3)),
        7
    );
    assert_eq!(direct_item_count_after_loot_release_like_cpp(1, Some(1)), 0);
}

#[test]
fn direct_item_count_caps_the_destroy_count_at_the_current_count() {
    // C++ `Item::SetCount(0)` semantics: a stale maximum must not underflow.
    assert_eq!(
        direct_item_count_after_loot_release_like_cpp(4, Some(u32::MAX)),
        0
    );
    assert_eq!(direct_item_count_after_loot_release_like_cpp(4, Some(5)), 0);
}

#[test]
fn direct_item_count_with_zero_destroy_count_keeps_the_stack() {
    assert_eq!(direct_item_count_after_loot_release_like_cpp(7, Some(0)), 7);
    assert_eq!(direct_item_count_after_loot_release_like_cpp(0, Some(0)), 0);
}

#[test]
fn durable_fanout_viewers_are_the_union_of_both_snapshots() {
    let first = ObjectGuid::create_player(1, 1);
    let second = ObjectGuid::create_player(1, 2);
    let third = ObjectGuid::create_player(1, 3);

    let viewers = durable_loot_item_fanout_viewers_like_cpp(&[first, second], &[second, third]);
    assert_eq!(viewers.len(), 3);
    assert!(viewers.contains(&first));
    assert!(viewers.contains(&second));
    assert!(viewers.contains(&third));
}

#[test]
fn durable_fanout_viewers_survive_an_empty_committed_snapshot() {
    let first = ObjectGuid::create_player(1, 1);
    let viewers = durable_loot_item_fanout_viewers_like_cpp(&[first], &[]);
    assert_eq!(viewers.len(), 1);
    assert!(viewers.contains(&first));

    let viewers = durable_loot_item_fanout_viewers_like_cpp(&[], &[]);
    assert!(viewers.is_empty());
}
