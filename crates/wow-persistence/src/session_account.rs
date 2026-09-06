//! Session account-data and tutorial persistence contracts.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

/// Which Characters-database account-data table a session operation addresses.
/// The identity is semantic; statement selection remains adapter-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAccountDataScopeLikeCpp {
    Global { account_id: u32 },
    Character { guid_low: u64 },
}

impl SessionAccountDataScopeLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// One raw account-data row. `WorldSession` retains the C++ table/mask
/// validation and owns publication into its account-data cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAccountDataRowLikeCpp {
    pub data_type: u8,
    pub time: i64,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAccountDataLoadOutcomeLikeCpp {
    Loaded(Vec<SessionAccountDataRowLikeCpp>),
    Failed { reason: String },
}

/// The tutorial row is absent for a new account and present as exactly the
/// eight values stored by C++ `WorldSession::LoadTutorialsData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionTutorialsLoadOutcomeLikeCpp {
    Loaded(Option<[u32; 8]>),
    Failed { reason: String },
}

/// One C++ `SetAccountData` replacement request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAccountDataSaveLikeCpp {
    pub scope: SessionAccountDataScopeLikeCpp,
    pub data_type: u8,
    pub time: i64,
    pub data: String,
}

/// SQLx-free persistence capability for account state canonically owned by
/// the authenticated session rather than by the Player lifecycle.
pub trait SessionAccountStatePortLikeCpp: Send + Sync {
    fn load_account_data_like_cpp<'a>(
        &'a self,
        scope: SessionAccountDataScopeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SessionAccountDataLoadOutcomeLikeCpp>;

    fn load_tutorials_like_cpp<'a>(
        &'a self,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SessionTutorialsLoadOutcomeLikeCpp>;

    fn save_account_data_like_cpp<'a>(
        &'a self,
        save: SessionAccountDataSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;
}
