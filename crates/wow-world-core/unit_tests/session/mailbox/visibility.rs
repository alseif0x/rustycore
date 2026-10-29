use super::*;
use std::cell::Cell;

#[test]
fn unseen_create_deduplicates_and_never_discovers_a_failed_enqueue() {
    let visible = SharedClientVisibleGuidsLikeCpp::default();
    let corpse = ObjectGuid::new(14_i64 << 58, 1);
    let unrelated = ObjectGuid::create_player(1, 2);
    visible.insert(unrelated);
    let (tx, rx) = flume::bounded(1);
    tx.try_send(vec![0]).unwrap();
    assert!(!visible.publish_unseen_create_like_cpp(corpse, || tx.try_send(vec![1]).is_ok()));
    assert_eq!(visible.snapshot_like_cpp(), HashSet::from([unrelated]));
    assert_eq!(rx.try_recv().unwrap(), vec![0]);
    assert!(visible.publish_unseen_create_like_cpp(corpse, || tx.try_send(vec![1]).is_ok()));
    assert!(
        visible.publish_unseen_create_like_cpp(corpse, || panic!("known corpse was re-enqueued"))
    );
    assert_eq!(rx.try_iter().collect::<Vec<_>>(), vec![vec![1]]);
    drop(rx);
    let other = ObjectGuid::new(14_i64 << 58, 3);
    assert!(!visible.publish_unseen_create_like_cpp(other, || tx.try_send(vec![2]).is_ok()));
    assert_eq!(
        visible.snapshot_like_cpp(),
        HashSet::from([unrelated, corpse])
    );
}

#[test]
fn stale_full_scan_cannot_erase_or_recreate_a_concurrently_discovered_corpse() {
    let visible = SharedClientVisibleGuidsLikeCpp::default();
    let unrelated = ObjectGuid::create_player(1, 2);
    let corpse = ObjectGuid::new(14_i64 << 58, 1);
    visible.insert(unrelated);
    let before_map_query = visible.snapshot_like_cpp();
    assert!(visible.publish_unseen_create_like_cpp(corpse, || true));
    let published = Cell::new(false);
    assert!(
        visible
            .publish_transition_if_snapshot_like_cpp(
                &before_map_query,
                |_| false,
                [],
                || published.set(true)
            )
            .is_none()
    );
    assert!(!published.get());
    assert_eq!(
        visible.snapshot_like_cpp(),
        HashSet::from([unrelated, corpse])
    );
    let current = visible.snapshot_like_cpp();
    assert!(
        visible
            .publish_transition_if_snapshot_like_cpp(&current, |_| true, [], || published.set(true))
            .is_some()
    );
    assert!(published.get());
}

#[test]
fn simultaneous_birth_publishers_enqueue_only_one_create() {
    let visible = SharedClientVisibleGuidsLikeCpp::default();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let (tx, rx) = flume::bounded(2);
    let workers = (0..2)
        .map(|_| {
            let (visible, barrier, tx) = (visible.clone(), barrier.clone(), tx.clone());
            std::thread::spawn(move || {
                barrier.wait();
                visible.publish_unseen_create_like_cpp(ObjectGuid::new(14_i64 << 58, 1), || {
                    tx.try_send(vec![1]).is_ok()
                })
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        assert!(worker.join().unwrap());
    }
    assert_eq!(rx.try_iter().collect::<Vec<_>>(), vec![vec![1]]);
}
