// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player lifecycle persistence capability.
//!
//! This crate owns *what* the Player lifecycle needs to persist and *how the
//! result is classified*. It owns no pool, row, transaction, statement or SQL
//! string, and has no dependencies at all — the MariaDB/SQLx adapter lives in
//! `wow-database`, which remains the only concrete owner of those.
//!
//! It exists because production uses it: `wow_world::session::lifecycle`
//! publishes offline state through this port rather than reaching for a
//! database handle. Issue #200 grows the same seam to cover the character save
//! and the account collections; the frozen order those must preserve is
//! `docs/migration/player-lifecycle-persistence-contract.md` (#187).

use std::future::Future;
use std::pin::Pin;

/// A future returned by a port method.
pub type PersistenceFutureLikeCpp<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Which offline state the lifecycle is publishing.
///
/// C++ `WorldSession::LogoutPlayer` marks the character offline and every
/// character on the account offline, and `WorldSession::~WorldSession` marks
/// the account itself offline. They are three distinct writes against two
/// logical databases, so they stay three distinct requests rather than one
/// "go offline" call that would hide which of them ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerOfflineMarkLikeCpp {
    /// The selected character, by GUID counter. Characters database.
    Character { guid_low: u32 },
    /// Every character on the account: one account has one online character.
    /// Characters database.
    CharacterAccount { account_id: u32 },
    /// The account itself, when the session is destroyed. Login database.
    LoginAccount { account_id: u32 },
}

impl PlayerOfflineMarkLikeCpp {
    /// Which logical database carries this write. Named here so callers and
    /// the persistence inventory agree without inspecting the adapter.
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        match self {
            Self::Character { .. } | Self::CharacterAccount { .. } => {
                LogicalDatabaseLikeCpp::Characters
            }
            Self::LoginAccount { .. } => LogicalDatabaseLikeCpp::Login,
        }
    }
}

/// The logical databases the lifecycle can address. Deliberately not a
/// connection, pool or URL — only which store a request belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalDatabaseLikeCpp {
    Characters,
    Login,
}

/// The normalized result of one lifecycle write.
///
/// `Unknown` is not a failure and not a success. The frozen contract requires
/// that an indeterminate outcome fences further mutation instead of being
/// collapsed into either, so it stays a distinct variant here rather than
/// being flattened into `Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceOutcomeLikeCpp {
    /// The write is durable. `rows` is what the adapter reported.
    Applied { rows: u64 },
    /// The write definitely did not apply; runtime state is unchanged.
    Failed { reason: String },
    /// The outcome could not be determined. The caller must fence.
    Unknown { reason: String },
}

impl PersistenceOutcomeLikeCpp {
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }

    /// True when the caller may not assume either outcome and must fence.
    pub fn is_indeterminate(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

/// One account's tutorial flags, ready to persist.
///
/// C++ `WorldSession::SaveTutorialsData` writes the eight flags in a single
/// statement, choosing INSERT or UPDATE by whether a row already exists. That
/// choice is data, not SQL, so it crosses the port as `already_persisted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTutorialsSaveLikeCpp {
    pub account_id: u32,
    /// The flags in index order; the adapter binds them positionally.
    pub tutorials: Vec<u32>,
    /// False on the first save for this account, which needs an INSERT.
    pub already_persisted: bool,
}

/// One row of an account-wide collection, ready to persist.
///
/// These are Battle.net account collections, not character state: C++ writes
/// them to the Login database during logout, each collection in its own
/// transaction. The five-transaction shape is preserved deliberately — #187
/// records that C++ appends them to one transaction and Rust does not, and
/// changing that is a behaviour fix with its own evidence, not something to
/// fold into an architecture move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionSaveLikeCpp {
    Mounts(Vec<AccountMountRowLikeCpp>),
    Toys(Vec<AccountToyRowLikeCpp>),
    Heirlooms(Vec<AccountHeirloomRowLikeCpp>),
}

impl AccountCollectionSaveLikeCpp {
    /// True when there is nothing to write. The caller skips the transaction
    /// rather than opening an empty one.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Mounts(rows) => rows.is_empty(),
            Self::Toys(rows) => rows.is_empty(),
            Self::Heirlooms(rows) => rows.is_empty(),
        }
    }

    /// Account collections live in the Login database.
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Login
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMountRowLikeCpp {
    pub bnet_account_id: u32,
    pub mount_spell_id: u32,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToyRowLikeCpp {
    pub bnet_account_id: u32,
    pub item_id: u32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloomRowLikeCpp {
    pub bnet_account_id: u32,
    pub item_id: u32,
    pub flags: u32,
}

/// The lifecycle capability the Session depends on.
///
/// The Session holds this, not a database handle. Anything the Session needs
/// to persist during login/logout arrives here as data, and comes back as a
/// classified outcome.
pub trait PlayerLifecyclePortLikeCpp: Send + Sync {
    /// Publish one offline mark. Never panics and never surfaces a driver
    /// error type: the outcome is the contract.
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist the account's tutorial flags in their own transaction, as C++
    /// `SaveTutorialsData` does. Separate from the character save on purpose:
    /// it runs on its own, not as part of that transaction.
    fn save_tutorials_like_cpp<'a>(
        &'a self,
        save: PlayerTutorialsSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist one account-wide collection in its own Login-database
    /// transaction, as C++ does during logout.
    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_offline_mark_names_its_logical_database_like_cpp() {
        assert_eq!(
            PlayerOfflineMarkLikeCpp::Character { guid_low: 1 }.logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
        assert_eq!(
            PlayerOfflineMarkLikeCpp::CharacterAccount { account_id: 1 }.logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
        assert_eq!(
            PlayerOfflineMarkLikeCpp::LoginAccount { account_id: 1 }.logical_database(),
            LogicalDatabaseLikeCpp::Login
        );
    }

    #[test]
    fn an_unknown_outcome_is_neither_applied_nor_a_plain_failure_like_cpp() {
        let unknown = PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT was sent".to_owned(),
        };
        assert!(!unknown.is_applied());
        assert!(unknown.is_indeterminate());

        let failed = PersistenceOutcomeLikeCpp::Failed {
            reason: "constraint violation".to_owned(),
        };
        assert!(!failed.is_applied());
        assert!(
            !failed.is_indeterminate(),
            "a definite rollback must not fence"
        );

        assert!(PersistenceOutcomeLikeCpp::Applied { rows: 1 }.is_applied());
    }
}
