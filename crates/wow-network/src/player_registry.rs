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
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64},
};
use std::time::Instant;
use wow_core::{ObjectGuid, Position};
use wow_loot::{LootClaimLease, OwnedLootAuthority};
use wow_packet::packets::loot::LootEntry;
use wow_packet::packets::movement::TransportInfo;
use wow_packet::packets::party::{
    PartyMemberAuraState, PartyMemberPetStats, PartyMemberPhaseStates, PartyUpdate,
};
use wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate;

/// C++ `Player::m_clientGUIDs` held behind a shared handle.
///
/// The owning session is the only writer, but recipient selection for durable
/// fan-out runs on the map tick, which has no access to session-local state.
/// Publishing the membership through the player registry lets a producer commit
/// the recipient decision at the moment the message is resolved instead of
/// re-deriving it from mutable state when the receiving session drains.
#[derive(Clone, Default)]
pub struct SharedClientVisibleGuidsLikeCpp {
    inner: Arc<std::sync::RwLock<HashSet<ObjectGuid>>>,
}

impl SharedClientVisibleGuidsLikeCpp {
    fn read_like_cpp(&self) -> std::sync::RwLockReadGuard<'_, HashSet<ObjectGuid>> {
        self.inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_like_cpp(&self) -> std::sync::RwLockWriteGuard<'_, HashSet<ObjectGuid>> {
        self.inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn insert(&self, guid: ObjectGuid) -> bool {
        self.write_like_cpp().insert(guid)
    }

    pub fn remove(&self, guid: &ObjectGuid) -> bool {
        self.write_like_cpp().remove(guid)
    }

    pub fn contains(&self, guid: &ObjectGuid) -> bool {
        self.read_like_cpp().contains(guid)
    }

    pub fn len(&self) -> usize {
        self.read_like_cpp().len()
    }

    pub fn is_empty(&self) -> bool {
        self.read_like_cpp().is_empty()
    }

    pub fn clear(&self) {
        self.write_like_cpp().clear();
    }

    pub fn retain(&self, mut keep: impl FnMut(&ObjectGuid) -> bool) {
        self.write_like_cpp().retain(|guid| keep(guid));
    }

    pub fn extend(&self, guids: impl IntoIterator<Item = ObjectGuid>) {
        self.write_like_cpp().extend(guids);
    }

    /// Drop the objects a visibility refresh no longer sees and add the ones it
    /// found, under a single write.
    ///
    /// Removing and re-adding in separate steps would briefly publish a
    /// half-rebuilt set, and a producer selecting recipients inside that window
    /// would skip a viewer that never actually lost the object.
    pub fn retain_and_extend_like_cpp(
        &self,
        mut keep: impl FnMut(&ObjectGuid) -> bool,
        added: impl IntoIterator<Item = ObjectGuid>,
    ) {
        let mut guard = self.write_like_cpp();
        guard.retain(|guid| keep(guid));
        guard.extend(added);
    }

    /// Copy the current membership. Callers that need to iterate must take this
    /// snapshot rather than hold the lock across session work.
    pub fn snapshot_like_cpp(&self) -> HashSet<ObjectGuid> {
        self.read_like_cpp().clone()
    }

    /// Whether both handles are the same allocation, and therefore the same
    /// session incarnation. A relogin or a replaced session builds a new set,
    /// so a command committed against the previous one cannot be honored.
    pub fn shares_storage_like_cpp(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl PartialEq for SharedClientVisibleGuidsLikeCpp {
    fn eq(&self, other: &Self) -> bool {
        self.shares_storage_like_cpp(other)
    }
}

impl Eq for SharedClientVisibleGuidsLikeCpp {}

impl std::fmt::Debug for SharedClientVisibleGuidsLikeCpp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedClientVisibleGuidsLikeCpp")
            .field("len", &self.len())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub enum SessionCommand {
    KickLikeCpp(KickLikeCppCommand),
    WorldSessionShutdownFlushLikeCpp(WorldSessionShutdownFlushLikeCppCommand),
    ApplyCreatureMeleeDamageLikeCpp(ApplyCreatureMeleeDamageLikeCppCommand),
    CreatureAttackStartLikeCpp(CreatureAttackStartLikeCppCommand),
    CreatureAttackStopLikeCpp(CreatureAttackStopLikeCppCommand),
    ReconcilePvpCombatExpiryLikeCpp(ReconcilePvpCombatExpiryLikeCppCommand),
    ApplyLootMoneyLikeCpp(ApplyLootMoneyLikeCppCommand),
    NotifyLootMoneyRemovedLikeCpp(NotifyLootMoneyRemovedLikeCppCommand),
    MasterLootGive(MasterLootGiveCommand),
    LootRollStoreWinner(LootRollStoreWinnerCommand),
    LootRollVote(LootRollVoteCommand),
    ResetSeasonalQuestStatus(ResetSeasonalQuestStatusCommand),
    SendVisibleObjectValuesUpdate(SendVisibleObjectValuesUpdateCommand),
    RefreshVisibleWorldCreaturesLikeCpp(RefreshVisibleWorldCreaturesLikeCppCommand),
    SendCreatureLootReleaseValuesUpdateLikeCpp(SendCreatureLootReleaseValuesUpdateLikeCppCommand),
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
    /// Deliver an already-serialized packet on the receiver's realm socket.
    ///
    /// C++ assigns party-control packets such as `SMSG_PARTY_INVITE` to
    /// `CONNECTION_TYPE_REALM`. Callers that hold only the target session's
    /// command channel use this command to request session-local routing.
    SendRealmPacketLikeCpp(SendRealmPacketLikeCppCommand),
    /// Apply C++ `Group::Disband`/`Group::RemoveMember` session-local cleanup
    /// for a connected remote member.
    ApplyGroupRemovalLikeCpp(ApplyGroupRemovalLikeCppCommand),
    /// Apply C++ `Player::SetGroup(group, subgroup)` and `SetPartyType` when a
    /// connected remote member is added from another session.
    ApplyGroupJoinLikeCpp(ApplyGroupJoinLikeCppCommand),
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
    /// Deliver one creature spell START+GO pair after one shared visibility
    /// gate, selecting the basic or full GO by the receiving player's C++
    /// advanced-combat-log preference.
    SendCreatureSpellCastIfVisibleLikeCpp(SendCreatureSpellCastIfVisibleLikeCppCommand),
    /// Same visibility/phase/range gate as `SendIfVisibleLikeCpp`, but route
    /// the accepted packet through the receiver's realm connection.
    SendRealmIfVisibleLikeCpp(SendIfVisibleLikeCppCommand),
    /// Realm-routed creature delivery whose sender validated a source owned
    /// by the transitional legacy map. The receiver may re-read that legacy
    /// source when no canonical mirror exists.
    SendRealmIfVisibleFromLegacySourceLikeCpp(SendIfVisibleLikeCppCommand),
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
    /// Deliver represented `SMSG_DUEL_REQUESTED` to the remote duel participant.
    SendRepresentedDuelRequestedLikeCpp(SendRepresentedDuelRequestedLikeCppCommand),
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
    /// Character that owned the registry entry when the command was queued.
    /// A `WorldSession` survives character logout, so the receiver must reject
    /// a delayed update after that session selects another character.
    pub recipient: ObjectGuid,
    pub party_update: PartyUpdate,
    pub member_full_state_packets: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendRealmPacketLikeCppCommand {
    /// Character that owned the registry entry when the packet was queued.
    pub recipient: ObjectGuid,
    pub packet_bytes: Vec<u8>,
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

/// Payload for [`SessionCommand::ApplyGroupJoinLikeCpp`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplyGroupJoinLikeCppCommand {
    pub group_guid: u64,
    pub category: u8,
    pub party_type: u8,
    pub subgroup: u8,
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

/// Payload for the transitional map-owned creature melee compatibility hit
/// against one player session.
///
/// The compatibility driver preserves the pre-existing damage bridge while the
/// full C++ `CalculateMeleeDamage` outcome/proc pipeline remains unrepresented.
/// It sets canonical health once and enqueues this command to the victim. The
/// session side treats the command as presentation-only: the monotonic health
/// revision suppresses replay, while health/death are reread from canonical
/// state instead of being written back from this delayed payload.
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
    pub victim_health_state_revision_after: u64,
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
    /// Previous melee victim, if this start switches an existing attack.
    /// C++ `Unit::Attack` removes only the old victim's attacker relation
    /// while retaining its combat/threat references.
    pub previous_victim_guid: Option<ObjectGuid>,
    pub map_id: u16,
    pub instance_id: u32,
    /// `true` when the global runtime already queued `SMSG_ATTACKSTART` through
    /// nearby-visible fanout and this command only mirrors session state.
    pub packet_already_broadcast: bool,
}

/// Payload for a map-owned creature evade/combat-stop transition.
#[derive(Clone, Debug)]
pub struct CreatureAttackStopLikeCppCommand {
    pub attacker_guid: ObjectGuid,
    pub victim_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
}

/// Session-local mirror of a canonical timed PvP combat-reference expiry.
#[derive(Clone, Debug)]
pub struct ReconcilePvpCombatExpiryLikeCppCommand {
    pub player_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
}

/// Durable FIFO handoff for map-owned creature transitions that have
/// already committed authoritative state.
///
/// The bounded general-purpose session queue may legitimately reject visual
/// fanout under backpressure. These commands cannot be dropped, but the global
/// map tick also cannot wait for a stalled session. C++ publishes every melee
/// swing and attack transition in order, so this rail retains each committed
/// event until the owning session drains it. A session that cannot drain a
/// bounded backlog is marked desynchronized and disconnected instead of
/// silently losing authoritative events or growing memory without limit.
pub const MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP: usize = 4_096;

#[derive(Default)]
pub struct DurableCreatureRuntimeCommandsLikeCpp {
    commands: VecDeque<SessionCommand>,
    overflowed: bool,
}

impl DurableCreatureRuntimeCommandsLikeCpp {
    fn publish_like_cpp(&mut self, command: SessionCommand) -> bool {
        if self.commands.len() >= MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            self.overflowed = true;
            return false;
        }
        self.commands.push_back(command);
        true
    }

    pub fn publish_attack_start_like_cpp(
        &mut self,
        command: CreatureAttackStartLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::CreatureAttackStartLikeCpp(command))
    }

    pub fn publish_attack_stop_like_cpp(
        &mut self,
        command: CreatureAttackStopLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::CreatureAttackStopLikeCpp(command))
    }

    pub fn publish_pvp_combat_expiry_like_cpp(
        &mut self,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ReconcilePvpCombatExpiryLikeCpp(command))
    }

    pub fn publish_melee_damage_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
    }

    pub fn publish_send_if_visible_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendIfVisibleLikeCpp(command))
    }

    /// Publish START+GO as one queue element so capacity checks and session
    /// drains cannot observe only one half of a committed spell cast.
    pub fn publish_creature_spell_cast_if_visible_like_cpp(
        &mut self,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
    }

    pub fn drain_like_cpp(&mut self) -> Vec<SessionCommand> {
        self.commands.drain(..).collect()
    }

    pub fn take_overflowed_and_discard_like_cpp(&mut self) -> bool {
        let overflowed = std::mem::take(&mut self.overflowed);
        if overflowed {
            self.commands.clear();
        }
        overflowed
    }
}

/// Payload for [`SessionCommand::SendIfVisibleLikeCpp`].
///
/// Carries both `map_id` and `instance_id` so the per-session gate can reject
/// cross-instance delivery without touching the canonical map manager.
#[derive(Clone, Debug)]
pub struct SendIfVisibleLikeCppCommand {
    /// Monotonic enqueue time. Rust uses this to drop movement fan-out produced
    /// before a login's initial visibility burst has completed.
    pub queued_at: Instant,
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

/// Atomic session handoff for one represented creature spell cast.
///
/// The two serialized frames remain separate so the socket writer sees the
/// normal START then GO packet boundary. They share one addressing envelope,
/// one durable queue slot and one session visibility gate. Both C++ packet
/// variants are committed together; the receiving session selects exactly one
/// GO representation from its player-local logging preference.
/// This atomicity ends at the session handoff: the current socket channel is
/// frame-oriented, so two later `send` calls are not a transactional batch
/// against cloned producers or a receiver that closes between frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendCreatureSpellCastIfVisibleLikeCppCommand {
    pub queued_at: Instant,
    pub source_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub start_packet_bytes: Vec<u8>,
    pub basic_go_packet_bytes: Vec<u8>,
    pub full_go_packet_bytes: Vec<u8>,
    /// The visibility membership this recipient decision was committed against.
    ///
    /// C++ picks recipients synchronously inside `SendSpellGo`, so the answer
    /// belongs to the moment the cast resolved. Carrying the producer's handle
    /// lets the receiving session honor that decision instead of re-deriving it
    /// from a `HaveAtClient` set that has moved on, while still proving the
    /// command belongs to this session incarnation.
    pub committed_visibility_like_cpp: SharedClientVisibleGuidsLikeCpp,
}

/// Carries C++ `WorldSession::DoLootRelease`'s forced creature DynamicFlags
/// update to the receiving session. The receiver must apply its own
/// `Player::isAllowedToLoot` view before serialising the VALUES packet; the
/// source session cannot safely predict session-local state such as a pending
/// instance bind.
#[derive(Clone, Debug)]
pub struct SendCreatureLootReleaseValuesUpdateLikeCppCommand {
    pub creature_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub unit_values_update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    pub authority: Option<OwnedLootAuthority>,
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

/// Payload for [`SessionCommand::SendRepresentedDuelRequestedLikeCpp`].
///
/// C++ `Spell::EffectDuel` sends one `SMSG_DUEL_REQUESTED` packet to the
/// caster and the target after creating the duel flag and before storing
/// `DuelInfo` for both players.
#[derive(Clone, Debug)]
pub struct SendRepresentedDuelRequestedLikeCppCommand {
    pub arbiter_guid: ObjectGuid,
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
    /// Cloneable claim ownership.  If the requester times out, the command's
    /// clone keeps the slot reserved until this target commits or drops it.
    pub claim: Option<LootClaimLease>,
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
    /// One normal roll award or the complete generated disenchant result.
    ///
    /// C++ `LootRoll::Finish` generates every disenchant material before it
    /// starts storing them.  Keeping the generated result in one command lets
    /// the target session preflight and persist the complete result in one
    /// transaction instead of acknowledging one material at a time.
    pub entries: Vec<LootEntry>,
    pub is_disenchant: bool,
    /// See [`MasterLootGiveCommand::claim`].
    pub claim: Option<LootClaimLease>,
    pub result_tx: flume::Sender<MasterLootGiveResult>,
}

/// Session-local application of one already-durable C++ shared-loot payout.
///
/// The source-side detached persistence worker creates this command only after
/// the complete group transaction and the object-owned money claim have both
/// committed.  No target acknowledgement participates in the persistence
/// decision, so two sessions looting concurrently cannot wait on each other's
/// command loops.
#[derive(Clone, Debug)]
pub struct ApplyLootMoneyLikeCppCommand {
    pub recipient: ObjectGuid,
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    /// C++ share advertised by `SMSG_LOOT_MONEY_NOTIFY`.
    pub amount: u64,
    /// Delta that the locked character row actually accepted.  This is shared
    /// with the detached worker so command delivery order cannot make runtime
    /// gold disagree with the durable cap decision.
    pub durable_applied_amount: Arc<AtomicU64>,
    /// Target character's directly shared persistence fence. The detached
    /// worker registers it before opening SQL; no command acknowledgement is
    /// part of the transaction decision.
    pub durable_persistence_tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    pub sole_looter: bool,
    /// Exact backing allocation whose scope epoch is recorded below. Epochs
    /// restart in a newly allocated authority, so the number alone is not an
    /// object-lifetime identity.
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    /// True only if the object-owned claim committed for the recorded generation.
    /// SQL may already be durable when lifecycle replacement makes this false;
    /// the payout still applies, but must not touch the replacement loot pool.
    pub authority_committed: Arc<AtomicBool>,
    /// This recipient was also viewing the pool, so its own session emits
    /// `CoinRemoved` immediately before `LootMoneyNotify`.
    pub send_coin_removed: Arc<AtomicBool>,
    /// The source handler may apply its own share immediately after awaiting
    /// the detached worker while the same command remains queued as the
    /// cancellation fallback. This gate makes those two paths exact-once.
    pub applied: Arc<AtomicBool>,
    /// Packet/criteria publication can occur after a save fence has already
    /// reconciled the durable delta. Keep its exact-once gate separate from
    /// the balance mutation gate.
    pub published: Arc<AtomicBool>,
}

/// Durable result of one character-row money mutation.
///
/// The detached SQL worker records this before publishing its session command.
/// Both paths share [`Self::applied`], so logout reconciliation and normal
/// command delivery cannot apply the same durable delta twice.
#[derive(Clone, Debug)]
pub struct DurableLootMoneyCompletionLikeCpp {
    pub durable_money_before: u64,
    pub durable_money_after: u64,
    pub durable_applied_amount: u64,
    pub applied: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct DurableLootMoneyPersistenceStateLikeCpp {
    in_flight: usize,
    completions: Vec<DurableLootMoneyCompletionLikeCpp>,
    indeterminate: bool,
    admission_closed: bool,
    permanently_closed: bool,
    active_save_fences: usize,
}

/// Per-character fence for detached loot-money transactions.
///
/// A tracker is published in [`PlayerBroadcastInfo`] and registered directly
/// by a source session before it opens a transaction for every recipient. This
/// avoids command acknowledgements (and their A↔B deadlocks), while allowing a
/// target session to wait for and reconcile durable completions before an
/// absolute `Player::SaveToDB` money write.
#[derive(Debug)]
pub struct DurableLootMoneyPersistenceTrackerLikeCpp {
    state: Mutex<DurableLootMoneyPersistenceStateLikeCpp>,
    changed: tokio::sync::watch::Sender<u64>,
    money_mutation_serial: Arc<tokio::sync::Mutex<()>>,
}

impl Default for DurableLootMoneyPersistenceTrackerLikeCpp {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            state: Mutex::new(DurableLootMoneyPersistenceStateLikeCpp::default()),
            changed,
            money_mutation_serial: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}

impl DurableLootMoneyPersistenceTrackerLikeCpp {
    /// Serialize DB money mutations for this character across stored-item and
    /// group payouts. Multi-recipient workers acquire these locks in sorted
    /// GUID order before taking the matching character-row locks.
    pub async fn lock_money_mutation_like_cpp(&self) -> tokio::sync::OwnedMutexGuard<()> {
        Arc::clone(&self.money_mutation_serial).lock_owned().await
    }

    #[must_use]
    pub fn begin_like_cpp(
        self: &Arc<Self>,
    ) -> Result<DurableLootMoneyPersistenceGuardLikeCpp, DurableLootMoneyAdmissionClosedLikeCpp>
    {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.admission_closed {
            return Err(DurableLootMoneyAdmissionClosedLikeCpp);
        }
        state.in_flight += 1;
        drop(state);
        let _ = self.changed.send_modify(|generation| {
            *generation = generation.wrapping_add(1);
        });
        Ok(DurableLootMoneyPersistenceGuardLikeCpp {
            tracker: Arc::clone(self),
            resolved: false,
        })
    }

    /// Close admission before observing `in_flight` and keep it closed across
    /// snapshot plus SQL commit. Either a source registers first and the save
    /// waits, or the save closes first and the source fails before BEGIN.
    #[must_use]
    pub fn close_admission_for_save_like_cpp(self: &Arc<Self>) -> DurableLootMoneySaveFenceLikeCpp {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.active_save_fences = state.active_save_fences.saturating_add(1);
        state.admission_closed = true;
        DurableLootMoneySaveFenceLikeCpp {
            tracker: Arc::clone(self),
        }
    }

    /// Logout closes the old registry-published tracker permanently. A source
    /// that cloned it before unregister cannot mutate the character after its
    /// final save.
    pub fn close_admission_permanently_like_cpp(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.admission_closed = true;
        state.permanently_closed = true;
    }

    pub async fn wait_until_idle_like_cpp(&self) {
        let mut changed = self.changed.subscribe();
        loop {
            if self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .in_flight
                == 0
            {
                return;
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }

    /// Returns every completion whose shared exact-once gate is still open.
    /// Applied entries are pruned only after their CAS is observable.
    #[must_use]
    pub fn pending_completions_like_cpp(&self) -> Vec<DurableLootMoneyCompletionLikeCpp> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.completions.retain(|completion| {
            !completion
                .applied
                .load(std::sync::atomic::Ordering::Acquire)
        });
        state.completions.clone()
    }

    #[must_use]
    pub fn is_indeterminate_like_cpp(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .indeterminate
    }

    pub fn mark_indeterminate_like_cpp(&self) {
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.indeterminate = true;
            state.admission_closed = true;
            state.permanently_closed = true;
        }
        let _ = self.changed.send_modify(|generation| {
            *generation = generation.wrapping_add(1);
        });
    }

    fn finish_like_cpp(
        &self,
        completion: Option<DurableLootMoneyCompletionLikeCpp>,
        indeterminate: bool,
    ) {
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            debug_assert!(state.in_flight != 0);
            state.in_flight = state.in_flight.saturating_sub(1);
            if let Some(completion) = completion {
                state.completions.push(completion);
            }
            state.indeterminate |= indeterminate;
            if indeterminate {
                state.admission_closed = true;
                state.permanently_closed = true;
            }
        }
        let _ = self.changed.send_modify(|generation| {
            *generation = generation.wrapping_add(1);
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DurableLootMoneyAdmissionClosedLikeCpp;

impl std::fmt::Display for DurableLootMoneyAdmissionClosedLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("character money persistence admission is closed")
    }
}

impl std::error::Error for DurableLootMoneyAdmissionClosedLikeCpp {}

#[derive(Debug)]
pub struct DurableLootMoneySaveFenceLikeCpp {
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
}

impl Drop for DurableLootMoneySaveFenceLikeCpp {
    fn drop(&mut self) {
        let mut state = self
            .tracker
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        debug_assert!(state.active_save_fences != 0);
        state.active_save_fences = state.active_save_fences.saturating_sub(1);
        if state.active_save_fences == 0 && !state.permanently_closed {
            state.admission_closed = false;
        }
    }
}

/// RAII registration for one recipient's durable money mutation.
#[derive(Debug)]
pub struct DurableLootMoneyPersistenceGuardLikeCpp {
    tracker: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    resolved: bool,
}

impl DurableLootMoneyPersistenceGuardLikeCpp {
    pub fn commit_like_cpp(&mut self, completion: DurableLootMoneyCompletionLikeCpp) {
        if self.resolved {
            return;
        }
        self.resolved = true;
        self.tracker.finish_like_cpp(Some(completion), false);
    }

    /// A COMMIT was attempted but its outcome could not be reconciled. The
    /// target must skip absolute money saves until it disconnects/reloads.
    pub fn mark_indeterminate_like_cpp(&mut self) {
        if self.resolved {
            return;
        }
        self.resolved = true;
        self.tracker.finish_like_cpp(None, true);
    }
}

impl Drop for DurableLootMoneyPersistenceGuardLikeCpp {
    fn drop(&mut self) {
        if !self.resolved {
            self.tracker.finish_like_cpp(None, false);
            self.resolved = true;
        }
    }
}

/// Durable `Loot::NotifyMoneyRemoved` delivery for an active viewer that is
/// not itself a payout recipient.
#[derive(Clone, Debug)]
pub struct NotifyLootMoneyRemovedLikeCppCommand {
    pub recipient: ObjectGuid,
    pub loot_owner: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub authority_committed: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyLootMoneyResultLikeCpp {
    Applied,
    PersistenceFailed,
    TargetMismatch,
}

#[derive(Clone, Debug)]
pub struct LootRollCommandIdentityLikeCpp {
    loot_obj: ObjectGuid,
    loot_list_id: u8,
    authority: OwnedLootAuthority,
    authority_generation: u64,
    /// Pointer-identity equivalent of the exact C++ `LootRoll*` registered in
    /// `Player::m_lootRolls` (`Player.cpp::GetLootRoll` / `RemoveLootRoll` and
    /// `Loot.cpp::LootRoll::~LootRoll`). A replacement roll may reuse both
    /// packet key and loot generation, so those values alone cannot reject a
    /// queued old vote.
    roll_instance: Arc<()>,
}

impl LootRollCommandIdentityLikeCpp {
    #[must_use]
    pub fn new_like_cpp(
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        authority: OwnedLootAuthority,
        authority_generation: u64,
    ) -> Self {
        Self {
            loot_obj,
            loot_list_id,
            authority,
            authority_generation,
            roll_instance: Arc::new(()),
        }
    }

    #[must_use]
    pub fn matches_key_like_cpp(&self, loot_obj: ObjectGuid, loot_list_id: u8) -> bool {
        self.loot_obj == loot_obj && self.loot_list_id == loot_list_id
    }

    /// Mirrors the lifetime identity of the C++ `LootRoll*`, while also
    /// fail-closing if an authority allocation or generation was replaced.
    #[must_use]
    pub fn is_exact_roll_like_cpp(&self, other: &Self) -> bool {
        self.matches_key_like_cpp(other.loot_obj, other.loot_list_id)
            && self.authority_generation == other.authority_generation
            && self.authority.shares_storage_like_cpp(&other.authority)
            && Arc::ptr_eq(&self.roll_instance, &other.roll_instance)
    }
}

#[derive(Clone, Debug)]
pub struct LootRollVoteCommand {
    pub voter_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub roll_type: u8,
    pub pass_on_group_loot: bool,
    /// Immutable enqueue-time identity of the exact roll instance. C++ queues
    /// no cross-session surrogate; its player retains the exact `LootRoll*`.
    pub roll_identity: LootRollCommandIdentityLikeCpp,
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
    /// Durable/coalesced equivalent of C++'s retained visibility notify bit.
    ///
    /// Senders set this before attempting the bounded command queue. The owning
    /// session consumes it even when the queue was full, so player entry/exit
    /// visibility cannot be lost under command backpressure.
    pub visibility_refresh_pending_like_cpp: Arc<AtomicBool>,
    /// Per-character durable loot-money fence used by remote source sessions.
    pub durable_loot_money_tracker_like_cpp: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
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
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx,
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
            durable_loot_money_tracker_like_cpp: Default::default(),
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
    }

    /// Verify that `SendIfVisibleLikeCppCommand` carries both `map_id` and
    /// `instance_id` — required so per-session gate can reject cross-instance
    /// delivery (Slice 4A.1b).
    #[test]
    fn send_if_visible_like_cpp_command_carries_map_and_instance_id() {
        let guid = ObjectGuid::create_player(1, 7);
        let cmd = SendIfVisibleLikeCppCommand {
            queued_at: Instant::now(),
            source_guid: guid,
            map_id: 532,
            instance_id: 99,
            packet_bytes: vec![0xDE, 0xAD],
        };
        assert_eq!(cmd.map_id, 532);
        assert_eq!(cmd.instance_id, 99);
    }

    #[test]
    fn creature_spell_start_and_both_go_variants_use_one_durable_queue_element_like_cpp() {
        let source_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            123,
            458,
        );
        let command = SendCreatureSpellCastIfVisibleLikeCppCommand {
            queued_at: Instant::now(),
            source_guid,
            map_id: 571,
            instance_id: 4,
            start_packet_bytes: vec![0x37, 0x2C, 0xAA],
            basic_go_packet_bytes: vec![0x36, 0x2C, 0xBB],
            full_go_packet_bytes: vec![0x36, 0x2C, 0xCC],
            committed_visibility_like_cpp: SharedClientVisibleGuidsLikeCpp::default(),
        };
        let mut durable = DurableCreatureRuntimeCommandsLikeCpp::default();

        assert!(durable.publish_creature_spell_cast_if_visible_like_cpp(command.clone()));
        let drained = durable.drain_like_cpp();

        assert_eq!(
            drained.len(),
            1,
            "a drain cannot split START from GO because the pair occupies one FIFO element"
        );
        let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(drained_command)] =
            drained.as_slice()
        else {
            panic!("expected one atomic spell cast command: {drained:?}");
        };
        assert_eq!(drained_command, &command);
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
            victim_health_state_revision_after: 7,
        };

        assert_eq!(cmd.attacker_guid, attacker);
        assert_eq!(cmd.victim_guid, victim);
        assert_eq!(cmd.map_id, 571);
        assert_eq!(cmd.instance_id, 3);
        assert_eq!(cmd.victim_health_after, 89);
        assert_eq!(cmd.victim_health_state_revision_after, 7);
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
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 4,
            packet_already_broadcast: false,
        };

        assert_eq!(cmd.attacker_guid, attacker);
        assert_eq!(cmd.victim_guid, victim);
        assert_eq!(cmd.map_id, 571);
        assert_eq!(cmd.instance_id, 4);
    }

    #[tokio::test]
    async fn durable_money_save_fence_closes_admission_then_waits_for_prior_worker() {
        let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        let worker = tracker.begin_like_cpp().unwrap();
        let save_fence = tracker.close_admission_for_save_like_cpp();

        assert!(tracker.begin_like_cpp().is_err());
        let wait_tracker = Arc::clone(&tracker);
        let waiter = tokio::spawn(async move {
            wait_tracker.wait_until_idle_like_cpp().await;
        });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        drop(worker);
        tokio::time::timeout(std::time::Duration::from_secs(1), waiter)
            .await
            .expect("prior durable worker must release the save wait")
            .unwrap();
        assert!(tracker.begin_like_cpp().is_err());

        drop(save_fence);
        assert!(tracker.begin_like_cpp().is_ok());
    }

    #[test]
    fn durable_money_permanent_logout_fence_never_reopens() {
        let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        let save_fence = tracker.close_admission_for_save_like_cpp();
        tracker.close_admission_permanently_like_cpp();
        drop(save_fence);

        assert!(tracker.begin_like_cpp().is_err());
    }

    #[test]
    fn overlapping_durable_money_save_fences_reopen_only_after_last_drop() {
        let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        let first = tracker.close_admission_for_save_like_cpp();
        let second = tracker.close_admission_for_save_like_cpp();

        drop(first);
        assert!(
            tracker.begin_like_cpp().is_err(),
            "dropping the first fence must not reopen admission under the second"
        );
        drop(second);
        assert!(tracker.begin_like_cpp().is_ok());
    }

    #[test]
    fn durable_money_completion_uses_one_shared_exact_once_gate() {
        let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        let mut worker = tracker.begin_like_cpp().unwrap();
        let applied = Arc::new(AtomicBool::new(false));
        worker.commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
            durable_money_before: 40,
            durable_money_after: 47,
            durable_applied_amount: 7,
            applied: Arc::clone(&applied),
        });

        let pending = tracker.pending_completions_like_cpp();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].durable_money_before, 40);
        assert_eq!(pending[0].durable_money_after, 47);
        assert_eq!(pending[0].durable_applied_amount, 7);
        assert!(
            pending[0]
                .applied
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                )
                .is_ok()
        );
        assert!(tracker.pending_completions_like_cpp().is_empty());
    }

    #[test]
    fn indeterminate_money_outcome_permanently_closes_admission_until_relogin() {
        let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
        let fence = tracker.close_admission_for_save_like_cpp();
        tracker.mark_indeterminate_like_cpp();
        drop(fence);

        assert!(tracker.is_indeterminate_like_cpp());
        assert!(tracker.begin_like_cpp().is_err());
    }

    #[test]
    fn durable_creature_runtime_commands_preserve_committed_fifo_like_cpp() {
        let victim = ObjectGuid::create_player(1, 900);
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            9001,
            901,
        );
        let mut pending = DurableCreatureRuntimeCommandsLikeCpp::default();
        assert!(
            pending.publish_attack_start_like_cpp(CreatureAttackStartLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            })
        );
        assert!(
            pending.publish_attack_stop_like_cpp(CreatureAttackStopLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                map_id: 571,
                instance_id: 0,
            })
        );
        assert!(pending.publish_pvp_combat_expiry_like_cpp(
            ReconcilePvpCombatExpiryLikeCppCommand {
                player_guid: victim,
                map_id: 571,
                instance_id: 0,
            }
        ));
        for (victim_health_after, victim_health_state_revision_after) in [(90, 7), (75, 8)] {
            assert!(pending.publish_melee_damage_like_cpp(
                ApplyCreatureMeleeDamageLikeCppCommand {
                    attacker_guid: attacker,
                    victim_guid: victim,
                    map_id: 571,
                    instance_id: 0,
                    damage: 15,
                    over_damage: -1,
                    target_level: 80,
                    victim_health_after,
                    victim_health_state_revision_after,
                }
            ));
        }

        let commands = pending.drain_like_cpp();
        assert_eq!(commands.len(), 5);
        assert!(matches!(
            commands[0],
            SessionCommand::CreatureAttackStartLikeCpp(_)
        ));
        assert!(matches!(
            commands[1],
            SessionCommand::CreatureAttackStopLikeCpp(_)
        ));
        assert!(matches!(
            commands[2],
            SessionCommand::ReconcilePvpCombatExpiryLikeCpp(_)
        ));
        let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(first_melee) = &commands[3] else {
            panic!("expected first melee event");
        };
        let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(second_melee) = &commands[4] else {
            panic!("expected second melee event");
        };
        assert_eq!(first_melee.victim_health_after, 90);
        assert_eq!(first_melee.victim_health_state_revision_after, 7);
        assert_eq!(second_melee.victim_health_after, 75);
        assert_eq!(second_melee.victim_health_state_revision_after, 8);
        assert!(pending.drain_like_cpp().is_empty());
    }

    #[test]
    fn durable_creature_runtime_commands_bound_stalled_session_memory_like_cpp() {
        let victim = ObjectGuid::create_player(1, 902);
        let attacker = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            571,
            0,
            9001,
            903,
        );
        let command = CreatureAttackStartLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            previous_victim_guid: None,
            map_id: 571,
            instance_id: 0,
            packet_already_broadcast: false,
        };
        let mut pending = DurableCreatureRuntimeCommandsLikeCpp::default();
        for _ in 0..MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            assert!(pending.publish_attack_start_like_cpp(command.clone()));
        }
        assert!(!pending.publish_attack_start_like_cpp(command));
        assert!(pending.take_overflowed_and_discard_like_cpp());
        assert!(pending.drain_like_cpp().is_empty());
        assert!(!pending.take_overflowed_and_discard_like_cpp());
    }
}
