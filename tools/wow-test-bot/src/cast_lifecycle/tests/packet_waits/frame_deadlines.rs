use super::*;
use crate::bot::{CMSG_TIME_SYNC_RESPONSE, SMSG_TIME_SYNC_REQUEST};

fn spawn_split_encrypted_server_packet(
    mut peer: TcpStream,
    packet: Vec<u8>,
    split_at: usize,
    delay: Duration,
) -> (
    oneshot::Receiver<()>,
    tokio::task::JoinHandle<std::io::Result<TcpStream>>,
) {
    let (started_tx, started_rx) = oneshot::channel();
    let sender = tokio::spawn(async move {
        peer.write_all(&packet[..split_at]).await?;
        let _ = started_tx.send(());
        tokio::time::sleep(delay).await;
        peer.write_all(&packet[split_at..]).await?;
        Ok(peer)
    });
    (started_rx, sender)
}

#[tokio::test]
async fn ordinary_wait_finishes_an_admitted_split_ambient_frame_before_the_plan_deadline() {
    const SEQUENCE: [u8; 4] = [0x34, 0x12, 0x00, 0x00];

    let mut controller = packet_controller(packet_plan(
        Action::Wait { milliseconds: 60 },
        raw_packet_expectation(Connection::Instance, SMSG_TIME_SYNC_REQUEST, &SEQUENCE),
        800,
    ));
    let (mut stream, peer) = tcp_pair().await;
    let key = [0x74; 16];
    let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let packet = encrypted_server_packet(&mut server_crypt, SMSG_TIME_SYNC_REQUEST, &SEQUENCE);
    let (started, sender) =
        spawn_split_encrypted_server_packet(peer, packet, 3, Duration::from_millis(120));
    started.await.unwrap();

    let mut inflater = ServerPacketInflater::default();
    let mut realm = None;
    tokio::time::timeout(
        Duration::from_secs(2),
        execute(
            &mut controller,
            &mut stream,
            &mut crypt,
            &mut inflater,
            &mut realm,
        ),
    )
    .await
    .expect("the wait and plan deadlines bound execution")
    .unwrap();
    let _peer = sender.await.unwrap().unwrap();

    assert!(
        controller.evidence.passed,
        "{:?}",
        controller.evidence.failure
    );
    assert_eq!(controller.evidence.events.len(), 1);
    assert_eq!(controller.evidence.events[0].opcode, SMSG_TIME_SYNC_REQUEST);
    assert_eq!(
        controller.evidence.events[0].body_hex,
        hex::encode(SEQUENCE)
    );
    assert!(controller.evidence.sent_packets.iter().any(|packet| {
        packet.connection == Connection::Instance
            && packet.opcode == CMSG_TIME_SYNC_RESPONSE
            && packet.write_completed
    }));

    let packet_observed = controller
        .evidence
        .trace
        .iter()
        .position(|entry| {
            entry.kind == TraceKind::PacketObserved
                && entry.connection == Some(Connection::Instance)
                && entry.opcode == Some(SMSG_TIME_SYNC_REQUEST)
        })
        .unwrap();
    let wait_completed = controller
        .evidence
        .trace
        .iter()
        .position(|entry| {
            entry.action.as_deref() == Some("wait") && entry.kind == TraceKind::ActionCompleted
        })
        .unwrap();
    assert!(packet_observed < wait_completed);
}

#[tokio::test]
async fn explicit_packet_wait_keeps_its_deadline_for_a_split_frame() {
    const SEQUENCE: [u8; 4] = [0x78, 0x56, 0x00, 0x00];

    let mut controller = packet_controller(packet_plan(
        Action::WaitForPacket {
            connection: Connection::Instance,
            opcode: SMSG_TIME_SYNC_REQUEST,
            timeout_ms: 60,
        },
        raw_packet_expectation(Connection::Instance, SMSG_TIME_SYNC_REQUEST, &SEQUENCE),
        800,
    ));
    let (mut stream, peer) = tcp_pair().await;
    let key = [0x75; 16];
    let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let packet = encrypted_server_packet(&mut server_crypt, SMSG_TIME_SYNC_REQUEST, &SEQUENCE);
    let (started, sender) =
        spawn_split_encrypted_server_packet(peer, packet, 3, Duration::from_millis(120));
    started.await.unwrap();

    let mut inflater = ServerPacketInflater::default();
    let mut realm = None;
    tokio::time::timeout(
        Duration::from_secs(1),
        execute(
            &mut controller,
            &mut stream,
            &mut crypt,
            &mut inflater,
            &mut realm,
        ),
    )
    .await
    .expect("an explicit packet wait remains bounded")
    .unwrap();
    let _peer = sender.await.unwrap().unwrap();

    let failure = controller.evidence.failure.as_deref().unwrap();
    assert!(failure.starts_with("action failed:"), "{failure}");
    assert!(
        failure.contains("encrypted frame read timed out"),
        "{failure}"
    );
    assert!(controller.evidence.events.is_empty());
    assert!(!controller.evidence.expected_rules[0].matched);
}

#[tokio::test]
async fn partial_ordinary_wait_frame_is_fatal_at_the_hard_plan_deadline() {
    let mut controller = packet_controller(packet_plan(
        Action::Wait { milliseconds: 60 },
        raw_packet_expectation(Connection::Instance, SMSG_LOGOUT_RESPONSE, &[0; 5]),
        300,
    ));
    let (mut stream, mut peer) = tcp_pair().await;
    let key = [0x76; 16];
    let mut server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let packet = encrypted_server_packet(&mut server_crypt, SMSG_LOGOUT_RESPONSE, &[0; 5]);
    peer.write_all(&packet[..3]).await.unwrap();

    let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);
    let mut inflater = ServerPacketInflater::default();
    let mut realm = None;
    tokio::time::timeout(
        Duration::from_secs(1),
        execute(
            &mut controller,
            &mut stream,
            &mut crypt,
            &mut inflater,
            &mut realm,
        ),
    )
    .await
    .expect("partial frame reading is capped by the plan deadline")
    .unwrap();

    let failure = controller.evidence.failure.as_deref().unwrap();
    assert!(failure.starts_with("action failed:"), "{failure}");
    assert!(
        failure.contains("encrypted frame read timed out"),
        "{failure}"
    );
    assert!(controller.evidence.events.is_empty());
    assert!(!controller.evidence.expected_rules[0].matched);
}
