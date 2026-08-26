// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player lifecycle and authenticated-session persistence capabilities.
//!
//! This crate owns *what* the Player lifecycle and authenticated session need
//! to persist and *how the result is classified*. It owns no pool, row,
//! transaction, statement or SQL string, and has no dependencies at all — the
//! MariaDB/SQLx adapters live in `wow-database`, which remains the only concrete
//! owner of those.
//!
//! It exists because production uses it: `wow_world::session::lifecycle`
//! publishes offline state, account collections and the semantic character-save
//! snapshot through one port, while `WorldSession` loads and saves its own
//! account state through another. Neither reaches for a database handle. The
//! frozen Player-lifecycle order is documented in
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

/// One account-collection read requested by the Player login lifecycle.
///
/// C++ prepares these Login-database reads in `AccountInfoQueryHolder` and
/// passes their rows to `CollectionMgr`. The request names the business
/// collection only; statement identity and row decoding remain adapter work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountCollectionLoadRequestLikeCpp {
    Mounts { bnet_account_id: u32 },
    Toys { bnet_account_id: u32 },
    Heirlooms { bnet_account_id: u32 },
    ItemAppearances { bnet_account_id: u32 },
    TransmogIllusions { bnet_account_id: u32 },
}

impl AccountCollectionLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Login
    }
}

/// Raw semantic rows returned by one account-collection read.
///
/// Signed identifiers deliberately stay signed here. Existing gameplay owns
/// the C++-faithful validation and must be able to distinguish malformed rows
/// rather than receiving a value fabricated by the database adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionLoadedLikeCpp {
    Mounts(Vec<AccountMountLoadRowLikeCpp>),
    Toys(Vec<AccountToyLoadRowLikeCpp>),
    Heirlooms(Vec<AccountHeirloomLoadRowLikeCpp>),
    ItemAppearances {
        appearance_blocks: AccountCollectionRowsLikeCpp<Vec<AccountMaskBlockLikeCpp>>,
        favorite_appearance_ids: AccountCollectionRowsLikeCpp<Vec<u32>>,
    },
    TransmogIllusions {
        illusion_blocks: Vec<AccountMaskBlockLikeCpp>,
    },
}

/// Result of one physical read inside a semantic collection load. Item
/// appearances use two independent C++ queries and preserve partial success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionRowsLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// A read has no indeterminate COMMIT state: it either produced typed rows or
/// failed. Callers preserve their existing fail-closed publication behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionLoadOutcomeLikeCpp {
    Loaded(AccountCollectionLoadedLikeCpp),
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMountLoadRowLikeCpp {
    pub mount_spell_id: i32,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToyLoadRowLikeCpp {
    pub item_id: i32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloomLoadRowLikeCpp {
    pub item_id: i32,
    pub flags: u32,
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
    /// Appearances are stored as packed masks per block, with the favourite
    /// list maintained by explicit inserts and deletes. Insert order before
    /// delete order is preserved: they share one transaction and a delete that
    /// overtook its insert would drop a favourite the client still shows.
    ItemAppearances {
        bnet_account_id: u32,
        appearance_blocks: Vec<AccountMaskBlockLikeCpp>,
        favorite_inserts: Vec<u32>,
        favorite_deletes: Vec<u32>,
    },
    TransmogIllusions {
        bnet_account_id: u32,
        illusion_blocks: Vec<AccountMaskBlockLikeCpp>,
    },
}

/// One packed bitmask block of an account-wide collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMaskBlockLikeCpp {
    pub block_index: u32,
    pub mask: u32,
}

impl AccountCollectionSaveLikeCpp {
    /// True when there is nothing to write. The caller skips the transaction
    /// rather than opening an empty one.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Mounts(rows) => rows.is_empty(),
            Self::Toys(rows) => rows.is_empty(),
            Self::Heirlooms(rows) => rows.is_empty(),
            Self::ItemAppearances {
                appearance_blocks,
                favorite_inserts,
                favorite_deletes,
                ..
            } => {
                appearance_blocks.is_empty()
                    && favorite_inserts.is_empty()
                    && favorite_deletes.is_empty()
            }
            Self::TransmogIllusions {
                illusion_blocks, ..
            } => illusion_blocks.is_empty(),
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

/// One SQLx-free semantic Player snapshot.
///
/// This request deliberately describes Player state groups, not prepared
/// statements. The MariaDB adapter owns the current statement decomposition
/// and the exact order in which it appends those statements to the single
/// Characters-database transaction. C++ anchor: `Player.cpp:19312-19655`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCharacterSaveRequestLikeCpp {
    pub player_guid: u64,
    pub account_id: u32,
    pub wall_clock_unix_secs: i64,
    pub character: PlayerCharacterSnapshotSaveLikeCpp,
    pub spells: Option<PlayerSpellSaveGroupLikeCpp>,
    pub skills: Option<Vec<PlayerSkillSaveLikeCpp>>,
    pub glyphs: Option<Vec<PlayerGlyphSaveLikeCpp>>,
    pub talents: Option<Vec<PlayerTalentSaveLikeCpp>>,
    pub spell_cooldowns: Option<Vec<PlayerSpellCooldownSaveLikeCpp>>,
    pub spell_charges: Option<Vec<PlayerSpellChargeSaveLikeCpp>>,
    pub action_buttons: Option<PlayerActionButtonsSaveLikeCpp>,
    pub equipment_sets: Option<Vec<PlayerEquipmentSetSaveLikeCpp>>,
    pub void_storage: Option<Vec<PlayerVoidStorageSlotSaveLikeCpp>>,
    pub tutorials: Option<PlayerTutorialsSaveLikeCpp>,
    pub instance_lock_times: Vec<PlayerInstanceLockTimeSaveLikeCpp>,
    pub played_time: PlayerPlayedTimeSaveLikeCpp,
    pub reputations: Vec<PlayerReputationSaveLikeCpp>,
    pub cuf_profiles: Option<Vec<PlayerCufProfileSlotSaveLikeCpp>>,
}

impl PlayerCharacterSaveRequestLikeCpp {
    pub fn committed_groups_like_cpp(&self) -> PlayerCharacterCommittedGroupsLikeCpp {
        let (player_spells, fallback_player_spells) = match &self.spells {
            Some(PlayerSpellSaveGroupLikeCpp::Complete {
                fallback_rows_were_present,
                ..
            }) => (true, *fallback_rows_were_present),
            Some(PlayerSpellSaveGroupLikeCpp::Fallback { .. }) => (false, true),
            None => (false, false),
        };
        PlayerCharacterCommittedGroupsLikeCpp {
            player_spells,
            fallback_player_spells,
            player_skills: self.skills.is_some(),
            equipment_sets: self.equipment_sets.as_ref().is_some_and(|sets| {
                sets.iter()
                    .any(|set| set.state != PlayerEquipmentSetStateLikeCpp::Unchanged)
            }),
            tutorials_changed: self.tutorials.is_some(),
            tutorials_insert: self
                .tutorials
                .as_ref()
                .is_some_and(|tutorials| !tutorials.already_persisted),
            reputation: !self.reputations.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerCharacterCommittedGroupsLikeCpp {
    pub player_spells: bool,
    pub fallback_player_spells: bool,
    pub player_skills: bool,
    pub equipment_sets: bool,
    pub tutorials_changed: bool,
    pub tutorials_insert: bool,
    pub reputation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCharacterSaveResultLikeCpp {
    pub outcome: PersistenceOutcomeLikeCpp,
    pub committed: PlayerCharacterCommittedGroupsLikeCpp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCharacterSnapshotSaveLikeCpp {
    pub position: PlayerPositionSaveLikeCpp,
    pub level: u8,
    pub xp: u32,
    pub money: u64,
    pub rest_state: u8,
    pub player_flags: u32,
    pub rest_bonus: f32,
    pub logout_time: u64,
    pub is_logout_resting: bool,
    pub health: u32,
    pub powers: Option<[i32; 10]>,
    pub talent_reset_cost: u32,
    pub talent_reset_time: u64,
    pub explored_zones: String,
    pub dungeon_difficulty: u32,
    pub raid_difficulty: u32,
    pub legacy_raid_difficulty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerPositionSaveLikeCpp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub map_id: u16,
    pub instance_id: u32,
    pub zone_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerSpellSaveGroupLikeCpp {
    Complete {
        rows: Vec<PlayerSpellSaveLikeCpp>,
        fallback_rows_were_present: bool,
    },
    Fallback {
        rows: Vec<PlayerFallbackSpellSaveLikeCpp>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpellStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Removed,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellSaveLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
    pub dependent: bool,
    pub favorite: bool,
    pub state: PlayerSpellStateLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerFallbackSpellSaveLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub dependent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSkillSaveLikeCpp {
    pub skill_id: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerGlyphSaveLikeCpp {
    pub talent_group: u8,
    pub glyph_slot: u8,
    pub glyph_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTalentSaveLikeCpp {
    pub talent_id: u32,
    pub rank: u8,
    pub talent_group: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCooldownSaveLikeCpp {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellChargeSaveLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerActionButtonsSaveLikeCpp {
    pub spec: u8,
    pub trait_config_id: i32,
    pub rows: Vec<PlayerActionButtonSaveLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActionButtonSaveLikeCpp {
    pub button: u8,
    pub packed_action: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetTypeLikeCpp {
    Equipment,
    Transmog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEquipmentSetSaveLikeCpp {
    pub set_guid: u64,
    pub set_id: u32,
    pub set_type: PlayerEquipmentSetTypeLikeCpp,
    pub state: PlayerEquipmentSetStateLikeCpp,
    pub name: String,
    pub icon: String,
    pub ignore_mask: u32,
    pub assigned_spec_index: i32,
    pub pieces: Vec<u64>,
    pub appearances: Vec<i32>,
    pub enchants: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVoidStorageSaveLikeCpp {
    pub item_id: u64,
    pub item_entry: u32,
    pub creator_guid: u64,
    pub fixed_scaling_level: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVoidStorageSlotSaveLikeCpp {
    pub slot: u8,
    pub item: Option<PlayerVoidStorageSaveLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTutorialsSaveLikeCpp {
    pub tutorials: [u32; 8],
    pub already_persisted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInstanceLockTimeSaveLikeCpp {
    pub instance_id: u32,
    pub release_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerPlayedTimeSaveLikeCpp {
    pub total_time: u32,
    pub level_time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerReputationSaveLikeCpp {
    pub faction_id: u16,
    pub standing: i32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfileSaveLikeCpp {
    pub profile_name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub bool_options: u32,
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfileSlotSaveLikeCpp {
    pub profile_id: u8,
    pub profile: Option<PlayerCufProfileSaveLikeCpp>,
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

    /// Load one account-wide collection from the Login database. The caller
    /// retains collection validation and represented-state publication.
    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp>;

    /// Persist one account-wide collection in its own Login-database
    /// transaction, as C++ does during logout.
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
    fn every_account_collection_load_names_the_login_database_like_cpp() {
        for request in [
            AccountCollectionLoadRequestLikeCpp::Mounts { bnet_account_id: 1 },
            AccountCollectionLoadRequestLikeCpp::Toys { bnet_account_id: 1 },
            AccountCollectionLoadRequestLikeCpp::Heirlooms { bnet_account_id: 1 },
            AccountCollectionLoadRequestLikeCpp::ItemAppearances { bnet_account_id: 1 },
            AccountCollectionLoadRequestLikeCpp::TransmogIllusions { bnet_account_id: 1 },
        ] {
            assert_eq!(request.logical_database(), LogicalDatabaseLikeCpp::Login);
        }
    }

    #[test]
    fn every_session_account_data_scope_names_the_characters_database_like_cpp() {
        for scope in [
            SessionAccountDataScopeLikeCpp::Global { account_id: 1 },
            SessionAccountDataScopeLikeCpp::Character { guid_low: 2 },
        ] {
            assert_eq!(scope.logical_database(), LogicalDatabaseLikeCpp::Characters);
        }
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
