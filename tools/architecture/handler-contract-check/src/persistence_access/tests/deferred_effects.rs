//! Regressions for deferred effects.

use super::*;

#[test]
fn persistence_inventory_resolves_local_closure_callables() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let factory = || database;
                    consume(factory().pool());
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
fn persistence_inventory_does_not_apply_closure_side_effects_at_declaration() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = Some(database);
                    let clear = || { value = None; };
                    drop(clear);
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
fn persistence_inventory_does_not_apply_async_side_effects_at_future_creation() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = Some(database);
                    let future = async { value = None; };
                    drop(future);
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
fn persistence_inventory_preserves_async_return_exit_mutations() {
    let baseline = inventory(
        r#"
                async fn persistent(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut slot = None;
                    async {
                        slot = Some(database);
                        if stop { return; }
                        slot = None;
                    }.await;
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
fn persistence_inventory_preserves_try_exit_mutations_in_deferred_bodies() {
    let baseline = inventory(
        r#"
                fn fallible() -> Result<(), ()> { Ok(()) }
                fn closure(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = || -> Result<(), ()> {
                        slot = Some(database);
                        fallible()?;
                        slot = None;
                        Ok(())
                    };
                    let _ = install();
                    consume(slot.unwrap().pool());
                }
                async fn asynchronous(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let _ = async {
                        slot = Some(database);
                        fallible()?;
                        slot = None;
                        Ok::<(), ()>(())
                    }.await;
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn closure", "fn asynchronous"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_retains_values_inserted_into_mutable_containers() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut values = Vec::new();
                    values.push(database);
                    consume(values[0].pool());
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
fn persistence_inventory_tracks_writes_through_mutable_aliases() {
    let baseline = inventory(
        r#"
                struct Holder { slot: Option<wow_database::CharacterDatabase> }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let output = &mut slot;
                    *output = Some(database);
                    consume(slot.unwrap().pool());
                }
                fn projected(database: wow_database::CharacterDatabase, mut holder: Holder) {
                    let output = &mut holder.slot;
                    *output = Some(database);
                    consume(holder.slot.unwrap().pool());
                }
                fn indexed(database: wow_database::CharacterDatabase) {
                    let mut slots = [None];
                    let output = &mut slots[0];
                    *output = Some(database);
                    consume(slots[0].unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn persistent", "fn projected", "fn indexed"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_tracks_option_insertions() {
    let baseline = inventory(
        r#"
                fn direct(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    slot.get_or_insert(database);
                    consume(slot.unwrap().pool());
                }
                fn lazy(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    slot.get_or_insert_with(|| database);
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn direct", "fn lazy"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_applies_closure_mutations_only_when_invoked() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = || { slot = Some(database); };
                    install();
                    consume(slot.unwrap().pool());
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = || { slot = Some(database); };
                    drop(install);
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_applies_closure_mutations_for_unmodeled_methods() {
    let baseline = inventory(
        r#"
                struct Runner;
                fn persistent(database: wow_database::CharacterDatabase, runner: Runner) {
                    let mut slot = None;
                    runner.invoke(|| slot = Some(database));
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
fn persistence_inventory_applies_closure_mutations_through_helpers() {
    let baseline = inventory(
        r#"
                fn invoke<F: FnOnce()>(callback: F) { callback() }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    invoke(|| slot = Some(database));
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
fn persistence_inventory_preserves_closure_return_exit_mutations() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, stop: bool) {
                    let mut slot = None;
                    let install = || {
                        slot = Some(database);
                        if stop { return; }
                        slot = None;
                    };
                    install();
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
fn persistence_inventory_applies_closure_mutations_for_combinators_conditionally() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    Some(()).map(|_| slot = Some(database));
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
fn persistence_inventory_propagates_mutable_argument_writes_conservatively() {
    let baseline = inventory(
        r#"
                fn install<T>(slot: &mut Option<T>, value: T) { *slot = Some(value); }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    install(&mut slot, database);
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
fn persistence_inventory_parameterizes_closure_mutation_effects() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let mut install = |value| slot = Some(value);
                    install(database);
                    consume(slot.unwrap().pool());
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let mut install = |value| slot = Some(value);
                    install(Clean);
                    consume(slot.unwrap().pool());
                    drop(database);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_projects_destructured_closure_parameters() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = |(value, _)| slot = Some(value);
                    install((Clean, database));
                    consume(slot.unwrap().pool());
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let install = |(value, _)| slot = Some(value);
                    install((database, Clean));
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_updates_projected_option_receivers() {
    let baseline = inventory(
        r#"
                struct Holder<T> { slot: Option<T> }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut holder = Holder { slot: None };
                    holder.slot.get_or_insert(database);
                    consume(holder.slot.as_ref().unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_applies_synthesized_mutable_helper_writes() {
    let baseline = inventory(
        r#"
                struct Holder { slot: Option<wow_database::CharacterDatabase> }
                fn make_database() -> wow_database::CharacterDatabase { unreachable!() }
                fn install(slot: &mut Option<wow_database::CharacterDatabase>) {
                    *slot = Some(make_database());
                }
                fn persistent() {
                    let mut slot = None;
                    install(&mut slot);
                    consume(slot.unwrap().pool());
                }
                fn install_holder(holder: &mut Holder) {
                    holder.slot = Some(make_database());
                }
                fn projected() {
                    let mut holder = Holder { slot: None };
                    install_holder(&mut holder);
                    consume(holder.slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn persistent", "fn projected"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_applies_expression_callee_closure_effects() {
    let baseline = inventory(
        r#"
                struct Callbacks<F> { install: F }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    let callbacks = Callbacks { install: || slot = Some(database) };
                    (callbacks.install)();
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
fn persistence_inventory_propagates_compound_assignment_writes() {
    let baseline = inventory(
        r#"
                struct Holder<T>(Option<T>);
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut holder = Holder(None);
                    holder += database;
                    consume(holder.0.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_applies_registered_mutable_method_effects() {
    let baseline = inventory(
        r#"
                struct Holder { slot: Option<wow_database::CharacterDatabase> }
                impl Holder {
                    fn install(&mut self) { self.slot = Some(unreachable!()); }
                }
                fn persistent(mut holder: Holder) {
                    holder.install();
                    consume(holder.slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_explicit_closure_return_values() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase, flag: bool) {
                    let choose = || {
                        if flag { return Some(database); }
                        None
                    };
                    consume(choose().unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}
