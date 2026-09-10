//! The existing Player lifecycle port and offline/homebind requests; timing and port breadth are unchanged.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionSaveLikeCpp, LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp,
    PersistenceOutcomeLikeCpp, PlayerBankSlotPurchaseRequestLikeCpp,
    PlayerBuybackClearRequestLikeCpp, PlayerCharacterBaseLoadOutcomeLikeCpp,
    PlayerCharacterBaseLoadRequestLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerCurrencySaveRequestLikeCpp,
    PlayerDurabilityRepairSaveLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
    PlayerLoginAdmissionLoadOutcomeLikeCpp, PlayerLoginAdmissionLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerLoginPetTalentResetOutcomeLikeCpp,
    PlayerLoginTransportLoadOutcomeLikeCpp, PlayerLoginTransportLoadRequestLikeCpp,
    PlayerMoneyTransactionOutcomeLikeCpp, PlayerMoneyTransactionRequestLikeCpp,
    PlayerMoneyWriteRequestLikeCpp, PlayerOnlineMarkRequestLikeCpp,
    PlayerRealmCharacterCountRefreshRequestLikeCpp, PlayerTalentResetPersistenceRequestLikeCpp,
    PlayerUncageItemStateLoadOutcomeLikeCpp, PlayerUncageItemStateRequestLikeCpp,
    PlayerXpPersistenceRequestLikeCpp,
};

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

/// One C++ Player homebind write against the Characters database.
///
/// The variants preserve the distinct `_LoadHomeBind` repair operations and
/// the live `SetHomebind` update. Live map/area values stay wide here because
/// C++ narrows them at the prepared-statement boundary, which belongs to the
/// concrete adapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerHomebindPersistenceRequestLikeCpp {
    DeleteInvalid {
        player_guid: u64,
    },
    InsertRepaired {
        player_guid: u64,
        map_id: u16,
        area_id: u16,
        x: f32,
        y: f32,
        z: f32,
        orientation: f32,
    },
    UpdateLive {
        player_guid: u64,
        map_id: u32,
        area_id: u32,
        x: f32,
        y: f32,
        z: f32,
        orientation: f32,
    },
}

impl PlayerHomebindPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
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

    /// Execute one non-transactional homebind write. C++ queues these writes
    /// on the Characters database; callers retain gameplay state/publication.
    fn persist_homebind_like_cpp<'a>(
        &'a self,
        request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Delete every represented buyback item in one Characters-database
    /// transaction. Runtime state remains owned and published by the Player
    /// lifecycle caller only after `Applied`.
    fn clear_buyback_like_cpp<'a>(
        &'a self,
        request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist absolute money and optional item-durability replacements in
    /// one ordered Characters transaction, then observe the durable money row
    /// if the COMMIT reply is lost.
    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp>;

    /// Persist the absolute money and bank-slot count selected by one bank
    /// purchase as a single checked Characters transaction.
    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        request: PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerMoneyTransactionOutcomeLikeCpp>;

    /// Inspect the durable owner and inventory link used by the recoverable
    /// uncage deletion. The concrete adapter owns statement identity, binds
    /// and row decoding; Session owns the pre/postcondition decisions.
    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        request: PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerUncageItemStateLoadOutcomeLikeCpp>;

    /// Execute one standalone non-transactional item-durability replacement.
    /// The caller retains item selection, runtime mutation and publication.
    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        repair: PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Execute one non-transactional absolute money write for the existing
    /// checked loot payout boundary.
    fn persist_money_write_like_cpp<'a>(
        &'a self,
        request: PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist one standalone C++ `_SaveCurrency` plan in a single ordered
    /// Characters transaction. Mixed inventory/currency workflows keep their
    /// wider transaction boundary and reuse the same typed rows separately.
    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        request: PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist one represented talent reset as an ordered Characters
    /// transaction. The adapter reconciles an ambiguous COMMIT with the exact
    /// before/after money marker and returns `Unknown` when it cannot prove it.
    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        request: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist the represented immediate XP/level row and, when changed, the
    /// online rest-state row in one ordered Characters transaction.
    fn persist_xp_like_cpp<'a>(
        &'a self,
        request: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Count this account's live characters in Characters, then publish the
    /// result for one realm in Login. These remain two independent database
    /// operations and do not claim a distributed transaction.
    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Load the raw World templates and then the Characters value overlay used
    /// by the represented Player-login world-state path.
    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp>;

    /// Load either all represented transport spawns or the one named spawn for
    /// Player login. Statement identity and row decoding remain in the adapter.
    fn load_login_transports_like_cpp<'a>(
        &'a self,
        request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp>;

    /// Load the core `characters` row consumed by `Player::LoadFromDB`.
    /// Gameplay validation, fallback values and publication remain in the
    /// Player lifecycle owner.
    fn load_character_base_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterBaseLoadOutcomeLikeCpp>;

    /// Load one account-wide collection from the Login database. The caller
    /// retains collection validation and represented-state publication.
    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp>;

    /// Load one early Player-login admission input. Location validation,
    /// fallback/kick policy and guild publication remain caller-owned.
    fn load_login_admission_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAdmissionLoadOutcomeLikeCpp>;

    /// Load one auxiliary Player-login input from the Characters database.
    /// Gameplay retains validation and publication into represented state.
    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp>;

    /// Apply one ordered batch of item repairs discovered by Player login.
    /// Statement expansion and the transaction boundary remain adapter-owned.
    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        request: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Execute the two independent C++ pet-talent reset writes in order.
    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        player_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp>;

    /// Publish the selected character's online bit at the caller's existing
    /// login sequencing point.
    fn mark_player_online_like_cpp<'a>(
        &'a self,
        request: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist one account-wide collection in its own Login transaction.
    /// This existing Rust boundary differs from C++ full save; see #187.
    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    /// Persist one semantic Player snapshot in one Characters-database
    /// transaction. No dirty state may be published until `Applied`.
    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp>;
}
