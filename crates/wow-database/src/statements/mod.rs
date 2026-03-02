//! Database statement definitions.
//!
//! Each database has its own statement enum that maps variant → SQL string.
//! The [`StatementDef`] trait is implemented by each enum.

pub mod login;
pub mod world;
pub mod character;
pub mod hotfix;

pub use login::LoginStatements;
pub use world::WorldStatements;
pub use character::CharStatements;
pub use hotfix::HotfixStatements;

/// Trait implemented by database statement enums.
///
/// Each variant maps to a static SQL string via [`sql()`](Self::sql).
/// The type parameter on [`Database<S>`](crate::Database) ensures that only
/// the correct statement type can be used with each database connection.
pub trait StatementDef: Copy + Send + Sync + 'static {
    /// Get the SQL string for this statement variant.
    ///
    /// Returns an empty string for variants that have no registered SQL
    /// (should not happen in a correctly configured server).
    fn sql(self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_statements_have_sql() {
        // Verify key BNet statements have non-empty SQL
        assert!(!LoginStatements::SEL_REALMLIST.sql().is_empty());
        assert!(!LoginStatements::SEL_BNET_AUTHENTICATION.sql().is_empty());
        assert!(!LoginStatements::SEL_BNET_ACCOUNT_INFO.sql().is_empty());
        assert!(!LoginStatements::UPD_BNET_LAST_LOGIN_INFO.sql().is_empty());
        assert!(!LoginStatements::DEL_EXPIRED_IP_BANS.sql().is_empty());
        assert!(!LoginStatements::INS_ACCOUNT.sql().is_empty());
    }

    #[test]
    fn login_sql_contains_expected_tables() {
        assert!(LoginStatements::SEL_REALMLIST.sql().contains("realmlist"));
        assert!(LoginStatements::SEL_BNET_AUTHENTICATION.sql().contains("battlenet_accounts"));
        assert!(LoginStatements::INS_IP_BANNED.sql().contains("ip_banned"));
        assert!(LoginStatements::SEL_CHECK_PASSWORD.sql().contains("salt"));
    }

    #[test]
    fn login_unregistered_statement_is_empty() {
        // SEL_BNET_ACCOUNT_SALT_BY_ID has no SQL in C# source
        assert!(LoginStatements::SEL_BNET_ACCOUNT_SALT_BY_ID.sql().is_empty());
    }

    #[test]
    fn world_statements_have_sql() {
        assert!(!WorldStatements::DEL_LINKED_RESPAWN.sql().is_empty());
        assert!(!WorldStatements::SEL_CREATURE_TEMPLATE.sql().is_empty());
        assert!(!WorldStatements::SEL_COMMANDS.sql().is_empty());
        assert!(!WorldStatements::INS_CREATURE.sql().is_empty());
    }

    #[test]
    fn world_sql_contains_expected_tables() {
        assert!(WorldStatements::SEL_CREATURE_TEXT.sql().contains("creature_text"));
        assert!(WorldStatements::SEL_SMART_SCRIPTS.sql().contains("smart_scripts"));
        assert!(WorldStatements::INS_GAME_TELE.sql().contains("game_tele"));
    }

    #[test]
    fn world_unregistered_statement_is_empty() {
        // SEL_GAMEOBJECT_TARGET has no SQL in C# source
        assert!(WorldStatements::SEL_GAMEOBJECT_TARGET.sql().is_empty());
    }

    #[test]
    fn login_sql_has_correct_placeholders() {
        // SEL_IP_INFO has 1 placeholder
        let sql = LoginStatements::SEL_IP_INFO.sql();
        assert_eq!(sql.matches('?').count(), 1);

        // INS_IP_BANNED has 4 placeholders
        let sql = LoginStatements::INS_IP_BANNED.sql();
        assert_eq!(sql.matches('?').count(), 4);

        // INS_ACCOUNT has 7 placeholders
        let sql = LoginStatements::INS_ACCOUNT.sql();
        assert_eq!(sql.matches('?').count(), 7);
    }
}
