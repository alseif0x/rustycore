use super::*;
use tokio::io::AsyncWriteExt;
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
        .process_packet(Connection::Instance, SMSG_LOGIN_VERIFY_WORLD, &login_position)
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

    assert!(controller.evidence.passed, "{:?}", controller.evidence.failure);
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
    assert!(timed_out
        .evidence
        .failure
        .as_deref()
        .unwrap()
        .contains("timed out waiting for Instance packet 0x276B"));
    assert!(timed_out.evidence.trace.iter().any(|entry| {
        entry.action.as_deref() == Some("wait_for_packet")
            && entry.kind == TraceKind::ActionFailed
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
    assert!(logged_out
        .evidence
        .failure
        .as_deref()
        .unwrap()
        .contains("LogoutComplete arrived before wait_for_packet"));
    assert!(logged_out.evidence.trace.iter().any(|entry| {
        entry.action.as_deref() == Some("wait_for_packet")
            && entry.kind == TraceKind::ActionFailed
    }));
}
