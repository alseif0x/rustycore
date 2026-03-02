//! Shared application state.

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use wow_database::LoginDatabase;

use crate::realm::RealmManager;

/// Shared application state accessible from all handlers.
pub struct AppState {
    /// Login database connection.
    pub login_db: LoginDatabase,

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

    /// REST login sessions (JSESSIONID → session state).
    pub rest_sessions: DashMap<String, RestSessionState>,

    /// Realm manager (initialized after construction).
    pub realm_mgr: RwLock<RealmManager>,
}

impl AppState {
    pub fn new(
        login_db: LoginDatabase,
        external_address: String,
        local_address: String,
        rest_port: u16,
        rpc_port: u16,
        ticket_duration: u64,
        wrong_pass_max: u32,
        wrong_pass_ban_time: u32,
        wrong_pass_ban_type: u32,
    ) -> Self {
        Self {
            login_db,
            external_address,
            local_address,
            rest_port,
            rpc_port,
            ticket_duration,
            wrong_pass_max,
            wrong_pass_ban_time,
            wrong_pass_ban_type,
            rest_sessions: DashMap::new(),
            realm_mgr: RwLock::new(RealmManager::new()),
        }
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
