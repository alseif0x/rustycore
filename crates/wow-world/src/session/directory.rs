// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Opaque directory of active session incarnations and addressing endpoints.
//! Gameplay values resolve through bounded queries against their canonical owners.

#[path = "directory/deferred_visibility.rs"]
mod deferred_visibility;

use crate::canonical_player_access::{
    HonorStatsLikeCpp, canonical_player_aggro_unit_state_like_cpp,
    canonical_player_honor_stats_like_cpp, canonical_player_presentation_like_cpp,
    canonical_player_reputation_standings_like_cpp, with_canonical_player_at_like_cpp,
    with_canonical_player_at_mut_like_cpp,
};
use crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp;
use crate::session::SharedCanonicalMapManager;
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyLootMoneyLikeCppCommand,
    ApplyPlayerMeleeResultLikeCppCommand, CreatureAttackStartLikeCppCommand,
    CreatureAttackStopLikeCppCommand, DestroyVisibleObjectLikeCppCommand,
    DurableCreatureRuntimeCommandsLikeCpp, LootRollCommandIdentityLikeCpp,
    ReconcilePvpCombatExpiryLikeCppCommand, RefreshVisibleWorldCreaturesLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPlayerSpellIfVisibleLikeCppCommand, SessionCommand, SharedClientVisibleGuidsLikeCpp,
    SharedClientVisibleTransportsLikeCpp,
};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use wow_constants::UnitFlags;
use wow_core::{ObjectGuid, Position};
use wow_loot::OwnedLootAuthority;
use wow_packet::packets::movement::TransportInfo;
use wow_packet::packets::party::{
    PartyMemberAuraState, PartyMemberPetStats, PartyMemberPhaseStates,
};
use wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate;

/// Everything a session hands the directory when it registers.
///
/// These are this session's own delivery channels, durable rail, and the
/// visibility/combat-logging state producers resolve against (#361). They live
/// beside the private registry entry, exactly as #189 placed the durable
/// loot-money coordinator, so no remote reader can obtain a handle from a
/// snapshot and no gameplay publish can replace one.
#[derive(Clone)]
pub struct PlayerSessionRegistrationLikeCpp {
    pub identity: PlayerDirectoryIdentityLikeCpp,
    pub placement: PlayerDirectoryPlacementLikeCpp,
    /// Durable loot-roll identities owned by this session incarnation.
    pub active_loot_rolls: Vec<LootRollCommandIdentityLikeCpp>,
    /// Channel used to push serialised packets to this player's primary
    /// (instance after `ConnectTo`) socket.
    pub send_tx: flume::Sender<Vec<u8>>,
    /// Channel used for opcodes registered on `CONNECTION_TYPE_REALM`.
    /// Before `ConnectTo`, or in single-socket tests, this may be the same
    /// channel as [`Self::send_tx`].
    pub realm_send_tx: flume::Sender<Vec<u8>>,
    /// Channel used for C++-style cross-session state mutations.
    pub command_tx: flume::Sender<SessionCommand>,
    /// The session's phase rail (#787): the canonical producer addresses its
    /// map phase here, never through the command mailbox a phase pass drains.
    pub session_phase_tx: flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    /// Durable FIFO rail for authoritative creature combat transitions.
    pub durable_creature_runtime_commands_like_cpp:
        Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>,
    /// Shared C++ `Player::m_clientGUIDs` membership for this session.
    ///
    /// Producers that must commit a recipient decision at the moment a message
    /// is resolved read it here instead of leaving the receiving session to
    /// re-derive visibility from state that moved on in the meantime.
    pub client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp,
    /// Shared C++ `Player::m_visibleTransports` membership. Transport VALUES
    /// use this set rather than the ordinary `m_clientGUIDs` visibility set.
    pub client_visible_transports_like_cpp: SharedClientVisibleTransportsLikeCpp,
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
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerDirectoryPlacementLikeCpp {
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub is_in_world: bool,
    pub level: u8,
    pub is_alive: bool,
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

/// A phase rail with no consumer, for fixtures and for any registration made
/// before its owning task parks on the real one.
///
/// The receiver is dropped immediately, so every delivery fails instead of
/// silently queueing for a session that will never answer.
#[must_use]
pub fn detached_session_phase_rail_like_cpp()
-> flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp> {
    let (tx, _rx) = flume::bounded(1);
    tx
}

/// Owned phase-rail address for one session incarnation (#787).
#[derive(Clone, Debug)]
pub struct SessionPhaseAddressLikeCpp {
    registration: PlayerRegistration,
    session_phase_tx: flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
}

impl SessionPhaseAddressLikeCpp {
    #[must_use]
    pub fn registration(&self) -> PlayerRegistration {
        self.registration
    }

    pub fn try_send(
        &self,
        request: crate::session::mailbox::SessionPhaseRequestLikeCpp,
    ) -> Result<(), flume::TrySendError<crate::session::mailbox::SessionPhaseRequestLikeCpp>> {
        self.session_phase_tx.try_send(request)
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
    /// `None` when the receiver's canonical `Player` could not be resolved —
    /// unknown, not empty. A receiver mid far-teleport has no resident owner,
    /// and treating that as "no reputation" would fail every standing gate
    /// against a real standing the caller cannot see (#252).
    pub reputation_standings: Option<Vec<(u32, i32)>>,
    pub active_expansion: u8,
}

/// Owned facts required by C++ player-vehicle interaction checks.
#[derive(Clone, Copy, Debug)]
pub struct PlayerVehicleInteractionSnapshot {
    pub map_id: u16,
    pub instance_id: u32,
    pub position: Position,
    pub has_vehicle_kit: bool,
}

#[derive(Clone, Debug)]
pub struct PlayerMovementDirectoryUpdate {
    pub position: Position,
    pub map_id: u16,
    pub instance_id: u32,
    pub is_in_world: bool,
    pub level: u8,
    pub is_alive: bool,
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
    /// The attacker's published combat mirror.
    ///
    /// Whoever owns the creature tick needs it to notice that a session still
    /// believes it is in combat with a victim the map has already resolved
    /// away (#28).
    pub in_combat: bool,
    pub advanced_combat_logging: bool,
    pub committed_visibility: SharedClientVisibleGuidsLikeCpp,
    /// Committed transport CREATE membership, equivalent to C++
    /// `Player::m_visibleTransports`.
    pub committed_visible_transports: SharedClientVisibleTransportsLikeCpp,
}

/// Owned online identity used by chat/social lookup. Delivery must use the
/// included registration so a reconnect cannot receive an older decision.
#[derive(Clone, Debug)]
pub struct PlayerSocialRecipientSnapshot {
    pub registration: PlayerRegistration,
    pub guid: ObjectGuid,
    pub player_name: String,
    pub race: u8,
    pub class: u8,
    pub map_id: u16,
    pub instance_id: u32,
    pub dungeon_difficulty_id: u32,
    pub is_game_master: bool,
    pub is_afk: bool,
    pub is_dnd: bool,
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
    pub unit_state: u32,
    pub is_game_master: bool,
    pub faction_template_id: u32,
    pub forced_reputation_ranks: Vec<(u32, wow_data::reputation::ReputationRankLikeCpp)>,
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
    identity: PlayerDirectoryIdentityLikeCpp,
    placement: PlayerDirectoryPlacementLikeCpp,
    active_loot_rolls: Vec<LootRollCommandIdentityLikeCpp>,
    /// Delivery handles for this incarnation, private to the directory (#270).
    ///
    /// They are not part of the projection: publishing gameplay state can no
    /// longer overwrite a route, and a snapshot cannot hand a remote session a
    /// raw sender.
    send_tx: flume::Sender<Vec<u8>>,
    realm_send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
    session_phase_tx: flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    durable_creature_runtime_commands_like_cpp: Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>,
    /// Shared handles read live at resolve time; beside the entry, not in the
    /// projection, so publishing gameplay state cannot replace one (#361).
    client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp,
    client_visible_transports_like_cpp: SharedClientVisibleTransportsLikeCpp,
    advanced_combat_logging_enabled_like_cpp: Arc<AtomicBool>,
    visibility_refresh_pending_like_cpp: Arc<AtomicBool>,
    /// Durable loot-money coordination for this incarnation.
    ///
    /// Issue #189 keeps this beside the entry: it is not gameplay state another
    /// session may read, it is the persistence handle the owning session already holds,
    /// resolved here only so a remote looter can address the recipient's
    /// coordinator. It creates no second store and no second authority.
    durable_loot_money: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
}

#[path = "directory/delivery.rs"]
mod delivery;
#[path = "directory/group_state.rs"]
mod group_state;
#[path = "directory/identity.rs"]
mod identity;
#[path = "directory/loot.rs"]
mod loot;
#[path = "directory/name_query.rs"]
mod name_query;
#[path = "directory/recipient_queries.rs"]
mod recipient_queries;
#[cfg(any(test, feature = "test-fixtures"))]
#[path = "directory/test_fixtures.rs"]
mod test_fixtures;
pub use identity::PlayerDirectoryIdentityLikeCpp;
pub use name_query::PlayerNameQuerySnapshotLikeCpp;

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
    canonical_map_manager: OnceLock<SharedCanonicalMapManager>,
    /// Group state changes that could not be handed to their target; see
    /// [`group_state`] for the contract (#743).
    pending_group_state_reconciliation_like_cpp: DashMap<ObjectGuid, ()>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fixture_installs_canonical_players: bool,
}

impl Default for PlayerRegistry {
    fn default() -> Self {
        Self {
            entries: DashMap::new(),
            next_generation: AtomicU64::new(1),
            canonical_map_manager: OnceLock::new(),
            pending_group_state_reconciliation_like_cpp: DashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_installs_canonical_players: false,
        }
    }
}

impl PlayerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_canonical_map_manager(&self, manager: SharedCanonicalMapManager) -> bool {
        if let Some(existing) = self.canonical_map_manager.get() {
            return Arc::ptr_eq(existing, &manager);
        }
        self.canonical_map_manager.set(manager).is_ok()
    }

    pub(crate) fn canonical_map_manager_like_cpp(&self) -> Option<SharedCanonicalMapManager> {
        self.canonical_map_manager.get().map(Arc::clone)
    }

    fn next_generation(&self) -> u64 {
        self.next_generation.fetch_add(1, Ordering::Relaxed)
    }

    /// Register this connected session, replacing an older incarnation for the
    /// same GUID and returning the identity required for later lifecycle work.
    pub fn register_or_replace(
        &self,
        guid: ObjectGuid,
        registration: PlayerSessionRegistrationLikeCpp,
        durable_loot_money: Arc<DurableLootMoneyPersistenceTrackerLikeCpp>,
    ) -> PlayerRegistration {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.install_canonical_player_fixture_like_cpp(guid, &registration);
        let generation = self.next_generation();
        let PlayerSessionRegistrationLikeCpp {
            identity,
            placement,
            active_loot_rolls,
            send_tx,
            realm_send_tx,
            command_tx,
            session_phase_tx,
            durable_creature_runtime_commands_like_cpp,
            client_visible_guids_like_cpp,
            client_visible_transports_like_cpp,
            advanced_combat_logging_enabled_like_cpp,
            visibility_refresh_pending_like_cpp,
        } = registration;
        self.entries.insert(
            guid,
            PlayerRegistryEntry {
                generation,
                identity,
                placement,
                active_loot_rolls,
                send_tx,
                realm_send_tx,
                command_tx,
                session_phase_tx,
                durable_creature_runtime_commands_like_cpp,
                client_visible_guids_like_cpp,
                client_visible_transports_like_cpp,
                advanced_combat_logging_enabled_like_cpp,
                visibility_refresh_pending_like_cpp,
                durable_loot_money,
            },
        );
        PlayerRegistration { guid, generation }
    }

    /// Clone the entry only when `registration` is still the current session.
    #[must_use]
    pub fn lookup_current(&self, registration: PlayerRegistration) -> Option<()> {
        let entry = self.entries.get(&registration.guid)?;
        (entry.generation == registration.generation).then_some(())
    }

    /// Resolve an owned command address for the current incarnation of `guid`.
    #[must_use]
    /// The phase rail of the current incarnation of `guid`, with the
    /// registration that rail belonged to when it was resolved (#787).
    #[must_use]
    pub fn session_phase_address_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<SessionPhaseAddressLikeCpp> {
        let entry = self.entries.get(&guid)?;
        Some(SessionPhaseAddressLikeCpp {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            session_phase_tx: entry.session_phase_tx.clone(),
        })
    }

    pub fn control_address(&self, guid: ObjectGuid) -> Option<PlayerControlAddress> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerControlAddress {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            command_tx: entry.command_tx.clone(),
        })
    }

    /// Resolve only current incarnation identities for account-scoped control.
    #[must_use]
    pub fn registrations_for_accounts(
        &self,
        account_ids: &[u32],
    ) -> Vec<(u32, PlayerRegistration)> {
        self.entries
            .iter()
            .filter_map(|entry| {
                account_ids.contains(&entry.identity.account_id).then_some((
                    entry.identity.account_id,
                    PlayerRegistration {
                        guid: *entry.key(),
                        generation: entry.generation,
                    },
                ))
            })
            .collect()
    }

    /// Remove only the exact registration supplied by its owning session.
    /// Returns `true` when that incarnation was still current.
    pub fn unregister(&self, registration: PlayerRegistration) -> bool {
        self.entries
            .remove_if(&registration.guid, |_, entry| {
                entry.generation == registration.generation
            })
            .inspect(|_| self.clear_group_state_reconciliation_like_cpp(registration.guid))
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
            .remove_if(&guid, |_, entry| entry.command_tx.same_channel(command_tx))
            .inspect(|_| self.clear_group_state_reconciliation_like_cpp(guid))
            .is_some()
    }

    /// Publish PartyMemberData::PartyType only for this exact session control
    /// channel, preventing a stale session from overwriting its replacement.
    pub fn publish_party_type_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        party_type: [u8; 2],
    ) -> bool {
        let (map_id, instance_id) = {
            let Some(entry) = self.entries.get(&guid) else {
                return false;
            };
            if !entry.command_tx.same_channel(command_tx) {
                return false;
            }
            (entry.placement.map_id, entry.placement.instance_id)
        };
        let Some(manager) = self.canonical_map_manager.get() else {
            return false;
        };
        with_canonical_player_at_mut_like_cpp(manager, guid, map_id.into(), instance_id, |player| {
            party_type
                .into_iter()
                .enumerate()
                .all(|(category, value)| player.set_party_type_like_cpp(category as u8, value))
        })
        .unwrap_or(false)
    }

    /// Publish the one combat bit to the canonical Player selected by this exact incarnation.
    pub fn publish_in_combat_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        in_combat: bool,
    ) -> bool {
        let (map_id, instance_id) = {
            let Some(entry) = self.entries.get(&guid) else {
                return false;
            };
            if !entry.command_tx.same_channel(command_tx) {
                return false;
            }
            (entry.placement.map_id, entry.placement.instance_id)
        };
        self.canonical_at_mut(guid, map_id, instance_id, |player| {
            let mut flags = player.unit().unit_flags_like_cpp();
            flags.set(UnitFlags::IN_COMBAT, in_combat);
            player.unit_mut().set_unit_flags_like_cpp(flags);
        })
        .is_some()
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
        if !entry.command_tx.same_channel(command_tx) {
            return false;
        }
        let old_placement = entry.placement;
        entry.placement.position = update.position;
        entry.placement.map_id = update.map_id;
        entry.placement.instance_id = update.instance_id;
        entry.placement.is_in_world = update.is_in_world;
        entry.placement.level = update.level;
        entry.placement.is_alive = update.is_alive;
        drop(entry);
        let transport = update
            .transport
            .map(|transport| wow_entities::PlayerTransportState {
                guid: transport.guid,
                x: transport.x,
                y: transport.y,
                z: transport.z,
                orientation: transport.o,
                seat: transport.seat,
                time: transport.time,
                prev_time: transport.prev_time,
                vehicle_id: transport.vehicle_id,
            });
        self.canonical_at_mut(
            guid,
            old_placement.map_id,
            old_placement.instance_id,
            |player| player.set_transport_like_cpp(transport),
        )
        .is_some()
    }
}
