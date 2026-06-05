// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! TCP listener and accept loop for the world server.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{debug, error, info};

use wow_constants::ClientOpcodes;
use wow_crypto::HmacSha256;
use wow_packet::ClientPacket;
use wow_packet::packets::auth::{AuthContinuedSession, ConnectToKey, EnterEncryptedMode};

use crate::group_registry::{GroupRegistry, PendingInvites};
use crate::player_registry::{GameEventQuestCompleteCommandLikeCpp, PlayerRegistry};
use crate::session_mgr::{InstanceLink, SessionManager};
use crate::world_socket::{
    AccountInfo, AccountLookup, WorldSocket, WorldSocketError, sign_enable_encryption,
};

/// C++ `World::rate_values` subset used by loot generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootDropRatesLikeCpp {
    pub item_poor: f32,
    pub item_normal: f32,
    pub item_uncommon: f32,
    pub item_rare: f32,
    pub item_epic: f32,
    pub item_legendary: f32,
    pub item_artifact: f32,
    pub item_referenced: f32,
    pub item_referenced_amount: f32,
    pub money: f32,
}

impl Default for LootDropRatesLikeCpp {
    fn default() -> Self {
        Self {
            item_poor: 1.0,
            item_normal: 1.0,
            item_uncommon: 1.0,
            item_rare: 1.0,
            item_epic: 1.0,
            item_legendary: 1.0,
            item_artifact: 1.0,
            item_referenced: 1.0,
            item_referenced_amount: 1.0,
            money: 1.0,
        }
    }
}

/// C++ `World::rate_values` subset used by reputation gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReputationRatesLikeCpp {
    pub gain: f32,
    pub low_level_kill: f32,
    pub low_level_quest: f32,
    pub recruit_a_friend_bonus: f32,
    pub recruit_a_friend_distance: f32,
}

impl Default for ReputationRatesLikeCpp {
    fn default() -> Self {
        Self {
            gain: 1.0,
            low_level_kill: 1.0,
            low_level_quest: 1.0,
            recruit_a_friend_bonus: 0.1,
            recruit_a_friend_distance: 100.0,
        }
    }
}

/// Resources needed for creating a WorldSession after authentication.
///
/// Held by the accept loop and cloned for each connection.
pub struct SessionResources {
    pub char_db: Option<Arc<wow_database::CharacterDatabase>>,
    pub login_db: Option<Arc<wow_database::LoginDatabase>>,
    pub world_db: Option<Arc<wow_database::WorldDatabase>>,
    pub guid_generator: Option<Arc<wow_core::ObjectGuidGenerator>>,
    pub instance_lock_mgr: Option<Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>>,
    pub currency_types_store: Option<Arc<wow_data::CurrencyTypesStore>>,
    pub import_price_stores: Option<Arc<wow_data::ImportPriceStores>>,
    pub item_class_store: Option<Arc<wow_data::ItemClassStore>>,
    pub item_currency_cost_store: Option<Arc<wow_data::ItemCurrencyCostStore>>,
    pub item_extended_cost_store: Option<Arc<wow_data::ItemExtendedCostStore>>,
    pub item_appearance_store: Option<Arc<wow_data::ItemAppearanceStore>>,
    pub item_store: Option<Arc<wow_data::ItemStore>>,
    pub item_modified_appearance_store: Option<Arc<wow_data::ItemModifiedAppearanceStore>>,
    pub item_search_name_store: Option<Arc<wow_data::ItemSearchNameStore>>,
    pub heirloom_store: Option<Arc<wow_data::HeirloomStore>>,
    pub transmog_set_item_store: Option<Arc<wow_data::TransmogSetItemStore>>,
    pub item_price_base_store: Option<Arc<wow_data::ItemPriceBaseStore>>,
    pub item_limit_category_store: Option<Arc<wow_data::ItemLimitCategoryStore>>,
    pub item_limit_category_condition_store: Option<Arc<wow_data::ItemLimitCategoryConditionStore>>,
    pub player_stats: Option<Arc<wow_data::PlayerStatsStore>>,
    pub item_stats_store: Option<Arc<wow_data::ItemStatsStore>>,
    pub durability_costs_store: Option<Arc<wow_data::DurabilityCostsStore>>,
    pub durability_quality_store: Option<Arc<wow_data::DurabilityQualityStore>>,
    pub item_random_suffix_store: Option<Arc<wow_data::ItemRandomSuffixStore>>,
    pub item_random_properties_store: Option<Arc<wow_data::ItemRandomPropertiesStore>>,
    pub rand_prop_points_store: Option<Arc<wow_data::RandPropPointsStore>>,
    pub item_random_enchantment_template_store:
        Option<Arc<wow_data::ItemRandomEnchantmentTemplateStore>>,
    pub item_spec_override_store: Option<Arc<wow_data::ItemSpecOverrideStore>>,
    pub item_disenchant_loot_store: Option<Arc<wow_data::ItemDisenchantLootStore>>,
    pub loot_stores: Option<Arc<wow_loot::LootStores>>,
    pub condition_store: Option<Arc<wow_data::ConditionEntriesByTypeStore>>,
    pub player_condition_store: Option<Arc<wow_data::PlayerConditionStore>>,
    pub content_tuning_store: Option<Arc<wow_data::progression_rewards::ContentTuningStore>>,
    pub progression_faction_store: Option<Arc<wow_data::progression_rewards::FactionStore>>,
    pub faction_template_store: Option<Arc<wow_data::progression_rewards::FactionTemplateStore>>,
    pub friendship_rep_reaction_store:
        Option<Arc<wow_data::progression_rewards::FriendshipRepReactionStore>>,
    pub paragon_reputation_store:
        Option<Arc<wow_data::progression_rewards::ParagonReputationStore>>,
    pub disable_mgr: Option<Arc<wow_data::DisableMgrLikeCpp>>,
    pub lock_store: Option<Arc<wow_data::LockStore>>,
    pub spell_item_enchantment_store: Option<Arc<wow_data::SpellItemEnchantmentStore>>,
    pub hotfix_blob_cache: Option<Arc<wow_data::HotfixBlobCache>>,
    pub skill_store: Option<Arc<wow_data::SkillStore>>,
    pub skill_line_store: Option<Arc<wow_data::SkillLineStore>>,
    pub spell_store: Option<Arc<wow_data::SpellStore>>,
    pub npc_spell_click_store: Option<Arc<wow_data::NpcSpellClickStoreLikeCpp>>,
    pub spell_misc_store: Option<Arc<wow_data::SpellMiscStore>>,
    pub spell_duration_store: Option<Arc<wow_data::SpellDurationStore>>,
    pub spell_radius_store: Option<Arc<wow_data::SpellRadiusStore>>,
    pub spell_range_store: Option<Arc<wow_data::SpellRangeStore>>,
    pub spell_target_position_store: Option<Arc<wow_data::SpellTargetPositionStoreLikeCpp>>,
    pub movie_store: Option<Arc<wow_data::MovieStore>>,
    pub gameobject_template_lifecycle_store:
        Option<Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>>,
    pub area_table_store: Option<Arc<wow_data::AreaTableStore>>,
    pub fishing_base_skill_store: Option<Arc<wow_data::FishingBaseSkillStoreLikeCpp>>,
    pub area_trigger_store: Option<Arc<wow_data::AreaTriggerStore>>,
    pub chr_specialization_store: Option<Arc<wow_data::ChrSpecializationStore>>,
    pub dungeon_encounter_store: Option<Arc<wow_data::DungeonEncounterStore>>,
    pub map_store: Option<Arc<wow_data::MapStore>>,
    pub map_difficulty_store: Option<Arc<wow_data::MapDifficultyStore>>,
    pub map_difficulty_x_condition_store: Option<Arc<wow_data::MapDifficultyXConditionStore>>,
    pub lfg_dungeons_store: Option<Arc<wow_data::LfgDungeonsStore>>,
    pub creature_template_mount_store: Option<Arc<wow_data::CreatureTemplateMountStoreLikeCpp>>,
    pub creature_display_info_store: Option<Arc<wow_data::CreatureDisplayInfoStore>>,
    pub gameobject_display_info_store: Option<Arc<wow_data::GameObjectDisplayInfoStore>>,
    pub creature_model_data_store: Option<Arc<wow_data::CreatureModelDataStore>>,
    pub mount_store: Option<Arc<wow_data::MountStore>>,
    pub mount_capability_store: Option<Arc<wow_data::MountCapabilityStore>>,
    pub mount_type_x_capability_store: Option<Arc<wow_data::MountTypeXCapabilityStore>>,
    pub mount_x_display_store: Option<Arc<wow_data::MountXDisplayStore>>,
    pub vehicle_store: Option<Arc<wow_data::VehicleStore>>,
    pub vehicle_seat_store: Option<Arc<wow_data::VehicleSeatStore>>,
    pub vehicle_template_store: Option<Arc<wow_data::VehicleTemplateStoreLikeCpp>>,
    pub vehicle_accessory_store: Option<Arc<wow_data::VehicleAccessoryStoreLikeCpp>>,
    pub terrain_swap_store: Option<Arc<wow_data::TerrainSwapStore>>,
    pub phase_store: Option<Arc<wow_data::PhaseStore>>,
    pub phase_group_store: Option<Arc<wow_data::PhaseGroupStore>>,
    pub quest_store: Option<Arc<wow_data::quest::QuestStore>>,
    pub quest_xp_store: Option<Arc<wow_data::quest_xp::QuestXpStore>>,
    pub quest_v2_store: Option<Arc<wow_data::progression_rewards::QuestV2Store>>,
    pub quest_info_store: Option<Arc<wow_data::progression_rewards::QuestInfoStore>>,
    pub quest_package_item_store: Option<Arc<wow_data::progression_rewards::QuestPackageItemStore>>,
    pub quest_faction_reward_store:
        Option<Arc<wow_data::progression_rewards::QuestFactionRewardStore>>,
    pub reputation_reward_rate_store:
        Option<Arc<wow_data::reputation::ReputationRewardRateStoreLikeCpp>>,
    pub creature_onkill_reputation_store:
        Option<Arc<wow_data::reputation::CreatureOnKillReputationStoreLikeCpp>>,
    pub reputation_spillover_template_store:
        Option<Arc<wow_data::reputation::RepSpilloverTemplateStoreLikeCpp>>,
    /// XP required per level: index = level (1-based), value = xp_needed.
    pub player_xp_table: Option<Arc<Vec<u32>>>,
    /// Shared registry of all active player sessions (for broadcast).
    pub player_registry: Option<Arc<PlayerRegistry>>,
    /// Session -> world-server bridge for C++ GameEventMgr::HandleQuestComplete.
    pub game_event_quest_complete_tx: Option<flume::Sender<GameEventQuestCompleteCommandLikeCpp>>,
    /// Shared registry of all active groups.
    pub group_registry: Option<Arc<GroupRegistry>>,
    /// Pending party invites: invited_guid → inviter_guid.
    pub pending_invites: Option<Arc<PendingInvites>>,
    pub loot_drop_rates: LootDropRatesLikeCpp,
    pub reputation_rates: ReputationRatesLikeCpp,
    pub repair_cost_rate: f32,
    pub quest_low_level_hide_diff: u32,
    pub quest_high_level_hide_diff: u32,
    pub enable_ae_loot: bool,
    pub realm_id: u16,
    /// External (public) IP from `realmlist.address`.
    pub realm_external_address: [u8; 4],
    /// Local (LAN) IP from `realmlist.localAddress`.
    pub realm_local_address: [u8; 4],
}

/// Start the world server TCP listener on the given address.
///
/// After each connection completes the auth handshake, channels are created
/// for the session. The `on_session_ready` callback receives:
/// - `AccountInfo` from the auth handshake
/// - `packet_rx` — channel to receive packets from the socket
/// - `send_tx` — channel to send responses back through the socket
/// - `SessionResources` — shared resources for the session
///
/// The callback should create a WorldSession and return a future that runs
/// the session update loop. This future is spawned alongside the socket
/// read and write loops.
pub async fn start_world_listener<F, Fut>(
    bind_addr: SocketAddr,
    account_lookup: Arc<dyn AccountLookup>,
    resources: Arc<SessionResources>,
    on_session_ready: F,
) -> std::io::Result<()>
where
    F: Fn(
            AccountInfo,
            flume::Receiver<wow_packet::WorldPacket>,
            flume::Sender<Vec<u8>>,
            Arc<SessionResources>,
        ) -> Fut
        + Send
        + Sync
        + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind(bind_addr).await?;
    info!("World server listening on {bind_addr}");

    let on_session = Arc::new(on_session_ready);

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {e}");
                continue;
            }
        };

        let lookup = Arc::clone(&account_lookup);
        let res = Arc::clone(&resources);
        let callback = Arc::clone(&on_session);

        tokio::spawn(async move {
            let mut socket = WorldSocket::new(stream, addr);

            // Phase 1: Handshake (connection strings + auth challenge)
            if let Err(e) = socket.start().await {
                error!("Handshake failed for {addr}: {e}");
                return;
            }

            // Phase 2: Authentication
            if let Err(e) = socket.authenticate(lookup.as_ref()).await {
                error!("Authentication failed for {addr}: {e}");
                return;
            }

            // Get account info and attach the client's real IP address
            // and the derived session key from realm auth
            let account_info = match socket.account_info() {
                Some(info) => {
                    let mut ai = info.clone();
                    ai.client_address = Some(addr.ip());
                    ai.derived_session_key =
                        socket.session_key().map(|k| k.to_vec()).unwrap_or_default();
                    ai
                }
                None => {
                    error!("No account info after auth for {addr}");
                    return;
                }
            };

            // Phase 3: Create session channels
            let (pkt_rx, send_tx) = socket.create_session_channels();

            // Phase 4: Split socket into read/write halves
            let pong_tx = send_tx.clone();
            let (reader, writer) = socket.split_for_io(pong_tx);

            // Phase 5: Spawn the write loop (session → TCP)
            tokio::spawn(async move {
                if let Err(e) = writer.run().await {
                    match e {
                        WorldSocketError::Closed => {}
                        _ => error!("Writer error for {addr}: {e}"),
                    }
                }
            });

            // Phase 6: Spawn session update loop
            let session_future = callback(account_info, pkt_rx, send_tx, res);
            tokio::spawn(session_future);

            // Phase 7: Run the encrypted read loop (blocks until disconnect)
            if let Err(e) = reader.run().await {
                match e {
                    WorldSocketError::Closed => {
                        info!("Client {addr} disconnected");
                    }
                    _ => {
                        error!("Socket error for {addr}: {e}");
                    }
                }
            }
        });
    }
}

// ── Instance listener ───────────────────────────────────────────

/// Seeds from C# WorldSocket.cs — must match the realm socket values.
const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2, 0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA, 0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];

/// Start the instance server TCP listener.
///
/// Instance connections come from clients that received `SMSG_CONNECT_TO`.
/// They perform a handshake (connection strings + AuthChallenge), then send
/// `AuthContinuedSession` instead of `AuthSession`.
pub async fn start_instance_listener(
    bind_addr: SocketAddr,
    session_mgr: Arc<SessionManager>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!("Instance server listening on {bind_addr}");

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept instance connection: {e}");
                continue;
            }
        };

        let mgr = Arc::clone(&session_mgr);

        tokio::spawn(async move {
            if let Err(e) = handle_instance_connection(stream, addr, &mgr).await {
                match e {
                    WorldSocketError::Closed => {
                        debug!("Instance client {addr} disconnected");
                    }
                    _ => {
                        error!("Instance connection error for {addr}: {e}");
                    }
                }
            }
        });
    }
}

/// Full instance connection flow: handshake → AuthContinuedSession → encryption → I/O.
async fn handle_instance_connection(
    stream: tokio::net::TcpStream,
    addr: SocketAddr,
    session_mgr: &SessionManager,
) -> Result<(), WorldSocketError> {
    let mut socket = WorldSocket::new(stream, addr);

    // Phase 1: Connection strings + AuthChallenge (same as realm)
    socket.start().await?;

    // Phase 2: Read AuthContinuedSession (unencrypted)
    let pkt = socket.read_unencrypted_packet().await?;
    let opcode = pkt.opcode_raw();

    if opcode != ClientOpcodes::AuthContinuedSession as u16 {
        return Err(WorldSocketError::AuthFailed(format!(
            "expected AuthContinuedSession (0x{:04X}), got 0x{opcode:04X}",
            ClientOpcodes::AuthContinuedSession as u16
        )));
    }

    let mut pkt = pkt;
    pkt.skip_opcode();
    let auth = AuthContinuedSession::read(&mut pkt)?;

    // Phase 3: Extract account_id from ConnectToKey
    let key = ConnectToKey::from_raw(auth.key);
    if key.connection_type != 1 {
        return Err(WorldSocketError::AuthFailed(
            "expected Instance connection type".into(),
        ));
    }

    let account_id = key.account_id;
    info!("Instance AuthContinuedSession from account {account_id} at {addr}");

    // Phase 4: Validate against SessionManager
    let validated = session_mgr
        .validate_and_take(account_id, auth.key)
        .map_err(|e| WorldSocketError::AuthFailed(format!("session manager: {e}")))?;

    let session_key = &validated.session_key;
    let server_challenge = *socket.server_challenge();

    // Phase 5: Validate HMAC-SHA256 digest
    // NOTE: AuthContinuedSession uses session_key DIRECTLY as HMAC key,
    // unlike AuthSession which uses SHA256(session_key).
    // C# ref: WorldSocket.cs HandleAuthContinuedSessionCallback line 777.

    // DEBUG: Log all HMAC inputs for comparison with C# server
    info!(
        "[DEBUG-HMAC] sessionKey({}): {}",
        session_key.len(),
        session_key
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] authKey(i64): {} bytes: {}",
        auth.key,
        auth.key
            .to_le_bytes()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] localChallenge({}): {}",
        auth.local_challenge.len(),
        auth.local_challenge
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] serverChallenge({}): {}",
        server_challenge.len(),
        server_challenge
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] continuedSeed: {}",
        CONTINUED_SESSION_SEED
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );

    let mut hmac = HmacSha256::new(session_key);
    hmac.update(&auth.key.to_le_bytes());
    hmac.update(&auth.local_challenge);
    hmac.update(&server_challenge);
    hmac.update(&CONTINUED_SESSION_SEED);
    let server_digest = hmac.finalize();

    info!(
        "[DEBUG-HMAC] serverDigest: {}",
        server_digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    info!(
        "[DEBUG-HMAC] clientDigest({}): {}",
        auth.digest.len(),
        auth.digest
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );

    if server_digest[..24] != auth.digest {
        return Err(WorldSocketError::AuthFailed(
            "AuthContinuedSession HMAC digest mismatch".into(),
        ));
    }

    debug!("Instance HMAC validated for account {account_id}");

    // Phase 6: Derive encryption key
    let encrypt_key = {
        let mut hmac = HmacSha256::new(session_key);
        hmac.update(&auth.local_challenge);
        hmac.update(&server_challenge);
        hmac.update(&ENCRYPTION_KEY_SEED);
        let full = hmac.finalize();
        let mut ek = [0u8; 16];
        ek.copy_from_slice(&full[..16]);
        ek
    };

    // Phase 7: Send EnterEncryptedMode
    let signature = sign_enable_encryption(&encrypt_key, true);
    let enter_encrypted = EnterEncryptedMode {
        signature,
        enabled: true,
    };
    socket.send_unencrypted_packet(&enter_encrypted).await?;

    // Phase 8: Wait for EnterEncryptedModeAck
    let ack_pkt = socket.read_unencrypted_packet().await?;
    let ack_opcode = ack_pkt.opcode_raw();
    if ack_opcode != ClientOpcodes::EnterEncryptedModeAck as u16 {
        return Err(WorldSocketError::AuthFailed(format!(
            "expected EnterEncryptedModeAck, got 0x{ack_opcode:04X}"
        )));
    }

    // Phase 9: Enable encryption
    socket.set_encrypt_key(encrypt_key);
    socket.handle_enter_encrypted_mode_ack()?;

    // Phase 10: Create channels and deliver InstanceLink
    let (pkt_tx, pkt_rx) = flume::bounded(256);
    let (send_tx, send_rx_for_socket) = flume::bounded(256);

    // pkt_tx → instance reader feeds decoded packets here
    // pkt_rx → session reads packets from here (via InstanceLink)
    // send_tx → session writes serialized packets here
    // send_rx_for_socket → instance writer reads from here
    let instance_link = InstanceLink {
        send_tx: send_tx.clone(),
        pkt_rx: Some(pkt_rx),
    };

    if validated.instance_link_tx.send(instance_link).is_err() {
        return Err(WorldSocketError::AuthFailed(
            "session dropped before instance link delivery".into(),
        ));
    }

    // Phase 11: Set up socket channels and split for I/O
    socket.set_session_channel(pkt_tx);
    socket.set_send_channel(send_rx_for_socket);
    let pong_tx = send_tx;
    let (reader, writer) = socket.split_for_io(pong_tx);

    // Spawn writer
    tokio::spawn(async move {
        if let Err(e) = writer.run().await {
            match e {
                WorldSocketError::Closed => {}
                _ => error!("Instance writer error for {addr}: {e}"),
            }
        }
    });

    info!("Instance socket fully linked for account {account_id}");

    // Run reader (blocks until disconnect)
    reader.run().await
}
