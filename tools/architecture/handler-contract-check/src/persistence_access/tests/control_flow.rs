//! Regressions for control flow.

use super::*;

#[test]
fn persistence_inventory_tracks_head_shaped_transactions_fields_branches_and_locks() {
    let baseline = inventory(
            r#"
                use sqlx::Acquire;
                use wow_database::{CharacterDatabase, ItemGuidAllocatorAdvisoryLockLikeCpp};
                struct State { login_db: wow_database::LoginDatabase }
                struct Session;
                impl Session { fn char_db(&self) -> Option<&CharacterDatabase> { None } }
                async fn work(state: &State, session: &Session, db: &CharacterDatabase) {
                    state.login_db.direct_query("SELECT 1").await.unwrap();
                    let mut tx = db.pool().begin().await.map_err(map_error).context("begin").unwrap();
                    tx.rollback().await.unwrap();
                    let mut tx = db.pool().begin().await.unwrap();
                    tx.commit().await.unwrap();
                    if let Some(char_db) = session.char_db() {
                        char_db.execute(&char_db.prepare(TODO)).await.unwrap();
                    }
                    let lock = ItemGuidAllocatorAdvisoryLockLikeCpp::acquire_like_cpp(db.pool()).await.unwrap();
                    lock.wait_until_lost_like_cpp().await.unwrap();
                    lock.release_like_cpp().await.unwrap();
                }
            "#,
        )
        .expect("HEAD-shaped persistence flow remains visible");
    let found = operations(&baseline);
    for expected in [
        (
            PersistenceTarget::LoginDatabase,
            PersistenceOperation::DirectQuery,
            "direct_query",
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::Rollback,
            "rollback",
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::Commit,
            "commit",
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::Execute,
            "execute",
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::PoolAccess,
            "pool",
        ),
        (
            PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
            PersistenceOperation::AdvisoryLock,
            "acquire_like_cpp",
        ),
        (
            PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
            PersistenceOperation::AdvisoryLock,
            "wait_until_lost_like_cpp",
        ),
        (
            PersistenceTarget::ItemGuidAllocatorAdvisoryLockLikeCpp,
            PersistenceOperation::AdvisoryLock,
            "release_like_cpp",
        ),
    ] {
        assert!(
            found.contains(&(expected.0, expected.1, expected.2.to_owned())),
            "missing {expected:?} from {found:#?}"
        );
    }
}

#[test]
fn persistence_inventory_unions_match_arm_assignments() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, pick: u8) {
                    let mut value = None;
                    match pick {
                        0 => value = Some(database),
                        _ => value = None,
                    }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_carries_failed_match_guard_mutations_to_later_arms() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    match true {
                        true if { slot = Some(database); false } => {}
                        _ => consume(slot.unwrap().pool()),
                    }
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_unions_if_else_branch_assignments() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, pick: bool) {
                    let mut value = None;
                    if pick { value = Some(database); } else { value = None; }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
                fn persistent_no_else(database: wow_database::CharacterDatabase, pick: bool) {
                    let mut value = None;
                    if pick { value = Some(database); }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
                fn clean(pick: bool) {
                    let mut value = None;
                    if pick { value = Some(1_u8); } else { value = None; }
                    if let Some(value) = value {
                        consume(value);
                    }
                }
            "#,
    )
    .unwrap();

    for enclosing in ["fn persistent", "fn persistent_no_else"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing pool-access row for {enclosing}"
        );
    }
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_retains_pre_loop_flow_for_zero_iteration_paths() {
    let baseline = inventory(
        r#"
                fn persistent_for(database: wow_database::CharacterDatabase, items: Vec<u8>) {
                    let mut value = Some(database);
                    for _ in items { value = None; }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
                fn persistent_while(database: wow_database::CharacterDatabase, running: bool) {
                    let mut value = Some(database);
                    while running { value = None; }
                    if let Some(database) = value {
                        consume(database.pool());
                    }
                }
            "#,
    )
    .unwrap();

    for enclosing in ["fn persistent_for", "fn persistent_while"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing pool-access row for {enclosing}"
        );
    }
}

#[test]
fn persistence_inventory_preserves_state_reachable_through_loop_breaks() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut value = Some(database);
                    loop {
                        if stop { break; }
                        value = None;
                    }
                    consume(value.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_mutations_at_loop_break_exits() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut slot = None;
                    loop {
                        slot = Some(&database);
                        if stop { break; }
                        slot = None;
                    }
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_continue_states_as_loop_back_edges() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, repeat: bool) {
                    let mut slot = None;
                    loop {
                        if let Some(db) = slot { consume(db.pool()); break; }
                        slot = Some(&database);
                        if repeat { continue; }
                        slot = None;
                    }
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_for_and_while_break_exits() {
    let baseline = inventory(
        r#"
                fn in_while(database: wow_database::CharacterDatabase, running: bool, stop: bool) {
                    let mut slot = None;
                    while running {
                        slot = Some(&database);
                        if stop { break; }
                        slot = None;
                    }
                    if let Some(db) = slot { consume(db.pool()); }
                }
                fn in_for(database: wow_database::CharacterDatabase, values: Vec<u8>, stop: bool) {
                    let mut slot = None;
                    for _ in values {
                        slot = Some(&database);
                        if stop { break; }
                        slot = None;
                    }
                    if let Some(db) = slot { consume(db.pool()); }
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn in_while", "fn in_for"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_routes_labeled_breaks_to_their_target_loop() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, clear: bool) {
                    let mut slot = None;
                    'outer: loop {
                        loop {
                            slot = Some(&database);
                            break 'outer;
                        }
                        if clear { slot = None; }
                    }
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_routes_labeled_breaks_to_blocks() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    'done: {
                        loop {
                            slot = Some(&database);
                            break 'done;
                        }
                        slot = None;
                    }
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_false_while_condition_mutations() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    while { slot = Some(&database); false } { slot = None; }
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_does_not_apply_diverging_let_else_state() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, maybe: Option<u8>) {
                    let mut value = Some(database);
                    let Some(_) = maybe else { value = None; return; };
                    consume(value.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn persistent"
                && row.operation == PersistenceOperation::PoolAccess)
    );
}

#[test]
fn persistence_inventory_binds_let_chain_patterns_in_while_body() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                fn persistent(maybe: Option<Holder>, enabled: bool) {
                    while let Some(holder) = maybe && enabled {
                        consume(holder.0.pool());
                        break;
                    }
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_binds_let_chain_values_in_later_conditions() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                fn persistent(maybe: Option<Holder>) {
                    if let Some(holder) = maybe && holder.0.pool().is_closed() {}
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_recomputes_loop_back_edges_to_a_fixed_point() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let source = &database;
                    let mut slot = None;
                    loop {
                        if let Some(db) = slot { consume(db.pool()); break; }
                        slot = Some(source);
                    }
                }
            "#,
    )
    .unwrap();
    let pool_rows = baseline
        .accesses
        .iter()
        .filter(|row| {
            row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
        })
        .collect::<Vec<_>>();
    assert_eq!(pool_rows.len(), 1);
    assert_eq!(pool_rows[0].count, 1);
}

#[test]
fn persistence_inventory_widens_recursively_growing_loop_values() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = database;
                    loop {
                        value = (value,);
                        consume(&value);
                    }
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::ValueAlias
    }));
}

#[test]
fn persistence_inventory_preserves_skipped_let_chain_state() {
    let baseline = inventory(
        r#"
                fn persistent(
                    database: wow_database::CharacterDatabase,
                    maybe: Option<()>,
                ) {
                    let mut slot = Some(database);
                    if let Some(_) = maybe && { slot = None; true } {}
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_keeps_pattern_failure_path_before_a_guard() {
    let baseline = inventory(
        r#"
                fn guarded(value: u8, pool: sqlx::MySqlPool) {
                    let mut slot = Some(pool);
                    match value {
                        0 if { slot = None; false } => {}
                        _ => {}
                    }
                    let kept = slot.unwrap();
                    sqlx::query("SELECT 1").execute(&kept);
                }
            "#,
    )
    .unwrap();
    // The pattern fails before the guard whenever `value != 0`, so the
    // wildcard arm must not inherit the cleared slot: the pool still
    // reaches the executor on that path.
    assert!(
        baseline.accesses.iter().any(|row| {
            row.enclosing == "fn guarded"
                && row.target == PersistenceTarget::MySqlPool
                && row.operation == PersistenceOperation::Execute
        }),
        "the pool access on the pattern-failure path was omitted"
    );
}

#[test]
fn persistence_inventory_records_persistence_returned_through_try() {
    let baseline = inventory(
        r#"
                fn forward(result: Result<(), sqlx::PgPool>) -> Result<(), sqlx::PgPool> {
                    result?;
                    Ok(())
                }
            "#,
    )
    .unwrap();
    // `?` hands the pool out of the function on the error path.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn forward"
            && row.operation == PersistenceOperation::ReturnEscape
            && row.target == PersistenceTarget::PgPool
    }));
}

#[test]
fn persistence_inventory_records_error_payloads_returned_through_try() {
    let baseline = inventory(
        r#"
                fn forward(pool: sqlx::PgPool) -> Result<(), sqlx::PgPool> {
                    let result = Err(pool);
                    result?;
                    Ok(())
                }
                fn mapped(pool: sqlx::PgPool) {
                    sqlx::query("SELECT 1").try_map(|row| Ok(row)).execute(&pool);
                }
            "#,
    )
    .unwrap();
    // The pool leaves the function as the error of `?`.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn forward"
            && row.operation == PersistenceOperation::ReturnEscape
            && row.target == PersistenceTarget::PgPool
    }));
    // `try_map` returns the query it maps, so the chained executor stays.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn mapped" && row.operation == PersistenceOperation::Execute
    }));
}

#[test]
fn persistence_inventory_records_control_flow_payloads_through_try() {
    let baseline = inventory(
        r#"
                use std::ops::ControlFlow;
                fn broken(database: wow_database::CharacterDatabase)
                    -> ControlFlow<wow_database::CharacterDatabase, ()>
                {
                    let broken = ControlFlow::Break(database);
                    broken?;
                    ControlFlow::Continue(())
                }
            "#,
    )
    .unwrap();
    // The database leaves the function through `?` just as an `Err` would.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn broken"
            && row.operation == PersistenceOperation::ReturnEscape
            && row.target == PersistenceTarget::CharacterDatabase
    }));
}

#[test]
fn persistence_inventory_preserves_state_across_short_circuit_rhs() {
    let baseline = inventory(
        r#"
                fn or_path(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut value = Some(database);
                    stop || { value = None; true };
                    consume(value.unwrap().pool());
                }
                fn and_path(database: wow_database::CharacterDatabase, proceed: bool) {
                    let mut value = Some(database);
                    proceed && { value = None; true };
                    consume(value.unwrap().pool());
                }
                fn unconditional(database: wow_database::CharacterDatabase) {
                    let mut value = Some(database);
                    value = None;
                    consume(value.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn or_path", "fn and_path"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn unconditional" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_binds_let_chain_patterns_in_then_branch() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                fn persistent(maybe: Option<Holder>, enabled: bool) {
                    if let Some(holder) = maybe && enabled {
                        consume(holder.0.pool());
                    }
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_labeled_break_values() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, choose: bool) {
                    let selected = 'done: {
                        if choose { break 'done database; }
                        panic!()
                    };
                    consume(selected.pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}
