use crate::session::{SessionCore, SessionState};
use std::sync::mpsc;
use std::time::Duration;
use wow_packet::packets::misc::{DeathReleaseLoc, LogoutComplete};

#[test]
fn disconnecting_rejects_publication_with_empty_and_full_separate_queues() {
    for full in [false, true] {
        for api in 0..5 {
            let (instance_tx, instance_rx) = flume::bounded(1);
            let (realm_tx, realm_rx) = flume::bounded(1);
            if full {
                instance_tx.try_send(vec![0x11]).unwrap();
                realm_tx.try_send(vec![0x22]).unwrap();
            }
            let (completed_tx, completed_rx) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let (_input_tx, input_rx) = flume::bounded(1);
                let (command_tx, command_rx) = flume::bounded(1);
                let mut core = SessionCore::new_for_world_session_like_cpp(
                    1,
                    "DisconnectedPublication".into(),
                    0,
                    2,
                    2,
                    54261,
                    "enUS".into(),
                    command_tx,
                    command_rx,
                    wow_session::SessionConnection::new(instance_tx, input_rx),
                    Vec::new(),
                );
                core.install_realm_send_channel_for_test(realm_tx);
                core.state = SessionState::Disconnecting;
                let accepted = match api {
                    0 => Some(core.send_packet(&LogoutComplete)),
                    1 => Some(core.try_send_packet(&LogoutComplete)),
                    2 => {
                        core.send_raw_packet(&[0x33]);
                        None
                    }
                    3 => {
                        core.send_packet_realm(&DeathReleaseLoc::cleared_like_cpp());
                        None
                    }
                    4 => {
                        core.send_raw_packet_realm(&[0x44]);
                        None
                    }
                    _ => unreachable!(),
                };
                completed_tx.send(accepted).unwrap();
            });
            let completed = completed_rx.recv_timeout(Duration::from_secs(5));
            if let Err(error) = completed {
                // Closing both queues releases a regressed blocking sender
                // before joining, so this regression cannot hang the suite.
                drop(instance_rx);
                drop(realm_rx);
                let _ = worker.join();
                panic!("publication API {api}, full={full}, did not return: {error}");
            }
            worker.join().expect("publication worker panicked");
            assert_eq!(
                completed.unwrap(),
                (api < 2).then_some(false),
                "publication API {api}, full={full}"
            );
            assert_eq!(
                instance_rx.try_iter().collect::<Vec<_>>(),
                if full { vec![vec![0x11]] } else { vec![] },
                "instance queue changed for API {api}, full={full}"
            );
            assert_eq!(
                realm_rx.try_iter().collect::<Vec<_>>(),
                if full { vec![vec![0x22]] } else { vec![] },
                "realm queue changed for API {api}, full={full}"
            );
        }
    }
}
