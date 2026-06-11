//! Shared application state.

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use wow_database::{DatabaseError, LoginDatabase, LoginStatements};

use crate::ip_location::IpLocationStore;
use crate::realm::RealmManager;

/// Shared application state accessible from all handlers.
pub struct AppState {
    /// Login database connection.
    pub login_db: LoginDatabase,
    /// IP location database loaded from `IPLocationFile`.
    pub ip_location: IpLocationStore,

    /// External address for clients (e.g., "example.com").
    pub external_address: String,
    /// Local/loopback address.
    pub local_address: String,
    /// REST API port.
    pub rest_port: u16,
    /// BNet RPC port.
    pub rpc_port: u16,
    /// Login ticket validity in seconds.
    pub ticket_duration: u64,
    /// Max failed login attempts before ban (0 = disabled).
    pub wrong_pass_max: u32,
    /// Ban duration in seconds after max failed attempts.
    pub wrong_pass_ban_time: u32,
    /// Ban type: 0 = IP, 1 = account.
    pub wrong_pass_ban_type: u32,
    /// Log wrong-password attempts.
    pub wrong_pass_logging: bool,

    /// REST login sessions (JSESSIONID → session state).
    pub rest_sessions: DashMap<String, RestSessionState>,

    /// Realm manager (initialized after construction).
    pub realm_mgr: RwLock<RealmManager>,
}

impl AppState {
    pub fn new(
        login_db: LoginDatabase,
        ip_location: IpLocationStore,
        external_address: String,
        local_address: String,
        rest_port: u16,
        rpc_port: u16,
        ticket_duration: u64,
        wrong_pass_max: u32,
        wrong_pass_ban_time: u32,
        wrong_pass_ban_type: u32,
        wrong_pass_logging: bool,
    ) -> Self {
        Self {
            login_db,
            ip_location,
            external_address,
            local_address,
            rest_port,
            rpc_port,
            ticket_duration,
            wrong_pass_max,
            wrong_pass_ban_time,
            wrong_pass_ban_type,
            wrong_pass_logging,
            rest_sessions: DashMap::new(),
            realm_mgr: RwLock::new(RealmManager::new()),
        }
    }

    /// Mirror TrinityCore bnetserver's pre-handshake `LOGIN_SEL_IP_INFO` check.
    ///
    /// C++ first purges expired IP bans, then closes the socket before TLS if
    /// any returned row has a non-zero `banned` column.
    pub async fn remote_ip_is_banned_like_cpp(
        &self,
        remote_ip: &str,
    ) -> Result<bool, DatabaseError> {
        let delete_expired = self.login_db.prepare(LoginStatements::DEL_EXPIRED_IP_BANS);
        if let Err(error) = self.login_db.execute(&delete_expired).await {
            tracing::warn!("Failed to delete expired IP bans before connection check: {error}");
        }

        let mut stmt = self.login_db.prepare(LoginStatements::SEL_IP_INFO);
        stmt.set_string(0, remote_ip);
        let mut result = self.login_db.query(&stmt).await?;

        if result.is_empty() {
            return Ok(false);
        }

        loop {
            if ip_ban_row_is_active_like_cpp(result.try_read::<u64>(0).unwrap_or(0)) {
                return Ok(true);
            }

            if !result.next_row() {
                break;
            }
        }

        Ok(false)
    }
}

fn ip_ban_row_is_active_like_cpp(banned: u64) -> bool {
    banned != 0
}

#[cfg(test)]
mod tests {
    use super::ip_ban_row_is_active_like_cpp;

    #[test]
    fn ip_ban_row_matches_cpp_nonzero_gate() {
        assert!(!ip_ban_row_is_active_like_cpp(0));
        assert!(ip_ban_row_is_active_like_cpp(1));
        assert!(ip_ban_row_is_active_like_cpp(u64::MAX));
    }
}

/// REST session state (per login session, tracked by cookie).
pub struct RestSessionState {
    /// BNet SRP6 state, if an SRP challenge is in progress.
    pub srp: Option<wow_crypto::BnetSrp6>,
    /// Account ID for ticket creation after SRP proof.
    pub account_id: u32,
}

/// BNet account info loaded from the database.
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub id: u32,
    pub login: String,
    pub is_locked_to_ip: bool,
    pub lock_country: String,
    pub last_ip: String,
    pub failed_logins: u32,
    pub is_banned: bool,
    pub is_permanently_banned: bool,
    pub game_accounts: HashMap<u32, GameAccountInfo>,
}

/// Game account info associated with a BNet account.
#[derive(Debug, Clone)]
pub struct GameAccountInfo {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub unban_date: u64,
    pub is_permanently_banned: bool,
    pub is_banned: bool,
    pub security_level: u8,
    /// Character counts per realm (realm_id → count).
    pub char_counts: HashMap<u32, u8>,
    /// Last played character per sub-region.
    pub last_played_chars: HashMap<String, LastPlayedCharInfo>,
}

/// Last played character info for a sub-region.
#[derive(Debug, Clone)]
pub struct LastPlayedCharInfo {
    pub realm_address: u32,
    pub character_name: String,
    pub character_guid: u64,
    pub last_played_time: u64,
}
