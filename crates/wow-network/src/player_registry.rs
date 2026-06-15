// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared registry of active player sessions for broadcast purposes.
//!
//! Each WorldSession registers itself here on player login and removes itself
//! on logout/disconnect. Chat, emote and movement handlers use the registry
//! to fan-out packets to nearby players on the same map.

use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use wow_core::{ObjectGuid, Position};
use wow_packet::packets::loot::LootEntry;
use wow_packet::packets::party::{
    PartyMemberAuraState, PartyMemberPetStats, PartyMemberPhaseStates, PartyUpdate,
};

#[derive(Clone, Debug)]
pub enum SessionCommand {
    KickLikeCpp(KickLikeCppCommand),
    WorldSessionShutdownFlushLikeCpp(WorldSessionShutdownFlushLikeCppCommand),
    ApplyCreatureMeleeDamageLikeCpp(ApplyCreatureMeleeDamageLikeCppCommand),
    CreatureAttackStartLikeCpp(CreatureAttackStartLikeCppCommand),
    MasterLootGive(MasterLootGiveCommand),
    LootRollStoreWinner(LootRollStoreWinnerCommand),
    LootRollVote(LootRollVoteCommand),
    ResetSeasonalQuestStatus(ResetSeasonalQuestStatusCommand),
    SendVisibleObjectValuesUpdate(SendVisibleObjectValuesUpdateCommand),
    RefreshVisibleWorldCreaturesLikeCpp(RefreshVisibleWorldCreaturesLikeCppCommand),
    RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
        SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    ),
    SyncChestGameobjectStateAndRefreshLikeCpp(SyncChestGameobjectStateAndRefreshLikeCppCommand),
    SyncGooberGameobjectStateAndRefreshLikeCpp(SyncGooberGameobjectStateAndRefreshLikeCppCommand),
    SetQuestSharingInfoAndSendDetails(SetQuestSharingInfoAndSendDetailsCommand),
    SendRepeatableTurnInRequestItemsLikeCpp(SendRepeatableTurnInRequestItemsLikeCppCommand),
    /// Deliver `PartyUpdate` from the receiver's own session so C++
    /// `Player::NextGroupUpdateSequenceNumber` is consumed per player.
    SendPartyUpdateLikeCpp(SendPartyUpdateLikeCppCommand),
    /// Apply C++ `Group::Disband`/`Group::RemoveMember` session-local cleanup
    /// for a connected remote member.
    ApplyGroupRemovalLikeCpp(ApplyGroupRemovalLikeCppCommand),
    /// Apply C++ `Group::Set*DifficultyID` session-local effects for a
    /// connected group member.
    ApplyGroupDifficultyLikeCpp(ApplyGroupDifficultyLikeCppCommand),
    /// Apply C++ `Player::SetGroup(group, subgroup)` session-local subgroup
    /// reference update for a connected group member.
    ApplyGroupSubgroupLikeCpp(ApplyGroupSubgroupLikeCppCommand),
    /// Deliver `packet_bytes` to this session if the source GUID is currently in
    /// `client_visible_guids_like_cpp` (HaveAtClient gate).
    ///
    /// Mirrors C++ `GridNotifiers.h : MessageDistDeliverer::SendPacket` /
    /// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)`.
    /// Routing is performed by `resolve_runtime_event_candidates_like_cpp` in
    /// world-server; the per-session gate is in
    /// `handle_send_if_visible_like_cpp_command_like_cpp` (Slice 4A.1b).
    SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand),
    /// Deliver an already-built addon chat packet only if this session accepts
    /// the addon prefix.
    ///
    /// Mirrors C++ `WorldSession::IsAddonRegistered(prefix)` used by
    /// `Group::BroadcastAddonMessagePacket` and `Player::WhisperAddon`.
    SendAddonIfRegisteredLikeCpp(SendAddonIfRegisteredLikeCppCommand),
    /// Deliver a represented trade-cancel status to the partner session and
    /// clear its represented `m_trade` equivalent.
    CancelRepresentedTradeLikeCpp(CancelRepresentedTradeLikeCppCommand),
    /// Deliver a represented trade status to the partner session without
    /// changing the bounded active-trade ownership.
    SendRepresentedTradeStatusLikeCpp(SendRepresentedTradeStatusLikeCppCommand),
    /// Clear the receiver's represented trade acceptance and send the already
    /// serialized `TRADE_STATUS_UNACCEPTED` packet without cancelling trade.
    UnacceptRepresentedTradeLikeCpp(UnacceptRepresentedTradeLikeCppCommand),
    /// Deliver represented `SMSG_DUEL_COUNTDOWN` to the remote duel participant.
    SendRepresentedDuelCountdownLikeCpp(SendRepresentedDuelCountdownLikeCppCommand),
}

/// Cross-session kick command mirroring C++ callers such as `World::BanAccount`
/// that locate another account session and call `WorldSession::KickPlayer`.
#[derive(Clone, Debug)]
pub struct KickLikeCppCommand {
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplyGroupSubgroupLikeCppCommand {
    pub group_guid: u64,
    pub subgroup: u8,
}

/// Acknowledgement request used by Rust's shutdown bridge after queuing
/// `World::KickAll`.
///
/// C++ `World::UpdateSessions(1)` owns and ticks every `WorldSession`
/// synchronously after `KickAll`. Rust sessions are still task-owned, so the
/// world server uses this command as a bounded flush point: when a session
/// drains it, all earlier `SessionCommand`s in the same channel have been
/// observed.
#[derive(Clone, Debug)]
pub struct WorldSessionShutdownFlushLikeCppCommand {
    pub diff_ms: u32,
    pub response_tx: flume::Sender<WorldSessionShutdownFlushResultLikeCpp>,
}

/// Result returned to the world server when a session observes a shutdown
/// flush command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSessionShutdownFlushResultLikeCpp {
    pub diff_ms: u32,
    pub disconnecting: bool,
}

/// Payload for C++ `Group::SendUpdateToPlayer`.
#[derive(Clone, Debug)]
pub struct SendPartyUpdateLikeCppCommand {
    pub party_update: PartyUpdate,
    pub member_full_state_packets: Vec<Vec<u8>>,
}

/// Payload for [`SessionCommand::ApplyGroupRemovalLikeCpp`].
///
/// C++ `Group::RemoveMember` and `Group::Disband` mutate the connected
/// player's own `Player` object (`SetGroup(nullptr)` / `SetPartyType(NONE)`)
/// before sending destroy/uninvite packets. Rust sessions are task-owned, so a
/// group mutation performed by one session must ask each affected remote
/// session to apply its own represented cleanup.
#[derive(Clone, Debug)]
pub struct ApplyGroupRemovalLikeCppCommand {
    pub group_guid: u64,
    pub category: u8,
    pub party_type: u8,
    pub send_group_destroyed: bool,
    pub send_group_uninvite: bool,
    pub refresh_visible_gameobjects_or_spellclicks: bool,
}

/// Payload for [`SessionCommand::ApplyGroupDifficultyLikeCpp`].
#[derive(Clone, Debug)]
pub struct ApplyGroupDifficultyLikeCppCommand {
    pub group_guid: u64,
    pub difficulty_id: u32,
    pub kind: GroupDifficultyKindLikeCpp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupDifficultyKindLikeCpp {
    Dungeon,
    Raid,
    LegacyRaid,
}

/// Payload for a map-owned creature melee hit against one player session.
///
/// Future global creature combat will compute the swing once from the map tick,
/// set the canonical player health to `victim_health_after`, then enqueue this
/// command to the victim's session. The session side is idempotent: it sets the
/// represented player health to the final value and sends one combat packet.
#[derive(Clone, Debug)]
pub struct ApplyCreatureMeleeDamageLikeCppCommand {
    pub attacker_guid: ObjectGuid,
    pub victim_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub damage: u32,
    pub over_damage: i32,
    pub target_level: u8,
    pub victim_health_after: u64,
}

/// Payload for a map-owned creature aggro transition against one player.
///
/// The global creature runtime computes the `MoveInLineOfSight`/aggro result
/// once from map state, then sends this command to the victim session so the
/// client receives one `SMSG_ATTACKSTART` and the session mirrors combat state.
#[derive(Clone, Debug)]
pub struct CreatureAttackStartLikeCppCommand {
    pub attacker_guid: ObjectGuid,
    pub victim_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
}

/// Payload for [`SessionCommand::SendIfVisibleLikeCpp`].
///
/// Carries both `map_id` and `instance_id` so the per-session gate can reject
/// cross-instance delivery without touching the canonical map manager.
#[derive(Clone, Debug)]
pub struct SendIfVisibleLikeCppCommand {
    /// GUID of the entity that emitted the packet — checked against
    /// `client_visible_guids_like_cpp` (C++ `HaveAtClient`).
    pub source_guid: ObjectGuid,
    /// Map the packet was generated on; must match `player_map_id_like_cpp()`.
    pub map_id: u16,
    /// Instance within that map; must match the session's canonical instance.
    /// 0 = world/default instance.
    pub instance_id: u32,
    /// Already-serialised wire payload ready to write to the socket.
    pub packet_bytes: Vec<u8>,
}

/// Payload for [`SessionCommand::SendAddonIfRegisteredLikeCpp`].
#[derive(Clone, Debug)]
pub struct SendAddonIfRegisteredLikeCppCommand {
    /// Addon prefix checked by the receiver's session-local registration list.
    pub prefix: String,
    /// Already-serialised `SMSG_CHAT` addon payload.
    pub packet_bytes: Vec<u8>,
}

/// Payload for [`SessionCommand::CancelRepresentedTradeLikeCpp`].
///
/// C++ `Player::TradeCancel` sends `SMSG_TRADE_STATUS` to both participants
/// and deletes both `m_trade` objects. Rust does not have full `TradeData` yet,
/// so this command carries the already-serialized status packet and asks the
/// partner session to clear its bounded represented active-trade state.
#[derive(Clone, Debug)]
pub struct CancelRepresentedTradeLikeCppCommand {
    pub status: u8,
    pub packet_bytes: Vec<u8>,
}

/// Payload for [`SessionCommand::SendRepresentedTradeStatusLikeCpp`].
///
/// Used by bounded trade handshakes such as `CMSG_BEGIN_TRADE` where C++ sends
/// `SMSG_TRADE_STATUS` to both sessions but keeps both `m_trade` objects alive.
#[derive(Clone, Debug)]
pub struct SendRepresentedTradeStatusLikeCppCommand {
    pub packet_bytes: Vec<u8>,
}

/// Payload for [`SessionCommand::UnacceptRepresentedTradeLikeCpp`].
///
/// C++ `TradeData::SetItem` calls `GetTraderData()->SetAccepted(false)` after
/// a local trade item change. Full `TradeData` is not wired yet, so this
/// bounded command mirrors the acceptance/status side effect on the partner.
#[derive(Clone, Debug)]
pub struct UnacceptRepresentedTradeLikeCppCommand {
    pub packet_bytes: Vec<u8>,
}

/// Payload for [`SessionCommand::SendRepresentedDuelCountdownLikeCpp`].
///
/// C++ `WorldSession::HandleDuelAccepted` sends the same
/// `SMSG_DUEL_COUNTDOWN` packet to both duel participants after moving both
/// duel records to `DUEL_STATE_COUNTDOWN`.
#[derive(Clone, Debug)]
pub struct SendRepresentedDuelCountdownLikeCppCommand {
    pub packet_bytes: Vec<u8>,
}

/// Requests a per-session creature visibility recomputation.
///
/// Used by the global creature runtime path when map-owned creature state
/// changed in a way that may require CREATE/DESTROY visibility deltas. Unlike
/// [`SendIfVisibleLikeCppCommand`], this is allowed to update the session's
/// `client_visible_guids_like_cpp` set by reusing the session visibility pass
/// (`Player::UpdateVisibilityOf` seam).
#[derive(Clone, Debug)]
pub struct RefreshVisibleWorldCreaturesLikeCppCommand {
    pub map_id: u16,
    pub instance_id: u32,
}

/// Syncs the bounded represented gathering-node state needed before running a
/// remote `UpdateVisibleGameobjectsOrSpellClicks` refresh.
///
/// C++ owns this state on the shared `GameObject`. Rust's represented runtime
/// still stores this subset per session, so the current bridge must carry the
/// changed fields to the receiver before asking it to recompute viewer-dependent
/// dynamic flags.
#[derive(Clone, Debug)]
pub struct SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
    pub gameobject_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub go_type: u8,
    pub loot_state: Option<u8>,
    pub loot_state_unit_guid: ObjectGuid,
    pub go_state: Option<i8>,
    pub dynamic_flags: u32,
    pub gathering_node_loot_id: Option<u32>,
    pub personal_loot_uses: u32,
    pub linked_trap_entry: Option<u32>,
    pub linked_trap_guid: Option<ObjectGuid>,
}

/// Syncs the bounded represented chest state needed before running a remote
/// `UpdateVisibleGameobjectsOrSpellClicks` refresh.
#[derive(Clone, Debug)]
pub struct SyncChestGameobjectStateAndRefreshLikeCppCommand {
    pub gameobject_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub go_type: u8,
    pub loot_state: Option<u8>,
    pub loot_state_unit_guid: ObjectGuid,
    pub chest_loot_id: u32,
    pub chest_personal_loot_id: u32,
    pub chest_push_loot_id: u32,
    pub chest_quest_id: u32,
    pub chest_restock_time_secs: u32,
    pub chest_consumable: bool,
    pub linked_trap_entry: Option<u32>,
    pub linked_trap_guid: Option<ObjectGuid>,
}

/// Syncs the bounded represented goober state needed before running a remote
/// `UpdateVisibleGameobjectsOrSpellClicks` refresh.
#[derive(Clone, Debug)]
pub struct SyncGooberGameobjectStateAndRefreshLikeCppCommand {
    pub gameobject_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub go_type: u8,
    pub gameobject_flags: u32,
    pub loot_state: Option<u8>,
    pub loot_state_unit_guid: ObjectGuid,
    pub go_state: Option<i8>,
    pub dynamic_flags: u32,
    pub linked_trap_entry: Option<u32>,
    pub linked_trap_guid: Option<ObjectGuid>,
}

#[derive(Clone, Debug)]
pub struct SendVisibleObjectValuesUpdateCommand {
    pub object_guid: ObjectGuid,
    pub map_id: u16,
    pub packet_bytes: Vec<u8>,
    pub unit_values_update: Option<wow_packet::packets::update::UnitDataValuesDeltaUpdate>,
}

#[derive(Clone, Debug)]
pub struct SetQuestSharingInfoAndSendDetailsCommand {
    pub sender_guid: ObjectGuid,
    pub quest: wow_data::quest::QuestTemplate,
}

#[derive(Clone, Debug)]
pub struct SendRepeatableTurnInRequestItemsLikeCppCommand {
    pub sender_guid: ObjectGuid,
    pub quest: wow_data::quest::QuestTemplate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetSeasonalQuestStatusCommand {
    pub event_id: u16,
    pub event_start_time: u64,
}

#[derive(Clone, Debug)]
pub struct GameEventQuestCompleteCommandLikeCpp {
    pub quest_id: u32,
    pub response_tx: flume::Sender<GameEventQuestCompleteResponseLikeCpp>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameEventQuestCompleteResponseLikeCpp {
    pub quest_id: u32,
    pub condition_save_updates_queued: usize,
    pub condition_save_updates_executed: usize,
    pub condition_save_updates_failed: usize,
    pub condition_save_updates_skipped_non_progress: usize,
    pub save_world_event_state_requested: bool,
    pub world_event_state_save_requested: usize,
    pub world_event_state_saves_queued: usize,
    pub world_event_state_saves_executed: usize,
    pub world_event_state_saves_failed: usize,
    pub world_event_state_saves_skipped_event_id_out_of_range: usize,
    pub world_event_state_saves_skipped_missing_event: usize,
    pub force_game_event_update_requested: bool,
    pub force_game_event_update_requests: usize,
    pub processor_failed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameEventQuestCompleteClientOutcomeLikeCpp {
    Ok(GameEventQuestCompleteResponseLikeCpp),
    SenderMissing { quest_id: u32 },
    SendFailed { quest_id: u32 },
    ResponseTimeout { quest_id: u32 },
    ResponseChannelClosed { quest_id: u32 },
}

#[derive(Clone, Debug)]
pub struct MasterLootGiveCommand {
    pub master_guid: ObjectGuid,
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub dungeon_encounter_id: u32,
    pub entry: LootEntry,
    pub result_tx: flume::Sender<MasterLootGiveResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MasterLootGiveResult {
    Stored,
    StoreFailed(u8),
    TargetMismatch,
}

#[derive(Clone, Debug)]
pub struct LootRollStoreWinnerCommand {
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub dungeon_encounter_id: u32,
    pub entry: LootEntry,
    pub result_tx: flume::Sender<MasterLootGiveResult>,
}

#[derive(Clone, Debug)]
pub struct LootRollVoteCommand {
    pub voter_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub roll_type: u8,
    pub pass_on_group_loot: bool,
}

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
    /// Represented C++ `Unit::GetLiquidStatus()` snapshot for remote accessibility gates.
    pub liquid_status: u32,
    /// Represented C++ `Player::IsInWorld()` receiver gate for global-message fanout.
    pub is_in_world: bool,
    /// Channel used to push serialised packets to this player's socket.
    pub send_tx: flume::Sender<Vec<u8>>,
    /// Channel used for C++-style cross-session state mutations.
    pub command_tx: flume::Sender<SessionCommand>,
    /// Represented pending loot-roll keys owned by this session.
    pub active_loot_rolls: Vec<(ObjectGuid, u8)>,
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
    pub visible_items: [(i32, u16, u16); 19],
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

/// Thread-safe registry of all active player sessions, keyed by player GUID.
///
/// Wrap in `Arc` and share between all `WorldSession` instances and the
/// `SessionResources` passed to `create_session`.
pub type PlayerRegistry = DashMap<ObjectGuid, PlayerBroadcastInfo>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `PlayerBroadcastInfo` carries `instance_id` so that
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
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            current_health: 100,
            max_health: 100,
            power_type: 0,
            current_power: 0,
            max_power: 0,
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
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
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
            visible_items: [(0, 0, 0); 19],
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
    }

    /// Verify that `SendIfVisibleLikeCppCommand` carries both `map_id` and
    /// `instance_id` — required so per-session gate can reject cross-instance
    /// delivery (Slice 4A.1b).
    #[test]
    fn send_if_visible_like_cpp_command_carries_map_and_instance_id() {
        let guid = ObjectGuid::create_player(1, 7);
        let cmd = SendIfVisibleLikeCppCommand {
            source_guid: guid,
            map_id: 532,
            instance_id: 99,
            packet_bytes: vec![0xDE, 0xAD],
        };
        assert_eq!(cmd.map_id, 532);
        assert_eq!(cmd.instance_id, 99);
    }

    /// Verify that creature visibility refresh commands are scoped by both map
    /// and instance. The receiving `WorldSession` applies the same gates before
    /// forcing its visibility pass.
    #[test]
    fn refresh_visible_world_creatures_like_cpp_command_carries_map_and_instance_id() {
        let cmd = RefreshVisibleWorldCreaturesLikeCppCommand {
            map_id: 571,
            instance_id: 7,
        };
        assert_eq!(cmd.map_id, 571);
        assert_eq!(cmd.instance_id, 7);
    }

    /// Verify the creature melee damage command carries both addressing data
    /// and final victim health so session delivery can be idempotent.
    #[test]
    fn apply_creature_melee_damage_like_cpp_command_carries_final_health() {
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            123,
            456,
        );
        let victim = ObjectGuid::create_player(1, 7);
        let cmd = ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 3,
            damage: 11,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 89,
        };

        assert_eq!(cmd.attacker_guid, attacker);
        assert_eq!(cmd.victim_guid, victim);
        assert_eq!(cmd.map_id, 571);
        assert_eq!(cmd.instance_id, 3);
        assert_eq!(cmd.victim_health_after, 89);
    }

    #[test]
    fn creature_attack_start_like_cpp_command_carries_map_and_instance_id() {
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            123,
            457,
        );
        let victim = ObjectGuid::create_player(1, 8);
        let cmd = CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 4,
        };

        assert_eq!(cmd.attacker_guid, attacker);
        assert_eq!(cmd.victim_guid, victim);
        assert_eq!(cmd.map_id, 571);
        assert_eq!(cmd.instance_id, 4);
    }
}
