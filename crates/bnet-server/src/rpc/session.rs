//! BNet RPC session — binary protobuf framing over TLS.
//!
//! Protocol:
//! ```text
//! [2 bytes: header length (big-endian)]
//! [header: protobuf Header message]
//! [payload: protobuf message of header.size bytes]
//! ```

use anyhow::{Context, Result, bail};
use prost::Message;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use wow_proto::bgs::protocol::Header;
use wow_proto::{RESPONSE_SERVICE_ID, service_hash};

use crate::state::{AccountInfo, AppState};

/// A single BNet RPC session, generic over the stream type.
///
/// `S` is typically `TlsStream<TcpStream>` for production or `TcpStream` for testing.
pub struct RpcSession<S> {
    stream: S,
    addr: SocketAddr,
    state: Arc<AppState>,

    /// Whether authentication is complete.
    pub authed: bool,
    /// Account info (populated after VerifyWebCredentials).
    pub account_info: Option<AccountInfo>,
    /// Client locale.
    pub locale: String,
    /// Client OS/platform.
    pub os: String,
    /// Client build number.
    pub build: u32,
    /// Timezone offset.
    pub timezone_offset: i32,
    /// Client IP country.
    pub ip_country: String,
    /// Client secret (32 bytes) for realm list ticket.
    pub client_secret: Vec<u8>,
    /// Auto-incrementing token for server-initiated requests.
    request_token: u32,
    /// Pending response callbacks (token → handler).
    #[allow(clippy::type_complexity)]
    response_callbacks: HashMap<u32, Box<dyn FnOnce(&[u8]) + Send>>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> RpcSession<S> {
    pub fn new(stream: S, addr: SocketAddr, state: Arc<AppState>) -> Self {
        Self {
            stream,
            addr,
            state,
            authed: false,
            account_info: None,
            locale: String::new(),
            os: String::new(),
            build: 0,
            timezone_offset: 0,
            ip_country: String::new(),
            client_secret: Vec::new(),
            request_token: 0,
            response_callbacks: HashMap::new(),
        }
    }

    /// Main session loop — read and dispatch messages.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Read 2-byte header length (big-endian)
            let header_len = self.stream.read_u16().await
                .context("Failed to read header length")? as usize;

            if header_len == 0 || header_len > 65535 {
                bail!("Invalid header length: {header_len}");
            }

            // Read header bytes
            let mut header_buf = vec![0u8; header_len];
            self.stream.read_exact(&mut header_buf).await
                .context("Failed to read header")?;

            let header = Header::decode(header_buf.as_slice())
                .context("Failed to decode protobuf header")?;

            // Read payload
            let payload_size = header.size.unwrap_or(0) as usize;
            let mut payload = vec![0u8; payload_size];
            if payload_size > 0 {
                self.stream.read_exact(&mut payload).await
                    .context("Failed to read payload")?;
            }

            // Log every incoming message with hex
            let header_hex = header_buf.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("-");
            tracing::debug!(
                "[BNET-RECV] service_id={} service_hash=0x{:08X} method={} token={} \
                 HasSize={} Size={} HasStatus={} Status={} \
                 header_len={} header_hex={}",
                header.service_id,
                header.service_hash.unwrap_or(0),
                header.method_id.unwrap_or(0),
                header.token,
                header.size.is_some(), payload_size,
                header.status.is_some(), header.status.unwrap_or(0),
                header_len,
                header_hex,
            );

            // Dispatch
            if header.service_id == RESPONSE_SERVICE_ID {
                // Response to a server-initiated request
                tracing::debug!("RPC: client response for token {} (status={:?})", header.token, header.status);
                if let Some(callback) = self.response_callbacks.remove(&header.token) {
                    callback(&payload);
                }
            } else if header.service_hash.unwrap_or(0) != 0 {
                // Client request — dispatch to service handler
                self.dispatch_request(&header, &payload).await?;
            }
        }
    }

    /// Dispatch a client request to the appropriate service handler.
    async fn dispatch_request(&mut self, header: &Header, payload: &[u8]) -> Result<()> {
        let hash = header.service_hash.unwrap_or(0);
        let method = header.method_id.unwrap_or(0);
        let token = header.token;

        let result = match hash {
            service_hash::CONNECTION_SERVICE => {
                super::services::connection::handle(self, method, payload).await
            }
            service_hash::AUTHENTICATION_SERVICE => {
                super::services::authentication::handle(self, method, payload).await
            }
            service_hash::ACCOUNT_SERVICE => {
                super::services::account::handle(self, method, payload).await
            }
            service_hash::GAME_UTILITIES_SERVICE => {
                super::services::game_utilities::handle(self, method, payload).await
            }
            _ => {
                tracing::warn!("Unknown service hash {hash:#010x} method {method}");
                Err(anyhow::anyhow!("Unknown service"))
            }
        };

        match result {
            Ok(Some(response_bytes)) => {
                self.send_response(token, 0, &response_bytes).await?;
            }
            Ok(None) => {
                // C# sends an empty message body with Size=0 explicitly set
                self.send_response(token, 0, &[]).await?;
            }
            Err(e) => {
                tracing::debug!("Service {hash:#010x}:{method} error: {e}");
                self.send_response_status(token, 1).await?;
            }
        }
        Ok(())
    }

    /// Send a response with a payload (Size always set, matching C# SendResponse(token, message)).
    pub async fn send_response(&mut self, token: u32, status: u32, payload: &[u8]) -> Result<()> {
        let header = Header {
            service_id: RESPONSE_SERVICE_ID,
            token,
            status: if status > 0 { Some(status) } else { None },
            size: Some(payload.len() as u32),
            ..Default::default()
        };

        self.send_header_and_payload(&header, payload).await
    }

    /// Send a response with status only, no body (matching C# SendResponse(token, status)).
    pub async fn send_response_status(&mut self, token: u32, status: u32) -> Result<()> {
        let header = Header {
            service_id: RESPONSE_SERVICE_ID,
            token,
            status: if status > 0 { Some(status) } else { None },
            ..Default::default()
        };

        self.send_header_and_payload(&header, &[]).await
    }

    /// Send a server-initiated request to the client.
    pub async fn send_request(
        &mut self,
        service_hash: u32,
        method_id: u32,
        payload: &[u8],
    ) -> Result<()> {
        let token = self.request_token;
        self.request_token += 1;

        let header = Header {
            service_id: 0,
            service_hash: Some(service_hash),
            method_id: Some(method_id),
            token,
            size: if payload.is_empty() { None } else { Some(payload.len() as u32) },
            ..Default::default()
        };

        self.send_header_and_payload(&header, payload).await
    }

    /// Write header + payload to the stream.
    async fn send_header_and_payload(&mut self, header: &Header, payload: &[u8]) -> Result<()> {
        let header_bytes = header.encode_to_vec();
        let header_len = header_bytes.len() as u16;

        // Build complete frame for logging
        let mut frame = Vec::with_capacity(2 + header_bytes.len() + payload.len());
        frame.extend_from_slice(&header_len.to_be_bytes());
        frame.extend_from_slice(&header_bytes);
        frame.extend_from_slice(payload);

        let header_hex = header_bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("-");
        let frame_hex = frame.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("-");

        tracing::debug!(
            "[BNET-SEND] service_id={} service_hash=0x{:08X} method={} token={} \
             HasStatus={} Status={} HasSize={} Size={} \
             header_bytes={} payload_bytes={} total_frame={} \
             header_hex={} frame_hex={}",
            header.service_id,
            header.service_hash.unwrap_or(0),
            header.method_id.unwrap_or(0),
            header.token,
            header.status.is_some(), header.status.unwrap_or(0),
            header.size.is_some(), header.size.unwrap_or(0),
            header_bytes.len(),
            payload.len(),
            frame.len(),
            header_hex,
            frame_hex,
        );

        // Write complete frame
        self.stream.write_all(&frame).await?;
        self.stream.flush().await?;
        Ok(())
    }

    /// Get a reference to the shared app state.
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get the client address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}
