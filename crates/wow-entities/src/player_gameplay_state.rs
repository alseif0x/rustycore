use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use wow_constants::{Stats, WeaponAttackType};
use wow_core::{ObjectGuid, Position};

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
    /// C++ `Player::_questRewardedTalentPoints`.
    pub quest_rewarded_talent_points: u32,
    /// C++ `Player::_ApplyItemBonuses` and `Player::ItemSetEff` runtime.
    pub item_modifiers: PlayerItemModifierRuntimeStateLikeCpp,
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
    /// C++ `Player::m_recentInstances`, keyed by map ID.
    pub recent_instances: HashMap<u32, u32>,
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
    /// C++ `Player::_usePvpItemLevels`.
    pub using_pvp_item_levels: bool,
    pub movement_control: PlayerMovementControlStateLikeCpp,
    pub damage_control: PlayerDamageControlStateLikeCpp,
    pub resurrection: PlayerResurrectionStateLikeCpp,
    pub teleport: PlayerTeleportStateLikeCpp,
    /// C++ `Player::m_homebind` / `m_homebindAreaId`.
    pub homebind: Option<PlayerHomebindLikeCpp>,
    /// C++ `Player::m_cinematic`, `m_movie` and the active `CinematicMgr`
    /// camera cursor.
    pub cinematic: PlayerCinematicStateLikeCpp,
    pub pet_lifecycle: PlayerPetLifecycleStateLikeCpp,
    /// C++ `Player::PlayerTalkClass` state. The network session services the
    /// menu, but its mutable interaction lifetime belongs to the Player.
    pub menu: PlayerMenuStateLikeCpp,
    pub reputations: Vec<PlayerReputationRecord>,
    pub achievements: Vec<PlayerAchievementRecord>,
    pub achievement_criteria: Vec<PlayerAchievementCriteriaRecord>,
    /// C++ `Player::_currencyStorage`, including its per-row persistence state.
    pub currencies: HashMap<u32, PlayerCurrency>,
    pub spell_cooldowns: Vec<PlayerSpellCooldownRecord>,
    pub spell_charges: Vec<PlayerSpellChargeRecord>,
    pub rest: PlayerRestState,
    /// C++ `CollectionMgr` state associated with this Player lifetime in the
    /// current single-character Session model. The account-wide persistence
    /// keys remain explicit at the database boundary; mutable collection
    /// decisions no longer live on the protocol Session shell.
    pub collections: PlayerCollectionStateLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerFavoriteAppearanceStateLikeCpp {
    New,
    Removed,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerAccountHeirloomDataLikeCpp {
    pub flags: u32,
    pub bonus_id: u32,
}

/// Canonical mutable owner for the represented C++ `CollectionMgr` families.
///
/// The update-field mirrors themselves remain on `Player::active_data`; this
/// state owns the account collection decisions, temporary providers and dirty
/// favorite transitions that feed those fields and persistence.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerCollectionStateLikeCpp {
    pub mounts: HashMap<i32, u8>,
    pub heirlooms: BTreeMap<u32, PlayerAccountHeirloomDataLikeCpp>,
    pub toys: BTreeMap<u32, u32>,
    pub item_appearances: HashSet<u32>,
    pub item_appearance_blocks: Vec<u32>,
    pub temporary_item_appearances: HashMap<u32, HashSet<ObjectGuid>>,
    pub favorite_item_appearances: HashMap<u32, PlayerFavoriteAppearanceStateLikeCpp>,
    pub transmog_illusions: HashSet<u32>,
}

/// Canonical runtime accumulated by C++ `Player::_ApplyItemBonuses`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerItemBonusStateLikeCpp {
    pub mana_base: i32,
    pub health_base: i32,
    pub armor_base: i32,
    pub armor_total: i32,
    pub stats_base: [i32; 5],
    pub attack_power_total: i32,
    pub ranged_attack_power_total: i32,
    pub resistances_base: [i32; 7],
    pub combat_ratings: [i32; 32],
    pub mana_regen_bonus: i32,
    pub spell_power_bonus: i32,
    pub health_regen_bonus: i32,
    pub spell_penetration_bonus: i32,
    pub shield_block_base_mod: i32,
    pub shield_block_value: u32,
    pub weapon_damage: [[f32; 2]; 3],
    pub base_attack_time: [u32; 3],
    pub stat_buff_updates: Vec<Stats>,
    pub damage_physical_updates: Vec<WeaponAttackType>,
}

impl Default for PlayerItemBonusStateLikeCpp {
    fn default() -> Self {
        Self {
            mana_base: 0,
            health_base: 0,
            armor_base: 0,
            armor_total: 0,
            stats_base: [0; 5],
            attack_power_total: 0,
            ranged_attack_power_total: 0,
            resistances_base: [0; 7],
            combat_ratings: [0; 32],
            mana_regen_bonus: 0,
            spell_power_bonus: 0,
            health_regen_bonus: 0,
            spell_penetration_bonus: 0,
            shield_block_base_mod: 0,
            shield_block_value: 0,
            weapon_damage: [[0.0; 2]; 3],
            base_attack_time: [0; 3],
            stat_buff_updates: Vec::new(),
            damage_physical_updates: Vec::new(),
        }
    }
}

/// Canonical C++ `ItemSetEffect` projection. DB2 row IDs replace pointers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerItemSetEffectLikeCpp {
    pub item_set_id: u32,
    pub equipped_items: HashSet<ObjectGuid>,
    pub set_bonuses: BTreeSet<u32>,
}

/// Player-owned item level limits consumed by `Item::GetItemLevel(Player const*)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerItemLevelCapsLikeCpp {
    pub min_item_level_cutoff: u32,
    pub min_item_level: u32,
    pub max_item_level: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerItemModifierRuntimeStateLikeCpp {
    pub bonuses: PlayerItemBonusStateLikeCpp,
    pub item_set_effects: HashMap<u32, PlayerItemSetEffectLikeCpp>,
    pub item_level_caps: PlayerItemLevelCapsLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerHomebindLikeCpp {
    pub map_id: u32,
    pub area_id: u32,
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCinematicStateLikeCpp {
    pub cinematic_id: Option<u32>,
    pub camera_ids: Option<[u16; 8]>,
    pub camera_index: i32,
    pub movie_id: Option<u32>,
}

impl Default for PlayerCinematicStateLikeCpp {
    fn default() -> Self {
        Self {
            cinematic_id: None,
            camera_ids: None,
            camera_index: -1,
            movie_id: None,
        }
    }
}

/// C++ `PlayerMenu::InteractionData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerInteractionDataLikeCpp {
    pub source_guid: ObjectGuid,
    pub trainer_id: u32,
    pub player_choice_id: u32,
}

impl PlayerInteractionDataLikeCpp {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_source(&mut self, source_guid: ObjectGuid) {
        *self = Self {
            source_guid,
            ..Self::default()
        };
    }

    pub fn set_trainer(&mut self, source_guid: ObjectGuid, trainer_id: u32) {
        *self = Self {
            source_guid,
            trainer_id,
            player_choice_id: 0,
        };
    }

    pub fn reset_if_source(&mut self, source_guid: ObjectGuid) -> bool {
        if self.source_guid != source_guid {
            return false;
        }
        self.reset();
        true
    }

    pub fn trainer_matches(&self, source_guid: ObjectGuid, trainer_id: i32) -> bool {
        self.trainer_id != 0
            && self.source_guid == source_guid
            && self.trainer_id == trainer_id as u32
    }
}

/// Server-side C++ `GossipMenuItem` projection needed to route a client
/// selection without introducing packet types into the canonical entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerGossipOptionLikeCpp {
    pub gossip_option_id: i32,
    pub menu_id: u32,
    pub order_index: u32,
    pub option_npc: u8,
    pub action_menu_id: u32,
}

/// Mutable subset of C++ `PlayerMenu` represented by the current runtime.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerMenuStateLikeCpp {
    pub interaction: PlayerInteractionDataLikeCpp,
    pub gossip_options: Vec<PlayerGossipOptionLikeCpp>,
}

/// Player-owned movement acknowledgement and fall bookkeeping from C++
/// `Player` (`m_forced_speed_changes`,
/// `m_movementForceModMagnitudeChanges`, `m_lastFallTime`, `m_lastFallZ`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerMovementControlStateLikeCpp {
    pub forced_speed_changes: [u8; 9],
    pub movement_force_mod_magnitude_changes: u8,
    pub last_fall_time: u32,
    pub last_fall_z: f32,
    /// Server-owned `MOVEMENTFLAG2_CAN_SWIM_TO_FLY_TRANS` state.
    pub can_swim_to_fly_transition: bool,
    /// Whether the vehicle moving this Player has `VEHICLE_FLAG_FIXED_POSITION`.
    pub mover_fixed_position_vehicle: bool,
    /// C++ `UnitData::ScaleDuration` used by collision-height movement packets.
    pub scale_duration: i32,
}

/// Player/Unit-owned damage gates currently represented by the Rust runtime.
/// C++ keeps the god command bit on `Player::_activeCheats`; physical and
/// environmental immunity are evaluated from the target Unit/Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerDamageControlStateLikeCpp {
    pub cheat_god: bool,
    pub normal_damage_immune: bool,
    pub environmental_damage_immune: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerResurrectionRequestLikeCpp {
    pub resurrecter: ObjectGuid,
    pub map_id: u32,
    pub position: wow_core::Position,
    pub health: u32,
    pub mana: u32,
    pub aura: u32,
}

/// C++ Player-owned resurrection lifecycle state: `_resurrectionData`,
/// `SelfResSpells`, `m_deathTimer`, delayed resurrection and spirit-healer queue.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerResurrectionStateLikeCpp {
    pub request: Option<PlayerResurrectionRequestLikeCpp>,
    pub delayed_after_teleport: Option<PlayerResurrectionRequestLikeCpp>,
    pub self_res_spells: BTreeSet<i32>,
    pub death_timer_active: bool,
    pub area_spirit_healer_guid: ObjectGuid,
}

/// C++ `Player` teleport bookkeeping (`m_teleport_dest`, teleport options,
/// near/far semaphores and delayed-teleport flags). The state remains owned by
/// the same canonical Player while MapManager marks it detached during a far
/// teleport.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerTeleportStateLikeCpp {
    pub can_delay: bool,
    pub has_delayed: bool,
    pub near_pending: bool,
    pub far_pending: bool,
    pub near_destination: Option<(u16, wow_core::Position)>,
    pub delayed: Option<(u32, wow_core::Position, u32)>,
    pub near_destination_zone_area: Option<(u32, u32)>,
}

/// Player-owned C++ pet lifetime bookkeeping. The live `Pet` remains owned by
/// canonical Map/Unit storage; query-holder spell/aura rows remain load staging.
///
/// Mirrors `Player::m_petStable`, `m_temporaryUnsummonedPetNumber`,
/// `m_temporaryPetReactState`, and `m_oldpetspell`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerPetLifecycleStateLikeCpp {
    pub stable: crate::PetStable,
    /// Complete empty result from the current Player's `character_pet` query.
    pub character_rows_empty_authority_complete: bool,
    pub temporary_unsummoned_pet_number: u32,
    pub old_pet_spell: u32,
    pub temporary_mount_react_state: Option<u8>,
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
