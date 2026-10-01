// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::core` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Hub state every domain reads: account and realm identity, the session state and command rails,
/// the selected-player binding, the map/registry/instance handles, the id generators and module
/// registry, and the nested transport, admission, driver, directory, account and realm sub-states.
pub(crate) struct SessionCore {
    // Account info
    pub account_id: u32,

    pub account_name: String,
    pub security: u8,
    pub expansion: u8,
    pub account_expansion: u8,
    pub build: u32,

    pub locale: String,

    // Inbound packet queue (from WorldSocket)

    // Outbound channel (serialized bytes back to WorldSocket)
    // FIFO completion fence paired with the current physical send channel.

    // Cross-session commands executed by this session's own update loop.
    pub(in crate::session) session_command_tx: flume::Sender<SessionCommand>,
    pub(in crate::session) session_command_rx: flume::Receiver<SessionCommand>,

    pub(in crate::session) durable_creature_runtime_commands_like_cpp:
        Arc<std::sync::Mutex<crate::session::mailbox::DurableCreatureRuntimeCommandsLikeCpp>>,

    // State
    pub(in crate::session) state: SessionState,
    /// Ordered record of the driver phases this session has run. Test-only:
    /// it exists so tests assert on the production sequence in
    /// `session::driver` instead of reimplementing it.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) driver_phase_trace_like_cpp:
        Vec<crate::session::driver::phases::SessionDriverPhaseLikeCpp>,

    // Shared player registry for broadcasting to nearby sessions
    pub(in crate::session) player_registry: Option<Arc<PlayerRegistry>>,

    /// Party registries and the world-event channel, grouped by the B4 split.
    pub(in crate::session) directory: SessionDirectory,

    /// Phase-driver services: time-sync state and the represented gameplay RNG.
    pub(in crate::session) driver: SessionDriverServices,

    /// Flags this session publishes to its publisher tasks.
    pub(crate) flags: SessionSharedFlags,

    /// Realm and instance policy for this session.
    pub(crate) realm_policy: SessionRealmPolicy,

    /// Account-level session state shared with the chat and character handlers.
    pub(crate) account_state: SessionAccountState,

    /// Admission, throttling and dispatch state for the session's inbound packets.
    pub(crate) admission: SessionAdmissionState,

    /// Transport kernel, remote address, session key and session manager handle.
    pub(crate) transport: SessionTransport,

    // Realm ID for GUID creation
    pub(in crate::session) realm_id: u16,

    // Process-owned GUID generators retained only as test fixtures.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) guid_generator: Option<Arc<ObjectGuidGenerator>>,
    // Process-wide C++ ObjectMgr generator retained only as a test fixture.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) item_guid_generator_like_cpp: Option<Arc<ObjectGuidGenerator>>,
    // Process-wide C++ ObjectMgr generator shared by equipment and transmog sets.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) equipment_set_guid_generator_like_cpp:
        Option<Arc<EquipmentSetGuidGeneratorLikeCpp>>,
    // Process-wide C++ ObjectMgr generator for character_void_storage.itemId.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) void_storage_item_id_generator_like_cpp:
        Option<Arc<VoidStorageItemIdGeneratorLikeCpp>>,

    /// GUID of the character currently logged in (set after login completes).
    pub(in crate::session) player_guid: Option<ObjectGuid>,

    /// Test fixtures may attach a Player bootstrap before injecting the
    /// production MapManager. Production attachment is represented solely by
    /// the generation-checked PlayerHandle.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) player_bootstrap_attached_like_cpp: bool,

    /// Current map ID for VALUES update packets.
    pub(in crate::session) current_map_id: u16,

    /// Login-only identity input consumed while the canonical Player is being
    /// constructed. Production reads resolve from that Player after install.
    pub(in crate::session) player_identity_bootstrap_like_cpp:
        Option<PlayerIdentityBootstrapLikeCpp>,

    /// Shared, server-wide map state. When `Some`, creature reads/writes can
    /// route through here so all sessions on the same map see the same world.
    /// `None` until the world server injects the manager (see `set_map_manager`).
    pub(crate) map_manager: Option<crate::map_manager::SharedMapManager>,
    /// Canonical C++-style `wow-map` manager. This is injected separately from
    /// the legacy `wow-world` manager while handlers migrate to `wow-map`.
    pub(crate) canonical_map_manager: Option<SharedCanonicalMapManager>,
    /// Generation-checked identity of the one canonical Player value owned by
    /// MapManager. It remains resolvable while detached for a far teleport.
    pub(in crate::session) player_handle_like_cpp: Option<wow_map::PlayerHandle>,

    /// Dedicated Detour owner handle. The underlying `MMapManager` remains on
    /// its worker thread because Detour state is not `Send + Sync`.
    pub(in crate::session) mmap_pathfinder_like_cpp: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    /// Shared C++ `InstanceLockMgr` analogue used by raid-info and instance entry paths.
    pub(crate) instance_lock_mgr: Option<Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>>,

    /// Test fixture for the process-owned linked-module registry. Production
    /// borrows the required registry from the session driver.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) module_registry_like_cpp: Option<Arc<wow_module_api::ModuleRegistry>>,

    // ── Dynamic visibility tracking ───────────────────────────────
    /// C++ `Player::m_clientGUIDs`: exact objects currently known by this client.
    /// Updated on login and each visibility refresh (player movement).
    pub(crate) client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp,
}
