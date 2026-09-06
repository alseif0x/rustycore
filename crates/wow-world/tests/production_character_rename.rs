//! Production-linked character-administration contract, without cfg(test) in wow-world.
//! C++ CharacterHandler.cpp:1550-1610 submits a read and admits a commit only in
//! its ready callback. Rust retains its result-before-publication fence.
//! Controlled futures test production Session continuations, not real DB durability.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tokio::sync::oneshot;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;
use wow_packet::packets::character::CharacterRenameRequest;
use wow_persistence::{
    CharacterAdministrationLoadOutcomeLikeCpp, CharacterAdministrationMutationOutcomeLikeCpp,
    CharacterAdministrationPersistencePortLikeCpp, CharacterCreatePersistenceRequestLikeCpp,
    CharacterCustomizationPersistenceLikeCpp, CharacterCustomizeCandidateLikeCpp,
    CharacterRenameCandidateLikeCpp, PersistenceFutureLikeCpp,
};
use wow_world::WorldSession;

type LoadResult = CharacterAdministrationLoadOutcomeLikeCpp<CharacterRenameCandidateLikeCpp>;
type CommitResult = CharacterAdministrationMutationOutcomeLikeCpp;

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_packet_tx, packet_rx) = flume::bounded(1);
    let (send_tx, send_rx) = flume::bounded(capacity);
    let session = WorldSession::new(
        1,
        "RenameContract".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "esES".into(),
        packet_rx,
        send_tx,
    );
    (session, send_rx)
}

fn candidate() -> LoadResult {
    CharacterAdministrationLoadOutcomeLikeCpp::Loaded(CharacterRenameCandidateLikeCpp {
        old_name: "Oldname".into(),
        at_login_flags: AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_CUSTOMIZE_LIKE_CPP,
    })
}

fn request() -> CharacterRenameRequest {
    CharacterRenameRequest {
        guid: ObjectGuid::create_player(1, 42),
        new_name: "Newname".into(),
    }
}

fn poll_once<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;
const AT_LOGIN_CUSTOMIZE_LIKE_CPP: u16 = 0x008;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RenameTraceLikeCpp {
    Load { guid: u64, name: String },
    Commit { guid: u64, name: String, flags: u16 },
}

#[derive(Default)]
struct RenamePortFixtureLikeCpp {
    trace: Mutex<Vec<RenameTraceLikeCpp>>,
    load_completion: Mutex<Option<oneshot::Receiver<LoadResult>>>,
    commit_completion: Mutex<Option<oneshot::Receiver<CommitResult>>>,
    commit_returned: Arc<std::sync::atomic::AtomicBool>,
    commits_returned: Arc<std::sync::atomic::AtomicUsize>,
}

impl CharacterAdministrationPersistencePortLikeCpp for RenamePortFixtureLikeCpp {
    fn find_character_name_like_cpp(
        &self,
        _name: &str,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationLoadOutcomeLikeCpp<()>> {
        panic!("rename test must not perform create admission")
    }

    fn load_account_character_count_like_cpp(
        &self,
        _account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationLoadOutcomeLikeCpp<u64>> {
        panic!("rename test must not perform create admission")
    }

    fn create_character_like_cpp(
        &self,
        _request: CharacterCreatePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        panic!("rename test must not create a character")
    }

    fn delete_owned_character_like_cpp(
        &self,
        _guid: u64,
        _account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        panic!("rename test must not delete a character")
    }

    fn load_rename_candidate_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CharacterAdministrationLoadOutcomeLikeCpp<CharacterRenameCandidateLikeCpp>,
    > {
        self.trace.lock().unwrap().push(RenameTraceLikeCpp::Load {
            guid,
            name: new_name.into(),
        });
        let completion = self.load_completion.lock().unwrap().take();
        Box::pin(async move {
            match completion {
                Some(receiver) => receiver.await.expect("controlled load result"),
                None => candidate(),
            }
        })
    }

    fn commit_rename_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
        at_login_flags: u16,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        self.trace.lock().unwrap().push(RenameTraceLikeCpp::Commit {
            guid,
            name: new_name.into(),
            flags: at_login_flags,
        });
        let completion = self.commit_completion.lock().unwrap().take();
        let returned = self.commit_returned.clone();
        let returned_count = self.commits_returned.clone();
        Box::pin(async move {
            let result = match completion {
                Some(receiver) => receiver.await.expect("controlled commit result"),
                None => CharacterAdministrationMutationOutcomeLikeCpp::Applied,
            };
            returned.store(true, std::sync::atomic::Ordering::SeqCst);
            returned_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            result
        })
    }

    fn load_customize_candidate_like_cpp(
        &self,
        _guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CharacterAdministrationLoadOutcomeLikeCpp<CharacterCustomizeCandidateLikeCpp>,
    > {
        panic!("rename test must not customize a character")
    }

    fn commit_customize_like_cpp(
        &self,
        _guid: u64,
        _name: &str,
        _at_login_flags: u16,
        _customizations: Vec<CharacterCustomizationPersistenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        panic!("rename test must not customize a character")
    }
}

async fn wait_for_trace(port: &RenamePortFixtureLikeCpp, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while port.trace.lock().unwrap().len() < count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("worker made the expected port call");
}

async fn response(session: &mut WorldSession, receive: &flume::Receiver<Vec<u8>>) -> Vec<u8> {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            session.process_ready_character_rename_callbacks_like_cpp();
            if let Ok(packet) = receive.try_recv() {
                return packet;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("ready callback response")
}

fn assert_response(bytes: &[u8], code: u8) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_uint8().unwrap(), code);
    assert_eq!(packet.read_bit().unwrap(), code == 0);
}

async fn wait_for_commit(session: &mut WorldSession, port: &RenamePortFixtureLikeCpp) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while port.trace.lock().unwrap().len() < 2 {
            session.process_ready_character_rename_callbacks_like_cpp();
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("ready query admitted a commit");
}

#[tokio::test]
async fn character_rename_uses_production_port_in_load_commit_publication_order() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let port = Arc::new(RenamePortFixtureLikeCpp::default());
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    assert!(send_rx.try_recv().is_err(), "handler does not publish");
    let bytes = response(&mut session, &send_rx).await;
    assert_response(&bytes, 0);
    assert_eq!(
        *port.trace.lock().unwrap(),
        vec![
            RenameTraceLikeCpp::Load {
                guid: 42,
                name: "Newname".into()
            },
            RenameTraceLikeCpp::Commit {
                guid: 42,
                name: "Newname".into(),
                flags: AT_LOGIN_CUSTOMIZE_LIKE_CPP
            },
        ]
    );
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn pending_query_does_not_hold_session_or_block_another_session() {
    let (mut slow, slow_rx) = make_session_with_send_capacity(2);
    slow.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        load_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    slow.set_character_administration_persistence_port_like_cpp(port.clone());
    slow.handle_character_rename_request(request()).await;
    wait_for_trace(&port, 1).await;
    slow.process_ready_character_rename_callbacks_like_cpp();
    assert_eq!(port.trace.lock().unwrap().len(), 1);
    assert!(slow_rx.try_recv().is_err());

    let (mut fast, fast_rx) = make_session_with_send_capacity(2);
    fast.set_legit_characters(vec![request().guid]);
    fast.set_character_administration_persistence_port_like_cpp(Arc::new(
        RenamePortFixtureLikeCpp::default(),
    ));
    fast.handle_character_rename_request(request()).await;
    assert_response(&response(&mut fast, &fast_rx).await, 0);
    assert!(
        slow_rx.try_recv().is_err(),
        "unrelated progress cannot complete the pending query"
    );

    complete.send(candidate()).unwrap();
    assert_response(&response(&mut slow, &slow_rx).await, 0);
    assert!(slow.finish_character_rename_callbacks_like_cpp().await);
    assert!(fast.finish_character_rename_callbacks_like_cpp().await);
}

#[tokio::test]
async fn retired_reads_cannot_start_a_transaction_even_if_the_result_is_available() {
    for ready in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        session.set_legit_characters(vec![request().guid]);
        let (complete, completion) = oneshot::channel();
        let port = Arc::new(RenamePortFixtureLikeCpp {
            load_completion: Mutex::new(Some(completion)),
            ..Default::default()
        });
        session.set_character_administration_persistence_port_like_cpp(port.clone());
        session.handle_character_rename_request(request()).await;
        wait_for_trace(&port, 1).await;
        if ready {
            complete.send(candidate()).unwrap();
            tokio::task::yield_now().await;
        } else {
            drop(complete);
        }
        assert!(session.finish_character_rename_callbacks_like_cpp().await);
        session.process_ready_character_rename_callbacks_like_cpp();
        tokio::task::yield_now().await;
        assert_eq!(
            port.trace.lock().unwrap().len(),
            1,
            "no callback may admit a commit"
        );
        assert!(send_rx.try_recv().is_err());
    }
}

#[tokio::test]
async fn pending_commit_preserves_existing_result_before_response_fence() {
    for applied in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        session.set_legit_characters(vec![request().guid]);
        let (complete, completion) = oneshot::channel();
        let port = Arc::new(RenamePortFixtureLikeCpp {
            commit_completion: Mutex::new(Some(completion)),
            ..Default::default()
        });
        session.set_character_administration_persistence_port_like_cpp(port.clone());
        session.handle_character_rename_request(request()).await;
        wait_for_commit(&mut session, &port).await;
        session.process_ready_character_rename_callbacks_like_cpp();
        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            port.trace.lock().unwrap().len(),
            2,
            "no repeated transaction"
        );
        complete
            .send(if applied {
                CommitResult::Applied
            } else {
                CommitResult::Failed {
                    reason: "controlled commit failure".into(),
                }
            })
            .unwrap();
        assert_response(
            &response(&mut session, &send_rx).await,
            if applied { 0 } else { 25 },
        );
        assert!(session.finish_character_rename_callbacks_like_cpp().await);
        assert!(send_rx.try_recv().is_err());
    }
}

#[tokio::test]
async fn cancelled_drain_retains_submitted_commit_and_resumes_without_publication() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        commit_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    wait_for_commit(&mut session, &port).await;

    let mut drain = Box::pin(session.finish_character_rename_callbacks_like_cpp());
    assert!(
        poll_once(drain.as_mut()).is_pending(),
        "submission is not completion"
    );
    drop(drain);
    complete.send(CommitResult::Applied).unwrap();
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
    session.process_ready_character_rename_callbacks_like_cpp();
    assert_eq!(port.trace.lock().unwrap().len(), 2);
    assert!(
        send_rx.try_recv().is_err(),
        "retirement never publishes a late success"
    );
}

#[tokio::test]
async fn rejected_query_outcomes_never_start_rename_commit() {
    for result in [
        LoadResult::NotFound,
        LoadResult::Failed {
            reason: "controlled query failure".into(),
        },
        LoadResult::Loaded(CharacterRenameCandidateLikeCpp {
            old_name: "Oldname".into(),
            at_login_flags: AT_LOGIN_CUSTOMIZE_LIKE_CPP,
        }),
    ] {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        session.set_legit_characters(vec![request().guid]);
        let (complete, completion) = oneshot::channel();
        let port = Arc::new(RenamePortFixtureLikeCpp {
            load_completion: Mutex::new(Some(completion)),
            ..Default::default()
        });
        session.set_character_administration_persistence_port_like_cpp(port.clone());
        complete.send(result).unwrap();
        session.handle_character_rename_request(request()).await;
        assert_response(&response(&mut session, &send_rx).await, 25);
        assert_eq!(port.trace.lock().unwrap().len(), 1);
        assert!(session.finish_character_rename_callbacks_like_cpp().await);
        assert!(send_rx.try_recv().is_err());
    }
}

#[tokio::test]
async fn lost_commit_worker_is_not_a_clean_retirement() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        commit_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    wait_for_commit(&mut session, &port).await;
    // The fixture panics if its submitted commit loses its completion source.
    // The supervisor must classify that JoinError, never invent Applied/rollback.
    drop(complete);
    assert!(!session.finish_character_rename_callbacks_like_cpp().await);
    assert!(!session.finish_character_rename_callbacks_like_cpp().await);
    assert_eq!(port.trace.lock().unwrap().len(), 2);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn saturated_callback_delivery_returns_and_retries_without_repeating_commit() {
    let (mut session, send_rx) = make_session_with_send_capacity(0);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        commit_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    wait_for_commit(&mut session, &port).await;
    complete.send(CommitResult::Applied).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !port
            .commit_returned
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    // Isolate any accidental synchronous send. The timeout branch releases the
    // rendezvous and joins the actual worker before failing, so old code cannot
    // strand a blocking thread or deadlock this test runtime.
    let mut pass = tokio::task::spawn_blocking(move || {
        session.process_ready_character_rename_callbacks_like_cpp();
        session
    });
    let mut session =
        match tokio::time::timeout(std::time::Duration::from_millis(250), &mut pass).await {
            Ok(result) => result.unwrap(),
            Err(_) => {
                let _ = send_rx.recv_async().await.unwrap();
                drop(pass.await.unwrap());
                panic!("ready callback blocked on saturated socket capacity");
            }
        };
    assert_eq!(port.trace.lock().unwrap().len(), 2);
    let bytes = send_rx
        .try_recv()
        .expect("reserved channel send is retained, not lost");
    session.process_ready_character_rename_callbacks_like_cpp();
    assert_response(&bytes, 0);
    session.process_ready_character_rename_callbacks_like_cpp();
    assert!(send_rx.try_recv().is_err(), "no repeated response");
    assert_eq!(port.trace.lock().unwrap().len(), 2, "no repeated commit");
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
}

#[tokio::test]
async fn closed_callback_sink_retires_session_without_replaying_the_commit() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let port = Arc::new(RenamePortFixtureLikeCpp::default());
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    drop(send_rx);
    session.handle_character_rename_request(request()).await;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !session.is_disconnecting() {
            session.process_ready_character_rename_callbacks_like_cpp();
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("closed callback sink retires the Session");
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
    assert_eq!(port.trace.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn failed_active_worker_stops_read_admission_and_cannot_publish_success() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        commit_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    wait_for_commit(&mut session, &port).await;
    let (read_complete, read_completion) = oneshot::channel();
    *port.load_completion.lock().unwrap() = Some(read_completion);
    session.handle_character_rename_request(request()).await;
    wait_for_trace(&port, 3).await;
    drop(complete); // the first commit worker fails, not a classified DB rejection
    let retired = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !session.is_disconnecting() {
            session.process_ready_character_rename_callbacks_like_cpp();
            tokio::task::yield_now().await;
        }
    })
    .await;
    // Always drain before asserting a timeout, including on the previous code.
    assert!(!session.finish_character_rename_callbacks_like_cpp().await);
    retired.expect("worker loss must retire the active Session");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !read_complete.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("read worker cancellation completes");
    assert!(
        read_complete.send(candidate()).is_err(),
        "unadmitted read is cancelled"
    );
    assert_eq!(port.trace.lock().unwrap().len(), 3, "no second transaction");
    assert!(
        send_rx.try_recv().is_err(),
        "unknown worker failure is not a response success"
    );
}

#[tokio::test]
async fn reserved_rename_response_precedes_a_later_packet_from_the_same_session() {
    use wow_packet::packets::character::DeleteChar;
    let (mut session, receive) = make_session_with_send_capacity(1);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        commit_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    wait_for_commit(&mut session, &port).await;
    assert!(session.send_packet(&DeleteChar { code: 0 })); // occupy channel capacity
    complete.send(CommitResult::Applied).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !port
            .commit_returned
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    session.process_ready_character_rename_callbacks_like_cpp();
    receive.try_recv().expect("remove the earlier sentinel");

    // A later ordinary send shares the same channel. Isolate its legitimate
    // blocking behavior so the test can drain it and join the producer.
    let later = tokio::task::spawn_blocking(move || {
        assert!(session.send_packet(&DeleteChar { code: 1 }));
        session
    });
    let first = receive.recv_async().await.unwrap();
    let mut session = later.await.unwrap();
    session.process_ready_character_rename_callbacks_like_cpp();
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
    assert_response(&first, 0);
    let last = receive.try_recv().expect("later packet follows rename");
    let mut last = WorldPacket::from_bytes(&last);
    last.skip_opcode();
    assert_eq!(last.read_uint8().unwrap(), 1);
    assert!(receive.try_recv().is_err());
    assert_eq!(port.trace.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn entire_ready_batch_reserves_fifo_positions_before_a_later_packet() {
    use wow_packet::ServerPacket;
    use wow_packet::packets::character::{CharacterRenameResult, DeleteChar};
    let (mut session, receive) = make_session_with_send_capacity(1);
    let second_guid = ObjectGuid::create_player(1, 43);
    session.set_legit_characters(vec![request().guid, second_guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        commit_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    session.handle_character_rename_request(request()).await;
    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid: second_guid,
            new_name: "Another".into(),
        })
        .await;
    wait_for_trace(&port, 2).await; // both current-thread read tasks completed
    session.process_ready_character_rename_callbacks_like_cpp();
    wait_for_trace(&port, 4).await;
    assert!(session.send_packet(&DeleteChar { code: 0 }));
    complete.send(CommitResult::Applied).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while port
            .commits_returned
            .load(std::sync::atomic::Ordering::SeqCst)
            < 2
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    session.process_ready_character_rename_callbacks_like_cpp();
    receive.try_recv().expect("earlier sentinel");
    let later = tokio::task::spawn_blocking(move || {
        assert!(session.send_packet(&DeleteChar { code: 1 }));
        session
    });
    let first = receive.recv_async().await.unwrap();
    let second = receive.recv_async().await.unwrap();
    let mut session = later.await.unwrap();
    session.process_ready_character_rename_callbacks_like_cpp();
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
    assert_eq!(
        first,
        CharacterRenameResult {
            result: 0,
            name: "Newname".into(),
            guid: Some(request().guid),
        }
        .to_bytes()
    );
    assert_eq!(
        second,
        CharacterRenameResult {
            result: 0,
            name: "Another".into(),
            guid: Some(second_guid),
        }
        .to_bytes()
    );
    assert_eq!(
        receive.try_recv().unwrap(),
        DeleteChar { code: 1 }.to_bytes()
    );
    assert!(receive.try_recv().is_err());
    assert_eq!(port.trace.lock().unwrap().len(), 4);
}
