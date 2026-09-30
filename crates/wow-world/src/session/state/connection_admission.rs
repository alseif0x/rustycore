//! Existing Session substate definitions; authority and field contracts are unchanged.

use super::*;

/// The session's transport and connection identity: the `wow-session` transport
/// kernel (#297), the physical remote address, the authentication session key
/// and the shared session manager handle for the ConnectTo flow.
pub(crate) struct SessionTransport {
    /// The realm/instance transport, owned by `wow-session` (#297).
    ///
    /// The first piece of this type to earn its own crate: it compiles without
    /// gameplay, databases or catalogs, so the compiler now prevents transport
    /// decisions from reaching a `Player`, a `Map` or a query.
    pub(in crate::session) connection: wow_session::SessionConnection,
    pub(in crate::session) remote_address_like_cpp: Option<String>,
    pub session_key: Vec<u8>,
    /// Session manager for ConnectTo flow (shared with instance listener).
    pub(in crate::session) session_mgr: Option<Arc<SessionManager>>,
}

/// Packet admission and dispatch state: the opcode dispatch table, the ingress
/// throttle and spoof-ban bookkeeping, the pending packet queue and the socket
/// timeout and phase-authority fences for the admitted traffic.
pub(crate) struct SessionAdmissionState {
    pub(in crate::session) dispatch_table: HashMap<ClientOpcodes, &'static PacketHandlerEntry>,
    pub(in crate::session) last_packet_time: Instant,
    /// The producer and step this session last accepted, per phase (#787).
    ///
    /// C++ has one caller and needs no such watermark. Here it is what rejects
    /// a foreign producer, a retired step and a replay of one already served,
    /// none of which the identity of the player can distinguish. It is kept per
    /// phase because one step legitimately issues the world phase and then the
    /// map phase under the same epoch (`World.cpp:2704` then `World.cpp:2748`).
    pub(in crate::session) last_phase_authority_like_cpp: [Option<(u64, u64)>; 2],
    /// Set by the first canonical map-phase request (#787). Until then this
    /// session has no coordinator and keeps draining its own queue.
    pub(in crate::session) map_phase_coordinated_like_cpp: bool,
    pub(in crate::session) packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp,
    pub(in crate::session) packet_throttling_like_cpp: HashMap<u16, PacketCounterLikeCpp>,
    pub(in crate::session) pending_packet_spoof_ban_like_cpp: Option<PacketSpoofPendingBanLikeCpp>,
    pub(in crate::session) pending_packets: VecDeque<WorldPacket>,
    pub(in crate::session) socket_timeout_deadline_like_cpp: Instant,
    pub(in crate::session) socket_timeouts_like_cpp: SocketTimeoutsLikeCpp,
}

/// The realm and instance policy the session admits play under: the realm's
/// region, battlegroup, name table and secret, the server expansion cap, the
/// hourly instance budget and the two instance-ignore switches.
pub(crate) struct SessionRealmPolicy {
    pub(in crate::session) realm_battlegroup: u8,
    pub(in crate::session) realm_region: u8,
    pub(in crate::session) realm_names_like_cpp: BTreeMap<u32, (String, String)>,
    pub(in crate::session) realm_list_secret_like_cpp: [u8; 32],
    pub(in crate::session) server_expansion_like_cpp: u8,
    pub(in crate::session) max_instances_per_hour_like_cpp: u32,
    pub(in crate::session) instance_ignore_level_like_cpp: bool,
    pub(in crate::session) instance_ignore_raid_like_cpp: bool,
}

/// Account-level session state: the Battle.net account id, the recruit-a-friend
/// edges, the account's legitimate characters, the recent character low guid and
/// the mute expiry the chat handlers enforce.
pub(crate) struct SessionAccountState {
    pub(in crate::session) battlenet_account_id: u32,
    pub(in crate::session) is_a_recruiter_like_cpp: bool,
    pub(in crate::session) recruiter_id_like_cpp: u32,
    pub(in crate::session) legit_characters: Vec<ObjectGuid>,
    /// C++ `WorldSession::m_GUIDLow`: last logged-in character low GUID kept after logout.
    pub(in crate::session) recent_player_guid_low_like_cpp: u64,
    pub(in crate::session) mute_time_like_cpp: i64,
}
