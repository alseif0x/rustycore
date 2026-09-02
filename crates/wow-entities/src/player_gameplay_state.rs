use std::collections::BTreeSet;

use wow_core::ObjectGuid;

use crate::{
    PlayerAchievementCriteriaRecord, PlayerAchievementRecord, PlayerActionButtonRecord,
    PlayerBattlegroundState, PlayerCurrencyRecord, PlayerCustomizationChoice, PlayerGroupState,
    PlayerGuildState, PlayerKnownSpellRecord, PlayerMailRecord, PlayerQuestGameplayState,
    PlayerReputationRecord, PlayerRestState, PlayerSkillRecord, PlayerSocialState,
    PlayerSpellChargeRecord, PlayerSpellCooldownRecord, PlayerTalentRecord, PlayerTaxiState,
    PlayerTransportState,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerGameplayState {
    pub quests: PlayerQuestGameplayState,
    pub skills: Vec<PlayerSkillRecord>,
    pub skills_loaded: bool,
    pub skills_complete: bool,
    pub occupied_skill_slots: Option<u16>,
    pub non_durable_skill_tombstones: BTreeSet<u16>,
    pub spells: Vec<PlayerKnownSpellRecord>,
    pub talents: Vec<PlayerTalentRecord>,
    pub action_buttons: Vec<PlayerActionButtonRecord>,
    /// C++ `Player::m_actionButtons` has been hydrated from its authoritative
    /// Character DB query. An empty button list is valid and must remain
    /// distinguishable from an unavailable load.
    pub action_buttons_loaded: bool,
    pub taxi: PlayerTaxiState,
    pub social: PlayerSocialState,
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
