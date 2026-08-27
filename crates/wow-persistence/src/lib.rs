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

/// The logical databases the lifecycle can address. Deliberately not a
/// connection, pool or URL — only which store a request belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalDatabaseLikeCpp {
    Characters,
    Login,
    World,
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

/// One read-only Characters-database input used while hydrating a Player.
///
/// C++ prepares these in `LoginQueryHolder::Initialize`. The request names the
/// lifecycle data being loaded; statement identity and row decoding remain in
/// the concrete adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginAuxiliaryLoadRequestLikeCpp {
    Customizations { player_guid: u64 },
    CompletedAchievements { player_guid: u64 },
    InstanceTimeRestrictions { account_id: u32 },
    SpellCooldowns { player_guid: u64 },
    SpellCharges { player_guid: u64 },
    TraitEntries { player_guid: u64 },
    TraitConfigs { player_guid: u64 },
    PetStable { player_guid: u64 },
    PetAuras { pet_number: u32 },
    PetAuraEffects { pet_number: u32 },
    PetSpells { pet_number: u32 },
    PetSpellCooldowns { pet_number: u32 },
    PetSpellCharges { pet_number: u32 },
    PetDeclinedNames { player_guid: u64, pet_number: u32 },
    GroupMembership { player_guid: u64 },
    EquipmentSets { player_guid: u64 },
    TransmogOutfits { player_guid: u64 },
    CufProfiles { player_guid: u64 },
    Currencies { player_guid: u64 },
}

/// Early Characters-database reads that decide where and under which guild
/// projection a Player enters the world. They are distinct from the Eq-only
/// auxiliary row families because locations carry floating-point coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginAdmissionLoadRequestLikeCpp {
    BattlegroundLocation { player_guid: u64 },
    HomebindLocation { player_guid: u64 },
    GuildMembership { player_guid: u64 },
}

impl PlayerLoginAdmissionLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// The core `characters` row requested first by C++
/// `CharacterLoginQueryHolder::Initialize` for `Player::LoadFromDB`.
///
/// The request deliberately carries only semantic identity. Statement choice,
/// bind width and MariaDB row decoding remain private to `wow-database`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCharacterBaseLoadRequestLikeCpp {
    pub player_guid: u64,
}

impl PlayerCharacterBaseLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// The subset of C++ `PlayerLoadData` currently consumed by Rust's represented
/// login path. Optional columns remain unknown across the adapter boundary so
/// the Player lifecycle caller, not the database adapter, retains its existing
/// fallback rules.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCharacterBaseLoadRowLikeCpp {
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub level: u8,
    pub xp: Option<u32>,
    pub money: Option<u64>,
    pub inventory_slots: Option<u8>,
    pub bank_slots: Option<u8>,
    pub rest_state: Option<u8>,
    pub player_flags: Option<u32>,
    pub player_flags_ex: Option<u32>,
    pub position_x: Option<f32>,
    pub position_y: Option<f32>,
    pub position_z: Option<f32>,
    pub map_id: Option<u16>,
    pub orientation: Option<f32>,
    pub create_mode: Option<u8>,
    pub total_played_time: Option<u32>,
    pub level_played_time: Option<u32>,
    pub rest_bonus: Option<f32>,
    pub logout_time_secs: Option<u64>,
    pub logout_was_resting: Option<u8>,
    pub talent_reset_cost: Option<u32>,
    pub talent_reset_time_secs: Option<u64>,
    pub active_talent_group: Option<u8>,
    pub bonus_talent_groups: Option<u8>,
    pub transport_x: Option<f32>,
    pub transport_y: Option<f32>,
    pub transport_z: Option<f32>,
    pub transport_orientation: Option<f32>,
    pub transport_guid_low: Option<u64>,
    pub summoned_pet_number: Option<u32>,
    pub at_login_flags: Option<u16>,
    pub zone_id: Option<u16>,
    pub dungeon_difficulty: Option<u32>,
    pub chosen_title: Option<u32>,
    pub health: Option<u32>,
    pub powers: [Option<u32>; 10],
    pub explored_zones: String,
    pub known_titles: Option<String>,
    pub raid_difficulty: Option<u32>,
    pub legacy_raid_difficulty: Option<u32>,
}

/// A read has no ambiguous-COMMIT state. `Loaded(None)` preserves the distinct
/// C++ missing-character branch; a driver failure remains separately reported.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerCharacterBaseLoadOutcomeLikeCpp {
    Loaded(Option<PlayerCharacterBaseLoadRowLikeCpp>),
    Failed { reason: String },
}

/// One logout-time clear of the represented Player buyback inventory.
///
/// The adapter deletes each item from `character_inventory` and then from
/// `item_instance` in this order, preserving the existing Rust representation
/// of C++ `Player::_SaveInventory` without exposing statements to Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBuybackClearRequestLikeCpp {
    pub player_guid: u64,
    pub item_db_guids: Vec<u64>,
}

/// Refresh the account's character count for one realm after a represented
/// character lifecycle mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRealmCharacterCountRefreshRequestLikeCpp {
    pub account_id: u32,
    pub realm_id: u32,
}

/// One raw World-database template row loaded by C++ `WorldStateMgr::LoadFromDB`.
///
/// Map/area validation stays in the gameplay/data owner because it requires the
/// canonical DB2 stores; the persistence adapter owns only statement selection
/// and row decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInitialWorldStateTemplateRowLikeCpp {
    pub id: i32,
    pub default_value: i32,
    pub map_ids_csv: String,
    pub area_ids_csv: String,
}

/// One Characters-database override loaded after the World templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInitialWorldStateValueRowLikeCpp {
    pub id: i32,
    pub value: i32,
}

/// Independently classified half of C++ `WorldStateMgr::LoadFromDB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerInitialWorldStateRowsLikeCpp<Row> {
    Loaded(Vec<Row>),
    Failed { reason: String },
}

/// Ordered result of loading World templates followed by Characters values.
///
/// The two outcomes remain independent: the existing runtime still performs
/// the second read after a failed first read, and retains template defaults
/// when only the second read fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInitialWorldStatesLoadOutcomeLikeCpp {
    pub templates: PlayerInitialWorldStateRowsLikeCpp<PlayerInitialWorldStateTemplateRowLikeCpp>,
    pub saved_values: PlayerInitialWorldStateRowsLikeCpp<PlayerInitialWorldStateValueRowLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginTransportLoadRequestLikeCpp {
    All,
    ByGuid { guid_low: u64 },
}

impl PlayerLoginTransportLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::World
    }
}

/// One joined World-database transport row required by the represented login
/// materialization path. Route, phase and time validation remain gameplay work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerLoginTransportLoadRowLikeCpp {
    pub guid_low: u32,
    pub entry: u32,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub display_id: u32,
    pub scale: f32,
    pub taxi_path_id: u16,
    pub move_speed: u32,
    pub accel_rate: u32,
    pub allow_stopping: bool,
    pub gameobject_flags: u32,
    pub faction_template: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerLoginTransportLoadOutcomeLikeCpp {
    Loaded(Vec<PlayerLoginTransportLoadRowLikeCpp>),
    Failed { reason: String },
}

impl PlayerRealmCharacterCountRefreshRequestLikeCpp {
    pub fn logical_databases(&self) -> [LogicalDatabaseLikeCpp; 2] {
        [
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Login,
        ]
    }
}

impl PlayerBuybackClearRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

impl PlayerLoginAuxiliaryLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCustomizationLoadRowLikeCpp {
    pub option_id: u32,
    pub choice_id: u32,
}

/// Raw optional columns used by C++ `Player::_LoadBGData`. Missing values stay
/// unknown until the Player lifecycle owner applies its existing validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerBattlegroundLocationLoadRowLikeCpp {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub orientation: Option<f32>,
    pub map_id: Option<u16>,
}

/// Raw optional columns used by C++ `Player::_LoadHomeBind`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerHomebindLocationLoadRowLikeCpp {
    pub map_id: Option<u16>,
    pub area_id: Option<u16>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub orientation: Option<f32>,
}

/// One raw guild-membership row. `None` is malformed/unknown, not guild zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerGuildMembershipLoadRowLikeCpp {
    pub guild_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInstanceTimeRestrictionLoadRowLikeCpp {
    pub instance_id: u32,
    pub release_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCooldownLoadRowLikeCpp {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end: i64,
    pub category_id: u32,
    pub category_end: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellChargeLoadRowLikeCpp {
    pub category_id: u32,
    pub recharge_start: i64,
    pub recharge_end: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerPetStableLoadRowLikeCpp {
    pub pet_number: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub level: u8,
    pub experience: u32,
    pub react_state: u8,
    pub slot: i16,
    pub name: String,
    pub was_renamed: bool,
    pub health: u32,
    pub mana: u32,
    pub action_bar: String,
    pub last_save_time: u32,
    pub created_by_spell_id: u32,
    pub pet_type: u8,
    pub specialization_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerPetAuraLoadRowLikeCpp {
    pub caster_guid_binary: Vec<u8>,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerPetAuraEffectLoadRowLikeCpp {
    pub caster_guid_binary: Vec<u8>,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerPetSpellLoadRowLikeCpp {
    pub spell_id: u32,
    pub active: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerPetSpellCooldownLoadRowLikeCpp {
    pub spell_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerPetSpellChargeLoadRowLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerPetDeclinedNamesLoadRowLikeCpp {
    pub names: [String; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEquipmentSetLoadRowLikeCpp {
    pub set_guid: u64,
    pub set_id: u8,
    pub name: String,
    pub icon: String,
    pub ignore_mask: u32,
    pub assigned_spec_index: i32,
    pub item_low_guids: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTransmogOutfitLoadRowLikeCpp {
    pub set_guid: u64,
    pub set_id: u8,
    pub name: String,
    pub icon: String,
    pub ignore_mask: u32,
    pub appearances: Vec<i32>,
    pub enchants: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfileLoadRowLikeCpp {
    pub id: u8,
    pub name: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCurrencyLoadRowLikeCpp {
    pub currency_id: u16,
    pub quantity: u32,
    pub weekly_quantity: u32,
    pub tracked_quantity: u32,
    pub increased_cap_quantity: u32,
    pub earned_quantity: u32,
    pub flags: u8,
}

/// One raw `character_trait_entry` row. Missing columns remain unknown so the
/// Player owner can keep its represented authority incomplete instead of
/// silently turning malformed database data into zero-valued gameplay state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTraitEntryLoadRowLikeCpp {
    pub trait_config_id: Option<i32>,
    pub trait_node_id: Option<i32>,
    pub trait_node_entry_id: Option<i32>,
    pub rank: Option<i32>,
    pub granted_ranks: Option<i32>,
}

/// One raw `character_trait_config` row with the same unknown-column contract
/// as `PlayerTraitEntryLoadRowLikeCpp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTraitConfigLoadRowLikeCpp {
    pub id: Option<i32>,
    pub config_type: Option<i32>,
    pub chr_specialization_id: Option<i32>,
    pub combat_config_flags: Option<i32>,
    pub local_identifier: Option<i32>,
    pub skill_line_id: Option<i32>,
    pub trait_system_id: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerLoginAuxiliaryLoadedLikeCpp {
    Customizations(Vec<PlayerCustomizationLoadRowLikeCpp>),
    CompletedAchievements(Vec<u32>),
    InstanceTimeRestrictions(Vec<PlayerInstanceTimeRestrictionLoadRowLikeCpp>),
    SpellCooldowns(Vec<PlayerSpellCooldownLoadRowLikeCpp>),
    SpellCharges(Vec<PlayerSpellChargeLoadRowLikeCpp>),
    TraitEntries(Vec<PlayerTraitEntryLoadRowLikeCpp>),
    TraitConfigs(Vec<PlayerTraitConfigLoadRowLikeCpp>),
    PetStable(Vec<PlayerPetStableLoadRowLikeCpp>),
    PetAuras(Vec<PlayerPetAuraLoadRowLikeCpp>),
    PetAuraEffects(Vec<PlayerPetAuraEffectLoadRowLikeCpp>),
    PetSpells(Vec<PlayerPetSpellLoadRowLikeCpp>),
    PetSpellCooldowns(Vec<PlayerPetSpellCooldownLoadRowLikeCpp>),
    PetSpellCharges(Vec<PlayerPetSpellChargeLoadRowLikeCpp>),
    PetDeclinedNames(Vec<PlayerPetDeclinedNamesLoadRowLikeCpp>),
    GroupMembership(Vec<u32>),
    EquipmentSets(Vec<PlayerEquipmentSetLoadRowLikeCpp>),
    TransmogOutfits(Vec<PlayerTransmogOutfitLoadRowLikeCpp>),
    CufProfiles(Vec<PlayerCufProfileLoadRowLikeCpp>),
    Currencies(Vec<PlayerCurrencyLoadRowLikeCpp>),
}

/// Read-only lifecycle loads have no unknown-COMMIT state: they either
/// produced typed rows or failed before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerLoginAuxiliaryLoadOutcomeLikeCpp {
    Loaded(PlayerLoginAuxiliaryLoadedLikeCpp),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerLoginAdmissionLoadedLikeCpp {
    BattlegroundLocation(Option<PlayerBattlegroundLocationLoadRowLikeCpp>),
    HomebindLocation(Option<PlayerHomebindLocationLoadRowLikeCpp>),
    GuildMembership(Vec<PlayerGuildMembershipLoadRowLikeCpp>),
}

/// Admission reads have no unknown-COMMIT state. Missing rows and driver
/// failure stay distinct so the Player owner can preserve each C++ branch.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerLoginAdmissionLoadOutcomeLikeCpp {
    Loaded(PlayerLoginAdmissionLoadedLikeCpp),
    Failed { reason: String },
}

/// One canonical-map corpse hydration request.
///
/// C++ `Map::LoadCorpseData` owns the state transition. This request keeps the
/// database identity out of the map/application layer while preserving the
/// exact `(mapId, instanceId)` scope shared by all three reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCorpseLoadRequestLikeCpp {
    pub map_id: u32,
    pub instance_id: u32,
}

impl MapCorpseLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapCorpseLoadRowLikeCpp {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub orientation: f32,
    pub map_id: u16,
    pub display_id: u32,
    pub item_cache: String,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub flags: u8,
    pub dynamic_flags: u8,
    pub ghost_time: u32,
    pub corpse_type: u8,
    pub instance_id: u32,
    pub owner_guid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCorpsePhaseLoadRowLikeCpp {
    pub owner_guid: u64,
    pub phase_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCorpseCustomizationLoadRowLikeCpp {
    pub owner_guid: u64,
    pub option_id: u32,
    pub choice_id: u32,
}

/// Each auxiliary read may fail independently after the base corpse rows have
/// loaded. C++ continues without that auxiliary data in either case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapCorpseAuxiliaryLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapCorpseLoadOutcomeLikeCpp {
    Loaded {
        corpses: Vec<MapCorpseLoadRowLikeCpp>,
        phases: MapCorpseAuxiliaryLoadOutcomeLikeCpp<MapCorpsePhaseLoadRowLikeCpp>,
        customizations: MapCorpseAuxiliaryLoadOutcomeLikeCpp<MapCorpseCustomizationLoadRowLikeCpp>,
    },
    Failed {
        reason: String,
    },
}

/// SQLx-free persistence boundary for C++ `Map::LoadCorpseData`.
pub trait MapCorpsePersistencePortLikeCpp: Send + Sync {
    fn load_map_corpses_like_cpp<'a>(
        &'a self,
        request: MapCorpseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, MapCorpseLoadOutcomeLikeCpp>;
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

/// One retained talent row written by C++ `Player::_SaveTalents` after the
/// active talent group has been reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTalentResetSaveRowLikeCpp {
    pub talent_id: u32,
    pub rank: u8,
    pub talent_group: u8,
}

/// The complete represented talent-reset transaction.
///
/// `money_before`/`money_after` are part of the durability contract rather
/// than gameplay state here: the MariaDB adapter uses the absolute money row
/// to reconcile a lost COMMIT reply. Equal values deliberately prove nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTalentResetPersistenceRequestLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub reset_cost: u32,
    pub reset_time_secs: u64,
    pub retained_talents: Vec<PlayerTalentResetSaveRowLikeCpp>,
}

impl PlayerTalentResetPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// The optional online rest-state row that accompanies one represented XP
/// durability write. Gameplay owns the values; the adapter owns their SQL
/// representation and transaction order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerXpRestStateSaveLikeCpp {
    pub rest_state: u8,
    pub player_flags: u32,
    pub rest_bonus: f32,
}

/// One SQLx-free request for Rusty's represented immediate XP durability
/// boundary. Legacy C++ mutates XP in `Player::GiveXP` and persists it through
/// the ordinary Player save; this contract deliberately preserves Rust's
/// current immediate transaction without claiming parity for its timing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerXpPersistenceRequestLikeCpp {
    pub player_guid: u64,
    pub level_changed: bool,
    pub level: u8,
    pub xp: u32,
    pub rest: Option<PlayerXpRestStateSaveLikeCpp>,
}

impl PlayerXpPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
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
    fn buyback_clear_names_the_character_database_like_cpp() {
        assert_eq!(
            PlayerBuybackClearRequestLikeCpp {
                player_guid: 1,
                item_db_guids: vec![2],
            }
            .logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
    }

    #[test]
    fn realm_character_count_refresh_names_both_independent_databases_like_cpp() {
        assert_eq!(
            PlayerRealmCharacterCountRefreshRequestLikeCpp {
                account_id: 1,
                realm_id: 2,
            }
            .logical_databases(),
            [
                LogicalDatabaseLikeCpp::Characters,
                LogicalDatabaseLikeCpp::Login,
            ]
        );
    }

    #[test]
    fn login_transport_load_names_the_world_database_like_cpp() {
        for request in [
            PlayerLoginTransportLoadRequestLikeCpp::All,
            PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 7 },
        ] {
            assert_eq!(request.logical_database(), LogicalDatabaseLikeCpp::World);
        }
    }

    #[test]
    fn talent_reset_persistence_names_the_characters_database_like_cpp() {
        let request = PlayerTalentResetPersistenceRequestLikeCpp {
            player_guid: 7,
            money_before: 10,
            money_after: 5,
            reset_cost: 5,
            reset_time_secs: 123,
            retained_talents: Vec::new(),
        };
        assert_eq!(
            request.logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
    }

    #[test]
    fn xp_persistence_names_the_characters_database_like_cpp() {
        assert_eq!(
            PlayerXpPersistenceRequestLikeCpp {
                player_guid: 7,
                level_changed: false,
                level: 10,
                xp: 42,
                rest: None,
            }
            .logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
    }

    #[test]
    fn character_base_load_names_the_characters_database_like_cpp() {
        assert_eq!(
            PlayerCharacterBaseLoadRequestLikeCpp { player_guid: 7 }.logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
    }

    #[test]
    fn every_player_login_auxiliary_load_names_the_characters_database_like_cpp() {
        for request in [
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { account_id: 2 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { pet_number: 2 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { pet_number: 2 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { pet_number: 2 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { pet_number: 2 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { pet_number: 2 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames {
                player_guid: 1,
                pet_number: 2,
            },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles { player_guid: 1 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies { player_guid: 1 },
        ] {
            assert_eq!(
                request.logical_database(),
                LogicalDatabaseLikeCpp::Characters
            );
        }
    }

    #[test]
    fn pet_login_rows_keep_success_empty_and_failure_distinct_like_cpp() {
        let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(vec![PlayerPetSpellLoadRowLikeCpp {
                spell_id: 17253,
                active: 1,
            }]),
        );
        let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(Vec::new()),
        );
        let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
            reason: "pet query failed".to_owned(),
        };

        assert_ne!(loaded, empty);
        assert_ne!(empty, failed);
        assert_ne!(loaded, failed);
    }

    #[test]
    fn group_login_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
        let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(vec![77]),
        );
        let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(Vec::new()),
        );
        let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
            reason: "group query failed".to_owned(),
        };

        assert_ne!(loaded, empty);
        assert_ne!(empty, failed);
        assert_ne!(loaded, failed);
    }

    #[test]
    fn profile_login_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
        let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(vec![PlayerCurrencyLoadRowLikeCpp {
                currency_id: 1,
                quantity: 2,
                weekly_quantity: 3,
                tracked_quantity: 4,
                increased_cap_quantity: 5,
                earned_quantity: 6,
                flags: 7,
            }]),
        );
        let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(Vec::new()),
        );
        let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
            reason: "profile query failed".to_owned(),
        };

        assert_ne!(loaded, empty);
        assert_ne!(empty, failed);
        assert_ne!(loaded, failed);
    }

    #[test]
    fn every_player_login_admission_load_names_the_characters_database_like_cpp() {
        for request in [
            PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation { player_guid: 1 },
            PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { player_guid: 1 },
            PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { player_guid: 1 },
        ] {
            assert_eq!(
                request.logical_database(),
                LogicalDatabaseLikeCpp::Characters
            );
        }
    }

    #[test]
    fn map_corpse_hydration_names_the_characters_database_like_cpp() {
        assert_eq!(
            MapCorpseLoadRequestLikeCpp {
                map_id: 571,
                instance_id: 9,
            }
            .logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
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
