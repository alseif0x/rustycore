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

#[derive(Debug, thiserror::Error)]
pub enum ForeverSocketError {
    #[error("invalid build-70170 world protocol")]
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
    server_challenge: [u8; 32],
    pending: Option<AuthSession>,
    keys: Option<SessionKeys>,
    cipher: Option<ForeverWorldCrypt>,
    phase: Phase,
    server_counter: u64,
    client_counter: u64,
}

impl ForeverSocket {
    pub fn new(stream: TcpStream, realm_address: u32, region_group: i32) -> Self {
        let mut server_challenge = [0; 32];
        rand::thread_rng().fill_bytes(&mut server_challenge);
        Self {
            stream,
            realm_address,
            region_group,
            server_challenge,
            pending: None,
            keys: None,
            cipher: None,
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
        self.keys = Some(
            verify_and_derive(
                join_key,
                build_key,
                &request.local_challenge,
                &self.server_challenge,
                &request.digest,
            )
            .map_err(|_| ForeverSocketError::Proof)?,
        );
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
        let data = wire::frame_data(opcode, payload)?;
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
        if self.phase != Phase::Encrypted {
            return Err(ForeverSocketError::Phase);
        }
        self.phase = Phase::Failed;
        let (data, tag) = self.read_frame().await?;
        let plaintext = self
            .cipher
            .as_mut()
            .ok_or(ForeverSocketError::Phase)?
            .decrypt(&data, &tag)
            .map_err(|_| ForeverSocketError::Cipher)?;
        let frame = Frame::decode(plaintext)?;
        self.phase = Phase::Encrypted;
        Ok(frame)
    }

    async fn send_plain(&mut self, opcode: u32, payload: &[u8]) -> Result<(), ForeverSocketError> {
        let data = wire::frame_data(opcode, payload)?;
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
        Frame::decode(data)
    }

    async fn write_frame(&mut self, data: &[u8], tag: [u8; 12]) -> Result<(), ForeverSocketError> {
        let header = wire::header(data.len(), tag)?;
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
