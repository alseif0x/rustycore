// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession` — per-player session that receives packets from the
//! [`WorldSocket`](wow_network::WorldSocket) and dispatches them to handlers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tracing::{debug, info, trace, warn};

use wow_constants::ClientOpcodes;
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_data::{HotfixBlobCache, ItemStore, ItemStatsStore, PlayerStatsStore, SkillStore, AreaTriggerStore, SpellStore};
use wow_database::{CharacterDatabase, LoginDatabase, WorldDatabase};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus, build_dispatch_table};
use wow_network::session_mgr::{InstanceLink, SessionManager};
use wow_network::{GroupRegistry, PendingInvites, PlayerBroadcastInfo, PlayerRegistry};
use wow_packet::{ClientPacket, WorldPacket};

/// Maximum number of packets processed per `update()` call.
const MAX_PACKETS_PER_UPDATE: usize = 100;

/// Current state of the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Authenticated but no character selected.
    Authed,
    /// Character is logged into the world.
    LoggedIn,
    /// Character is transferring between maps.
    Transfer,
    /// Session is being disconnected.
    Disconnecting,
}

/// Spell casting state — tracks an in-progress spell cast with a timer.
///
/// Used for spells with cast time > 0. When the player initiates a cast,
/// this stores the spell ID, target, and cast start time. The main loop
/// calls `tick_active_spell_cast()` each frame to check if casting completes.
#[derive(Debug, Clone)]
pub struct SpellCastState {
    /// Spell ID being cast.
    pub spell_id: i32,
    /// Target GUID for the spell.
    pub target_guid: ObjectGuid,
    /// Client's cast ID (for SMSG_SPELL_GO echo).
    pub cast_id: ObjectGuid,
    /// When the cast started (Instant::now()).
    pub cast_start_time: Instant,
    /// Total cast time in milliseconds.
    pub cast_time_ms: u32,
    /// Spell visual IDs.
    pub spell_visual: wow_packet::packets::spell::SpellCastVisual,
}

/// Per-player session on the world server.
///
/// Receives deserialized packets from the socket layer via a channel,
/// dispatches them to registered handlers, and sends responses back.
pub struct WorldSession {
    // Account info
    pub account_id: u32,
    pub account_name: String,
    pub security: u8,
    pub expansion: u8,
    pub account_expansion: u8,
    pub build: u32,
    pub session_key: Vec<u8>,
    pub locale: String,

    // Inbound packet queue (from WorldSocket)
    packet_rx: flume::Receiver<WorldPacket>,

    // Outbound channel (serialized bytes back to WorldSocket)
    send_tx: flume::Sender<Vec<u8>>,

    // State
    state: SessionState,
    last_packet_time: Instant,

    // Dispatch table (built once, shared ref)
    dispatch_table: HashMap<ClientOpcodes, &'static PacketHandlerEntry>,

    // Character database
    char_db: Option<Arc<CharacterDatabase>>,

    // Login database (for realmcharacters updates)
    login_db: Option<Arc<LoginDatabase>>,

    // World database (for creature templates, spawns, etc.)
    world_db: Option<Arc<WorldDatabase>>,

    // Item store (Item.db2 data — inventory types, class/subclass)
    item_store: Option<Arc<ItemStore>>,

    // Player level stats store (race/class/level → base stats)
    player_stats: Option<Arc<PlayerStatsStore>>,

    // Item stat modifiers store (item_id → stat bonuses from ItemSparse.db2)
    item_stats_store: Option<Arc<ItemStatsStore>>,

    // Hotfix blob cache: raw DB2 record bytes for DBReply responses
    hotfix_blob_cache: Option<Arc<HotfixBlobCache>>,

    // Skill store (auto-learned spells from SkillLineAbility.db2 + SkillRaceClassInfo.db2)
    skill_store: Option<Arc<SkillStore>>,

    // Area trigger store (collision detection + teleportation)
    area_trigger_store: Option<Arc<AreaTriggerStore>>,

    // Shared player registry for broadcasting to nearby sessions
    player_registry: Option<Arc<PlayerRegistry>>,

    // Shared group registry for party management
    group_registry: Option<Arc<GroupRegistry>>,

    // Pending party invites: invited_guid → inviter_guid
    pending_invites: Option<Arc<PendingInvites>>,

    // Group state for this session
    pub(crate) group_guid: Option<u64>,

    // Realm ID for GUID creation
    realm_id: u16,

    // GUID generator for new characters
    guid_generator: Option<Arc<ObjectGuidGenerator>>,

    // Characters confirmed for this account
    legit_characters: Vec<ObjectGuid>,

    // Pending async packets to process
    pending_packets: Vec<WorldPacket>,

    // ── ConnectTo flow ──────────────────────────────────────────
    /// GUID of the character being logged in (set during PlayerLogin).
    player_loading: Option<ObjectGuid>,

    /// The ConnectToKey.Raw value for the pending instance connection.
    connect_to_key: Option<i64>,

    /// The last ConnectToSerial used (for retry logic).
    connect_to_serial: Option<wow_packet::packets::auth::ConnectToSerial>,

    /// Session manager for ConnectTo flow (shared with instance listener).
    session_mgr: Option<Arc<SessionManager>>,

    /// Instance server address (IP for ConnectTo packet).
    instance_address: [u8; 4],

    /// Instance server port.
    instance_port: u16,

    /// Oneshot receiver for instance link delivery.
    instance_link_rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>,

    // ── Time sync ─────────────────────────────────────────────────
    /// Next sequence index for TimeSyncRequest.
    pub(crate) time_sync_next_counter: u32,

    /// Time remaining until next TimeSyncRequest (in ms).
    pub(crate) time_sync_timer_ms: u32,

    // ── Logout ──────────────────────────────────────────────────────
    /// When set, the session is counting down to logout (20s timer).
    /// `None` means no logout is pending.
    pub(crate) logout_time: Option<Instant>,
    /// Timestamp set when the player enters the world (PlayerLogin).
    pub(crate) login_time: Option<Instant>,
    /// Total played time loaded from DB (seconds).
    pub(crate) total_played_time: u32,
    /// Time played at current level loaded from DB (seconds).
    pub(crate) level_played_time: u32,
    /// Player's current money in copper (1 gold = 10,000 copper).
    /// Loaded from `characters.money` on login; saved on logout + buy/sell.
    pub(crate) player_gold: u64,
    /// Currently selected target GUID (SetSelection).
    pub(crate) selection_guid: Option<wow_core::ObjectGuid>,

    /// GUID of the character currently logged in (set after login completes).
    pub(crate) player_guid: Option<ObjectGuid>,

    /// Pending creature spawn request (set during login, processed async).
    pub(crate) pending_creature_spawn: Option<PendingCreatureSpawn>,
    /// Creatures waiting to respawn after corpse despawn.
    pub(crate) respawn_queue: Vec<PendingRespawn>,

    /// In-memory inventory: slot → (item ObjectGuid, entry_id, db_guid).
    pub(crate) inventory_items: HashMap<u8, InventoryItem>,

    /// Current map ID for VALUES update packets.
    pub(crate) current_map_id: u16,

    /// Race of the currently logged-in character (set at login).
    pub(crate) player_race: u8,
    /// Class of the currently logged-in character (set at login).
    pub(crate) player_class: u8,
    /// Level of the currently logged-in character (set at login).
    pub(crate) player_level: u8,
    /// Gender of the currently logged-in character (set at login).
    pub(crate) player_gender: u8,
    /// All known spell IDs for the logged-in character (DB + DBC merged).
    pub(crate) known_spells: Vec<i32>,

    // ── Dual-connection (realm + instance) ───────────────────────
    // After ConnectTo completes, the session uses the instance socket for
    // game packets but MUST keep the realm socket alive — the WoW client
    // disconnects if either connection drops.
    /// Realm packet receiver — kept alive after ConnectTo to prevent realm
    /// socket closure.  Also drained in `update()` for realm-type packets.
    realm_packet_rx: Option<flume::Receiver<WorldPacket>>,
    /// Realm send channel — kept alive so the realm writer task persists.
    realm_send_tx: Option<flume::Sender<Vec<u8>>>,

    // ── Movement & World position ─────────────────────────────────
    /// Server-side position of the player (updated from CMSG_MOVE_*).
    pub(crate) player_position: Option<wow_core::Position>,

    /// Cached character name for chat messages.
    pub(crate) player_name: Option<String>,

    // ── Creature AI tracking ──────────────────────────────────────
    /// All creatures visible/tracked by this session, keyed by GUID.
    pub(crate) creatures: std::collections::HashMap<wow_core::ObjectGuid, wow_ai::CreatureAI>,

    /// Tick counter for creature movement (throttle to every N ticks).
    pub(crate) creature_tick: u32,

    // ── Combat state ─────────────────────────────────────────────
    /// Current auto-attack target (None if not in combat).
    pub(crate) combat_target: Option<wow_core::ObjectGuid>,

    /// True when the player is engaged in combat.
    pub(crate) in_combat: bool,

    // ── Aura system ───────────────────────────────────────────────
    /// All visible auras on the player: slot (0-254) → AuraApplication
    pub(crate) visible_auras: HashMap<u8, AuraApplication>,

    // ── Spell casting ──────────────────────────────────────────────
    /// Spell store (metadata for all known spells: cast time, cooldown, effects, etc.)
    pub spell_store: Option<Arc<SpellStore>>,
    /// Currently active spell cast (if any). Set when a cast starts, cleared when it completes.
    pub(crate) active_spell_cast: Option<SpellCastState>,
    /// Last time a spell was executed (used to enforce global cooldown timers).
    pub(crate) last_spell_cast_time: Option<Instant>,
    /// Per-spell cooldown tracking: spell_id → last cast time.
    /// Used to enforce spell-specific cooldown timers.
    pub(crate) last_spell_cast_time_per_spell: HashMap<i32, Instant>,

    // ── Quest system ───────────────────────────────────────────────
    /// Quest template store (loaded from world DB at startup).
    pub(crate) quest_store: Option<Arc<wow_data::quest::QuestStore>>,
    /// Active quests for this player: quest_id → status.
    pub(crate) player_quests: HashMap<u32, crate::handlers::quest::PlayerQuestStatus>,
    /// Quests the player has already been rewarded for (non-repeatable quests cannot be re-taken).
    /// C# ref: m_RewardedQuests
    pub(crate) rewarded_quests: std::collections::HashSet<u32>,

    // ── Loot ──────────────────────────────────────────────────────
    /// Active loot windows keyed by creature GUID.
    pub(crate) loot_table: std::collections::HashMap<wow_core::ObjectGuid, wow_packet::packets::loot::CreatureLoot>,

    // ── Dynamic visibility tracking ───────────────────────────────
    /// GUIDs of all creatures currently visible to this client.
    /// Updated on login and each visibility refresh (player movement).
    pub(crate) visible_creatures: std::collections::HashSet<wow_core::ObjectGuid>,
    /// GUIDs of all game objects currently visible to this client.
    pub(crate) visible_gameobjects: std::collections::HashSet<wow_core::ObjectGuid>,
    /// Position at which visibility was last fully recalculated.
    pub(crate) last_visibility_pos: Option<wow_core::Position>,

    // ── Gossip state ──────────────────────────────────────────────
    /// Active gossip options for the NPC the player is talking to.
    /// Stored when SMSG_GOSSIP_MESSAGE is sent, used when CMSG_GOSSIP_SELECT_OPTION arrives.
    pub(crate) gossip_options: Vec<GossipOptionInfo>,
    /// GUID of the NPC the current gossip menu belongs to.
    pub(crate) gossip_source_guid: Option<wow_core::ObjectGuid>,

    // ── Area trigger tracking ──────────────────────────────────────
    /// Currently active area trigger ID (to prevent retriggering on same position).
    /// Set to Some(trigger_id) when entered, None when exited.
    pub(crate) active_area_trigger: Option<u32>,
    /// Pending far teleport destination (map_id, position).
    /// Set by `teleport_to`, consumed by `handle_world_port_response`.
    pub(crate) pending_teleport: Option<(u32, wow_core::Position)>,

    // ── QueryCreature cache ────────────────────────────────────────
    /// Creature entry IDs for which we've already sent a QueryCreatureResponse.
    /// The client caches the response locally, so we skip duplicates.
    pub(crate) creature_query_cache: std::collections::HashSet<u32>,
}

/// A gossip option stored server-side for routing when the player selects it.
#[derive(Debug, Clone)]
pub struct GossipOptionInfo {
    pub gossip_option_id: i32,
    pub option_npc: u8,
    pub action_menu_id: u32,
}

/// An item tracked in the session's in-memory inventory.
#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub db_guid: u64,
    /// InventoryType from Item.db2 (e.g. 1=Head, 5=Chest, 13=Weapon).
    /// Loaded from the item store at login, with slot-based fallback.
    pub inventory_type: Option<u8>,
}

/// An aura applied to the player.
#[derive(Debug, Clone)]
pub struct AuraApplication {
    /// Spell ID of the aura
    pub spell_id: i32,
    /// GUID of the unit that cast the aura
    pub caster_guid: ObjectGuid,
    /// Aura slot (0-254)
    pub slot: u8,
    /// Total duration in milliseconds (0 = permanent)
    pub duration_total: u32,
    /// Remaining duration in milliseconds
    pub duration_remaining: u32,
    /// Stack count
    pub stack_count: u8,
    /// Aura flags (bitmask)
    pub aura_flags: u32,
    /// Monotonic timestamp when this aura was applied — used for expiry checks.
    pub applied_at: Instant,
}

/// Parameters for spawning nearby creatures after login.
pub struct PendingCreatureSpawn {
    pub map_id: u16,
    pub position: wow_core::Position,
    pub zone_id: u32,
}

/// A creature waiting to respawn after its corpse despawned.
///
/// Stored in `WorldSession::respawn_queue`; processed by `tick_creatures_sync`.
/// C# ref: `Creature::AllLootRemovedFromCorpse` → `m_respawnTime` → `Map::AddToMap`.
pub struct PendingRespawn {
    /// When to respawn.
    pub respawn_at: std::time::Instant,
    /// Home position (spawn point).
    pub home_pos: wow_core::Position,
    /// Full create data — reused verbatim for the respawn CREATE packet.
    pub create_data: wow_packet::packets::update::CreatureCreateData,
    /// AI fields needed to rebuild `CreatureAI`.
    pub max_hp: u32,
    pub level: u8,
    pub min_dmg: u32,
    pub max_dmg: u32,
    pub aggro_radius: f32,
    pub npc_flags: u32,
    pub unit_flags: u32,
    pub map_id: u16,
}

impl WorldSession {
    /// Create a new session with the given account info and channels.
    pub fn new(
        account_id: u32,
        account_name: String,
        security: u8,
        expansion: u8,
        account_expansion: u8,
        build: u32,
        session_key: Vec<u8>,
        locale: String,
        packet_rx: flume::Receiver<WorldPacket>,
        send_tx: flume::Sender<Vec<u8>>,
    ) -> Self {
        Self {
            account_id,
            account_name,
            security,
            expansion,
            account_expansion,
            build,
            session_key,
            locale,
            packet_rx,
            send_tx,
            state: SessionState::Authed,
            last_packet_time: Instant::now(),
            dispatch_table: build_dispatch_table(),
            char_db: None,
            login_db: None,
            world_db: None,
            item_store: None,
            player_stats: None,
            item_stats_store: None,
            hotfix_blob_cache: None,
            skill_store: None,
            area_trigger_store: None,
            player_registry: None,
            group_registry: None,
            pending_invites: None,
            group_guid: None,
            realm_id: 1,
            guid_generator: None,
            legit_characters: Vec::new(),
            pending_packets: Vec::new(),
            player_loading: None,
            connect_to_key: None,
            connect_to_serial: None,
            session_mgr: None,
            instance_address: [127, 0, 0, 1],
            instance_port: 8086,
            instance_link_rx: None,
            time_sync_next_counter: 0,
            time_sync_timer_ms: 0,
            logout_time: None,
            login_time: None,
            total_played_time: 0,
            level_played_time: 0,
            player_gold: 0,
            selection_guid: None,
            player_guid: None,
            pending_creature_spawn: None,
            respawn_queue: Vec::new(),
            inventory_items: HashMap::new(),
            current_map_id: 0,
            player_race: 0,
            player_class: 0,
            player_level: 0,
            player_gender: 0,
            known_spells: Vec::new(),
            realm_packet_rx: None,
            realm_send_tx: None,
            player_position: None,
            player_name: None,
            creatures: std::collections::HashMap::new(),
            creature_tick: 0,
            combat_target: None,
            in_combat: false,
            visible_auras: HashMap::new(),
            spell_store: None,
            quest_store: None,
            player_quests: HashMap::new(),
            rewarded_quests: std::collections::HashSet::new(),
            active_spell_cast: None,
            last_spell_cast_time: None,
            last_spell_cast_time_per_spell: HashMap::new(),
            loot_table: std::collections::HashMap::new(),
            visible_creatures: std::collections::HashSet::new(),
            visible_gameobjects: std::collections::HashSet::new(),
            last_visibility_pos: None,
            gossip_options: Vec::new(),
            gossip_source_guid: None,
            active_area_trigger: None,
            pending_teleport: None,
            creature_query_cache: std::collections::HashSet::new(),
        }
    }

    /// Set the character database for this session.
    pub fn set_char_db(&mut self, db: Arc<CharacterDatabase>) {
        self.char_db = Some(db);
    }

    /// Set the realm ID for GUID creation.
    pub fn set_realm_id(&mut self, realm_id: u16) {
        self.realm_id = realm_id;
    }

    /// Compute the Virtual Realm Address: `(Region << 24) | (Battlegroup << 16) | RealmId`.
    ///
    /// Region and Battlegroup come from the `realmlist` table in the auth database.
    /// For a typical single-realm setup: Region=1, Battlegroup=1.
    pub(crate) fn virtual_realm_address(&self) -> u32 {
        // TODO: read Region/Battlegroup from realmlist table instead of hardcoding
        let region: u32 = 1;
        let battlegroup: u32 = 1;
        (region << 24) | (battlegroup << 16) | u32::from(self.realm_id)
    }

    /// Set the GUID generator for new characters.
    pub fn set_guid_generator(&mut self, generator: Arc<ObjectGuidGenerator>) {
        self.guid_generator = Some(generator);
    }

    /// Set the login database for this session.
    pub fn set_login_db(&mut self, db: Arc<LoginDatabase>) {
        self.login_db = Some(db);
    }

    /// Get the character database reference.
    pub fn char_db(&self) -> Option<&Arc<CharacterDatabase>> {
        self.char_db.as_ref()
    }

    /// Get the login database reference.
    pub fn login_db(&self) -> Option<&Arc<LoginDatabase>> {
        self.login_db.as_ref()
    }

    /// Set the world database for this session.
    pub fn set_world_db(&mut self, db: Arc<WorldDatabase>) {
        self.world_db = Some(db);
    }

    /// Get the world database reference.
    pub fn world_db(&self) -> Option<&Arc<WorldDatabase>> {
        self.world_db.as_ref()
    }

    /// Set the item store for this session.
    pub fn set_item_store(&mut self, store: Arc<ItemStore>) {
        self.item_store = Some(store);
    }

    /// Get the item store reference.
    pub fn item_store(&self) -> Option<&Arc<ItemStore>> {
        self.item_store.as_ref()
    }

    /// Set the player stats store for this session.
    pub fn set_player_stats(&mut self, store: Arc<PlayerStatsStore>) {
        self.player_stats = Some(store);
    }

    /// Get the player stats store reference.
    pub fn player_stats(&self) -> Option<&Arc<PlayerStatsStore>> {
        self.player_stats.as_ref()
    }

    /// Set the item stats store for this session.
    pub fn set_item_stats_store(&mut self, store: Arc<ItemStatsStore>) {
        self.item_stats_store = Some(store);
    }

    /// Get the item stats store reference.
    pub fn item_stats_store(&self) -> Option<&Arc<ItemStatsStore>> {
        self.item_stats_store.as_ref()
    }

    /// Set the hotfix blob cache for this session.
    pub fn set_hotfix_blob_cache(&mut self, cache: Arc<HotfixBlobCache>) {
        self.hotfix_blob_cache = Some(cache);
    }

    /// Get the hotfix blob cache reference.
    pub fn hotfix_blob_cache(&self) -> Option<&Arc<HotfixBlobCache>> {
        self.hotfix_blob_cache.as_ref()
    }

    /// Set the area trigger store for this session.
    pub fn set_area_trigger_store(&mut self, store: Arc<AreaTriggerStore>) {
        self.area_trigger_store = Some(store);
    }

    /// Get the area trigger store reference.
    pub fn area_trigger_store(&self) -> Option<&Arc<AreaTriggerStore>> {
        self.area_trigger_store.as_ref()
    }

    /// Set the skill store for this session.
    pub fn set_skill_store(&mut self, store: Arc<SkillStore>) {
        self.skill_store = Some(store);
    }

    /// Get the skill store reference.
    pub fn skill_store(&self) -> Option<&Arc<SkillStore>> {
        self.skill_store.as_ref()
    }

    /// Set the spell store for this session.
    pub fn set_spell_store(&mut self, store: Arc<SpellStore>) {
        self.spell_store = Some(store);
    }

    /// Get the spell store reference.
    pub fn spell_store(&self) -> Option<&Arc<SpellStore>> {
        self.spell_store.as_ref()
    }

    /// Set the quest store shared reference.
    pub fn set_quest_store(&mut self, store: Arc<wow_data::quest::QuestStore>) {
        self.quest_store = Some(store);
    }

    /// Save current player gold to the characters DB.
    pub(crate) async fn save_player_gold(&self) {
        use wow_database::CharStatements;
        let guid = match self.player_guid { Some(g) => g.counter() as u32, None => return };
        let char_db = match self.char_db() { Some(db) => Arc::clone(db), None => return };
        let mut stmt = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        stmt.set_u64(0, self.player_gold);
        stmt.set_u32(1, guid);
        let _ = char_db.execute(&stmt).await;
    }

    /// Called when the player kills a creature. Checks all active kill-objective quests
    /// and updates progress. Sends SMSG_QUEST_UPDATE_ADD_CREDIT if progress was made.
    pub(crate) async fn on_creature_killed(
        &mut self,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        use wow_packet::packets::quest::{QuestUpdateAddCredit, QuestUpdateComplete};
        use wow_packet::ServerPacket;

        let Some(store) = self.quest_store.clone() else { return };

        // Objective type 0 = Monster/NPC kill
        const OBJ_TYPE_MONSTER: u8 = 0;

        // Collect quest IDs that have a matching kill objective to avoid borrow issues
        let matching: Vec<(u32, usize, i32)> = self.player_quests.values()
            .filter(|qs| qs.status == 1) // only incomplete quests
            .filter_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                for (i, obj) in quest.objectives.iter().enumerate() {
                    if obj.obj_type == OBJ_TYPE_MONSTER && obj.object_id == creature_entry as i32 {
                        let idx = obj.storage_index.max(0) as usize;
                        return Some((qs.quest_id, idx, obj.amount));
                    }
                }
                None
            })
            .collect();

        for (quest_id, obj_idx, required) in matching {
            let Some(qs) = self.player_quests.get_mut(&quest_id) else { continue };
            if qs.objective_counts.len() <= obj_idx {
                qs.objective_counts.resize(obj_idx + 1, 0);
            }
            if qs.objective_counts[obj_idx] >= required {
                continue; // Already done
            }
            qs.objective_counts[obj_idx] += 1;
            let current = qs.objective_counts[obj_idx];

            debug!(
                account = self.account_id,
                quest_id, obj_idx, current, required,
                "Quest kill objective progress"
            );

            // SMSG_QUEST_UPDATE_ADD_CREDIT
            self.send_packet(&QuestUpdateAddCredit {
                victim_guid: creature_guid,
                quest_id,
                object_id: creature_entry as i32,
                count: current as u16,
                required: required as u16,
                objective_type: OBJ_TYPE_MONSTER,
            });

            // Check if quest is now complete (all objectives satisfied)
            let all_done = {
                let quest = store.get(quest_id);
                quest.map_or(false, |q| {
                    let qs = self.player_quests.get(&quest_id).unwrap();
                    q.objectives.iter().enumerate().all(|(i, obj)| {
                        let idx = obj.storage_index.max(0) as usize;
                        let progress = qs.objective_counts.get(idx).copied().unwrap_or(0);
                        progress >= obj.amount
                    })
                })
            };

            if all_done {
                if let Some(qs) = self.player_quests.get_mut(&quest_id) {
                    qs.status = 2; // Complete
                }
                self.send_packet(&QuestUpdateComplete { quest_id });
                info!(account = self.account_id, quest_id, "Quest objectives complete");
            }
        }
    }

    /// Calculate XP reward for a quest based on difficulty index and player level.
    /// Simplified version — TrinityCore uses quest_xp.db2 table.
    /// Difficulty 0–9 map to base values; reduced for grey quests.
    pub(crate) fn calculate_quest_xp(&self, difficulty: u32, _player_level: u32) -> u32 {
        // Base XP table by difficulty (approximate WotLK values)
        const XP_TABLE: [u32; 10] = [0, 50, 100, 200, 400, 650, 1000, 1500, 2500, 4000];
        let idx = difficulty.min(9) as usize;
        XP_TABLE[idx]
    }

    /// Check if a spell is on cooldown (global or per-spell).
    ///
    /// Returns true if either the global cooldown (1500ms) or the spell-specific
    /// cooldown from SpellStore is still active.
    pub fn is_spell_on_cooldown(&self, spell_id: i32) -> bool {
        let Some(last_cast) = self.last_spell_cast_time else {
            return false; // Never casted
        };

        let elapsed_ms = last_cast.elapsed().as_millis() as u32;

        // Global cooldown: 1500ms
        if elapsed_ms < 1500 {
            return true;
        }

        // Per-spell cooldown (if exists in SpellStore)
        if let Some(store) = &self.spell_store {
            if let Some(spell_info) = store.get(spell_id) {
                if elapsed_ms < spell_info.cooldown_ms {
                    return true;
                }
            }
        }

        false
    }

    /// Set the shared player registry (used for broadcast).
    pub fn set_player_registry(&mut self, registry: Arc<PlayerRegistry>) {
        self.player_registry = Some(registry);
    }

    /// Get a reference to the shared player registry.
    pub fn player_registry(&self) -> Option<&Arc<PlayerRegistry>> {
        self.player_registry.as_ref()
    }

    /// Set the shared group registry and pending invites.
    pub fn set_group_registry(&mut self, reg: Arc<GroupRegistry>, invites: Arc<PendingInvites>) {
        self.group_registry = Some(reg);
        self.pending_invites = Some(invites);
    }

    /// Get a reference to the shared group registry.
    pub fn group_registry(&self) -> Option<&Arc<GroupRegistry>> {
        self.group_registry.as_ref()
    }

    /// Get a reference to the shared pending invites map.
    pub fn pending_invites(&self) -> Option<&Arc<PendingInvites>> {
        self.pending_invites.as_ref()
    }

    /// Register this session in the player registry.
    /// Called after player login is complete (player_guid + position both set).
    pub(crate) fn register_in_player_registry(&self) {
        use crate::handlers::character::default_display_id;
        let (Some(guid), Some(pos), Some(name), Some(reg)) = (
            self.player_guid,
            self.player_position,
            &self.player_name,
            &self.player_registry,
        ) else {
            return;
        };
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        for (slot, item) in &self.inventory_items {
            if (*slot as usize) < 19 {
                visible_items[*slot as usize] = (item.entry_id as i32, 0u16, 0u16);
            }
        }
        reg.insert(guid, PlayerBroadcastInfo {
            map_id: self.current_map_id,
            position: pos,
            send_tx: self.send_tx.clone(),
            player_name: name.clone(),
            account_id: self.account_id,
            race: self.player_race,
            class: self.player_class,
            sex: self.player_gender,
            level: self.player_level,
            display_id: default_display_id(self.player_race, self.player_gender),
            visible_items,
        });
        debug!(
            "Registered player {:?} ({}) in broadcast registry (map {})",
            guid, name, self.current_map_id
        );
    }

    /// Remove this session from the player registry.
    /// Called on logout or disconnect.
    pub(crate) fn unregister_from_player_registry(&self) {
        let (Some(guid), Some(reg)) = (self.player_guid, &self.player_registry) else {
            return;
        };
        reg.remove(&guid);
        debug!("Unregistered player {:?} from broadcast registry", guid);
    }

    /// Update this session's position (and map) in the player registry.
    /// Called whenever `player_position` changes.
    pub(crate) fn update_registry_position(&self) {
        let (Some(guid), Some(pos), Some(reg)) = (
            self.player_guid,
            self.player_position,
            &self.player_registry,
        ) else {
            return;
        };
        if let Some(mut entry) = reg.get_mut(&guid) {
            entry.position = pos;
            entry.map_id = self.current_map_id;
        }
    }

    /// Get the realm ID.
    pub fn realm_id(&self) -> u16 {
        self.realm_id
    }

    /// Get the GUID generator.
    pub fn guid_generator(&self) -> Option<&Arc<ObjectGuidGenerator>> {
        self.guid_generator.as_ref()
    }

    /// Set the session manager for ConnectTo flow.
    pub fn set_session_mgr(&mut self, mgr: Arc<SessionManager>) {
        self.session_mgr = Some(mgr);
    }

    /// Set the instance server address and port.
    pub fn set_instance_endpoint(&mut self, addr: [u8; 4], port: u16) {
        self.instance_address = addr;
        self.instance_port = port;
    }

    /// Get the session manager reference.
    pub fn session_mgr(&self) -> Option<&Arc<SessionManager>> {
        self.session_mgr.as_ref()
    }

    /// Get the instance server address.
    pub fn instance_address(&self) -> [u8; 4] {
        self.instance_address
    }

    /// Get the instance server port.
    pub fn instance_port(&self) -> u16 {
        self.instance_port
    }

    /// Set the player loading GUID (ConnectTo flow).
    pub fn set_player_loading(&mut self, guid: Option<ObjectGuid>) {
        self.player_loading = guid;
    }

    /// Get the player loading GUID.
    pub fn player_loading(&self) -> Option<ObjectGuid> {
        self.player_loading
    }

    /// Set the ConnectTo key.
    pub fn set_connect_to_key(&mut self, key: Option<i64>) {
        self.connect_to_key = key;
    }

    /// Set the ConnectTo serial.
    pub fn set_connect_to_serial(&mut self, serial: Option<wow_packet::packets::auth::ConnectToSerial>) {
        self.connect_to_serial = serial;
    }

    /// Set the instance link receiver.
    pub fn set_instance_link_rx(&mut self, rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>) {
        self.instance_link_rx = rx;
    }

    /// Get a clone of the send channel.
    pub fn send_tx(&self) -> &flume::Sender<Vec<u8>> {
        &self.send_tx
    }

    /// Set the list of legitimate characters for this account.
    pub fn set_legit_characters(&mut self, guids: Vec<ObjectGuid>) {
        self.legit_characters = guids;
    }

    /// Check if a GUID is in the legit characters list.
    pub fn is_legit_character(&self, guid: &ObjectGuid) -> bool {
        self.legit_characters.contains(guid)
    }

    /// Remove a GUID from the legit characters list.
    pub fn remove_legit_character(&mut self, guid: &ObjectGuid) {
        self.legit_characters.retain(|g| g != guid);
    }

    /// Process queued packets (up to [`MAX_PACKETS_PER_UPDATE`] per call).
    ///
    /// Returns the number of packets processed.
    pub fn update(&mut self, diff_ms: u32) -> usize {
        let mut processed = 0;

        // Drain the primary (instance) packet channel
        while processed < MAX_PACKETS_PER_UPDATE {

            let pkt = match self.packet_rx.try_recv() {
                Ok(p) => p,
                Err(flume::TryRecvError::Empty) => break,
                Err(flume::TryRecvError::Disconnected) => {
                    debug!("Packet channel disconnected for account {}", self.account_id);
                    self.state = SessionState::Disconnecting;
                    break;
                }
            };

            self.last_packet_time = Instant::now();
            self.pending_packets.push(pkt);
            processed += 1;
        }

        // Also drain the realm socket channel (after ConnectTo, realm-type
        // packets like BattlenetRequest, Ping, etc. arrive here)
        if let Some(ref realm_rx) = self.realm_packet_rx {
            while processed < MAX_PACKETS_PER_UPDATE {
                match realm_rx.try_recv() {
                    Ok(pkt) => {
                        self.last_packet_time = Instant::now();
                        self.pending_packets.push(pkt);
                        processed += 1;
                    }
                    Err(flume::TryRecvError::Empty) => break,
                    Err(flume::TryRecvError::Disconnected) => {
                        info!(
                            "Realm socket disconnected for account {} (instance still active)",
                            self.account_id
                        );
                        // Realm dropped — don't disconnect immediately, the
                        // instance socket may still be fine.
                        self.realm_packet_rx = None;
                        break;
                    }
                }
            }
        }

        // ── Creature AI tick ─────────────────────────────────────────
        // Throttle to every 4 ticks (~200ms at 50ms tick).
        if self.state == SessionState::LoggedIn {
            self.creature_tick = self.creature_tick.wrapping_add(1);
            if self.creature_tick % 4 == 0 {
                self.tick_creatures_sync();
            }
            // Combat tick every 2 ticks (~100ms)
            if self.creature_tick % 2 == 0 {
                self.tick_combat_sync();
            }
            // Aura expiry tick every 4 ticks (~200ms)
            if self.creature_tick % 4 == 0 {
                self.tick_auras();
            }
        }

        // ── Periodic TimeSyncRequest ──────────────────────────────
        // C# sends first resync 5s after login, then every 10s.
        // The client MUST receive periodic TimeSyncRequests or its
        // internal clock sync state becomes inconsistent → crash.
        if self.state == SessionState::LoggedIn && self.time_sync_timer_ms > 0 {
            if diff_ms >= self.time_sync_timer_ms {
                self.send_time_sync();
            } else {
                self.time_sync_timer_ms -= diff_ms;
            }
        }

        // ── Logout timer ────────────────────────────────────────────
        if let Some(logout_time) = self.logout_time {
            if Instant::now() >= logout_time {
                self.logout_time = None;
                self.complete_logout();
            }
        }

        processed
    }

    // ── Aura system ───────────────────────────────────────────────

    /// Apply an aura to the player and send SMSG_AURA_UPDATE.
    pub fn apply_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
    ) -> Result<(), &'static str> {

        // Find a free slot (0-254)
        let mut slot = 0u8;
        while self.visible_auras.contains_key(&slot) && slot < 255 {
            slot += 1;
        }

        if slot >= 255 {
            return Err("No free aura slots");
        }

        // Create aura
        let aura = AuraApplication {
            spell_id,
            caster_guid,
            slot,
            duration_total: duration_ms,
            duration_remaining: duration_ms,
            stack_count: 1,
            aura_flags,
            applied_at: Instant::now(),
        };

        self.visible_auras.insert(slot, aura);

        // Send SMSG_AURA_UPDATE
        self.send_aura_update_applied(spell_id, slot, caster_guid, duration_ms, aura_flags);

        Ok(())
    }

    /// Remove an aura by slot and send SMSG_AURA_UPDATE.
    pub fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {

        if self.visible_auras.remove(&slot).is_none() {
            return Err("Aura slot not found");
        }

        // Send SMSG_AURA_UPDATE (removal)
        self.send_aura_update_removed(slot);

        Ok(())
    }

    /// Check all active auras for expiry and remove those whose duration has elapsed.
    /// Called from the synchronous tick loop (~every 200ms via creature_tick).
    pub(crate) fn tick_auras(&mut self) {
        if self.visible_auras.is_empty() {
            return;
        }

        // Collect expired slots (avoid borrow conflict)
        let expired: Vec<u8> = self
            .visible_auras
            .values()
            .filter(|a| {
                // Permanent auras (duration_total == 0) never expire
                a.duration_total > 0
                    && a.applied_at.elapsed().as_millis() as u32 >= a.duration_total
            })
            .map(|a| a.slot)
            .collect();

        for slot in expired {
            let spell_id = self.visible_auras.get(&slot).map(|a| a.spell_id).unwrap_or(0);
            let _ = self.remove_aura(slot);
            debug!(
                account = self.account_id,
                slot = slot,
                spell_id = spell_id,
                "Aura expired"
            );
        }
    }

    fn send_aura_update_applied(
        &self,
        spell_id: i32,
        slot: u8,
        caster: ObjectGuid,
        duration: u32,
        flags: u32,
    ) {
        use wow_packet::packets::aura::{AuraUpdate, AuraData};
        use wow_packet::ServerPacket;

        let update = AuraUpdate {
            target_guid: self.player_guid.unwrap_or(ObjectGuid::EMPTY),
            updated_auras: vec![AuraData {
                slot,
                spell_id,
                aura_flags: flags,
                duration_total: duration,
                duration_remaining: duration,
                stack_count: 1,
                caster_guid: caster,
                effect_data: None,
            }],
            removed_aura_slots: vec![],
        };
        self.send_packet(&update);
    }

    fn send_aura_update_removed(&self, slot: u8) {
        use wow_packet::packets::aura::AuraUpdate;

        let update = AuraUpdate {
            target_guid: self.player_guid.unwrap_or(ObjectGuid::EMPTY),
            updated_auras: vec![],
            removed_aura_slots: vec![slot],
        };
        self.send_packet(&update);
    }

    /// Send a TimeSyncRequest and schedule the next one.
    pub(crate) fn send_time_sync(&mut self) {
        use wow_packet::packets::misc::TimeSyncRequest;
        self.send_packet(&TimeSyncRequest {
            sequence_index: self.time_sync_next_counter,
        });
        trace!(
            "Sent TimeSyncRequest(seq={}) for account {}",
            self.time_sync_next_counter,
            self.account_id
        );
        // First 2 syncs are 5s apart, then every 10s (matches C#)
        self.time_sync_timer_ms = if self.time_sync_next_counter <= 1 {
            5000
        } else {
            10000
        };
        self.time_sync_next_counter += 1;
    }

    /// Process pending packets asynchronously. Call after `update()`.
    pub async fn process_pending(&mut self) {
        // ── Spell casting tick ─────────────────────────────────────────
        // Check if an active spell cast has completed and execute it.
        if self.state == SessionState::LoggedIn {
            self.tick_active_spell_cast().await;
        }

        // Check for instance link delivery (ConnectTo flow)
        self.poll_instance_link().await;

        // Process pending creature/gameobject spawn (async DB query)
        if let Some(spawn) = self.pending_creature_spawn.take() {
            self.send_nearby_creatures(spawn.map_id, &spawn.position, spawn.zone_id)
                .await;
            self.send_nearby_gameobjects(spawn.map_id, &spawn.position, spawn.zone_id)
                .await;
        }

        let packets: Vec<WorldPacket> = self.pending_packets.drain(..).collect();
        for pkt in packets {
            self.dispatch_packet(pkt).await;
        }
    }


    /// Poll the instance link oneshot. When received, swap channels and
    /// continue the player login on the instance socket.
    async fn poll_instance_link(&mut self) {
        let rx = match self.instance_link_rx.as_mut() {
            Some(rx) => rx,
            None => return,
        };

        // Non-blocking check
        match rx.try_recv() {
            Ok(link) => {
                info!(
                    "Instance link received for account {}, swapping channels",
                    self.account_id
                );

                // Keep the old realm channels alive — if either TCP connection
                // drops the WoW client disconnects the whole session.
                // The realm reader/writer tasks hold the other ends of these
                // channels, so keeping these receivers/senders prevents the
                // realm socket from closing.
                let old_send_tx = std::mem::replace(&mut self.send_tx, link.send_tx);
                self.realm_send_tx = Some(old_send_tx);

                if let Some(pkt_rx) = link.pkt_rx {
                    let old_packet_rx = std::mem::replace(&mut self.packet_rx, pkt_rx);
                    self.realm_packet_rx = Some(old_packet_rx);
                }

                self.instance_link_rx = None;

                // Continue the player login sequence on the instance socket
                self.handle_continue_player_login().await;
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                // Not ready yet, keep waiting
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                warn!(
                    "Instance link channel closed for account {} — instance connection failed",
                    self.account_id
                );
                self.instance_link_rx = None;
                self.player_loading = None;
                self.connect_to_key = None;
            }
        }
    }

    /// Dispatch a single packet to its registered handler.
    async fn dispatch_packet(&mut self, mut pkt: WorldPacket) {
        let opcode_raw = pkt.opcode_raw();
        let opcode: ClientOpcodes = match num_traits::FromPrimitive::from_u32(u32::from(opcode_raw)) {
            Some(op) => op,
            None => {
                info!(
                    "Unknown client opcode 0x{opcode_raw:04X} from account {}",
                    self.account_id
                );
                return;
            }
        };

        let entry = match self.dispatch_table.get(&opcode) {
            Some(e) => *e,
            None => {
                info!(
                    "No handler for {:?} (0x{opcode_raw:04X}) from account {}",
                    opcode,
                    self.account_id
                );
                return;
            }
        };

        // Check session status
        if !self.is_status_allowed(entry.status) {
            warn!(
                "Handler {} rejected: session state {:?} doesn't match required {:?}",
                entry.handler_name, self.state, entry.status
            );
            return;
        }

        debug!(
            "Dispatching {:?} via {} for account {}",
            opcode, entry.handler_name, self.account_id
        );

        // Skip opcode before reading payload
        pkt.skip_opcode();

        match opcode {
            ClientOpcodes::EnumCharacters => {
                self.handle_enum_characters().await;
            }
            ClientOpcodes::CreateCharacter => {
                match wow_packet::packets::character::CreateCharacter::read(&mut pkt) {
                    Ok(create) => self.handle_create_character(create).await,
                    Err(e) => warn!("Failed to read CreateCharacter: {e}"),
                }
            }
            ClientOpcodes::CharDelete => {
                match wow_packet::packets::character::CharDelete::read(&mut pkt) {
                    Ok(del) => self.handle_char_delete(del).await,
                    Err(e) => warn!("Failed to read CharDelete: {e}"),
                }
            }
            ClientOpcodes::PlayerLogin => {
                match wow_packet::packets::character::PlayerLogin::read(&mut pkt) {
                    Ok(login) => self.handle_player_login(login).await,
                    Err(e) => warn!("Failed to read PlayerLogin: {e}"),
                }
            }
            ClientOpcodes::ConnectToFailed => {
                match wow_packet::packets::auth::ConnectToFailed::read(&mut pkt) {
                    Ok(failed) => self.handle_connect_to_failed(failed).await,
                    Err(e) => warn!("Failed to read ConnectToFailed: {e}"),
                }
            }
            ClientOpcodes::GetUndeleteCharacterCooldownStatus => {
                self.handle_get_undelete_cooldown_status().await;
            }
            ClientOpcodes::BattlenetRequest => {
                match wow_packet::packets::battlenet::BattlenetRequest::read(&mut pkt) {
                    Ok(req) => self.handle_battlenet_request(req).await,
                    Err(e) => warn!("Failed to read BattlenetRequest: {e}"),
                }
            }
            ClientOpcodes::ServerTimeOffsetRequest => {
                self.handle_server_time_offset_request().await;
            }
            ClientOpcodes::RequestPlayedTime => {
                // TriggerScriptEvent: 1 byte bool — mirrors it back in the response.
                let trigger = pkt.read_uint8().unwrap_or(0) != 0;
                self.handle_request_played_time(trigger).await;
            }
            ClientOpcodes::SetSelection => {
                self.handle_set_selection(pkt).await;
            }
            ClientOpcodes::AreaTrigger => {
                self.handle_area_trigger(pkt).await;
            }
            ClientOpcodes::RequestCemeteryList => {
                self.handle_request_cemetery_list(pkt).await;
            }
            ClientOpcodes::TaxiNodeStatusQuery => {
                self.handle_taxi_node_status_query(pkt).await;
            }
            ClientOpcodes::ChatJoinChannel => {
                self.handle_chat_join_channel(pkt).await;
            }
            ClientOpcodes::MoveTimeSkipped => {
                self.handle_move_time_skipped(pkt).await;
            }
            ClientOpcodes::DbQueryBulk => {
                match wow_packet::packets::misc::DbQueryBulk::read(&mut pkt) {
                    Ok(query) => self.handle_db_query_bulk(query).await,
                    Err(e) => warn!("Failed to read DbQueryBulk: {e}"),
                }
            }
            ClientOpcodes::HotfixRequest => {
                match wow_packet::packets::misc::HotfixRequest::read(&mut pkt) {
                    Ok(req) => self.handle_hotfix_request(req).await,
                    Err(e) => warn!("Failed to read HotfixRequest: {e}"),
                }
            }
            ClientOpcodes::TimeSyncResponse => {
                match wow_packet::packets::misc::TimeSyncResponse::read(&mut pkt) {
                    Ok(resp) => self.handle_time_sync_response(resp).await,
                    Err(e) => warn!("Failed to read TimeSyncResponse: {e}"),
                }
            }
            ClientOpcodes::LogoutRequest => {
                match wow_packet::packets::misc::LogoutRequest::read(&mut pkt) {
                    Ok(req) => self.handle_logout_request(req).await,
                    Err(e) => warn!("Failed to read LogoutRequest: {e}"),
                }
            }
            ClientOpcodes::LogoutCancel => {
                self.handle_logout_cancel().await;
            }
            ClientOpcodes::QueryCreature => {
                match wow_packet::packets::query::QueryCreature::read(&mut pkt) {
                    Ok(query) => self.handle_query_creature(query).await,
                    Err(e) => warn!("Failed to read QueryCreature: {e}"),
                }
            }

            ClientOpcodes::QueryGameObject => {
                match wow_packet::packets::query::QueryGameObject::read(&mut pkt) {
                    Ok(query) => self.handle_query_game_object(query).await,
                    Err(e) => warn!("Failed to read QueryGameObject: {e}"),
                }
            }
            ClientOpcodes::QueryPlayerNames => {
                match wow_packet::packets::query::QueryPlayerNames::read(&mut pkt) {
                    Ok(query) => self.handle_query_player_names(query).await,
                    Err(e) => warn!("Failed to read QueryPlayerNames: {e}"),
                }
            }
            ClientOpcodes::QueryRealmName => {
                match wow_packet::packets::query::QueryRealmName::read(&mut pkt) {
                    Ok(query) => self.handle_query_realm_name(query),
                    Err(e) => warn!("Failed to read QueryRealmName: {e}"),
                }
            }
            ClientOpcodes::Ping => {
                match wow_packet::packets::auth::Ping::read(&mut pkt) {
                    Ok(ping) => self.handle_ping(ping).await,
                    Err(e) => warn!("Failed to read Ping: {e}"),
                }
            }
            ClientOpcodes::TalkToGossip => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_gossip_hello(hello).await,
                    Err(e) => warn!("Failed to read TalkToGossip: {e}"),
                }
            }
            ClientOpcodes::AuctionHelloRequest => {
                self.handle_auction_hello_request(pkt).await;
            }
            ClientOpcodes::BankerActivate => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_banker_activate(hello).await,
                    Err(e) => warn!("Failed to read BankerActivate: {e}"),
                }
            }
            ClientOpcodes::BinderActivate => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_binder_activate(hello).await,
                    Err(e) => warn!("Failed to read BinderActivate: {e}"),
                }
            }
            ClientOpcodes::TabardVendorActivate => {
                self.handle_tabard_vendor_activate(pkt).await;
            }
            ClientOpcodes::SpiritHealerActivate => {
                self.handle_spirit_healer_activate(pkt).await;
            }
            ClientOpcodes::RepairItem => {
                self.handle_repair_item(pkt).await;
            }
            ClientOpcodes::RequestStabledPets => {
                self.handle_request_stabled_pets(pkt).await;
            }
            ClientOpcodes::GossipSelectOption => {
                match wow_packet::packets::gossip::GossipSelectOption::read(&mut pkt) {
                    Ok(select) => self.handle_gossip_select_option(select).await,
                    Err(e) => warn!("Failed to read GossipSelectOption: {e}"),
                }
            }
            ClientOpcodes::QueryNpcText => {
                match wow_packet::packets::gossip::QueryNpcText::read(&mut pkt) {
                    Ok(query) => self.handle_query_npc_text(query).await,
                    Err(e) => warn!("Failed to read QueryNpcText: {e}"),
                }
            }
            ClientOpcodes::ListInventory => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_list_inventory(hello).await,
                    Err(e) => warn!("Failed to read ListInventory: {e}"),
                }
            }
            ClientOpcodes::BuyItem => {
                match wow_packet::packets::misc::BuyItem::read(&mut pkt) {
                    Ok(buy) => self.handle_buy_item(buy).await,
                    Err(e) => warn!("Failed to read BuyItem: {e}"),
                }
            }
            ClientOpcodes::SellItem => {
                match wow_packet::packets::misc::SellItem::read(&mut pkt) {
                    Ok(sell) => self.handle_sell_item(sell).await,
                    Err(e) => warn!("Failed to read SellItem: {e}"),
                }
            }
            ClientOpcodes::TrainerList => {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => self.handle_trainer_list(hello).await,
                    Err(e) => warn!("Failed to read TrainerList: {e}"),
                }
            }
            ClientOpcodes::QuestGiverHello => {
                self.handle_quest_giver_hello(pkt).await;
            }
            ClientOpcodes::QuestGiverStatusQuery => {
                self.handle_quest_giver_status_query(pkt).await;
            }
            ClientOpcodes::QuestGiverStatusMultipleQuery => {
                self.handle_quest_giver_status_multiple_query().await;
            }
            ClientOpcodes::SwapInvItem => {
                match wow_packet::packets::item::SwapInvItem::read(&mut pkt) {
                    Ok(swap) => self.handle_swap_inv_item(swap).await,
                    Err(e) => warn!("Failed to read SwapInvItem: {e}"),
                }
            }
            ClientOpcodes::AutoEquipItem => {
                match wow_packet::packets::item::AutoEquipItem::read(&mut pkt) {
                    Ok(equip) => self.handle_auto_equip_item(equip).await,
                    Err(e) => warn!("Failed to read AutoEquipItem: {e}"),
                }
            }
            ClientOpcodes::SwapItem => {
                match wow_packet::packets::item::SwapItem::read(&mut pkt) {
                    Ok(swap) => self.handle_swap_item(swap).await,
                    Err(e) => warn!("Failed to read SwapItem: {e}"),
                }
            }
            ClientOpcodes::AutoStoreBagItem => {
                match wow_packet::packets::item::AutoStoreBagItem::read(&mut pkt) {
                    Ok(store) => self.handle_auto_store_bag_item(store).await,
                    Err(e) => warn!("Failed to read AutoStoreBagItem: {e}"),
                }
            }
            ClientOpcodes::DestroyItem => {
                match wow_packet::packets::item::DestroyItemPkt::read(&mut pkt) {
                    Ok(destroy) => self.handle_destroy_item(destroy).await,
                    Err(e) => warn!("Failed to read DestroyItem: {e}"),
                }
            }
            ClientOpcodes::ShowTradeSkill => {
                match wow_packet::packets::misc::ShowTradeSkill::read(&mut pkt) {
                    Ok(show) => self.handle_show_trade_skill(show).await,
                    Err(e) => warn!("Failed to read ShowTradeSkill: {e}"),
                }
            }
            // ── Movement opcodes (all share the same handler) ───────
            ClientOpcodes::MoveStartForward
            | ClientOpcodes::MoveStartBackward
            | ClientOpcodes::MoveStop
            | ClientOpcodes::MoveStartStrafeLeft
            | ClientOpcodes::MoveStartStrafeRight
            | ClientOpcodes::MoveStopStrafe
            | ClientOpcodes::MoveStartTurnLeft
            | ClientOpcodes::MoveStartTurnRight
            | ClientOpcodes::MoveStopTurn
            | ClientOpcodes::MoveStartPitchUp
            | ClientOpcodes::MoveStartPitchDown
            | ClientOpcodes::MoveStopPitch
            | ClientOpcodes::MoveSetRunMode
            | ClientOpcodes::MoveSetWalkMode
            | ClientOpcodes::MoveHeartbeat
            | ClientOpcodes::MoveFallLand
            | ClientOpcodes::MoveFallReset
            | ClientOpcodes::MoveJump
            | ClientOpcodes::MoveSetFacing
            | ClientOpcodes::MoveSetFacingHeartbeat
            | ClientOpcodes::MoveSetPitch
            | ClientOpcodes::MoveSetFly
            | ClientOpcodes::MoveStartAscend
            | ClientOpcodes::MoveStopAscend
            | ClientOpcodes::MoveStartDescend
            | ClientOpcodes::MoveStartSwim
            | ClientOpcodes::MoveStopSwim
            | ClientOpcodes::MoveUpdateFallSpeed => {
                self.handle_movement(pkt).await;
            }

            // ── Movement control opcodes ────────────────────────────
            ClientOpcodes::SetActiveMover => {
                match wow_packet::packets::movement::SetActiveMover::read(&mut pkt) {
                    Ok(mover) => self.handle_set_active_mover(mover).await,
                    Err(e) => warn!("Failed to read SetActiveMover: {e}"),
                }
            }
            ClientOpcodes::MoveInitActiveMoverComplete => {
                match wow_packet::packets::movement::MoveInitActiveMoverComplete::read(&mut pkt) {
                    Ok(init) => self.handle_move_init_active_mover_complete(init).await,
                    Err(e) => warn!("Failed to read MoveInitActiveMoverComplete: {e}"),
                }
            }

            // ── Combat opcodes ──────────────────────────────────────
            ClientOpcodes::AttackSwing => {
                self.handle_attack_swing(pkt).await;
            }
            ClientOpcodes::AttackStop => {
                self.handle_attack_stop(pkt).await;
            }
            ClientOpcodes::SetSheathed => {
                self.handle_set_sheathed(pkt);
            }

            // ── Loot opcodes ────────────────────────────────────────
            ClientOpcodes::LootUnit => {
                self.handle_loot_unit(pkt).await;
            }
            ClientOpcodes::LootItem => {
                self.handle_loot_item(pkt).await;
            }
            ClientOpcodes::LootRelease => {
                self.handle_loot_release(pkt).await;
            }

            // ── Chat opcodes ────────────────────────────────────────
            ClientOpcodes::ChatMessageSay => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Say).await;
            }
            ClientOpcodes::ChatMessageYell => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Yell).await;
            }
            ClientOpcodes::ChatMessageParty => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Party).await;
            }
            ClientOpcodes::ChatMessageGuild => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Guild).await;
            }
            ClientOpcodes::ChatMessageRaid => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::Raid).await;
            }
            ClientOpcodes::ChatMessageRaidWarning => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::RaidWarning).await;
            }
            ClientOpcodes::ChatMessageInstanceChat => {
                self.handle_chat_message(pkt, wow_packet::packets::chat::ChatMsg::InstanceChat).await;
            }
            ClientOpcodes::ChatMessageWhisper => {
                self.handle_chat_whisper(pkt).await;
            }
            ClientOpcodes::ChatMessageEmote => {
                self.handle_chat_emote(pkt).await;
            }

            // ── Spell cast ────────────────────────────────────────────────────
            ClientOpcodes::CastSpell => {
                self.handle_cast_spell(pkt).await;
            }
            ClientOpcodes::CancelCast => {
                self.handle_cancel_cast(pkt).await;
            }
            ClientOpcodes::CancelChannelling => {
                self.handle_cancel_channelling(pkt).await;
            }

            // ── QueryTime / QueryNextMailTime ─────────────────────────────────
            ClientOpcodes::QueryTime => {
                self.handle_query_time().await;
            }
            ClientOpcodes::QueryNextMailTime => {
                self.handle_query_next_mail_time().await;
            }

            // ── Silent-ignore stubs (login-time client packets, no response) ──
            ClientOpcodes::LoadingScreenNotify => {
                self.handle_loading_screen_notify(pkt).await;
            }
            ClientOpcodes::ViolenceLevel => {
                self.handle_violence_level(pkt).await;
            }
            ClientOpcodes::OverrideScreenFlash => {
                self.handle_override_screen_flash(pkt).await;
            }
            ClientOpcodes::QueuedMessagesEnd => {
                self.handle_queued_messages_end(pkt).await;
            }
            ClientOpcodes::ChatUnregisterAllAddonPrefixes => {
                self.handle_chat_unregister_all_addon_prefixes(pkt).await;
            }
            ClientOpcodes::SetActionBarToggles => {
                self.handle_set_action_bar_toggles(pkt).await;
            }
            ClientOpcodes::SaveCufProfiles => {
                self.handle_save_cuf_profiles(pkt).await;
            }
            ClientOpcodes::GuildSetAchievementTracking => {
                self.handle_guild_set_achievement_tracking(pkt).await;
            }
            ClientOpcodes::GetItemPurchaseData => {
                self.handle_get_item_purchase_data(pkt).await;
            }
            ClientOpcodes::RequestForcedReactions => {
                self.handle_request_forced_reactions(pkt).await;
            }
            ClientOpcodes::RequestBattlefieldStatus => {
                self.handle_request_battlefield_status(pkt).await;
            }
            ClientOpcodes::RequestRatedPvpInfo => {
                self.handle_request_rated_pvp_info(pkt).await;
            }
            ClientOpcodes::RequestPvpRewards => {
                self.handle_request_pvp_rewards(pkt).await;
            }
            ClientOpcodes::DfGetSystemInfo => {
                self.handle_df_get_system_info(pkt).await;
            }
            ClientOpcodes::DfGetJoinStatus => {
                self.handle_df_get_join_status(pkt).await;
            }
            ClientOpcodes::CalendarGetNumPending => {
                self.handle_calendar_get_num_pending(pkt).await;
            }
            ClientOpcodes::GmTicketGetCaseStatus => {
                self.handle_gm_ticket_get_case_status(pkt).await;
            }
            ClientOpcodes::GuildBankRemainingWithdrawMoneyQuery => {
                self.handle_guild_bank_remaining_withdraw_money_query(pkt).await;
            }
            ClientOpcodes::BattlePetRequestJournal => {
                self.handle_battle_pet_request_journal(pkt).await;
            }
            ClientOpcodes::ArenaTeamRoster => {
                self.handle_arena_team_roster(pkt).await;
            }
            ClientOpcodes::RequestRaidInfo => {
                self.handle_request_raid_info(pkt).await;
            }
            ClientOpcodes::RequestConquestFormulaConstants => {
                self.handle_request_conquest_formula_constants(pkt).await;
            }
            ClientOpcodes::RequestLfgListBlacklist => {
                self.handle_request_lfg_list_blacklist(pkt).await;
            }
            ClientOpcodes::LfgListGetStatus => {
                self.handle_lfg_list_get_status(pkt).await;
            }
            ClientOpcodes::GetAccountCharacterList => {
                self.handle_get_account_character_list(pkt).await;
            }
            ClientOpcodes::QueryCountdownTimer => {
                self.handle_request_countdown_timer(pkt).await;
            }
            ClientOpcodes::CalendarGet => {
                self.handle_calendar_get(pkt).await;
            }
            ClientOpcodes::CloseInteraction => {
                self.handle_close_interaction(pkt).await;
            }
            ClientOpcodes::AuctionListBidderItems => {
                self.handle_auction_list_bidder_items(pkt).await;
            }
            ClientOpcodes::AuctionListOwnerItems => {
                self.handle_auction_list_owner_items(pkt).await;
            }
            ClientOpcodes::AuctionListPendingSales => {
                self.handle_auction_list_pending_sales(pkt).await;
            }
            ClientOpcodes::CommerceTokenGetLog => {
                self.handle_commerce_token_get_log(pkt).await;
            }
            ClientOpcodes::GameObjUse => {
                self.handle_game_obj_use(pkt).await;
            }
            ClientOpcodes::GameObjReportUse => {
                self.handle_game_obj_report_use(pkt).await;
            }
            ClientOpcodes::AddFriend => {
                self.handle_add_friend(pkt).await;
            }
            ClientOpcodes::DelFriend => {
                self.handle_del_friend(pkt).await;
            }
            ClientOpcodes::SendContactList => {
                self.handle_send_contact_list(pkt).await;
            }

            // ── Group / Party opcodes ─────────────────────────────────────────
            ClientOpcodes::PartyInvite => {
                self.handle_party_invite(pkt).await;
            }
            ClientOpcodes::PartyInviteResponse => {
                self.handle_party_invite_response(pkt).await;
            }
            ClientOpcodes::LeaveGroup => {
                self.handle_leave_group(pkt).await;
            }

            ClientOpcodes::Inspect => {
                self.handle_inspect(pkt).await;
            }

            // Empty stubs matching C# — these client opcodes are sent during
            // character select but require no response (Blizzard services).
            ClientOpcodes::BattlePayGetProductList
            | ClientOpcodes::BattlePayGetPurchaseList
            | ClientOpcodes::UpdateVasPurchaseStates
            | ClientOpcodes::SocialContractRequest => {
                trace!(
                    "Stub handler for {:?} (0x{:04X}) — no response needed",
                    opcode,
                    opcode_raw
                );
            }
            _ => {
                match entry.processing {
                    PacketProcessing::Inplace => {
                        trace!(
                            "Processing {:?} inplace via {}",
                            opcode,
                            entry.handler_name
                        );
                    }
                    PacketProcessing::ThreadUnsafe => {
                        trace!(
                            "Queuing {:?} for thread-unsafe processing via {}",
                            opcode,
                            entry.handler_name
                        );
                    }
                }
            }
        }
    }

    /// Check if the handler's required status matches the current session state.
    ///
    /// Matches C# WorldSession.Update() switch logic:
    /// - `Authed` → allowed in ANY state (authenticated, in-world, or transferring)
    /// - `LoggedIn` → only when player is in-world
    /// - `Transfer` → only during map transfers
    /// - `LoggedInOrRecentlyLogout` → in-world or recently disconnected
    fn is_status_allowed(&self, required: SessionStatus) -> bool {
        match required {
            SessionStatus::Authed => true, // C#: always allowed once authenticated
            SessionStatus::LoggedIn => self.state == SessionState::LoggedIn,
            SessionStatus::Transfer => self.state == SessionState::Transfer,
            SessionStatus::LoggedInOrRecentlyLogout => {
                self.state == SessionState::LoggedIn
                    || self.state == SessionState::Disconnecting
            }
        }
    }

    /// Check for area triggers at the player's current position.
    ///
    /// This is called after movement updates to handle:
    /// - Teleportation triggers (e.g., dungeon exits)
    /// - Spell effects (e.g., silencing fields)
    /// - Custom trigger actions
    ///
    /// Manages trigger state to prevent retriggering:
    /// - Entry: when player enters a trigger (was not in one)
    /// - Exit: when player leaves a trigger (was in one, no longer is)
    pub async fn check_area_triggers(&mut self) {
        let (Some(pos), Some(store)) = (self.player_position, self.area_trigger_store.as_ref()) else {
            return;
        };

        // Get all triggers at the current position on the player's current map
        let triggers = store.get_triggers_at_position(self.current_map_id, &pos);

        // Check if we've exited the previous trigger
        if let Some(prev_trigger_id) = self.active_area_trigger {
            if !triggers.iter().any(|t| t.trigger_id == prev_trigger_id) {
                info!(
                    account = self.account_id,
                    trigger_id = prev_trigger_id,
                    "Exited area trigger"
                );
                self.active_area_trigger = None;
            }
        }

        // Check if we've entered a new trigger
        if let Some(trigger) = triggers.first() {
            let trigger_id = trigger.trigger_id;

            // Only trigger if this is a NEW trigger (wasn't active before)
            if self.active_area_trigger != Some(trigger_id) {
                info!(
                    account = self.account_id,
                    trigger_id = trigger.trigger_id,
                    "Entered area trigger"
                );
                self.active_area_trigger = Some(trigger_id);

                // Handle teleportation if present
                if let Some(ref teleport) = trigger.teleport {
                    info!(
                        account = self.account_id,
                        trigger_id = trigger.trigger_id,
                        target_map = teleport.target_map,
                        target_x = teleport.target_position.x,
                        target_y = teleport.target_position.y,
                        target_z = teleport.target_position.z,
                        "Teleporting player via area trigger"
                    );
                    self.teleport_to(teleport.target_map, teleport.target_position)
                        .await;
                }
            }
        }
    }

    /// Teleport the player to a new map and position.
    ///
    /// Sends SMSG_TRANSFER_PENDING (0x25cd) to initiate the transfer.
    /// The client will respond with CMSG_WORLD_PORT_ACK when ready.
    ///
    /// C# ref: Player.TeleportTo → SendTransferPending
    pub async fn teleport_to(&mut self, new_map: u32, new_pos: wow_core::Position) {
        // Validate inputs
        if new_map as u16 > 0xFFF {
            warn!(
                "Invalid map ID {} for teleport from account {}",
                new_map, self.account_id
            );
            return;
        }

        let Some(current_pos) = self.player_position else {
            warn!(
                "Cannot teleport account {}: no current position",
                self.account_id
            );
            return;
        };

        info!(
            account = self.account_id,
            old_map = self.current_map_id,
            new_map = new_map,
            old_pos = format!("({:.2}, {:.2}, {:.2})", current_pos.x, current_pos.y, current_pos.z),
            new_pos = format!("({:.2}, {:.2}, {:.2})", new_pos.x, new_pos.y, new_pos.z),
            "Player teleporting to new map"
        );

        use wow_packet::packets::misc::{SuspendToken, TransferPending};

        // 1. SMSG_TRANSFER_PENDING — tell client to start loading screen
        let transfer_pending = TransferPending {
            map_id: new_map,
            old_map_position: current_pos,
            ship: None,
            transfer_spell_id: None,
        };
        self.send_packet(&transfer_pending);

        // 2. Store pending destination — completed in handle_world_port_response
        self.pending_teleport = Some((new_map, new_pos));
        self.active_area_trigger = None;

        // 3. SMSG_SUSPEND_TOKEN — pause movement processing on client
        self.send_packet(&SuspendToken { sequence_index: 1, reason: 1 });

        // 4. Transition to Transfer state — only WorldPortResponse accepted now
        self.state = SessionState::Transfer;

        info!(
            account = self.account_id,
            "Teleport initiated: map {} → {} dest ({:.2}, {:.2}, {:.2}); awaiting WorldPortResponse",
            self.current_map_id, new_map, new_pos.x, new_pos.y, new_pos.z
        );
    }

    /// Send a server packet back to the client via the instance (default) channel.
    pub fn send_packet(&self, pkt: &impl wow_packet::ServerPacket) {
        let data = pkt.to_bytes();
        if self.send_tx.send(data).is_err() {
            warn!("Send channel closed for account {}", self.account_id);
        }
    }

    /// Send a server packet on the **realm** connection.
    ///
    /// Some packets (e.g. `QueryPlayerNamesResponse`) must travel on the
    /// realm socket, not the instance socket.  Falls back to `send_tx` if
    /// no realm channel exists (pre-ConnectTo or single-connection mode).
    pub fn send_packet_realm(&self, pkt: &impl wow_packet::ServerPacket) {
        let data = pkt.to_bytes();
        let tx = self.realm_send_tx.as_ref().unwrap_or(&self.send_tx);
        if tx.send(data).is_err() {
            warn!("Realm send channel closed for account {}", self.account_id);
        }
    }

    /// Send pre-serialized packet bytes to the client.
    ///
    /// Used for packets with dynamic opcodes (e.g. `SetSpellModifier`
    /// which uses the same struct for Flat and Pct variants).
    pub fn send_raw_packet(&self, data: &[u8]) {
        if self.send_tx.send(data.to_vec()).is_err() {
            warn!("Send channel closed for account {}", self.account_id);
        }
    }

    /// Send session initialization packets (first encrypted packets after
    /// EnterEncryptedModeAck). Matches C# `InitializeSessionCallback`.
    ///
    /// These packets are sent immediately when the session starts, before any
    /// client packets are processed. They tell the client that auth succeeded
    /// and provide the initial glue screen data (character select).
    ///
    /// Exact C# order:
    /// 1. AuthResponse
    /// 2. SetTimeZoneInformation
    /// 3. FeatureSystemStatusGlueScreen (NOT the in-game FeatureSystemStatus!)
    /// 4. ClientCacheVersion
    /// 5. AvailableHotfixes (empty)
    /// 6. AccountDataTimes (global)
    /// 7. TutorialFlags
    /// 8. ConnectionStatus (State=1)
    pub fn send_session_init_packets(&self) {
        use wow_packet::packets::auth::*;
        use wow_packet::packets::misc::*;

        let vra = self.virtual_realm_address();

        // 1. AuthResponse (OK) — tells the client authentication succeeded
        let auth_response = AuthResponse {
            result: 0, // OK
            success_info: Some(AuthSuccessInfo {
                virtual_realm_address: vra,
                virtual_realms: vec![VirtualRealmInfo {
                    realm_address: vra,
                    is_local: true,
                    is_internal_realm: false,
                    realm_name_actual: String::from("RustyCore"),
                    realm_name_normalized: String::from("rustycore"),
                }],
                time_rested: 0,
                active_expansion_level: self.expansion,
                account_expansion_level: self.account_expansion,
                time_seconds_until_pc_kick: 0,
                available_classes: default_available_classes(),
                templates: vec![],
                currency_id: 0,
                time: unix_now(),
                game_time_info: GameTimeInfo {
                    billing_plan: 0,
                    time_remain: 0,
                    unknown735: 0,
                    in_game_room: false,
                },
                is_expansion_trial: false,
                force_character_template: false,
                num_players_horde: None,
                num_players_alliance: None,
                expansion_trial_expiration: None,
            }),
            wait_info: None,
        };
        self.send_packet(&auth_response);

        // 2. SetTimeZoneInformation
        self.send_packet(&SetTimeZoneInformation::utc());

        // 3. FeatureSystemStatusGlueScreen (character select version, NOT in-game)
        self.send_packet(&FeatureSystemStatusGlueScreen::default_wotlk());

        // 4. ClientCacheVersion (from world DB version.cache_id = 24081)
        self.send_packet(&ClientCacheVersion { cache_version: 24081 });

        // 5. AvailableHotfixes (empty — no hotfixes)
        self.send_packet(&AvailableHotfixes {
            virtual_realm_address: vra,
        });

        // 6. AccountDataTimes (global)
        self.send_packet(&AccountDataTimes::global());

        // 7. TutorialFlags
        self.send_packet(&TutorialFlags::all_shown());

        // 8. ConnectionStatus (State=1, SuppressNotification=true)
        // C# BattlenetPackets.cs: ConnectionStatus has no ConnectionType override,
        // so it's sent on the realm socket. State uses 2 bits, SuppressNotification
        // defaults to true.
        self.send_packet(&ConnectionStatus {
            state: 1,
            suppress_notification: true,
        });

        info!(
            "Session init packets sent for account {} (8 packets: AuthResponse → ConnectionStatus)",
            self.account_id
        );
    }

    /// Set the logged-in player GUID.
    pub fn set_player_guid(&mut self, guid: Option<ObjectGuid>) {
        self.player_guid = guid;
    }

    /// Get the logged-in player GUID.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.player_guid
    }

    /// Complete the logout: send LogoutComplete and mark session for disconnect.
    fn complete_logout(&mut self) {
        use wow_packet::packets::misc::LogoutComplete;

        info!("Logout complete for account {}", self.account_id);
        self.send_packet(&LogoutComplete);
        self.player_guid = None;
        self.state = SessionState::Authed;
    }

    /// Kick the session (mark as disconnecting).
    pub fn kick(&mut self, reason: &str) {
        warn!(
            "Kicking account {} ({}): {reason}",
            self.account_id, self.account_name
        );
        self.state = SessionState::Disconnecting;
    }

    /// Get the current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Set the session state (e.g., after character login).
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    /// Time since the last packet was received.
    pub fn idle_time(&self) -> std::time::Duration {
        self.last_packet_time.elapsed()
    }

    /// Whether the session is disconnecting.
    pub fn is_disconnecting(&self) -> bool {
        self.state == SessionState::Disconnecting
    }

    /// Restore the realm socket as the primary send/receive channel.
    ///
    /// After a ConnectTo flow, `send_tx` and `packet_rx` point to the
    /// instance socket while the realm channels are stored in
    /// `realm_send_tx` / `realm_packet_rx`.  On logout the client
    /// returns to character select on the REALM connection, so we must
    /// swap back.  The old instance channels are simply dropped — the
    /// instance reader/writer tasks will notice and exit.
    pub(crate) fn restore_realm_channels(&mut self) {
        if let Some(realm_tx) = self.realm_send_tx.take() {
            info!(
                "Restoring realm send channel as primary for account {}",
                self.account_id
            );
            self.send_tx = realm_tx;
        }
        if let Some(realm_rx) = self.realm_packet_rx.take() {
            info!(
                "Restoring realm packet channel as primary for account {}",
                self.account_id
            );
            self.packet_rx = realm_rx;
        }
        // Clear any pending ConnectTo state
        self.instance_link_rx = None;
        self.connect_to_key = None;
        self.connect_to_serial = None;
        self.player_loading = None;
    }
}

// ── Creature AI / Combat tick methods ────────────────────────────

impl WorldSession {
    /// Called every ~200ms from the update loop.
    /// Advances creature movement state and sends MonsterMove packets.
    pub(crate) fn tick_creatures_sync(&mut self) {
        use wow_packet::packets::movement::MonsterMove;
        use wow_packet::ServerPacket;

        // Collect packets to send (avoids borrow conflict with send_packet)
        let mut to_send: Vec<Vec<u8>> = Vec::new();

        let guids: Vec<wow_core::ObjectGuid> = self.creatures.keys().cloned().collect();

        // ── Corpse despawn ─────────────────────────────────────────────────
        // C# ref: `Creature.RemoveCorpse` / `AllLootRemovedFromCorpse`.
        // After `corpse_despawn_at` passes, remove the dead creature from the
        // world and notify the client (destroy block in SMSG_UPDATE_OBJECT).
        let now = std::time::Instant::now();
        let despawn_guids: Vec<wow_core::ObjectGuid> = guids
            .iter()
            .filter(|g| {
                self.creatures
                    .get(g)
                    .map(|c| {
                        !c.is_alive
                            && c.corpse_despawn_at
                                .map(|t| now >= t)
                                .unwrap_or(false)
                    })
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        if !despawn_guids.is_empty() {
            use wow_packet::packets::update::{CreatureCreateData, UpdateObject};
            use wow_packet::ServerPacket;

            let map_id = self.current_map_id;
            for g in &despawn_guids {
                // Before removing, save data needed for respawn.
                if let Some(c) = self.creatures.remove(g) {
                    // C# ref: AllLootRemovedFromCorpse → m_respawnTime = corpseRemoveTime + respawnDelay
                    let respawn_at = now + std::time::Duration::from_secs(c.respawn_time_secs);
                    // Build CreatureCreateData from saved AI fields (with sensible defaults
                    // for fields not stored in CreatureAI: scale, unit_class, timers, speeds).
                    let create_data = CreatureCreateData {
                        guid: c.guid,
                        entry: c.entry,
                        display_id: c.display_id,
                        native_display_id: c.display_id,
                        health: c.max_hp as i64,
                        max_health: c.max_hp as i64,
                        level: c.level,
                        faction_template: c.faction as i32,
                        npc_flags: c.npc_flags as u64,
                        unit_flags: c.unit_flags,
                        unit_flags2: 0,
                        unit_flags3: 0,
                        scale: 1.0,
                        unit_class: 1,
                        base_attack_time: 2000,
                        ranged_attack_time: 0,
                        zone_id: 0,
                        speed_walk_rate: 1.0,
                        speed_run_rate: 1.14286,
                    };
                    self.respawn_queue.push(PendingRespawn {
                        respawn_at,
                        home_pos: c.home_pos,
                        create_data,
                        max_hp: c.max_hp,
                        level: c.level,
                        min_dmg: c.min_dmg,
                        max_dmg: c.max_dmg,
                        aggro_radius: c.aggro_radius,
                        npc_flags: c.npc_flags,
                        unit_flags: c.unit_flags,
                        map_id,
                    });
                    tracing::info!(
                        "Corpse despawned: {:?} (entry {}) — respawn in {}s",
                        g, c.entry, c.respawn_time_secs
                    );
                }
                self.visible_creatures.remove(g);
            }
            let pkt = UpdateObject::destroy_objects(despawn_guids, map_id);
            if let Err(e) = self.send_tx.send(pkt.to_bytes()) {
                tracing::warn!("Failed to send despawn UpdateObject: {e}");
            }
        }
        // ── Respawn queue ──────────────────────────────────────────────────
        // C# ref: Creature::Update → RemoveCorpse → respawn via Map::AddToMap.
        let ready: Vec<PendingRespawn> = {
            let mut remaining = Vec::new();
            let mut spawn_now = Vec::new();
            for r in self.respawn_queue.drain(..) {
                if now >= r.respawn_at {
                    spawn_now.push(r);
                } else {
                    remaining.push(r);
                }
            }
            self.respawn_queue = remaining;
            spawn_now
        };

        for r in ready {
            use wow_packet::packets::update::UpdateObject;
            use wow_packet::ServerPacket;

            let guid = r.create_data.guid;
            let entry = r.create_data.entry;
            let display_id = r.create_data.display_id;
            let faction = r.create_data.faction_template as u32;

            tracing::info!(
                "Creature respawned: {:?} (entry {}) at {:?}",
                guid, entry, r.home_pos
            );

            // Send CREATE block to client.
            let block = UpdateObject::create_creature_block(r.create_data, &r.home_pos);
            let pkt = UpdateObject::create_creatures(vec![block], r.map_id);
            if let Err(e) = self.send_tx.send(pkt.to_bytes()) {
                tracing::warn!("Failed to send respawn packet: {e}");
            }

            // Recreate the creature AI at home position, fully alive.
            let ai = wow_ai::CreatureAI::new(
                guid,
                entry,
                r.home_pos,
                r.max_hp,
                r.level,
                r.min_dmg,
                r.max_dmg,
                r.aggro_radius,
                display_id,
                faction,
                r.npc_flags,
                r.unit_flags,
            );

            self.creatures.insert(guid, ai);
            self.visible_creatures.insert(guid);
        }
        // ──────────────────────────────────────────────────────────────────

        for guid in guids {
            let creature = match self.creatures.get_mut(&guid) {
                Some(c) => c,
                None => continue,
            };

            if !creature.is_alive {
                if creature.should_respawn() {
                    creature.respawn();
                }
                continue;
            }

            match creature.state {
                wow_ai::CreatureState::Idle => {
                    if creature.movement_finished() {
                        if creature.move_target.is_some() {
                            creature.finish_move();
                        }
                        if creature.should_wander() {
                            let dst = creature.pick_wander_destination();
                            let from = creature.current_pos;
                            let sid = creature.spline_id;
                            let dist = from.distance(&dst);
                            let dur = ((dist / 2.5) * 1000.0) as u32;
                            creature.begin_move(dst);
                            creature.state = wow_ai::CreatureState::WalkingRandom;
                            creature.reset_wander_timer();
                            // TODO: verify MonsterMove wire format before enabling
                        // let pkt = MonsterMove { ... };
                        // to_send.push(pkt.to_bytes());
                        let _ = (guid, from, sid, dur, dst);
                        }
                    }
                }
                wow_ai::CreatureState::WalkingRandom => {
                    if creature.movement_finished() {
                        creature.finish_move();
                        creature.state = wow_ai::CreatureState::Idle;
                        creature.reset_wander_timer();
                    }
                }
                wow_ai::CreatureState::Returning => {
                    if creature.movement_finished() {
                        creature.finish_move();
                        creature.state = wow_ai::CreatureState::Idle;
                    }
                }
                wow_ai::CreatureState::InCombat
                | wow_ai::CreatureState::Dead
                | wow_ai::CreatureState::WalkingWaypoint => {}
            }
        }

        // Send all movement packets
        for data in to_send {
            if self.send_tx.send(data).is_err() {
                break;
            }
        }
    }

    /// Called every ~100ms. Checks if an in-progress spell cast has completed.
    ///
    /// If `active_spell_cast` is set and its cast time has elapsed, this method
    /// executes the spell (applies effects, cooldowns, etc.) and clears the cast state.
    pub(crate) async fn tick_active_spell_cast(&mut self) {
        let Some(ref cast_state) = self.active_spell_cast.clone() else {
            return;
        };

        let elapsed_ms = cast_state.cast_start_time.elapsed().as_millis() as u32;

        if elapsed_ms >= cast_state.cast_time_ms {
            let spell_id = cast_state.spell_id;
            let target = cast_state.target_guid;
            let cast_id = cast_state.cast_id;
            let spell_visual = cast_state.spell_visual.clone();

            self.active_spell_cast = None;
            self.last_spell_cast_time = Some(Instant::now());

            // ← AQUÍ: Ejecutar spell
            if let Err(e) = self.execute_spell_with_visual(spell_id, target, cast_id, spell_visual).await {
                warn!(account = self.account_id, "Spell execution failed: {}", e);
                // Send CastFailed so client cancels cast animation
                use wow_packet::packets::spell::CastFailed;
                use wow_packet::ServerPacket;
                self.send_packet(&CastFailed {
                    cast_id,
                    spell_id,
                    reason: 2, // SpellCastResult::NotKnown
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
            }
        }
    }

    /// Called every ~100ms. Handles auto-attack swing timer (player → creature).
    pub(crate) fn tick_combat_sync(&mut self) {
        use wow_packet::packets::combat::{AttackerStateUpdate, VICTIM_STATE_HIT, SAttackStop};
        use wow_packet::ServerPacket;

        let (player_guid, combat_target) = match (self.player_guid, self.combat_target) {
            (Some(pg), Some(ct)) => (pg, ct),
            _ => return,
        };

        // Check if target still exists
        let creature_exists = self.creatures.contains_key(&combat_target);
        if !creature_exists {
            self.combat_target = None;
            self.in_combat = false;
            return;
        }

        // Gather combat data, mutate creature state
        let (dmg, target_level, now_dead, was_alive) = {
            let creature = self.creatures.get_mut(&combat_target).unwrap();
            if !creature.is_alive {
                return;
            }
            if creature.state != wow_ai::CreatureState::InCombat {
                creature.enter_combat(player_guid);
            }
            if !creature.can_swing() {
                return;
            }
            let dmg = creature.roll_damage().max(1);
            let level = creature.level;
            let died = creature.take_damage(dmg);
            creature.record_swing();
            (dmg, level, died, true)
        };

        if !was_alive { return; }

        let over_damage = if now_dead { 0i32 } else { -1i32 };

        // Send damage event
        let state_update = AttackerStateUpdate {
            attacker: player_guid,
            victim: combat_target,
            damage: dmg as i32,
            over_damage,
            victim_state: VICTIM_STATE_HIT,
            school_mask: 1,
            target_level,
            expansion: 2,
        };
        if self.send_tx.send(state_update.to_bytes()).is_err() { return; }

        // TODO: creature health VALUES update — format needs verification vs client
        // (temporarily disabled to prevent client crash from malformed packet)

        if now_dead {
            let stop = SAttackStop {
                attacker: player_guid,
                victim: combat_target,
                now_dead: true,
            };
            let _ = self.send_tx.send(stop.to_bytes());
            self.combat_target = None;
            self.in_combat = false;
        }
    }

    /// Broadcast the newly logged-in player's CREATE block to all other players on the same map.
    ///
    /// Called after login is complete. Iterates through all players in the registry
    /// who are on the same map, creates an UpdateObject with the new player's CREATE block,
    /// and sends it to each via their send_tx channel.
    ///
    /// C# ref: `Player::SendInitialPacketsAfterAddToMap` → WorldSession broadcast logic.
    pub(crate) fn broadcast_create_player_to_others(&self) {
        use wow_packet::packets::update::{UpdateObject, PlayerCombatStats};
        use wow_packet::ServerPacket;

        let Some(guid) = self.player_guid else { return };
        let Some(registry) = &self.player_registry else { return };
        let Some(pos) = self.player_position else { return };

        // Build visible_items from this player's equipped inventory.
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        for (slot, item) in &self.inventory_items {
            if (*slot as usize) < 19 {
                visible_items[*slot as usize] = (item.entry_id as i32, 0u16, 0u16);
            }
        }
        let empty_inv_slots = [ObjectGuid::EMPTY; 141];
        let empty_skills = Vec::new();

        // Create the UpdateObject for this player (with is_self=false for other players)
        use crate::handlers::character::default_display_id;
        let update = UpdateObject::create_player(
            guid,
            self.player_race,
            self.player_class,
            self.player_gender,
            self.player_level,
            default_display_id(self.player_race, self.player_gender),
            &pos,
            self.current_map_id,
            0, // zone_id (would need to track)
            false, // is_self: other players see this as a regular player, not ActivePlayer
            visible_items,
            empty_inv_slots,
            PlayerCombatStats::default(), // other players don't need detailed combat stats
            empty_skills,
            self.player_gold,
        );

        // Serialize once, reuse for all broadcasts
        let bytes = update.to_bytes();

        // Count players to broadcast to
        let mut broadcast_count = 0;

        // Iterate through all players in the registry on the same map
        for entry in registry.iter() {
            let (other_guid, broadcast_info) = entry.pair();
            // Don't send to ourselves
            if *other_guid == guid {
                continue;
            }
            // Only send to players on the same map
            if broadcast_info.map_id != self.current_map_id {
                continue;
            }

            broadcast_count += 1;

            if let Err(_) = broadcast_info.send_tx.send(bytes.clone()) {
                debug!("Failed to broadcast CreatePlayer to {:?}", other_guid);
            } else {
                trace!("Broadcast CreatePlayer {:?} to {:?}", guid, other_guid);
            }
        }

        if broadcast_count > 0 {
            info!(
                "Broadcasted CreatePlayer for {:?} to {} players on map {}",
                guid, broadcast_count, self.current_map_id
            );
        }
    }

    /// Broadcast DestroyObject to all players on the same map when this player disconnects.
    pub(crate) fn broadcast_destroy_player_to_others(&self) {
        use wow_packet::packets::update::UpdateObject;
        use wow_packet::ServerPacket;

        let Some(guid) = self.player_guid else { return };
        let Some(registry) = &self.player_registry else { return };

        let destroy = UpdateObject::destroy_objects(vec![guid], self.current_map_id);
        let bytes = destroy.to_bytes();

        let mut count = 0usize;
        for entry in registry.iter() {
            let (other_guid, info) = entry.pair();
            if *other_guid == guid { continue; }
            if info.map_id != self.current_map_id { continue; }
            if info.send_tx.send(bytes.clone()).is_ok() {
                count += 1;
            }
        }
        if count > 0 {
            info!("Broadcast DestroyPlayer {:?} to {} players", guid, count);
        }
    }

    /// Receive CREATE blocks from all other players currently on the same map.
    ///
    /// Called after login is complete. Queries the player registry for all players
    /// on the current map (excluding self), builds their CREATE blocks, and sends
    /// them as UpdateObjects to this session.
    ///
    /// C# ref: `Player::SendInitialPacketsAfterAddToMap` → populate visibility with other players.
    pub(crate) fn receive_other_players_on_map(&self) {
        use wow_packet::packets::update::{UpdateObject, PlayerCombatStats};

        let Some(guid) = self.player_guid else { return };
        let Some(registry) = &self.player_registry else { return };

        let empty_inv_slots = [ObjectGuid::EMPTY; 141];
        let empty_skills = Vec::new();
        let default_combat_stats = PlayerCombatStats::default();

        let mut player_count = 0;

        // Iterate through all players in the registry
        for entry in registry.iter() {
            let (other_guid, broadcast_info) = entry.pair();

            // Skip self and players on different maps
            if *other_guid == guid || broadcast_info.map_id != self.current_map_id {
                continue;
            }

            player_count += 1;

            // Create UpdateObject for this other player using cached data from broadcast_info
            let update = UpdateObject::create_player(
                *other_guid,
                broadcast_info.race,
                broadcast_info.class,
                broadcast_info.sex,
                broadcast_info.level,
                broadcast_info.display_id,
                &broadcast_info.position,
                broadcast_info.map_id,
                0, // zone_id (unknown — would need separate tracking)
                false, // is_self: this is another player, not us
                broadcast_info.visible_items,
                empty_inv_slots,
                default_combat_stats,
                empty_skills.clone(),
                0, // coinage (don't send other players' gold)
            );

            self.send_packet(&update);
            trace!(
                "Sent CREATE block for other player {:?} to account {}",
                other_guid, self.account_id
            );
        }

        if player_count > 0 {
            info!(
                "Received CREATE blocks from {} other players on map {} for {:?}",
                player_count, self.current_map_id, guid
            );
        }
    }

    /// Check if any hostile creature should aggro the player based on proximity.
    /// Called from movement handlers (CMSG_MOVE_*).
    pub(crate) async fn check_creature_aggro(&mut self) {
        use wow_packet::packets::combat::AttackStart;
        use wow_packet::ServerPacket;

        if self.in_combat { return; }

        let player_pos = match self.player_position { Some(p) => p, None => return };
        let player_guid = match self.player_guid { Some(g) => g, None => return };

        let guids: Vec<wow_core::ObjectGuid> = self.creatures.keys().cloned().collect();
        let mut aggro_guid: Option<wow_core::ObjectGuid> = None;

        for guid in guids {
            let creature = match self.creatures.get_mut(&guid) {
                Some(c) => c,
                None => continue,
            };
            if !creature.is_alive || creature.aggro_radius <= 0.0 { continue; }
            if creature.try_aggro(player_guid, &player_pos) {
                aggro_guid = Some(guid);
                break;
            }
        }

        if let Some(guid) = aggro_guid {
            let start = AttackStart { attacker: guid, victim: player_guid };
            let _ = self.send_tx.send(start.to_bytes());
            self.combat_target = Some(guid);
            self.in_combat = true;
        }
    }

    /// Execute a spell — apply effects, set cooldown, send SMSG_SPELL_GO.
    ///
    /// Called for instant-cast spells. Delegates to execute_spell_with_visual
    /// with default cast_id and visual.
    pub async fn execute_spell(&mut self, spell_id: i32, target_guid: ObjectGuid) -> Result<(), &'static str> {
        use wow_packet::packets::spell::SpellCastVisual;
        
        self.execute_spell_with_visual(
            spell_id,
            target_guid,
            ObjectGuid::EMPTY,
            SpellCastVisual {
                spell_visual_id: 0,
                script_visual_id: 0,
            },
        ).await
    }

    /// Execute a spell with full visual/cast info — apply effects, set cooldown, send SMSG_SPELL_GO.
    ///
    /// Called after cast time completes or for instant-cast spells.
    /// Supports: heal (type 6), damage (type 2), aura application (type 35).
    pub async fn execute_spell_with_visual(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid.ok_or("No player GUID")?;

        // Obtener SpellInfo
        let spell_info = self.spell_store()
            .and_then(|store| store.get(spell_id))
            .ok_or("Spell not found")?;

        info!(
            account = self.account_id,
            spell_id = spell_id,
            target = ?target_guid,
            effect_type = spell_info.effect_type,
            "Executing spell effect"
        );

        // Send SMSG_SPELL_GO
        use wow_packet::packets::spell::{SpellGoPkt, SpellTargetData};
        use wow_packet::ServerPacket;

        let go_pkt = SpellGoPkt {
            caster: player_guid,
            cast_id,
            spell_id,
            visual: spell_visual,
            target: SpellTargetData {
                flags: 0x2, // SpellCastTargetFlags::Unit
                unit: target_guid,
                item: ObjectGuid::EMPTY,
            },
            hit_targets: vec![target_guid],
        };
        self.send_packet(&go_pkt);

        // Aplicar efecto según type
        match spell_info.effect_type {
            6 => {
                // SPELL_EFFECT_HEAL
                let heal_amount = spell_info.effect_base_points as u32;
                self.apply_heal(target_guid, heal_amount).await?;
            }
            2 => {
                // SPELL_EFFECT_SCHOOL_DAMAGE
                let damage_amount = spell_info.effect_base_points as u32;
                self.apply_damage(target_guid, damage_amount).await?;
            }
            35 => {
                // SPELL_EFFECT_APPLY_AURA
                self.apply_aura(spell_id, player_guid, 30000, 0x00000001)?;
            }
            _ => {
                debug!("Spell effect type {} not yet implemented", spell_info.effect_type);
            }
        }

        // Set global cooldown
        self.last_spell_cast_time = Some(Instant::now());

        // Set per-spell cooldown
        self.last_spell_cast_time_per_spell.insert(spell_id, Instant::now());

        // Notify client so action bar shows the cooldown animation
        use wow_packet::packets::spell::CooldownEvent;
        self.send_packet(&CooldownEvent { spell_id, is_pet: false });

        Ok(())
    }

    /// Helper: apply heal to target (self or creature).
    async fn apply_heal(&mut self, target_guid: ObjectGuid, heal_amount: u32) -> Result<(), &'static str> {
        let player_guid = self.player_guid.ok_or("No player GUID")?;

        // Si target es el mismo jugador
        if target_guid == player_guid {
            info!(
                account = self.account_id,
                heal = heal_amount,
                "Healed self"
            );
            // TODO: Actualizar HP del jugador en la DB
            // self.player_health = min(self.player_health + heal_amount, self.player_max_health);
            // Enviar UpdateObject con VALUES update
            return Ok(());
        }

        // Si target es otra criatura/jugador
        if let Some(_creature) = self.creatures.get(&target_guid) {
            info!(
                account = self.account_id,
                creature = ?target_guid,
                heal = heal_amount,
                "Healed creature"
            );
            // TODO: Actualizar HP de la criatura
            return Ok(());
        }

        Err("Target not found")
    }

    /// Helper: apply damage to target creature.
    async fn apply_damage(&mut self, target_guid: ObjectGuid, damage_amount: u32) -> Result<(), &'static str> {
        let _player_guid = self.player_guid.ok_or("No player GUID")?;

        // Si target es otra criatura — end mutable borrow before calling async methods
        let kill_info: Option<(u32, wow_core::ObjectGuid)> = {
            if let Some(creature) = self.creatures.get_mut(&target_guid) {
                info!(
                    account = self.account_id,
                    creature = ?target_guid,
                    damage = damage_amount,
                    "Dealt damage to creature"
                );

                creature.hp = creature.hp.saturating_sub(damage_amount);

                if creature.hp == 0 {
                    info!("Creature {} (entry={}) killed", target_guid, creature.entry);
                    creature.state = wow_ai::CreatureState::Dead;
                    creature.is_alive = false;
                    Some((creature.entry, target_guid))
                } else {
                    None
                }
            } else {
                return Err("Target creature not found");
            }
        };

        // Process creature death outside the mutable borrow
        if let Some((entry, guid)) = kill_info {
            self.on_creature_killed(entry, guid).await;
        }

        Ok(())
    }
}

/// Current Unix timestamp (seconds since epoch).
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Available race/class combinations from `class_expansion_requirement` table.
///
/// Data matches exactly what C# ObjectManager loads from the world DB.
/// ActiveExpansionLevel/AccountExpansionLevel: 0 for all except Death Knight (class 6)
/// which requires WotLK (active=2). MinActiveExpansionLevel is the minimum active
/// expansion across all races for that class.
fn default_available_classes() -> Vec<wow_packet::packets::auth::RaceClassAvailability> {
    use wow_packet::packets::auth::{ClassAvailability, RaceClassAvailability};

    // (race_id, &[(class_id, active_expansion_level, account_expansion_level)])
    let data: &[(u8, &[(u8, u8, u8)])] = &[
        (1,  &[(1,0,0),(2,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(8,0,0),(9,0,0)]),              // Human
        (2,  &[(1,0,0),(3,0,0),(4,0,0),(6,2,0),(7,0,0),(8,0,0),(9,0,0)]),                       // Orc
        (3,  &[(1,0,0),(2,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(7,0,0),(8,0,0),(9,0,0)]),       // Dwarf
        (4,  &[(1,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(8,0,0),(11,0,0)]),                       // Night Elf
        (5,  &[(1,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(8,0,0),(9,0,0)]),                       // Undead
        (6,  &[(1,0,0),(2,0,0),(3,0,0),(5,0,0),(6,2,0),(7,0,0),(11,0,0)]),                      // Tauren
        (7,  &[(1,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(8,0,0),(9,0,0)]),                       // Gnome
        (8,  &[(1,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(7,0,0),(8,0,0),(9,0,0),(11,0,0)]),      // Troll
        (10, &[(1,0,0),(2,0,0),(3,0,0),(4,0,0),(5,0,0),(6,2,0),(8,0,0),(9,0,0)]),                // Blood Elf
        (11, &[(1,0,0),(2,0,0),(3,0,0),(5,0,0),(6,2,0),(7,0,0),(8,0,0)]),                       // Draenei
    ];

    // MinActiveExpansionLevel per class = min across all races for that class
    // All classes have active=0 across all races except class 6 (DK) which is always 2
    let min_active = |class_id: u8| -> u8 {
        if class_id == 6 { 2 } else { 0 }
    };

    data.iter()
        .map(|&(race_id, classes)| RaceClassAvailability {
            race_id,
            classes: classes
                .iter()
                .map(|&(class_id, active_exp, account_exp)| ClassAvailability {
                    class_id,
                    active_expansion_level: active_exp,
                    account_expansion_level: account_exp,
                    min_active_expansion_level: min_active(class_id),
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> (WorldSession, flume::Sender<WorldPacket>, flume::Receiver<Vec<u8>>) {
        let (pkt_tx, pkt_rx) = flume::bounded(100);
        let (send_tx, send_rx) = flume::bounded(100);

        let session = WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9, // account_expansion (raw from DB)
            54261,
            vec![0u8; 40],
            "esES".into(),
            pkt_rx,
            send_tx,
        );

        (session, pkt_tx, send_rx)
    }

    #[test]
    fn session_starts_authed() {
        let (session, _, _) = make_session();
        assert_eq!(session.state(), SessionState::Authed);
    }

    #[test]
    fn update_empty_queue() {
        let (mut session, _, _) = make_session();
        let processed = session.update(100);
        assert_eq!(processed, 0);
    }

    #[test]
    fn update_processes_packets() {
        let (mut session, pkt_tx, _) = make_session();

        // Send some packets (they'll be logged as "no handler" but won't crash)
        for _ in 0..5 {
            let pkt = WorldPacket::from_bytes(&[0x00, 0x00]); // opcode 0
            pkt_tx.send(pkt).unwrap();
        }

        let processed = session.update(100);
        assert_eq!(processed, 5);
        assert_eq!(session.pending_packets.len(), 5);
    }

    #[test]
    fn kick_sets_disconnecting() {
        let (mut session, _, _) = make_session();
        session.kick("test");
        assert!(session.is_disconnecting());
    }

    #[test]
    fn disconnected_channel_sets_disconnecting() {
        let (mut session, pkt_tx, _) = make_session();
        drop(pkt_tx); // Close the channel

        session.update(100);
        assert!(session.is_disconnecting());
    }

    #[test]
    fn send_packet_works() {
        let (session, _, send_rx) = make_session();

        let pong = wow_packet::packets::auth::Pong { serial: 42 };
        session.send_packet(&pong);

        let data = send_rx.try_recv().unwrap();
        assert_eq!(data.len(), 6); // opcode(2) + serial(4)
    }

    #[test]
    fn state_transitions() {
        let (mut session, _, _) = make_session();
        assert_eq!(session.state(), SessionState::Authed);

        session.set_state(SessionState::LoggedIn);
        assert_eq!(session.state(), SessionState::LoggedIn);

        session.set_state(SessionState::Transfer);
        assert_eq!(session.state(), SessionState::Transfer);
    }

    #[test]
    fn legit_characters_management() {
        let (mut session, _, _) = make_session();

        let guid1 = ObjectGuid::create_player(1, 1);
        let guid2 = ObjectGuid::create_player(1, 2);
        let guid3 = ObjectGuid::create_player(1, 3);

        session.set_legit_characters(vec![guid1, guid2, guid3]);
        assert!(session.is_legit_character(&guid1));
        assert!(session.is_legit_character(&guid2));
        assert!(!session.is_legit_character(&ObjectGuid::create_player(1, 99)));

        session.remove_legit_character(&guid2);
        assert!(!session.is_legit_character(&guid2));
        assert!(session.is_legit_character(&guid1));
    }

    #[test]
    fn char_db_and_realm_id() {
        let (mut session, _, _) = make_session();

        assert!(session.char_db().is_none());
        assert_eq!(session.realm_id(), 1);

        session.set_realm_id(5);
        assert_eq!(session.realm_id(), 5);
    }
}
