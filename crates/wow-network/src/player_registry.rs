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
use wow_packet::packets::party::PartyMemberPhaseStates;

#[derive(Clone, Debug)]
pub enum SessionCommand {
    ApplyCreatureMeleeDamageLikeCpp(ApplyCreatureMeleeDamageLikeCppCommand),
    CreatureAttackStartLikeCpp(CreatureAttackStartLikeCppCommand),
    MasterLootGive(MasterLootGiveCommand),
    LootRollStoreWinner(LootRollStoreWinnerCommand),
    LootRollVote(LootRollVoteCommand),
    ResetSeasonalQuestStatus(ResetSeasonalQuestStatusCommand),
    SendVisibleObjectValuesUpdate(SendVisibleObjectValuesUpdateCommand),
    RefreshVisibleWorldCreaturesLikeCpp(RefreshVisibleWorldCreaturesLikeCppCommand),
    RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
    SetQuestSharingInfoAndSendDetails(SetQuestSharingInfoAndSendDetailsCommand),
    SendRepeatableTurnInRequestItemsLikeCpp(SendRepeatableTurnInRequestItemsLikeCppCommand),
    /// Deliver `packet_bytes` to this session if the source GUID is currently in
    /// `client_visible_guids_like_cpp` (HaveAtClient gate).
    ///
    /// Mirrors C++ `GridNotifiers.h : MessageDistDeliverer::SendPacket` /
    /// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)`.
    /// Routing is performed by `resolve_runtime_event_candidates_like_cpp` in
    /// world-server; the per-session gate is in
    /// `handle_send_if_visible_like_cpp_command_like_cpp` (Slice 4A.1b).
    SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand),
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
    /// Represented `Unit::GetUnitFlags()` snapshot for global creature targetability gates.
    pub unit_flags: u32,
    /// Represented `Unit::GetUnitState()` snapshot for fake-death/unattackable targetability gates.
    pub unit_state: u32,
    /// Represented `Player::IsGameMaster()` snapshot; C++ rejects GM players as attack targets.
    pub is_game_master: bool,
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
    /// Represented current reputation standing by faction for remote `SatisfyQuestReputation`.
    /// Missing factions are interpreted as standing 0 like C++ no-state path.
    pub reputation_standings: Vec<(u32, i32)>,
    /// Direct inventory item counts, keyed by item entry, used for remote quest-loot gates.
    pub inventory_item_counts: HashMap<u32, u32>,
    /// C++ `PartyMemberPhaseStates` snapshot for SMSG_PARTY_MEMBER_FULL_STATE.
    pub party_member_phase_states: PartyMemberPhaseStates,
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
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            unit_flags: 0,
            unit_state: 0,
            is_game_master: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            reputation_standings: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_phase_states: Default::default(),
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
