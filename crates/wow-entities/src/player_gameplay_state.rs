use std::collections::{BTreeMap, BTreeSet, HashMap};

use wow_core::ObjectGuid;

use crate::{
    PlayerAchievementCriteriaRecord, PlayerAchievementRecord, PlayerActionButtonRecord,
    PlayerBattlegroundState, PlayerCufProfile, PlayerCustomizationChoice,
    PlayerEquipmentSetLikeCpp, PlayerGroupState, PlayerGroupUpdateSequenceLikeCpp,
    PlayerGuildState, PlayerMailRecord, PlayerPersistentCapabilityStateLikeCpp,
    PlayerQuestGameplayState, PlayerReputationRecord, PlayerRestState, PlayerSkillRecord,
    PlayerSocialState, PlayerSpellChargeRecord, PlayerSpellCooldownRecord, PlayerSpellRuntimeState,
    PlayerTalentRuntimeState, PlayerTaxiState, PlayerTradeStateLikeCpp, PlayerTransportState,
    PlayerVoidStorageItemLikeCpp, PlayerWorldLocalState,
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
    /// C++ `Player::mSkillStatus` and its corresponding ActivePlayerData skill slots.
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
    /// C++ `ReputationMgr` cached visible/honored/revered/exalted counters.
    pub reputation_rank_counters: [u8; 4],
    /// C++ `ReputationMgr::_sendFactionIncreased`.
    pub send_faction_increased: bool,
    /// C++ `Player::m_ChampioningFaction`.
    pub championing_faction_id: u32,
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
    /// C++ `Player::_CUFProfiles`; five stable slots plus load authority.
    pub cuf_profiles: Vec<Option<PlayerCufProfile>>,
    pub cuf_profiles_loaded: bool,
    /// C++ `Player::_equipmentSets` plus its coherent Character DB load marker.
    pub equipment_sets: BTreeMap<u64, PlayerEquipmentSetLikeCpp>,
    pub equipment_sets_loaded: bool,
    /// C++ `Player::_voidStorageItems`; normalized to 160 slots by its owner API.
    pub void_storage_items: Vec<Option<PlayerVoidStorageItemLikeCpp>>,
    pub void_storage_loaded: bool,
    pub group: Option<PlayerGroupState>,
    /// C++ `Player::m_groupUpdateSequences`; home and instance categories.
    pub group_update_sequences: [PlayerGroupUpdateSequenceLikeCpp; 2],
    pub guild: PlayerGuildState,
    /// C++ `Player::m_trade`; `None` is the normal no-trade state.
    pub trade: Option<PlayerTradeStateLikeCpp>,
    pub persistent_capabilities: PlayerPersistentCapabilityStateLikeCpp,
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
