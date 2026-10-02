//! REST API for BNet login — raw HTTP implementation.
//!
//! Raw HTTP over TLS, with modern cookie-bound login sessions and request
//! keep-alive semantics (TrinityCore 6ebe044c Http::DispatcherService).
//!
//! Endpoints:
//! - `GET  /bnetserver/login/`       — Login form definition
//! - `POST /bnetserver/login/`       — Authenticate with credentials
//! - `POST /bnetserver/login/srp/`   — SRP challenge request
//! - `GET  /bnetserver/gameAccounts/` — List game accounts
//! - `GET  /bnetserver/portal/`      — Get BNet RPC address
//! - `POST /bnetserver/refreshLoginTicket/` — Refresh login ticket

pub mod handlers;
mod sessions;
pub mod types;
pub use sessions::RestSessions;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Notify;

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

/// Tracks REST requests that have been read and are still producing/writing a response.
///
/// The raw REST connection may stay open waiting for the client after a response;
/// that idle wait is not counted as in-flight work, matching the shutdown goal of
/// letting active responses finish without waiting forever for keep-alive clients.
#[derive(Clone, Debug)]
pub struct RestDrain {
    inner: Arc<RestDrainInner>,
}

#[derive(Debug, Default)]
struct RestDrainInner {
    shutting_down: AtomicBool,
    in_flight: AtomicUsize,
    idle: Notify,
}

impl RestDrain {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RestDrainInner::default()),
        }
    }

    pub fn begin_shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::SeqCst);
        self.inner.idle.notify_waiters();
    }

    pub fn is_shutting_down(&self) -> bool {
        self.inner.shutting_down.load(Ordering::SeqCst)
    }

    pub fn in_flight(&self) -> usize {
        self.inner.in_flight.load(Ordering::SeqCst)
    }

    fn begin_request(&self) -> Option<RestRequestGuard> {
        if self.is_shutting_down() {
            return None;
        }

        self.inner.in_flight.fetch_add(1, Ordering::SeqCst);
        if self.is_shutting_down() {
            self.finish_request();
            return None;
        }

        Some(RestRequestGuard {
            drain: self.clone(),
        })
    }

    fn finish_request(&self) {
        if self.inner.in_flight.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.inner.idle.notify_waiters();
        }
    }

    pub async fn wait_for_idle(&self) {
        loop {
            if self.in_flight() == 0 {
                return;
            }

            self.inner.idle.notified().await;
        }
    }
}

impl Default for RestDrain {
    fn default() -> Self {
        Self::new()
    }
}

struct RestRequestGuard {
    drain: RestDrain,
}

impl Drop for RestRequestGuard {
    fn drop(&mut self) {
        self.drain.finish_request();
    }
}

/// Handle a single REST (HTTPS) connection using raw HTTP.
///
/// A BNet session lease survives for this connection; its SRP state remains
/// recoverable by cookie for five minutes after the last connection closes.
pub async fn handle_rest_connection<S>(
    stream: S,
    state: Arc<AppState>,
    addr: std::net::SocketAddr,
    drain: RestDrain,
    sessions: RestSessions,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf_stream = BufReader::new(stream);
    let mut bot_connection_state = handlers::RestConnectionState::default();
    let mut login_session: Option<sessions::SessionLease> = None;

    loop {
        if drain.is_shutting_down() {
            tracing::debug!("REST: closing idle connection from {addr} during shutdown");
            return;
        }

        // Read the HTTP request
        let request = match read_http_request(&mut buf_stream).await {
            Some(req) => req,
            None => {
                tracing::debug!("REST: connection from {addr} closed by client");
                return;
            }
        };
        let Some(_request_guard) = drain.begin_request() else {
            tracing::debug!("REST: refusing request from {addr} during shutdown");
            return;
        };

        tracing::info!("REST {} {} (from {addr})", request.method, request.path);
        let mut set_cookie = None;
        // Bot-only endpoints retain their separate connection-local contract.
        let mut response = if request.path.starts_with("/bnetserver/") {
            if login_session.is_none() {
                let (lease, created) =
                    sessions.acquire(request.headers.get("cookie").map(String::as_str), addr.ip());
                if created {
                    set_cookie = Some(
                        lease.set_cookie(
                            request
                                .headers
                                .get("host")
                                .map(String::as_str)
                                .unwrap_or_default(),
                        ),
                    );
                }
                login_session = Some(lease);
            }
            let mut session = login_session
                .as_ref()
                .expect("acquired login session")
                .lock()
                .await;
            handlers::route(
                &state,
                &request.method,
                &request.path,
                &request.headers,
                request.body.as_deref(),
                &mut session,
            )
            .await
        } else {
            handlers::route(
                &state,
                &request.method,
                &request.path,
                &request.headers,
                request.body.as_deref(),
                &mut bot_connection_state,
            )
            .await
        };
        if let Some(cookie) = set_cookie {
            response.headers.push(("Set-Cookie", cookie));
        }

        let body_len = response.body.len();
        let connection = if request.keep_alive {
            "keep-alive"
        } else {
            "close"
        };
        let mut resp_text = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {body_len}\r\nConnection: {connection}\r\n",
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
        // Bodies and headers carry passwords, proofs, cookies and login tickets.
        // Log only the metadata above, even with debug logging enabled.

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

        if !request.keep_alive {
            return;
        }
    }
}

/// Parsed HTTP request.
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    keep_alive: bool,
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
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1") {
        tracing::warn!("REST: malformed request line: {request_line}");
        return None;
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let http11 = parts[2] == "HTTP/1.1";

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

    let connection = headers
        .get("connection")
        .map(String::as_str)
        .unwrap_or_default();
    let tokens: Vec<_> = connection.split(',').map(str::trim).collect();
    let keep_alive = !tokens.iter().any(|s| s.eq_ignore_ascii_case("close"))
        && (http11 || tokens.iter().any(|s| s.eq_ignore_ascii_case("keep-alive")));
    Some(HttpRequest {
        method,
        path,
        headers,
        body,
        keep_alive,
    })
}

#[cfg(test)]
mod tests {
    use super::RestDrain;

    #[tokio::test]
    async fn http_connection_policy_matches_request_version_and_tokens() {
        for (version, connection, expected) in [
            ("HTTP/1.1", "", true),
            ("HTTP/1.1", "Connection: close\r\n", false),
            ("HTTP/1.1", "Connection: Keep-Alive, Close\r\n", false),
            ("HTTP/1.0", "", false),
            ("HTTP/1.0", "Connection: keep-alive\r\n", true),
        ] {
            let bytes =
                format!("GET /bnetserver/login/ {version}\r\nHost: localhost\r\n{connection}\r\n");
            let mut reader = super::BufReader::new(bytes.as_bytes());
            let request = super::read_http_request(&mut reader).await.unwrap();
            assert_eq!(request.keep_alive, expected);
        }
    }

    #[test]
    fn rest_drain_rejects_new_requests_after_shutdown_like_cpp_stop_accepting() {
        let drain = RestDrain::new();

        let guard = drain.begin_request().expect("request before shutdown");
        assert_eq!(drain.in_flight(), 1);

        drain.begin_shutdown();
        assert!(drain.begin_request().is_none());
        assert_eq!(drain.in_flight(), 1);

        drop(guard);
        assert_eq!(drain.in_flight(), 0);
    }

    #[tokio::test]
    async fn rest_drain_waits_until_active_request_finishes() {
        let drain = RestDrain::new();
        let guard = drain.begin_request().expect("request before shutdown");

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), drain.wait_for_idle())
                .await
                .is_err()
        );

        drop(guard);
        tokio::time::timeout(std::time::Duration::from_millis(10), drain.wait_for_idle())
            .await
            .expect("drain should complete");
    }
}
