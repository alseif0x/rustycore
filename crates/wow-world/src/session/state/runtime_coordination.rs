//! Existing Session substate definitions; authority and field contracts are unchanged.

use super::*;

/// Shared registries and the game-event channel the session coordinates through.
#[derive(Default)]
pub(in crate::session) struct SessionDirectory {
    /// Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    pub(in crate::session) game_event_quest_complete_tx:
        Option<flume::Sender<GameEventQuestCompleteCommandLikeCpp>>,
    /// Shared group registry for party management.
    pub(in crate::session) group_registry: Option<Arc<GroupRegistry>>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub(in crate::session) pending_invites: Option<Arc<PendingInvites>>,
}

/// Session-owned services the phase driver consults: the canonical time-sync
/// protocol state and the represented gameplay RNG.
pub(in crate::session) struct SessionDriverServices {
    /// Canonical per-session time-sync protocol state.
    pub(in crate::session) time_synchronization: TimeSynchronizationStateLikeCpp,
    /// Session-owned RNG for represented gameplay choices that C++ resolves through
    /// `urand`/`SelectRandomContainerElement` while the owning Player/Map runtime is
    /// still being split out of `WorldSession`.
    pub(in crate::session) represented_runtime_rng_like_cpp: StdRng,
}

/// The session's view of the world it is in: the active area trigger, the taxi
/// travel map lookup, the combat-tick bookkeeping and the realm PvP flags.
pub(in crate::session) struct SessionWorldView {
    /// C++ `World::IsPvPRealm()` classification.
    pub(in crate::session) is_pvp_realm_like_cpp: bool,
    /// C++ `World::IsFFAPvPRealm()` classification.
    pub(in crate::session) is_ffa_pvp_realm_like_cpp: bool,
    /// Last represented player melee tick used to decrement C++ `m_attackTimer`.
    pub(in crate::session) combat_tick_last_at_like_cpp: Instant,
    /// High-water mark for map-owned creature-melee presentation commands.
    /// Canonical health/death authority lives on `wow-map`; this suppresses
    /// durable FIFO replay without writing delayed values back to that owner.
    pub(in crate::session) last_presented_creature_melee_health_state_revision_like_cpp: u64,
    /// Minimal TaxiNodes.db2 map lookup used by represented `MoveSplineDone` taxi transitions.
    pub(in crate::session) taxi_node_map_ids_like_cpp: HashMap<u32, u16>,
    /// Currently active area trigger ID, set when entered and cleared when exited.
    pub(in crate::session) active_area_trigger: Option<u32>,
}

/// The canonical producer's phase rail for this session (#787), separate from
/// the command mailbox because a phase pass drains that mailbox.
pub(in crate::session) struct SessionPhaseRail {
    pub(in crate::session) tx: flume::Sender<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
    pub(in crate::session) rx: flume::Receiver<crate::session::mailbox::SessionPhaseRequestLikeCpp>,
}

/// Cross-thread session flags shared with the services that publish for this
/// session: whether advanced combat logging selects the full spell-log payload,
/// and whether a deferred visibility refresh is still owed.
pub(crate) struct SessionSharedFlags {
    /// C++ `Player::_advancedCombatLoggingEnabled`; consumed when combat-log fanout selects full/basic payloads.
    /// C++ `WorldSession::_filterAddonMessages`' sibling for
    /// `SMSG_SPELL_GO`: shared so a producer can commit the combat-log packet
    /// variant per recipient while distributing a cast, the way C++ selects it
    /// synchronously inside `WorldObject::SendCombatLogMessage`.
    pub(in crate::session) advanced_combat_logging_enabled_like_cpp: Arc<AtomicBool>,
    pub(in crate::session) visibility_refresh_pending_like_cpp: Arc<AtomicBool>,
}
