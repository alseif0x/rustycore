use super::{wire, *};
use aes_gcm::{
    AesGcm, KeyInit, Nonce,
    aead::{AeadInPlace, consts::U12},
    aes::Aes256,
};
use sha2::{Digest, Sha512};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, timeout},
};
use wow_crypto::forever::{ForeverWorldCrypt, SessionKeys, verify_and_derive};

const JOIN: [u8; 64] = [11; 64];
const BUILD_KEY: [u8; 16] = [22; 16];
const LOCAL: [u8; 32] = [33; 32];
const AUTH_SEED: [u8; 32] = [
    0xde, 0x3a, 0x2a, 0x8e, 0x6b, 0x89, 0x52, 0x66, 0x88, 0x9d, 0x7e, 0x7a, 0x77, 0x1d, 0x5d, 0x1f,
    0x4e, 0xd9, 0x0c, 0x23, 0x9b, 0xcd, 0x0e, 0xdc, 0xd2, 0xe8, 0x04, 0x3a, 0x68, 0x64, 0xc7, 0xb0,
];

type TestAesGcm = AesGcm<Aes256, U12, U12>;

fn encrypt_client_for_test(key: &[u8; 32], counter: u64, plaintext: &[u8]) -> (Vec<u8>, [u8; 12]) {
    let mut nonce_bytes = [0; 12];
    nonce_bytes[..8].copy_from_slice(&counter.to_le_bytes());
    nonce_bytes[8..].copy_from_slice(&0x544E_4C43_u32.to_le_bytes());
    let cipher = TestAesGcm::new_from_slice(key).unwrap();
    let mut ciphertext = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(Nonce::from_slice(&nonce_bytes), &[], &mut ciphertext)
        .unwrap();
    (ciphertext, tag.into())
}

// Independent RFC2104 construction, not the production proof helper.
fn proof(server: &[u8; 32]) -> [u8; 24] {
    let key = Sha512::digest([JOIN.as_slice(), BUILD_KEY.as_slice()].concat());
    let mut inner = [0x36; 128];
    let mut outer = [0x5c; 128];
    for (i, byte) in key.iter().enumerate() {
        inner[i] ^= byte;
        outer[i] ^= byte;
    }
    let body = [LOCAL.as_slice(), server.as_slice(), AUTH_SEED.as_slice()].concat();
    let hash = Sha512::digest([inner.as_slice(), body.as_slice()].concat());
    Sha512::digest([outer.as_slice(), hash.as_slice()].concat())[..24]
        .try_into()
        .unwrap()
}

fn auth_payload(server: &[u8; 32]) -> Vec<u8> {
    let mut body = vec![0; 8];
    for id in [2u32, 1, 1] {
        body.extend_from_slice(&id.to_le_bytes());
    }
    body.extend_from_slice(&LOCAL);
    body.extend_from_slice(&proof(server));
    body.push(0);
    body.extend_from_slice(&2u32.to_le_bytes());
    body.extend_from_slice(b"{}");
    body
}

async fn read_frame(peer: &mut TcpStream) -> (Vec<u8>, [u8; 12]) {
    let mut header = [0; 16];
    peer.read_exact(&mut header).await.unwrap();
    let size = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
    let mut data = vec![0; size];
    peer.read_exact(&mut data).await.unwrap();
    (data, header[4..].try_into().unwrap())
}

async fn write_plain(peer: &mut TcpStream, opcode: u32, body: &[u8]) {
    let data = wire::frame_data(opcode, body).unwrap();
    // Fragmentation is intentional: framing cannot assume one TCP read.
    let bytes = [
        wire::header(data.len(), [0; 12]).unwrap().as_slice(),
        data.as_slice(),
    ]
    .concat();
    for chunk in bytes.chunks(3) {
        peer.write_all(chunk).await.unwrap();
    }
}

async fn pending() -> (ForeverSocket, TcpStream, SessionKeys) {
    pending_for_build(super::BUILD).await
}

async fn pending_for_build(build: u32) -> (ForeverSocket, TcpStream, SessionKeys) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client, accepted) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    let mut peer = client.unwrap();
    // The certificate RegionGroup is independent of geographic realm Region 2.
    let mut socket = ForeverSocket::new_for_build(accepted.unwrap().0, 0x02010001, 7, build);
    let client_side = async {
        let mut hello = vec![0; wire::SERVER_HELLO.len()];
        peer.read_exact(&mut hello).await.unwrap();
        assert_eq!(hello, wire::SERVER_HELLO);
        peer.write_all(wire::CLIENT_HELLO).await.unwrap();
        let (data, tag) = read_frame(&mut peer).await;
        assert_eq!(tag, [0; 12]);
        let frame = Frame::decode(data).unwrap();
        assert_eq!(
            frame.opcode(),
            wire::server_opcode(AUTH_CHALLENGE, build >= 70291)
        );
        assert_eq!(frame.payload().len(), 65);
        let challenge: [u8; 32] = frame.payload()[32..64].try_into().unwrap();
        write_plain(&mut peer, AUTH_SESSION, &auth_payload(&challenge)).await;
        verify_and_derive(&JOIN, &BUILD_KEY, &LOCAL, &challenge, &proof(&challenge)).unwrap()
    };
    let (started, keys) = tokio::join!(socket.start(), client_side);
    started.unwrap();
    (socket, peer, keys)
}

async fn encrypted() -> (ForeverSocket, TcpStream, SessionKeys) {
    encrypted_for_build(super::BUILD).await
}

async fn encrypted_for_build(build: u32) -> (ForeverSocket, TcpStream, SessionKeys) {
    let (mut socket, mut peer, keys) = pending_for_build(build).await;
    socket.verify_credentials(&JOIN, &BUILD_KEY).unwrap();
    let client_side = async {
        let (data, tag) = read_frame(&mut peer).await;
        assert_eq!(tag, [0; 12]);
        let frame = Frame::decode(data).unwrap();
        assert_eq!(
            frame.opcode(),
            wire::server_opcode(ENTER_ENCRYPTED_MODE, build >= 70291)
        );
        assert_eq!(frame.payload().len(), 69);
        write_plain(&mut peer, ENTER_ENCRYPTED_MODE_ACK, &[]).await;
    };
    let (completed, ()) = tokio::join!(socket.complete_encryption(1), client_side);
    completed.unwrap();
    (socket, peer, keys)
}

#[test]
fn wire_bounds_and_auth_session_exact_length() {
    for size in [0, 3, 65536, usize::MAX] {
        assert!(wire::header(size, [0; 12]).is_err());
    }
    assert!(wire::header(4, [0; 12]).is_ok());
    assert!(wire::header(65535, [0; 12]).is_ok());
    assert!(wire::frame_data(1, &vec![0; 65532]).is_err());
    let body = auth_payload(&[1; 32]);
    assert!(wire::AuthSession::decode(&body).is_ok());
    for size in 0..body.len() {
        assert!(wire::AuthSession::decode(&body[..size]).is_err());
    }
    let mut extra = body.clone();
    extra.push(0);
    assert!(wire::AuthSession::decode(&extra).is_err());
    let mut padding = body.clone();
    padding[76] = 1;
    assert!(wire::AuthSession::decode(&padding).is_err());
    let mut overflow = body;
    overflow[77..81].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(wire::AuthSession::decode(&overflow).is_err());
}

#[test]
fn build_70291_opcode_shift_preserves_the_old_profile_and_translation_boundaries() {
    for opcode in [AUTH_CHALLENGE, AUTH_RESPONSE, 0x460098, 0x4A0000, 0x6BFFFF] {
        assert_eq!(wire::server_opcode(opcode, false), opcode);
    }
    assert_eq!(wire::server_opcode(AUTH_CHALLENGE, true), 0x4E0000);
    assert_eq!(wire::server_opcode(AUTH_RESPONSE, true), 0x470001);
    assert_eq!(wire::server_opcode(0x460098, true), 0x470099);
    assert_eq!(wire::server_opcode(0x4A0000, true), 0x4B0000);
    assert_eq!(wire::server_opcode(0x450007, true), 0x450007);
    for (raw, normalized) in [
        (0x460007, 0x450007),
        (0x440150, 0x44014F),
        (0x44013B, 0x44013B),
        (0x450006, 0x450006),
    ] {
        let mut frame = Frame::decode(wire::frame_data(raw, &[]).unwrap()).unwrap();
        frame.normalize_client_opcode(true);
        assert_eq!(frame.opcode(), normalized);
    }
}

#[tokio::test]
async fn native_order_requires_proof_persistence_ack_and_counter_two() {
    let (mut socket, mut peer, keys) = pending().await;
    assert_eq!(socket.join_ticket().unwrap(), "{}");
    assert!(socket.send(AUTH_RESPONSE, &[]).await.is_err());
    socket.verify_credentials(&JOIN, &BUILD_KEY).unwrap();
    assert_eq!(socket.session_key().unwrap(), keys.session_key());
    let client_side = async {
        let (data, tag) = read_frame(&mut peer).await;
        assert_eq!(tag, [0; 12]);
        let frame = Frame::decode(data).unwrap();
        assert_eq!(frame.opcode(), ENTER_ENCRYPTED_MODE);
        assert_eq!(frame.payload().len(), 69);
        assert_eq!(&frame.payload()[..4], &7_i32.to_le_bytes());
        assert_eq!(frame.payload()[68], 0x80);
        assert_eq!(
            &frame.payload()[4..68],
            &crate::world_socket::sign_enable_encryption_forever(keys.encryption_key())
        );
        write_plain(&mut peer, ENTER_ENCRYPTED_MODE_ACK, &[]).await;
    };
    let (completed, ()) = tokio::join!(socket.complete_encryption(1), client_side);
    completed.unwrap();
    assert!(socket.join_ticket().is_err());
    assert!(socket.start().await.is_err());
    // The first outbound encrypted packet is now the native Pong (counter 2),
    // followed by the complete denial response at counter 3, not a reused IV.
    let mut expected_crypt = ForeverWorldCrypt::new(keys.encryption_key(), 2, 2);
    for (opcode, body) in [
        (PONG, &[1, 2, 3, 4][..]),
        (AUTH_RESPONSE, &[3, 0, 0, 0, 0][..]),
    ] {
        socket.send(opcode, body).await.unwrap();
        let (data, tag) = read_frame(&mut peer).await;
        let expected = expected_crypt
            .encrypt(&wire::frame_data(opcode, body).unwrap())
            .unwrap();
        assert_eq!((data, tag), expected);
    }
    // Tampered client packet fails terminally, with no phase retry.
    peer.write_all(&wire::header(4, [1; 12]).unwrap())
        .await
        .unwrap();
    peer.write_all(&[0; 4]).await.unwrap();
    assert!(matches!(
        socket.receive().await,
        Err(ForeverSocketError::Cipher)
    ));
    assert!(matches!(
        socket.receive().await,
        Err(ForeverSocketError::Phase)
    ));
}

#[tokio::test]
async fn missing_or_ambiguous_persistence_never_publishes_offer() {
    for rows in [0, 2] {
        let (mut socket, mut peer, _) = pending().await;
        socket.verify_credentials(&JOIN, &BUILD_KEY).unwrap();
        assert!(matches!(
            socket.complete_encryption(rows).await,
            Err(ForeverSocketError::Persistence)
        ));
        drop(socket);
        let mut byte = [0];
        assert_eq!(peer.read(&mut byte).await.unwrap(), 0);
    }
}

#[tokio::test]
async fn explicit_unknown_build_key_path_preserves_session_derivation() {
    let (mut socket, _peer, keys) = pending().await;
    socket.verify_credentials_without_build_key(&JOIN).unwrap();
    assert_eq!(socket.session_key().unwrap(), keys.session_key());
}

#[tokio::test]
async fn invalid_proof_is_terminal_and_cannot_publish() {
    let (mut socket, _, _) = pending().await;
    assert!(matches!(
        socket.verify_credentials(&JOIN, &[0; 16]),
        Err(ForeverSocketError::Proof)
    ));
    assert!(socket.session_key().is_err());
    assert!(socket.complete_encryption(1).await.is_err());
    assert!(socket.verify_credentials(&JOIN, &BUILD_KEY).is_err());
}

#[tokio::test]
async fn nonempty_ack_fails_before_encrypted_traffic() {
    let (mut socket, mut peer, _) = pending().await;
    socket.verify_credentials(&JOIN, &BUILD_KEY).unwrap();
    let client_side = async {
        read_frame(&mut peer).await;
        write_plain(&mut peer, ENTER_ENCRYPTED_MODE_ACK, &[0]).await;
    };
    let (result, ()) = tokio::join!(socket.complete_encryption(1), client_side);
    assert!(matches!(
        result,
        Err(ForeverSocketError::UnexpectedEncryptionAck {
            opcode: ENTER_ENCRYPTED_MODE_ACK,
            payload_bytes: 1
        })
    ));
    assert!(socket.send(AUTH_RESPONSE, &[]).await.is_err());
}

#[tokio::test]
async fn idle_receive_timeout_is_terminal_without_consuming_bytes() {
    let (mut socket, _peer, _) = encrypted().await;

    assert!(matches!(
        socket
            .receive_with_idle_timeout(Duration::from_millis(5))
            .await,
        Err(ForeverSocketError::Timeout)
    ));
    assert!(matches!(
        socket.receive().await,
        Err(ForeverSocketError::Phase)
    ));
}

#[tokio::test]
async fn idle_receive_rejects_wrong_phase_without_waiting() {
    let (mut socket, _peer, _) = pending().await;

    let result = timeout(
        Duration::from_millis(25),
        socket.receive_with_idle_timeout(Duration::from_secs(60)),
    )
    .await
    .expect("wrong-phase receive must not wait for idle timeout");
    assert!(matches!(result, Err(ForeverSocketError::Phase)));
}

#[tokio::test]
async fn cancelling_idle_receive_leaves_socket_terminal() {
    let (mut socket, _peer, _) = encrypted().await;

    assert!(
        timeout(
            Duration::from_millis(5),
            socket.receive_with_idle_timeout(Duration::from_secs(60)),
        )
        .await
        .is_err()
    );
    assert!(matches!(
        socket.receive().await,
        Err(ForeverSocketError::Phase)
    ));
}

#[tokio::test]
async fn partial_frame_is_bounded_by_idle_deadline() {
    let (mut socket, mut peer, _) = encrypted().await;
    let header = wire::header(4, [0; 12]).unwrap();
    peer.write_all(&header[..1]).await.unwrap();

    assert!(matches!(
        socket
            .receive_with_idle_timeout(Duration::from_millis(5))
            .await,
        Err(ForeverSocketError::Timeout)
    ));
    assert!(matches!(
        socket.receive().await,
        Err(ForeverSocketError::Phase)
    ));
}

#[tokio::test]
async fn first_encrypted_client_frame_uses_counter_two_with_idle_policy() {
    let (mut socket, mut peer, keys) = encrypted().await;
    let plaintext = wire::frame_data(PING, &[1, 2, 3, 4]).unwrap();
    let (ciphertext, tag) = encrypt_client_for_test(keys.encryption_key(), 2, &plaintext);
    let header = wire::header(ciphertext.len(), tag).unwrap();
    peer.write_all(&header).await.unwrap();
    peer.write_all(&ciphertext).await.unwrap();

    let frame = socket
        .receive_with_idle_timeout(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(frame.opcode(), PING);
    assert_eq!(frame.payload(), &[1, 2, 3, 4]);
}

#[tokio::test]
async fn large_encrypted_packet_uses_persistent_raw_deflate_and_four_byte_checksums() {
    let (mut socket, mut peer, keys) = encrypted().await;
    let body: Vec<u8> = (0..78_592).map(|index| (index % 251) as u8).collect();
    socket.send(AUTH_RESPONSE, &body).await.unwrap();

    let (data, tag) = read_frame(&mut peer).await;
    let mut nonce_bytes = [0; 12];
    nonce_bytes[..8].copy_from_slice(&2_u64.to_le_bytes());
    nonce_bytes[8..].copy_from_slice(&0x5256_5253_u32.to_le_bytes());
    let cipher = TestAesGcm::new_from_slice(keys.encryption_key()).unwrap();
    assert!(data.len() < wire::MAX_FRAME);
    let mut plaintext = data;
    cipher
        .decrypt_in_place_detached(
            Nonce::from_slice(&nonce_bytes),
            &[],
            &mut plaintext,
            (&tag).into(),
        )
        .unwrap();
    let frame = Frame::decode(plaintext).unwrap();
    assert_eq!(frame.opcode(), wire::COMPRESSED_PACKET);

    let payload = frame.payload();
    let uncompressed_size = u32::from_le_bytes(payload[..4].try_into().unwrap()) as usize;
    let uncompressed_adler = u32::from_le_bytes(payload[4..8].try_into().unwrap());
    let compressed_adler = u32::from_le_bytes(payload[8..12].try_into().unwrap());
    assert_eq!(uncompressed_size, body.len() + 4);
    assert_eq!(
        compressed_adler,
        wow_packet::compression::adler32(&payload[12..])
    );

    let expected = [AUTH_RESPONSE.to_le_bytes().as_slice(), &body].concat();
    assert_eq!(
        uncompressed_adler,
        wow_packet::compression::adler32(&expected)
    );
    let mut output = vec![0; uncompressed_size];
    let mut decompressor = flate2::Decompress::new(false);
    decompressor
        .decompress(&payload[12..], &mut output, flate2::FlushDecompress::Sync)
        .unwrap();
    assert_eq!(decompressor.total_out() as usize, expected.len());
    assert_eq!(output, expected);
}

#[tokio::test]
async fn modern_large_hotfix_response_uses_uncompressed_original_opcode() {
    let (mut socket, mut peer, keys) = encrypted_for_build(70450).await;
    let body = vec![0xA5; 1_983_623];
    socket.send(AUTH_RESPONSE, &body).await.unwrap();

    let (data, tag) = read_frame(&mut peer).await;
    assert_eq!(data.len(), body.len() + 4);
    let mut nonce_bytes = [0; 12];
    nonce_bytes[..8].copy_from_slice(&2_u64.to_le_bytes());
    nonce_bytes[8..].copy_from_slice(&0x5256_5253_u32.to_le_bytes());
    let cipher = TestAesGcm::new_from_slice(keys.encryption_key()).unwrap();
    let mut plaintext = data;
    cipher
        .decrypt_in_place_detached(
            Nonce::from_slice(&nonce_bytes),
            &[],
            &mut plaintext,
            (&tag).into(),
        )
        .unwrap();
    assert_eq!(
        u32::from_le_bytes(plaintext[..4].try_into().unwrap()),
        wire::server_opcode(AUTH_RESPONSE, true)
    );
    assert_eq!(&plaintext[4..], body);
    assert!(wire::header_outgoing(0xFF_FFFF, [0; 12]).is_ok());
    assert!(wire::header_outgoing(0x100_0000, [0; 12]).is_err());
}
