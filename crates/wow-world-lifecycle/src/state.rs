use std::sync::Arc;
use std::time::Instant;

use wow_core::ObjectGuid;
use wow_packet::packets::misc::NUM_ACCOUNT_DATA_TYPES;
use wow_world_core::battle_pet_account::BattlePetAccountAttachmentLikeCpp;
use wow_world_core::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp;

use crate::{
    AccountDataLikeCpp, DurableItemLootPersistenceTrackerLikeCpp, HomebindPersistenceJobLikeCpp,
    PetLoadQueryHolderRowsLikeCpp, RenameCallbacks, SessionFinalization,
    SessionPersistencePortsLikeCpp, default_account_data_like_cpp,
};

mod account;
mod battle_pet_login;
mod bootstrap;
mod cleanup;
mod collections;
mod corpses;
mod finalization;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod group;
mod homebind;
mod item_loot;
mod load;
mod login_claims;
mod login_load;
mod logout;
mod module_login;
mod money_persistence;
mod money_plans;
mod persistence_ports;
mod pet_load;
mod playtime;
mod rename_callbacks;
mod runtime;
mod save;
mod stored_item_loot;
mod stored_item_loot_contracts;
mod transfer;

pub use group::group_persistence_command_like_cpp;
pub use money_plans::{LootMoneyPersistenceErrorLikeCpp, RepresentedTalentResetStatePlanLikeCpp};
pub use stored_item_loot_contracts::{
    LootTemplateRow, LootTemplateTable, WrappedGiftLoad, WrappedGiftRow,
};

#[cfg(any(test, feature = "test-fixtures"))]
use crate::{LoadedPlayerFlagsTestFixtureLikeCpp, RepresentedAtLoginFlagRemovalLikeCpp};

pub const DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP: u32 = 15 * 60 * 1000;

/// The session's persistence and lifecycle timeline: the login/logout instants
/// and the periodic-save schedule, the player loading and logout claims, the
/// tutorial and account-data state it persists, the persistence ports and the
/// finalization rail it hands work to, and the pet-load and loot trackers.
pub struct SessionLifecycleState {
    /// C++ `WorldSession::_accountData`, represented in-memory until DB load/save is wired.
    pub(crate) account_data_like_cpp: [AccountDataLikeCpp; NUM_ACCOUNT_DATA_TYPES],
    pub(crate) battle_pet_account_attachment_like_cpp: Option<BattlePetAccountAttachmentLikeCpp>,
    pub(crate) character_rename_callbacks: RenameCallbacks,
    /// Detached durable loot grants and their post-commit runtime
    /// publications. This covers claimed world-owner items plus Item-owner
    /// items/money; Item owners have no map-owned loot authority.
    pub(crate) durable_item_loot_persistence_like_cpp: DurableItemLootPersistenceTrackerLikeCpp,
    /// Per-character fence published to remote loot sources before they begin
    /// mutating this character's durable balance.
    pub(crate) durable_loot_money_persistence_like_cpp:
        Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    pub(crate) finalization: Option<SessionFinalization>,
    pub(crate) homebind_persistence_tx_like_cpp:
        Option<tokio::sync::mpsc::UnboundedSender<HomebindPersistenceJobLikeCpp>>,
    /// Time played at current level loaded from DB (seconds).
    pub(crate) level_played_time: u32,
    /// Timestamp set when the player enters the world (PlayerLogin).
    pub(crate) login_time: Option<Instant>,
    /// When set, the session is counting down to logout (20s timer).
    /// `None` means no logout is pending.
    pub(crate) logout_time: Option<Instant>,
    /// C++ `Player::m_nextSave` countdown in milliseconds; 0 disables autosave.
    pub(crate) next_player_save_ms_like_cpp: u32,
    /// Set by the sync update loop when the autosave countdown expires.
    pub(crate) pending_periodic_player_save_like_cpp: bool,
    /// Typed database capabilities live behind one indirection so adding a
    /// persistence workflow does not keep growing this already-large session;
    /// `wow-database` supplies the concrete adapters.
    pub(crate) persistence_ports_like_cpp: Box<SessionPersistencePortsLikeCpp>,
    /// Per-character asynchronous C++ `PetLoadQueryHolder` result lifetime.
    pub(crate) pet_load_query_holder_rows_like_cpp: PetLoadQueryHolderRowsLikeCpp,
    /// GUID of the character being logged in (set during PlayerLogin).
    pub(crate) player_loading: Option<ObjectGuid>,
    /// Strong identity for this session's process-wide live-character claim.
    pub(crate) player_login_claim_like_cpp: Option<(ObjectGuid, Arc<()>)>,
    /// C++ `WorldSession::m_playerLogout`: true only while the logout routine is executing.
    pub(crate) player_logout_like_cpp: bool,
    /// C++ `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` in milliseconds.
    pub(crate) player_save_interval_ms_like_cpp: u32,
    /// Total played time loaded from DB (seconds).
    pub(crate) total_played_time: u32,
    pub(crate) tutorials_changed_like_cpp: bool,
    /// C++ `WorldSession::_tutorials`, account-scoped tutorial completion flags.
    pub(crate) tutorials_like_cpp: [u32; 8],
    pub(crate) tutorials_loaded_coherently_like_cpp: bool,
    pub(crate) tutorials_loaded_from_db_like_cpp: bool,
    /// Detached loaded Player flag values used only by persistence tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_flags_test_fixture_like_cpp: LoadedPlayerFlagsTestFixtureLikeCpp,
    /// C++ `Player::m_atLoginFlags`, represented for reset-on-login side effects.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_at_login_flags_like_cpp: u16,
    /// Represented persistent `RemoveAtLoginFlag` calls until direct character DB execution is live.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_at_login_flag_removals_like_cpp:
        Vec<RepresentedAtLoginFlagRemovalLikeCpp>,
    /// Explicit test seam for persistence-sensitive loot-money paths. Production
    /// never bypasses the character database.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) loot_money_persistence_test_result_like_cpp: Option<bool>,
    /// C++ `PlayerData::Customizations` loaded before the self CREATE and
    /// retained for non-owner visibility CREATE blocks.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) loaded_player_customizations_like_cpp:
        Box<Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>>,
}

impl SessionLifecycleState {
    pub fn new_like_cpp() -> Self {
        Self {
            account_data_like_cpp: default_account_data_like_cpp(),
            battle_pet_account_attachment_like_cpp: None,
            character_rename_callbacks: Default::default(),
            durable_item_loot_persistence_like_cpp:
                DurableItemLootPersistenceTrackerLikeCpp::default(),
            durable_loot_money_persistence_like_cpp: Arc::new(
                DurableLootMoneyPersistenceTrackerLikeCpp::default(),
            ),
            finalization: None,
            homebind_persistence_tx_like_cpp: None,
            level_played_time: 0,
            login_time: None,
            logout_time: None,
            next_player_save_ms_like_cpp: DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
            pending_periodic_player_save_like_cpp: false,
            persistence_ports_like_cpp: Box::default(),
            pet_load_query_holder_rows_like_cpp: PetLoadQueryHolderRowsLikeCpp::default(),
            player_loading: None,
            player_login_claim_like_cpp: None,
            player_logout_like_cpp: false,
            player_save_interval_ms_like_cpp: DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
            total_played_time: 0,
            tutorials_changed_like_cpp: false,
            tutorials_like_cpp: [0; 8],
            tutorials_loaded_coherently_like_cpp: false,
            tutorials_loaded_from_db_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_flags_test_fixture_like_cpp: LoadedPlayerFlagsTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_at_login_flags_like_cpp: 0,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_at_login_flag_removals_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            loot_money_persistence_test_result_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            loaded_player_customizations_like_cpp: Box::default(),
        }
    }
}
