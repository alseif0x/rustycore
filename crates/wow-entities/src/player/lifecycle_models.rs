use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerLifecyclePower {
    pub power: PowerType,
    pub current: i32,
    pub max: i32,
}

impl PlayerLifecyclePower {
    pub const fn new(power: PowerType, current: i32, max: i32) -> Self {
        Self {
            power,
            current,
            max,
        }
    }
}

/// Represented subset of TrinityCore `Player::Create` input.
///
/// Appearance validation, player-info starter spells/items/actions, skills, inventory item
/// creation and threat/combat subsystem startup remain deferred until their canonical systems are
/// ported. This record only carries fields currently owned by `wow-entities`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCreateLifecycleRecord {
    pub guid: ObjectGuid,
    pub name: String,
    pub race: u8,
    pub class_id: u8,
    pub gender: Gender,
    pub level: u8,
    pub xp: i32,
    pub money: u64,
    pub inventory_slot_count: u8,
    pub bank_bag_slot_count: u8,
    pub map_id: u32,
    pub position: Position,
    pub max_health: u64,
    pub health: u64,
    pub powers: Vec<PlayerLifecyclePower>,
    pub display_power: PowerType,
    pub faction_template: Option<u32>,
    pub display_id: Option<u32>,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub extra_flags: u32,
    pub create_time: Option<u64>,
    pub create_mode: Option<u8>,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub active_talent_group: Option<u8>,
}

/// Represented subset of TrinityCore `Player::LoadFromDB` base `characters` row.
///
/// Ownership/coordinate validation and subsystem loads (spells, items, quests, guild, auras,
/// action buttons, reputation, currencies, achievements) are deliberately not faked here; callers
/// should layer those bridges when the relevant systems exist.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDbLoadLifecycleRecord {
    pub guid: ObjectGuid,
    pub account_id: u32,
    pub name: String,
    pub race: u8,
    pub class_id: u8,
    pub gender: Gender,
    pub level: u8,
    pub xp: i32,
    pub money: u64,
    pub inventory_slot_count: u8,
    pub bank_bag_slot_count: u8,
    pub map_id: u32,
    pub position: Position,
    pub max_health: u64,
    pub health: u64,
    pub powers: Vec<PlayerLifecyclePower>,
    pub display_power: PowerType,
    pub faction_template: Option<u32>,
    pub display_id: Option<u32>,
    pub player_flags: u32,
    pub player_flags_ex: u32,
    pub extra_flags: u32,
    pub create_time: Option<u64>,
    pub create_mode: Option<u8>,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub active_talent_group: Option<u8>,
    pub zone_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlayerLifecycleBase {
    pub(super) guid: ObjectGuid,
    pub(super) name: String,
    pub(super) race: u8,
    pub(super) class_id: u8,
    pub(super) gender: Gender,
    pub(super) level: u8,
    pub(super) xp: i32,
    pub(super) money: u64,
    pub(super) inventory_slot_count: u8,
    pub(super) bank_bag_slot_count: u8,
    pub(super) map_id: u32,
    pub(super) position: Position,
    pub(super) max_health: u64,
    pub(super) health: u64,
    pub(super) powers: Vec<PlayerLifecyclePower>,
    pub(super) display_power: PowerType,
    pub(super) faction_template: Option<u32>,
    pub(super) display_id: Option<u32>,
    pub(super) player_flags: u32,
    pub(super) player_flags_ex: u32,
    pub(super) extra_flags: u32,
    pub(super) metadata: PlayerLifecycleMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerLifecycleMetadata {
    pub account_id: Option<u32>,
    pub create_time: Option<u64>,
    pub create_mode: Option<u8>,
    pub played_time_total: u32,
    pub played_time_level: u32,
    pub active_talent_group: Option<u8>,
    pub zone_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLoginLifecycleStep {
    LoadFromDb,
    LoadAccountData,
    SendTutorialData,
    SendFeatureSystemStatus,
    SendTimeZoneInformation,
    SendMotd,
    SendPvpSeasonInfo,
    SendInitialPacketsBeforeAddToMap,
    PlayFirstLoginCinematic,
    AddPlayerToMap,
    RegisterObjectAccessor,
    RestoreGuildAndAuras,
    SendInitialPacketsAfterAddToMap,
    BootstrapVisibility,
    SendZoneWorldStates,
    SendCompactUnitFrameProfiles,
    ApplyLoginAuraEffects,
    SendMovementCompoundState,
    MarkOnline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLoginLifecyclePlan {
    steps: Vec<PlayerLoginLifecycleStep>,
}

impl PlayerLoginLifecyclePlan {
    pub fn trinity_handle_player_login() -> Self {
        Self {
            steps: vec![
                PlayerLoginLifecycleStep::LoadFromDb,
                PlayerLoginLifecycleStep::LoadAccountData,
                PlayerLoginLifecycleStep::SendTutorialData,
                PlayerLoginLifecycleStep::SendFeatureSystemStatus,
                PlayerLoginLifecycleStep::SendTimeZoneInformation,
                PlayerLoginLifecycleStep::SendMotd,
                PlayerLoginLifecycleStep::SendPvpSeasonInfo,
                PlayerLoginLifecycleStep::SendInitialPacketsBeforeAddToMap,
                PlayerLoginLifecycleStep::PlayFirstLoginCinematic,
                PlayerLoginLifecycleStep::AddPlayerToMap,
                PlayerLoginLifecycleStep::RegisterObjectAccessor,
                PlayerLoginLifecycleStep::RestoreGuildAndAuras,
                PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap,
                PlayerLoginLifecycleStep::BootstrapVisibility,
                PlayerLoginLifecycleStep::SendZoneWorldStates,
                PlayerLoginLifecycleStep::SendCompactUnitFrameProfiles,
                PlayerLoginLifecycleStep::ApplyLoginAuraEffects,
                PlayerLoginLifecycleStep::SendMovementCompoundState,
                PlayerLoginLifecycleStep::MarkOnline,
            ],
        }
    }

    pub fn steps(&self) -> &[PlayerLoginLifecycleStep] {
        &self.steps
    }

    pub fn position_of(&self, step: PlayerLoginLifecycleStep) -> Option<usize> {
        self.steps.iter().position(|candidate| *candidate == step)
    }

    pub fn occurs_before(
        &self,
        before: PlayerLoginLifecycleStep,
        after: PlayerLoginLifecycleStep,
    ) -> bool {
        match (self.position_of(before), self.position_of(after)) {
            (Some(before_index), Some(after_index)) => before_index < after_index,
            _ => false,
        }
    }
}

impl Default for PlayerLoginLifecyclePlan {
    fn default() -> Self {
        Self::trinity_handle_player_login()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerWorldInsertionState {
    pub added_to_map: bool,
    pub object_accessor_registered: bool,
    pub visibility_bootstrapped: bool,
    pub worldstates_sent: bool,
}

impl PlayerWorldInsertionState {
    pub fn from_completed_steps(steps: &[PlayerLoginLifecycleStep]) -> Self {
        Self {
            added_to_map: steps.contains(&PlayerLoginLifecycleStep::AddPlayerToMap),
            object_accessor_registered: steps
                .contains(&PlayerLoginLifecycleStep::RegisterObjectAccessor),
            visibility_bootstrapped: steps.contains(&PlayerLoginLifecycleStep::BootstrapVisibility),
            worldstates_sent: steps.contains(&PlayerLoginLifecycleStep::SendZoneWorldStates),
        }
    }
}

/// TrinityCore `Player::LoadFromDB` gameplay subsystem load order, represented as a bridge plan.
///
/// These steps deliberately describe ordering and owned entity-state buckets only. They are not a
/// DB loader, packet delivery pipeline, spell runtime, manager implementation, or session queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerGameplayLoadStep {
    LoadAchievementsAndQuestCriteria,
    LoadHomeBind,
    InitializeSkillFields,
    LoadGroup,
    LoadCurrency,
    LoadInstanceLocks,
    LoadBattlegroundData,
    LoadTaxiMaskAndDestinations,
    InitTaxiNodesForLevel,
    InitStatsForLevel,
    ApplyRestBonus,
    LoadSkills,
    UpdateSkillsForLevel,
    LoadTalents,
    LoadSpells,
    LoadCollectionsGlyphsAndAuras,
    LoadQuestStatus,
    LoadQuestObjectives,
    LoadRewardedQuests,
    LoadDailyWeeklyMonthlySeasonalQuests,
    LoadRandomBattleground,
    LearnDefaultSkills,
    LearnCustomSpells,
    LoadTraits,
    LoadReputation,
    LoadInventory,
    LoadVoidStorage,
    LoadActionButtons,
    LoadMail,
    LoadSocial,
    FinalRelocate,
    LoadSpellCooldownsAndCharges,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerGameplayLoadPlan {
    steps: Vec<PlayerGameplayLoadStep>,
}

impl PlayerGameplayLoadPlan {
    pub fn trinity_load_from_db() -> Self {
        Self {
            steps: vec![
                PlayerGameplayLoadStep::LoadAchievementsAndQuestCriteria,
                PlayerGameplayLoadStep::LoadHomeBind,
                PlayerGameplayLoadStep::InitializeSkillFields,
                PlayerGameplayLoadStep::LoadGroup,
                PlayerGameplayLoadStep::LoadCurrency,
                PlayerGameplayLoadStep::LoadInstanceLocks,
                PlayerGameplayLoadStep::LoadBattlegroundData,
                PlayerGameplayLoadStep::LoadTaxiMaskAndDestinations,
                PlayerGameplayLoadStep::InitTaxiNodesForLevel,
                PlayerGameplayLoadStep::InitStatsForLevel,
                PlayerGameplayLoadStep::ApplyRestBonus,
                PlayerGameplayLoadStep::LoadSkills,
                PlayerGameplayLoadStep::UpdateSkillsForLevel,
                PlayerGameplayLoadStep::LoadTalents,
                PlayerGameplayLoadStep::LoadSpells,
                PlayerGameplayLoadStep::LoadCollectionsGlyphsAndAuras,
                PlayerGameplayLoadStep::LoadQuestStatus,
                PlayerGameplayLoadStep::LoadQuestObjectives,
                PlayerGameplayLoadStep::LoadRewardedQuests,
                PlayerGameplayLoadStep::LoadDailyWeeklyMonthlySeasonalQuests,
                PlayerGameplayLoadStep::LoadRandomBattleground,
                PlayerGameplayLoadStep::LearnDefaultSkills,
                PlayerGameplayLoadStep::LearnCustomSpells,
                PlayerGameplayLoadStep::LoadTraits,
                PlayerGameplayLoadStep::LoadReputation,
                PlayerGameplayLoadStep::LoadInventory,
                PlayerGameplayLoadStep::LoadVoidStorage,
                PlayerGameplayLoadStep::LoadActionButtons,
                PlayerGameplayLoadStep::LoadMail,
                PlayerGameplayLoadStep::LoadSocial,
                PlayerGameplayLoadStep::FinalRelocate,
                PlayerGameplayLoadStep::LoadSpellCooldownsAndCharges,
            ],
        }
    }

    pub fn steps(&self) -> &[PlayerGameplayLoadStep] {
        &self.steps
    }

    pub fn position_of(&self, step: PlayerGameplayLoadStep) -> Option<usize> {
        self.steps.iter().position(|candidate| *candidate == step)
    }

    pub fn occurs_before(
        &self,
        before: PlayerGameplayLoadStep,
        after: PlayerGameplayLoadStep,
    ) -> bool {
        match (self.position_of(before), self.position_of(after)) {
            (Some(before_index), Some(after_index)) => before_index < after_index,
            _ => false,
        }
    }
}

impl Default for PlayerGameplayLoadPlan {
    fn default() -> Self {
        Self::trinity_load_from_db()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerQuestStatusRecord {
    pub quest_id: u32,
    pub status: u8,
    pub explored: bool,
    pub accept_time_secs: i64,
    pub end_time_secs: i64,
    pub objective_counts: Vec<i32>,
    pub slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSkillRecord {
    pub skill_line_id: u32,
    pub current_value: u16,
    pub max_value: u16,
    pub step: u16,
    pub profession_slot: i8,
    pub state: PlayerSkillLoadState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerSkillLoadState {
    #[default]
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpellLoadState {
    Unchanged,
    New,
    Changed,
    Removed,
    Temporary,
}

impl Default for PlayerSpellLoadState {
    fn default() -> Self {
        Self::Unchanged
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerKnownSpellRecord {
    pub spell_id: i32,
    pub state: PlayerSpellLoadState,
    pub active: bool,
    pub disabled: bool,
    pub favorite: bool,
    pub dependent: bool,
}

pub const PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP: usize = 4;
pub const PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerActionButtonRecord {
    pub button: u8,
    pub action_id: u32,
    pub action_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerSocialState {
    pub friend_guids: Vec<ObjectGuid>,
    pub ignore_guids: Vec<ObjectGuid>,
    pub auto_reply_msg_like_cpp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCustomizationChoice {
    pub option_id: u32,
    pub choice_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerTransportState {
    pub guid: ObjectGuid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub seat: i8,
    pub time: u32,
    pub prev_time: Option<u32>,
    pub vehicle_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMailRecord {
    pub mail_id: u32,
    /// C++ `Mail::messageType`; non-normal mail uses the raw sender as the
    /// alternate sender identifier rather than a Player GUID.
    pub message_type: u8,
    pub sender: u64,
    pub receiver: u64,
    pub template_id: Option<u32>,
    pub deliver_time: u64,
    pub expire_time: u64,
    pub checked_flags: u32,
    pub stationery_id: i32,
}

/// C++ `CUFProfile`, owned by `Player::_CUFProfiles` rather than the packet
/// session. Wire conversion remains in `wow-world`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfile {
    pub profile_name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
    pub bool_options: u32,
}

pub const PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP: usize = 19;
pub const PLAYER_VOID_STORAGE_MAX_SLOTS_LIKE_CPP: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetTypeLikeCpp {
    Equipment = 0,
    Transmog = 1,
}

impl PlayerEquipmentSetTypeLikeCpp {
    pub fn handler_branch_from_i32_like_cpp(value: i32) -> Option<Self> {
        if value > Self::Transmog.as_i32_like_cpp() {
            return None;
        }
        if value == Self::Equipment.as_i32_like_cpp() {
            Some(Self::Equipment)
        } else {
            Some(Self::Transmog)
        }
    }

    pub const fn as_i32_like_cpp(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetUpdateStateLikeCpp {
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Deleted = 3,
}

/// C++ `EquipmentSetInfo`, owned by `Player::_equipmentSets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEquipmentSetLikeCpp {
    pub raw_set_type: i32,
    pub set_type: PlayerEquipmentSetTypeLikeCpp,
    pub guid: u64,
    pub set_id: u32,
    pub ignore_mask: u32,
    pub pieces: [ObjectGuid; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub appearances: [i32; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub enchants: [i32; 2],
    pub secondary_shoulder_appearance_id: i32,
    pub secondary_shoulder_slot: i32,
    pub secondary_weapon_appearance_id: i32,
    pub secondary_weapon_slot: i32,
    pub assigned_spec_index: i32,
    pub set_name: String,
    pub set_icon: String,
    pub state: PlayerEquipmentSetUpdateStateLikeCpp,
}

impl PlayerEquipmentSetLikeCpp {
    pub fn equipment(
        set_id: u32,
        assigned_spec_index: i32,
        state: PlayerEquipmentSetUpdateStateLikeCpp,
    ) -> Self {
        Self {
            raw_set_type: 0,
            set_type: PlayerEquipmentSetTypeLikeCpp::Equipment,
            guid: 0,
            set_id,
            ignore_mask: 0,
            pieces: [ObjectGuid::EMPTY; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
            appearances: [0; PLAYER_EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_shoulder_appearance_id: 0,
            secondary_shoulder_slot: 0,
            secondary_weapon_appearance_id: 0,
            secondary_weapon_slot: 0,
            assigned_spec_index,
            set_name: String::new(),
            set_icon: String::new(),
            state,
        }
    }

    pub fn transmog(
        set_id: u32,
        assigned_spec_index: i32,
        state: PlayerEquipmentSetUpdateStateLikeCpp,
    ) -> Self {
        let mut set = Self::equipment(set_id, assigned_spec_index, state);
        set.raw_set_type = 1;
        set.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
        set
    }
}

/// C++ `VoidStorageItem`, owned by `Player::_voidStorageItems`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVoidStorageItemLikeCpp {
    pub item_id: u64,
    pub item_entry: u32,
    pub creator_guid: ObjectGuid,
    pub fixed_scaling_level: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerGroupState {
    pub group_guid: ObjectGuid,
    pub leader_guid: ObjectGuid,
    pub role_mask: u8,
    pub subgroup: u8,
}

/// C++ `Player::GroupUpdateSequence`, owned per player and group category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerGroupUpdateSequenceLikeCpp {
    pub group_guid: Option<u64>,
    pub update_sequence_number: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerGuildState {
    pub guild_id: Option<u64>,
    pub invited_guild_id: Option<u64>,
    pub rank_id: Option<u32>,
    /// True after C++ `_LoadGuild` resolved membership, including no guild.
    pub authority_complete: bool,
}

/// C++ `TradeData`, uniquely owned by `Player::m_trade` while a trade is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTradeStateLikeCpp {
    pub partner_guid: ObjectGuid,
    pub accepted: bool,
    pub partner_server_state_index: u32,
    pub client_state_index: u32,
    pub server_state_index: u32,
    pub items: [Option<ObjectGuid>; PLAYER_TRADE_SLOT_COUNT_LIKE_CPP],
    pub money: u64,
    pub spell_id: u32,
    pub spell_cast_item_guid: Option<ObjectGuid>,
}

/// Persistent C++ `Player` capability fields loaded with the character row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerPersistentCapabilityStateLikeCpp {
    pub at_login_flags: u16,
    pub weapon_proficiency: u32,
    pub armor_proficiency: u32,
}

impl PlayerTradeStateLikeCpp {
    pub const fn new(partner_guid: ObjectGuid) -> Self {
        Self {
            partner_guid,
            accepted: false,
            partner_server_state_index: 0,
            client_state_index: 1,
            server_state_index: 1,
            items: [None; PLAYER_TRADE_SLOT_COUNT_LIKE_CPP],
            money: 0,
            spell_id: 0,
            spell_cast_item_guid: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBattlegroundQueueTypeIdLikeCpp {
    pub battlemaster_list_id: u16,
    pub queue_type: u8,
    pub rated: bool,
    pub team_size: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerBattlegroundQueueSlotLikeCpp {
    pub slot: u32,
    pub queue_type_id: PlayerBattlegroundQueueTypeIdLikeCpp,
    pub invited_instance_guid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBattlegroundQueueRecord {
    pub queue_id: u32,
    pub bracket_id: u8,
    pub joined_at: u64,
    pub team_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerRandomBattlegroundState {
    pub reward_claimed_today: bool,
    pub last_reward_time: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAchievementRecord {
    pub achievement_id: u32,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAchievementCriteriaRecord {
    pub criteria_id: u32,
    pub counter: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCurrencyRecord {
    pub currency_id: u32,
    pub count: u32,
    pub weekly_count: u32,
    pub tracked_quantity: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellCooldownRecord {
    pub spell_id: u32,
    pub item_id: Option<u32>,
    pub category_id: Option<u32>,
    pub cooldown_expires_at: u64,
    pub category_cooldown_expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellChargeRecord {
    pub category_id: u32,
    pub consumed_charges: u8,
    pub recharge_started_at: Option<u64>,
    pub recharge_ends_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDuelStateLikeCpp {
    Challenged,
    Countdown,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDuelInfoLikeCpp {
    pub opponent: ObjectGuid,
    pub state: PlayerDuelStateLikeCpp,
}

/// Canonical `wow-entities` bridge snapshot for gameplay data loaded by TrinityCore
/// `Player::LoadFromDB` after the base `characters` row.
///
/// This state is intentionally independent from update masks. Runtime managers, DB loaders,
/// packet serializers/delivery, spell/aura execution, social/mail managers and session queues
/// remain separate layers and should consume/produce these buckets explicitly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerGameplayLoadRecord {
    pub state: PlayerGameplayState,
}
