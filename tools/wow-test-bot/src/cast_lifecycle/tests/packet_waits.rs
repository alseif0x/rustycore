use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

fn packet_plan(action: Action, expected: ExpectedRule, timeout_ms: u64) -> Plan {
    Plan {
        character_guid: None,
        observe_only: false,
        timeout_ms,
        actions: vec![action],
        expect: vec![expected],
        finish_policy: FinishPolicy::Disconnect,
    }
}

fn packet_controller(plan: Plan) -> Controller {
    Controller::new(
        Options {
            source_path: "packet-waits-test".to_string(),
            plan,
        },
        17,
        realm_id(),
    )
}

fn raw_packet_expectation(connection: Connection, opcode: u16, body: &[u8]) -> ExpectedRule {
    ExpectedRule::Packet {
        connection,
        opcode,
        body_hex: hex::encode(body),
    }
}

async fn tcp_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).await.unwrap();
    let (server, _) = listener.accept().await.unwrap();
    (client, server)
}

fn encrypted_server_packet(crypt: &mut WorldCrypt, opcode: u16, body: &[u8]) -> Vec<u8> {
    let mut plaintext = opcode.to_le_bytes().to_vec();
    plaintext.extend_from_slice(body);
    let (ciphertext, tag) = crypt.encrypt_server(&plaintext, &[]).unwrap();
    let mut packet = Vec::with_capacity(16 + ciphertext.len());
    packet.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
    packet.extend_from_slice(&tag);
    packet.extend_from_slice(&ciphertext);
    packet
}

fn logout_controller() -> Controller {
    let mut plan = packet_plan(
        Action::Wait { milliseconds: 50 },
        raw_packet_expectation(Connection::Realm, SMSG_LOGOUT_COMPLETE, &[]),
        100,
    );
    plan.finish_policy = FinishPolicy::Logout;
    packet_controller(plan)
}

#[tokio::test]
async fn finish_logout_waits_for_delayed_realm_ack_after_instance_eof() {
    let mut controller = logout_controller();
    let (mut stream, mut instance_peer) = tcp_pair().await;
    let (realm_stream, mut realm_peer) = tcp_pair().await;
    let key = [0x6a; 16];
    let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut inflater = ServerPacketInflater::default();
    let mut realm = Some(EncryptedWorldConnection {
        stream: realm_stream,
        crypt: WorldCrypt::new_with_counters(&key, 0, 0),
        inflater: ServerPacketInflater::default(),
    });
    let mut finishing = Box::pin(finish(
        &mut stream,
        &mut crypt,
        &mut inflater,
        &mut realm,
        &mut controller,
    ));
    // Drive the real finish until its encrypted request is on the wire.
    let mut request = [0u8; 19];
    tokio::select! {
        _ = &mut finishing => panic!("finish returned before its logout request"),
        result = instance_peer.read_exact(&mut request) => { result.unwrap(); }
    }
    assert_eq!(u32::from_le_bytes(request[..4].try_into().unwrap()), 3);
    let mut instance_server = WorldCrypt::new_with_counters(&key, 0, 0);
    instance_peer
        .write_all(&encrypted_server_packet(
            &mut instance_server,
            0x2683,
            &[0; 5],
        ))
        .await
        .unwrap();
    instance_peer.shutdown().await.unwrap();
    // EOF is already available, but the collector must remain pending until
    // the independent realm socket delivers its actual acknowledgement.
    assert!(
        tokio::time::timeout(Duration::from_millis(40), &mut finishing)
            .await
            .is_err()
    );
    let mut realm_server = WorldCrypt::new_with_counters(&key, 0, 0);
    realm_peer
        .write_all(&encrypted_server_packet(
            &mut realm_server,
            SMSG_LOGOUT_COMPLETE,
            &[],
        ))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), &mut finishing)
        .await
        .expect("realm acknowledgement completes logout");
    drop(finishing);
    assert!(
        controller.evidence.failure.is_none(),
        "{:?}",
        controller.evidence.failure
    );
    assert!(controller.evidence.logout_request_completed);
    assert!(controller.evidence.logout_confirmed);
    assert_eq!(controller.evidence.logout_route.as_deref(), Some("realm"));
    assert!(controller.evidence.finish_completed);
    assert_eq!(
        controller.evidence.sent_packets[0].opcode,
        CMSG_LOGOUT_REQUEST
    );
    assert!(controller.evidence.sent_packets[0].write_completed);
    assert_eq!(
        controller
            .evidence
            .events
            .iter()
            .map(|event| (event.connection, event.opcode))
            .collect::<Vec<_>>(),
        [
            (Connection::Instance, 0x2683),
            (Connection::Realm, SMSG_LOGOUT_COMPLETE)
        ]
    );
    let ack = controller
        .evidence
        .trace
        .iter()
        .position(|entry| entry.opcode == Some(SMSG_LOGOUT_COMPLETE))
        .unwrap();
    let done = controller
        .evidence
        .trace
        .iter()
        .position(|entry| {
            entry.action.as_deref() == Some("finish_logout")
                && entry.kind == TraceKind::ActionCompleted
        })
        .unwrap();
    assert!(ack < done);
}

#[tokio::test]
async fn typed_logout_request_marks_completion_only_after_successful_write() {
    for write_succeeds in [true, false] {
        let mut controller = packet_controller(packet_plan(
            Action::LogoutRequest { idle_logout: true },
            raw_packet_expectation(Connection::Instance, SMSG_LOGOUT_RESPONSE, &[0; 5]),
            20,
        ));
        let (mut stream, _peer) = tcp_pair().await;
        if !write_succeeds {
            stream.shutdown().await.unwrap();
        }
        let key = [0x6c; 16];
        let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
        let mut inflater = ServerPacketInflater::default();
        let mut realm = None;
        execute(
            &mut controller,
            &mut stream,
            &mut crypt,
            &mut inflater,
            &mut realm,
        )
        .await
        .unwrap();

        assert_eq!(
            controller.evidence.logout_request_completed, write_succeeds,
            "write_succeeds={write_succeeds}"
        );
        assert_eq!(controller.evidence.sent_packets.len(), 1);
        assert_eq!(
            controller.evidence.sent_packets[0].write_completed,
            write_succeeds
        );
        let boundary = if write_succeeds {
            TraceKind::ActionCompleted
        } else {
            TraceKind::ActionFailed
        };
        assert!(controller.evidence.trace.iter().any(|entry| {
            entry.action.as_deref() == Some("logout_request") && entry.kind == boundary
        }));
    }
}

#[tokio::test]
async fn logout_eof_never_substitutes_for_ack_and_frame_errors_remain_fatal() {
    for case in [
        "missing_ack",
        "realm_eof",
        "invalid_frame",
        "truncated_frame",
    ] {
        let mut controller = logout_controller();
        controller.evidence.logout_request_completed = true;
        let (mut stream, mut instance_peer) = tcp_pair().await;
        let (realm_stream, mut realm_peer) = tcp_pair().await;
        let key = [0x6b; 16];
        let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
        let mut inflater = ServerPacketInflater::default();
        let mut realm = Some(EncryptedWorldConnection {
            stream: realm_stream,
            crypt: WorldCrypt::new_with_counters(&key, 0, 0),
            inflater: ServerPacketInflater::default(),
        });
        match case {
            "invalid_frame" => instance_peer.write_all(&[0; 16]).await.unwrap(),
            "truncated_frame" => instance_peer.write_all(&[3, 0, 0, 0]).await.unwrap(),
            _ => {}
        }
        instance_peer.shutdown().await.unwrap();
        if case == "realm_eof" {
            realm_peer.shutdown().await.unwrap();
        }
        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
        let error = collect_logout_until(
            deadline,
            &mut stream,
            &mut crypt,
            &mut inflater,
            &mut realm,
            &mut controller,
        )
        .await
        .expect_err(case)
        .to_string();
        match case {
            "missing_ack" => assert!(error.contains("timed out waiting for realm LogoutComplete")),
            "realm_eof" => {
                assert!(error.contains("cast lifecycle realm packet: connection closed"))
            }
            _ => assert!(
                !error.contains("timed out waiting for realm LogoutComplete"),
                "{case}: {error}"
            ),
        }
        assert!(!controller.evidence.logout_confirmed, "{case}");
        assert!(!controller.evidence.finish_completed, "{case}");
    }
}

#[tokio::test]
async fn instance_eof_is_fatal_during_actions_even_with_a_previous_logout_marker() {
    for previously_requested in [false, true] {
        let mut controller = logout_controller();
        controller.evidence.logout_request_completed = previously_requested;
        let (mut stream, mut instance_peer) = tcp_pair().await;
        let (realm_stream, _realm_peer) = tcp_pair().await;
        instance_peer.shutdown().await.unwrap();
        let key = [0x6c; 16];
        let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
        let mut inflater = ServerPacketInflater::default();
        let mut realm = Some(EncryptedWorldConnection {
            stream: realm_stream,
            crypt: WorldCrypt::new_with_counters(&key, 0, 0),
            inflater: ServerPacketInflater::default(),
        });
        execute(
            &mut controller,
            &mut stream,
            &mut crypt,
            &mut inflater,
            &mut realm,
        )
        .await
        .unwrap();
        assert!(
            controller
                .evidence
                .failure
                .as_deref()
                .unwrap()
                .contains("cast lifecycle instance packet: connection closed")
        );
        assert!(
            controller
                .evidence
                .trace
                .iter()
                .any(|entry| entry.action.as_deref() == Some("wait")
                    && entry.kind == TraceKind::ActionFailed)
        );
        assert!(!controller.evidence.logout_confirmed);
    }
}

#[test]
fn packet_wait_validation_includes_supported_movement_controls_only_for_waits() {
    let mut plan = packet_plan(
        Action::WaitForPacket {
            connection: Connection::Instance,
            opcode: SMSG_MOVE_ROOT,
            timeout_ms: 1,
        },
        raw_packet_expectation(Connection::Realm, SMSG_LOGOUT_COMPLETE, &[]),
        100,
    );
    plan::validate_plan(&plan).unwrap();
    assert!(plan::supported_wait_packet_opcode(SMSG_MOVE_ROOT));
    assert!(!plan::supported_exact_packet_opcode(SMSG_MOVE_ROOT));

    plan.actions[0] = Action::WaitForPacket {
        connection: Connection::Instance,
        opcode: REDACTED_SMSG_AUTH_CHALLENGE,
        timeout_ms: 1,
    };
    assert!(plan::validate_plan(&plan).is_err());

    for timeout_ms in [0, MAX_WAIT_MS + 1] {
        plan.actions[0] = Action::WaitForPacket {
            connection: Connection::Instance,
            opcode: SMSG_MOVE_ROOT,
            timeout_ms,
        };
        assert!(plan::validate_plan(&plan).is_err());
    }
    plan.actions[0] = Action::WaitForPacket {
        connection: Connection::Instance,
        opcode: SMSG_MOVE_ROOT,
        timeout_ms: MAX_WAIT_MS,
    };
    plan::validate_plan(&plan).unwrap();
}

#[tokio::test]
async fn packet_wait_requires_a_new_root_and_keeps_expectations_independent() {
    let target_opcode = SMSG_MOVE_ROOT;
    let expected_opcode = RAW_SMSG_PRE_RESSURECT;
    let plan_timeout_ms = 100;
    let mut controller = packet_controller(packet_plan(
        Action::WaitForPacket {
            connection: Connection::Instance,
            opcode: target_opcode,
            timeout_ms: 60,
        },
        raw_packet_expectation(Connection::Instance, expected_opcode, &[0xA1]),
        plan_timeout_ms,
    ));
    controller.evidence.events.push(decode_event(
        0,
        Connection::Instance,
        target_opcode,
        &[0xEE],
    ));
    controller.evidence.events.push(decode_event(
        1,
        Connection::Instance,
        expected_opcode,
        &[0xA1],
    ));

    let mut login_position = 530i32.to_le_bytes().to_vec();
    for coordinate in [1.25f32, -2.5, 8.0, 0.75] {
        login_position.extend(coordinate.to_le_bytes());
    }
    login_position.extend(0u32.to_le_bytes());
    controller
        .process_packet(
            Connection::Instance,
            SMSG_LOGIN_VERIFY_WORLD,
            &login_position,
        )
        .unwrap();

    let (mut stream, mut peer) = tcp_pair().await;
    let key = [0x57; 16];
    let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let (low, high) = create_player_guid_raw(17, realm_id());
    let mover = Guid { low, high };
    let mut root_body = mover.packed();
    root_body.extend(23u32.to_le_bytes());
    let received_root_hex = hex::encode(&root_body);
    let (release_peer, keep_peer_open) = oneshot::channel();
    let send_root = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        peer.write_all(&encrypted_server_packet(
            &mut server_crypt,
            target_opcode,
            &root_body,
        ))
        .await?;
        let _ = keep_peer_open.await;
        Ok::<(), std::io::Error>(())
    });

    let mut inflater = ServerPacketInflater::default();
    let mut realm = None;
    execute(
        &mut controller,
        &mut stream,
        &mut crypt,
        &mut inflater,
        &mut realm,
    )
    .await
    .unwrap();
    let _ = release_peer.send(());
    send_root.await.unwrap().unwrap();

    assert!(
        controller.evidence.passed,
        "{:?}",
        controller.evidence.failure
    );
    let root_events = controller
        .evidence
        .events
        .iter()
        .filter(|event| event.opcode == target_opcode)
        .collect::<Vec<_>>();
    assert_eq!(root_events.len(), 2);
    assert_eq!(root_events[0].body_hex, "ee");
    assert_eq!(root_events[1].body_hex, received_root_hex);
    assert!(controller.evidence.expected_rules[0].matched);
    assert!(controller.evidence.sent_packets.iter().any(|packet| {
        packet.action == "ack_move_root" && packet.opcode == 0x3A0E && packet.write_completed
    }));
    let action_boundaries = controller
        .evidence
        .trace
        .iter()
        .filter(|entry| entry.action.as_deref() == Some("wait_for_packet"))
        .map(|entry| entry.kind.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        action_boundaries,
        [TraceKind::ActionStarted, TraceKind::ActionCompleted]
    );
}

#[tokio::test]
async fn packet_wait_times_out_despite_historical_match_and_fails_on_early_logout() {
    let target_opcode = RAW_SMSG_PRE_RESSURECT;
    let expected_body = [0xA1];
    let mut timed_out = packet_controller(packet_plan(
        Action::WaitForPacket {
            connection: Connection::Instance,
            opcode: target_opcode,
            timeout_ms: 20,
        },
        raw_packet_expectation(Connection::Instance, target_opcode, &expected_body),
        100,
    ));
    timed_out.evidence.events.push(decode_event(
        0,
        Connection::Instance,
        target_opcode,
        &expected_body,
    ));
    let (mut stream, _peer) = tcp_pair().await;
    let key = [0x57; 16];
    let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut inflater = ServerPacketInflater::default();
    let mut realm = None;
    execute(
        &mut timed_out,
        &mut stream,
        &mut crypt,
        &mut inflater,
        &mut realm,
    )
    .await
    .unwrap();
    assert!(timed_out.evidence.expected_rules[0].matched);
    assert!(
        timed_out
            .evidence
            .failure
            .as_deref()
            .unwrap()
            .contains("timed out waiting for Instance packet 0x276B")
    );
    assert!(timed_out.evidence.trace.iter().any(|entry| {
        entry.action.as_deref() == Some("wait_for_packet") && entry.kind == TraceKind::ActionFailed
    }));

    let mut logged_out = packet_controller(packet_plan(
        Action::WaitForPacket {
            connection: Connection::Instance,
            opcode: target_opcode,
            timeout_ms: 500,
        },
        raw_packet_expectation(Connection::Instance, target_opcode, &expected_body),
        600,
    ));
    logged_out.evidence.logout_request_completed = true;
    let (mut instance_stream, _instance_peer) = tcp_pair().await;
    let (realm_stream, mut realm_peer) = tcp_pair().await;
    let mut instance_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let realm_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut realm_server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut realm = Some(EncryptedWorldConnection {
        stream: realm_stream,
        crypt: realm_crypt,
        inflater: ServerPacketInflater::default(),
    });
    let send_logout = tokio::spawn(async move {
        realm_peer
            .write_all(&encrypted_server_packet(
                &mut realm_server_crypt,
                SMSG_LOGOUT_COMPLETE,
                &[],
            ))
            .await
    });
    let mut instance_inflater = ServerPacketInflater::default();
    execute(
        &mut logged_out,
        &mut instance_stream,
        &mut instance_crypt,
        &mut instance_inflater,
        &mut realm,
    )
    .await
    .unwrap();
    send_logout.await.unwrap().unwrap();
    assert!(
        logged_out
            .evidence
            .failure
            .as_deref()
            .unwrap()
            .contains("LogoutComplete arrived before wait_for_packet")
    );
    assert!(logged_out.evidence.trace.iter().any(|entry| {
        entry.action.as_deref() == Some("wait_for_packet") && entry.kind == TraceKind::ActionFailed
    }));
}
