//! Production-linked character-administration contract, without cfg(test) in wow-world.
//! C++ CharacterHandler.cpp:1550-1610 submits the query, then its ready callback
//! enqueues the transaction. The current Rust handler awaits both operations.
//! Pending-future cases characterize that existing fence, NOT C++ scheduling parity
//! or real database durability. Keep them explicit when introducing phase callbacks.

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
        Box::pin(async move {
            match completion {
                Some(receiver) => receiver.await.expect("controlled commit result"),
                None => CharacterAdministrationMutationOutcomeLikeCpp::Applied,
            }
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

#[tokio::test]
async fn character_rename_uses_production_port_in_load_commit_publication_order() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);
    let port = Arc::new(RenamePortFixtureLikeCpp::default());
    session.set_character_administration_persistence_port_like_cpp(port.clone());

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: "Newname".into(),
        })
        .await;

    assert_eq!(
        *port.trace.lock().unwrap(),
        vec![
            RenameTraceLikeCpp::Load {
                guid: 42,
                name: "Newname".into(),
            },
            RenameTraceLikeCpp::Commit {
                guid: 42,
                name: "Newname".into(),
                flags: AT_LOGIN_CUSTOMIZE_LIKE_CPP,
            },
        ]
    );
    let sent = send_rx.try_recv().expect("rename success");
    let mut packet = WorldPacket::from_bytes(&sent);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert!(packet.read_bit().unwrap());
}

#[test]
fn pending_query_keeps_current_handler_pending_without_committing_or_publishing() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        load_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    let mut handler = Box::pin(session.handle_character_rename_request(request()));

    assert!(poll_once(handler.as_mut()).is_pending());
    assert_eq!(
        port.trace.lock().unwrap().as_slice(),
        &[RenameTraceLikeCpp::Load {
            guid: 42,
            name: "Newname".into(),
        }]
    );
    assert!(send_rx.try_recv().is_err());
    // A second poll does not resubmit the query or advance to the commit.
    assert!(poll_once(handler.as_mut()).is_pending());
    assert_eq!(port.trace.lock().unwrap().len(), 1);

    complete.send(candidate()).unwrap();
    assert!(poll_once(handler.as_mut()).is_ready());
    assert_eq!(port.trace.lock().unwrap().len(), 2);
    let sent = send_rx.try_recv().expect("response only after completion");
    let mut packet = WorldPacket::from_bytes(&sent);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn cancelled_pending_query_cannot_start_a_transaction_or_publish_late() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_legit_characters(vec![request().guid]);
    let (complete, completion) = oneshot::channel();
    let port = Arc::new(RenamePortFixtureLikeCpp {
        load_completion: Mutex::new(Some(completion)),
        ..Default::default()
    });
    session.set_character_administration_persistence_port_like_cpp(port.clone());
    let mut handler = Box::pin(session.handle_character_rename_request(request()));
    assert!(poll_once(handler.as_mut()).is_pending());
    drop(handler);

    assert!(
        complete.send(candidate()).is_err(),
        "cancelled query result is not retained"
    );
    assert_eq!(port.trace.lock().unwrap().len(), 1);
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn pending_commit_preserves_existing_result_before_response_fence() {
    for applied in [false, true] {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        session.set_legit_characters(vec![request().guid]);
        let (complete, completion) = oneshot::channel();
        let port = Arc::new(RenamePortFixtureLikeCpp {
            commit_completion: Mutex::new(Some(completion)),
            ..Default::default()
        });
        session.set_character_administration_persistence_port_like_cpp(port.clone());
        let mut handler = Box::pin(session.handle_character_rename_request(request()));
        assert!(poll_once(handler.as_mut()).is_pending());
        assert_eq!(port.trace.lock().unwrap().len(), 2);
        assert!(
            send_rx.try_recv().is_err(),
            "no response before commit result"
        );
        assert!(poll_once(handler.as_mut()).is_pending());
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
        assert!(poll_once(handler.as_mut()).is_ready());
        let sent = send_rx.try_recv().expect("exactly one classified response");
        let mut packet = WorldPacket::from_bytes(&sent);
        assert_eq!(
            packet.server_opcode(),
            Some(ServerOpcodes::CharacterRenameResult)
        );
        packet.skip_opcode();
        let result = packet.read_uint8().unwrap();
        assert_eq!(
            result == 0,
            applied,
            "failed persistence must not publish success"
        );
        assert!(send_rx.try_recv().is_err());
    }
}
