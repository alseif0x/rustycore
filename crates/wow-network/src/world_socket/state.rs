//! World socket read/write loop state definitions, part 1 of 1.
//!
//! Separated from the world_socket.rs root under #658. Behaviour is preserved.

use super::*;

pub(super) const SERVER_CONNECTION_INIT: &[u8] =
    b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n";

pub(super) const CLIENT_CONNECTION_INIT: &[u8] =
    b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n";

pub(super) const AUTH_CHECK_SEED: [u8; 16] = [
    0xC5, 0xC6, 0x98, 0x95, 0x76, 0x3F, 0x1D, 0xCD, 0xB6, 0xA1, 0x37, 0x28, 0xB3, 0x12, 0xFF, 0x8A,
];

pub(super) const SESSION_KEY_SEED: [u8; 16] = [
    0x58, 0xCB, 0xCF, 0x40, 0xFE, 0x2E, 0xCE, 0xA6, 0x5A, 0x90, 0xB8, 0x01, 0x68, 0x6C, 0x28, 0x0B,
];

#[allow(dead_code)]
pub(super) const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2, 0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

pub(super) const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA, 0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];

pub(super) const ENABLE_ENCRYPTION_SEED: [u8; 16] = [
    0x90, 0x9C, 0xD0, 0x50, 0x5A, 0x2C, 0x14, 0xDD, 0x5C, 0x2C, 0xC0, 0x64, 0x14, 0xF3, 0xFE, 0xC9,
];

pub(super) const ENABLE_ENCRYPTION_CONTEXT: [u8; 16] = [
    0xA7, 0x1F, 0xB6, 0x9B, 0xC9, 0x7C, 0xDD, 0x96, 0xE9, 0xBB, 0xB8, 0x21, 0x39, 0x8D, 0x5A, 0xD4,
];

pub(super) const DEFAULT_MAX_OVERSPEED_PINGS_LIKE_CPP: u32 = 2;

pub(super) const OVERSPEED_PING_WINDOW_LIKE_CPP: Duration = Duration::from_secs(27);

pub(super) const REALM_CONNECTION_ID_LIKE_CPP: u32 = 0;

pub(super) const INSTANCE_CONNECTION_ID_LIKE_CPP: u32 = 1;

pub(super) static PACKET_DUMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

// Internal transport marker. A real server packet can never use NULL_OPCODE,
// so this byte sequence cannot collide with valid packet bytes.
pub(super) const WRITE_FENCE_PREFIX_LIKE_CPP: [u8; 8] = [0, 0, b'R', b'C', b'F', b'E', b'N', b'C'];

#[derive(Default)]
pub(super) struct SocketWriteFenceStateLikeCpp {
    pub(super) next_id: AtomicU64,
    pub(super) writer_closed: AtomicBool,
    pub(super) pending: Mutex<HashMap<u64, tokio::sync::oneshot::Sender<()>>>,
}

/// Cancellation guard for one pending marker acknowledgement.
///
/// A session handler can be dropped while `send_async` is waiting for channel
/// capacity. In that case no marker will ever reach the writer, so the pending
/// sender must be removed by `Drop` rather than by an async tail.
pub(super) struct PendingSocketWriteFenceLikeCpp {
    pub(super) state: Arc<SocketWriteFenceStateLikeCpp>,
    pub(super) id: u64,
}

impl Drop for PendingSocketWriteFenceLikeCpp {
    fn drop(&mut self) {
        self.state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.id);
    }
}

/// FIFO write fence for one physical world socket.
///
/// TrinityCore enqueues each `SendDirectMessage` during one session update,
/// while RustyCore has independent realm and instance writer tasks. A fence
/// marker is queued after a packet and acknowledged by that socket's writer
/// only after it has fully written every earlier packet. This lets a caller
/// retain observed cross-connection order without relying on scheduler timing
/// or `flume::Sender::is_empty()`.
#[derive(Clone, Default)]
pub struct SocketWriteFenceLikeCpp {
    pub(super) state: Arc<SocketWriteFenceStateLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketWriteFenceWaitResultLikeCpp {
    Written,
    TimedOut,
    WriterClosed,
}

impl SocketWriteFenceLikeCpp {
    pub(super) fn marker_like_cpp(id: u64) -> Vec<u8> {
        let mut marker = Vec::with_capacity(16);
        marker.extend_from_slice(&WRITE_FENCE_PREFIX_LIKE_CPP);
        marker.extend_from_slice(&id.to_le_bytes());
        marker
    }

    pub(super) fn marker_id_like_cpp(data: &[u8]) -> Option<u64> {
        if data.len() != 16 || data[..8] != WRITE_FENCE_PREFIX_LIKE_CPP {
            return None;
        }
        Some(u64::from_le_bytes(data[8..16].try_into().ok()?))
    }

    /// Acknowledge a marker consumed by a physical or test socket writer.
    ///
    /// Returns `true` when `data` is an internal fence marker and therefore
    /// must not be written as a world packet.
    pub fn acknowledge_marker_like_cpp(&self, data: &[u8]) -> bool {
        let Some(id) = Self::marker_id_like_cpp(data) else {
            return false;
        };
        let acknowledgement = self
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&id);
        if let Some(acknowledgement) = acknowledgement {
            let _ = acknowledgement.send(());
        }
        true
    }

    pub(super) fn close_writer_like_cpp(&self) {
        self.state.writer_closed.store(true, Ordering::Release);
        self.state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    /// Wait until this socket's writer has written every packet queued before
    /// the fence. A slow but live writer is allowed to apply normal socket
    /// backpressure up to the caller's configured socket-liveness bound.
    /// Callers must keep durable gameplay state committed for every outcome.
    pub async fn wait_for_prior_packets_written_like_cpp(
        &self,
        send_tx: &flume::Sender<Vec<u8>>,
        timeout: Duration,
    ) -> SocketWriteFenceWaitResultLikeCpp {
        let id = self.state.next_id.fetch_add(1, Ordering::Relaxed);
        let (acknowledgement_tx, acknowledgement_rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self
                .state
                .pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if self.state.writer_closed.load(Ordering::Acquire) {
                return SocketWriteFenceWaitResultLikeCpp::WriterClosed;
            }
            pending.insert(id, acknowledgement_tx);
        }
        let _pending = PendingSocketWriteFenceLikeCpp {
            state: self.state.clone(),
            id,
        };

        match tokio::time::timeout(timeout, async {
            send_tx
                .send_async(Self::marker_like_cpp(id))
                .await
                .map_err(|_| ())?;
            acknowledgement_rx.await.map_err(|_| ())
        })
        .await
        {
            Ok(Ok(())) => SocketWriteFenceWaitResultLikeCpp::Written,
            Ok(Err(())) => SocketWriteFenceWaitResultLikeCpp::WriterClosed,
            Err(_) => SocketWriteFenceWaitResultLikeCpp::TimedOut,
        }
    }
}

/// Ed25519 private key seed used for signing `EnterEncryptedMode`.
pub(super) const ENTER_ENCRYPTED_MODE_PRIVATE_KEY: [u8; 32] = [
    0x08, 0xBD, 0xC7, 0xA3, 0xCC, 0xC3, 0x4F, 0x3F, 0x6A, 0x0B, 0xFF, 0xCF, 0x31, 0xC1, 0xB6, 0x97,
    0x69, 0x1E, 0x72, 0x9A, 0x0A, 0xAB, 0x2C, 0x77, 0xC3, 0x6F, 0x8A, 0xE7, 0x5A, 0x9A, 0xA7, 0xC9,
];

pub(super) fn sanitize_packet_dump_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn dump_world_packet_like_cpp(
    direction: &str,
    connection_id: u32,
    addr: SocketAddr,
    counter: u64,
    opcode_raw: u16,
    opcode_name: &str,
    data: &[u8],
) {
    let Some(root) = std::env::var_os("RUSTYCORE_PACKET_DUMP_DIR") else {
        return;
    };

    let root = Path::new(&root);
    if let Err(err) = fs::create_dir_all(root) {
        warn!(
            "packet dump disabled: could not create {}: {err}",
            root.display()
        );
        return;
    }

    let seq = PACKET_DUMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let safe_name = sanitize_packet_dump_name(opcode_name);
    let stem = format!(
        "rust-{direction}-{seq:08}-counter{counter}-0x{opcode_raw:04X}-{safe_name}-len{}",
        data.len()
    );
    let bin_path = root.join(format!("{stem}.bin"));
    let meta_path = root.join(format!("{stem}.meta"));

    if let Err(err) = fs::write(&bin_path, data) {
        warn!("packet dump write failed for {}: {err}", bin_path.display());
        return;
    }

    let meta = format!(
        "direction={direction}\nconnection_id={connection_id}\naddr={addr}\nseq={seq}\ncounter={counter}\nopcode=0x{opcode_raw:04X}\nname={opcode_name}\nlen={}\n",
        data.len()
    );
    if let Err(err) = fs::write(&meta_path, meta) {
        warn!(
            "packet dump metadata write failed for {}: {err}",
            meta_path.display()
        );
    }
}

pub(super) fn trace_unencrypted_packet(
    direction: &str,
    connection_id: u32,
    addr: SocketAddr,
    counter: u64,
    opcode_raw: u16,
    opcode_name: &str,
    header: &[u8],
    data: &[u8],
) {
    if std::env::var_os("RUSTYCORE_PACKET_DUMP_DIR").is_some() {
        dump_world_packet_like_cpp(
            direction,
            connection_id,
            addr,
            counter,
            opcode_raw,
            opcode_name,
            data,
        );
    }

    if std::env::var_os("RUSTYCORE_HANDSHAKE_TRACE").is_some() {
        tracing::error!(
            "RUST_HANDSHAKE {direction} connection_id={connection_id} addr={addr} counter={counter} opcode=0x{opcode_raw:04X} name={opcode_name} header={:02X?} len={} payload={:02X?}",
            header,
            data.len(),
            data
        );
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SocketState {
    Uninitialized,
    ConnectionStringSent,
    AuthChallengeSent,
    AuthSessionReceived,
    EncryptedModeEnabled,
}

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
    /// C++ `WorldSession::IsARecruiter()`, derived from `LEFT JOIN account r`.
    pub is_a_recruiter: bool,
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

/// Per-client connection handler for the world server.
pub struct WorldSocket {
    pub(super) stream: TcpStream,
    pub(super) addr: SocketAddr,
    /// C++ `ConnectionType`: realm (`0`) or instance (`1`). Kept on the
    /// socket so pre-encryption and split-I/O packet dumps use the same route.
    pub(super) connection_id: u32,

    // Crypto
    pub(super) crypt: Option<WorldCrypt>,
    pub(super) server_challenge: [u8; 16],
    pub(super) encrypt_key: Option<[u8; 16]>,
    pub(super) session_key: Option<Vec<u8>>,

    // Session channel — sends deserialized packets to WorldSession
    pub(super) session_tx: Option<flume::Sender<WorldPacket>>,

    // Outbound channel — receives serialized bytes from WorldSession
    pub(super) send_rx: Option<flume::Receiver<Vec<u8>>>,
    // Sidecar state for internal FIFO markers acknowledged by SocketWriter.
    pub(super) send_write_fence_like_cpp: SocketWriteFenceLikeCpp,

    // Persistent compression stream for direct encrypted sends.
    pub(super) compressor: compression::PacketCompressor,

    // State
    pub(super) state: SocketState,

    // Account info (populated after AuthSession)
    pub(super) account_info: Option<AccountInfo>,

    // C++ sIPLocation equivalent for world auth country-lock checks.
    pub(super) ip_location_store_like_cpp: Option<Arc<IpLocationStore>>,

    // Tracks the number of packet headers sent before encryption is enabled.
    // The WoW client increments its receive counter for every header it reads,
    // even for unencrypted packets. The SocketWriter must start its server
    // counter at this offset to keep in sync with the client.
    pub(super) unencrypted_packets_sent: u64,

    // Tracks the number of packets received from the client before encryption.
    // The WoW client always increments its send counter in Encrypt(), even for
    // unencrypted packets (AuthSession, EnterEncryptedModeAck). The SocketReader
    // must start its client counter at this offset to decrypt correctly.
    pub(super) unencrypted_packets_received: u64,

    pub(super) max_overspeed_pings_like_cpp: u32,
    pub(super) overspeed_ping_tracker_like_cpp: OverspeedPingTrackerLikeCpp,
}

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
            crypt: WorldCrypt::new_with_client_counter(
                &encrypt_key,
                self.unencrypted_packets_received,
            ),
            session_tx,
            pong_tx,
            addr: self.addr,
            connection_id: self.connection_id,
            max_overspeed_pings_like_cpp: self.max_overspeed_pings_like_cpp,
            overspeed_ping_tracker_like_cpp: self.overspeed_ping_tracker_like_cpp,
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
            send_write_fence_like_cpp: self.send_write_fence_like_cpp,
            addr: self.addr,
            connection_id: self.connection_id,
            // C++ owns one z_stream for the complete physical WorldSocket
            // lifetime. Moving the existing stream preserves any direct-send
            // history accumulated before the async read/write ownership split.
            compressor: self.compressor,
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
    pub(super) reader: tokio::net::tcp::OwnedReadHalf,
    pub(super) crypt: WorldCrypt,
    pub(super) session_tx: flume::Sender<WorldPacket>,
    /// Sends serialized Pong packets to the write loop (bypasses session).
    pub(super) pong_tx: flume::Sender<Vec<u8>>,
    pub(super) addr: SocketAddr,
    pub(super) connection_id: u32,
    pub(super) max_overspeed_pings_like_cpp: u32,
    pub(super) overspeed_ping_tracker_like_cpp: OverspeedPingTrackerLikeCpp,
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
                    info!(
                        "Reader[{}]: decrypt failed, size={}, counter={}: {e}",
                        self.addr,
                        header.size,
                        self.crypt.client_counter()
                    );
                    return Err(e.into());
                }
            };
            let pkt = WorldPacket::new_client(BytesMut::from(data.as_slice()));

            let opcode = pkt.opcode_raw();
            let opcode_name = ClientOpcodes::from_u16(opcode)
                .map(|opcode| format!("{opcode:?}"))
                .unwrap_or_else(|| "Unknown".to_string());
            dump_world_packet_like_cpp(
                "c2s",
                self.connection_id,
                self.addr,
                self.crypt.client_counter(),
                opcode,
                &opcode_name,
                &data,
            );
            if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
                info!(
                    "RUST_PACKET_IN addr={} seq={} opcode=0x{:04X} name={} len={}",
                    self.addr,
                    self.crypt.client_counter(),
                    opcode,
                    opcode_name,
                    data.len()
                );
            }
            {
                let dump_len = data.len().min(256);
                let hex: String = data[..dump_len]
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let suffix = if data.len() > 256 { "..." } else { "" };
                trace!(
                    "Reader[{}]: received opcode 0x{:04X}, size={}\nHEX: {}{suffix}",
                    self.addr,
                    opcode,
                    data.len(),
                    hex
                );
            }

            // Handle Ping inline
            if opcode == ClientOpcodes::Ping as u16 {
                let mut pkt = pkt;
                pkt.skip_opcode();
                if let Ok(ping) = Ping::read(&mut pkt) {
                    if self
                        .overspeed_ping_tracker_like_cpp
                        .record_ping(Instant::now(), self.max_overspeed_pings_like_cpp)
                    {
                        warn!(
                            "WorldSocket::HandlePing: {} kicked for over-speed pings",
                            self.addr
                        );
                        return Err(WorldSocketError::Closed);
                    }
                    let pong = Pong {
                        serial: ping.serial,
                    };
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
    pub(super) writer: tokio::net::tcp::OwnedWriteHalf,
    pub(super) crypt: WorldCrypt,
    pub(super) send_rx: flume::Receiver<Vec<u8>>,
    pub(super) send_write_fence_like_cpp: SocketWriteFenceLikeCpp,
    pub(super) addr: SocketAddr,
    pub(super) connection_id: u32,
    pub(super) compressor: compression::PacketCompressor,
}

impl Drop for SocketWriter {
    fn drop(&mut self) {
        // Wake any cross-socket ordering wait immediately when this physical
        // writer ends. Transient backpressure must not masquerade as failure,
        // but a dead socket can never acknowledge an already queued marker.
        self.send_write_fence_like_cpp.close_writer_like_cpp();
    }
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

            if self
                .send_write_fence_like_cpp
                .acknowledge_marker_like_cpp(&data)
            {
                continue;
            }

            self.write_encrypted(&data).await?;
        }
    }

    /// Encrypt a serialized packet and write it to the TCP stream.
    pub(super) async fn write_encrypted(&mut self, data: &[u8]) -> Result<(), WorldSocketError> {
        let mut data = data.to_vec();

        // Log the opcode being sent
        let opcode_raw = if data.len() >= 2 {
            u16::from_le_bytes([data[0], data[1]])
        } else {
            0
        };
        let opcode_name = ServerOpcodes::from_u16(opcode_raw)
            .map(|opcode| format!("{opcode:?}"))
            .unwrap_or_else(|| "Unknown".to_string());
        dump_world_packet_like_cpp(
            "s2c",
            self.connection_id,
            self.addr,
            self.crypt.server_counter(),
            opcode_raw,
            &opcode_name,
            &data,
        );
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
            info!(
                "RUST_PACKET_OUT addr={} seq={} opcode=0x{:04X} name={} len={}",
                self.addr,
                self.crypt.server_counter(),
                opcode_raw,
                opcode_name,
                data.len()
            );
        }
        // Log every packet with hex dump for debugging (truncate at 512 bytes).
        {
            let dump_len = data.len().min(512);
            let hex: String = data[..dump_len]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let suffix = if data.len() > 512 { "..." } else { "" };
            trace!(
                "Writer[{}]: PKT#{} opcode=0x{:04X} len={}\nHEX: {}{suffix}",
                self.addr,
                self.crypt.server_counter(),
                opcode_raw,
                data.len(),
                hex
            );
        }

        // C++ compares `WorldPacket::size()` here: payload length without opcode.
        if should_compress_server_packet_like_cpp(&data) {
            let original_len = data.len();
            let opcode_bytes = opcode_raw.to_le_bytes();
            let compressed = self.compressor.compress_packet(&opcode_bytes, &data[2..]);

            let comp_opcode = ServerOpcodes::CompressedPacket.to_u16().unwrap_or(0);
            data = Vec::with_capacity(2 + compressed.len());
            data.extend_from_slice(&comp_opcode.to_le_bytes());
            data.extend_from_slice(&compressed);

            info!(
                "Writer[{}]: Compressed 0x{:04X} {} → {} bytes (CompressedPacket 0x{:04X})",
                self.addr,
                opcode_raw,
                original_len,
                data.len(),
                comp_opcode
            );
            if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
                info!(
                    "RUST_PACKET_OUT_COMPRESSED addr={} seq={} original_opcode=0x{:04X} original_len={} wire_opcode=0x{:04X} wire_len={}",
                    self.addr,
                    self.crypt.server_counter(),
                    opcode_raw,
                    original_len,
                    comp_opcode,
                    data.len()
                );
            }
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

#[derive(Debug, Default)]
pub(super) struct OverspeedPingTrackerLikeCpp {
    pub(super) last_ping_time: Option<Instant>,
    pub(super) overspeed_pings: u32,
}

impl OverspeedPingTrackerLikeCpp {
    /// Returns true when TC's `WorldSocket::HandlePing` would close the socket.
    pub(super) fn record_ping(&mut self, now: Instant, max_allowed: u32) -> bool {
        match self.last_ping_time.replace(now) {
            None => false,
            Some(last_ping_time) => {
                let diff = now
                    .checked_duration_since(last_ping_time)
                    .unwrap_or(Duration::ZERO);

                if diff < OVERSPEED_PING_WINDOW_LIKE_CPP {
                    self.overspeed_pings = self.overspeed_pings.saturating_add(1);
                    max_allowed != 0 && self.overspeed_pings > max_allowed
                } else {
                    self.overspeed_pings = 0;
                    false
                }
            }
        }
    }
}

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
pub(super) fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0))
        .collect()
}

pub(super) fn account_ip_lock_rejects_like_cpp(
    is_locked_to_ip: bool,
    last_ip: &str,
    current_ip: &str,
) -> bool {
    is_locked_to_ip && last_ip != current_ip
}

pub(super) fn account_country_lock_rejects_like_cpp(lock_country: &str, ip_country: &str) -> bool {
    !lock_country.is_empty()
        && lock_country != "00"
        && !ip_country.is_empty()
        && lock_country != ip_country
}

pub(super) fn should_compress_server_packet_like_cpp(data: &[u8]) -> bool {
    let payload_len = data.len().saturating_sub(2);
    compression::should_compress_payload_len(payload_len)
}
