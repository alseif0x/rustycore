// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared directory of active player sessions for broadcast purposes.
//!
//! Each WorldSession registers itself here on player login and removes itself
//! on logout/disconnect. Chat, emote and movement handlers use the directory
//! to fan-out packets to nearby players on the same map.
//!
//! Identifying live session incarnations and resolving their control endpoints
//! is session coordination, not socket transport, so the directory is owned
//! here. The physical byte channels, socket write fences and `InstanceLink`
//! remain in `wow-network`, and the Session mailbox protocol plus its durable
//! rails stay there until issue #140 relocates them.

use crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp;
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyLootMoneyLikeCppCommand,
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand,
    DurableCreatureRuntimeCommandsLikeCpp, LootRollCommandIdentityLikeCpp,
    ReconcilePvpCombatExpiryLikeCppCommand, RefreshVisibleWorldCreaturesLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand, SessionCommand,
    SharedClientVisibleGuidsLikeCpp,
};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use wow_core::{ObjectGuid, Position};
use wow_loot::OwnedLootAuthority;
use wow_packet::packets::movement::TransportInfo;
use wow_packet::packets::party::{
    PartyMemberAuraState, PartyMemberPetStats, PartyMemberPhaseStates,
};
use wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate;

/// Information stored for each active player session.
#[derive(Clone)]
pub struct PlayerBroadcastInfo {
    /// Map ID the player is currently on.
    pub map_id: u16,
    /// Instance ID within the map — distinguishes multiple concurrent instances
    /// of the same dungeon/raid.  0 = world/default instance (fallback when no
    /// canonical map key is available), mirroring C++ Phase/instance filtering
    /// in `GridNotifiersImpl.h : MessageDistDeliverer::Visit`.
    pub instance_id: u32,
    /// Server-side world position (updated on every movement packet).
    pub position: Position,
    /// Current combat reach used by C++ distance gates such as `GetDistanceZ`.
    pub combat_reach: f32,
    /// C++ `Player::IsInCombat` mirrored from the owning session, refreshed
    /// at every combat transition through `set_in_combat_like_cpp` and the
    /// registry state sync; group-level gates (like the LFG boot combat
    /// check) read this per member.
    pub in_combat: bool,
    /// Represented C++ `Unit::GetLiquidStatus()` snapshot for remote accessibility gates.
    pub liquid_status: u32,
    /// Represented C++ `Player::IsInWorld()` receiver gate for global-message fanout.
    pub is_in_world: bool,
    /// Channel used to push serialised packets to this player's primary
    /// (instance after `ConnectTo`) socket.
    pub send_tx: flume::Sender<Vec<u8>>,
    /// Channel used for opcodes registered on `CONNECTION_TYPE_REALM`.
    /// Before `ConnectTo`, or in single-socket tests, this may be the same
    /// channel as [`Self::send_tx`].
    pub realm_send_tx: flume::Sender<Vec<u8>>,
    /// Channel used for C++-style cross-session state mutations.
    pub command_tx: flume::Sender<SessionCommand>,
    /// Durable FIFO rail for authoritative creature combat transitions.
    pub durable_creature_runtime_commands_like_cpp:
        Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>,
    /// Shared C++ `Player::m_clientGUIDs` membership for this session.
    ///
    /// Producers that must commit a recipient decision at the moment a message
    /// is resolved read it here instead of leaving the receiving session to
    /// re-derive visibility from state that moved on in the meantime.
    pub client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp,
    /// Shared C++ advanced-combat-logging preference for this session.
    ///
    /// `WorldObject::SendCombatLogMessage` picks the basic or full `SMSG_SPELL_GO`
    /// frame per receiver while it distributes the cast, so a producer reads this
    /// when the cast resolves rather than leaving the choice to drain time.
    pub advanced_combat_logging_enabled_like_cpp: Arc<AtomicBool>,
    /// Durable/coalesced equivalent of C++'s retained visibility notify bit.
    ///
    /// Senders set this before attempting the bounded command queue. The owning
    /// session consumes it even when the queue was full, so player entry/exit
    /// visibility cannot be lost under command backpressure.
    pub visibility_refresh_pending_like_cpp: Arc<AtomicBool>,
    /// Exact represented pending loot-roll identities owned by this session.
    /// The packet key may be reused, so cross-session routing must clone this
    /// identity into the queued command rather than publishing keys alone.
    pub active_loot_rolls: Vec<LootRollCommandIdentityLikeCpp>,
    /// Current `Player::GetPassOnGroupLoot()` state for group/NBG roll startup.
    pub pass_on_group_loot: bool,
    /// Represented `Player::GetSkillValue(SKILL_ENCHANTING)` used by group-roll disenchant masks.
    pub enchanting_skill: u16,
    /// Represented `Player::IsAlive()` snapshot for cross-session receiver gates.
    pub is_alive: bool,
    /// Represented `Unit::GetHealth()` snapshot for party member full-state packets.
    pub current_health: u32,
    /// Represented `Unit::GetMaxHealth()` snapshot for party member full-state packets.
    pub max_health: u32,
    /// Represented `Unit::GetPowerType()` snapshot for party member full-state packets.
    pub power_type: u8,
    /// Represented `Unit::GetPower(GetPowerType())` snapshot for party member full-state packets.
    pub current_power: u16,
    /// Represented `Unit::GetMaxPower(GetPowerType())` snapshot for party member full-state packets.
    pub max_power: u16,
    /// C++ `UnitData::BaseMana` snapshot used independently from live maximum power in CREATE.
    pub base_mana: i32,
    /// Current MO-transport passenger movement state used by player CREATEs.
    pub transport: Option<TransportInfo>,
    /// Represented `Player::IsPvP()` snapshot for party member full-state packets.
    pub is_pvp: bool,
    /// Represented `Player::IsFFAPvP()` snapshot for party member full-state packets.
    pub is_ffa_pvp: bool,
    /// Represented `Player::HasPlayerFlag(PLAYER_FLAGS_GHOST)` snapshot for party member full-state packets.
    pub is_ghost: bool,
    /// Represented `Player::isAFK()` snapshot for party member full-state packets.
    pub is_afk: bool,
    /// Represented `Player::isDND()` snapshot for party member full-state packets.
    pub is_dnd: bool,
    /// Represented `Player::autoReplyMsg` snapshot used by C++ whisper AFK/DND replies.
    pub auto_reply_msg_like_cpp: String,
    /// Represented `Player::GetVehicle() != nullptr` snapshot for party member full-state packets.
    pub in_vehicle: bool,
    /// Represented `Player::GetVehicleKit() != nullptr` snapshot for player-vehicle interact gates.
    pub has_vehicle_kit_like_cpp: bool,
    /// Represented `VehicleSeatEntry::ID` from `Vehicle::GetSeatForPassenger(player)`.
    pub party_member_vehicle_seat: i32,
    /// Represented `Player::GetZoneId()` snapshot for party member full-state packets.
    pub zone_id: u32,
    /// Represented `Player::GetPrimarySpecialization()` snapshot for party member full-state packets.
    pub spec_id: u32,
    /// Represented `Unit::GetUnitFlags()` snapshot for global creature targetability gates.
    pub unit_flags: u32,
    /// Represented `Unit::GetUnitFlags2()` snapshot for reputation-ignore gates.
    pub unit_flags2: u32,
    /// Represented `Unit::GetUnitState()` snapshot for fake-death/unattackable targetability gates.
    pub unit_state: u32,
    /// Represented `Player::IsGameMaster()` snapshot; C++ rejects GM players as attack targets.
    pub is_game_master: bool,
    /// Represented `Player::GetDungeonDifficultyID()` snapshot for cross-session party invite gates.
    pub dungeon_difficulty_id: u32,
    /// Represented `PLAYER_FLAGS_CONTESTED_PVP` snapshot for contested-guard attackability.
    pub is_contested_pvp: bool,
    /// Active expansion derived from canonical `WorldSession::expansion` for receiver-only quest gates.
    pub active_expansion: u8,
    /// Represented non-empty `Player::GetPlayerSharingQuest()` snapshot for party quest sharing.
    pub pending_quest_sharing: Option<(ObjectGuid, u32)>,
    /// Current known spells, used for remote `ConditionMgr`/loot checks that mirror `Player::HasSpell`.
    pub known_spells: Vec<i32>,
    /// Current quest status map, keyed by quest id, used for remote `Player::GetQuestStatus` checks.
    pub active_quest_statuses: HashMap<u32, u8>,
    /// Active quest objective counters, keyed by quest id, used for remote `Player::HasQuestForItem`.
    pub active_quest_objective_counts: HashMap<u32, Vec<i32>>,
    /// Rewarded quest ids, used for remote `QUEST_STATUS_REWARDED` checks.
    pub rewarded_quests: HashSet<u32>,
    /// C++ `Player::HasAchieved` snapshot for connected-player gates that resolve
    /// another live player through `ObjectAccessor::FindPlayer`, such as
    /// `Player::Satisfy(access_requirement)` checking the group leader.
    pub completed_achievements: HashSet<u32>,
    /// Represented `ActivePlayerData::DailyQuestsCompleted` snapshot for remote `SatisfyQuestDay`.
    pub daily_quests_completed: HashSet<u32>,
    /// Represented `Player::m_DFQuests` snapshot for remote `SatisfyQuestDay`.
    pub df_quests: HashSet<u32>,
    /// Represented `Unit::GetFactionTemplateEntry()` id for C++ hostility/reputation checks.
    pub faction_template_id: u32,
    /// Represented current reputation standing by faction for remote `SatisfyQuestReputation`.
    /// Missing factions are interpreted as standing 0 like C++ no-state path.
    pub reputation_standings: Vec<(u32, i32)>,
    /// Represented reputation flags by faction, including `REPUTATION_FLAG_AT_WAR`.
    pub reputation_state_flags: Vec<(u32, u32)>,
    /// Represented `Player::GetReputationMgr().GetForcedRankIfAny()` ranks.
    pub forced_reputation_ranks: Vec<(u32, wow_data::reputation::ReputationRankLikeCpp)>,
    /// Represented forced-reaction membership mirrored on the canonical player.
    pub forced_reputation_faction_ids: Vec<u32>,
    /// Direct inventory item counts, keyed by item entry, used for remote quest-loot gates.
    pub inventory_item_counts: HashMap<u32, u32>,
    /// C++ `PlayerData::PartyType[2]` snapshot for SMSG_PARTY_MEMBER_FULL_STATE.
    pub party_member_party_type: [u8; 2],
    /// C++ `PartyMemberPhaseStates` snapshot for SMSG_PARTY_MEMBER_FULL_STATE.
    pub party_member_phase_states: PartyMemberPhaseStates,
    /// C++ `PartyMemberAuraStates` snapshot for SMSG_PARTY_MEMBER_FULL_STATE.
    pub party_member_auras: Vec<PartyMemberAuraState>,
    /// C++ `PartyMemberPetStats` snapshot for SMSG_PARTY_MEMBER_FULL_STATE.
    pub party_member_pet_stats: Option<PartyMemberPetStats>,
    /// Character name — used for whisper target lookups.
    pub player_name: String,
    /// Account ID — kept for future same-account filtering.
    pub account_id: u32,
    /// Login account recruiter ID, used by C++ Recruit-A-Friend reward checks.
    pub recruiter_id: u32,
    // ── Character attributes for broadcast packets ──
    /// Race (human, dwarf, etc.)
    pub race: u8,
    /// Class (warrior, mage, etc.)
    pub class: u8,
    /// Sex (0=male, 1=female)
    pub sex: u8,
    /// Character level
    pub level: u8,
    /// C++ `Trinity::XP::GetGrayLevel(level)` snapshot for receiver-side
    /// aggro decisions. Sessions publish this so global map-owned scans do not
    /// recompute or lose script-adjusted gray-level state.
    pub gray_level: u8,
    /// Display ID for model rendering
    pub display_id: u32,
    /// Equipped item display info: (item_entry, enchant_display_id, subclass) per slot 0-18
    pub visible_items: Arc<[(i32, u16, u16); 19]>,
    /// C++ `PlayerData::Customizations` snapshot used by non-owner CREATE blocks.
    pub customizations: Arc<Vec<ChrCustomizationChoiceValuesUpdate>>,
    /// C++ `ActivePlayerData::LifetimeHonorableKills` snapshot for inspect honor stats.
    pub lifetime_honorable_kills: u32,
    /// C++ `ActivePlayerData::ThisWeekContribution` snapshot for inspect honor stats.
    pub this_week_contribution: u32,
    /// C++ `ActivePlayerData::YesterdayContribution` snapshot for inspect honor stats.
    pub yesterday_contribution: u32,
    /// C++ `ActivePlayerData::TodayHonorableKills` snapshot for inspect honor stats.
    pub today_honorable_kills: u16,
    /// C++ `ActivePlayerData::YesterdayHonorableKills` snapshot for inspect honor stats.
    pub yesterday_honorable_kills: u16,
    /// C++ `ActivePlayerData::LifetimeMaxRank` snapshot for inspect honor stats.
    pub lifetime_max_rank: u32,
    /// C++ `PlayerData::HonorLevel` snapshot for inspect honor stats.
    pub honor_level: u32,
}

/// Identity of one concrete connected-session registration.
///
/// A GUID can be registered again while an older session is still unwinding.
/// The generation prevents that older session from looking up or removing the
/// replacement entry.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PlayerRegistration {
    guid: ObjectGuid,
    generation: u64,
}

impl PlayerRegistration {
    #[must_use]
    pub fn guid(self) -> ObjectGuid {
        self.guid
    }

    #[must_use]
    pub fn generation(self) -> u64 {
        self.generation
    }
}

/// Owned control-channel address for one session incarnation.
///
/// The sender is cloned from the selected entry. Replacing the GUID cannot
/// redirect an already-resolved command to the new session.
#[derive(Clone, Debug)]
pub struct PlayerControlAddress {
    registration: PlayerRegistration,
    command_tx: flume::Sender<SessionCommand>,
}

impl PlayerControlAddress {
    #[must_use]
    pub fn registration(&self) -> PlayerRegistration {
        self.registration
    }

    pub fn try_send(
        &self,
        command: SessionCommand,
    ) -> Result<(), flume::TrySendError<SessionCommand>> {
        self.command_tx.try_send(command)
    }
}

/// Failure to deliver through a generation-checked directory address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerDirectorySendError {
    /// The registration disappeared or was replaced before delivery.
    StaleRegistration,
    /// The bounded destination queue is currently full.
    Full,
    /// The destination session has disconnected.
    Disconnected,
}

/// Result of retaining an authoritative command across bounded backpressure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerDirectoryReliableSendOutcome {
    /// The command entered the selected session queue immediately.
    Queued,
    /// The queue was full and an owned retry now retains the command.
    Retrying,
    /// The selected incarnation is stale or its queue disconnected.
    StaleOrDisconnected,
}

/// Owned presence facts used by C++ loot recipient and reward-distance gates.
#[derive(Clone, Copy, Debug)]
pub struct PlayerLootPresenceSnapshot {
    pub registration: PlayerRegistration,
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub is_in_world: bool,
}

/// Owned remote-Player facts required by represented C++ loot conditions.
#[derive(Clone, Debug)]
pub struct PlayerLootContextSnapshot {
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub known_spells: Vec<i32>,
    pub active_quest_statuses: HashMap<u32, u8>,
    pub active_quest_objective_counts: HashMap<u32, Vec<i32>>,
    pub rewarded_quests: HashSet<u32>,
    pub inventory_item_counts: HashMap<u32, u32>,
}

/// Owned Player facts used by C++ group reward calculations.
#[derive(Clone, Copy, Debug)]
pub struct PlayerGroupRewardSnapshot {
    pub level: u8,
    pub map_id: u16,
    pub position: Position,
    pub is_alive: bool,
}

/// Owned receiver facts used by C++ quest-sharing eligibility checks.
/// The mirrored fields remain temporary and receive canonical-owner cutovers
/// in the field-by-field retirement ledger required by issue #196.
#[derive(Clone, Debug)]
pub struct PlayerQuestSharingSnapshot {
    pub registration: PlayerRegistration,
    pub pending_quest_sharing: Option<(ObjectGuid, u32)>,
    pub is_alive: bool,
    pub rewarded_quests: HashSet<u32>,
    pub active_quest_statuses: HashMap<u32, u8>,
    pub df_quests: HashSet<u32>,
    pub daily_quests_completed: HashSet<u32>,
    pub level: u8,
    pub class: u8,
    pub race: u8,
    pub reputation_standings: Vec<(u32, i32)>,
    pub active_expansion: u8,
}

/// Owned facts required by C++ player-vehicle interaction checks. The
/// temporary mirror fields are assigned retirement cutovers by issue #196.
#[derive(Clone, Copy, Debug)]
pub struct PlayerVehicleInteractionSnapshot {
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub has_vehicle_kit: bool,
}

/// Session-owned directory mirrors refreshed after authoritative movement.
/// Issue #196 records the canonical owner and retirement cutover for each.
#[derive(Clone, Debug)]
pub struct PlayerMovementDirectoryUpdate {
    pub position: Position,
    pub map_id: u16,
    pub instance_id: u32,
    pub liquid_status: u32,
    pub transport: Option<TransportInfo>,
}

/// Tracker-free input for preparing one remote durable loot-money command.
#[derive(Clone, Debug)]
pub struct PrepareLootMoneyApplicationLikeCpp {
    pub recipient: ObjectGuid,
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub amount: u64,
    pub durable_applied_amount: Arc<AtomicU64>,
    pub sole_looter: bool,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub authority_committed: Arc<AtomicBool>,
    pub send_coin_removed: Arc<AtomicBool>,
    pub applied: Arc<AtomicBool>,
    pub published: Arc<AtomicBool>,
}

/// Generation-bound durable loot-money command prepared by the directory.
#[derive(Clone, Debug)]
pub struct PreparedLootMoneyApplicationLikeCpp {
    pub registration: PlayerRegistration,
    pub command: ApplyLootMoneyLikeCppCommand,
}

/// Owned facts used only for runtime recipient selection.
#[derive(Clone, Debug)]
pub struct PlayerRuntimeRecipient {
    pub registration: PlayerRegistration,
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub combat_reach: f32,
    pub liquid_status: u32,
    pub is_in_world: bool,
    pub is_alive: bool,
    pub account_id: u32,
    pub advanced_combat_logging: bool,
    pub committed_visibility: SharedClientVisibleGuidsLikeCpp,
}

/// Owned online identity used by chat/social lookup. Delivery must use the
/// included registration so a reconnect cannot receive an older decision.
#[derive(Clone, Debug)]
pub struct PlayerSocialRecipientSnapshot {
    pub registration: PlayerRegistration,
    pub guid: ObjectGuid,
    pub player_name: String,
    pub race: u8,
    pub map_id: u16,
    pub instance_id: u32,
    pub dungeon_difficulty_id: u32,
    pub is_game_master: bool,
    pub is_afk: bool,
    pub is_dnd: bool,
    pub auto_reply_msg_like_cpp: String,
}

/// Owned presence facts used by Group decisions which depend on a connected
/// Player rather than on Group membership itself.
#[derive(Clone, Copy, Debug)]
pub struct PlayerGroupPresenceSnapshot {
    pub registration: PlayerRegistration,
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub is_in_world: bool,
    pub is_alive: bool,
    pub level: u8,
    pub account_id: u32,
    pub recruiter_id: u32,
    pub in_combat: bool,
    pub has_active_loot_rolls: bool,
}

/// Owned connected-player projection required to build C++ PartyUpdate and
/// PartyMemberFullState payloads outside the session directory.
#[derive(Clone, Debug)]
pub struct PlayerPartyMemberSnapshot {
    pub registration: PlayerRegistration,
    pub guid: ObjectGuid,
    pub player_name: String,
    pub race: u8,
    pub class: u8,
    pub position: Position,
    pub is_pvp: bool,
    pub is_alive: bool,
    pub is_ghost: bool,
    pub is_ffa_pvp: bool,
    pub is_afk: bool,
    pub is_dnd: bool,
    pub in_vehicle: bool,
    pub power_type: u8,
    pub current_health: u32,
    pub max_health: u32,
    pub current_power: u16,
    pub max_power: u16,
    pub level: u8,
    pub spec_id: u32,
    pub zone_id: u32,
    pub party_member_vehicle_seat: i32,
    pub party_member_party_type: [u8; 2],
    pub party_member_phase_states: PartyMemberPhaseStates,
    pub party_member_auras: Vec<PartyMemberAuraState>,
    pub party_member_pet_stats: Option<PartyMemberPetStats>,
}

/// Owned facts required by inspect, honor-inspect and inspect-achievement
/// handlers. Keeping this projection bounded prevents those handlers from
/// depending on the session mirror or directory storage.
#[derive(Clone, Debug)]
pub struct PlayerInspectSnapshot {
    pub guid: ObjectGuid,
    pub map_id: u16,
    pub position: Position,
    pub faction_template_id: u32,
    pub player_name: String,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub visible_items: Arc<[(i32, u16, u16); 19]>,
    pub lifetime_honorable_kills: u32,
    pub this_week_contribution: u32,
    pub yesterday_contribution: u32,
    pub today_honorable_kills: u16,
    pub yesterday_honorable_kills: u16,
    pub lifetime_max_rank: u32,
    pub honor_level: u32,
}

/// Owned player facts required by the legacy creature aggro compatibility cut.
#[derive(Clone, Debug)]
pub struct PlayerAggroCandidateSnapshot {
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub combat_reach: f32,
    pub liquid_status: u32,
    pub level: u8,
    pub gray_level: u8,
    pub unit_flags: u32,
    pub unit_flags2: u32,
    pub unit_state: u32,
    pub is_game_master: bool,
    pub is_contested_pvp: bool,
    pub faction_template_id: u32,
    pub reputation_standings: Vec<(u32, i32)>,
    pub reputation_state_flags: Vec<(u32, u32)>,
    pub forced_reputation_ranks: Vec<(u32, wow_data::reputation::ReputationRankLikeCpp)>,
    pub forced_reputation_faction_ids: Vec<u32>,
}

/// Owned CREATE payload facts for one spatially eligible player.
#[derive(Clone, Debug)]
pub struct PlayerVisibilityCreateSnapshot {
    pub guid: ObjectGuid,
    pub position: Position,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub display_id: u32,
    pub zone_id: u32,
    pub current_health: u32,
    pub max_health: u32,
    pub power_type: u8,
    pub current_power: u16,
    pub max_power: u16,
    pub base_mana: i32,
    pub transport: Option<TransportInfo>,
    pub visible_items: Arc<[(i32, u16, u16); 19]>,
    pub customizations: Arc<Vec<ChrCustomizationChoiceValuesUpdate>>,
    pub party_member_party_type: [u8; 2],
}

/// Private storage record. Consumers receive only owned projections and
/// incarnation-aware addresses from [`PlayerRegistry`].
struct PlayerRegistryEntry {
    generation: u64,
    info: PlayerBroadcastInfo,
    /// Durable loot-money coordination for this incarnation.
    ///
    /// Issue #189 keeps this beside the entry rather than inside
    /// [`PlayerBroadcastInfo`]: it is not a gameplay projection another session
    /// may read, it is the persistence handle the owning session already holds,
    /// resolved here only so a remote looter can address the recipient's
    /// coordinator. It creates no second store and no second authority.
    durable_loot_money: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
}

/// Thread-safe directory of active player sessions, keyed by player GUID.
///
/// Storage is private. The lifecycle API returns only owned registrations,
/// snapshots and channel addresses.
///
/// Generic backing-storage operations are intentionally unavailable outside
/// this owner module:
///
/// ```compile_fail
/// use wow_world::session::directory::PlayerRegistry;
///
/// let registry = PlayerRegistry::new();
/// let _entries = registry.iter();
/// ```
pub struct PlayerRegistry {
    entries: DashMap<ObjectGuid, PlayerRegistryEntry>,
    next_generation: AtomicU64,
}

impl Default for PlayerRegistry {
    fn default() -> Self {
        Self {
            entries: DashMap::new(),
            next_generation: AtomicU64::new(1),
        }
    }
}

impl PlayerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn next_generation(&self) -> u64 {
        self.next_generation.fetch_add(1, Ordering::Relaxed)
    }

    /// Register this connected session, replacing an older incarnation for the
    /// same GUID and returning the identity required for later lifecycle work.
    pub fn register_or_replace(
        &self,
        guid: ObjectGuid,
        info: PlayerBroadcastInfo,
        durable_loot_money: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    ) -> PlayerRegistration {
        let generation = self.next_generation();
        self.entries.insert(
            guid,
            PlayerRegistryEntry {
                generation,
                info,
                durable_loot_money,
            },
        );
        PlayerRegistration { guid, generation }
    }

    /// Clone the entry only when `registration` is still the current session.
    #[must_use]
    pub fn lookup_current(&self, registration: PlayerRegistration) -> Option<PlayerBroadcastInfo> {
        let entry = self.entries.get(&registration.guid)?;
        (entry.generation == registration.generation).then(|| entry.info.clone())
    }

    /// Resolve an owned command address for the current incarnation of `guid`.
    #[must_use]
    pub fn control_address(&self, guid: ObjectGuid) -> Option<PlayerControlAddress> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerControlAddress {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            command_tx: entry.info.command_tx.clone(),
        })
    }

    /// Remove only the exact registration supplied by its owning session.
    /// Returns `true` when that incarnation was still current.
    pub fn unregister(&self, registration: PlayerRegistration) -> bool {
        self.entries
            .remove_if(&registration.guid, |_, entry| {
                entry.generation == registration.generation
            })
            .is_some()
    }

    /// Remove the entry only when it still belongs to this exact control
    /// channel. Session lifecycle uses this when it does not retain the owned
    /// registration token; a replacement always owns a different channel.
    pub fn unregister_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
    ) -> bool {
        self.entries
            .remove_if(&guid, |_, entry| {
                entry.info.command_tx.same_channel(command_tx)
            })
            .is_some()
    }

    /// Snapshot the bounded facts used by runtime recipient selection.
    #[must_use]
    pub fn runtime_recipients(&self) -> Vec<PlayerRuntimeRecipient> {
        self.entries
            .iter()
            .map(|entry| {
                let guid = *entry.key();
                let entry = entry.value();
                PlayerRuntimeRecipient {
                    registration: PlayerRegistration {
                        guid,
                        generation: entry.generation,
                    },
                    guid,
                    map_id: entry.info.map_id,
                    instance_id: entry.info.instance_id,
                    position: entry.info.position,
                    combat_reach: entry.info.combat_reach,
                    liquid_status: entry.info.liquid_status,
                    is_in_world: entry.info.is_in_world,
                    is_alive: entry.info.is_alive,
                    account_id: entry.info.account_id,
                    advanced_combat_logging: entry
                        .info
                        .advanced_combat_logging_enabled_like_cpp
                        .load(Ordering::Relaxed),
                    committed_visibility: entry.info.client_visible_guids_like_cpp.clone(),
                }
            })
            .collect()
    }

    /// Resolve one current runtime recipient without exposing directory storage.
    #[must_use]
    pub fn runtime_recipient(&self, guid: ObjectGuid) -> Option<PlayerRuntimeRecipient> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerRuntimeRecipient {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            guid,
            map_id: entry.info.map_id,
            instance_id: entry.info.instance_id,
            position: entry.info.position,
            combat_reach: entry.info.combat_reach,
            liquid_status: entry.info.liquid_status,
            is_in_world: entry.info.is_in_world,
            is_alive: entry.info.is_alive,
            account_id: entry.info.account_id,
            advanced_combat_logging: entry
                .info
                .advanced_combat_logging_enabled_like_cpp
                .load(Ordering::Relaxed),
            committed_visibility: entry.info.client_visible_guids_like_cpp.clone(),
        })
    }

    /// Resolve one connected chat/social identity by case-insensitive player
    /// name without exposing the directory iterator.
    #[must_use]
    pub fn social_recipient_by_name(
        &self,
        player_name: &str,
    ) -> Option<PlayerSocialRecipientSnapshot> {
        self.entries.iter().find_map(|entry| {
            entry
                .info
                .player_name
                .eq_ignore_ascii_case(player_name)
                .then(|| Self::social_snapshot(entry.key(), entry.value()))
        })
    }

    /// Resolve one connected chat/social identity by GUID.
    #[must_use]
    pub fn social_recipient(&self, guid: ObjectGuid) -> Option<PlayerSocialRecipientSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(Self::social_snapshot(&guid, &entry))
    }

    fn social_snapshot(
        guid: &ObjectGuid,
        entry: &PlayerRegistryEntry,
    ) -> PlayerSocialRecipientSnapshot {
        PlayerSocialRecipientSnapshot {
            registration: PlayerRegistration {
                guid: *guid,
                generation: entry.generation,
            },
            guid: *guid,
            player_name: entry.info.player_name.clone(),
            race: entry.info.race,
            map_id: entry.info.map_id,
            instance_id: entry.info.instance_id,
            dungeon_difficulty_id: entry.info.dungeon_difficulty_id,
            is_game_master: entry.info.is_game_master,
            is_afk: entry.info.is_afk,
            is_dnd: entry.info.is_dnd,
            auto_reply_msg_like_cpp: entry.info.auto_reply_msg_like_cpp.clone(),
        }
    }

    /// Resolve connected presence facts for one Group member.
    #[must_use]
    pub fn group_presence(&self, guid: ObjectGuid) -> Option<PlayerGroupPresenceSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerGroupPresenceSnapshot {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            guid,
            map_id: entry.info.map_id,
            instance_id: entry.info.instance_id,
            position: entry.info.position,
            is_in_world: entry.info.is_in_world,
            is_alive: entry.info.is_alive,
            level: entry.info.level,
            account_id: entry.info.account_id,
            recruiter_id: entry.info.recruiter_id,
            in_combat: entry.info.in_combat,
            has_active_loot_rolls: !entry.info.active_loot_rolls.is_empty(),
        })
    }

    /// Resolve connected Group members in the authoritative input order.
    #[must_use]
    pub fn group_presences_in_order(
        &self,
        member_guids: &[ObjectGuid],
    ) -> Vec<PlayerGroupPresenceSnapshot> {
        member_guids
            .iter()
            .filter_map(|guid| self.group_presence(*guid))
            .collect()
    }

    /// Resolve the connected projection used to build PartyUpdate payloads.
    #[must_use]
    pub fn party_member(&self, guid: ObjectGuid) -> Option<PlayerPartyMemberSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerPartyMemberSnapshot {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            guid,
            player_name: entry.info.player_name.clone(),
            race: entry.info.race,
            class: entry.info.class,
            position: entry.info.position,
            is_pvp: entry.info.is_pvp,
            is_alive: entry.info.is_alive,
            is_ghost: entry.info.is_ghost,
            is_ffa_pvp: entry.info.is_ffa_pvp,
            is_afk: entry.info.is_afk,
            is_dnd: entry.info.is_dnd,
            in_vehicle: entry.info.in_vehicle,
            power_type: entry.info.power_type,
            current_health: entry.info.current_health,
            max_health: entry.info.max_health,
            current_power: entry.info.current_power,
            max_power: entry.info.max_power,
            level: entry.info.level,
            spec_id: entry.info.spec_id,
            zone_id: entry.info.zone_id,
            party_member_vehicle_seat: entry.info.party_member_vehicle_seat,
            party_member_party_type: entry.info.party_member_party_type,
            party_member_phase_states: entry.info.party_member_phase_states.clone(),
            party_member_auras: entry.info.party_member_auras.clone(),
            party_member_pet_stats: entry.info.party_member_pet_stats.clone(),
        })
    }

    /// Resolve connected PartyUpdate projections in authoritative Group order.
    #[must_use]
    pub fn party_members_in_order(
        &self,
        member_guids: &[ObjectGuid],
    ) -> Vec<PlayerPartyMemberSnapshot> {
        member_guids
            .iter()
            .filter_map(|guid| self.party_member(*guid))
            .collect()
    }

    /// Query the temporary connected-session achievement mirror through a
    /// bounded semantic operation.
    #[must_use]
    pub fn connected_player_has_achievement(&self, guid: ObjectGuid, achievement_id: u32) -> bool {
        self.entries
            .get(&guid)
            .is_some_and(|entry| entry.info.completed_achievements.contains(&achievement_id))
    }

    /// Publish PartyMemberData::PartyType only for this exact session control
    /// channel, preventing a stale session from overwriting its replacement.
    pub fn publish_party_type_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        party_type: [u8; 2],
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        if !entry.info.command_tx.same_channel(command_tx) {
            return false;
        }
        entry.info.party_member_party_type = party_type;
        true
    }

    /// Replace the published compatibility mirror only for the exact owning
    /// control channel while retaining its incarnation generation.
    pub fn publish_broadcast_info_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        info: PlayerBroadcastInfo,
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        if !entry.info.command_tx.same_channel(command_tx) {
            return false;
        }
        entry.info = info;
        true
    }

    /// Publish the one combat bit required by immediate group combat gates.
    pub fn publish_in_combat_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        in_combat: bool,
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        if !entry.info.command_tx.same_channel(command_tx) {
            return false;
        }
        entry.info.in_combat = in_combat;
        true
    }

    /// Read the represented unit-state fallback used only when the canonical
    /// map Player is unavailable.
    #[must_use]
    pub fn represented_unit_state(&self, guid: ObjectGuid) -> Option<u32> {
        self.entries.get(&guid).map(|entry| entry.info.unit_state)
    }

    /// Resolve the bounded connected-player view required by inspect handlers.
    #[must_use]
    pub fn inspect_snapshot(&self, guid: ObjectGuid) -> Option<PlayerInspectSnapshot> {
        let entry = self.entries.get(&guid)?;
        let info = &entry.info;
        Some(PlayerInspectSnapshot {
            guid,
            map_id: info.map_id,
            position: info.position,
            faction_template_id: info.faction_template_id,
            player_name: info.player_name.clone(),
            race: info.race,
            class: info.class,
            sex: info.sex,
            level: info.level,
            visible_items: Arc::clone(&info.visible_items),
            lifetime_honorable_kills: info.lifetime_honorable_kills,
            this_week_contribution: info.this_week_contribution,
            yesterday_contribution: info.yesterday_contribution,
            today_honorable_kills: info.today_honorable_kills,
            yesterday_honorable_kills: info.yesterday_honorable_kills,
            lifetime_max_rank: info.lifetime_max_rank,
            honor_level: info.honor_level,
        })
    }

    /// Snapshot only the presence facts used by loot and group-reward gates.
    #[must_use]
    pub fn loot_presence(&self, guid: ObjectGuid) -> Option<PlayerLootPresenceSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerLootPresenceSnapshot {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            guid,
            map_id: entry.info.map_id,
            instance_id: entry.info.instance_id,
            position: entry.info.position,
            is_in_world: entry.info.is_in_world,
        })
    }

    /// Resolve current in-world recipients in one exact map instance.
    #[must_use]
    pub fn same_map_loot_recipients(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Vec<PlayerRegistration> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let info = &entry.value().info;
                (guid != excluded_guid
                    && info.is_in_world
                    && info.map_id == map_id
                    && info.instance_id == instance_id)
                    .then_some(PlayerRegistration {
                        guid,
                        generation: entry.value().generation,
                    })
            })
            .collect()
    }

    /// Resolve one current recipient only when it is in the requested map instance.
    #[must_use]
    pub fn loot_delivery_recipient(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<PlayerRegistration> {
        let entry = self.entries.get(&guid)?;
        (entry.info.map_id == map_id && entry.info.instance_id == instance_id).then_some(
            PlayerRegistration {
                guid,
                generation: entry.generation,
            },
        )
    }

    /// Resolve one current in-world recipient in the requested map instance.
    #[must_use]
    pub fn in_world_loot_delivery_recipient(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<PlayerRegistration> {
        let entry = self.entries.get(&guid)?;
        (entry.info.is_in_world
            && entry.info.map_id == map_id
            && entry.info.instance_id == instance_id)
            .then_some(PlayerRegistration {
                guid,
                generation: entry.generation,
            })
    }

    /// Read one remote enchanting skill without exposing the Player mirror.
    #[must_use]
    pub fn loot_enchanting_skill(&self, guid: ObjectGuid) -> Option<u16> {
        self.entries
            .get(&guid)
            .map(|entry| entry.info.enchanting_skill)
    }

    /// Read one connected peer's C++ `Player::GetPassOnGroupLoot()` state.
    #[must_use]
    pub fn loot_pass_on_group_loot(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<bool> {
        let entry = self.entries.get(&guid)?;
        (entry.info.map_id == map_id && entry.info.instance_id == instance_id)
            .then_some(entry.info.pass_on_group_loot)
    }

    /// Snapshot the exact remote facts used by represented loot conditions.
    #[must_use]
    pub fn loot_player_context(&self, guid: ObjectGuid) -> Option<PlayerLootContextSnapshot> {
        let entry = self.entries.get(&guid)?;
        let info = &entry.info;
        Some(PlayerLootContextSnapshot {
            race: info.race,
            class: info.class,
            sex: info.sex,
            level: info.level,
            known_spells: info.known_spells.clone(),
            active_quest_statuses: info.active_quest_statuses.clone(),
            active_quest_objective_counts: info.active_quest_objective_counts.clone(),
            rewarded_quests: info.rewarded_quests.clone(),
            inventory_item_counts: info.inventory_item_counts.clone(),
        })
    }

    /// Snapshot the exact remote facts used by C++ group reward calculations.
    #[must_use]
    pub fn group_reward_snapshot(&self, guid: ObjectGuid) -> Option<PlayerGroupRewardSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerGroupRewardSnapshot {
            level: entry.info.level,
            map_id: entry.info.map_id,
            position: entry.info.position,
            is_alive: entry.info.is_alive,
        })
    }

    /// Resolve in-world recipients inside one exact map-instance radius.
    #[must_use]
    pub fn movement_recipients_within_range(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
    ) -> Vec<PlayerRegistration> {
        let range_sq = range * range;
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let info = &entry.value().info;
                if guid == excluded_guid
                    || !info.is_in_world
                    || info.map_id != map_id
                    || info.instance_id != instance_id
                {
                    return None;
                }
                let dx = info.position.x - source_position.x;
                let dy = info.position.y - source_position.y;
                (dx * dx + dy * dy <= range_sq).then_some(PlayerRegistration {
                    guid,
                    generation: entry.value().generation,
                })
            })
            .collect()
    }

    /// Resolve every other in-world movement recipient in one map instance.
    #[must_use]
    pub fn same_map_movement_recipients(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Vec<PlayerRegistration> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let info = &entry.value().info;
                (guid != excluded_guid
                    && info.is_in_world
                    && info.map_id == map_id
                    && info.instance_id == instance_id)
                    .then_some(PlayerRegistration {
                        guid,
                        generation: entry.value().generation,
                    })
            })
            .collect()
    }

    /// Resolve spell-pull observers using the represented C++ combat-reach radius.
    #[must_use]
    pub fn spell_pull_recipients(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        source_combat_reach: f32,
        visibility_range: f32,
    ) -> Vec<PlayerRegistration> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let info = &entry.value().info;
                if guid == excluded_guid
                    || !info.is_in_world
                    || info.map_id != map_id
                    || info.instance_id != instance_id
                {
                    return None;
                }
                let dx = info.position.x - source_position.x;
                let dy = info.position.y - source_position.y;
                let reach =
                    visibility_range + source_combat_reach.max(0.0) + info.combat_reach.max(0.0);
                (dx * dx + dy * dy < reach * reach).then_some(PlayerRegistration {
                    guid,
                    generation: entry.value().generation,
                })
            })
            .collect()
    }

    /// Snapshot fellow passengers needed by C++ `Map::SendInitSelf` CREATE blocks.
    #[must_use]
    pub fn fellow_transport_passengers(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        transport_guid: ObjectGuid,
    ) -> Vec<PlayerVisibilityCreateSnapshot> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let info = &entry.value().info;
                if guid == excluded_guid
                    || !info.is_in_world
                    || info.map_id != map_id
                    || info.instance_id != instance_id
                    || !info
                        .transport
                        .as_ref()
                        .is_some_and(|transport| transport.guid == transport_guid)
                {
                    return None;
                }
                Some(PlayerVisibilityCreateSnapshot {
                    guid,
                    position: info.position,
                    race: info.race,
                    class: info.class,
                    sex: info.sex,
                    level: info.level,
                    display_id: info.display_id,
                    zone_id: info.zone_id,
                    current_health: info.current_health,
                    max_health: info.max_health,
                    power_type: info.power_type,
                    current_power: info.current_power,
                    max_power: info.max_power,
                    base_mana: info.base_mana,
                    transport: info.transport.clone(),
                    visible_items: Arc::clone(&info.visible_items),
                    customizations: Arc::clone(&info.customizations),
                    party_member_party_type: info.party_member_party_type,
                })
            })
            .collect()
    }

    /// Read one connected player's active status for an exact shared quest.
    #[must_use]
    pub fn quest_active_status(&self, guid: ObjectGuid, quest_id: u32) -> Option<Option<u8>> {
        let entry = self.entries.get(&guid)?;
        Some(entry.info.active_quest_statuses.get(&quest_id).copied())
    }

    /// Snapshot the exact receiver facts used by represented quest sharing.
    /// The projection cannot grow outside the retirement ledger in issue #196.
    #[must_use]
    pub fn quest_sharing_snapshot(&self, guid: ObjectGuid) -> Option<PlayerQuestSharingSnapshot> {
        let entry = self.entries.get(&guid)?;
        let info = &entry.info;
        Some(PlayerQuestSharingSnapshot {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            pending_quest_sharing: info.pending_quest_sharing,
            is_alive: info.is_alive,
            rewarded_quests: info.rewarded_quests.clone(),
            active_quest_statuses: info.active_quest_statuses.clone(),
            df_quests: info.df_quests.clone(),
            daily_quests_completed: info.daily_quests_completed.clone(),
            level: info.level,
            class: info.class,
            race: info.race,
            reputation_standings: info.reputation_standings.clone(),
            active_expansion: info.active_expansion,
        })
    }

    /// Read one connected player's race for PvP quest-credit team comparison.
    #[must_use]
    pub fn quest_credit_race(&self, guid: ObjectGuid) -> Option<u8> {
        self.entries.get(&guid).map(|entry| entry.info.race)
    }

    /// Snapshot one player target for represented vehicle interaction.
    #[must_use]
    pub fn vehicle_interaction_snapshot(
        &self,
        guid: ObjectGuid,
    ) -> Option<PlayerVehicleInteractionSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerVehicleInteractionSnapshot {
            map_id: entry.info.map_id,
            instance_id: entry.info.instance_id,
            position: entry.info.position,
            has_vehicle_kit: entry.info.has_vehicle_kit_like_cpp,
        })
    }

    /// Refresh movement mirrors only for the session that owns the control channel.
    pub fn publish_movement_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        update: PlayerMovementDirectoryUpdate,
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        if !entry.info.command_tx.same_channel(command_tx) {
            return false;
        }
        entry.info.position = update.position;
        entry.info.map_id = update.map_id;
        entry.info.instance_id = update.instance_id;
        entry.info.liquid_status = update.liquid_status;
        entry.info.transport = update.transport;
        true
    }

    /// Find the exact live loot-roll identity owned by another map peer.
    #[must_use]
    pub fn loot_roll_owner(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
    ) -> Option<(PlayerRegistration, LootRollCommandIdentityLikeCpp)> {
        self.entries.iter().find_map(|entry| {
            let guid = *entry.key();
            let info = &entry.value().info;
            if guid == excluded_guid || info.map_id != map_id || info.instance_id != instance_id {
                return None;
            }
            let identity = info
                .active_loot_rolls
                .iter()
                .find(|identity| identity.matches_key_like_cpp(loot_obj, loot_list_id))?
                .clone();
            Some((
                PlayerRegistration {
                    guid,
                    generation: entry.value().generation,
                },
                identity,
            ))
        })
    }

    /// Publish the current session's represented C++ `Player::m_lootRolls` identities.
    pub fn replace_loot_rolls_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        identities: Vec<LootRollCommandIdentityLikeCpp>,
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        if !entry.info.command_tx.same_channel(command_tx) {
            return false;
        }
        entry.info.active_loot_rolls = identities;
        true
    }

    /// Prepare a remote loot-money application without exposing its persistence tracker.
    #[must_use]
    pub fn prepare_loot_money_application(
        &self,
        input: PrepareLootMoneyApplicationLikeCpp,
    ) -> Option<PreparedLootMoneyApplicationLikeCpp> {
        let entry = self.entries.get(&input.recipient)?;
        Some(PreparedLootMoneyApplicationLikeCpp {
            registration: PlayerRegistration {
                guid: input.recipient,
                generation: entry.generation,
            },
            command: ApplyLootMoneyLikeCppCommand {
                recipient: input.recipient,
                loot_owner: input.loot_owner,
                loot_obj: input.loot_obj,
                amount: input.amount,
                durable_applied_amount: input.durable_applied_amount,
                durable_persistence_tracker: Arc::clone(&entry.durable_loot_money),
                sole_looter: input.sole_looter,
                authority: input.authority,
                authority_generation: input.authority_generation,
                authority_committed: input.authority_committed,
                send_coin_removed: input.send_coin_removed,
                applied: input.applied,
                published: input.published,
            },
        })
    }

    /// Snapshot only live player facts needed by the legacy aggro compatibility cut.
    #[must_use]
    pub fn legacy_aggro_candidates(&self) -> Vec<PlayerAggroCandidateSnapshot> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let entry = entry.value();
                let info = &entry.info;
                (info.is_in_world && info.is_alive).then(|| PlayerAggroCandidateSnapshot {
                    player_guid: guid,
                    map_id: info.map_id,
                    instance_id: info.instance_id,
                    position: info.position,
                    combat_reach: info.combat_reach,
                    liquid_status: info.liquid_status,
                    level: info.level,
                    gray_level: info.gray_level,
                    unit_flags: info.unit_flags,
                    unit_flags2: info.unit_flags2,
                    unit_state: info.unit_state,
                    is_game_master: info.is_game_master,
                    is_contested_pvp: info.is_contested_pvp,
                    faction_template_id: info.faction_template_id,
                    reputation_standings: info.reputation_standings.clone(),
                    reputation_state_flags: info.reputation_state_flags.clone(),
                    forced_reputation_ranks: info.forced_reputation_ranks.clone(),
                    forced_reputation_faction_ids: info.forced_reputation_faction_ids.clone(),
                })
            })
            .collect()
    }

    /// Select player CREATE candidates by directory-owned presence and spatial facts.
    #[must_use]
    pub fn player_visibility_create_candidates(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        source_combat_reach: f32,
        visibility_radius: f32,
    ) -> Vec<PlayerVisibilityCreateSnapshot> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let entry = entry.value();
                let info = &entry.info;
                if guid == excluded_guid
                    || !info.is_in_world
                    || info.map_id != map_id
                    || info.instance_id != instance_id
                {
                    return None;
                }
                let dx = info.position.x - source_position.x;
                let dy = info.position.y - source_position.y;
                let reach =
                    visibility_radius + source_combat_reach.max(0.0) + info.combat_reach.max(0.0);
                if dx * dx + dy * dy >= reach * reach {
                    return None;
                }
                Some(PlayerVisibilityCreateSnapshot {
                    guid,
                    position: info.position,
                    race: info.race,
                    class: info.class,
                    sex: info.sex,
                    level: info.level,
                    display_id: info.display_id,
                    zone_id: info.zone_id,
                    current_health: info.current_health,
                    max_health: info.max_health,
                    power_type: info.power_type,
                    current_power: info.current_power,
                    max_power: info.max_power,
                    base_mana: info.base_mana,
                    transport: info.transport.clone(),
                    visible_items: Arc::clone(&info.visible_items),
                    customizations: Arc::clone(&info.customizations),
                    party_member_party_type: info.party_member_party_type,
                })
            })
            .collect()
    }

    /// Queue a command only if the selected incarnation is still current.
    pub fn try_send_current_command(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.info.command_tx.clone();
        drop(entry);
        tx.try_send(command).map_err(|error| match error {
            flume::TrySendError::Full(_) => PlayerDirectorySendError::Full,
            flume::TrySendError::Disconnected(_) => PlayerDirectorySendError::Disconnected,
        })
    }

    /// Queue packet bytes on the normal socket only for the current incarnation.
    pub fn try_send_current_packet(
        &self,
        registration: PlayerRegistration,
        packet: Vec<u8>,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.info.send_tx.clone();
        drop(entry);
        tx.try_send(packet).map_err(|error| match error {
            flume::TrySendError::Full(_) => PlayerDirectorySendError::Full,
            flume::TrySendError::Disconnected(_) => PlayerDirectorySendError::Disconnected,
        })
    }

    /// Send packet bytes on the normal socket for the selected incarnation.
    pub fn send_current_packet(
        &self,
        registration: PlayerRegistration,
        packet: Vec<u8>,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.info.send_tx.clone();
        drop(entry);
        tx.send(packet)
            .map_err(|_| PlayerDirectorySendError::Disconnected)
    }

    /// Send packet bytes on the realm socket for the selected incarnation.
    pub fn send_current_realm_packet(
        &self,
        registration: PlayerRegistration,
        packet: Vec<u8>,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.info.realm_send_tx.clone();
        drop(entry);
        tx.send(packet)
            .map_err(|_| PlayerDirectorySendError::Disconnected)
    }

    /// Wait for command-queue capacity for the selected incarnation.
    pub async fn send_current_command(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.info.command_tx.clone();
        drop(entry);
        tx.send_async(command)
            .await
            .map_err(|_| PlayerDirectorySendError::Disconnected)
    }

    /// Wait for command-queue capacity up to the caller's delivery deadline.
    pub async fn send_current_command_timeout(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
        timeout: std::time::Duration,
    ) -> Result<(), PlayerDirectorySendError> {
        tokio::time::timeout(timeout, self.send_current_command(registration, command))
            .await
            .map_err(|_| PlayerDirectorySendError::Full)?
    }

    /// Blocking timeout variant used by synchronous publication adapters.
    pub fn send_current_command_blocking_timeout(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
        timeout: std::time::Duration,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.info.command_tx.clone();
        drop(entry);
        tx.send_timeout(command, timeout)
            .map_err(|error| match error {
                flume::SendTimeoutError::Timeout(_) => PlayerDirectorySendError::Full,
                flume::SendTimeoutError::Disconnected(_) => PlayerDirectorySendError::Disconnected,
            })
    }

    /// Retain an authoritative command across bounded queue backpressure.
    pub fn queue_current_command_reliably(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> PlayerDirectoryReliableSendOutcome {
        let Some(entry) = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
        else {
            return PlayerDirectoryReliableSendOutcome::StaleOrDisconnected;
        };
        let tx = entry.info.command_tx.clone();
        drop(entry);
        match tx.try_send(command) {
            Ok(()) => PlayerDirectoryReliableSendOutcome::Queued,
            Err(flume::TrySendError::Disconnected(_)) => {
                PlayerDirectoryReliableSendOutcome::StaleOrDisconnected
            }
            Err(flume::TrySendError::Full(command)) => {
                tokio::spawn(async move {
                    let _ = tx.send_async(command).await;
                });
                PlayerDirectoryReliableSendOutcome::Retrying
            }
        }
    }

    fn with_current_durable_runtime(
        &self,
        registration: PlayerRegistration,
    ) -> Option<Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>> {
        let entry = self.entries.get(&registration.guid)?;
        (entry.generation == registration.generation)
            .then(|| Arc::clone(&entry.info.durable_creature_runtime_commands_like_cpp))
    }

    pub fn publish_current_attack_start(
        &self,
        registration: PlayerRegistration,
        command: CreatureAttackStartLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_attack_start_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_attack_stop(
        &self,
        registration: PlayerRegistration,
        command: CreatureAttackStopLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_attack_stop_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_melee_damage(
        &self,
        registration: PlayerRegistration,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_melee_damage_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_send_if_visible(
        &self,
        registration: PlayerRegistration,
        command: SendIfVisibleLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_send_if_visible_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_creature_spell_cast_if_visible(
        &self,
        registration: PlayerRegistration,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable.lock().ok().map(|mut durable| {
                    durable.publish_creature_spell_cast_if_visible_like_cpp(command)
                })
            })
            .unwrap_or(false)
    }

    pub fn publish_current_pvp_combat_expiry(
        &self,
        registration: PlayerRegistration,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable.lock().ok().map(|mut durable| {
                    durable.publish_pvp_combat_expiry_like_cpp(command);
                    true
                })
            })
            .unwrap_or(false)
    }

    /// Coalesce and queue a visibility refresh for a current recipient.
    pub fn request_current_visibility_refresh(
        &self,
        registration: PlayerRegistration,
        map_id: u16,
        instance_id: u32,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let pending = Arc::clone(&entry.info.visibility_refresh_pending_like_cpp);
        let tx = entry.info.command_tx.clone();
        drop(entry);
        pending.store(true, Ordering::Release);
        let command = SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
            RefreshVisibleWorldCreaturesLikeCppCommand {
                map_id,
                instance_id,
            },
        );
        match tx.try_send(command) {
            Ok(()) | Err(flume::TrySendError::Full(_)) => Ok(()),
            Err(flume::TrySendError::Disconnected(_)) => {
                pending.store(false, Ordering::Release);
                Err(PlayerDirectorySendError::Disconnected)
            }
        }
    }

    /// Clone one fixture entry without exposing a storage guard. Available
    /// only when a dependent crate explicitly enables test fixtures.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_snapshot(&self, guid: ObjectGuid) -> Option<PlayerBroadcastInfo> {
        self.entries.get(&guid).map(|entry| entry.info.clone())
    }

    /// Mutate one fixture entry while keeping storage and its generation
    /// private. Production crates cannot enable this capability accidentally.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_update(
        &self,
        guid: ObjectGuid,
        update: impl FnOnce(&mut PlayerBroadcastInfo),
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        update(&mut entry.info);
        true
    }

    /// Remove one fixture registration without exporting its storage record.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_remove(&self, guid: ObjectGuid) -> bool {
        self.entries.remove(&guid).is_some()
    }

    /// Count connected fixture registrations without exposing iteration.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cross-instance delivery can be filtered (Slice 4A.1b).
    /// C++ anchor: `GridNotifiersImpl.h : MessageDistDeliverer::Visit` — instance
    /// separation via `InSamePhase` + map instance ID check.
    #[test]
    fn player_broadcast_info_has_instance_id_field_like_cpp() {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
        let info = PlayerBroadcastInfo {
            map_id: 571,
            instance_id: 42,
            position: Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx,
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
            active_loot_rolls: Vec::new(),
            in_combat: false,
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            current_health: 100,
            max_health: 100,
            power_type: 0,
            current_power: 0,
            max_power: 0,
            base_mana: 0,
            transport: None,
            is_pvp: false,
            is_ffa_pvp: false,
            is_ghost: false,
            is_afk: false,
            is_dnd: false,
            auto_reply_msg_like_cpp: String::new(),
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            dungeon_difficulty_id: 1,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            completed_achievements: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_party_type: [0; 2],
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            player_name: "TestPlayer".to_string(),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: Arc::new([(0, 0, 0); 19]),
            customizations: Arc::default(),
            lifetime_honorable_kills: 0,
            this_week_contribution: 0,
            yesterday_contribution: 0,
            today_honorable_kills: 0,
            yesterday_honorable_kills: 0,
            lifetime_max_rank: 0,
            honor_level: 0,
        };
        assert_eq!(info.instance_id, 42);
        assert_eq!(info.map_id, 571);

        let registry = PlayerRegistry::new();
        let alpha = ObjectGuid::create_player(1, 100);
        let beta = ObjectGuid::create_player(1, 101);
        let first_alpha = registry.register_or_replace(alpha, info.clone(), Default::default());

        let social = registry
            .social_recipient_by_name("testplayer")
            .expect("case-insensitive connected-player lookup");
        assert_eq!(social.registration, first_alpha);
        assert_eq!(social.map_id, 571);
        assert_eq!(social.instance_id, 42);
        assert_eq!(social.dungeon_difficulty_id, 1);

        let mut replacement = info.clone();
        replacement.player_name = "Replacement".to_string();
        let replacement_alpha =
            registry.register_or_replace(alpha, replacement, Default::default());
        assert_eq!(
            registry.send_current_packet(first_alpha, vec![1]),
            Err(PlayerDirectorySendError::StaleRegistration),
            "an older social/group decision must not reach a replacement session"
        );
        assert_eq!(
            registry.social_recipient(alpha).unwrap().registration,
            replacement_alpha
        );

        let mut beta_info = info;
        beta_info.player_name = "Beta".to_string();
        registry.register_or_replace(beta, beta_info, Default::default());
        let ordered =
            registry.group_presences_in_order(&[beta, ObjectGuid::create_player(1, 999), alpha]);
        assert_eq!(
            ordered.iter().map(|member| member.guid).collect::<Vec<_>>(),
            vec![beta, alpha],
            "connected Group projections preserve authoritative member order"
        );
    }
}
