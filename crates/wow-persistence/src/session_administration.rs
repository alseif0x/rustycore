//! Authenticated-session admission-ban and support-report persistence capabilities.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

/// The two persistence targets supported by C++ `World::BanAccount` for the
/// packet-spoof admission path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketSpoofBanTargetLikeCpp {
    Account { account_id: u32 },
    Ip { address: String },
}

/// One semantic PacketSpoof ban write. Statement selection and transaction
/// construction remain private to the concrete adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketSpoofBanWriteRequestLikeCpp {
    pub target: PacketSpoofBanTargetLikeCpp,
    pub duration_secs: u32,
    pub author: String,
    pub reason: String,
}

impl PacketSpoofBanWriteRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Login
    }
}

/// The IP lookup is deliberately classified separately from the subsequent
/// ban write: current Rust behavior still attempts the ban when this lookup
/// fails, but has no affected sessions to kick afterward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketSpoofAffectedAccountsLoadOutcomeLikeCpp {
    Loaded(Vec<u32>),
    Failed { reason: String },
}

/// SQLx-free admission persistence capability for PacketSpoof bans.
pub trait PacketSpoofBanPersistencePortLikeCpp: Send + Sync {
    fn load_accounts_by_ip_like_cpp<'a>(
        &'a self,
        address: &'a str,
    ) -> PersistenceFutureLikeCpp<'a, PacketSpoofAffectedAccountsLoadOutcomeLikeCpp>;

    fn persist_packet_spoof_ban_like_cpp<'a>(
        &'a self,
        request: PacketSpoofBanWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;
}

/// One legacy `CMSG_BUG_REPORT` write. The packet's report-type bit is parsed
/// by gameplay but C++ persists only Text and DiagInfo in that order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportBugReportWriteRequestLikeCpp {
    pub text: String,
    pub diagnostic_info: String,
}

impl SupportBugReportWriteRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// SQLx-free persistence capability for the legacy bug-report opcode.
pub trait SupportBugReportPersistencePortLikeCpp: Send + Sync {
    fn persist_bug_report_like_cpp<'a>(
        &'a self,
        request: SupportBugReportWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;
}
