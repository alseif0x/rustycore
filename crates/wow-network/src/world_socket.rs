// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-client `WorldSocket` — handles the connection lifecycle from initial
//! handshake through encrypted packet I/O.
//!
//! ## Connection flow
//!
//! 1. Server sends: `"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"`
//! 2. Client sends: `"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n"`
//! 3. Server sends `SMSG_AUTH_CHALLENGE`
//! 4. Client sends `CMSG_AUTH_SESSION`
//! 5. Server validates digest, derives keys
//! 6. Server sends `AuthResponse` + `EnterEncryptedMode` (Ed25519 signed)
//! 7. Client sends `EnterEncryptedModeAck`
//! 8. All subsequent packets are AES-128-GCM encrypted

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

use bytes::BytesMut;
use num_traits::ToPrimitive;
use rand::Rng;
use sha2::{Sha256, Digest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info, trace, warn};

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_crypto::{HmacSha256, SessionKeyGenerator256, WorldCrypt};
use wow_packet::header::{PacketHeader, HEADER_SIZE, TAG_SIZE};
use wow_packet::packets::auth::{
    AuthChallenge, AuthSession,
    EnterEncryptedMode, Ping, Pong,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket, compression};

// ── Protocol constants ────────────────────────────────────────────

const SERVER_CONNECTION_INIT: &[u8] = b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n";
const CLIENT_CONNECTION_INIT: &[u8] = b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n";

const AUTH_CHECK_SEED: [u8; 16] = [
    0xC5, 0xC6, 0x98, 0x95, 0x76, 0x3F, 0x1D, 0xCD,
    0xB6, 0xA1, 0x37, 0x28, 0xB3, 0x12, 0xFF, 0x8A,
];

const SESSION_KEY_SEED: [u8; 16] = [
    0x58, 0xCB, 0xCF, 0x40, 0xFE, 0x2E, 0xCE, 0xA6,
    0x5A, 0x90, 0xB8, 0x01, 0x68, 0x6C, 0x28, 0x0B,
];

#[allow(dead_code)]
const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2,
    0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA,
    0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];

const ENABLE_ENCRYPTION_SEED: [u8; 16] = [
    0x90, 0x9C, 0xD0, 0x50, 0x5A, 0x2C, 0x14, 0xDD,
    0x5C, 0x2C, 0xC0, 0x64, 0x14, 0xF3, 0xFE, 0xC9,
];

const ENABLE_ENCRYPTION_CONTEXT: [u8; 16] = [
    0xA7, 0x1F, 0xB6, 0x9B, 0xC9, 0x7C, 0xDD, 0x96,
    0xE9, 0xBB, 0xB8, 0x21, 0x39, 0x8D, 0x5A, 0xD4,
];

// Build-specific auth seeds are loaded from the `build_info` DB table at startup
// and stored in SessionResources. They are passed to AccountInfo during lookup.

/// Ed25519 private key seed used for signing `EnterEncryptedMode`.
const ENTER_ENCRYPTED_MODE_PRIVATE_KEY: [u8; 32] = [
    0x08, 0xBD, 0xC7, 0xA3, 0xCC, 0xC3, 0x4F, 0x3F,
    0x6A, 0x0B, 0xFF, 0xCF, 0x31, 0xC1, 0xB6, 0x97,
    0x69, 0x1E, 0x72, 0x9A, 0x0A, 0xAB, 0x2C, 0x77,
    0xC3, 0x6F, 0x8A, 0xE7, 0x5A, 0x9A, 0xA7, 0xC9,
];

// ── Error type ────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum WorldSocketError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("packet error: {0}")]
    Packet(#[from] wow_packet::PacketError),

    #[error("crypto error: {0}")]
    Crypto(#[from] wow_crypto::world_crypt::WorldCryptError),

    #[error("invalid connection string from client")]
    InvalidConnectionString,

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("invalid packet size: {0}")]
    InvalidSize(i32),

    #[error("unknown opcode: 0x{0:04X}")]
    UnknownOpcode(u16),

    #[error("connection closed")]
    Closed,
}

// ── Socket state machine ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SocketState {
    Uninitialized,
    ConnectionStringSent,
    AuthChallengeSent,
    AuthSessionReceived,
    EncryptedModeEnabled,
}

// ── Account info from DB ─────────────────────────────────────────

/// Account information retrieved from the login database during auth.
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub id: u32,
    pub session_key_hex: String,
    pub last_ip: String,
    pub is_locked_to_ip: bool,
    pub lock_country: String,
    pub expansion: u8,
    pub mute_time: i64,
    pub locale: String,
    pub recruiter: u32,
    pub os: String,
    pub timezone_offset: i32,
    pub battlenet_account_id: u32,
    pub security: u8,
    pub is_banned_bnet: bool,
    pub is_banned_account: bool,
    /// Build-specific Win64 auth seed (16 bytes, from `build_info` table).
    pub win64_auth_seed: [u8; 16],
    /// Client's actual IP address (set by accept loop, not from DB).
    pub client_address: Option<std::net::IpAddr>,
    /// Derived session key (40 bytes) from realm auth handshake.
    /// This is HMAC-derived from the BNet key and challenge data.
    /// Used for instance socket HMAC validation (AuthContinuedSession).
    pub derived_session_key: Vec<u8>,
}

// ── WorldSocket ──────────────────────────────────────────────────

/// Per-client connection handler for the world server.
pub struct WorldSocket {
    stream: TcpStream,
    addr: SocketAddr,

    // Crypto
    crypt: Option<WorldCrypt>,
    server_challenge: [u8; 16],
    encrypt_key: Option<[u8; 16]>,
    session_key: Option<Vec<u8>>,

    // Session channel — sends deserialized packets to WorldSession
    session_tx: Option<flume::Sender<WorldPacket>>,

    // Outbound channel — receives serialized bytes from WorldSession
    send_rx: Option<flume::Receiver<Vec<u8>>>,

    // State
    state: SocketState,

    // Account info (populated after AuthSession)
    account_info: Option<AccountInfo>,

    // Tracks the number of packet headers sent before encryption is enabled.
    // The WoW client increments its receive counter for every header it reads,
    // even for unencrypted packets. The SocketWriter must start its server
    // counter at this offset to keep in sync with the client.
    unencrypted_packets_sent: u64,

    // Tracks the number of packets received from the client before encryption.
    // The WoW client always increments its send counter in Encrypt(), even for
    // unencrypted packets (AuthSession, EnterEncryptedModeAck). The SocketReader
    // must start its client counter at this offset to decrypt correctly.
    unencrypted_packets_received: u64,
}

impl WorldSocket {
    /// Create a new socket wrapper for an accepted TCP connection.
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        let mut challenge = [0u8; 16];
        rand::thread_rng().fill(&mut challenge);

        Self {
            stream,
            addr,
            crypt: None,
            server_challenge: challenge,
            encrypt_key: None,
            session_key: None,
            session_tx: None,
            send_rx: None,
            state: SocketState::Uninitialized,
            account_info: None,
            unencrypted_packets_sent: 0,
            unencrypted_packets_received: 0,
        }
    }

    /// Set the session channel for forwarding packets to the WorldSession.
    pub fn set_session_channel(&mut self, tx: flume::Sender<WorldPacket>) {
        self.session_tx = Some(tx);
    }

    /// Get the remote address of this connection.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get a reference to the account info (available after auth).
    pub fn account_info(&self) -> Option<&AccountInfo> {
        self.account_info.as_ref()
    }

    /// Get the session key (available after auth).
    pub fn session_key(&self) -> Option<&[u8]> {
        self.session_key.as_deref()
    }

    /// Whether encryption is active.
    pub fn is_encrypted(&self) -> bool {
        self.crypt.is_some()
    }

    // ── Handshake flow ────────────────────────────────────────────

    /// Start the handshake: send server connection string.
    pub async fn start(&mut self) -> Result<(), WorldSocketError> {
        info!("WorldSocket: new connection from {}", self.addr);

        // Step 1: Send server connection string
        self.stream.write_all(SERVER_CONNECTION_INIT).await?;
        self.state = SocketState::ConnectionStringSent;
        debug!("Sent server connection string to {}", self.addr);

        // Step 2: Read client connection string
        let mut buf = vec![0u8; CLIENT_CONNECTION_INIT.len()];
        self.stream.read_exact(&mut buf).await?;

        if buf != CLIENT_CONNECTION_INIT {
            warn!("Invalid connection string from {}", self.addr);
            return Err(WorldSocketError::InvalidConnectionString);
        }
        debug!("Received valid client connection string from {}", self.addr);

        // Step 3: Send AuthChallenge
        self.send_auth_challenge().await?;

        Ok(())
    }

    /// Send SMSG_AUTH_CHALLENGE to the client.
    async fn send_auth_challenge(&mut self) -> Result<(), WorldSocketError> {
        let mut dos_challenge = [0u8; 32];
        rand::thread_rng().fill(&mut dos_challenge);

        let challenge = AuthChallenge {
            dos_challenge,
            challenge: self.server_challenge,
            dos_zero_bits: 1,
        };

        self.send_unencrypted_packet(&challenge).await?;
        self.state = SocketState::AuthChallengeSent;
        debug!("Sent AuthChallenge to {}", self.addr);
        Ok(())
    }

    /// Validate an incoming AuthSession packet.
    ///
    /// This performs the HMAC-SHA256 digest validation, derives the session key
    /// and encryption key, then sends AuthResponse + EnterEncryptedMode.
    pub async fn handle_auth_session(
        &mut self,
        auth_session: &AuthSession,
        account: &AccountInfo,
    ) -> Result<(), WorldSocketError> {
        let key_data = hex_to_bytes(&account.session_key_hex);

        // Step 1: Hash(KeyData || PlatformAuthSeed) — SHA256 with build-specific seed
        // C#: digestKeyHash.Process(KeyData, len); digestKeyHash.Finish(Win64AuthSeed);
        let platform_seed = match account.os.as_str() {
            "Wn64" => &account.win64_auth_seed,
            "Mc64" => {
                return Err(WorldSocketError::AuthFailed(
                    "Mac64 auth seed not configured".into(),
                ));
            }
            other => {
                return Err(WorldSocketError::AuthFailed(
                    format!("unsupported platform: {other}"),
                ));
            }
        };
        let digest_key_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&key_data);
            hasher.update(platform_seed);
            let h: [u8; 32] = hasher.finalize().into();
            h
        };

        // Step 2: HMAC-SHA256(digest_key_hash, local_challenge || server_challenge || AuthCheckSeed)
        let mut hmac = HmacSha256::new(&digest_key_hash);
        hmac.update(&auth_session.local_challenge);
        hmac.update(&self.server_challenge);
        hmac.update(&AUTH_CHECK_SEED);
        let server_digest = hmac.finalize();

        // Step 3: Compare first 24 bytes with client's digest
        if server_digest[..24] != auth_session.digest {
            debug!(
                "HMAC mismatch debug:\n  key_data({} bytes): {}\n  platform_seed: {}\n  digest_key_hash: {}\n  local_challenge: {}\n  server_challenge: {}\n  auth_check_seed: {}\n  server_digest: {}\n  client_digest: {}",
                key_data.len(),
                key_data.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                platform_seed.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                digest_key_hash.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                auth_session.local_challenge.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                self.server_challenge.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                AUTH_CHECK_SEED.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                server_digest.iter().map(|b| format!("{b:02X}")).collect::<String>(),
                auth_session.digest.iter().map(|b| format!("{b:02X}")).collect::<String>(),
            );
            return Err(WorldSocketError::AuthFailed(
                "HMAC digest mismatch".into(),
            ));
        }
        debug!("Auth digest validated for {}", self.addr);

        // Step 4: Derive session key (40 bytes)
        let session_key = {
            let key_hash = {
                let mut hasher = Sha256::new();
                hasher.update(&key_data);
                let h: [u8; 32] = hasher.finalize().into();
                h
            };

            let mut hmac = HmacSha256::new(&key_hash);
            hmac.update(&self.server_challenge);
            hmac.update(&auth_session.local_challenge);
            hmac.update(&SESSION_KEY_SEED);
            let seed = hmac.finalize();

            let mut session_key = vec![0u8; 40];
            let mut keygen = SessionKeyGenerator256::new(&seed);
            keygen.generate(&mut session_key);
            session_key
        };

        // Step 5: Derive encryption key (16 bytes)
        let encrypt_key = {
            let mut hmac = HmacSha256::new(&session_key);
            hmac.update(&auth_session.local_challenge);
            hmac.update(&self.server_challenge);
            hmac.update(&ENCRYPTION_KEY_SEED);
            let full = hmac.finalize();
            let mut key = [0u8; 16];
            key.copy_from_slice(&full[..16]);
            key
        };

        self.session_key = Some(session_key);
        self.encrypt_key = Some(encrypt_key);
        self.account_info = Some(account.clone());
        self.state = SocketState::AuthSessionReceived;

        info!("Account {} authenticated from {}", account.id, self.addr);
        Ok(())
    }

    /// Send EnterEncryptedMode to the client (no AuthResponse — that comes
    /// later as an encrypted packet from the WorldSession, matching C# flow).
    pub async fn send_enter_encrypted_mode(&mut self) -> Result<(), WorldSocketError> {
        let encrypt_key = self
            .encrypt_key
            .ok_or_else(|| WorldSocketError::AuthFailed("no encryption key".into()))?;

        let signature = sign_enable_encryption(&encrypt_key, true);

        let enter_encrypted = EnterEncryptedMode {
            signature,
            enabled: true,
        };
        self.send_unencrypted_packet(&enter_encrypted).await?;
        debug!("Sent EnterEncryptedMode to {}", self.addr);

        Ok(())
    }

    /// Handle the client's EnterEncryptedModeAck — enable crypto.
    pub fn handle_enter_encrypted_mode_ack(&mut self) -> Result<(), WorldSocketError> {
        let key = self
            .encrypt_key
            .ok_or_else(|| WorldSocketError::AuthFailed("no encryption key".into()))?;

        self.crypt = Some(WorldCrypt::new(&key));
        self.state = SocketState::EncryptedModeEnabled;
        info!("Encryption enabled for {}", self.addr);
        Ok(())
    }

    /// Set the encryption key (used by the instance handshake which derives
    /// the key externally from AuthContinuedSession).
    pub fn set_encrypt_key(&mut self, key: [u8; 16]) {
        self.encrypt_key = Some(key);
    }

    /// Take the send_rx channel out of the socket.
    pub fn take_send_channel(&mut self) -> Option<flume::Receiver<Vec<u8>>> {
        self.send_rx.take()
    }

    /// Set the send channel (used by instance handshake).
    pub fn set_send_channel(&mut self, rx: flume::Receiver<Vec<u8>>) {
        self.send_rx = Some(rx);
    }

    // ── Packet I/O ────────────────────────────────────────────────

    /// Send a server packet WITHOUT encryption (used during handshake).
    /// Public version for use by the instance accept loop.
    ///
    /// Each call increments `unencrypted_packets_sent` because the WoW client
    /// tracks every packet header from the server (including unencrypted ones)
    /// to keep its AES-GCM nonce counter in sync.
    pub async fn send_unencrypted_packet(
        &mut self,
        pkt: &impl ServerPacket,
    ) -> Result<(), WorldSocketError> {
        let data = pkt.to_bytes();
        let size = data.len() as i32;

        let header = PacketHeader::new(size, [0u8; TAG_SIZE]);
        let header_bytes = header.to_bytes();

        self.stream.write_all(&header_bytes).await?;
        self.stream.write_all(&data).await?;
        self.stream.flush().await?;

        self.unencrypted_packets_sent += 1;
        Ok(())
    }

    /// Send a server packet with encryption (after handshake).
    pub async fn send_packet(
        &mut self,
        pkt: &impl ServerPacket,
    ) -> Result<(), WorldSocketError> {
        let crypt = match self.crypt.as_mut() {
            Some(c) => c,
            None => return self.send_unencrypted_packet(pkt).await,
        };

        let mut data = pkt.to_bytes();
        let opcode_raw = if data.len() >= 2 {
            u16::from_le_bytes([data[0], data[1]])
        } else {
            0
        };

        // Compress if above threshold
        if data.len() > compression::COMPRESSION_THRESHOLD {
            let opcode_bytes = opcode_raw.to_le_bytes();
            let compressed = compression::compress_packet(&opcode_bytes, &data[2..]);

            // Replace data with CompressedPacket opcode + compressed payload
            let comp_opcode = ServerOpcodes::CompressedPacket
                .to_u16()
                .unwrap_or(0);
            data = Vec::with_capacity(2 + compressed.len());
            data.extend_from_slice(&comp_opcode.to_le_bytes());
            data.extend_from_slice(&compressed);
        }

        // Encrypt
        let (encrypted, tag) = crypt.encrypt(&data, &[])?;

        let header = PacketHeader::new(encrypted.len() as i32, tag);
        let header_bytes = header.to_bytes();

        self.stream.write_all(&header_bytes).await?;
        self.stream.write_all(&encrypted).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// Read a single unencrypted packet from the wire (during handshake).
    ///
    /// Each call increments `unencrypted_packets_received` because the WoW
    /// client increments its send counter for every packet it sends (including
    /// unencrypted ones like AuthSession and EnterEncryptedModeAck).
    pub async fn read_unencrypted_packet(&mut self) -> Result<WorldPacket, WorldSocketError> {
        let mut header_buf = [0u8; HEADER_SIZE];
        self.stream.read_exact(&mut header_buf).await?;

        let header = PacketHeader::read(&header_buf);
        if !header.is_valid_size() {
            return Err(WorldSocketError::InvalidSize(header.size));
        }

        let mut data = vec![0u8; header.size as usize];
        self.stream.read_exact(&mut data).await?;

        self.unencrypted_packets_received += 1;

        Ok(WorldPacket::new_client(BytesMut::from(data.as_slice())))
    }

    /// Read a single encrypted packet from the wire.
    pub async fn read_encrypted_packet(&mut self) -> Result<WorldPacket, WorldSocketError> {
        let crypt = self
            .crypt
            .as_mut()
            .ok_or_else(|| WorldSocketError::AuthFailed("encryption not enabled".into()))?;

        let mut header_buf = [0u8; HEADER_SIZE];
        self.stream.read_exact(&mut header_buf).await?;

        let header = PacketHeader::read(&header_buf);
        if !header.is_valid_size() {
            return Err(WorldSocketError::InvalidSize(header.size));
        }

        let mut encrypted = vec![0u8; header.size as usize];
        self.stream.read_exact(&mut encrypted).await?;

        let data = crypt.decrypt(&encrypted, &header.tag, &[])?;
        Ok(WorldPacket::new_client(BytesMut::from(data.as_slice())))
    }

    /// Perform the authentication handshake.
    ///
    /// Waits for AuthSession, validates, sends AuthResponse + EnterEncryptedMode,
    /// and waits for EnterEncryptedModeAck. After this call succeeds, the socket
    /// is ready for encrypted packet I/O.
    pub async fn authenticate(
        &mut self,
        account_lookup: &dyn AccountLookup,
    ) -> Result<(), WorldSocketError> {
        // Phase 1: Wait for AuthSession (unencrypted)
        let pkt = self.read_unencrypted_packet().await?;
        let opcode = pkt.opcode_raw();

        if opcode != ClientOpcodes::AuthSession as u16 {
            return Err(WorldSocketError::AuthFailed(format!(
                "expected AuthSession (0x{:04X}), got 0x{opcode:04X}",
                ClientOpcodes::AuthSession as u16
            )));
        }

        let mut pkt = pkt;
        pkt.skip_opcode();
        let auth_session = AuthSession::read(&mut pkt)?;

        // Look up the account
        tracing::debug!(
            "Auth ticket received: '{}' (len={})",
            &auth_session.realm_join_ticket,
            auth_session.realm_join_ticket.len()
        );
        let account = account_lookup
            .lookup_account(&auth_session.realm_join_ticket)
            .await
            .ok_or_else(|| {
                tracing::warn!("Ticket lookup failed for: '{}'", &auth_session.realm_join_ticket);
                WorldSocketError::AuthFailed("account not found for ticket".into())
            })?;

        self.handle_auth_session(&auth_session, &account).await?;

        // Send EnterEncryptedMode only — AuthResponse is sent later as an
        // encrypted packet from the WorldSession (matches C# flow).
        self.send_enter_encrypted_mode().await?;

        // Phase 2: Wait for EnterEncryptedModeAck (unencrypted)
        let pkt = self.read_unencrypted_packet().await?;
        let opcode = pkt.opcode_raw();

        if opcode != ClientOpcodes::EnterEncryptedModeAck as u16 {
            return Err(WorldSocketError::AuthFailed(format!(
                "expected EnterEncryptedModeAck (0x{:04X}), got 0x{opcode:04X}",
                ClientOpcodes::EnterEncryptedModeAck as u16
            )));
        }

        self.handle_enter_encrypted_mode_ack()?;
        Ok(())
    }

    /// Set up session channels and return the receivers/sender for session creation.
    ///
    /// Returns `(packet_rx, send_tx)` — the session reads packets from `packet_rx`
    /// and writes responses via `send_tx`.
    pub fn create_session_channels(&mut self) -> (flume::Receiver<WorldPacket>, flume::Sender<Vec<u8>>) {
        let (pkt_tx, pkt_rx) = flume::bounded(256);
        let (send_tx, send_rx) = flume::bounded(256);

        self.session_tx = Some(pkt_tx);
        self.send_rx = Some(send_rx);

        (pkt_rx, send_tx)
    }

    /// Run the encrypted read loop, forwarding packets to the session channel.
    ///
    /// Also spawns a writer task that sends outbound packets from the session.
    /// Runs until the connection is closed or an error occurs.
    pub async fn read_loop(&mut self) -> Result<(), WorldSocketError> {
        loop {
            let pkt = match self.read_encrypted_packet().await {
                Ok(p) => p,
                Err(WorldSocketError::Io(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    info!("Client {} disconnected", self.addr);
                    return Ok(());
                }
                Err(e) => return Err(e),
            };

            let opcode = pkt.opcode_raw();

            // Handle Ping inline
            if opcode == ClientOpcodes::Ping as u16 {
                let mut pkt = pkt;
                pkt.skip_opcode();
                if let Ok(ping) = Ping::read(&mut pkt) {
                    let pong = Pong { serial: ping.serial };
                    self.send_packet(&pong).await?;
                }
                continue;
            }

            // Forward to session
            if let Some(ref tx) = self.session_tx {
                if tx.send(pkt).is_err() {
                    warn!("Session channel closed for {}", self.addr);
                    return Err(WorldSocketError::Closed);
                }
            } else {
                debug!("No session channel; dropping packet 0x{opcode:04X} from {}", self.addr);
            }
        }
    }

    /// Main read loop — reads and dispatches packets after handshake is complete.
    ///
    /// Legacy method that combines authenticate + read_loop for backward
    /// compatibility. Prefer using `authenticate()` + `create_session_channels()` +
    /// `read_loop()` separately for better session control.
    pub async fn read_loop_with_auth(
        &mut self,
        account_lookup: &dyn AccountLookup,
    ) -> Result<(), WorldSocketError> {
        self.authenticate(account_lookup).await?;
        self.read_loop().await
    }
}

// ── Split I/O types ─────────────────────────────────────────────

impl WorldSocket {
    /// Split the authenticated socket into separate read/write halves for
    /// concurrent I/O.
    ///
    /// The `pong_tx` sender is used by the reader to send Pong responses
    /// inline (without going through the session). It should be a clone of
    /// the session's `send_tx`.
    ///
    /// # Panics
    ///
    /// Panics if called before authentication completes (no encryption key,
    /// session channel, or send channel).
    pub fn split_for_io(self, pong_tx: flume::Sender<Vec<u8>>) -> (SocketReader, SocketWriter) {
        let encrypt_key = self
            .encrypt_key
            .expect("split_for_io: no encryption key — call authenticate() first");
        let session_tx = self
            .session_tx
            .expect("split_for_io: no session channel — call create_session_channels() first");
        let send_rx = self
            .send_rx
            .expect("split_for_io: no send channel — call create_session_channels() first");

        let (read_half, write_half) = self.stream.into_split();

        // The reader must start at the number of unencrypted packets received,
        // because the WoW client always increments its send counter in Encrypt(),
        // even for unencrypted packets (AuthSession, EnterEncryptedModeAck).
        // This matches C#'s WorldCrypt.Decrypt() which always increments
        // _clientCounter, even when IsInitialized is false.
        let reader = SocketReader {
            reader: read_half,
            crypt: WorldCrypt::new_with_client_counter(&encrypt_key, self.unencrypted_packets_received),
            session_tx,
            pong_tx,
            addr: self.addr,
        };

        // The writer must start at the number of unencrypted packets sent,
        // because the WoW client increments its receive counter for every
        // packet header — including headers for unencrypted packets. This
        // matches C#'s WorldCrypt.Encrypt() which always increments
        // _serverCounter, even when IsInitialized is false.
        let writer = SocketWriter {
            writer: write_half,
            crypt: WorldCrypt::new_with_server_counter(&encrypt_key, self.unencrypted_packets_sent),
            send_rx,
            addr: self.addr,
            compressor: compression::PacketCompressor::new(),
        };

        info!(
            "Split socket for {}: writer starting at server_counter={}, reader starting at client_counter={}",
            self.addr, self.unencrypted_packets_sent, self.unencrypted_packets_received
        );

        (reader, writer)
    }

    /// Get the encryption key (available after auth). Used by the instance
    /// listener to construct separate reader/writer crypto.
    pub fn encrypt_key(&self) -> Option<&[u8; 16]> {
        self.encrypt_key.as_ref()
    }

    /// Get the server challenge bytes (needed for AuthContinuedSession validation).
    pub fn server_challenge(&self) -> &[u8; 16] {
        &self.server_challenge
    }
}

/// Read half of a split WorldSocket — decrypts and dispatches incoming packets.
pub struct SocketReader {
    reader: tokio::net::tcp::OwnedReadHalf,
    crypt: WorldCrypt,
    session_tx: flume::Sender<WorldPacket>,
    /// Sends serialized Pong packets to the write loop (bypasses session).
    pong_tx: flume::Sender<Vec<u8>>,
    addr: SocketAddr,
}

impl SocketReader {
    /// Run the read loop: decrypt packets from TCP and dispatch to session.
    ///
    /// Returns `Ok(())` on clean disconnect, `Err` on protocol/I/O errors.
    pub async fn run(mut self) -> Result<(), WorldSocketError> {
        info!("Reader[{}]: encrypted read loop started", self.addr);
        loop {
            // Read header
            let mut header_buf = [0u8; HEADER_SIZE];
            match self.reader.read_exact(&mut header_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    info!("Client {} disconnected (reader got EOF)", self.addr);
                    return Ok(());
                }
                Err(e) => {
                    info!("Client {} reader error: {e}", self.addr);
                    return Err(WorldSocketError::Io(e));
                }
            }

            let header = PacketHeader::read(&header_buf);
            if !header.is_valid_size() {
                info!("Reader[{}]: invalid header size={}", self.addr, header.size);
                return Err(WorldSocketError::InvalidSize(header.size));
            }

            // Read encrypted data
            let mut encrypted = vec![0u8; header.size as usize];
            self.reader.read_exact(&mut encrypted).await?;

            // Decrypt
            let data = match self.crypt.decrypt(&encrypted, &header.tag, &[]) {
                Ok(d) => d,
                Err(e) => {
                    info!("Reader[{}]: decrypt failed, size={}, counter={}: {e}",
                        self.addr, header.size, self.crypt.client_counter());
                    return Err(e.into());
                }
            };
            let pkt = WorldPacket::new_client(BytesMut::from(data.as_slice()));

            let opcode = pkt.opcode_raw();
            {
                let dump_len = data.len().min(256);
                let hex: String = data[..dump_len].iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
                let suffix = if data.len() > 256 { "..." } else { "" };
                trace!("Reader[{}]: received opcode 0x{:04X}, size={}\nHEX: {}{suffix}", self.addr, opcode, data.len(), hex);
            }

            // Handle Ping inline
            if opcode == ClientOpcodes::Ping as u16 {
                let mut pkt = pkt;
                pkt.skip_opcode();
                if let Ok(ping) = Ping::read(&mut pkt) {
                    let pong = Pong { serial: ping.serial };
                    let pong_bytes = pong.to_bytes();
                    if self.pong_tx.send(pong_bytes).is_err() {
                        return Err(WorldSocketError::Closed);
                    }
                }
                continue;
            }

            // Forward to session
            if self.session_tx.send(pkt).is_err() {
                warn!("Session channel closed for {}", self.addr);
                return Err(WorldSocketError::Closed);
            }
        }
    }
}

/// Write half of a split WorldSocket — encrypts and sends outbound packets.
pub struct SocketWriter {
    writer: tokio::net::tcp::OwnedWriteHalf,
    crypt: WorldCrypt,
    send_rx: flume::Receiver<Vec<u8>>,
    addr: SocketAddr,
    compressor: compression::PacketCompressor,
}

impl SocketWriter {
    /// Run the write loop: receive serialized packets from session, encrypt, write to TCP.
    ///
    /// Returns `Ok(())` when all senders are dropped (clean shutdown).
    pub async fn run(mut self) -> Result<(), WorldSocketError> {
        loop {
            let data = match self.send_rx.recv_async().await {
                Ok(d) => d,
                Err(_) => {
                    debug!("All senders dropped for {}, writer exiting", self.addr);
                    return Ok(());
                }
            };

            self.write_encrypted(&data).await?;
        }
    }

    /// Encrypt a serialized packet and write it to the TCP stream.
    async fn write_encrypted(&mut self, data: &[u8]) -> Result<(), WorldSocketError> {
        let mut data = data.to_vec();

        // Log the opcode being sent
        let opcode_raw = if data.len() >= 2 {
            u16::from_le_bytes([data[0], data[1]])
        } else {
            0
        };
        // Log every packet with hex dump for debugging (truncate at 512 bytes).
        {
            let dump_len = data.len().min(512);
            let hex: String = data[..dump_len].iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
            let suffix = if data.len() > 512 { "..." } else { "" };
            trace!(
                "Writer[{}]: PKT#{} opcode=0x{:04X} len={}\nHEX: {}{suffix}",
                self.addr, self.crypt.server_counter(), opcode_raw, data.len(), hex
            );
        }

        // Compress if above threshold (uses persistent compressor per socket)
        if data.len() > compression::COMPRESSION_THRESHOLD {
            let original_len = data.len();
            let opcode_bytes = opcode_raw.to_le_bytes();
            let compressed = self.compressor.compress_packet(&opcode_bytes, &data[2..]);

            let comp_opcode = ServerOpcodes::CompressedPacket
                .to_u16()
                .unwrap_or(0);
            data = Vec::with_capacity(2 + compressed.len());
            data.extend_from_slice(&comp_opcode.to_le_bytes());
            data.extend_from_slice(&compressed);

            info!(
                "Writer[{}]: Compressed 0x{:04X} {} → {} bytes (CompressedPacket 0x{:04X})",
                self.addr, opcode_raw, original_len, data.len(), comp_opcode
            );
        }

        // Encrypt
        let (encrypted, tag) = self.crypt.encrypt(&data, &[])?;

        let header = PacketHeader::new(encrypted.len() as i32, tag);
        let header_bytes = header.to_bytes();

        debug!(
            "Writer[{}]: header size={}, tag={}, encrypted {} bytes",
            self.addr,
            header.size,
            tag.iter().map(|b| format!("{b:02X}")).collect::<String>(),
            encrypted.len()
        );

        self.writer.write_all(&header_bytes).await?;
        self.writer.write_all(&encrypted).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

// ── Account lookup trait ─────────────────────────────────────────

/// Trait for looking up account information during authentication.
///
/// Implementations should query the login database for the account associated
/// with the given realm join ticket (login ticket).
pub trait AccountLookup: Send + Sync {
    fn lookup_account(
        &self,
        realm_join_ticket: &str,
    ) -> Pin<Box<dyn Future<Output = Option<AccountInfo>> + Send + '_>>;
}

// ── Helper functions ─────────────────────────────────────────────

/// Sign the EnterEncryptedMode packet using Ed25519ctx (RFC 8032, phflag=0).
///
/// C# uses Ed25519 with `phflag=0` and `ctx=EnableEncryptionContext`, which
/// is **Ed25519ctx** — a contextualized variant that produces a completely
/// different signature from standard Ed25519.
pub fn sign_enable_encryption(encrypt_key: &[u8; 16], enabled: bool) -> [u8; 64] {
    // HMAC-SHA256(encrypt_key, [enabled_byte] || EnableEncryptionSeed)
    let mut hmac = HmacSha256::new(encrypt_key);
    hmac.update(&[u8::from(enabled)]);
    hmac.update(&ENABLE_ENCRYPTION_SEED);
    let to_sign = hmac.finalize();

    // Ed25519ctx sign (with EnableEncryptionContext as context)
    wow_crypto::ed25519ctx::sign_ed25519ctx(
        &ENTER_ENCRYPTED_MODE_PRIVATE_KEY,
        &to_sign,
        &ENABLE_ENCRYPTION_CONTEXT,
    )
}

/// Convert a hex string to bytes.
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_conversion() {
        let bytes = hex_to_bytes("DEADBEEF");
        assert_eq!(bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn hex_empty() {
        let bytes = hex_to_bytes("");
        assert!(bytes.is_empty());
    }

    #[test]
    fn sign_encryption_produces_64_bytes() {
        let key = [0x42u8; 16];
        let sig = sign_enable_encryption(&key, true);
        assert_eq!(sig.len(), 64);

        // Same inputs should produce same signature (Ed25519 is deterministic)
        let sig2 = sign_enable_encryption(&key, true);
        assert_eq!(sig, sig2);
    }

    #[test]
    fn sign_encryption_differs_with_enabled_flag() {
        let key = [0x42u8; 16];
        let sig_on = sign_enable_encryption(&key, true);
        let sig_off = sign_enable_encryption(&key, false);
        assert_ne!(sig_on, sig_off);
    }

    #[test]
    fn connection_init_strings() {
        assert_eq!(SERVER_CONNECTION_INIT.len(), 53);
        assert_eq!(CLIENT_CONNECTION_INIT.len(), 53);
        assert!(SERVER_CONNECTION_INIT.ends_with(b"\n"));
        assert!(CLIENT_CONNECTION_INIT.ends_with(b"\n"));
    }

    #[test]
    fn seeds_are_16_bytes() {
        assert_eq!(AUTH_CHECK_SEED.len(), 16);
        assert_eq!(SESSION_KEY_SEED.len(), 16);
        assert_eq!(CONTINUED_SESSION_SEED.len(), 16);
        assert_eq!(ENCRYPTION_KEY_SEED.len(), 16);
        assert_eq!(ENABLE_ENCRYPTION_SEED.len(), 16);
        assert_eq!(ENABLE_ENCRYPTION_CONTEXT.len(), 16);
    }

    #[test]
    fn private_key_is_32_bytes() {
        assert_eq!(ENTER_ENCRYPTED_MODE_PRIVATE_KEY.len(), 32);
    }
}
