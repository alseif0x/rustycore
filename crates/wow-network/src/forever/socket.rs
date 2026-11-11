//! One physical connection owns its pending request, keys and packet counters.
//! The caller performs account admission and confirms the 40-byte session-key
//! write before the signed encryption offer. No socket task accesses SQL.

use super::wire::{self, AuthSession, Frame};
use super::{AUTH_CHALLENGE, AUTH_SESSION, ENTER_ENCRYPTED_MODE, ENTER_ENCRYPTED_MODE_ACK};
use rand::RngCore;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use wow_crypto::forever::{ForeverWorldCrypt, SessionKeys, verify_and_derive};
use wow_packet::compression::{PacketCompressor, should_compress_payload_len};

#[derive(Debug, thiserror::Error)]
pub enum ForeverSocketError {
    #[error("invalid Forever world protocol")]
    Protocol,
    #[error("invalid world connection phase")]
    Phase,
    #[error("world authentication proof rejected")]
    Proof,
    #[error(
        "unexpected encryption acknowledgement: opcode=0x{opcode:06X}, payload_bytes={payload_bytes}"
    )]
    UnexpectedEncryptionAck { opcode: u32, payload_bytes: usize },
    #[error("world session persistence was not confirmed")]
    Persistence,
    #[error("world packet authentication failed")]
    Cipher,
    #[error("world transport timeout")]
    Timeout,
    #[error("world transport I/O failed")]
    Io,
}

#[derive(PartialEq, Eq)]
enum Phase {
    New,
    AwaitingCredentials,
    Verified,
    Encrypted,
    Failed,
}

pub struct ForeverSocket {
    stream: TcpStream,
    realm_address: u32,
    region_group: i32,
    shifted_opcodes: bool,
    server_challenge: [u8; 32],
    pending: Option<AuthSession>,
    keys: Option<SessionKeys>,
    cipher: Option<ForeverWorldCrypt>,
    compressor: PacketCompressor,
    phase: Phase,
    server_counter: u64,
    client_counter: u64,
}

impl ForeverSocket {
    pub fn new(stream: TcpStream, realm_address: u32, region_group: i32) -> Self {
        Self::new_for_build(stream, realm_address, region_group, super::BUILD)
    }

    pub fn new_for_build(
        stream: TcpStream,
        realm_address: u32,
        region_group: i32,
        build: u32,
    ) -> Self {
        let mut server_challenge = [0; 32];
        rand::thread_rng().fill_bytes(&mut server_challenge);
        Self {
            stream,
            realm_address,
            region_group,
            shifted_opcodes: build >= 70291,
            server_challenge,
            pending: None,
            keys: None,
            cipher: None,
            compressor: PacketCompressor::new(),
            phase: Phase::New,
            server_counter: 0,
            client_counter: 0,
        }
    }

    pub async fn start(&mut self) -> Result<(), ForeverSocketError> {
        if self.phase != Phase::New {
            return Err(ForeverSocketError::Phase);
        }
        // Mark failed until the complete transition succeeds; cancellation
        // therefore cannot leave a half-initialized socket reusable.
        self.phase = Phase::Failed;
        timeout(Duration::from_secs(10), async {
            self.stream
                .write_all(wire::SERVER_HELLO)
                .await
                .map_err(|_| ForeverSocketError::Io)?;
            let mut hello = vec![0; wire::CLIENT_HELLO.len()];
            self.stream
                .read_exact(&mut hello)
                .await
                .map_err(|_| ForeverSocketError::Io)?;
            if hello != wire::CLIENT_HELLO {
                return Err(ForeverSocketError::Protocol);
            }
            Ok::<_, ForeverSocketError>(())
        })
        .await
        .map_err(|_| ForeverSocketError::Timeout)??;
        let mut challenge = [0; 65];
        rand::thread_rng().fill_bytes(&mut challenge[..32]);
        challenge[32..64].copy_from_slice(&self.server_challenge);
        challenge[64] = 1;
        self.send_plain(AUTH_CHALLENGE, &challenge).await?;
        let frame = self.receive_plain().await?;
        if frame.opcode() != AUTH_SESSION {
            return Err(ForeverSocketError::Protocol);
        }
        self.pending = Some(AuthSession::decode(frame.payload())?);
        self.phase = Phase::AwaitingCredentials;
        Ok(())
    }

    /// Raw JSON is interpreted by composition's account adapter, never logged.
    pub fn join_ticket(&self) -> Result<&str, ForeverSocketError> {
        if self.phase != Phase::AwaitingCredentials {
            return Err(ForeverSocketError::Phase);
        }
        Ok(&self
            .pending
            .as_ref()
            .ok_or(ForeverSocketError::Phase)?
            .ticket)
    }

    /// Caller must already have admitted account/build/variant, bans and locks.
    pub fn verify_credentials(
        &mut self,
        join_key: &[u8; 64],
        build_key: &[u8; 16],
    ) -> Result<(), ForeverSocketError> {
        self.verify_credentials_inner(join_key, Some(build_key))
    }

    /// Explicit development path for a build whose auth key is unknown. This
    /// skips only the per-build digest after normal BNet ticket admission.
    pub fn verify_credentials_without_build_key(
        &mut self,
        join_key: &[u8; 64],
    ) -> Result<(), ForeverSocketError> {
        self.verify_credentials_inner(join_key, None)
    }

    fn verify_credentials_inner(
        &mut self,
        join_key: &[u8; 64],
        build_key: Option<&[u8; 16]>,
    ) -> Result<(), ForeverSocketError> {
        if self.phase != Phase::AwaitingCredentials {
            return Err(ForeverSocketError::Phase);
        }
        self.phase = Phase::Failed;
        let request = self.pending.as_ref().ok_or(ForeverSocketError::Phase)?;
        if request.region != self.realm_address >> 24
            || request.district != (self.realm_address >> 16) & 0xFF
            || request.realm != self.realm_address & 0xFFFF
        {
            return Err(ForeverSocketError::Protocol);
        }
        self.keys = Some(if let Some(build_key) = build_key {
            verify_and_derive(
                join_key,
                build_key,
                &request.local_challenge,
                &self.server_challenge,
                &request.digest,
            )
            .map_err(|_| ForeverSocketError::Proof)?
        } else {
            wow_crypto::forever::derive_without_build_key(
                join_key,
                &request.local_challenge,
                &self.server_challenge,
            )
        });
        self.phase = Phase::Verified;
        Ok(())
    }

    /// Session key for the existing continued-session statement; not a login ticket.
    pub fn session_key(&self) -> Result<&[u8; 40], ForeverSocketError> {
        if self.phase != Phase::Verified && self.phase != Phase::Encrypted {
            return Err(ForeverSocketError::Phase);
        }
        Ok(self
            .keys
            .as_ref()
            .ok_or(ForeverSocketError::Phase)?
            .session_key())
    }

    pub async fn complete_encryption(
        &mut self,
        persisted_rows: u64,
    ) -> Result<(), ForeverSocketError> {
        if self.phase != Phase::Verified {
            return Err(ForeverSocketError::Phase);
        }
        self.phase = Phase::Failed;
        if persisted_rows != 1 {
            return Err(ForeverSocketError::Persistence);
        }
        let key = self
            .keys
            .as_ref()
            .ok_or(ForeverSocketError::Phase)?
            .encryption_key();
        let mut offer = Vec::with_capacity(69);
        offer.extend_from_slice(&self.region_group.to_le_bytes());
        offer.extend_from_slice(&crate::world_socket::sign_enable_encryption_forever(key));
        offer.push(0x80);
        self.send_plain(ENTER_ENCRYPTED_MODE, &offer).await?;
        let acknowledgement = self.receive_plain().await?;
        if acknowledgement.opcode() != ENTER_ENCRYPTED_MODE_ACK
            || !acknowledgement.payload().is_empty()
        {
            let reason = acknowledgement
                .payload()
                .get(..4)
                .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("reason width")));
            eprintln!("Forever encryption acknowledgement rejected: reason={reason:?}");
            return Err(ForeverSocketError::UnexpectedEncryptionAck {
                opcode: acknowledgement.opcode(),
                payload_bytes: acknowledgement.payload().len(),
            });
        }
        let key = self
            .keys
            .as_ref()
            .ok_or(ForeverSocketError::Phase)?
            .encryption_key();
        self.cipher = Some(ForeverWorldCrypt::new(
            key,
            self.server_counter,
            self.client_counter,
        ));
        self.pending = None;
        self.phase = Phase::Encrypted;
        Ok(())
    }

    pub async fn send(&mut self, opcode: u32, payload: &[u8]) -> Result<(), ForeverSocketError> {
        if self.phase != Phase::Encrypted {
            return Err(ForeverSocketError::Phase);
        }
        self.phase = Phase::Failed;
        let wire_opcode = wire::server_opcode(opcode, self.shifted_opcodes);
        eprintln!(
            "Forever encrypted send: opcode=0x{opcode:06X}, wire_opcode=0x{wire_opcode:06X}, payload_bytes={}",
            payload.len()
        );
        let data = if !self.shifted_opcodes && should_compress_payload_len(payload.len()) {
            let compressed = self
                .compressor
                .compress_forever_packet(&wire_opcode.to_le_bytes(), payload);
            eprintln!(
                "Forever packet compressed: uncompressed_bytes={}, frame_bytes={}",
                payload.len() + 4,
                compressed.len() + 4
            );
            wire::frame_data(wire::COMPRESSED_PACKET, &compressed)?
        } else {
            wire::frame_data_outgoing(wire_opcode, payload)?
        };
        let (ciphertext, tag) = self
            .cipher
            .as_mut()
            .ok_or(ForeverSocketError::Phase)?
            .encrypt(&data)
            .map_err(|_| ForeverSocketError::Cipher)?;
        self.write_frame(&ciphertext, tag).await?;
        self.phase = Phase::Encrypted;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Frame, ForeverSocketError> {
        self.receive_bounded(None).await
    }

    /// Authenticated character-phase idle policy, separate from the bounded
    /// frame-completion deadline. The caller owns admission and its timeout.
    /// Cancellation is terminal even while waiting without consuming bytes.
    pub async fn receive_with_idle_timeout(
        &mut self,
        idle: Duration,
    ) -> Result<Frame, ForeverSocketError> {
        self.receive_bounded(Some(idle)).await
    }

    async fn receive_bounded(
        &mut self,
        idle: Option<Duration>,
    ) -> Result<Frame, ForeverSocketError> {
        if self.phase != Phase::Encrypted {
            return Err(ForeverSocketError::Phase);
        }
        self.phase = Phase::Failed;
        let read = async {
            if idle.is_some() {
                let mut byte = [0];
                let available = self
                    .stream
                    .peek(&mut byte)
                    .await
                    .map_err(|_| ForeverSocketError::Io)?;
                if available == 0 {
                    return Err(ForeverSocketError::Io);
                }
            }
            self.read_frame().await
        };
        // The session's remaining deadline also covers a partial frame. First
        // byte availability never renews that session deadline.
        let received = match idle {
            Some(idle) => timeout(idle, read)
                .await
                .map_err(|_| ForeverSocketError::Timeout)?,
            None => read.await,
        };
        let (data, tag) = received.map_err(|error| {
            eprintln!("Forever encrypted receive transport failed: {error}");
            error
        })?;
        eprintln!("Forever encrypted receive: frame_bytes={}", data.len());
        let plaintext = self
            .cipher
            .as_mut()
            .ok_or(ForeverSocketError::Phase)?
            .decrypt(&data, &tag)
            .map_err(|_| {
                eprintln!("Forever encrypted receive authentication failed");
                ForeverSocketError::Cipher
            })?;
        let plaintext_bytes = plaintext.len();
        let mut frame = Frame::decode(plaintext).map_err(|error| {
            eprintln!(
                "Forever decrypted frame rejected: plaintext_bytes={plaintext_bytes}, error={error}"
            );
            error
        })?;
        frame.normalize_client_opcode(self.shifted_opcodes);
        self.phase = Phase::Encrypted;
        Ok(frame)
    }

    async fn send_plain(&mut self, opcode: u32, payload: &[u8]) -> Result<(), ForeverSocketError> {
        let data = wire::frame_data(wire::server_opcode(opcode, self.shifted_opcodes), payload)?;
        self.write_frame(&data, [0; 12]).await?;
        self.server_counter += 1;
        Ok(())
    }

    async fn receive_plain(&mut self) -> Result<Frame, ForeverSocketError> {
        let (data, tag) = self.read_frame().await?;
        if tag != [0; 12] {
            return Err(ForeverSocketError::Protocol);
        }
        self.client_counter += 1;
        let mut frame = Frame::decode(data)?;
        frame.normalize_client_opcode(self.shifted_opcodes);
        Ok(frame)
    }

    async fn write_frame(&mut self, data: &[u8], tag: [u8; 12]) -> Result<(), ForeverSocketError> {
        let header = wire::header_outgoing(data.len(), tag)?;
        timeout(Duration::from_secs(10), async {
            self.stream
                .write_all(&header)
                .await
                .map_err(|_| ForeverSocketError::Io)?;
            self.stream
                .write_all(data)
                .await
                .map_err(|_| ForeverSocketError::Io)?;
            Ok::<_, ForeverSocketError>(())
        })
        .await
        .map_err(|_| ForeverSocketError::Timeout)?
    }

    async fn read_frame(&mut self) -> Result<(Vec<u8>, [u8; 12]), ForeverSocketError> {
        timeout(Duration::from_secs(30), async {
            let mut header = [0; wire::HEADER_SIZE];
            self.stream
                .read_exact(&mut header)
                .await
                .map_err(|_| ForeverSocketError::Io)?;
            let size = wire::frame_size(&header)?;
            let mut data = vec![0; size];
            self.stream
                .read_exact(&mut data)
                .await
                .map_err(|_| ForeverSocketError::Io)?;
            Ok((data, header[4..].try_into().expect("fixed header")))
        })
        .await
        .map_err(|_| ForeverSocketError::Timeout)?
    }
}
