//! Canonical reservation/generator ownership, without database connections.
//! The coupled SQL writer/transaction lifetime requires real DB acceptance.
use super::*;
use std::{future::Future, sync::Arc, task::Poll};
use tokio::sync::Mutex;

async fn operation() -> (
    Box<CreationReservation>,
    Arc<Mutex<super::super::CreationIds>>,
) {
    let ids = Arc::new(Mutex::new(super::super::CreationIds {
        player: Some(18),
        item: Some(41),
        recovery_required: false,
    }));
    let guard = ids.clone().lock_owned().await;
    let operation = Box::new(CreationReservation {
        ids: guard,
        identity: CreationIdentity {
            guid: 17,
            account: 19,
            realm: 0x02010001,
        },
        item_start: 41,
        name: "Synthetic".into(),
        realm_count: 0,
    });
    (operation, ids)
}

#[tokio::test]
async fn constructor_identity_and_item_allocations_share_the_owned_writer_gate() {
    let (mut operation, ids) = operation().await;
    assert_eq!(
        operation.identity(),
        CreationIdentity {
            guid: 17,
            account: 19,
            realm: 0x02010001
        }
    );
    assert_eq!(operation.next_item_guid(), Ok(41));
    assert_eq!(operation.next_item_guid(), Ok(42));
    assert!(ids.try_lock().is_err());
    let mut waiter = Box::pin(ids.clone().lock_owned());
    assert!(std::future::poll_fn(|cx| Poll::Ready(waiter.as_mut().poll(cx).is_pending())).await);
    drop(operation);
    let guard = waiter.await;
    assert_eq!(guard.player, Some(18));
    assert_eq!(guard.item, Some(43));
    assert!(!guard.recovery_required);
}

#[tokio::test]
async fn dropping_an_unpolled_reservation_use_releases_its_owned_gate() {
    let (operation, ids) = operation().await;
    let work = async move {
        let _operation = operation;
        std::future::pending::<()>().await
    };
    drop(work);
    let guard = ids.try_lock().unwrap();
    assert!(!guard.recovery_required);
    assert_eq!(guard.player, Some(18));
}

fn character() -> NewCharacter {
    let mut character = super::super::tests::character();
    character.guid = 17;
    character.account = 19;
    character
}

#[tokio::test]
async fn reserved_identity_and_name_cannot_be_substituted_at_the_save_boundary() {
    for field in 0..4 {
        let (operation, ids) = operation().await;
        let mut character = character();
        match field {
            0 => character.guid = 18,
            1 => character.account = 20,
            2 => character.realm = 0x02010002,
            _ => character.name = "Other".into(),
        }
        assert_eq!(operation.validate(&character), Err(CreateError::InvalidRow));
        drop(operation);
        assert!(!ids.try_lock().unwrap().recovery_required);
    }
}

#[tokio::test]
async fn an_item_identity_must_have_been_allocated_by_this_operation() {
    for (allocations, supplied) in [(0, 41), (2, 40), (2, 43)] {
        let (mut operation, ids) = operation().await;
        for _ in 0..allocations {
            operation.next_item_guid().unwrap();
        }
        let mut character = character();
        let mut item = super::super::items::tests::item();
        item.guid = supplied;
        character.items.push(item);
        assert_eq!(operation.validate(&character), Err(CreateError::InvalidRow));
        drop(operation);
        assert!(!ids.try_lock().unwrap().recovery_required);
    }
}

#[tokio::test]
async fn invalid_login_timestamp_and_short_taxi_routes_fail_before_database_work() {
    for invalid in 0..3 {
        let (operation, ids) = operation().await;
        let mut character = character();
        if invalid == 0 {
            character.state.save_time = u64::from(u32::MAX) + 1;
        } else {
            character.state.taxi_path =
                Some(wow_persistence::forever::creation::save::NewTaxiPath {
                    flight_master_faction: 1,
                    nodes: if invalid == 1 { vec![] } else { vec![1] },
                });
        }
        assert_eq!(operation.validate(&character), Err(CreateError::InvalidRow));
        drop(operation);
        assert!(!ids.try_lock().unwrap().recovery_required);
    }
}

#[test]
fn counters_advance_past_other_owners_without_reusing_local_unpersisted_ids() {
    let mut counter = Some(100);
    synchronize_counter(&mut counter, Some(41)).unwrap();
    assert_eq!(counter, Some(100));
    synchronize_counter(&mut counter, Some(200)).unwrap();
    assert_eq!(counter, Some(201));
    assert_eq!(
        synchronize_counter(&mut counter, Some(u64::MAX)),
        Err(CreateError::InvalidRow)
    );
    assert_eq!(counter, Some(201));
    let mut counter = None;
    synchronize_counter(&mut counter, None).unwrap();
    assert_eq!(counter, Some(1));
}
