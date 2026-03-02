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
use wow_packet::packets::auth::{
    AuthContinuedSession, ConnectToKey, EnterEncryptedMode,
};
use wow_packet::ClientPacket;

use crate::group_registry::{GroupRegistry, PendingInvites};
use crate::player_registry::PlayerRegistry;
use crate::session_mgr::{InstanceLink, SessionManager};
use crate::world_socket::{
    AccountInfo, AccountLookup, WorldSocket, WorldSocketError, sign_enable_encryption,
};

/// Resources needed for creating a WorldSession after authentication.
///
/// Held by the accept loop and cloned for each connection.
pub struct SessionResources {
    pub char_db: Option<Arc<wow_database::CharacterDatabase>>,
    pub login_db: Option<Arc<wow_database::LoginDatabase>>,
    pub world_db: Option<Arc<wow_database::WorldDatabase>>,
    pub guid_generator: Option<Arc<wow_core::ObjectGuidGenerator>>,
    pub item_store: Option<Arc<wow_data::ItemStore>>,
    pub player_stats: Option<Arc<wow_data::PlayerStatsStore>>,
    pub item_stats_store: Option<Arc<wow_data::ItemStatsStore>>,
    pub hotfix_blob_cache: Option<Arc<wow_data::HotfixBlobCache>>,
    pub skill_store: Option<Arc<wow_data::SkillStore>>,
    pub spell_store: Option<Arc<wow_data::SpellStore>>,
    pub area_trigger_store: Option<Arc<wow_data::AreaTriggerStore>>,
    /// Shared registry of all active player sessions (for broadcast).
    pub player_registry: Option<Arc<PlayerRegistry>>,
    /// Shared registry of all active groups.
    pub group_registry: Option<Arc<GroupRegistry>>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub pending_invites: Option<Arc<PendingInvites>>,
    pub realm_id: u16,
    /// External (public) IP from `realmlist.address`.
    pub realm_external_address: [u8; 4],
    /// Local (LAN) IP from `realmlist.localAddress`.
    pub realm_local_address: [u8; 4],
}

/// Start the world server TCP listener on the given address.
///
/// After each connection completes the auth handshake, channels are created
/// for the session. The `on_session_ready` callback receives:
/// - `AccountInfo` from the auth handshake
/// - `packet_rx` — channel to receive packets from the socket
/// - `send_tx` — channel to send responses back through the socket
/// - `SessionResources` — shared resources for the session
///
/// The callback should create a WorldSession and return a future that runs
/// the session update loop. This future is spawned alongside the socket
/// read and write loops.
pub async fn start_world_listener<F, Fut>(
    bind_addr: SocketAddr,
    account_lookup: Arc<dyn AccountLookup>,
    resources: Arc<SessionResources>,
    on_session_ready: F,
) -> std::io::Result<()>
where
    F: Fn(
        AccountInfo,
        flume::Receiver<wow_packet::WorldPacket>,
        flume::Sender<Vec<u8>>,
        Arc<SessionResources>,
    ) -> Fut
    + Send
    + Sync
    + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind(bind_addr).await?;
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
        let res = Arc::clone(&resources);
        let callback = Arc::clone(&on_session);

        tokio::spawn(async move {
            let mut socket = WorldSocket::new(stream, addr);

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
                    ai.derived_session_key = socket.session_key()
                        .map(|k| k.to_vec())
                        .unwrap_or_default();
                    ai
                }
                None => {
                    error!("No account info after auth for {addr}");
                    return;
                }
            };

            // Phase 3: Create session channels
            let (pkt_rx, send_tx) = socket.create_session_channels();

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
            let session_future = callback(account_info, pkt_rx, send_tx, res);
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
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2,
    0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA,
    0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];

/// Start the instance server TCP listener.
///
/// Instance connections come from clients that received `SMSG_CONNECT_TO`.
/// They perform a handshake (connection strings + AuthChallenge), then send
/// `AuthContinuedSession` instead of `AuthSession`.
pub async fn start_instance_listener(
    bind_addr: SocketAddr,
    session_mgr: Arc<SessionManager>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
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
    info!("[DEBUG-HMAC] sessionKey({}): {}",
        session_key.len(),
        session_key.iter().map(|b| format!("{b:02X}")).collect::<String>());
    info!("[DEBUG-HMAC] authKey(i64): {} bytes: {}",
        auth.key,
        auth.key.to_le_bytes().iter().map(|b| format!("{b:02X}")).collect::<String>());
    info!("[DEBUG-HMAC] localChallenge({}): {}",
        auth.local_challenge.len(),
        auth.local_challenge.iter().map(|b| format!("{b:02X}")).collect::<String>());
    info!("[DEBUG-HMAC] serverChallenge({}): {}",
        server_challenge.len(),
        server_challenge.iter().map(|b| format!("{b:02X}")).collect::<String>());
    info!("[DEBUG-HMAC] continuedSeed: {}",
        CONTINUED_SESSION_SEED.iter().map(|b| format!("{b:02X}")).collect::<String>());

    let mut hmac = HmacSha256::new(session_key);
    hmac.update(&auth.key.to_le_bytes());
    hmac.update(&auth.local_challenge);
    hmac.update(&server_challenge);
    hmac.update(&CONTINUED_SESSION_SEED);
    let server_digest = hmac.finalize();

    info!("[DEBUG-HMAC] serverDigest: {}",
        server_digest.iter().map(|b| format!("{b:02X}")).collect::<String>());
    info!("[DEBUG-HMAC] clientDigest({}): {}",
        auth.digest.len(),
        auth.digest.iter().map(|b| format!("{b:02X}")).collect::<String>());

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
