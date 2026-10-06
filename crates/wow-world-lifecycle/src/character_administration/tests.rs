//! Read-ready/write admission contract used by the production rename operation.
//! Controlled port futures prove staging, not real SQL cancellation or durability.

use super::test_fixture::{candidate, fixture};
use super::*;
use std::future::Future;
use std::task::{Context, Poll, Waker};

fn poll_once<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

fn owned_send_future<F: Future + Send + 'static>(future: F) -> F {
    future
}

#[test]
fn ready_read_requires_explicit_consumption_before_any_commit() {
    let (port, complete, request) = fixture();
    let mut query = Box::pin(owned_send_future(prepare_rename(port.clone(), request)));
    assert!(poll_once(query.as_mut()).is_pending());
    assert!(poll_once(query.as_mut()).is_pending());
    assert!(port.commit_snapshot().is_empty());
    complete.send(candidate()).unwrap();
    let Poll::Ready(RenamePreparation::Ready(prepared)) = poll_once(query.as_mut()) else {
        panic!("read must yield a prepared, unsubmitted operation");
    };
    drop(query);
    assert!(port.commit_snapshot().is_empty());

    let mut commit = Box::pin(owned_send_future(prepared.commit()));
    assert!(
        port.commit_snapshot().is_empty(),
        "future construction is not submission"
    );
    let Poll::Ready(outcome) = poll_once(commit.as_mut()) else {
        panic!("fixture commit is ready");
    };
    assert_eq!(
        port.commit_snapshot().as_slice(),
        &[(42, "Newname".into(), 0x008)]
    );
    assert_eq!(outcome.new_name, "Newname");
    assert!(matches!(outcome.result, Ok(old_name) if old_name == "Oldname"));
}

#[test]
fn retiring_read_before_or_after_readiness_never_submits_a_transaction() {
    for ready in [false, true] {
        let (port, complete, request) = fixture();
        let mut query = Box::pin(prepare_rename(port.clone(), request));
        assert!(poll_once(query.as_mut()).is_pending());
        if ready {
            complete.send(candidate()).unwrap();
            let Poll::Ready(RenamePreparation::Ready(prepared)) = poll_once(query.as_mut()) else {
                panic!("ready candidate");
            };
            drop(prepared);
            drop(query);
        } else {
            drop(query);
            assert!(complete.send(candidate()).is_err());
        }
        assert!(port.commit_snapshot().is_empty());
    }
}

#[test]
fn discarding_unpolled_commit_continuation_does_not_submit_it() {
    let (port, complete, request) = fixture();
    complete.send(candidate()).unwrap();
    let mut query = Box::pin(prepare_rename(port.clone(), request));
    let Poll::Ready(RenamePreparation::Ready(prepared)) = poll_once(query.as_mut()) else {
        panic!("ready candidate");
    };
    drop(prepared.commit());
    assert!(port.commit_snapshot().is_empty());
}
