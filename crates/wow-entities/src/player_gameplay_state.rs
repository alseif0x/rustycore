use std::collections::BTreeSet;

use wow_core::ObjectGuid;

use crate::{
    PlayerAchievementCriteriaRecord, PlayerAchievementRecord, PlayerActionButtonRecord,
    PlayerBattlegroundState, PlayerCurrencyRecord, PlayerCustomizationChoice, PlayerGroupState,
    PlayerGuildState, PlayerMailRecord, PlayerQuestGameplayState, PlayerReputationRecord,
    PlayerRestState, PlayerSkillRecord, PlayerSocialState, PlayerSpellChargeRecord,
    PlayerSpellCooldownRecord, PlayerSpellRuntimeState, PlayerTalentRuntimeState, PlayerTaxiState,
    PlayerTransportState,
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
    pub pass_on_group_loot: bool,
    pub inventory_item_counts: Vec<(u32, u32)>,
    pub forced_reputation_ranks: Vec<(u32, u8)>,
    pub transport: Option<PlayerTransportState>,
    pub in_vehicle: bool,
    pub has_vehicle_kit: bool,
    pub vehicle_seat: i32,
    pub pet_guid: Option<ObjectGuid>,
    pub mails: Vec<PlayerMailRecord>,
    pub group: Option<PlayerGroupState>,
    pub guild: PlayerGuildState,
    pub battleground: PlayerBattlegroundState,
    pub reputations: Vec<PlayerReputationRecord>,
    pub achievements: Vec<PlayerAchievementRecord>,
    pub achievement_criteria: Vec<PlayerAchievementCriteriaRecord>,
    pub currencies: Vec<PlayerCurrencyRecord>,
    pub spell_cooldowns: Vec<PlayerSpellCooldownRecord>,
    pub spell_charges: Vec<PlayerSpellChargeRecord>,
    pub rest: PlayerRestState,
}

impl PlayerGameplayState {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}
