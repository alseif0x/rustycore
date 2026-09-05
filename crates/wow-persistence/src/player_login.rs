//! Player login/admission projections and ordered login repair/online requests.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceOutcomeLikeCpp};

/// One read-only Characters-database input used while hydrating a Player.
///
/// C++ prepares these in `LoginQueryHolder::Initialize`. The request names the
/// lifecycle data being loaded; statement identity and row decoding remain in
/// the concrete adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginAuxiliaryLoadRequestLikeCpp {
    Mail {
        player_guid: u64,
    },
    Customizations {
        player_guid: u64,
    },
    CompletedAchievements {
        player_guid: u64,
    },
    InstanceTimeRestrictions {
        account_id: u32,
    },
    SpellCooldowns {
        player_guid: u64,
    },
    SpellCharges {
        player_guid: u64,
    },
    TraitEntries {
        player_guid: u64,
    },
    TraitConfigs {
        player_guid: u64,
    },
    PetStable {
        player_guid: u64,
    },
    PetAuras {
        pet_number: u32,
    },
    PetAuraEffects {
        pet_number: u32,
    },
    PetSpells {
        pet_number: u32,
    },
    PetSpellCooldowns {
        pet_number: u32,
    },
    PetSpellCharges {
        pet_number: u32,
    },
    PetDeclinedNames {
        player_guid: u64,
        pet_number: u32,
    },
    GroupMembership {
        player_guid: u64,
    },
    EquipmentSets {
        player_guid: u64,
    },
    TransmogOutfits {
        player_guid: u64,
    },
    CufProfiles {
        player_guid: u64,
    },
    Currencies {
        player_guid: u64,
    },
    Spells {
        player_guid: u64,
    },
    SpellFavorites {
        player_guid: u64,
    },
    Skills {
        player_guid: u64,
    },
    Talents {
        player_guid: u64,
    },
    Glyphs {
        player_guid: u64,
    },
    ActionButtons {
        player_guid: u64,
        active_spec: u8,
        trait_config_id: i32,
    },
    Reputation {
        player_guid: u64,
    },
    CharacterAuras {
        player_guid: u64,
    },
    CharacterAuraEffects {
        player_guid: u64,
    },
    EquipmentInventory {
        player_guid: u64,
    },
    BagInventory {
        player_guid: u64,
    },
    VoidStorage {
        player_guid: u64,
    },
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

/// C++ `CHAR_SEL_MAIL` projection installed into the canonical Player during
/// `Player::LoadFromDB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerMailLoadRowLikeCpp {
    pub mail_id: u32,
    pub message_type: u8,
    pub sender: u64,
    pub receiver: u64,
    pub expire_time: u64,
    pub deliver_time: u64,
    pub checked_flags: u32,
    pub stationery_id: i32,
    pub template_id: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellLoadRowLikeCpp {
    pub spell_id: u32,
    pub active: u8,
    pub disabled: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSkillLoadRowLikeCpp {
    pub skill_id: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTalentLoadRowLikeCpp {
    pub talent_id: u32,
    pub rank: u8,
    pub talent_group: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerGlyphLoadRowLikeCpp {
    pub talent_group: u8,
    pub glyph_slot: u8,
    pub glyph_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActionButtonLoadRowLikeCpp {
    pub button: u8,
    pub action: u32,
    pub button_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerReputationLoadRowLikeCpp {
    pub faction_id: u16,
    pub standing: i32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCharacterAuraLoadRowLikeCpp {
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
pub struct PlayerCharacterAuraEffectLoadRowLikeCpp {
    pub caster_guid_binary: Vec<u8>,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

/// Shared item-instance projection selected by both halves of C++
/// `Player::_LoadInventory`. The adapter owns the joined SQL column order;
/// gameplay retains all item-template and slot interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInventoryItemLoadRowLikeCpp {
    pub item_entry: u32,
    pub item_db_guid: u64,
    pub count: u32,
    pub durability: u32,
    pub context: u8,
    pub flags: u32,
    pub played_time: u32,
    pub enchantments: String,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub gems: [(i32, String, u8); 3],
    pub paid_money: Option<u64>,
    pub paid_extended_cost: Option<u16>,
    pub expiration: u32,
    pub spell_charges: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEquipmentInventoryLoadRowLikeCpp {
    pub slot: u8,
    pub item: PlayerInventoryItemLoadRowLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBagInventoryLoadRowLikeCpp {
    pub bag_slot: u8,
    pub inner_slot: u8,
    pub item: PlayerInventoryItemLoadRowLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerVoidStorageLoadRowLikeCpp {
    pub item_id: u64,
    pub item_entry: u32,
    pub slot: u8,
    pub creator_guid: u64,
    pub fixed_scaling_level: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
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
    Mail(Vec<PlayerMailLoadRowLikeCpp>),
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
    Spells(Vec<PlayerSpellLoadRowLikeCpp>),
    SpellFavorites(Vec<u32>),
    Skills(Vec<PlayerSkillLoadRowLikeCpp>),
    Talents(Vec<PlayerTalentLoadRowLikeCpp>),
    Glyphs(Vec<PlayerGlyphLoadRowLikeCpp>),
    ActionButtons(Vec<PlayerActionButtonLoadRowLikeCpp>),
    Reputation(Vec<PlayerReputationLoadRowLikeCpp>),
    CharacterAuras(Vec<PlayerCharacterAuraLoadRowLikeCpp>),
    CharacterAuraEffects(Vec<PlayerCharacterAuraEffectLoadRowLikeCpp>),
    EquipmentInventory(Vec<PlayerEquipmentInventoryLoadRowLikeCpp>),
    BagInventory(Vec<PlayerBagInventoryLoadRowLikeCpp>),
    VoidStorage(Vec<PlayerVoidStorageLoadRowLikeCpp>),
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

/// One ordered repair discovered while hydrating Player inventory at login.
///
/// Gameplay owns the decision and the corrected values. The concrete adapter
/// owns the C++ statement expansion: clearing refundable metadata is two
/// statements, while normalizing mutable item state is one statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginItemRepairActionLikeCpp {
    ClearRefundable {
        item_guid: u64,
        new_flags: u32,
    },
    NormalizeOnLoad {
        item_guid: u64,
        expiration: u32,
        flags: u32,
        durability: u32,
    },
}

/// One existing Player-login item repair transaction.
///
/// The caller submits equipment and bag repairs separately so their current
/// transaction boundaries and load order cannot be accidentally merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLoginItemRepairRequestLikeCpp {
    pub actions: Vec<PlayerLoginItemRepairActionLikeCpp>,
}

/// The two independent writes C++ performs for
/// `AT_LOGIN_RESET_PET_TALENTS`, in execution order.
///
/// They are deliberately not one transaction or one flattened outcome:
/// failure deleting spells must not suppress the specialization reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLoginPetTalentResetOutcomeLikeCpp {
    pub spell_delete: PersistenceOutcomeLikeCpp,
    pub specialization_reset: PersistenceOutcomeLikeCpp,
}

/// One best-effort Characters-database online mark at the existing Rust login
/// publication point. Statement identity and bind width remain adapter-owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerOnlineMarkRequestLikeCpp {
    /// Existing Rust storage domain. C++ binds UInt64 here; widening that
    /// observable adapter detail is a separate fidelity change, not #432.
    pub player_guid: u32,
}

impl PlayerOnlineMarkRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

impl PlayerLoginItemRepairRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}
