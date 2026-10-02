use super::{wire, *};
use sha2::{Digest, Sha512};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use wow_crypto::forever::{ForeverWorldCrypt, SessionKeys, verify_and_derive};

const JOIN: [u8; 64] = [11; 64];
const BUILD_KEY: [u8; 16] = [22; 16];
const LOCAL: [u8; 32] = [33; 32];
const AUTH_SEED: [u8; 32] = [
    0xde, 0x3a, 0x2a, 0x8e, 0x6b, 0x89, 0x52, 0x66, 0x88, 0x9d, 0x7e, 0x7a, 0x77, 0x1d, 0x5d, 0x1f,
    0x4e, 0xd9, 0x0c, 0x23, 0x9b, 0xcd, 0x0e, 0xdc, 0xd2, 0xe8, 0x04, 0x3a, 0x68, 0x64, 0xc7, 0xb0,
];

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
    let mut data = vec![0; wire::frame_size(&header).unwrap()];
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
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client, accepted) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    let mut peer = client.unwrap();
    // The certificate RegionGroup is independent of geographic realm Region 2.
    let mut socket = ForeverSocket::new(accepted.unwrap().0, 0x02010001, 7);
    let client_side = async {
        let mut hello = vec![0; wire::SERVER_HELLO.len()];
        peer.read_exact(&mut hello).await.unwrap();
        assert_eq!(hello, wire::SERVER_HELLO);
        peer.write_all(wire::CLIENT_HELLO).await.unwrap();
        let (data, tag) = read_frame(&mut peer).await;
        assert_eq!(tag, [0; 12]);
        let frame = Frame::decode(data).unwrap();
        assert_eq!(frame.opcode(), AUTH_CHALLENGE);
        assert_eq!(frame.payload().len(), 65);
        let challenge: [u8; 32] = frame.payload()[32..64].try_into().unwrap();
        write_plain(&mut peer, AUTH_SESSION, &auth_payload(&challenge)).await;
        verify_and_derive(&JOIN, &BUILD_KEY, &LOCAL, &challenge, &proof(&challenge)).unwrap()
    };
    let (started, keys) = tokio::join!(socket.start(), client_side);
    started.unwrap();
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
