//! Regressions for type and value flow.

use super::*;

#[test]
fn persistence_inventory_tracks_transforming_combinator_arguments() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(Some(0_u8).map(|_| database).unwrap().pool());
                }
                fn clean() {
                    consume(Some(0_u8).map(|_| 1_u8).unwrap());
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_tracks_result_producing_combinators() {
    let baseline = inventory(
        r#"
                fn and_then(database: wow_database::CharacterDatabase) {
                    consume(Some(()).and_then(|_| Some(database)).unwrap().pool());
                }
                fn map_or(
                    first: wow_database::CharacterDatabase,
                    second: wow_database::CharacterDatabase,
                ) {
                    consume(Some(()).map_or(first, |_| second).pool());
                }
                fn map_or_else(
                    first: wow_database::CharacterDatabase,
                    second: wow_database::CharacterDatabase,
                ) {
                    consume(Result::<(), ()>::Ok(()).map_or_else(|_| first, |_| second).pool());
                }
            "#,
    )
    .unwrap();

    for enclosing in ["fn and_then", "fn map_or", "fn map_or_else"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_records_nominal_container_argument_escapes() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                struct PlainHolder(u8);
                fn persistent(database: wow_database::CharacterDatabase) {
                    let holder = Holder(database);
                    send(holder);
                }
                fn clean() { send(PlainHolder(1_u8)); }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "send"
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_updates_destructuring_assignment_bindings() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut value = None;
                    (value,) = (Some(database),);
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
fn persistence_inventory_updates_struct_destructuring_assignment_bindings() {
    let baseline = inventory(
        r#"
                struct Holder { value: wow_database::CharacterDatabase, clean: u8 }
                fn persistent(holder: Holder) {
                    let mut slot = unreachable!();
                    Holder { value: slot, .. } = holder;
                    consume(slot.pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_updates_supported_places_in_mixed_assignments() {
    let baseline = inventory(
        r#"
                struct Holder { field: u8 }
                fn persistent(
                    database: wow_database::CharacterDatabase,
                    mut holder: Holder,
                ) {
                    let mut value = None;
                    (holder.field, value) = (1_u8, Some(database));
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
fn persistence_inventory_retains_values_inserted_through_ufcs() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut values = Vec::new();
                    Vec::push(&mut values, database);
                    consume(values[0].pool());
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let mut values = Vec::new();
                    Vec::push(&mut values, Clean);
                    consume(values[0].pool());
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
fn persistence_inventory_propagates_standard_replacement_writes() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let mut slot = None;
                    std::mem::replace(&mut slot, Some(database));
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
fn persistence_inventory_preserves_values_returned_by_standard_replacements() {
    let baseline = inventory(
        r#"
                fn replace(database: wow_database::CharacterDatabase) {
                    let mut slot = Some(database);
                    let previous = std::mem::replace(&mut slot, None);
                    consume(previous.unwrap().pool());
                }
                fn take(database: wow_database::CharacterDatabase) {
                    let mut slot = Some(database);
                    let previous = std::mem::take(&mut slot);
                    consume(previous.unwrap().pool());
                }
                fn cleared(database: wow_database::CharacterDatabase) {
                    let mut slot = Some(database);
                    drop(std::mem::take(&mut slot));
                    consume(slot.unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn replace", "fn take"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn cleared" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_propagates_assignments_into_places() {
    let baseline = inventory(
        r#"
                struct Holder<T> { value: T }
                fn field(database: wow_database::CharacterDatabase) {
                    let mut holder = Holder { value: None };
                    holder.value = Some(database);
                    consume(holder.value.unwrap().pool());
                }
                fn index(database: wow_database::CharacterDatabase) {
                    let mut values = [None];
                    values[0] = Some(database);
                    consume(values[0].unwrap().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn field", "fn index"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::StoreEscape
        }));
    }
}

#[test]
fn persistence_inventory_retains_block_local_tail_values() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let wrapped = { let alias = database; alias };
                    consume(wrapped.pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_standard_wrapper_constructor_flow() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let guarded = std::sync::Mutex::new(database);
                    consume(guarded.into_inner().unwrap().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_standard_identity_flow() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(std::convert::identity(database).pool());
                }
                use std::convert::identity;
                fn imported(database: wow_database::CharacterDatabase) {
                    consume(identity(database).pool());
                }
                mod custom {
                    pub fn identity(value: bool) -> bool { value }
                }
                fn clean(value: bool) {
                    consume(custom::identity(value));
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn persistent", "fn imported"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_retains_struct_literal_nominal_fields() {
    let baseline = inventory(
        r#"
                struct Holder {
                    database: wow_database::CharacterDatabase,
                    clean: bool,
                }
                fn clean(database: wow_database::CharacterDatabase) {
                    let holder = Holder { database, clean: false };
                    consume(holder.clean);
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let holder = Holder { database, clean: false };
                    consume(holder.database.pool());
                }
                fn side_effect(database: wow_database::CharacterDatabase) {
                    let holder = Holder {
                        database: wow_database::CharacterDatabase::default(),
                        clean: { drop(database); false },
                    };
                    consume(holder.clean);
                }
                struct Decoded {
                    value: u32,
                    clean: bool,
                }
                fn decoded(result: wow_database::SqlResult) {
                    let decoded = Decoded {
                        value: result.try_read(0).unwrap_or(0),
                        clean: false,
                    };
                    consume(decoded.value);
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean"
            && matches!(
                row.operation,
                PersistenceOperation::ArgumentEscape | PersistenceOperation::PoolAccess
            )
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn side_effect"
            && row.symbol == "consume"
            && row.operation == PersistenceOperation::ArgumentEscape
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn decoded"
            && row.target == PersistenceTarget::SqlResult
            && row.operation == PersistenceOperation::ArgumentEscape
    }));
}
