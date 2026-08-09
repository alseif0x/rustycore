// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! TCP listener and accept loop for the world server.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{debug, error, info};

use wow_constants::ClientOpcodes;
use wow_crypto::HmacSha256;
use wow_packet::ClientPacket;
use wow_packet::packets::auth::{AuthContinuedSession, ConnectToKey, EnterEncryptedMode};

use crate::session_mgr::{InstanceLink, SessionManager};
use crate::world_socket::{
    AccountInfo, AccountLookup, SocketWriteFenceLikeCpp, WorldSocket, WorldSocketError,
    sign_enable_encryption,
};

/// C++ `SocketTimeOutTime{,Active}` represented in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketTimeoutsLikeCpp {
    pub unauthenticated_secs: u64,
    pub active_secs: u64,
}

impl Default for SocketTimeoutsLikeCpp {
    fn default() -> Self {
        Self {
            unauthenticated_secs: 900,
            active_secs: 60,
        }
    }
}

/// Socket/auth policy owned by the world listener.
///
/// Gameplay stores, database pools, registries, and gameplay policy snapshots
/// belong to the application callback and never cross this boundary.
#[derive(Debug, Clone)]
pub struct WorldListenerPolicyLikeCpp {
    pub max_overspeed_pings: u32,
    pub socket_timeouts: SocketTimeoutsLikeCpp,
    pub ip_location_store: Option<Arc<wow_core::IpLocationStore>>,
}

fn handoff_authenticated_world_session_like_cpp<F, Fut>(
    callback: &F,
    account_info: AccountInfo,
    packet_rx: flume::Receiver<wow_packet::WorldPacket>,
    send_tx: flume::Sender<Vec<u8>>,
    send_write_fence_like_cpp: SocketWriteFenceLikeCpp,
    socket_timeouts: SocketTimeoutsLikeCpp,
) -> Fut
where
    F: Fn(
        AccountInfo,
        flume::Receiver<wow_packet::WorldPacket>,
        flume::Sender<Vec<u8>>,
        SocketWriteFenceLikeCpp,
        SocketTimeoutsLikeCpp,
    ) -> Fut,
{
    callback(
        account_info,
        packet_rx,
        send_tx,
        send_write_fence_like_cpp,
        socket_timeouts,
    )
}

/// Start the world server TCP listener on the given address.
///
/// After each connection completes the auth handshake, channels are created
/// for the session. The `on_session_ready` callback receives:
/// - `AccountInfo` from the auth handshake
/// - `packet_rx` — channel to receive packets from the socket
/// - `send_tx` — channel to send responses back through the socket
/// - `send_write_fence` — FIFO completion fence paired with `send_tx`
/// - `socket_timeouts` — transport-owned session liveness policy
///
/// The callback should create a WorldSession and return a future that runs
/// the session update loop. This future is spawned alongside the socket
/// read and write loops.
pub async fn start_world_listener<F, Fut>(
    bind_addr: SocketAddr,
    account_lookup: Arc<dyn AccountLookup>,
    listener_policy: WorldListenerPolicyLikeCpp,
    on_session_ready: F,
    ready_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
) -> std::io::Result<()>
where
    F: Fn(
            AccountInfo,
            flume::Receiver<wow_packet::WorldPacket>,
            flume::Sender<Vec<u8>>,
            SocketWriteFenceLikeCpp,
            SocketTimeoutsLikeCpp,
        ) -> Fut
        + Send
        + Sync
        + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = match TcpListener::bind(bind_addr).await {
        Ok(listener) => {
            let _ = ready_tx.send(Ok(()));
            listener
        }
        Err(error) => {
            let _ = ready_tx.send(Err(error.to_string()));
            return Err(error);
        }
    };
    info!("World server listening on {bind_addr}");

    let on_session = Arc::new(on_session_ready);

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {e}");
                continue;
            }
        };

        let lookup = Arc::clone(&account_lookup);
        let policy = listener_policy.clone();
        let callback = Arc::clone(&on_session);

        tokio::spawn(async move {
            let mut socket = WorldSocket::new(stream, addr);
            socket.set_max_overspeed_pings_like_cpp(policy.max_overspeed_pings);
            socket.set_ip_location_store_like_cpp(policy.ip_location_store.clone());

            // Phase 1: Handshake (connection strings + auth challenge)
            if let Err(e) = socket.start().await {
                error!("Handshake failed for {addr}: {e}");
                return;
            }

            // Phase 2: Authentication
            if let Err(e) = socket.authenticate(lookup.as_ref()).await {
                error!("Authentication failed for {addr}: {e}");
                return;
            }

            // Get account info and attach the client's real IP address
            // and the derived session key from realm auth
            let account_info = match socket.account_info() {
                Some(info) => {
                    let mut ai = info.clone();
                    ai.client_address = Some(addr.ip());
                    ai.derived_session_key =
                        socket.session_key().map(|k| k.to_vec()).unwrap_or_default();
                    ai
                }
                None => {
                    error!("No account info after auth for {addr}");
                    return;
                }
            };

            // Phase 3: Create session channels
            let (pkt_rx, send_tx, send_write_fence_like_cpp) = socket.create_session_channels();

            // Phase 4: Split socket into read/write halves
            let pong_tx = send_tx.clone();
            let (reader, writer) = socket.split_for_io(pong_tx);

            // Phase 5: Spawn the write loop (session → TCP)
            tokio::spawn(async move {
                if let Err(e) = writer.run().await {
                    match e {
                        WorldSocketError::Closed => {}
                        _ => error!("Writer error for {addr}: {e}"),
                    }
                }
            });

            // Phase 6: Spawn session update loop
            let session_future = handoff_authenticated_world_session_like_cpp(
                callback.as_ref(),
                account_info,
                pkt_rx,
                send_tx,
                send_write_fence_like_cpp,
                policy.socket_timeouts,
            );
            tokio::spawn(session_future);

            // Phase 7: Run the encrypted read loop (blocks until disconnect)
            if let Err(e) = reader.run().await {
                match e {
                    WorldSocketError::Closed => {
                        info!("Client {addr} disconnected");
                    }
                    _ => {
                        error!("Socket error for {addr}: {e}");
                    }
                }
            }
        });
    }
}

// ── Instance listener ───────────────────────────────────────────

/// Seeds from C# WorldSocket.cs — must match the realm socket values.
const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2, 0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA, 0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];

/// Start the instance server TCP listener.
///
/// Instance connections come from clients that received `SMSG_CONNECT_TO`.
/// They perform a handshake (connection strings + AuthChallenge), then send
/// `AuthContinuedSession` instead of `AuthSession`.
pub async fn start_instance_listener(
    bind_addr: SocketAddr,
    session_mgr: Arc<SessionManager>,
    ready_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
) -> std::io::Result<()> {
    let listener = match TcpListener::bind(bind_addr).await {
        Ok(listener) => {
            let _ = ready_tx.send(Ok(()));
            listener
        }
        Err(error) => {
            let _ = ready_tx.send(Err(error.to_string()));
            return Err(error);
        }
    };
    info!("Instance server listening on {bind_addr}");

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept instance connection: {e}");
                continue;
            }
        };

        let mgr = Arc::clone(&session_mgr);

        tokio::spawn(async move {
            if let Err(e) = handle_instance_connection(stream, addr, &mgr).await {
                match e {
                    WorldSocketError::Closed => {
                        debug!("Instance client {addr} disconnected");
                    }
                    _ => {
                        error!("Instance connection error for {addr}: {e}");
                    }
                }
            }
        });
    }
}

/// Full instance connection flow: handshake → AuthContinuedSession → encryption → I/O.
async fn handle_instance_connection(
    stream: tokio::net::TcpStream,
    addr: SocketAddr,
    session_mgr: &SessionManager,
) -> Result<(), WorldSocketError> {
    let mut socket = WorldSocket::new(stream, addr);
    socket.mark_instance_connection_like_cpp();

    // Phase 1: Connection strings + AuthChallenge (same as realm)
    socket.start().await?;

    // Phase 2: Read AuthContinuedSession (unencrypted)
    let pkt = socket.read_unencrypted_packet().await?;
    let opcode = pkt.opcode_raw();

    if opcode != ClientOpcodes::AuthContinuedSession as u16 {
        return Err(WorldSocketError::AuthFailed(format!(
            "expected AuthContinuedSession (0x{:04X}), got 0x{opcode:04X}",
            ClientOpcodes::AuthContinuedSession as u16
        )));
    }

    let mut pkt = pkt;
    pkt.skip_opcode();
    let auth = AuthContinuedSession::read(&mut pkt)?;

    // Phase 3: Extract account_id from ConnectToKey
    let key = ConnectToKey::from_raw(auth.key);
    if key.connection_type != 1 {
        return Err(WorldSocketError::AuthFailed(
            "expected Instance connection type".into(),
        ));
    }

    let account_id = key.account_id;
    info!("Instance AuthContinuedSession from account {account_id} at {addr}");

    // Phase 4: Validate against SessionManager
    let validated = session_mgr
        .validate_and_take(account_id, auth.key)
        .map_err(|e| WorldSocketError::AuthFailed(format!("session manager: {e}")))?;

    let session_key = &validated.session_key;
    let server_challenge = *socket.server_challenge();

    // Phase 5: Validate HMAC-SHA256 digest
    // NOTE: AuthContinuedSession uses session_key DIRECTLY as HMAC key,
    // unlike AuthSession which uses SHA256(session_key).
    // C# ref: WorldSocket.cs HandleAuthContinuedSessionCallback line 777.

    // DEBUG: Log all HMAC inputs for comparison with C# server
    info!(
        "[DEBUG-HMAC] sessionKey({}): {}",
        session_key.len(),
        session_key
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] authKey(i64): {} bytes: {}",
        auth.key,
        auth.key
            .to_le_bytes()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] localChallenge({}): {}",
        auth.local_challenge.len(),
        auth.local_challenge
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] serverChallenge({}): {}",
        server_challenge.len(),
        server_challenge
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] continuedSeed: {}",
        CONTINUED_SESSION_SEED
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );

    let mut hmac = HmacSha256::new(session_key);
    hmac.update(&auth.key.to_le_bytes());
    hmac.update(&auth.local_challenge);
    hmac.update(&server_challenge);
    hmac.update(&CONTINUED_SESSION_SEED);
    let server_digest = hmac.finalize();

    info!(
        "[DEBUG-HMAC] serverDigest: {}",
        server_digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] clientDigest({}): {}",
        auth.digest.len(),
        auth.digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );

    if server_digest[..24] != auth.digest {
        return Err(WorldSocketError::AuthFailed(
            "AuthContinuedSession HMAC digest mismatch".into(),
        ));
    }

    debug!("Instance HMAC validated for account {account_id}");

    // Phase 6: Derive encryption key
    let encrypt_key = {
        let mut hmac = HmacSha256::new(session_key);
        hmac.update(&auth.local_challenge);
        hmac.update(&server_challenge);
        hmac.update(&ENCRYPTION_KEY_SEED);
        let full = hmac.finalize();
        let mut ek = [0u8; 16];
        ek.copy_from_slice(&full[..16]);
        ek
    };

    // Phase 7: Send EnterEncryptedMode
    let signature = sign_enable_encryption(&encrypt_key, true);
    let enter_encrypted = EnterEncryptedMode {
        signature,
        enabled: true,
    };
    socket.send_unencrypted_packet(&enter_encrypted).await?;

    // Phase 8: Wait for EnterEncryptedModeAck
    let ack_pkt = socket.read_unencrypted_packet().await?;
    let ack_opcode = ack_pkt.opcode_raw();
    if ack_opcode != ClientOpcodes::EnterEncryptedModeAck as u16 {
        return Err(WorldSocketError::AuthFailed(format!(
            "expected EnterEncryptedModeAck, got 0x{ack_opcode:04X}"
        )));
    }

    // Phase 9: Enable encryption
    socket.set_encrypt_key(encrypt_key);
    socket.handle_enter_encrypted_mode_ack()?;

    // Phase 10: Create channels and deliver InstanceLink
    let (pkt_tx, pkt_rx) = flume::bounded(256);
    let (send_tx, send_rx_for_socket) = flume::bounded(256);

    // pkt_tx → instance reader feeds decoded packets here
    // pkt_rx → session reads packets from here (via InstanceLink)
    // send_tx → session writes serialized packets here
    // send_rx_for_socket → instance writer reads from here
    let instance_link = InstanceLink {
        send_tx: send_tx.clone(),
        send_write_fence_like_cpp: Some(socket.send_write_fence_like_cpp()),
        pkt_rx: Some(pkt_rx),
    };

    if validated.instance_link_tx.send(instance_link).is_err() {
        return Err(WorldSocketError::AuthFailed(
            "session dropped before instance link delivery".into(),
        ));
    }

    // Phase 11: Set up socket channels and split for I/O
    socket.set_session_channel(pkt_tx);
    socket.set_send_channel(send_rx_for_socket);
    let pong_tx = send_tx;
    let (reader, writer) = socket.split_for_io(pong_tx);

    // Spawn writer
    tokio::spawn(async move {
        if let Err(e) = writer.run().await {
            match e {
                WorldSocketError::Closed => {}
                _ => error!("Instance writer error for {addr}: {e}"),
            }
        }
    });

    info!("Instance socket fully linked for account {account_id}");

    // Run reader (blocks until disconnect)
    reader.run().await
}

#[cfg(test)]
mod tests {
    use super::{
        SocketTimeoutsLikeCpp, WorldListenerPolicyLikeCpp,
        handoff_authenticated_world_session_like_cpp, start_instance_listener,
        start_world_listener,
    };
    use crate::world_socket::{AccountInfo, AccountLookup};
    use crate::{SessionManager, SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct MissingAccountLookup;

    impl AccountLookup for MissingAccountLookup {
        fn lookup_account(
            &self,
            _realm_join_ticket: &str,
        ) -> Pin<Box<dyn Future<Output = Option<AccountInfo>> + Send + '_>> {
            Box::pin(async { None })
        }
    }

    fn account_info_fixture() -> AccountInfo {
        AccountInfo {
            id: 42,
            session_key_hex: "A1B2C3D4".to_owned(),
            last_ip: "192.0.2.10".to_owned(),
            is_locked_to_ip: true,
            lock_country: "ES".to_owned(),
            expansion: 9,
            mute_time: 1234,
            locale: "esES".to_owned(),
            recruiter: 77,
            is_a_recruiter: true,
            os: "Wn64".to_owned(),
            timezone_offset: 120,
            battlenet_account_id: 314,
            security: 3,
            is_banned_bnet: false,
            is_banned_account: true,
            win64_auth_seed: [0xA5; 16],
            client_address: Some("198.51.100.7".parse().unwrap()),
            derived_session_key: vec![0x5A; 40],
        }
    }

    fn world_listener_policy_fixture() -> WorldListenerPolicyLikeCpp {
        WorldListenerPolicyLikeCpp {
            max_overspeed_pings: 7,
            socket_timeouts: SocketTimeoutsLikeCpp {
                unauthenticated_secs: 123,
                active_secs: 45,
            },
            ip_location_store: None,
        }
    }

    fn assert_account_info_matches(actual: &AccountInfo, expected: &AccountInfo) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.session_key_hex, expected.session_key_hex);
        assert_eq!(actual.last_ip, expected.last_ip);
        assert_eq!(actual.is_locked_to_ip, expected.is_locked_to_ip);
        assert_eq!(actual.lock_country, expected.lock_country);
        assert_eq!(actual.expansion, expected.expansion);
        assert_eq!(actual.mute_time, expected.mute_time);
        assert_eq!(actual.locale, expected.locale);
        assert_eq!(actual.recruiter, expected.recruiter);
        assert_eq!(actual.is_a_recruiter, expected.is_a_recruiter);
        assert_eq!(actual.os, expected.os);
        assert_eq!(actual.timezone_offset, expected.timezone_offset);
        assert_eq!(actual.battlenet_account_id, expected.battlenet_account_id);
        assert_eq!(actual.security, expected.security);
        assert_eq!(actual.is_banned_bnet, expected.is_banned_bnet);
        assert_eq!(actual.is_banned_account, expected.is_banned_account);
        assert_eq!(actual.win64_auth_seed, expected.win64_auth_seed);
        assert_eq!(actual.client_address, expected.client_address);
        assert_eq!(actual.derived_session_key, expected.derived_session_key);
    }

    #[tokio::test]
    async fn authenticated_world_session_handoff_preserves_transport_contract_exactly_once() {
        let expected_account = account_info_fixture();
        let (_packet_tx, packet_rx) = flume::bounded(256);
        let expected_packet_rx = packet_rx.clone();
        let (send_tx, send_rx) = flume::bounded(256);
        let expected_send_tx = send_tx.clone();
        let expected_write_fence = SocketWriteFenceLikeCpp::default();
        let expected_timeouts = SocketTimeoutsLikeCpp {
            unauthenticated_secs: 321,
            active_secs: 54,
        };
        let callback_count = Arc::new(AtomicUsize::new(0));
        let (observed_tx, observed_rx) = flume::bounded(2);

        let callback = {
            let callback_count = Arc::clone(&callback_count);
            move |account, packet_rx, send_tx, write_fence, socket_timeouts| {
                callback_count.fetch_add(1, Ordering::SeqCst);
                observed_tx
                    .send((account, packet_rx, send_tx, write_fence, socket_timeouts))
                    .unwrap();
                async {}
            }
        };

        handoff_authenticated_world_session_like_cpp(
            &callback,
            expected_account.clone(),
            packet_rx,
            send_tx,
            expected_write_fence.clone(),
            expected_timeouts,
        )
        .await;

        let (account, packet_rx, send_tx, write_fence, socket_timeouts) =
            observed_rx.recv_async().await.unwrap();
        assert_account_info_matches(&account, &expected_account);
        assert!(packet_rx.same_channel(&expected_packet_rx));
        assert!(send_tx.same_channel(&expected_send_tx));
        assert_eq!(socket_timeouts, expected_timeouts);
        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
        assert!(observed_rx.try_recv().is_err());

        let fence_wait = tokio::spawn(async move {
            write_fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_secs(1))
                .await
        });
        let marker = send_rx.recv_async().await.unwrap();
        assert!(expected_write_fence.acknowledge_marker_like_cpp(&marker));
        assert_eq!(
            fence_wait.await.unwrap(),
            SocketWriteFenceWaitResultLikeCpp::Written
        );
    }

    #[tokio::test]
    async fn world_listener_reports_ready_only_after_successful_bind() {
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(start_world_listener(
            "127.0.0.1:0".parse().unwrap(),
            Arc::new(MissingAccountLookup),
            world_listener_policy_fixture(),
            |_account, _packet_rx, _send_tx, _write_fence, _socket_timeouts| async {},
            ready_tx,
        ));

        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), ready_rx)
                .await
                .expect("listener readiness must not hang")
                .expect("listener task must retain readiness sender"),
            Ok(())
        );
        handle.abort();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn world_listener_reports_bind_failure_before_returning() {
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let occupied_addr = occupied.local_addr().unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(start_world_listener(
            occupied_addr,
            Arc::new(MissingAccountLookup),
            world_listener_policy_fixture(),
            |_account, _packet_rx, _send_tx, _write_fence, _socket_timeouts| async {},
            ready_tx,
        ));

        let readiness = tokio::time::timeout(Duration::from_secs(1), ready_rx)
            .await
            .expect("bind failure readiness must not hang")
            .expect("listener must report its bind result");
        assert!(readiness.is_err());
        assert!(handle.await.expect("listener task must join").is_err());
    }

    #[test]
    fn world_listener_source_and_api_are_transport_only() {
        let source = include_str!("accept.rs");
        let start = source
            .find("pub struct WorldListenerPolicyLikeCpp")
            .expect("world listener policy must remain present");
        let end = source[start..]
            .find("// ── Instance listener")
            .map(|offset| start + offset)
            .expect("instance listener boundary must remain present");
        let listener_source = &source[start..end];

        for required in [
            "AccountLookup",
            "WorldListenerPolicyLikeCpp",
            "SocketTimeoutsLikeCpp",
            "SocketWriteFenceLikeCpp",
            "flume::Receiver<wow_packet::WorldPacket>",
            "flume::Sender<Vec<u8>>",
        ] {
            assert!(
                listener_source.contains(required),
                "world listener transport API lost {required}"
            );
        }

        for forbidden in [
            "SessionResources",
            "wow_database",
            "wow_instances",
            "wow_data",
            "wow_loot",
            "PlayerRegistry",
            "GroupRegistry",
            "player_registry",
            "group_registry",
            "LootDropRatesLikeCpp",
            "ReputationRatesLikeCpp",
            "ChatLevelRequirementsLikeCpp",
            "ChatListenRangesLikeCpp",
            "ChatFloodConfigLikeCpp",
            "PacketSpoofConfigLikeCpp",
        ] {
            assert!(
                !listener_source.contains(forbidden),
                "application dependency {forbidden} crossed the world listener boundary"
            );
        }

        let without_transport_store = listener_source
            .replace("IpLocationStore", "")
            .replace("ip_location_store", "");
        for forbidden in ["Store", "_store", "Pool", "_pool"] {
            assert!(
                !without_transport_store.contains(forbidden),
                "application {forbidden} dependency crossed the world listener boundary"
            );
        }
    }

    #[tokio::test]
    async fn instance_listener_reports_ready_only_after_successful_bind() {
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(start_instance_listener(
            "127.0.0.1:0".parse().unwrap(),
            Arc::new(SessionManager::new()),
            ready_tx,
        ));

        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), ready_rx)
                .await
                .expect("listener readiness must not hang")
                .expect("listener task must retain readiness sender"),
            Ok(())
        );
        handle.abort();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn instance_listener_reports_bind_failure_before_returning() {
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let occupied_addr = occupied.local_addr().unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(start_instance_listener(
            occupied_addr,
            Arc::new(SessionManager::new()),
            ready_tx,
        ));

        let readiness = tokio::time::timeout(std::time::Duration::from_secs(1), ready_rx)
            .await
            .expect("bind failure readiness must not hang")
            .expect("listener must report its bind result");
        assert!(readiness.is_err());
        assert!(handle.await.expect("listener task must join").is_err());
    }
}
