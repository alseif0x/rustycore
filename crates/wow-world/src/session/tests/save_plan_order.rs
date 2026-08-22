// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The exact statement order of the Player character save.
//!
//! The #187 contract fixture pins semantic *groups* — one
//! `player.save.represented_snapshot` write group inside one CharacterDatabase
//! transaction — and deliberately not the statement inventory inside it. That
//! leaves the order within the transaction unguarded, which is precisely what
//! #286 is about to move into the adapter.
//!
//! This freezes that order first. It is not a claim that the order is correct
//! against C++; it is a claim that moving the builders must not change it.

use super::*;

/// A session with just enough state for the save snapshot to exist.
fn populate(session: &mut WorldSession, counter: i64) {
    session.set_player_guid(Some(ObjectGuid::create_player(1, counter)));
    session.set_state(SessionState::LoggedIn);
    session.set_player_map_position_like_cpp(
        0,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
        },
    );
}

/// Ordered SQL of every statement the aggregate plan appends, for a session
/// with a selected character and otherwise default state.
fn plan_statement_sql(session: &mut WorldSession) -> Vec<String> {
    let snapshot = session
        .sync_session_from_save_to_db_snapshot_like_cpp()
        .expect("a selected character yields a save snapshot");
    let plan = session
        .current_player_save_to_db_statement_plan_like_cpp(&snapshot, 1_700_000_000)
        .expect("a coherent session yields a plan");
    plan.statements
        .iter()
        .map(|statement| statement.sql().to_owned())
        .collect()
}

/// The frozen order. A move that reorders these rows inside the single
/// transaction changes crash and retry behaviour even though every final row
/// would look identical, so the sequence is asserted exactly rather than as a
/// set.
#[test]
fn the_character_save_plan_keeps_its_statement_order_like_cpp() {
    let (mut session, _, _) = make_session();
    populate(&mut session, 0x8100_0001);

    let order = plan_statement_sql(&mut session);

    assert!(
        !order.is_empty(),
        "a logged-in session must produce a save plan"
    );
    // Two passes over unchanged state produce the identical sequence: the plan
    // must not depend on iteration order of any map it reads.
    let (mut twin, _, _) = make_session();
    populate(&mut twin, 0x8100_0001);
    assert_eq!(
        order,
        plan_statement_sql(&mut twin),
        "the plan is not deterministic across identical sessions"
    );

    let mut again = plan_statement_sql(&mut session);
    assert_eq!(
        order, again,
        "the plan is not stable when built twice from the same session"
    );
    again.clear();

    insta_like_snapshot(&order);
}

/// Compare against the committed golden, and print a copy-pasteable
/// replacement when it differs so a deliberate change is easy to review.
fn insta_like_snapshot(order: &[String]) {
    let golden: Vec<String> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/player-save-plan-order.json"
    ))
    .expect("golden parses");
    assert_eq!(
        order,
        golden.as_slice(),
        "\nthe character save statement order changed.\nIf deliberate, replace \
         crates/wow-world/tests/fixtures/player-save-plan-order.json with:\n{}\n",
        serde_json::to_string_pretty(&order).unwrap_or_default()
    );
}
