use std::collections::{BTreeSet, HashMap};

use wow_core::ObjectGuid;

use crate::{
    PlayerAchievementCriteriaRecord, PlayerAchievementRecord, PlayerActionButtonRecord,
    PlayerBattlegroundState, PlayerCustomizationChoice, PlayerGroupState, PlayerGuildState,
    PlayerMailRecord, PlayerQuestGameplayState, PlayerReputationRecord, PlayerRestState,
    PlayerSkillRecord, PlayerSocialState, PlayerSpellChargeRecord, PlayerSpellCooldownRecord,
    PlayerSpellRuntimeState, PlayerTalentRuntimeState, PlayerTaxiState, PlayerTransportState,
    PlayerWorldLocalState,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerGameplayState {
    /// C++ `Player::m_createMode`.
    pub create_mode: u8,
    /// Transitional raw C++ shapeshift-form projection until aura state owns it.
    pub shapeshift_form_id: u32,
    /// C++ `UF::ActivePlayerData::LootSpecID`.
    pub loot_specialization_id: u32,
    pub quests: PlayerQuestGameplayState,
    pub skills: Vec<PlayerSkillRecord>,
    pub skills_loaded: bool,
    pub skills_complete: bool,
    pub occupied_skill_slots: Option<u16>,
    pub non_durable_skill_tombstones: BTreeSet<u16>,
    pub spells: PlayerSpellRuntimeState,
    pub talents: PlayerTalentRuntimeState,
    pub action_buttons: Vec<PlayerActionButtonRecord>,
    /// C++ `Player::m_actionButtons` has been hydrated from its authoritative
    /// Character DB query. An empty button list is valid and must remain
    /// distinguishable from an unavailable load.
    pub action_buttons_loaded: bool,
    pub taxi: PlayerTaxiState,
    pub social: PlayerSocialState,
    /// C++ `UF::ActivePlayerData::KnownTitles`, represented as bit indices.
    pub known_title_ids: BTreeSet<u32>,
    pub customizations: Vec<PlayerCustomizationChoice>,
    pub gray_level: u8,
    pub liquid_status: u32,
    pub dungeon_difficulty_id: u32,
    pub raid_difficulty_id: u32,
    pub legacy_raid_difficulty_id: u32,
    pub pass_on_group_loot: bool,
    pub forced_reputation_ranks: Vec<(u32, u8)>,
    pub transport: Option<PlayerTransportState>,
    pub world_local: PlayerWorldLocalState,
    /// C++ `Unit::m_vehicleKit` for Player mount vehicles.
    pub mount_vehicle_kit: Option<crate::Vehicle>,
    /// Current C++ `VehicleSeatEntry::Flags` and `ID` for the Player passenger.
    pub vehicle_seat_flags: Option<i32>,
    pub vehicle_seat_id: Option<u32>,
    /// C++ `UF::ActivePlayerData::LocalFlags`.
    pub active_local_flags: u32,
    /// C++ `UF::ActivePlayerData::TransportServerTime`.
    pub active_transport_server_time: i32,
    /// C++ `UF::ActivePlayerData::MultiActionBars`.
    pub multi_action_bars: u8,
    pub pet_guid: Option<ObjectGuid>,
    pub mails: Vec<PlayerMailRecord>,
    pub group: Option<PlayerGroupState>,
    pub guild: PlayerGuildState,
    pub battleground: PlayerBattlegroundState,
    pub reputations: Vec<PlayerReputationRecord>,
    pub achievements: Vec<PlayerAchievementRecord>,
    pub achievement_criteria: Vec<PlayerAchievementCriteriaRecord>,
    /// C++ `Player::_currencyStorage`, including its per-row persistence state.
    pub currencies: HashMap<u32, PlayerCurrency>,
    pub spell_cooldowns: Vec<PlayerSpellCooldownRecord>,
    pub spell_charges: Vec<PlayerSpellChargeRecord>,
    pub rest: PlayerRestState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCurrencyState {
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Removed = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCurrency {
    pub state: PlayerCurrencyState,
    pub quantity: u32,
    pub weekly_quantity: u32,
    pub tracked_quantity: u32,
    pub increased_cap_quantity: u32,
    pub earned_quantity: u32,
    pub flags: u8,
}

impl PlayerGameplayState {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}
