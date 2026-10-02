//! Cookie-bound SRP lifetime, independent of individual HTTP connections.
//!
//! TrinityCore 6ebe044c: LoginHttpSession::ObtainSessionState and
//! Http::SessionService::{FindAndRefreshSessionState,MarkSessionInactive}.
//! The listener owns the registry; a connection owns one lease. Only the
//! per-session async operation gate spans handler/DB awaits, never the registry
//! lock. The gate is released before network output. Cancellation drops the gate
//! and lease; completed challenge state survives failed output/reconnect.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::{Mutex as AsyncMutex, MutexGuard};

use super::handlers::RestConnectionState;

const INACTIVE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Default)]
pub struct RestSessions {
    entries: Arc<Mutex<HashMap<String, Entry>>>,
}

struct Entry {
    remote_ip: IpAddr,
    state: Arc<AsyncMutex<RestConnectionState>>,
    connections: usize,
    inactive_since: Option<Instant>,
}

pub(super) struct SessionLease {
    registry: RestSessions,
    id: String,
    state: Arc<AsyncMutex<RestConnectionState>>,
}

impl RestSessions {
    pub(super) fn acquire(&self, cookie: Option<&str>, remote_ip: IpAddr) -> (SessionLease, bool) {
        self.acquire_at(cookie, remote_ip, Instant::now())
    }

    fn acquire_at(
        &self,
        cookie: Option<&str>,
        remote_ip: IpAddr,
        now: Instant,
    ) -> (SessionLease, bool) {
        let mut entries = self.entries.lock();
        // Expired entries cannot be reacquired. Reclaim lazily on admission,
        // rather than adding a second background clock to this REST listener.
        entries.retain(|_, entry| {
            entry
                .inactive_since
                .is_none_or(|since| now.duration_since(since) <= INACTIVE_TTL)
        });
        let requested = cookie.and_then(session_cookie);
        if let Some(id) = requested
            && let Some(entry) = entries.get_mut(id)
            && entry.remote_ip == remote_ip
        {
            entry.connections += 1;
            entry.inactive_since = None;
            return (
                SessionLease {
                    registry: self.clone(),
                    id: id.to_owned(),
                    state: Arc::clone(&entry.state),
                },
                false,
            );
        }

        // Random 128-bit UUID representation, matching the C++ cookie format.
        // Never trust a client-provided unknown identifier as a new session ID.
        let id = loop {
            let value = rand::random::<u128>();
            let hex = format!("{value:032x}");
            let id = format!(
                "{}-{}-{}-{}-{}",
                &hex[..8],
                &hex[8..12],
                &hex[12..16],
                &hex[16..20],
                &hex[20..]
            );
            if value != 0 && !entries.contains_key(&id) {
                break id;
            }
        };
        let state = Arc::new(AsyncMutex::new(RestConnectionState::default()));
        entries.insert(
            id.clone(),
            Entry {
                remote_ip,
                state: Arc::clone(&state),
                connections: 1,
                inactive_since: None,
            },
        );
        (
            SessionLease {
                registry: self.clone(),
                id,
                state,
            },
            true,
        )
    }
}

impl SessionLease {
    pub(super) async fn lock(&self) -> MutexGuard<'_, RestConnectionState> {
        self.state.lock().await
    }

    pub(super) fn set_cookie(&self, host: &str) -> String {
        // Hostnames/IPv4 with optional port; IPv6 uses a host-only cookie.
        let domain = host.split(':').next().unwrap_or_default();
        let domain = if !domain.is_empty()
            && domain
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
        {
            format!(" Domain={domain};")
        } else {
            String::new()
        };
        format!(
            "JSESSIONID={}; Path=/bnetserver;{domain} Secure; HttpOnly; SameSite=None",
            self.id
        )
    }
}

impl Drop for SessionLease {
    fn drop(&mut self) {
        let mut entries = self.registry.entries.lock();
        if let Some(entry) = entries.get_mut(&self.id) {
            entry.connections -= 1;
            if entry.connections == 0 {
                entry.inactive_since = Some(Instant::now());
            }
        }
    }
}

fn session_cookie(header: &str) -> Option<&str> {
    header
        .split(';')
        .find_map(|part| part.trim().strip_prefix("JSESSIONID="))
}

#[cfg(test)]
#[path = "sessions_tests.rs"]
mod tests;
