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
//! ## Coverage
//!
//! The groups below are pinned: position, level/xp, money, rest state, health,
//! talent reset, explored zones, difficulties, glyphs, tutorials and played
//! time — twelve groups, thirty-five statements.
//!
//! Not pinned, because they need state a bare test session cannot hold: skills,
//! talents, spells and their cooldowns/charges, action buttons, CUF profiles,
//! equipment sets, void storage and instance time restrictions all read through
//! a player controller. Their order inside the transaction is still unguarded,
//! and #286 must not treat this file as covering them.
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

    // Populate the collection groups too. With an empty session the plan emits
    // only its nine scalar rows, which would leave the order of every
    // collection group unpinned — exactly the part #286 moves.
    session.set_player_skill_values_like_cpp(HashMap::from([(6u16, 300u16), (43u16, 150u16)]));
    session.mark_represented_glyphs_loaded_like_cpp();
    session.tutorials_changed_like_cpp = true;
    session.tutorials_loaded_coherently_like_cpp = true;
}

/// Ordered SQL of every statement the aggregate plan appends, for a session
/// with a selected character and otherwise default state.
fn plan_statement_sql(session: &mut WorldSession) -> Vec<(String, usize)> {
    let snapshot = session
        .sync_session_from_save_to_db_snapshot_like_cpp()
        .expect("a selected character yields a save snapshot");
    let plan = session
        .current_player_save_to_db_statement_plan_like_cpp(&snapshot, 1_700_000_000)
        .expect("a coherent session yields a plan");
    // Run-length encoded: a collection group emits the same statement once per
    // row, so recording every repeat would bury the group boundaries this test
    // exists to pin. `(sql, count)` keeps the order and the boundaries exact
    // while staying readable.
    let mut runs: Vec<(String, usize)> = Vec::new();
    for statement in &plan.statements {
        let sql = statement.sql().to_owned();
        match runs.last_mut() {
            Some((previous, count)) if *previous == sql => *count += 1,
            _ => runs.push((sql, 1)),
        }
    }
    runs
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
fn insta_like_snapshot(order: &[(String, usize)]) {
    let golden: Vec<(String, usize)> = serde_json::from_str(include_str!(
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
