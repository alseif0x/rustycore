//! Existing Session substate definitions; authority and field contracts are unchanged.

use super::*;
use crate::session::lifecycle;

/// The session's persistence and lifecycle timeline: the login/logout instants
/// and the periodic-save schedule, the player loading and logout claims, the
/// tutorial and account-data state it persists, the persistence ports and the
/// finalization rail it hands work to, and the pet-load and loot trackers.
pub(crate) struct SessionLifecycleState {
    /// C++ `WorldSession::_accountData`, represented in-memory until DB load/save is wired.
    pub(in crate::session) account_data_like_cpp: [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES],
    pub(in crate::session) battle_pet_account_attachment_like_cpp:
        Option<BattlePetAccountAttachmentLikeCpp>,
    pub(in crate::session) character_rename_callbacks: driver::RenameCallbacks,
    /// Detached durable loot grants and their post-commit runtime
    /// publications. This covers claimed world-owner items plus Item-owner
    /// items/money; Item owners have no map-owned loot authority.
    pub(in crate::session) durable_item_loot_persistence_like_cpp:
        DurableItemLootPersistenceTrackerLikeCpp,
    /// Per-character fence published to remote loot sources before they begin
    /// mutating this character's durable balance.
    pub(in crate::session) durable_loot_money_persistence_like_cpp:
        Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    pub(in crate::session) finalization: Option<crate::finalization::SessionFinalization>,
    pub(in crate::session) homebind_persistence_tx_like_cpp:
        Option<tokio::sync::mpsc::UnboundedSender<HomebindPersistenceJobLikeCpp>>,
    /// Time played at current level loaded from DB (seconds).
    pub(crate) level_played_time: u32,
    /// Timestamp set when the player enters the world (PlayerLogin).
    pub(crate) login_time: Option<Instant>,
    /// When set, the session is counting down to logout (20s timer).
    /// `None` means no logout is pending.
    pub(crate) logout_time: Option<Instant>,
    /// C++ `Player::m_nextSave` countdown in milliseconds; 0 disables autosave.
    pub(in crate::session) next_player_save_ms_like_cpp: u32,
    /// Set by the sync update loop when the autosave countdown expires.
    pub(in crate::session) pending_periodic_player_save_like_cpp: bool,
    /// Typed database capabilities live behind one indirection so adding a
    /// persistence workflow does not keep growing this already-large session;
    /// `wow-database` supplies the concrete adapters.
    pub(crate) persistence_ports_like_cpp: Box<SessionPersistencePortsLikeCpp>,
    /// Per-character asynchronous C++ `PetLoadQueryHolder` result lifetime.
    pub(in crate::session) pet_load_query_holder_rows_like_cpp:
        lifecycle::PetLoadQueryHolderRowsLikeCpp,
    /// GUID of the character being logged in (set during PlayerLogin).
    pub(in crate::session) player_loading: Option<ObjectGuid>,
    /// Strong identity for this session's process-wide live-character claim.
    pub(in crate::session) player_login_claim_like_cpp: Option<(ObjectGuid, Arc<()>)>,
    /// C++ `WorldSession::m_playerLogout`: true only while the logout routine is executing.
    pub(in crate::session) player_logout_like_cpp: bool,
    /// C++ `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` in milliseconds.
    pub(in crate::session) player_save_interval_ms_like_cpp: u32,
    /// Total played time loaded from DB (seconds).
    pub(crate) total_played_time: u32,
    pub(in crate::session) tutorials_changed_like_cpp: bool,
    /// C++ `WorldSession::_tutorials`, account-scoped tutorial completion flags.
    pub(in crate::session) tutorials_like_cpp: [u32; 8],
    pub(in crate::session) tutorials_loaded_coherently_like_cpp: bool,
    pub(in crate::session) tutorials_loaded_from_db_like_cpp: bool,
}
