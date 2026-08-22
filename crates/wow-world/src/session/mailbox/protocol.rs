// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordinary Session mailbox protocol: the cross-session command enum and every
//! payload a producer may enqueue on the owning session.
//!
//! Enqueueing is the only way another task mutates this session; the payloads
//! carry committed facts, never borrowed state or transport handles.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Instant;

use wow_core::ObjectGuid;
use wow_loot::{LootClaimLease, OwnedLootAuthority};
use wow_packet::packets::loot::LootEntry;
use wow_packet::packets::party::PartyUpdate;
pub use wow_social::group::GroupDifficultyKindLikeCpp;

use super::durable::DurableLootMoneyPersistenceTrackerLikeCpp;

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
        keep: impl FnMut(&ObjectGuid) -> bool,
        added: impl IntoIterator<Item = ObjectGuid>,
    ) {
        self.publish_transition_like_cpp(keep, added, || ());
    }

    /// Publish a visibility transition together with the packets that carry it.
    ///
    /// The membership and its `SMSG_UPDATE_OBJECT` form one client-visible step.
    /// A producer that read the old membership while the create block was
    /// already queued would skip a viewer whose client now has the object, and
    /// one that read the new membership while the out-of-range block was queued
    /// would address an object the client is destroying. Running `publish` under
    /// the same write makes both intermediate states unobservable.
    ///
    /// `publish` must not touch this set again — it would deadlock on the write
    /// it is already inside.
    pub fn publish_transition_like_cpp<R>(
        &self,
        mut keep: impl FnMut(&ObjectGuid) -> bool,
        added: impl IntoIterator<Item = ObjectGuid>,
        publish: impl FnOnce() -> R,
    ) -> R {
        let mut guard = self.write_like_cpp();
        guard.retain(|guid| keep(guid));
        guard.extend(added);
        publish()
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
    /// The GO frame chosen for this recipient when the cast resolved.
    ///
    /// C++ selects the basic or full combat-log representation synchronously
    /// while distributing the cast, so the choice cannot depend on a preference
    /// the client may toggle before this command is drained.
    pub go_packet_bytes: Vec<u8>,
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
