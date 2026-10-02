use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Instant;

use rand::rngs::StdRng;
use wow_core::ObjectGuid;
use wow_network::{SocketTimeoutsLikeCpp, session_mgr::SessionManager};
use wow_packet::WorldPacket;
use wow_social::group::{GroupRegistry, PendingInvites};

use crate::session::mailbox::GameEventQuestCompleteCommandLikeCpp;
use crate::session::{
    PacketCounterLikeCpp, PacketSpoofPendingBanLikeCpp, TimeSynchronizationStateLikeCpp,
};
use crate::session_policy::PacketSpoofConfigLikeCpp;

pub mod catalogs;
pub use catalogs::SessionCatalogs;

pub mod config;
pub use config::SessionWorldConfig;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod combat;
#[cfg(any(test, feature = "test-fixtures"))]
pub use combat::CombatState;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod identity;
#[cfg(any(test, feature = "test-fixtures"))]
pub use identity::PlayerIdentityState;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod collections;
#[cfg(any(test, feature = "test-fixtures"))]
pub use collections::CollectionsState;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod auras;
#[cfg(any(test, feature = "test-fixtures"))]
pub use auras::AuraState;

#[cfg(any(test, feature = "test-fixtures"))]
mod progression;
#[cfg(any(test, feature = "test-fixtures"))]
pub use progression::ProgressionState;

#[cfg(any(test, feature = "test-fixtures"))]
mod presentation;
#[cfg(any(test, feature = "test-fixtures"))]
pub use presentation::PlayerPresentationState;

#[cfg(any(test, feature = "test-fixtures"))]
mod movement;
#[cfg(any(test, feature = "test-fixtures"))]
pub use movement::MovementState;

#[cfg(any(test, feature = "test-fixtures"))]
mod pets;
#[cfg(any(test, feature = "test-fixtures"))]
pub use pets::PetState;

#[cfg(any(test, feature = "test-fixtures"))]
mod teleport;
#[cfg(any(test, feature = "test-fixtures"))]
pub use teleport::TeleportState;

#[cfg(any(test, feature = "test-fixtures"))]
mod vehicles;
#[cfg(any(test, feature = "test-fixtures"))]
pub use vehicles::TaxiVehicleState;

#[cfg(any(test, feature = "test-fixtures"))]
mod battleground;
#[cfg(any(test, feature = "test-fixtures"))]
pub use battleground::BattlegroundState;

pub mod session_core;
pub use session_core::SessionCore;

mod driver_phase;
pub use driver_phase::SessionDriverPhaseLikeCpp;

pub mod hub_support;

/// Shared registries and the game-event channel the session coordinates through.
#[derive(Default)]
pub struct SessionDirectory {
    /// Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    pub game_event_quest_complete_tx: Option<flume::Sender<GameEventQuestCompleteCommandLikeCpp>>,
    /// Shared group registry for party management.
    pub group_registry: Option<Arc<GroupRegistry>>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub pending_invites: Option<Arc<PendingInvites>>,
}

/// Session-owned services the phase driver consults: the canonical time-sync
/// protocol state and the represented gameplay RNG.
pub struct SessionDriverServices {
    /// Canonical per-session time-sync protocol state.
    pub time_synchronization: TimeSynchronizationStateLikeCpp,
    /// Session-owned RNG for represented gameplay choices that C++ resolves through
    /// `urand`/`SelectRandomContainerElement` while the owning Player/Map runtime is
    /// still being split out of `WorldSession`.
    pub represented_runtime_rng_like_cpp: StdRng,
}

/// The session's transport and connection identity: the `wow-session` transport
/// kernel (#297), the physical remote address, the authentication session key
/// and the shared session manager handle for the ConnectTo flow.
pub struct SessionTransport {
    /// The realm/instance transport, owned by `wow-session` (#297).
    ///
    /// The first piece of this type to earn its own crate: it compiles without
    /// gameplay, databases or catalogs, so the compiler now prevents transport
    /// decisions from reaching a `Player`, a `Map` or a query.
    pub connection: wow_session::SessionConnection,
    pub remote_address_like_cpp: Option<String>,
    pub session_key: Vec<u8>,
    /// Session manager for ConnectTo flow (shared with instance listener).
    pub session_mgr: Option<Arc<SessionManager>>,
}

/// Packet admission state: the ingress throttle and spoof-ban bookkeeping, the
/// pending packet queue and the socket timeout and phase-authority fences for the
/// admitted traffic.
pub struct SessionAdmissionState {
    pub last_packet_time: Instant,
    /// The producer and step this session last accepted, per phase (#787).
    ///
    /// C++ has one caller and needs no such watermark. Here it is what rejects
    /// a foreign producer, a retired step and a replay of one already served,
    /// none of which the identity of the player can distinguish. It is kept per
    /// phase because one step legitimately issues the world phase and then the
    /// map phase under the same epoch (`World.cpp:2704` then `World.cpp:2748`).
    pub last_phase_authority_like_cpp: [Option<(u64, u64)>; 2],
    /// Set by the first canonical map-phase request (#787). Until then this
    /// session has no coordinator and keeps draining its own queue.
    pub map_phase_coordinated_like_cpp: bool,
    pub packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp,
    pub(in crate::session) packet_throttling_like_cpp: HashMap<u16, PacketCounterLikeCpp>,
    pub pending_packet_spoof_ban_like_cpp: Option<PacketSpoofPendingBanLikeCpp>,
    pub pending_packets: VecDeque<WorldPacket>,
    pub socket_timeout_deadline_like_cpp: Instant,
    pub socket_timeouts_like_cpp: SocketTimeoutsLikeCpp,
}

/// The realm and instance policy the session admits play under: the realm's
/// region, battlegroup, name table and secret, the server expansion cap, the
/// hourly instance budget and the two instance-ignore switches.
pub struct SessionRealmPolicy {
    pub realm_battlegroup: u8,
    pub realm_region: u8,
    pub realm_names_like_cpp: BTreeMap<u32, (String, String)>,
    pub realm_list_secret_like_cpp: [u8; 32],
    pub server_expansion_like_cpp: u8,
    pub max_instances_per_hour_like_cpp: u32,
    pub instance_ignore_level_like_cpp: bool,
    pub instance_ignore_raid_like_cpp: bool,
}

/// Account-level session state: the Battle.net account id, the recruit-a-friend
/// edges, the account's legitimate characters, the recent character low guid and
/// the mute expiry the chat handlers enforce.
pub struct SessionAccountState {
    pub battlenet_account_id: u32,
    pub is_a_recruiter_like_cpp: bool,
    pub recruiter_id_like_cpp: u32,
    pub legit_characters: Vec<ObjectGuid>,
    /// C++ `WorldSession::m_GUIDLow`: last logged-in character low GUID kept after logout.
    pub recent_player_guid_low_like_cpp: u64,
    pub mute_time_like_cpp: i64,
}

/// Cross-thread session flags shared with the services that publish for this
/// session: whether advanced combat logging selects the full spell-log payload,
/// and whether a deferred visibility refresh is still owed.
pub struct SessionSharedFlags {
    /// C++ `Player::_advancedCombatLoggingEnabled`; consumed when combat-log fanout selects full/basic payloads.
    /// C++ `WorldSession::_filterAddonMessages`' sibling for
    /// `SMSG_SPELL_GO`: shared so a producer can commit the combat-log packet
    /// variant per recipient while distributing a cast, the way C++ selects it
    /// synchronously inside `WorldObject::SendCombatLogMessage`.
    pub advanced_combat_logging_enabled_like_cpp: Arc<AtomicBool>,
    pub visibility_refresh_pending_like_cpp: Arc<AtomicBool>,
}
