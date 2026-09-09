//! World socket read/write loop operations, part 1 of 1.
//!
//! The inherent `WorldSocket` impl is divided by responsibility under
//! #658; every method keeps its original body.

use super::*;

impl WorldSocket {
    /// Create a new socket wrapper for an accepted TCP connection.
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        let mut challenge = [0u8; 16];
        rand::thread_rng().fill(&mut challenge);

        Self {
            stream,
            addr,
            connection_id: REALM_CONNECTION_ID_LIKE_CPP,
            crypt: None,
            server_challenge: challenge,
            encrypt_key: None,
            session_key: None,
            session_tx: None,
            send_rx: None,
            send_write_fence_like_cpp: SocketWriteFenceLikeCpp::default(),
            compressor: compression::PacketCompressor::new(),
            state: SocketState::Uninitialized,
            account_info: None,
            ip_location_store_like_cpp: None,
            unencrypted_packets_sent: 0,
            unencrypted_packets_received: 0,
            max_overspeed_pings_like_cpp: DEFAULT_MAX_OVERSPEED_PINGS_LIKE_CPP,
            overspeed_ping_tracker_like_cpp: OverspeedPingTrackerLikeCpp::default(),
        }
    }
    /// Configure `MaxOverspeedPings`, matching TC's validated 0-or-2..infinity range.
    pub fn set_max_overspeed_pings_like_cpp(&mut self, max_overspeed_pings: u32) {
        self.max_overspeed_pings_like_cpp = max_overspeed_pings;
    }
    /// Mark this accepted socket as C++ `CONNECTION_TYPE_INSTANCE` before its
    /// handshake starts, so every packet in the capture has `ConnectionId=1`.
    pub fn mark_instance_connection_like_cpp(&mut self) {
        self.connection_id = INSTANCE_CONNECTION_ID_LIKE_CPP;
    }
    /// Configure the shared C++ `sIPLocation` equivalent used by world auth.
    pub fn set_ip_location_store_like_cpp(&mut self, store: Option<Arc<IpLocationStore>>) {
        self.ip_location_store_like_cpp = store;
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
    pub(super) async fn send_auth_challenge(&mut self) -> Result<(), WorldSocketError> {
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
                return Err(WorldSocketError::AuthFailed(format!(
                    "unsupported platform: {other}"
                )));
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
                key_data
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                platform_seed
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                digest_key_hash
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                auth_session
                    .local_challenge
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                self.server_challenge
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                AUTH_CHECK_SEED
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                server_digest
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
                auth_session
                    .digest
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
            );
            return Err(WorldSocketError::AuthFailed("HMAC digest mismatch".into()));
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

        self.check_account_ip_country_lock_like_cpp(account)?;

        self.session_key = Some(session_key);
        self.encrypt_key = Some(encrypt_key);
        self.account_info = Some(account.clone());
        self.state = SocketState::AuthSessionReceived;

        info!("Account {} authenticated from {}", account.id, self.addr);
        Ok(())
    }
    pub(super) fn check_account_ip_country_lock_like_cpp(
        &self,
        account: &AccountInfo,
    ) -> Result<(), WorldSocketError> {
        let address = self.addr.ip().to_string();

        if account_ip_lock_rejects_like_cpp(account.is_locked_to_ip, &account.last_ip, &address) {
            debug!(
                "WorldSocket::HandleAuthSession: Account IP differs. Original IP: {}, new IP: {}.",
                account.last_ip, address
            );
            return Err(WorldSocketError::AuthFailed(
                "risk account locked: ip mismatch".into(),
            ));
        }

        if account.is_locked_to_ip {
            return Ok(());
        }

        let Some(store) = &self.ip_location_store_like_cpp else {
            return Ok(());
        };
        let Some(ip_country) = store.country_for_ip_like_cpp(&address) else {
            return Ok(());
        };

        if account_country_lock_rejects_like_cpp(&account.lock_country, ip_country) {
            debug!(
                "WorldSocket::HandleAuthSession: Account country differs. Original country: {}, new country: {}.",
                account.lock_country, ip_country
            );
            return Err(WorldSocketError::AuthFailed(
                "risk account locked: country mismatch".into(),
            ));
        }

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
    /// Clone the write fence paired with this physical socket's send channel.
    pub fn send_write_fence_like_cpp(&self) -> SocketWriteFenceLikeCpp {
        self.send_write_fence_like_cpp.clone()
    }
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
        let opcode_raw = if data.len() >= 2 {
            u16::from_le_bytes([data[0], data[1]])
        } else {
            0
        };
        let opcode_name = ServerOpcodes::from_u16(opcode_raw)
            .map(|opcode| format!("{opcode:?}"))
            .unwrap_or_else(|| "UNKNOWN_SERVER_OPCODE".to_string());

        let header = PacketHeader::new(size, [0u8; TAG_SIZE]);
        let header_bytes = header.to_bytes();

        self.stream.write_all(&header_bytes).await?;
        self.stream.write_all(&data).await?;
        self.stream.flush().await?;

        self.unencrypted_packets_sent += 1;
        trace_unencrypted_packet(
            "s2c-unencrypted",
            self.connection_id,
            self.addr,
            self.unencrypted_packets_sent,
            opcode_raw,
            &opcode_name,
            &header_bytes,
            &data,
        );
        Ok(())
    }
    /// Send a server packet with encryption (after handshake).
    pub async fn send_packet(&mut self, pkt: &impl ServerPacket) -> Result<(), WorldSocketError> {
        if self.crypt.is_none() {
            return self.send_unencrypted_packet(pkt).await;
        }

        let mut data = pkt.to_bytes();
        let opcode_raw = if data.len() >= 2 {
            u16::from_le_bytes([data[0], data[1]])
        } else {
            0
        };

        // C++ compares `WorldPacket::size()` here: payload length without opcode.
        if should_compress_server_packet_like_cpp(&data) {
            let opcode_bytes = opcode_raw.to_le_bytes();
            let compressed = self.compressor.compress_packet(&opcode_bytes, &data[2..]);

            // Replace data with CompressedPacket opcode + compressed payload
            let comp_opcode = ServerOpcodes::CompressedPacket.to_u16().unwrap_or(0);
            data = Vec::with_capacity(2 + compressed.len());
            data.extend_from_slice(&comp_opcode.to_le_bytes());
            data.extend_from_slice(&compressed);
        }

        // Encrypt
        let crypt = self
            .crypt
            .as_mut()
            .ok_or_else(|| WorldSocketError::AuthFailed("encryption not enabled".into()))?;
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
        let opcode_raw = if data.len() >= 2 {
            u16::from_le_bytes([data[0], data[1]])
        } else {
            0
        };
        let opcode_name = ClientOpcodes::from_u16(opcode_raw)
            .map(|opcode| format!("{opcode:?}"))
            .unwrap_or_else(|| "UNKNOWN_CLIENT_OPCODE".to_string());
        trace_unencrypted_packet(
            "c2s-unencrypted",
            self.connection_id,
            self.addr,
            self.unencrypted_packets_received,
            opcode_raw,
            &opcode_name,
            &header_buf,
            &data,
        );

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
                tracing::warn!(
                    "Ticket lookup failed for: '{}'",
                    &auth_session.realm_join_ticket
                );
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
            if opcode == ClientOpcodes::LogDisconnect as u16 {
                let reason = if pkt.data().len() >= 6 {
                    u32::from_le_bytes([pkt.data()[2], pkt.data()[3], pkt.data()[4], pkt.data()[5]])
                } else {
                    0
                };
                warn!(
                    "Client {} sent CMSG_LOG_DISCONNECT while waiting for EnterEncryptedModeAck: reason={reason} len={}",
                    self.addr,
                    pkt.data().len()
                );
            }
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
    /// Returns `(packet_rx, send_tx, write_fence)` — the session reads packets
    /// from `packet_rx`, writes responses via `send_tx`, and can fence that
    /// physical writer when cross-connection C++ ordering requires it.
    pub fn create_session_channels(
        &mut self,
    ) -> (
        flume::Receiver<WorldPacket>,
        flume::Sender<Vec<u8>>,
        SocketWriteFenceLikeCpp,
    ) {
        let (pkt_tx, pkt_rx) = flume::bounded(256);
        let (send_tx, send_rx) = flume::bounded(256);

        self.session_tx = Some(pkt_tx);
        self.send_rx = Some(send_rx);

        (pkt_rx, send_tx, self.send_write_fence_like_cpp())
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
                debug!(
                    "No session channel; dropping packet 0x{opcode:04X} from {}",
                    self.addr
                );
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
