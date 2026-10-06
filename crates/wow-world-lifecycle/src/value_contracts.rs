use std::sync::Arc;
use wow_packet::packets::misc::NUM_ACCOUNT_DATA_TYPES;

/// C++ `WorldSession::_accountData[NUM_ACCOUNT_DATA_TYPES]` entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountDataLikeCpp {
    pub time: i64,
    pub data: String,
}

pub const ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP: u32 = 0x7FFF;
pub const GLOBAL_CACHE_MASK_LIKE_CPP: u32 = 0x2515;
pub const PER_CHARACTER_CACHE_MASK_LIKE_CPP: u32 = 0x5AEA;

pub fn default_account_data_like_cpp() -> [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES] {
    std::array::from_fn(|_| AccountDataLikeCpp::default())
}

pub struct HomebindPersistenceJobLikeCpp {
    pub port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    pub request: wow_persistence::PlayerHomebindPersistenceRequestLikeCpp,
    pub guid_counter: u64,
}

/// Evidence for C++ `RemoveAtLoginFlag(flags, persist=true)`.
///
/// Non-persistent at-login removals intentionally mutate only the represented
/// in-memory flag field and do not push this boundary record.
#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedAtLoginFlagRemovalLikeCpp {
    pub flags: u16,
    pub persist: bool,
    pub db_statement_unrepresented: bool,
}
