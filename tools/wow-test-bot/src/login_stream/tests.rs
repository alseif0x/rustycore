use super::*;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn login_drain_requires_object_publication_and_live_sockets() {
    for (publish, close_realm) in [(true, false), (true, true), (false, false)] {
        let (mut instance, mut server) = socket_pair().await;
        let (realm, realm_server) = socket_pair().await;
        let mut connection = Some(EncryptedWorldConnection {
            stream: realm,
            crypt: WorldCrypt::new(&[0; 16]),
            inflater: ServerPacketInflater::default(),
        });
        let mut server_crypt = WorldCrypt::new(&[0; 16]);
        let (ciphertext, tag) = server_crypt
            .encrypt_server(&SMSG_UPDATE_OBJECT.to_le_bytes(), &[])
            .unwrap();
        if publish {
            server
                .write_all(&(ciphertext.len() as u32).to_le_bytes())
                .await
                .unwrap();
            server.write_all(&tag).await.unwrap();
            server.write_all(&ciphertext).await.unwrap();
        }
        let realm_server = if close_realm {
            drop(realm_server);
            None
        } else {
            Some(realm_server)
        };
        let mut result = BotRunResult::default();
        let drained = tokio::time::timeout(
            Duration::from_secs(2),
            drain_login_streams(
                &mut instance,
                &mut WorldCrypt::new(&[0; 16]),
                &mut ServerPacketInflater::default(),
                &mut connection,
                &mut result,
            ),
        )
        .await;
        assert_eq!(matches!(drained, Ok(Ok(()))), publish && !close_realm);
        assert_eq!(result.login_stream_drained, publish && !close_realm);
        if result.login_stream_drained {
            assert!(result.login_instance_object_update_seen);
        }
        drop(realm_server);
    }
}

async fn socket_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    (client, listener.accept().await.unwrap().0)
}

fn encrypted_connection(stream: TcpStream) -> EncryptedWorldConnection {
    EncryptedWorldConnection {
        stream,
        crypt: WorldCrypt::new(&[0; 16]),
        inflater: ServerPacketInflater::default(),
    }
}

async fn publish(stream: &mut TcpStream, crypt: &mut WorldCrypt, opcode: u16) {
    let (ciphertext, tag) = crypt.encrypt_server(&opcode.to_le_bytes(), &[]).unwrap();
    stream
        .write_all(&(ciphertext.len() as u32).to_le_bytes())
        .await
        .unwrap();
    stream.write_all(&tag).await.unwrap();
    stream.write_all(&ciphertext).await.unwrap();
}

#[test]
fn login_publication_evidence_requires_instance_and_survives_other_packets() {
    let mut result = BotRunResult::default();
    assert!(!result.login_instance_object_update_seen);
    observe_login_packet(false, SMSG_UPDATE_OBJECT, &mut result);
    observe_login_packet(true, SMSG_TIME_SYNC_REQUEST, &mut result);
    assert!(!result.login_instance_object_update_seen);
    observe_login_packet(true, SMSG_UPDATE_OBJECT, &mut result);
    observe_login_packet(false, SMSG_TIME_SYNC_REQUEST, &mut result);
    assert!(result.login_instance_object_update_seen);
    assert_eq!(
        serde_json::to_value(&result).unwrap()["login_instance_object_update_seen"],
        true
    );
}

#[tokio::test]
async fn login_drain_reuses_initial_instance_publication_without_another_packet() {
    let (mut instance, _instance_server) = socket_pair().await;
    let (realm, _realm_server) = socket_pair().await;
    let mut connection = Some(encrypted_connection(realm));
    let mut result = BotRunResult::default();
    // The same observer is called by the initial login loop before login_ready can exit.
    observe_login_packet(true, SMSG_UPDATE_OBJECT, &mut result);
    let started = tokio::time::Instant::now();
    tokio::time::timeout(
        Duration::from_secs(2),
        drain_login_streams(
            &mut instance,
            &mut WorldCrypt::new(&[0; 16]),
            &mut ServerPacketInflater::default(),
            &mut connection,
            &mut result,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(started.elapsed() >= Duration::from_secs(1));
    assert!(result.login_instance_object_update_seen);
    assert!(result.login_stream_drained);
    assert!(
        result.seen_opcodes.is_empty(),
        "a second publication was not received"
    );
}

#[tokio::test]
async fn login_drain_does_not_credit_realm_object_publication() {
    let (mut instance, _instance_server) = socket_pair().await;
    let (realm, mut realm_server) = socket_pair().await;
    let mut connection = Some(encrypted_connection(realm));
    publish(
        &mut realm_server,
        &mut WorldCrypt::new(&[0; 16]),
        SMSG_UPDATE_OBJECT,
    )
    .await;
    let mut result = BotRunResult::default();
    let drained = tokio::time::timeout(
        Duration::from_millis(1500),
        drain_login_streams(
            &mut instance,
            &mut WorldCrypt::new(&[0; 16]),
            &mut ServerPacketInflater::default(),
            &mut connection,
            &mut result,
        ),
    )
    .await;
    assert!(
        drained.is_err(),
        "realm publication must not permit a quiet drain"
    );
    assert_eq!(
        result.seen_opcodes,
        vec![format!("0x{SMSG_UPDATE_OBJECT:04X}")]
    );
    assert!(!result.login_instance_object_update_seen);
    assert!(!result.login_stream_drained);
}

#[tokio::test]
async fn login_drain_rejects_closed_instance_even_after_prior_publication() {
    let (mut instance, instance_server) = socket_pair().await;
    let (realm, _realm_server) = socket_pair().await;
    let mut connection = Some(encrypted_connection(realm));
    let mut result = BotRunResult::default();
    observe_login_packet(true, SMSG_UPDATE_OBJECT, &mut result);
    drop(instance_server);
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        drain_login_streams(
            &mut instance,
            &mut WorldCrypt::new(&[0; 16]),
            &mut ServerPacketInflater::default(),
            &mut connection,
            &mut result,
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains("instance closed"));
    assert!(!result.login_stream_drained);
}

#[tokio::test]
async fn login_drain_restarts_quiet_period_for_traffic_on_either_socket() {
    let (mut instance, mut instance_server) = socket_pair().await;
    let (realm, mut realm_server) = socket_pair().await;
    let mut connection = Some(encrypted_connection(realm));
    let mut result = BotRunResult::default();
    observe_login_packet(true, SMSG_UPDATE_OBJECT, &mut result);
    let mut crypt = WorldCrypt::new(&[0; 16]);
    let mut inflater = ServerPacketInflater::default();
    let mut instance_server_crypt = WorldCrypt::new(&[0; 16]);
    let mut realm_server_crypt = WorldCrypt::new(&[0; 16]);
    let traffic = async {
        let mut last_sent = tokio::time::Instant::now();
        for index in 0..3 {
            tokio::time::sleep(Duration::from_millis(400)).await;
            last_sent = tokio::time::Instant::now();
            if index == 1 {
                publish(&mut realm_server, &mut realm_server_crypt, 0x304E).await;
            } else {
                publish(&mut instance_server, &mut instance_server_crypt, 0x304E).await;
            }
        }
        last_sent
    };
    let drain = tokio::time::timeout(
        Duration::from_secs(4),
        drain_login_streams(
            &mut instance,
            &mut crypt,
            &mut inflater,
            &mut connection,
            &mut result,
        ),
    );
    let (drained, last_sent) = tokio::join!(drain, traffic);
    drained.unwrap().unwrap();
    assert!(last_sent.elapsed() >= Duration::from_secs(1));
    assert_eq!(result.seen_opcodes, vec!["0x304E"; 3]);
    assert!(result.login_stream_drained);
}
