//! Read-ready/write admission contract used by the production rename operation.
//! Controlled port futures prove staging, not real SQL cancellation or durability.

use super::*;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};
use tokio::sync::oneshot;
use wow_persistence::{
    CharacterCreatePersistenceRequestLikeCpp, CharacterCustomizationPersistenceLikeCpp,
    CharacterCustomizeCandidateLikeCpp, CharacterRenameCandidateLikeCpp, PersistenceFutureLikeCpp,
};

type Candidate = LoadOutcome<CharacterRenameCandidateLikeCpp>;

struct Port {
    candidate: Mutex<Option<oneshot::Receiver<Candidate>>>,
    commits: Mutex<Vec<(u64, String, u16)>>,
}

impl CharacterAdministrationPersistencePortLikeCpp for Port {
    fn find_character_name_like_cpp(
        &self,
        _: &str,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<()>> {
        panic!("not a rename operation")
    }

    fn load_account_character_count_like_cpp(
        &self,
        _: u32,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<u64>> {
        panic!("not a rename operation")
    }

    fn create_character_like_cpp(
        &self,
        _: CharacterCreatePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        panic!("not a rename operation")
    }

    fn delete_owned_character_like_cpp(
        &self,
        _: u64,
        _: u32,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        panic!("not a rename operation")
    }

    fn load_rename_candidate_like_cpp(
        &self,
        guid: u64,
        name: &str,
    ) -> PersistenceFutureLikeCpp<'_, Candidate> {
        assert_eq!((guid, name), (42, "Newname"));
        let completion = self.candidate.lock().unwrap().take().expect("one read");
        Box::pin(async move { completion.await.expect("controlled read completion") })
    }

    fn commit_rename_like_cpp(
        &self,
        guid: u64,
        name: &str,
        flags: u16,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        self.commits
            .lock()
            .unwrap()
            .push((guid, name.into(), flags));
        Box::pin(async { MutationOutcome::Applied })
    }

    fn load_customize_candidate_like_cpp(
        &self,
        _: u64,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<CharacterCustomizeCandidateLikeCpp>> {
        panic!("not a rename operation")
    }

    fn commit_customize_like_cpp(
        &self,
        _: u64,
        _: &str,
        _: u16,
        _: Vec<CharacterCustomizationPersistenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        panic!("not a rename operation")
    }
}

fn fixture() -> (Arc<Port>, oneshot::Sender<Candidate>, RenameRequest) {
    let (send, receive) = oneshot::channel();
    (
        Arc::new(Port {
            candidate: Mutex::new(Some(receive)),
            commits: Mutex::new(Vec::new()),
        }),
        send,
        RenameRequest {
            guid: 42,
            new_name: "Newname".into(),
        },
    )
}

fn candidate() -> Candidate {
    LoadOutcome::Loaded(CharacterRenameCandidateLikeCpp {
        old_name: "Oldname".into(),
        at_login_flags: 0x009,
    })
}

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
    assert!(port.commits.lock().unwrap().is_empty());
    complete.send(candidate()).unwrap();
    let Poll::Ready(RenamePreparation::Ready(prepared)) = poll_once(query.as_mut()) else {
        panic!("read must yield a prepared, unsubmitted operation");
    };
    drop(query);
    assert!(port.commits.lock().unwrap().is_empty());

    let mut commit = Box::pin(owned_send_future(prepared.commit()));
    assert!(
        port.commits.lock().unwrap().is_empty(),
        "future construction is not submission"
    );
    let Poll::Ready(outcome) = poll_once(commit.as_mut()) else {
        panic!("fixture commit is ready");
    };
    assert_eq!(
        port.commits.lock().unwrap().as_slice(),
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
        assert!(port.commits.lock().unwrap().is_empty());
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
    assert!(port.commits.lock().unwrap().is_empty());
}

#[tokio::test]
async fn production_session_driver_executes_ready_rename_callbacks() {
    use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};
    use wow_core::ObjectGuid;
    use wow_packet::packets::character::CharacterRenameRequest;

    let (port, complete, _) = fixture();
    let (_packet_tx, packet_rx) = flume::bounded(1);
    let (send_tx, send_rx) = flume::bounded(2);
    let mut session = WorldSession::new(
        1,
        "RenameDriver".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "esES".into(),
        packet_rx,
        send_tx,
    );
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    let catalogs = SessionHandlerCatalogsLikeCpp::default();
    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: "Newname".into(),
        })
        .await;

    // Drive the actual Session pass, not the standalone callback adapter. A
    // pending query must not keep this future pending or submit the transaction.
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        session.process_pending_with_catalogs_like_cpp(&catalogs),
    )
    .await
    .expect("pending DB read does not block the driver");
    assert!(port.commits.lock().unwrap().is_empty());
    assert!(send_rx.try_recv().is_err());
    complete.send(candidate()).unwrap();

    let bytes = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            session
                .process_pending_with_catalogs_like_cpp(&catalogs)
                .await;
            if let Ok(bytes) = send_rx.try_recv() {
                break bytes;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("real driver publishes the completed rename");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::CharacterRenameResult)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert_eq!(port.commits.lock().unwrap().len(), 1);
    assert!(session.finish_character_rename_callbacks_like_cpp().await);
}
