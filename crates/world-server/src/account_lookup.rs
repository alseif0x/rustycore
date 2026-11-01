use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use wow_database::{LoginDatabase, LoginStatements};
use wow_network::world_socket::{AccountInfo, AccountLookup};

use super::locale_id_to_name;

// ── Account lookup implementation ────────────────────────────────

/// Looks up account information from the login database using the realm join ticket.
///
/// The realm join ticket sent by the client in AuthSession is actually the game
/// account username (e.g. "2#1"), NOT the BNet LoginTicket (TC-xxx). The C#
/// RustyCore WorldSocket.HandleAuthSession uses SEL_ACCOUNT_INFO_BY_NAME with
/// `WHERE a.username = ?` to look it up directly.
pub(super) struct DbAccountLookup {
    pub(super) login_db: Arc<LoginDatabase>,
    pub(super) realm_id: u16,
    pub(super) win64_auth_seed: [u8; 16],
}

impl AccountLookup for DbAccountLookup {
    fn lookup_account(
        &self,
        realm_join_ticket: &str,
    ) -> Pin<Box<dyn Future<Output = Option<AccountInfo>> + Send + '_>> {
        let ticket = realm_join_ticket.to_owned();
        let realm_id = self.realm_id;
        Box::pin(async move {
            // The realm_join_ticket is the game account username (e.g. "2#1").
            // Query SEL_ACCOUNT_INFO_BY_NAME: params are (RealmID, username).
            //
            // Columns returned:
            //  0: a.id                  (account_id)
            //  1: a.session_key_bnet    (64 raw bytes; hex-encoded below for auth helper)
            //  2: ba.last_ip
            //  3: ba.locked
            //  4: ba.lock_country
            //  5: a.expansion
            //  6: a.mutetime
            //  7: ba.locale
            //  8: a.recruiter
            //  9: a.os
            // 10: a.timezone_offset
            // 11: ba.id                 (battlenet_account_id)
            // 12: aa.SecurityLevel
            // 13: bab ban expr          (is_banned_bnet)
            // 14: ab ban expr           (is_banned_account)
            // 15: r.id                  (recruiter)
            let mut stmt = self
                .login_db
                .prepare(LoginStatements::SEL_ACCOUNT_INFO_BY_NAME);
            stmt.set_i32(0, i32::from(realm_id));
            stmt.set_string(1, &ticket);

            let result = match self.login_db.query(&stmt).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("DB error looking up account by name '{ticket}': {e}");
                    return None;
                }
            };

            if result.is_empty() {
                tracing::warn!("No account found for realm_join_ticket '{ticket}'");
                return None;
            }

            let account_id: u32 = result.read(0);
            // session_key_bnet is varbinary(64) — read as raw bytes, then hex-encode
            let session_key_raw: Vec<u8> = result.try_read(1).unwrap_or_default();
            let session_key_hex: String =
                session_key_raw.iter().map(|b| format!("{b:02X}")).collect();
            let last_ip: String = result.try_read(2).unwrap_or_default();
            let is_locked: u8 = result.try_read(3).unwrap_or(0);
            let lock_country: String = result.try_read(4).unwrap_or_default();
            let expansion: u8 = result.try_read(5).unwrap_or(2);
            let mutetime: i64 = result.try_read(6).unwrap_or(0);
            let locale_raw: String = result
                .try_read::<u8>(7)
                .map(|v| v.to_string())
                .unwrap_or_else(|| result.try_read::<String>(7).unwrap_or_default());
            let recruiter: u32 = result.try_read(8).unwrap_or(0);
            let os: String = result.try_read(9).unwrap_or_default();
            let timezone_offset: i16 = result.try_read(10).unwrap_or(0);
            let Some(bnet_id) = result.try_read::<u32>(11).filter(|id| *id != 0) else {
                tracing::warn!(
                    "Game account {account_id} has no valid Battle.net account link; rejecting world authentication"
                );
                return None;
            };
            let security: u8 = result.try_read(12).unwrap_or(0);
            let is_banned_bnet: u32 = result.try_read(13).unwrap_or(0);
            let is_banned_account: u32 = result.try_read(14).unwrap_or(0);
            let is_a_recruiter = result.try_read::<u32>(15).unwrap_or(0) != 0;

            if account_id == 0 {
                tracing::warn!("Account id is 0 for ticket '{ticket}'");
                return None;
            }

            if session_key_hex.is_empty() {
                tracing::warn!("No session key for account {account_id} (ticket '{ticket}')");
                return None;
            }

            let locale_name = locale_id_to_name(&locale_raw);
            tracing::info!(
                "Account lookup OK: id={account_id}, bnet_id={bnet_id}, os={os}, locale_raw='{locale_raw}', locale='{locale_name}'"
            );

            let mut rbac_stmt = self
                .login_db
                .prepare(LoginStatements::SEL_RBAC_ACCOUNT_PERMISSIONS);
            rbac_stmt.set_u32(0, account_id);
            rbac_stmt.set_i32(1, i32::from(realm_id));
            let account_permissions = match self.login_db.query(&rbac_stmt).await {
                Ok(mut result) => {
                    let mut permissions = Vec::new();
                    if !result.is_empty() {
                        loop {
                            let permission_id: u32 = result.read(0);
                            if matches!(permission_id, 1 | 194) {
                                let granted: bool = result.read(1);
                                permissions.push((permission_id, granted));
                            }
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    permissions
                }
                Err(e) => {
                    tracing::error!(
                        "DB error loading instant-logout RBAC permissions for account {account_id}, realm {realm_id}: {e}; using default security only"
                    );
                    Vec::new()
                }
            };
            let instant_logout_permission =
                wow_world_core::session::effective_instant_logout_permission_like_cpp(
                    security,
                    &account_permissions,
                );

            Some(AccountInfo {
                id: account_id,
                session_key_hex,
                last_ip,
                is_locked_to_ip: is_locked != 0,
                lock_country,
                expansion,
                mute_time: mutetime,
                locale: locale_name,
                recruiter,
                is_a_recruiter,
                os,
                timezone_offset: i32::from(timezone_offset),
                battlenet_account_id: bnet_id,
                security,
                instant_logout_permission,
                is_banned_bnet: is_banned_bnet != 0,
                is_banned_account: is_banned_account != 0,
                win64_auth_seed: self.win64_auth_seed,
                client_address: None,            // Set by accept loop after auth
                derived_session_key: Vec::new(), // Set by accept loop after auth
            })
        })
    }
}
