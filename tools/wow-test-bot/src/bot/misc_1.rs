//! Misc operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) const SMSG_COMPRESSED_PACKET: u16 = 0x3052;
pub(crate) const CMSG_STAND_STATE_CHANGE: u16 = 0x318C;
pub(crate) const SMSG_STAND_STATE_UPDATE: u16 = 0x271C;
pub(crate) const CMSG_PING: u16 = 0x3768;
pub(crate) const SMSG_PONG: u16 = 0x304E;
pub(crate) const SMSG_UPDATE_OBJECT: u16 = 0x27CB;
pub(crate) const SMSG_AURA_UPDATE: u16 = 0x2C1F;
pub(crate) const SMSG_SEND_KNOWN_SPELLS: u16 = 0x2C27;
pub(crate) const SMSG_TIME_SYNC_REQUEST: u16 = 0x2DD2;
pub(crate) const CMSG_TIME_SYNC_RESPONSE: u16 = 0x3A3D;
pub(crate) const SMSG_ON_MONSTER_MOVE: u16 = 0x2DD4;
pub(crate) const SMSG_ATTACK_START: u16 = 0x293D;
pub(crate) const SMSG_ATTACK_STOP: u16 = 0x293E;
pub(crate) const CMSG_BANKER_ACTIVATE: u16 = 0x34B3;
pub(crate) const CMSG_BINDER_ACTIVATE: u16 = 0x34B2;
pub(crate) const CMSG_AUTOBANK_ITEM: u16 = 0x3997;
pub(crate) const CMSG_AUTOSTORE_BANK_ITEM: u16 = 0x3996;
pub(crate) const CMSG_SWAP_INV_ITEM: u16 = 0x399B;
pub(crate) const CMSG_SWAP_ITEM: u16 = 0x399A;
pub(crate) const CMSG_LIST_INVENTORY: u16 = 0x34A1;
pub(crate) const CMSG_BUY_ITEM: u16 = 0x34A3;
pub(crate) const CMSG_ATTACK_SWING: u16 = 0x3255;
pub(crate) const CMSG_MOVE_HEARTBEAT: u16 = 0x3A10;
pub(crate) const CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE: u16 = 0x3A46;
pub(crate) const SMSG_LOG_XP_GAIN: u16 = 0x26E5;
pub(crate) const SMSG_ATTACKER_STATE_UPDATE: u16 = 0x2952;
pub(crate) const SMSG_NPC_INTERACTION_OPEN_RESULT: u16 = 0x288A;
pub(crate) const SMSG_INVENTORY_CHANGE_FAILURE: u16 = 0x2DA5;
pub(crate) const SMSG_BIND_POINT_UPDATE: u16 = 0x257D;
pub(crate) const SMSG_GOSSIP_COMPLETE: u16 = 0x2A97;
pub(crate) const SMSG_PLAYER_BOUND: u16 = 0x2FF8;
pub(crate) const SMSG_SPELL_START: u16 = 0x2C37;
pub(crate) const SMSG_SPELL_GO: u16 = 0x2C36;
pub(crate) const CMSG_LOGOUT_REQUEST: u16 = 0x34D6;
pub(crate) const SMSG_LOGOUT_COMPLETE: u16 = 0x2684;
pub(crate) const SMSG_VENDOR_INVENTORY: u16 = 0x25B8;
pub(crate) const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;
pub(crate) const SMSG_BUY_SUCCEEDED: u16 = 0x26C6;
pub(crate) const SMSG_BUY_FAILED: u16 = 0x26C7;
pub(crate) const SMSG_SET_CURRENCY: u16 = 0x2574;
pub(crate) const CMSG_SAVE_EQUIPMENT_SET: u16 = 0x3509;
pub(crate) const SMSG_EQUIPMENT_SET_ID: u16 = 0x26B2;
pub(crate) const SMSG_LOAD_EQUIPMENT_SET: u16 = 0x270E;
pub(crate) const EQUIPMENT_SET_SLOTS_LIKE_CPP: usize = 19;
pub(crate) const MAX_EQUIPMENT_SET_INDEX_LIKE_CPP: u32 = 20;
pub(crate) const EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP: u32 =
    (1 << EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;
// Keep the mixed signed/unsigned aggregate pinned to an unsigned MySQL wire
// type, matching the world-server startup query for this shared GUID namespace.
pub(crate) const SHARED_EQUIPMENT_SET_GUID_MAX_QUERY: &str = "SELECT CAST(MAX(maxguid) AS UNSIGNED) FROM ((SELECT MAX(setguid) AS maxguid FROM character_equipmentsets) UNION (SELECT MAX(setguid) AS maxguid FROM character_transmog_outfits)) allsets";
// Login can legitimately contain more than 30 packets before
// SMSG_LOGIN_VERIFY_WORLD when another player is already on the map and its
// CREATE/broadcast traffic is interleaved. Keep the guard wall-clock based so
// a busy but healthy login cannot exhaust an arbitrary packet budget.
pub(crate) const LOGIN_VERIFY_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const LOGIN_VERIFY_READ_SLICE: Duration = Duration::from_secs(5);
pub(crate) const INITIAL_NETWORK_IO_TIMEOUT: Duration = Duration::from_secs(15);
// C++ WorldSession::ShouldLogOut waits 20 wall-clock seconds after a normal
// logout request (`WorldSession.h`) before it can send SMSG_LOGOUT_COMPLETE.
// Keep a bounded margin for the world update that observes the expired timer.
pub(crate) const NORMAL_LOGOUT_COMPLETE_WAIT_SECS: u64 = 30;
pub(crate) const RESTED_XP_DISCONNECT_SAVE_MAX_WAIT_SECS: u64 = 90;
pub(crate) const INVENTORY_SLOT_BAG_0: u8 = 255;
pub(crate) const INVENTORY_SLOT_ITEM_START: u8 = 35;
pub(crate) const BANK_SLOT_ITEM_START: u8 = 59;
pub(crate) const BANK_SLOT_ITEM_END: u8 = 87;
pub(crate) const NPC_FLAG_BANKER: u32 = 0x20000;
pub(crate) const NPC_FLAG_INNKEEPER: u32 = 0x10000;
pub(crate) const NPC_FLAG_VAULT_KEEPER: u32 = 0x2000_0000;
pub(crate) const DEFAULT_BANK_SMOKE_ITEM_ENTRY: u32 = 2589;
pub(crate) const DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A: u32 = 2589;
pub(crate) const DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_B: u32 = 2592;
// Issue #20 D-C1/D-C2 live fixture. ItemRandomProperties.db2 record 5 maps
// its first property enchantment to id 79 in the 3.4.3 client data used by
// both runtimes. The persisted enchantment array is authoritative in C++, so
// the fixture stores that generated Property2 value alongside one explicit
// permanent enchantment.
pub(crate) const ISSUE20_ITEM_PERMANENT_ENCHANT_ID: i32 = 2673;
pub(crate) const ISSUE20_ITEM_RANDOM_PROPERTY_ID: i32 = 5;
pub(crate) const ISSUE20_ITEM_RANDOM_PROPERTY_ENCHANT_ID: i32 = 79;
pub(crate) const ISSUE20_ITEM_RANDOM_PROPERTY_SLOT: usize = 10;
pub(crate) const ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT: usize = 13;
pub(crate) const DEFAULT_VENDOR_ENTRY: u32 = 18_525;
pub(crate) const DEFAULT_VENDOR_SPAWN_GUID: u64 = 96_654;
pub(crate) const DEFAULT_VENDOR_ITEM_ENTRY: u32 = 30_183;
pub(crate) const DEFAULT_VENDOR_EXTENDED_COST: u32 = 1_642;
pub(crate) const DEFAULT_VENDOR_CURRENCY_ID: u32 = 42;
pub(crate) const DEFAULT_VENDOR_CURRENCY_COST: u32 = 15;
pub(crate) const DEFAULT_VENDOR_CURRENCY_QUANTITY: u32 = 30;
pub(crate) const VENDOR_CAPTURE_FENCE_SERIAL: u32 = 0x5645_4E44;
pub(crate) const DEFAULT_RESTED_XP_CREATURE_ENTRY: u32 = 15274;
pub(crate) const DEFAULT_RESTED_XP_OFFLINE_SECS: u64 = 86_400;
// A fresh level-1 Mana Wyrm has 42 HP. The intentionally empty disposable
// fixture attacks for 1 damage roughly every two seconds, so 45 seconds could
// never complete a legitimate unarmed kill.
pub(crate) const DEFAULT_RESTED_XP_TIMEOUT_SECS: u64 = 120;
pub(crate) const MIN_RESTED_XP_TARGET_RESPAWN_SECS: u32 = 30;
pub(crate) const MAX_RESTED_XP_TARGET_RESPAWN_SECS: u32 = 600;
pub(crate) const RESTED_XP_RESPAWN_GRACE_SECS: u64 = 15;
pub(crate) const MAX_RESTED_XP_RESPAWN_CLEANUP_WAIT_SECS: u64 = 900;
pub(crate) const ACK_DISPOSABLE_RESTED_XP_FLAG: &str = "--ack-disposable-rested-xp";
pub(crate) const ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG: &str = "--ack-disposable-detour-fixture";
pub(crate) const DETOUR_CHASE_FIXTURE_ACCOUNT: &str = "TESTBOT2@bot.local";
pub(crate) const DETOUR_CHASE_FIXTURE_ACCOUNT_ID: u32 = 9;
pub(crate) const DETOUR_CHASE_FIXTURE_CHARACTER_GUID: u64 = 15;
pub(crate) const DETOUR_CHASE_FIXTURE_FLOW: &str = "detour-chase-around-obstacle";
pub(crate) const DEFAULT_DETOUR_CHASE_TIMEOUT_SECS: u64 = 30;
pub(crate) const DETOUR_CHASE_TARGET_MATCH_RADIUS: f32 = 0.5;
pub(crate) const DETOUR_CHASE_QUIET_PERIOD: Duration = Duration::from_millis(350);
pub(crate) const DETOUR_CHASE_MAP_ASSET_SHA256: &str =
    "3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a";
pub(crate) const DETOUR_CHASE_TILE_ASSET_SHA256: &str =
    "693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47";
pub(crate) const ISSUE_24_PING_FENCE_WIRE: [u8; 4] = *b"DTOR";
pub(crate) const ISSUE_24_PING_FENCE_SERIAL: u32 = u32::from_le_bytes(ISSUE_24_PING_FENCE_WIRE);
pub(crate) const CREATURE_SPELL_FIXTURE_FLOW: &str = "creature-spell-casting";
pub(crate) const CREATURE_SPELL_FIXTURE_CONTRACT: &str = "creature-spell-casting-shell-fixture-v2";
pub(crate) const CREATURE_SPELL_FIXTURE_ACCOUNT: &str = "TESTBOT2@bot.local";
pub(crate) const CREATURE_SPELL_FIXTURE_ACCOUNT_ID: u32 = 9;
pub(crate) const CREATURE_SPELL_FIXTURE_CHARACTER_GUID: u64 = 15;
pub(crate) const CREATURE_SPELL_FIXTURE_ENTRY: u32 = 22_378;
pub(crate) const CREATURE_SPELL_FIXTURE_SPAWN_GUID: u64 = 78_686;
pub(crate) const CREATURE_SPELL_FIXTURE_SPELL_ID: u32 = 15_691;
pub(crate) const CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID: u32 = 8_326;
pub(crate) const CREATURE_SPELL_FIXTURE_SPELL_X_VISUAL_ID: u32 = 244_493;
pub(crate) const CREATURE_SPELL_FIXTURE_MAP_ID: u16 = 530;
pub(crate) const CREATURE_SPELL_FIXTURE_X: f32 = -2_764.52;
pub(crate) const CREATURE_SPELL_FIXTURE_Y: f32 = 5_431.19;
pub(crate) const CREATURE_SPELL_FIXTURE_Z: f32 = -34.4548;
pub(crate) const CREATURE_SPELL_CHARACTER_START_X: f32 = -2_749.52;
pub(crate) const CREATURE_SPELL_CHARACTER_PULL_X: f32 = -2_760.52;
pub(crate) const CREATURE_SPELL_CHARACTER_ORIENTATION: f32 = 0.0;
pub(crate) const CREATURE_SPELL_TARGET_MATCH_RADIUS: f32 = 0.5;
pub(crate) const DEFAULT_CREATURE_SPELL_CAPTURE_TIMEOUT_SECS: u64 = 30;
pub(crate) const CREATURE_SPELL_FIXTURE_MANIFEST_SHA256: &str =
    "3cef5dd6201c88fc85c1c2cb767fec27cd11921ec7ecdc2c7705379fd54e356d";
pub(crate) const REST_STATE_RESTED: u8 = 1;
pub(crate) const REST_STATE_NORMAL: u8 = 2;
pub(crate) const PLAYER_FLAGS_RESTING: u32 = 0x0000_0020;
pub(crate) const PLAYER_FLAGS_NO_XP_GAIN: u32 = 0x0200_0000;
pub(crate) const CREATURE_STATIC_FLAG_NO_XP: u32 = 0x0000_0002;
pub(crate) const CREATURE_FLAG_EXTRA_NO_XP: u32 = 0x0000_0040;
pub(crate) const CREATURE_TYPE_CRITTER: u8 = 8;
pub(crate) const OBJECT_GUID_COUNTER_MASK: u64 = 0xFF_FFFF_FFFF;
pub(crate) const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;
pub(crate) const RESTED_XP_INSTANCE_OBSERVATION_WINDOW: Duration = Duration::from_secs(2);
// Both legacy C++ implementations use `NextLevelXP * 1.5f / 2`.
pub(crate) const REST_BONUS_CAP_NEXT_LEVEL_FACTOR: f32 = 1.5 / 2.0;
pub(crate) const REST_OFFLINE_WILDERNESS_BUBBLE: f32 = 0.031;
pub(crate) const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE: f32 = 0.125;
pub(crate) const UNIT_STAND_STATE_STAND: u8 = 0;
pub(crate) const UNIT_STAND_STATE_SIT: u8 = 1;
pub(crate) const UNIT_STAND_STATE_SLEEP: u8 = 3;
pub(crate) const UNIT_STAND_STATE_KNEEL: u8 = 8;
pub(crate) const STAND_STATE_LOGIN_QUIET_PERIOD: Duration = Duration::from_millis(500);
pub(crate) const STAND_STATE_LOGIN_DRAIN_LIMIT: Duration = Duration::from_secs(5);
pub(crate) const STAND_STATE_POST_ACTION_QUIET_PERIOD: Duration = Duration::from_millis(250);
pub(crate) const STAND_STATE_POST_ACTION_DRAIN_LIMIT: Duration = Duration::from_secs(5);
pub(crate) const STAND_STATE_CAPTURE_FENCE_SERIAL: u32 = 0x5354_414E;

#[derive(Debug)]
pub(crate) struct ServerPacketInflater {
    pub(crate) decompressor: Decompress,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoginVerifyBudget {
    pub(crate) deadline: tokio::time::Instant,
}
impl LoginVerifyBudget {
    pub(crate) fn new(timeout: Duration) -> Self {
        Self {
            deadline: tokio::time::Instant::now() + timeout,
        }
    }

    pub(crate) fn next_read_timeout(self) -> Option<Duration> {
        let remaining = self
            .deadline
            .saturating_duration_since(tokio::time::Instant::now());
        (!remaining.is_zero()).then(|| remaining.min(LOGIN_VERIFY_READ_SLICE))
    }
}
impl Default for ServerPacketInflater {
    fn default() -> Self {
        Self {
            decompressor: Decompress::new(false),
        }
    }
}
pub(crate) fn world_host() -> String {
    std::env::var("WORLD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}
pub(crate) fn world_port() -> u16 {
    std::env::var("WORLD_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8085)
}
pub(crate) fn client_build() -> u32 {
    std::env::var("WOW_BOT_CLIENT_BUILD")
        .or_else(|_| std::env::var("WOW_BOT_BUILD"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(54261)
}
pub(crate) fn world_db_url() -> Result<String> {
    database_url("WOW_BOT_WORLD_DB_URL", "WorldDatabaseInfo")
}
pub(crate) const DEFAULT_DUNGEON_ID: u32 = 259;
pub(crate) const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2, 0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct QuestObjectiveDbRow {
    pub(crate) objective: u8,
    pub(crate) data: i32,
}
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct BotRunResult {
    pub(crate) account: String,
    pub(crate) account_id: u32,
    pub(crate) character_guid: u64,
    pub(crate) dungeon_id: u32,
    pub(crate) role: u8,
    pub(crate) join_result: Option<u8>,
    pub(crate) join_detail: Option<u8>,
    pub(crate) got_proposal: bool,
    pub(crate) accepted_proposal: bool,
    pub(crate) got_ready_check: bool,
    pub(crate) group_formed: bool,
    pub(crate) teleport_denied_reason: Option<u8>,
    pub(crate) entered_world: bool,
    pub(crate) world_auth: bool,
    pub(crate) enum_characters: bool,
    pub(crate) player_login_verified: bool,
    pub(crate) login_instance_object_update_seen: bool,
    pub(crate) login_stream_drained: bool,
    pub(crate) login_save: Option<login_save::Evidence>,
    pub(crate) spell_acquisition: Option<spell_acquisition::Evidence>,
    pub(crate) cast_lifecycle: Option<cast_lifecycle::Evidence>,
    pub(crate) login_only: bool,
    pub(crate) stand_state_smoke: bool,
    pub(crate) stand_state_smoke_passed: Option<bool>,
    pub(crate) stand_states_requested: Vec<u8>,
    pub(crate) stand_states_confirmed: Vec<u8>,
    pub(crate) stand_state_failure: Option<String>,
    pub(crate) bank_smoke: bool,
    pub(crate) bank_smoke_passed: Option<bool>,
    pub(crate) bank_banker_entry: Option<u32>,
    pub(crate) bank_banker_spawn_guid: Option<u64>,
    pub(crate) bank_banker_guid_counter: Option<u64>,
    pub(crate) bank_item_guid: Option<u64>,
    pub(crate) bank_item_entry: Option<u32>,
    pub(crate) bank_inventory_slot: Option<u8>,
    pub(crate) bank_bank_slot: Option<u8>,
    pub(crate) bank_open_confirmed: bool,
    pub(crate) bank_deposit_persisted: bool,
    pub(crate) bank_relogin_after_deposit: bool,
    pub(crate) bank_withdraw_persisted: bool,
    pub(crate) bank_failure: Option<String>,
    pub(crate) void_storage_smoke: bool,
    pub(crate) void_storage_smoke_passed: Option<bool>,
    pub(crate) void_storage_query_capture: bool,
    pub(crate) void_storage_query_capture_passed: Option<bool>,
    pub(crate) void_storage_unlock_persisted: bool,
    pub(crate) void_storage_deposit_persisted: bool,
    pub(crate) void_storage_deposit_relogin_verified: bool,
    pub(crate) void_storage_swap_persisted: bool,
    pub(crate) void_storage_swap_relogin_verified: bool,
    pub(crate) void_storage_withdraw_persisted: bool,
    pub(crate) void_storage_withdraw_relogin_verified: bool,
    pub(crate) void_storage_item_id: Option<u64>,
    pub(crate) void_storage_failure: Option<String>,
    pub(crate) homebind_smoke: bool,
    pub(crate) homebind_smoke_passed: Option<bool>,
    pub(crate) homebind_innkeeper_entry: Option<u32>,
    pub(crate) homebind_innkeeper_spawn_guid: Option<u64>,
    pub(crate) homebind_innkeeper_guid_counter: Option<u64>,
    pub(crate) homebind_spell_go_seen: bool,
    pub(crate) homebind_bind_point_update_seen: bool,
    pub(crate) homebind_player_bound_seen: bool,
    pub(crate) homebind_gossip_complete_seen: bool,
    pub(crate) homebind_db_persisted: bool,
    pub(crate) homebind_relogin_verified: bool,
    pub(crate) homebind_failure: Option<String>,
    pub(crate) inventory_swap_smoke: bool,
    pub(crate) inventory_swap_smoke_passed: Option<bool>,
    pub(crate) inventory_swap_item_guid_a: Option<u64>,
    pub(crate) inventory_swap_item_guid_b: Option<u64>,
    pub(crate) inventory_swap_item_entry_a: Option<u32>,
    pub(crate) inventory_swap_item_entry_b: Option<u32>,
    pub(crate) inventory_swap_slot_a: Option<u8>,
    pub(crate) inventory_swap_slot_b: Option<u8>,
    pub(crate) inventory_swap_forward_persisted: bool,
    pub(crate) inventory_swap_relogin_after_forward: bool,
    pub(crate) inventory_swap_reverse_persisted: bool,
    pub(crate) inventory_swap_relogin_after_reverse: bool,
    pub(crate) inventory_swap_item_create_sha256: Option<String>,
    pub(crate) inventory_swap_item_create_relogin_verified: bool,
    pub(crate) inventory_swap_item_metadata_persisted: bool,
    pub(crate) inventory_swap_validation_gate_seen: bool,
    pub(crate) inventory_swap_failure: Option<String>,
    pub(crate) vendor_smoke: bool,
    pub(crate) vendor_smoke_passed: Option<bool>,
    pub(crate) vendor_entry: Option<u32>,
    pub(crate) vendor_spawn_guid: Option<u64>,
    pub(crate) vendor_runtime_counter: Option<u64>,
    pub(crate) vendor_item_entry: Option<u32>,
    pub(crate) vendor_extended_cost: Option<u32>,
    pub(crate) vendor_currency_id: Option<u32>,
    pub(crate) vendor_currency_before: Option<u32>,
    pub(crate) vendor_currency_after: Option<u32>,
    pub(crate) vendor_item_total_after: Option<u64>,
    pub(crate) vendor_inventory_seen: bool,
    pub(crate) vendor_buy_succeeded_seen: bool,
    pub(crate) vendor_set_currency_seen: bool,
    pub(crate) vendor_item_push_seen: bool,
    pub(crate) vendor_relogin_verified: bool,
    pub(crate) vendor_failure: Option<String>,
    pub(crate) equipment_set_smoke: bool,
    pub(crate) equipment_set_smoke_passed: Option<bool>,
    pub(crate) equipment_set_type: Option<i32>,
    pub(crate) equipment_set_id: Option<u32>,
    pub(crate) equipment_set_generated_guid: Option<u64>,
    pub(crate) equipment_set_login_count: Option<u32>,
    pub(crate) equipment_set_load_seen: bool,
    pub(crate) equipment_set_db_persisted: bool,
    pub(crate) equipment_set_relogin_verified: bool,
    pub(crate) equipment_set_failure: Option<String>,
    pub(crate) rested_xp_smoke: bool,
    pub(crate) rested_xp_smoke_passed: Option<bool>,
    pub(crate) rested_xp_offline_wilderness_bonus: Option<f32>,
    pub(crate) rested_xp_offline_resting_bonus: Option<f32>,
    pub(crate) rested_xp_target_entry: Option<u32>,
    pub(crate) rested_xp_target_spawn_guid: Option<u64>,
    pub(crate) rested_xp_target_guid_counter: Option<u64>,
    pub(crate) rested_xp_packet_amount: Option<i32>,
    pub(crate) rested_xp_packet_original: Option<i32>,
    pub(crate) rested_xp_db_xp_before: Option<u32>,
    pub(crate) rested_xp_db_xp_after: Option<u32>,
    pub(crate) rested_xp_db_rest_before: Option<f32>,
    pub(crate) rested_xp_db_rest_after: Option<f32>,
    pub(crate) rested_xp_relog_verified: bool,
    pub(crate) rested_xp_failure: Option<String>,
    pub(crate) detour_chase_capture: bool,
    pub(crate) detour_chase_capture_passed: Option<bool>,
    pub(crate) detour_chase_target_entry: Option<u32>,
    pub(crate) detour_chase_target_spawn_guid: Option<u64>,
    pub(crate) detour_chase_target_runtime_counter: Option<u64>,
    pub(crate) detour_chase_target_discovered: bool,
    pub(crate) detour_chase_active_mover_ack_sent: bool,
    pub(crate) detour_chase_attack_start_confirmed: bool,
    pub(crate) detour_chase_first_swing_confirmed: bool,
    pub(crate) detour_chase_prewindow_target_moves: u32,
    pub(crate) detour_chase_heartbeat_sent: bool,
    pub(crate) detour_chase_heartbeat_sha256: Option<String>,
    pub(crate) detour_chase_window_target_moves: u32,
    pub(crate) detour_chase_monster_move_sha256: Option<String>,
    pub(crate) detour_chase_monster_move_bytes: Option<usize>,
    pub(crate) detour_chase_ping_serial: Option<u32>,
    pub(crate) detour_chase_pong_confirmed: bool,
    pub(crate) detour_chase_time_sync_before_window: u32,
    pub(crate) detour_chase_time_sync_during_window: u32,
    pub(crate) detour_chase_time_sync_after_fence: u32,
    pub(crate) detour_chase_logout_confirmed: bool,
    pub(crate) detour_chase_failure: Option<String>,
    pub(crate) creature_spell_capture: bool,
    pub(crate) creature_spell_capture_passed: Option<bool>,
    pub(crate) creature_spell_fixture_manifest_sha256: Option<String>,
    pub(crate) creature_spell_target_entry: Option<u32>,
    pub(crate) creature_spell_target_spawn_guid: Option<u64>,
    pub(crate) creature_spell_target_runtime_counter: Option<u64>,
    pub(crate) creature_spell_target_discovered: bool,
    pub(crate) creature_spell_heartbeat_sent: bool,
    pub(crate) creature_spell_heartbeat_sha256: Option<String>,
    pub(crate) creature_spell_start_opcode: Option<u16>,
    pub(crate) creature_spell_start_body_sha256: Option<String>,
    pub(crate) creature_spell_start_body_bytes: Option<usize>,
    pub(crate) creature_spell_go_opcode: Option<u16>,
    pub(crate) creature_spell_go_body_sha256: Option<String>,
    pub(crate) creature_spell_go_body_bytes: Option<usize>,
    pub(crate) creature_spell_cast_id_low: Option<u64>,
    pub(crate) creature_spell_cast_id_high: Option<u64>,
    pub(crate) creature_spell_caster_guid_low: Option<u64>,
    pub(crate) creature_spell_caster_guid_high: Option<u64>,
    pub(crate) creature_spell_victim_guid_low: Option<u64>,
    pub(crate) creature_spell_victim_guid_high: Option<u64>,
    pub(crate) creature_spell_spell_id: Option<u32>,
    pub(crate) creature_spell_start_cast_flags: Option<u32>,
    pub(crate) creature_spell_go_cast_flags: Option<u32>,
    pub(crate) creature_spell_cast_flags_ex: Option<u32>,
    pub(crate) creature_spell_go_hit_target_count: Option<u16>,
    pub(crate) creature_spell_go_miss_target_count: Option<u16>,
    pub(crate) creature_spell_full_combat_log: Option<bool>,
    pub(crate) creature_spell_advanced_logging_sent: bool,
    pub(crate) creature_spell_adjacent_start_go: bool,
    pub(crate) creature_spell_disconnect_confirmed: bool,
    pub(crate) creature_spell_logout_confirmed: bool,
    pub(crate) creature_spell_failure: Option<String>,
    pub(crate) loot_race_smoke: bool,
    pub(crate) loot_race_smoke_passed: Option<bool>,
    pub(crate) loot_race_target_entry: Option<u32>,
    pub(crate) loot_race_target_spawn_guid: Option<u64>,
    pub(crate) loot_race_target_runtime_counter: Option<u64>,
    pub(crate) loot_race_party_confirmed: bool,
    pub(crate) loot_race_target_discovered: bool,
    pub(crate) loot_race_loot_opened: bool,
    pub(crate) loot_race_loot_list_id: Option<u8>,
    pub(crate) loot_race_loot_coins: Option<u32>,
    pub(crate) loot_race_item_push_seen: bool,
    pub(crate) loot_race_loot_removed_seen: bool,
    pub(crate) loot_race_money_notify_amount: Option<u64>,
    pub(crate) loot_race_coin_removed_seen: bool,
    pub(crate) loot_race_db_item_total: Option<u64>,
    pub(crate) loot_race_db_money_delta: Option<u64>,
    pub(crate) loot_race_relog_verified: bool,
    pub(crate) loot_race_failure: Option<String>,
    pub(crate) group_capacity_race_smoke: bool,
    pub(crate) group_capacity_race_smoke_passed: Option<bool>,
    pub(crate) group_capacity_group_id: Option<u32>,
    pub(crate) group_capacity_outcome: Option<String>,
    pub(crate) group_capacity_final_member_count: Option<u64>,
    pub(crate) group_capacity_failure: Option<String>,
    pub(crate) quest_smoke: bool,
    pub(crate) quest_smoke_passed: Option<bool>,
    pub(crate) quest_target_entry: Option<u32>,
    pub(crate) quest_target_spawn_guid: Option<u64>,
    pub(crate) quest_target_guid_counter: Option<u64>,
    pub(crate) quest_target_map_id: Option<u16>,
    pub(crate) quest_gossip_hello_sent: bool,
    pub(crate) quest_questgiver_hello_sent: bool,
    pub(crate) quest_gossip_id_seen: Option<i32>,
    pub(crate) quest_gossip_select_sent: bool,
    pub(crate) quest_gossip_message_seen: bool,
    pub(crate) quest_quest_list_seen: bool,
    pub(crate) quest_details_seen: bool,
    pub(crate) quest_request_items_seen: bool,
    pub(crate) trainer_list_seen: bool,
    pub(crate) trainer_id_seen: Option<i32>,
    pub(crate) trainer_spell_count_seen: Option<u32>,
    pub(crate) quest_accept_sent: bool,
    pub(crate) quest_accept_confirm_seen: bool,
    pub(crate) quest_db_verified: bool,
    pub(crate) quest_db_status: Option<u8>,
    pub(crate) quest_objective_persist: bool,
    pub(crate) quest_objective_seeded: Vec<QuestObjectiveDbRow>,
    pub(crate) quest_objective_db_before: Vec<QuestObjectiveDbRow>,
    pub(crate) quest_objective_db_after: Vec<QuestObjectiveDbRow>,
    pub(crate) quest_objective_db_verified: bool,
    pub(crate) quest_objective_update_seen: bool,
    pub(crate) quest_objective_update_has_expected: bool,
    pub(crate) quest_ids_seen: Vec<u32>,
    pub(crate) quest_titles_seen: Vec<String>,
    pub(crate) quest_failure: Option<String>,
    pub(crate) seen_opcodes: Vec<String>,
}
impl BotRunResult {
    pub(crate) fn success(
        &self,
        require_proposal: bool,
        require_group: bool,
        login_only: bool,
    ) -> bool {
        if self.stand_state_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.stand_state_smoke_passed.unwrap_or(false);
        }
        if self.quest_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.quest_smoke_passed.unwrap_or(false);
        }
        if self.bank_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.bank_smoke_passed.unwrap_or(false);
        }
        if self.void_storage_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.void_storage_smoke_passed.unwrap_or(false)
                && self.void_storage_unlock_persisted
                && self.void_storage_deposit_persisted
                && self.void_storage_deposit_relogin_verified
                && self.void_storage_swap_persisted
                && self.void_storage_swap_relogin_verified
                && self.void_storage_withdraw_persisted
                && self.void_storage_withdraw_relogin_verified;
        }
        if self.void_storage_query_capture {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.void_storage_query_capture_passed.unwrap_or(false);
        }
        if self.homebind_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.homebind_smoke_passed.unwrap_or(false);
        }
        if self.inventory_swap_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.inventory_swap_smoke_passed.unwrap_or(false)
                && self.inventory_swap_item_create_sha256.is_some()
                && self.inventory_swap_item_create_relogin_verified
                && self.inventory_swap_relogin_after_reverse
                && self.inventory_swap_item_metadata_persisted
                && self.inventory_swap_validation_gate_seen;
        }
        if self.vendor_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.vendor_smoke_passed.unwrap_or(false)
                && self.vendor_relogin_verified;
        }
        if self.equipment_set_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.equipment_set_smoke_passed.unwrap_or(false)
                && self.equipment_set_db_persisted
                && self.equipment_set_relogin_verified;
        }
        if self.rested_xp_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.rested_xp_smoke_passed.unwrap_or(false);
        }
        if self.detour_chase_capture {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.detour_chase_capture_passed.unwrap_or(false)
                && self.detour_chase_target_discovered
                && self.detour_chase_active_mover_ack_sent
                && self.detour_chase_attack_start_confirmed
                && self.detour_chase_first_swing_confirmed
                && self.detour_chase_heartbeat_sent
                && self.detour_chase_heartbeat_sha256.is_some()
                && self.detour_chase_window_target_moves == 1
                && self.detour_chase_monster_move_sha256.is_some()
                && self.detour_chase_monster_move_bytes.unwrap_or(0) > 0
                && self.detour_chase_ping_serial == Some(ISSUE_24_PING_FENCE_SERIAL)
                && self.detour_chase_pong_confirmed
                && self.detour_chase_logout_confirmed;
        }
        if self.creature_spell_capture {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.creature_spell_capture_passed.unwrap_or(false)
                && self.creature_spell_target_discovered
                && self.creature_spell_heartbeat_sent
                && self.creature_spell_start_opcode == Some(SMSG_SPELL_START)
                && self.creature_spell_go_opcode == Some(SMSG_SPELL_GO)
                && self.creature_spell_spell_id == Some(CREATURE_SPELL_FIXTURE_SPELL_ID)
                && self.creature_spell_go_hit_target_count == Some(1)
                && self.creature_spell_go_miss_target_count == Some(0)
                && self.creature_spell_full_combat_log == Some(false)
                && !self.creature_spell_advanced_logging_sent
                && self.creature_spell_adjacent_start_go
                && self.creature_spell_disconnect_confirmed
                && !self.creature_spell_logout_confirmed;
        }
        if self.cast_lifecycle.is_some() {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self
                    .cast_lifecycle
                    .as_ref()
                    .is_some_and(|evidence| evidence.passed);
        }
        if self.loot_race_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.loot_race_smoke_passed.unwrap_or(false)
                && self.loot_race_relog_verified;
        }
        if self.group_capacity_race_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.group_capacity_race_smoke_passed.unwrap_or(false)
                && self.group_capacity_final_member_count == Some(5);
        }
        if login_only {
            return self.world_auth && self.enum_characters && self.player_login_verified;
        }
        self.join_result == Some(0)
            && (!require_proposal || self.got_proposal)
            && (!require_group || self.group_formed)
    }
}
#[derive(Debug, Serialize)]
pub(crate) struct RunReport {
    pub(crate) dungeon_id: u32,
    pub(crate) timeout_secs: u64,
    pub(crate) require_proposal: bool,
    pub(crate) require_group: bool,
    pub(crate) auto_teleport: bool,
    pub(crate) login_only: bool,
    pub(crate) stand_state_smoke: bool,
    pub(crate) bank_smoke: bool,
    pub(crate) void_storage_smoke: bool,
    pub(crate) void_storage_query_capture: bool,
    pub(crate) homebind_smoke: bool,
    pub(crate) inventory_swap_smoke: bool,
    pub(crate) vendor_smoke: bool,
    pub(crate) equipment_set_race_smoke: bool,
    pub(crate) rested_xp_smoke: bool,
    pub(crate) detour_chase_capture: bool,
    pub(crate) creature_spell_capture: bool,
    pub(crate) cast_lifecycle: bool,
    pub(crate) loot_race_smoke: bool,
    pub(crate) loot_item_capture: bool,
    pub(crate) group_capacity_race_smoke: bool,
    pub(crate) quest_smoke: bool,
    pub(crate) results: Vec<BotRunResult>,
}
#[derive(Debug, Clone)]
pub(crate) struct ConnectToTarget {
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
    pub(crate) serial: u32,
    pub(crate) connection_type: u8,
    pub(crate) key: i64,
}
pub(crate) struct EncryptedWorldConnection {
    pub(crate) stream: TcpStream,
    pub(crate) crypt: WorldCrypt,
    pub(crate) inflater: ServerPacketInflater,
}
#[derive(Debug, Clone)]
pub(crate) struct WorldAuthDbContext {
    pub(crate) username: String,
    pub(crate) realm_build: u32,
    pub(crate) win64_auth_seed: [u8; 16],
}
#[derive(Debug, Clone)]
pub(crate) struct QuestSmokeOptions {
    pub(crate) creature_entry: u32,
    pub(crate) creature_spawn_guid: Option<u64>,
    pub(crate) creature_guid_counter: Option<u64>,
    pub(crate) map_id: Option<u16>,
    pub(crate) expected_quest_id: Option<u32>,
    pub(crate) forbidden_quest_id: Option<u32>,
    pub(crate) forbidden_title_contains: Option<String>,
    pub(crate) query_details: bool,
    pub(crate) accept: bool,
    pub(crate) reset_before_run: bool,
    pub(crate) relocate_before_login: bool,
    pub(crate) set_level_before_login: Option<u8>,
    pub(crate) set_race_before_login: Option<u8>,
    pub(crate) set_class_before_login: Option<u8>,
    pub(crate) objective_persist: bool,
    pub(crate) objective_seed: Vec<QuestObjectiveDbRow>,
    pub(crate) objective_status: u8,
    pub(crate) gossip_select_option_id: Option<i32>,
    pub(crate) expect_trainer_list: bool,
    pub(crate) expect_trainer_id: Option<i32>,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone)]
pub(crate) struct StandStateSmokeOptions {
    pub(crate) states: Vec<u8>,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BankSmokePhase {
    Deposit,
    Withdraw,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VoidStorageSmokePhase {
    UnlockDeposit,
    VerifyDepositSwap,
    VerifySwapWithdraw,
    VerifyWithdraw,
    QueryCapture,
}
#[derive(Debug, Clone)]
pub(crate) struct VoidStorageSmokeOptions {
    pub(crate) phase: VoidStorageSmokePhase,
    pub(crate) vault_keeper: ResolvedCreatureTarget,
    pub(crate) runtime_realm_id: u16,
    pub(crate) discover_runtime_guid: bool,
    pub(crate) fixture_item_guid: u64,
    pub(crate) item_entry: u32,
    pub(crate) inventory_slot: u8,
    pub(crate) expected_void_item_id: Option<u64>,
    pub(crate) expected_void_slot: u8,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone)]
pub(crate) struct VoidStorageSmokeFixture {
    pub(crate) options: VoidStorageSmokeOptions,
    pub(crate) original_position: CharacterPositionSnapshot,
    pub(crate) original_money: u64,
    pub(crate) original_player_flags: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VoidStorageItemWire {
    pub(crate) item_id: u64,
    pub(crate) slot: u32,
    pub(crate) item_entry: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoidStorageDbState {
    pub(crate) money: u64,
    pub(crate) player_flags: u32,
    pub(crate) void_items: Vec<(u64, u32, u8)>,
    pub(crate) inventory_items: Vec<(u64, u8, u32)>,
}
#[derive(Debug, Clone)]
pub(crate) struct BankSmokeOptions {
    pub(crate) phase: BankSmokePhase,
    pub(crate) banker: ResolvedCreatureTarget,
    pub(crate) item_guid: u64,
    pub(crate) item_entry: u32,
    pub(crate) inventory_slot: u8,
    pub(crate) bank_slot: u8,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize)]
pub(crate) struct CharacterPositionSnapshot {
    pub(crate) map_id: u32,
    pub(crate) zone_id: u32,
    pub(crate) instance_id: u32,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) orientation: f32,
}
#[derive(Debug, Clone)]
pub(crate) struct BankSmokeFixture {
    pub(crate) options: BankSmokeOptions,
    pub(crate) original_position: CharacterPositionSnapshot,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HomebindSmokePhase {
    Bind,
    VerifyRelog,
}
#[derive(Debug, Clone)]
pub(crate) struct HomebindSmokeOptions {
    pub(crate) phase: HomebindSmokePhase,
    pub(crate) innkeeper: ResolvedCreatureTarget,
    pub(crate) discover_runtime_guid: bool,
    pub(crate) expected_homebind: Option<HomebindRowSnapshot>,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HomebindRowSnapshot {
    pub(crate) map_id: u16,
    pub(crate) zone_id: u16,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) orientation: f32,
}
#[derive(Debug, Clone)]
pub(crate) struct HomebindSmokeFixture {
    pub(crate) options: HomebindSmokeOptions,
    pub(crate) original_position: CharacterPositionSnapshot,
    pub(crate) original_homebind: Option<HomebindRowSnapshot>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InventorySwapSmokePhase {
    Forward,
    Reverse,
    VerifyReverseRelog,
}
#[derive(Debug, Clone)]
pub(crate) struct InventorySwapSmokeOptions {
    pub(crate) phase: InventorySwapSmokePhase,
    pub(crate) item_guid_a: u64,
    pub(crate) item_guid_b: u64,
    pub(crate) item_entry_a: u32,
    pub(crate) item_entry_b: u32,
    pub(crate) slot_a: u8,
    pub(crate) slot_b: u8,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Issue20ItemCreateEvidence {
    pub(crate) block_sha256: String,
    pub(crate) enchantments: [i32; ISSUE20_ITEM_ENCHANTMENT_SLOT_COUNT],
    pub(crate) random_properties_seed: i32,
    pub(crate) random_properties_id: i32,
}
#[derive(Debug, Clone)]
pub(crate) struct InventorySwapSmokeFixture {
    pub(crate) options: InventorySwapSmokeOptions,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VendorSmokePhase {
    Purchase,
    VerifyRelog,
}
#[derive(Debug, Clone)]
pub(crate) struct VendorSmokeOptions {
    pub(crate) phase: VendorSmokePhase,
    pub(crate) vendor: ResolvedCreatureTarget,
    pub(crate) target_match_radius: f32,
    pub(crate) item_entry: u32,
    pub(crate) extended_cost: u32,
    pub(crate) currency_id: u32,
    pub(crate) currency_before: u32,
    pub(crate) currency_cost: u32,
    pub(crate) expected_item_total: u64,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorCurrencyRowSnapshot {
    pub(crate) quantity: u32,
    pub(crate) weekly_quantity: u32,
    pub(crate) tracked_quantity: u32,
    pub(crate) increased_cap_quantity: u32,
    pub(crate) earned_quantity: u32,
    pub(crate) flags: u8,
}
#[derive(Debug, Clone)]
pub(crate) struct VendorSmokeFixture {
    pub(crate) options: VendorSmokeOptions,
    pub(crate) original_position: CharacterPositionSnapshot,
    pub(crate) original_currency: Option<VendorCurrencyRowSnapshot>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipmentSetSmokePhase {
    Save,
    VerifyRelog,
}
#[derive(Debug, Clone)]
pub(crate) struct EquipmentSetSmokeOptions {
    pub(crate) phase: EquipmentSetSmokePhase,
    pub(crate) set_type: i32,
    pub(crate) set_id: u32,
    pub(crate) set_name: String,
    pub(crate) set_icon: String,
    pub(crate) expected_guid: Option<u64>,
    pub(crate) save_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone)]
pub(crate) struct EquipmentSetSmokeFixture {
    pub(crate) initial_max_guid: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquipmentSetWire {
    pub(crate) set_type: i32,
    pub(crate) guid: u64,
    pub(crate) set_id: u32,
    pub(crate) ignore_mask: u32,
    pub(crate) pieces: [[u8; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub(crate) appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub(crate) enchants: [i32; 2],
    pub(crate) secondary_appearances_and_slots: [i32; 4],
    pub(crate) assigned_spec_index: i32,
    pub(crate) set_name: String,
    pub(crate) set_icon: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquipmentSetDbRow {
    pub(crate) set_guid: u64,
    pub(crate) set_index: u32,
    pub(crate) name: String,
    pub(crate) icon_name: String,
    pub(crate) ignore_mask: u32,
    pub(crate) assigned_spec_index: i32,
    pub(crate) items: [u64; EQUIPMENT_SET_SLOTS_LIKE_CPP],
}
