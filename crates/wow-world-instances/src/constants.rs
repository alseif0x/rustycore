pub const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
pub const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
pub const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) const HOUR_SECS_LIKE_CPP: u64 = 60 * 60;
