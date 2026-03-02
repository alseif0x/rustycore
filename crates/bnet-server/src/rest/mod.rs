//! REST API for BNet login — raw HTTP implementation.
//!
//! Uses raw HTTP over TLS to match C#'s SslStream behavior exactly.
//! Hyper/axum sends TLS CloseNotify after responses, which the WoW 3.4.3
//! client interprets as an error. This raw implementation keeps the TLS
//! connection open after writing the response, matching C#'s behavior.
//!
//! Endpoints:
//! - `GET  /bnetserver/login/`       — Login form definition
//! - `POST /bnetserver/login/`       — Authenticate with credentials
//! - `POST /bnetserver/login/srp/`   — SRP challenge request
//! - `GET  /bnetserver/gameAccounts/` — List game accounts
//! - `GET  /bnetserver/portal/`      — Get BNet RPC address
//! - `POST /bnetserver/refreshLoginTicket/` — Refresh login ticket

pub mod handlers;
pub mod types;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::state::AppState;

/// HTTP response to be written to the stream.
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: &'static str,
    /// Handler-specific headers (Set-Cookie, Content-Type, etc.).
    /// Content-Length and Connection are added automatically.
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

/// Handle a single REST (HTTPS) connection using raw HTTP.
///
/// After writing the response, the server keeps the TLS connection open
/// and waits for the client to close it. This matches C#'s SslStream
/// behavior where the stream stays open after WriteAsync() completes.
pub async fn handle_rest_connection<S>(
    stream: S,
    state: Arc<AppState>,
    addr: std::net::SocketAddr,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf_stream = BufReader::new(stream);

    loop {
        // Read the HTTP request
        let request = match read_http_request(&mut buf_stream).await {
            Some(req) => req,
            None => {
                tracing::debug!("REST: connection from {addr} closed by client");
                return;
            }
        };

        tracing::info!("REST {} {} (from {addr})", request.method, request.path);
        for (name, value) in &request.headers {
            tracing::debug!("  Header: {name}: {value}");
        }

        // Route to handler
        let response = handlers::route(
            &state,
            &request.method,
            &request.path,
            &request.headers,
            request.body.as_deref(),
        )
        .await;

        // Build raw HTTP response matching C# header format exactly:
        //   Content-Length, Connection: close, [handler headers], then body
        let body_len = response.body.len();
        let mut resp_text = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {body_len}\r\nConnection: close\r\n",
            response.status_code, response.status_text
        );
        for (name, value) in &response.headers {
            resp_text.push_str(&format!("{name}: {value}\r\n"));
        }
        resp_text.push_str("\r\n");
        resp_text.push_str(&response.body);

        tracing::info!(
            "REST {} {} -> {} ({body_len} bytes body)",
            request.method,
            request.path,
            response.status_code
        );
        tracing::debug!("REST response:\n{resp_text}");

        // Write complete response as a single write
        let writer = buf_stream.get_mut();
        if let Err(e) = writer.write_all(resp_text.as_bytes()).await {
            tracing::debug!("REST: write failed for {addr}: {e}");
            return;
        }
        if let Err(e) = writer.flush().await {
            tracing::debug!("REST: flush failed for {addr}: {e}");
            return;
        }

        // Do NOT close the TLS connection or call shutdown().
        // Wait for the client to close (next read returns EOF).
        // This matches C#'s SslStream behavior exactly.
    }
}

/// Parsed HTTP request.
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

/// Read a complete HTTP request (request line + headers + optional body).
async fn read_http_request<R>(reader: &mut BufReader<R>) -> Option<HttpRequest>
where
    R: AsyncRead + Unpin,
{
    // Read request line (e.g., "GET /bnetserver/login/ HTTP/1.1\r\n")
    let mut request_line = String::new();
    match reader.read_line(&mut request_line).await {
        Ok(0) => return None, // EOF — client closed connection
        Err(e) => {
            tracing::debug!("REST: read request line error: {e}");
            return None;
        }
        Ok(_) => {}
    }

    let request_line = request_line.trim().to_string();
    if request_line.is_empty() {
        return None;
    }

    // Parse "METHOD /path HTTP/1.1"
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        tracing::warn!("REST: malformed request line: {request_line}");
        return None;
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Read headers until empty line
    let mut headers = HashMap::new();
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => return None,
            Err(e) => {
                tracing::debug!("REST: read header error: {e}");
                return None;
            }
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break; // End of headers
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            let name_lower = name.trim().to_lowercase();
            let value_trimmed = value.trim().to_string();
            if name_lower == "content-length" {
                content_length = value_trimmed.parse().unwrap_or(0);
            }
            headers.insert(name_lower, value_trimmed);
        }
    }

    // Read body if Content-Length > 0
    let body = if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        match reader.read_exact(&mut buf).await {
            Ok(_) => Some(buf),
            Err(e) => {
                tracing::warn!("REST: failed to read {content_length}-byte body: {e}");
                return None;
            }
        }
    } else {
        None
    };

    Some(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}
