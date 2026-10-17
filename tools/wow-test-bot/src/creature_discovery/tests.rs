use super::*;
use crate::{build_packed_guid, create_creature_guid_raw, find_creature_guid_in_update_object};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn socket_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    (client, listener.accept().await.unwrap().0)
}

async fn read_client_opcode(stream: &mut TcpStream, crypt: &mut WorldCrypt) -> u16 {
    let mut header = [0u8; 16];
    stream.read_exact(&mut header).await.unwrap();
    let size = u32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
    let tag: [u8; 12] = header[4..16].try_into().unwrap();
    let mut ciphertext = vec![0u8; size];
    stream.read_exact(&mut ciphertext).await.unwrap();
    let plaintext = crypt.decrypt_client(&ciphertext, &tag, &[]).unwrap();
    u16::from_le_bytes([plaintext[0], plaintext[1]])
}

async fn publish(stream: &mut TcpStream, crypt: &mut WorldCrypt, opcode: u16, body: &[u8]) {
    let mut plaintext = opcode.to_le_bytes().to_vec();
    plaintext.extend_from_slice(body);
    let (ciphertext, tag) = crypt.encrypt_server(&plaintext, &[]).unwrap();
    stream
        .write_all(&(ciphertext.len() as u32).to_le_bytes())
        .await
        .unwrap();
    stream.write_all(&tag).await.unwrap();
    stream.write_all(&ciphertext).await.unwrap();
}

fn innkeeper_create(low: u64, high: u64) -> Vec<u8> {
    let mut payload = vec![0, 0, 0, 0, 1, 0, 0, 0, 1];
    payload.extend(build_packed_guid(low, high));
    payload.push(5);
    payload
}

/// A C++-like server: nothing is visible until the ACK arrives, and the
/// CREATE follows after a quiet gap longer than one poll interval.
#[tokio::test]
async fn discovery_sends_active_mover_ack_and_outlasts_a_quiet_gap() {
    let (mut client, mut server) = socket_pair().await;
    let (low, high) = create_creature_guid_raw(0, 12_196, 143);
    let server_task = tokio::spawn(async move {
        let mut crypt = WorldCrypt::new(&[0; 16]);
        let opcode = read_client_opcode(&mut server, &mut crypt).await;
        tokio::time::sleep(Duration::from_millis(700)).await;
        publish(
            &mut server,
            &mut crypt,
            SMSG_UPDATE_OBJECT,
            &innkeeper_create(low, high),
        )
        .await;
        (opcode, server)
    });
    let mut result = BotRunResult::default();
    let found = discover_creature_after_active_mover_ack(
        &mut client,
        &mut WorldCrypt::new(&[0; 16]),
        &mut ServerPacketInflater::default(),
        Duration::from_secs(5),
        "test discovery",
        &mut result,
        |payload| find_creature_guid_in_update_object(payload, 0, 12_196),
    )
    .await
    .unwrap();
    let (opcode, _server) = server_task.await.unwrap();
    assert_eq!(opcode, CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE);
    assert_eq!(found, Some((low, high)));
    assert_eq!(result.seen_opcodes, ["0x27CB"]);
}

#[tokio::test]
async fn discovery_ignores_other_creatures_and_reports_absence_at_the_window() {
    let (mut client, mut server) = socket_pair().await;
    let (low, high) = create_creature_guid_raw(0, 99_999, 7);
    let server_task = tokio::spawn(async move {
        let mut crypt = WorldCrypt::new(&[0; 16]);
        read_client_opcode(&mut server, &mut crypt).await;
        publish(
            &mut server,
            &mut crypt,
            SMSG_UPDATE_OBJECT,
            &innkeeper_create(low, high),
        )
        .await;
        server
    });
    let mut result = BotRunResult::default();
    let started = tokio::time::Instant::now();
    let found = discover_creature_after_active_mover_ack(
        &mut client,
        &mut WorldCrypt::new(&[0; 16]),
        &mut ServerPacketInflater::default(),
        Duration::from_millis(800),
        "test discovery",
        &mut result,
        |payload| find_creature_guid_in_update_object(payload, 0, 12_196),
    )
    .await
    .unwrap();
    let _server = server_task.await.unwrap();
    assert_eq!(found, None);
    assert!(started.elapsed() >= Duration::from_millis(800));
    assert_eq!(result.seen_opcodes, ["0x27CB"]);
}
