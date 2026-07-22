//! WoW Test Bot - TrinityCore 3.4.3 Modern Protocol with Full SRP6
//! Combines BNet SRP6 Auth + World Server AES-GCM Encryption + LFG

use anyhow::{anyhow, bail, Context, Result};
use flate2::{Decompress, FlushDecompress};
use rand::RngCore;
use serde::Serialize;
use std::net::IpAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

mod bot_srp6;
mod config;
mod loot_race;
mod packet_parser;
mod protocol;
mod srp6_auth;
mod wow_crypto;

use packet_parser::*;
use protocol::*;
use wow_crypto::WorldCrypt;

const SMSG_COMPRESSED_PACKET: u16 = 0x3052;
const CMSG_STAND_STATE_CHANGE: u16 = 0x318C;
const SMSG_STAND_STATE_UPDATE: u16 = 0x271C;
const CMSG_PING: u16 = 0x3768;
const SMSG_PONG: u16 = 0x304E;
const SMSG_UPDATE_OBJECT: u16 = 0x27CB;
const SMSG_AURA_UPDATE: u16 = 0x2C1F;
const SMSG_TIME_SYNC_REQUEST: u16 = 0x2DD2;
const CMSG_TIME_SYNC_RESPONSE: u16 = 0x3A3D;
const SMSG_ON_MONSTER_MOVE: u16 = 0x2DD4;
const SMSG_ATTACK_STOP: u16 = 0x293E;
const CMSG_BANKER_ACTIVATE: u16 = 0x34B3;
const CMSG_BINDER_ACTIVATE: u16 = 0x34B2;
const CMSG_AUTOBANK_ITEM: u16 = 0x3997;
const CMSG_AUTOSTORE_BANK_ITEM: u16 = 0x3996;
const CMSG_SWAP_INV_ITEM: u16 = 0x399B;
const CMSG_LIST_INVENTORY: u16 = 0x34A1;
const CMSG_BUY_ITEM: u16 = 0x34A3;
const CMSG_ATTACK_SWING: u16 = 0x3255;
const CMSG_MOVE_HEARTBEAT: u16 = 0x3A10;
const CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE: u16 = 0x3A46;
const SMSG_LOG_XP_GAIN: u16 = 0x26E5;
const SMSG_ATTACKER_STATE_UPDATE: u16 = 0x2952;
const SMSG_NPC_INTERACTION_OPEN_RESULT: u16 = 0x288A;
const SMSG_INVENTORY_CHANGE_FAILURE: u16 = 0x2DA5;
const SMSG_BIND_POINT_UPDATE: u16 = 0x257D;
const SMSG_GOSSIP_COMPLETE: u16 = 0x2A97;
const SMSG_PLAYER_BOUND: u16 = 0x2FF8;
const SMSG_SPELL_GO: u16 = 0x2C36;
const CMSG_LOGOUT_REQUEST: u16 = 0x34D6;
const SMSG_LOGOUT_COMPLETE: u16 = 0x2684;
const SMSG_VENDOR_INVENTORY: u16 = 0x25B8;
const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;
const SMSG_BUY_SUCCEEDED: u16 = 0x26C6;
const SMSG_BUY_FAILED: u16 = 0x26C7;
const SMSG_SET_CURRENCY: u16 = 0x2574;
const CMSG_SAVE_EQUIPMENT_SET: u16 = 0x3509;
const CMSG_UNLOCK_VOID_STORAGE: u16 = 0x31A2;
const CMSG_QUERY_VOID_STORAGE: u16 = 0x31A3;
const CMSG_VOID_STORAGE_TRANSFER: u16 = 0x31A4;
const CMSG_SWAP_VOID_ITEM: u16 = 0x31A5;
const SMSG_EQUIPMENT_SET_ID: u16 = 0x26B2;
const SMSG_LOAD_EQUIPMENT_SET: u16 = 0x270E;
const SMSG_VOID_STORAGE_FAILED: u16 = 0x2DA0;
const SMSG_VOID_STORAGE_CONTENTS: u16 = 0x2DA1;
const SMSG_VOID_STORAGE_TRANSFER_CHANGES: u16 = 0x2DA2;
const SMSG_VOID_TRANSFER_RESULT: u16 = 0x2DA3;
const SMSG_VOID_ITEM_SWAP_RESPONSE: u16 = 0x2DA4;
const EQUIPMENT_SET_SLOTS_LIKE_CPP: usize = 19;
const MAX_EQUIPMENT_SET_INDEX_LIKE_CPP: u32 = 20;
const EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP: u32 = (1 << EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;
// Keep the mixed signed/unsigned aggregate pinned to an unsigned MySQL wire
// type, matching the world-server startup query for this shared GUID namespace.
const SHARED_EQUIPMENT_SET_GUID_MAX_QUERY: &str = "SELECT CAST(MAX(maxguid) AS UNSIGNED) FROM ((SELECT MAX(setguid) AS maxguid FROM character_equipmentsets) UNION (SELECT MAX(setguid) AS maxguid FROM character_transmog_outfits)) allsets";
// Login can legitimately contain more than 30 packets before
// SMSG_LOGIN_VERIFY_WORLD when another player is already on the map and its
// CREATE/broadcast traffic is interleaved. Keep the guard wall-clock based so
// a busy but healthy login cannot exhaust an arbitrary packet budget.
const LOGIN_VERIFY_TIMEOUT: Duration = Duration::from_secs(30);
const LOGIN_VERIFY_READ_SLICE: Duration = Duration::from_secs(5);
const INITIAL_NETWORK_IO_TIMEOUT: Duration = Duration::from_secs(15);
// C++ WorldSession::ShouldLogOut waits 20 wall-clock seconds after a normal
// logout request (`WorldSession.h`) before it can send SMSG_LOGOUT_COMPLETE.
// Keep a bounded margin for the world update that observes the expired timer.
const NORMAL_LOGOUT_COMPLETE_WAIT_SECS: u64 = 30;
const RESTED_XP_DISCONNECT_SAVE_MAX_WAIT_SECS: u64 = 90;
const INVENTORY_SLOT_BAG_0: u8 = 255;
const INVENTORY_SLOT_ITEM_START: u8 = 35;
const BANK_SLOT_ITEM_START: u8 = 59;
const BANK_SLOT_ITEM_END: u8 = 87;
const NPC_FLAG_BANKER: u32 = 0x20000;
const NPC_FLAG_INNKEEPER: u32 = 0x10000;
const NPC_FLAG_VAULT_KEEPER: u32 = 0x2000_0000;
const DEFAULT_BANK_SMOKE_ITEM_ENTRY: u32 = 2589;
const DEFAULT_VOID_STORAGE_SMOKE_ITEM_ENTRY: u32 = 2589;
const PLAYER_FLAGS_VOID_UNLOCKED: u32 = 0x2000_0000;
const VOID_STORAGE_UNLOCK_COST: u64 = 1_000_000;
const VOID_STORAGE_STORE_ITEM_COST: u64 = 100_000;
const DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A: u32 = 2589;
const DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_B: u32 = 2592;
const DEFAULT_VENDOR_ENTRY: u32 = 18_525;
const DEFAULT_VENDOR_SPAWN_GUID: u64 = 96_654;
const DEFAULT_VENDOR_ITEM_ENTRY: u32 = 30_183;
const DEFAULT_VENDOR_EXTENDED_COST: u32 = 1_642;
const DEFAULT_VENDOR_CURRENCY_ID: u32 = 42;
const DEFAULT_VENDOR_CURRENCY_COST: u32 = 15;
const DEFAULT_VENDOR_CURRENCY_QUANTITY: u32 = 30;
const VENDOR_CAPTURE_FENCE_SERIAL: u32 = 0x5645_4E44;
const DEFAULT_RESTED_XP_CREATURE_ENTRY: u32 = 15274;
const DEFAULT_RESTED_XP_OFFLINE_SECS: u64 = 86_400;
// A fresh level-1 Mana Wyrm has 42 HP. The intentionally empty disposable
// fixture attacks for 1 damage roughly every two seconds, so 45 seconds could
// never complete a legitimate unarmed kill.
const DEFAULT_RESTED_XP_TIMEOUT_SECS: u64 = 120;
const MIN_RESTED_XP_TARGET_RESPAWN_SECS: u32 = 30;
const MAX_RESTED_XP_TARGET_RESPAWN_SECS: u32 = 600;
const RESTED_XP_RESPAWN_GRACE_SECS: u64 = 15;
const MAX_RESTED_XP_RESPAWN_CLEANUP_WAIT_SECS: u64 = 900;
const ACK_DISPOSABLE_RESTED_XP_FLAG: &str = "--ack-disposable-rested-xp";
const REST_STATE_RESTED: u8 = 1;
const REST_STATE_NORMAL: u8 = 2;
const PLAYER_FLAGS_RESTING: u32 = 0x0000_0020;
const PLAYER_FLAGS_NO_XP_GAIN: u32 = 0x0200_0000;
const CREATURE_STATIC_FLAG_NO_XP: u32 = 0x0000_0002;
const CREATURE_FLAG_EXTRA_NO_XP: u32 = 0x0000_0040;
const CREATURE_TYPE_CRITTER: u8 = 8;
const OBJECT_GUID_COUNTER_MASK: u64 = 0xFF_FFFF_FFFF;
const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;
const RESTED_XP_INSTANCE_OBSERVATION_WINDOW: Duration = Duration::from_secs(2);
// Both legacy C++ implementations use `NextLevelXP * 1.5f / 2`.
const REST_BONUS_CAP_NEXT_LEVEL_FACTOR: f32 = 1.5 / 2.0;
const REST_OFFLINE_WILDERNESS_BUBBLE: f32 = 0.031;
const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE: f32 = 0.125;
const UNIT_STAND_STATE_STAND: u8 = 0;
const UNIT_STAND_STATE_SIT: u8 = 1;
const UNIT_STAND_STATE_SLEEP: u8 = 3;
const UNIT_STAND_STATE_KNEEL: u8 = 8;
const STAND_STATE_LOGIN_QUIET_PERIOD: Duration = Duration::from_millis(500);
const STAND_STATE_LOGIN_DRAIN_LIMIT: Duration = Duration::from_secs(5);
const STAND_STATE_POST_ACTION_QUIET_PERIOD: Duration = Duration::from_millis(250);
const STAND_STATE_POST_ACTION_DRAIN_LIMIT: Duration = Duration::from_secs(5);
const STAND_STATE_CAPTURE_FENCE_SERIAL: u32 = 0x5354_414E;

#[derive(Debug)]
struct ServerPacketInflater {
    decompressor: Decompress,
}

#[derive(Debug, Clone, Copy)]
struct LoginVerifyBudget {
    deadline: tokio::time::Instant,
}

impl LoginVerifyBudget {
    fn new(timeout: Duration) -> Self {
        Self {
            deadline: tokio::time::Instant::now() + timeout,
        }
    }

    fn next_read_timeout(self) -> Option<Duration> {
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

// Server endpoints (overridable via env)
fn bnet_host() -> String {
    std::env::var("BNET_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}
fn bnet_port() -> u16 {
    std::env::var("BNET_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8081)
}
fn world_host() -> String {
    std::env::var("WORLD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}
fn world_port() -> u16 {
    std::env::var("WORLD_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8085)
}
fn realm_id() -> u32 {
    std::env::var("REALM_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}
fn client_build() -> u32 {
    std::env::var("WOW_BOT_CLIENT_BUILD")
        .or_else(|_| std::env::var("WOW_BOT_BUILD"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(54261)
}
fn auth_db_url() -> Result<String> {
    database_url("WOW_BOT_AUTH_DB_URL", "LoginDatabaseInfo")
}
fn characters_db_url() -> Result<String> {
    database_url("WOW_BOT_CHAR_DB_URL", "CharacterDatabaseInfo")
}
fn world_db_url() -> Result<String> {
    database_url("WOW_BOT_WORLD_DB_URL", "WorldDatabaseInfo")
}

const DEFAULT_DUNGEON_ID: u32 = 259;
const CONTINUED_SESSION_SEED: [u8; 16] = [
    0x16, 0xAD, 0x0C, 0xD4, 0x46, 0xF9, 0x4F, 0xB2, 0xEF, 0x7D, 0xEA, 0x2A, 0x17, 0x66, 0x4D, 0x2F,
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct QuestObjectiveDbRow {
    objective: u8,
    data: i32,
}

#[derive(Debug, Clone)]
struct CliOptions {
    config_path: String,
    dungeon_id: Option<u32>,
    timeout_secs: Option<u64>,
    single_account: Option<String>,
    sequential: bool,
    auto_teleport: Option<bool>,
    cleanup_groups: Option<bool>,
    require_group: bool,
    ensure_test_accounts: bool,
    login_only: bool,
    stand_state_smoke: bool,
    stand_state: Option<u8>,
    stand_state_timeout_secs: u64,
    bank_smoke: bool,
    bank_item_entry: u32,
    bank_runtime_counter: Option<u64>,
    bank_timeout_secs: u64,
    void_storage_smoke: bool,
    void_storage_query_capture: bool,
    void_storage_item_entry: u32,
    void_storage_runtime_counter: Option<u64>,
    void_storage_timeout_secs: u64,
    homebind_smoke: bool,
    homebind_runtime_counter: Option<u64>,
    homebind_timeout_secs: u64,
    inventory_swap_smoke: bool,
    inventory_swap_item_entry_a: u32,
    inventory_swap_item_entry_b: u32,
    inventory_swap_timeout_secs: u64,
    vendor_smoke: bool,
    vendor_entry: u32,
    vendor_spawn_guid: u64,
    vendor_runtime_counter: Option<u64>,
    vendor_item_entry: u32,
    vendor_extended_cost: u32,
    vendor_currency_id: u32,
    vendor_currency_cost: u32,
    vendor_currency_quantity: u32,
    vendor_timeout_secs: u64,
    equipment_set_race_smoke: bool,
    equipment_set_account_a: String,
    equipment_set_account_b: String,
    equipment_set_timeout_secs: u64,
    rested_xp_smoke: bool,
    ack_disposable_rested_xp: bool,
    rested_xp_creature_entry: u32,
    rested_xp_creature_guid: Option<u64>,
    rested_xp_runtime_counter: Option<u64>,
    rested_xp_offline_secs: u64,
    rested_xp_timeout_secs: u64,
    loot_race_smoke: bool,
    loot_item_capture: bool,
    ack_disposable_overworld_loot_race: bool,
    loot_race_account_a: String,
    loot_race_account_b: String,
    loot_race_creature_entry: u32,
    loot_race_creature_spawn_guid: u64,
    loot_race_runtime_counter: u64,
    loot_race_item_entry: u32,
    loot_race_timeout_secs: u64,
    loot_workflow_deadline_secs: u64,
    recover_loot_fixture: bool,
    group_capacity_race_smoke: bool,
    group_capacity_leader_account: String,
    group_capacity_candidate_a_account: String,
    group_capacity_candidate_b_account: String,
    group_capacity_group_id: u32,
    group_capacity_timeout_secs: u64,
    quest_smoke: bool,
    quest_creature_entry: Option<u32>,
    quest_creature_guid: Option<u64>,
    quest_guid_counter: Option<u64>,
    quest_map_id: Option<u16>,
    quest_expected_id: Option<u32>,
    quest_forbidden_id: Option<u32>,
    quest_forbidden_title: Option<String>,
    quest_query_details: bool,
    quest_accept: bool,
    quest_reset: bool,
    quest_relocate: bool,
    quest_set_level: Option<u8>,
    quest_set_race: Option<u8>,
    quest_set_class: Option<u8>,
    quest_objective_persist: bool,
    quest_objectives: Vec<QuestObjectiveDbRow>,
    quest_objective_status: u8,
    gossip_select_option_id: Option<i32>,
    expect_trainer_list: bool,
    expect_trainer_id: Option<i32>,
    quest_timeout_secs: u64,
    report_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(Default))]
struct BotRunResult {
    account: String,
    account_id: u32,
    character_guid: u64,
    dungeon_id: u32,
    role: u8,
    join_result: Option<u8>,
    join_detail: Option<u8>,
    got_proposal: bool,
    accepted_proposal: bool,
    got_ready_check: bool,
    group_formed: bool,
    teleport_denied_reason: Option<u8>,
    entered_world: bool,
    world_auth: bool,
    enum_characters: bool,
    player_login_verified: bool,
    login_only: bool,
    stand_state_smoke: bool,
    stand_state_smoke_passed: Option<bool>,
    stand_states_requested: Vec<u8>,
    stand_states_confirmed: Vec<u8>,
    stand_state_failure: Option<String>,
    bank_smoke: bool,
    bank_smoke_passed: Option<bool>,
    bank_banker_entry: Option<u32>,
    bank_banker_spawn_guid: Option<u64>,
    bank_banker_guid_counter: Option<u64>,
    bank_item_guid: Option<u64>,
    bank_item_entry: Option<u32>,
    bank_inventory_slot: Option<u8>,
    bank_bank_slot: Option<u8>,
    bank_open_confirmed: bool,
    bank_deposit_persisted: bool,
    bank_relogin_after_deposit: bool,
    bank_withdraw_persisted: bool,
    bank_failure: Option<String>,
    void_storage_smoke: bool,
    void_storage_smoke_passed: Option<bool>,
    void_storage_query_capture: bool,
    void_storage_query_capture_passed: Option<bool>,
    void_storage_unlock_persisted: bool,
    void_storage_deposit_persisted: bool,
    void_storage_deposit_relogin_verified: bool,
    void_storage_swap_persisted: bool,
    void_storage_swap_relogin_verified: bool,
    void_storage_withdraw_persisted: bool,
    void_storage_withdraw_relogin_verified: bool,
    void_storage_item_id: Option<u64>,
    void_storage_failure: Option<String>,
    homebind_smoke: bool,
    homebind_smoke_passed: Option<bool>,
    homebind_innkeeper_entry: Option<u32>,
    homebind_innkeeper_spawn_guid: Option<u64>,
    homebind_innkeeper_guid_counter: Option<u64>,
    homebind_spell_go_seen: bool,
    homebind_bind_point_update_seen: bool,
    homebind_player_bound_seen: bool,
    homebind_gossip_complete_seen: bool,
    homebind_db_persisted: bool,
    homebind_relogin_verified: bool,
    homebind_failure: Option<String>,
    inventory_swap_smoke: bool,
    inventory_swap_smoke_passed: Option<bool>,
    inventory_swap_item_guid_a: Option<u64>,
    inventory_swap_item_guid_b: Option<u64>,
    inventory_swap_item_entry_a: Option<u32>,
    inventory_swap_item_entry_b: Option<u32>,
    inventory_swap_slot_a: Option<u8>,
    inventory_swap_slot_b: Option<u8>,
    inventory_swap_forward_persisted: bool,
    inventory_swap_relogin_after_forward: bool,
    inventory_swap_reverse_persisted: bool,
    inventory_swap_failure: Option<String>,
    vendor_smoke: bool,
    vendor_smoke_passed: Option<bool>,
    vendor_entry: Option<u32>,
    vendor_spawn_guid: Option<u64>,
    vendor_runtime_counter: Option<u64>,
    vendor_item_entry: Option<u32>,
    vendor_extended_cost: Option<u32>,
    vendor_currency_id: Option<u32>,
    vendor_currency_before: Option<u32>,
    vendor_currency_after: Option<u32>,
    vendor_item_total_after: Option<u64>,
    vendor_inventory_seen: bool,
    vendor_buy_succeeded_seen: bool,
    vendor_set_currency_seen: bool,
    vendor_item_push_seen: bool,
    vendor_relogin_verified: bool,
    vendor_failure: Option<String>,
    equipment_set_smoke: bool,
    equipment_set_smoke_passed: Option<bool>,
    equipment_set_type: Option<i32>,
    equipment_set_id: Option<u32>,
    equipment_set_generated_guid: Option<u64>,
    equipment_set_login_count: Option<u32>,
    equipment_set_load_seen: bool,
    equipment_set_db_persisted: bool,
    equipment_set_relogin_verified: bool,
    equipment_set_failure: Option<String>,
    rested_xp_smoke: bool,
    rested_xp_smoke_passed: Option<bool>,
    rested_xp_offline_wilderness_bonus: Option<f32>,
    rested_xp_offline_resting_bonus: Option<f32>,
    rested_xp_target_entry: Option<u32>,
    rested_xp_target_spawn_guid: Option<u64>,
    rested_xp_target_guid_counter: Option<u64>,
    rested_xp_packet_amount: Option<i32>,
    rested_xp_packet_original: Option<i32>,
    rested_xp_db_xp_before: Option<u32>,
    rested_xp_db_xp_after: Option<u32>,
    rested_xp_db_rest_before: Option<f32>,
    rested_xp_db_rest_after: Option<f32>,
    rested_xp_relog_verified: bool,
    rested_xp_failure: Option<String>,
    loot_race_smoke: bool,
    loot_race_smoke_passed: Option<bool>,
    loot_race_target_entry: Option<u32>,
    loot_race_target_spawn_guid: Option<u64>,
    loot_race_target_runtime_counter: Option<u64>,
    loot_race_party_confirmed: bool,
    loot_race_target_discovered: bool,
    loot_race_loot_opened: bool,
    loot_race_loot_list_id: Option<u8>,
    loot_race_loot_coins: Option<u32>,
    loot_race_item_push_seen: bool,
    loot_race_loot_removed_seen: bool,
    loot_race_money_notify_amount: Option<u64>,
    loot_race_coin_removed_seen: bool,
    loot_race_db_item_total: Option<u64>,
    loot_race_db_money_delta: Option<u64>,
    loot_race_relog_verified: bool,
    loot_race_failure: Option<String>,
    group_capacity_race_smoke: bool,
    group_capacity_race_smoke_passed: Option<bool>,
    group_capacity_group_id: Option<u32>,
    group_capacity_outcome: Option<String>,
    group_capacity_final_member_count: Option<u64>,
    group_capacity_failure: Option<String>,
    quest_smoke: bool,
    quest_smoke_passed: Option<bool>,
    quest_target_entry: Option<u32>,
    quest_target_spawn_guid: Option<u64>,
    quest_target_guid_counter: Option<u64>,
    quest_target_map_id: Option<u16>,
    quest_gossip_hello_sent: bool,
    quest_questgiver_hello_sent: bool,
    quest_gossip_id_seen: Option<i32>,
    quest_gossip_select_sent: bool,
    quest_gossip_message_seen: bool,
    quest_quest_list_seen: bool,
    quest_details_seen: bool,
    quest_request_items_seen: bool,
    trainer_list_seen: bool,
    trainer_id_seen: Option<i32>,
    trainer_spell_count_seen: Option<u32>,
    quest_accept_sent: bool,
    quest_accept_confirm_seen: bool,
    quest_db_verified: bool,
    quest_db_status: Option<u8>,
    quest_objective_persist: bool,
    quest_objective_seeded: Vec<QuestObjectiveDbRow>,
    quest_objective_db_before: Vec<QuestObjectiveDbRow>,
    quest_objective_db_after: Vec<QuestObjectiveDbRow>,
    quest_objective_db_verified: bool,
    quest_objective_update_seen: bool,
    quest_objective_update_has_expected: bool,
    quest_ids_seen: Vec<u32>,
    quest_titles_seen: Vec<String>,
    quest_failure: Option<String>,
    seen_opcodes: Vec<String>,
}

impl BotRunResult {
    fn success(&self, require_proposal: bool, require_group: bool, login_only: bool) -> bool {
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
                && self.inventory_swap_smoke_passed.unwrap_or(false);
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
struct RunReport {
    dungeon_id: u32,
    timeout_secs: u64,
    require_proposal: bool,
    require_group: bool,
    auto_teleport: bool,
    login_only: bool,
    stand_state_smoke: bool,
    bank_smoke: bool,
    void_storage_smoke: bool,
    void_storage_query_capture: bool,
    homebind_smoke: bool,
    inventory_swap_smoke: bool,
    vendor_smoke: bool,
    equipment_set_race_smoke: bool,
    rested_xp_smoke: bool,
    loot_race_smoke: bool,
    loot_item_capture: bool,
    group_capacity_race_smoke: bool,
    quest_smoke: bool,
    results: Vec<BotRunResult>,
}

#[derive(Debug, Clone)]
struct ConnectToTarget {
    address: IpAddr,
    port: u16,
    serial: u32,
    connection_type: u8,
    key: i64,
}

struct EncryptedWorldConnection {
    stream: TcpStream,
    crypt: WorldCrypt,
    inflater: ServerPacketInflater,
}

#[derive(Debug, Clone)]
struct WorldAuthDbContext {
    username: String,
    realm_build: u32,
    win64_auth_seed: [u8; 16],
}

#[derive(Debug, Clone)]
struct QuestSmokeOptions {
    creature_entry: u32,
    creature_spawn_guid: Option<u64>,
    creature_guid_counter: Option<u64>,
    map_id: Option<u16>,
    expected_quest_id: Option<u32>,
    forbidden_quest_id: Option<u32>,
    forbidden_title_contains: Option<String>,
    query_details: bool,
    accept: bool,
    reset_before_run: bool,
    relocate_before_login: bool,
    set_level_before_login: Option<u8>,
    set_race_before_login: Option<u8>,
    set_class_before_login: Option<u8>,
    objective_persist: bool,
    objective_seed: Vec<QuestObjectiveDbRow>,
    objective_status: u8,
    gossip_select_option_id: Option<i32>,
    expect_trainer_list: bool,
    expect_trainer_id: Option<i32>,
    timeout_secs: u64,
}

#[derive(Debug, Clone)]
struct StandStateSmokeOptions {
    states: Vec<u8>,
    timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BankSmokePhase {
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoidStorageSmokePhase {
    UnlockDeposit,
    VerifyDepositSwap,
    VerifySwapWithdraw,
    VerifyWithdraw,
    QueryCapture,
}

#[derive(Debug, Clone)]
struct VoidStorageSmokeOptions {
    phase: VoidStorageSmokePhase,
    vault_keeper: ResolvedCreatureTarget,
    runtime_realm_id: u16,
    discover_runtime_guid: bool,
    fixture_item_guid: u64,
    item_entry: u32,
    inventory_slot: u8,
    expected_void_item_id: Option<u64>,
    expected_void_slot: u8,
    timeout_secs: u64,
}

#[derive(Debug, Clone)]
struct VoidStorageSmokeFixture {
    options: VoidStorageSmokeOptions,
    original_position: CharacterPositionSnapshot,
    original_money: u64,
    original_player_flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VoidStorageItemWire {
    item_id: u64,
    slot: u32,
    item_entry: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VoidStorageDbState {
    money: u64,
    player_flags: u32,
    void_items: Vec<(u64, u32, u8)>,
    inventory_items: Vec<(u64, u8, u32)>,
}

#[derive(Debug, Clone)]
struct BankSmokeOptions {
    phase: BankSmokePhase,
    banker: ResolvedCreatureTarget,
    item_guid: u64,
    item_entry: u32,
    inventory_slot: u8,
    bank_slot: u8,
    timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize)]
struct CharacterPositionSnapshot {
    map_id: u32,
    zone_id: u32,
    instance_id: u32,
    x: f64,
    y: f64,
    z: f64,
    orientation: f32,
}

#[derive(Debug, Clone)]
struct BankSmokeFixture {
    options: BankSmokeOptions,
    original_position: CharacterPositionSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomebindSmokePhase {
    Bind,
    VerifyRelog,
}

#[derive(Debug, Clone)]
struct HomebindSmokeOptions {
    phase: HomebindSmokePhase,
    innkeeper: ResolvedCreatureTarget,
    discover_runtime_guid: bool,
    expected_homebind: Option<HomebindRowSnapshot>,
    timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct HomebindRowSnapshot {
    map_id: u16,
    zone_id: u16,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
}

#[derive(Debug, Clone)]
struct HomebindSmokeFixture {
    options: HomebindSmokeOptions,
    original_position: CharacterPositionSnapshot,
    original_homebind: Option<HomebindRowSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventorySwapSmokePhase {
    Forward,
    Reverse,
}

#[derive(Debug, Clone)]
struct InventorySwapSmokeOptions {
    phase: InventorySwapSmokePhase,
    item_guid_a: u64,
    item_guid_b: u64,
    item_entry_a: u32,
    item_entry_b: u32,
    slot_a: u8,
    slot_b: u8,
    timeout_secs: u64,
}

#[derive(Debug, Clone)]
struct InventorySwapSmokeFixture {
    options: InventorySwapSmokeOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VendorSmokePhase {
    Purchase,
    VerifyRelog,
}

#[derive(Debug, Clone)]
struct VendorSmokeOptions {
    phase: VendorSmokePhase,
    vendor: ResolvedCreatureTarget,
    target_match_radius: f32,
    item_entry: u32,
    extended_cost: u32,
    currency_id: u32,
    currency_before: u32,
    currency_cost: u32,
    expected_item_total: u64,
    timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VendorCurrencyRowSnapshot {
    quantity: u32,
    weekly_quantity: u32,
    tracked_quantity: u32,
    increased_cap_quantity: u32,
    earned_quantity: u32,
    flags: u8,
}

#[derive(Debug, Clone)]
struct VendorSmokeFixture {
    options: VendorSmokeOptions,
    original_position: CharacterPositionSnapshot,
    original_currency: Option<VendorCurrencyRowSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EquipmentSetSmokePhase {
    Save,
    VerifyRelog,
}

#[derive(Debug, Clone)]
struct EquipmentSetSmokeOptions {
    phase: EquipmentSetSmokePhase,
    set_type: i32,
    set_id: u32,
    set_name: String,
    set_icon: String,
    expected_guid: Option<u64>,
    save_barrier: Option<std::sync::Arc<tokio::sync::Barrier>>,
    timeout_secs: u64,
}

#[derive(Debug, Clone)]
struct EquipmentSetSmokeFixture {
    initial_max_guid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EquipmentSetWire {
    set_type: i32,
    guid: u64,
    set_id: u32,
    ignore_mask: u32,
    pieces: [[u8; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    enchants: [i32; 2],
    secondary_appearances_and_slots: [i32; 4],
    assigned_spec_index: i32,
    set_name: String,
    set_icon: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EquipmentSetDbRow {
    set_guid: u64,
    set_index: u32,
    name: String,
    icon_name: String,
    ignore_mask: u32,
    assigned_spec_index: i32,
    items: [u64; EQUIPMENT_SET_SLOTS_LIKE_CPP],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransmogOutfitDbRow {
    set_guid: u64,
    set_index: u32,
    name: String,
    icon_name: String,
    ignore_mask: u32,
    appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    main_hand_enchant: i32,
    off_hand_enchant: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VendorInventoryItemWire {
    muid: i32,
    item_id: i32,
    item_type: i32,
    price: u64,
    stack_count: i32,
    extended_cost: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestedXpSmokePhase {
    OfflineWilderness,
    OfflineResting,
    ConsumeKill,
    VerifyRelog,
}

#[derive(Debug, Clone)]
struct RestedXpSmokeOptions {
    phase: RestedXpSmokePhase,
    target: ResolvedCreatureTarget,
    target_match_radius: f32,
    test_level: u8,
    next_level_xp: u32,
    seeded_rest_bonus: f32,
    expected_xp: Option<u32>,
    expected_rest_bonus: Option<f32>,
    timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct RestedXpCharacterRestorePoint {
    level: u8,
    xp: u32,
    rest_state: u8,
    player_flags: u32,
    rest_bonus: f32,
    logout_time: u64,
    is_logout_resting: u8,
    map_id: u32,
    zone_id: u32,
    instance_id: u32,
    x: f64,
    y: f64,
    z: f64,
    orientation: f32,
    health: u32,
    powers: [u32; 10],
    total_kills: u32,
    today_kills: u16,
    yesterday_kills: u16,
    total_time: u32,
    level_time: u32,
    latency: u32,
    last_login_build: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RestedXpDbState {
    level: u8,
    xp: u32,
    rest_state: u8,
    rest_bonus: f32,
    online: u8,
}

#[derive(Debug, Clone)]
struct RestedXpSmokeFixture {
    options: RestedXpSmokeOptions,
    original: RestedXpCharacterRestorePoint,
    original_achievements: Vec<(u32, i64)>,
    original_achievement_progress: Vec<(u32, u64, i64)>,
    original_trait_configs: Vec<RestedXpTraitConfigSnapshot>,
    original_trait_entries: Vec<RestedXpTraitEntrySnapshot>,
    original_homebind: Option<RestedXpHomebindSnapshot>,
    original_fishing_steps: Option<u8>,
    original_battleground_data: Option<RestedXpBattlegroundDataSnapshot>,
    original_last_played_characters: Vec<RestedXpLastPlayedCharacterSnapshot>,
    original_battle_pet_slots: Vec<RestedXpBattlePetSlotSnapshot>,
    battlenet_account_id: u32,
    target_respawn_secs: u32,
    test_level: u8,
    offline_secs: u64,
    wilderness_rate: f32,
    resting_rate: f32,
}

type RestedXpTraitConfigSnapshot = (
    i32,
    i32,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    String,
);
type RestedXpTraitEntrySnapshot = (i32, i32, i32, i32, i32);
type RestedXpHomebindSnapshot = (u16, u16, f32, f32, f32, f32);
type RestedXpBattlegroundDataSnapshot = (
    u32,
    u16,
    f32,
    f32,
    f32,
    f32,
    u16,
    u32,
    u32,
    u32,
    Option<u64>,
);
type RestedXpLastPlayedCharacterSnapshot = (
    u8,
    u8,
    Option<u32>,
    Option<String>,
    Option<u64>,
    Option<u32>,
);
type RestedXpBattlePetSlotSnapshot = (i8, i64, i8);

#[derive(Debug, Default, PartialEq, Eq)]
struct RestedXpFixtureSafetyState {
    at_login: u16,
    game_account_online: u8,
    bnet_email_matches_configured_account: bool,
    characters_on_game_account: u64,
    game_accounts_on_bnet_account: u64,
    nonempty_side_state: Vec<(String, u64)>,
}

#[derive(Debug, Clone)]
struct ResolvedCreatureTarget {
    entry: u32,
    spawn_guid: u64,
    guid_counter: u64,
    map_id: u16,
    x: f64,
    y: f64,
    z: f64,
    orientation: f32,
    packed_guid: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DiscoveredCreatureGuid {
    low: u64,
    high: u64,
    x: f32,
    y: f32,
    z: f32,
}

fn void_storage_login_target_ready(discover_runtime_guid: bool, target_seen: bool) -> bool {
    !discover_runtime_guid || target_seen
}

fn test_dungeon_id(app_config: &config::AppConfig) -> u32 {
    // Allow override via env var (handy for ad-hoc testing); otherwise pick up
    // the value from config.json::test_config.dungeon_id.
    std::env::var("WOW_BOT_DUNGEON_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            if app_config.test_config.dungeon_id != 0 {
                app_config.test_config.dungeon_id
            } else {
                DEFAULT_DUNGEON_ID
            }
        })
}

fn parse_cli() -> Result<CliOptions> {
    let stand_state = std::env::var("WOW_BOT_STAND_STATE")
        .ok()
        .map(|value| value.parse::<u8>())
        .transpose()?;
    let stand_state_smoke = std::env::var("WOW_BOT_STAND_STATE_SMOKE")
        .ok()
        .is_some_and(|value| is_truthy(&value))
        || stand_state.is_some();

    let mut opts = CliOptions {
        config_path: std::env::var("WOW_BOT_CONFIG").unwrap_or_else(|_| "config.json".to_string()),
        dungeon_id: std::env::var("WOW_BOT_DUNGEON_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        timeout_secs: std::env::var("WOW_BOT_LFG_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok()),
        single_account: None,
        sequential: false,
        auto_teleport: std::env::var("WOW_BOT_AUTO_TELEPORT")
            .ok()
            .map(|v| is_truthy(&v)),
        cleanup_groups: std::env::var("WOW_BOT_CLEANUP_GROUPS")
            .ok()
            .map(|v| is_truthy(&v)),
        require_group: std::env::var("WOW_BOT_REQUIRE_GROUP")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        ensure_test_accounts: std::env::var("WOW_BOT_ENSURE_TEST_ACCOUNTS")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        login_only: std::env::var("WOW_BOT_LOGIN_ONLY")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        stand_state_smoke,
        stand_state,
        stand_state_timeout_secs: std::env::var("WOW_BOT_STAND_STATE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(5),
        bank_smoke: std::env::var("WOW_BOT_BANK_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        bank_item_entry: std::env::var("WOW_BOT_BANK_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_BANK_SMOKE_ITEM_ENTRY),
        bank_runtime_counter: std::env::var("WOW_BOT_BANK_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        bank_timeout_secs: std::env::var("WOW_BOT_BANK_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        void_storage_smoke: std::env::var("WOW_BOT_VOID_STORAGE_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        void_storage_query_capture: std::env::var("WOW_BOT_VOID_STORAGE_QUERY_CAPTURE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        void_storage_item_entry: std::env::var("WOW_BOT_VOID_STORAGE_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VOID_STORAGE_SMOKE_ITEM_ENTRY),
        void_storage_runtime_counter: std::env::var("WOW_BOT_VOID_STORAGE_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        void_storage_timeout_secs: std::env::var("WOW_BOT_VOID_STORAGE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        homebind_smoke: std::env::var("WOW_BOT_HOMEBIND_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        homebind_runtime_counter: std::env::var("WOW_BOT_HOMEBIND_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        homebind_timeout_secs: std::env::var("WOW_BOT_HOMEBIND_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        inventory_swap_smoke: std::env::var("WOW_BOT_INVENTORY_SWAP_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        inventory_swap_item_entry_a: std::env::var("WOW_BOT_INVENTORY_SWAP_ITEM_ENTRY_A")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_A),
        inventory_swap_item_entry_b: std::env::var("WOW_BOT_INVENTORY_SWAP_ITEM_ENTRY_B")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_INVENTORY_SWAP_ITEM_ENTRY_B),
        inventory_swap_timeout_secs: std::env::var("WOW_BOT_INVENTORY_SWAP_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        vendor_smoke: std::env::var("WOW_BOT_VENDOR_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        vendor_entry: std::env::var("WOW_BOT_VENDOR_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_ENTRY),
        vendor_spawn_guid: std::env::var("WOW_BOT_VENDOR_SPAWN_GUID")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_SPAWN_GUID),
        vendor_runtime_counter: std::env::var("WOW_BOT_VENDOR_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        vendor_item_entry: std::env::var("WOW_BOT_VENDOR_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_ITEM_ENTRY),
        vendor_extended_cost: std::env::var("WOW_BOT_VENDOR_EXTENDED_COST")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_EXTENDED_COST),
        vendor_currency_id: std::env::var("WOW_BOT_VENDOR_CURRENCY_ID")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_CURRENCY_ID),
        vendor_currency_cost: std::env::var("WOW_BOT_VENDOR_CURRENCY_COST")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_CURRENCY_COST),
        vendor_currency_quantity: std::env::var("WOW_BOT_VENDOR_CURRENCY_QUANTITY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_VENDOR_CURRENCY_QUANTITY),
        vendor_timeout_secs: std::env::var("WOW_BOT_VENDOR_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(8),
        equipment_set_race_smoke: std::env::var("WOW_BOT_EQUIPMENT_SET_RACE_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        equipment_set_account_a: std::env::var("WOW_BOT_EQUIPMENT_SET_ACCOUNT_A")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_A.to_string()),
        equipment_set_account_b: std::env::var("WOW_BOT_EQUIPMENT_SET_ACCOUNT_B")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_B.to_string()),
        equipment_set_timeout_secs: std::env::var("WOW_BOT_EQUIPMENT_SET_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(10),
        rested_xp_smoke: std::env::var("WOW_BOT_RESTED_XP_SMOKE")
            .ok()
            .map(|value| is_truthy(&value))
            .unwrap_or(false),
        // Deliberately CLI-only: an inherited environment must never acknowledge
        // destructive use of a disposable fixture implicitly.
        ack_disposable_rested_xp: false,
        rested_xp_creature_entry: std::env::var("WOW_BOT_RESTED_XP_CREATURE_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(DEFAULT_RESTED_XP_CREATURE_ENTRY),
        rested_xp_creature_guid: std::env::var("WOW_BOT_RESTED_XP_CREATURE_GUID")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        rested_xp_runtime_counter: std::env::var("WOW_BOT_RESTED_XP_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?,
        rested_xp_offline_secs: std::env::var("WOW_BOT_RESTED_XP_OFFLINE_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_RESTED_XP_OFFLINE_SECS),
        rested_xp_timeout_secs: std::env::var("WOW_BOT_RESTED_XP_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_RESTED_XP_TIMEOUT_SECS),
        loot_race_smoke: std::env::var("WOW_BOT_LOOT_RACE_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        loot_item_capture: std::env::var("WOW_BOT_LOOT_ITEM_CAPTURE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        // Deliberately CLI-only: inherited environment cannot acknowledge
        // disposable loot-fixture mutation on the caller's behalf.
        ack_disposable_overworld_loot_race: false,
        loot_race_account_a: std::env::var("WOW_BOT_LOOT_RACE_ACCOUNT_A")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_A.to_string()),
        loot_race_account_b: std::env::var("WOW_BOT_LOOT_RACE_ACCOUNT_B")
            .unwrap_or_else(|_| loot_race::DEFAULT_ACCOUNT_B.to_string()),
        loot_race_creature_entry: std::env::var("WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY")
            .or_else(|_| std::env::var("WOW_BOT_LOOT_RACE_CREATURE_ENTRY"))
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_CREATURE_ENTRY),
        loot_race_creature_spawn_guid: std::env::var("WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID")
            .or_else(|_| std::env::var("WOW_BOT_LOOT_RACE_CREATURE_SPAWN_GUID"))
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_CREATURE_SPAWN_GUID),
        loot_race_runtime_counter: std::env::var("WOW_BOT_LOOT_RACE_RUNTIME_COUNTER")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_RUNTIME_COUNTER),
        loot_race_item_entry: std::env::var("WOW_BOT_LOOT_RACE_ITEM_ENTRY")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_ITEM_ENTRY),
        loot_race_timeout_secs: std::env::var("WOW_BOT_LOOT_RACE_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_TIMEOUT_SECS),
        loot_workflow_deadline_secs: std::env::var("WOW_BOT_LOOT_WORKFLOW_DEADLINE_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_WORKFLOW_DEADLINE_SECS),
        recover_loot_fixture: false,
        group_capacity_race_smoke: std::env::var("WOW_BOT_GROUP_CAPACITY_RACE_SMOKE")
            .ok()
            .is_some_and(|value| is_truthy(&value)),
        group_capacity_leader_account: std::env::var("WOW_BOT_GROUP_CAPACITY_LEADER")
            .unwrap_or_else(|_| loot_race::DEFAULT_GROUP_CAPACITY_LEADER.to_string()),
        group_capacity_candidate_a_account: std::env::var("WOW_BOT_GROUP_CAPACITY_CANDIDATE_A")
            .unwrap_or_else(|_| loot_race::DEFAULT_GROUP_CAPACITY_CANDIDATE_A.to_string()),
        group_capacity_candidate_b_account: std::env::var("WOW_BOT_GROUP_CAPACITY_CANDIDATE_B")
            .unwrap_or_else(|_| loot_race::DEFAULT_GROUP_CAPACITY_CANDIDATE_B.to_string()),
        group_capacity_group_id: std::env::var("WOW_BOT_GROUP_CAPACITY_GROUP_ID")
            .ok()
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(0),
        group_capacity_timeout_secs: std::env::var("WOW_BOT_GROUP_CAPACITY_TIMEOUT_SECS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(loot_race::DEFAULT_GROUP_CAPACITY_TIMEOUT_SECS),
        quest_smoke: std::env::var("WOW_BOT_QUEST_SMOKE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_creature_entry: std::env::var("WOW_BOT_QUEST_CREATURE_ENTRY")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_creature_guid: std::env::var("WOW_BOT_QUEST_CREATURE_GUID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_guid_counter: std::env::var("WOW_BOT_QUEST_RUNTIME_COUNTER")
            .or_else(|_| std::env::var("WOW_BOT_QUEST_GUID_COUNTER"))
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_map_id: std::env::var("WOW_BOT_QUEST_MAP_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_expected_id: std::env::var("WOW_BOT_QUEST_EXPECT_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_forbidden_id: std::env::var("WOW_BOT_QUEST_FORBID_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_forbidden_title: std::env::var("WOW_BOT_QUEST_FORBID_TITLE_CONTAINS").ok(),
        quest_query_details: std::env::var("WOW_BOT_QUEST_QUERY_DETAILS")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(true),
        quest_accept: std::env::var("WOW_BOT_QUEST_ACCEPT")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_reset: std::env::var("WOW_BOT_QUEST_RESET")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_relocate: std::env::var("WOW_BOT_QUEST_RELOCATE")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_set_level: std::env::var("WOW_BOT_QUEST_SET_LEVEL")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_set_race: std::env::var("WOW_BOT_QUEST_SET_RACE")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_set_class: std::env::var("WOW_BOT_QUEST_SET_CLASS")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_objective_persist: std::env::var("WOW_BOT_QUEST_OBJECTIVE_PERSIST")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        quest_objectives: match std::env::var("WOW_BOT_QUEST_OBJECTIVES") {
            Ok(value) if !value.trim().is_empty() => parse_quest_objective_rows(&value)?,
            _ => Vec::new(),
        },
        quest_objective_status: std::env::var("WOW_BOT_QUEST_OBJECTIVE_STATUS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3),
        gossip_select_option_id: std::env::var("WOW_BOT_GOSSIP_SELECT_OPTION_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        expect_trainer_list: std::env::var("WOW_BOT_EXPECT_TRAINER_LIST")
            .ok()
            .map(|v| is_truthy(&v))
            .unwrap_or(false),
        expect_trainer_id: std::env::var("WOW_BOT_EXPECT_TRAINER_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
        quest_timeout_secs: std::env::var("WOW_BOT_QUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5),
        report_path: std::env::var("WOW_BOT_REPORT").ok(),
    };

    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    if raw_args.len() == 6 && !raw_args[0].starts_with("--") {
        opts.single_account = Some(raw_args[0].clone());
        opts.dungeon_id = Some(raw_args[5].parse()?);
        return Ok(opts);
    }

    let mut args = raw_args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => opts.config_path = next_arg(&mut args, "--config")?,
            "--dungeon" => opts.dungeon_id = Some(next_arg(&mut args, "--dungeon")?.parse()?),
            "--timeout" => opts.timeout_secs = Some(next_arg(&mut args, "--timeout")?.parse()?),
            "--single" => opts.single_account = Some(next_arg(&mut args, "--single")?),
            "--sequential" => opts.sequential = true,
            "--parallel" => opts.sequential = false,
            "--auto-teleport" => opts.auto_teleport = Some(true),
            "--no-auto-teleport" => opts.auto_teleport = Some(false),
            "--cleanup-groups" => opts.cleanup_groups = Some(true),
            "--no-cleanup-groups" => opts.cleanup_groups = Some(false),
            "--require-group" => opts.require_group = true,
            "--ensure-test-accounts" => opts.ensure_test_accounts = true,
            "--login-only" => opts.login_only = true,
            "--stand-state-smoke" => opts.stand_state_smoke = true,
            "--stand-state" => {
                opts.stand_state_smoke = true;
                opts.stand_state = Some(next_arg(&mut args, "--stand-state")?.parse()?);
            }
            "--stand-state-timeout" => {
                opts.stand_state_timeout_secs =
                    next_arg(&mut args, "--stand-state-timeout")?.parse()?;
            }
            "--bank-smoke" => opts.bank_smoke = true,
            "--bank-item-entry" => {
                opts.bank_item_entry = next_arg(&mut args, "--bank-item-entry")?.parse()?;
            }
            "--bank-runtime-counter" => {
                opts.bank_runtime_counter =
                    Some(next_arg(&mut args, "--bank-runtime-counter")?.parse()?);
            }
            "--bank-timeout" => {
                opts.bank_timeout_secs = next_arg(&mut args, "--bank-timeout")?.parse()?;
            }
            "--void-storage-smoke" => opts.void_storage_smoke = true,
            "--void-storage-query-capture" => opts.void_storage_query_capture = true,
            "--void-storage-item-entry" => {
                opts.void_storage_item_entry =
                    next_arg(&mut args, "--void-storage-item-entry")?.parse()?;
            }
            "--void-storage-runtime-counter" => {
                opts.void_storage_runtime_counter =
                    Some(next_arg(&mut args, "--void-storage-runtime-counter")?.parse()?);
            }
            "--void-storage-timeout" => {
                opts.void_storage_timeout_secs =
                    next_arg(&mut args, "--void-storage-timeout")?.parse()?;
            }
            "--homebind-smoke" => opts.homebind_smoke = true,
            "--homebind-runtime-counter" => {
                opts.homebind_runtime_counter =
                    Some(next_arg(&mut args, "--homebind-runtime-counter")?.parse()?);
            }
            "--homebind-timeout" => {
                opts.homebind_timeout_secs = next_arg(&mut args, "--homebind-timeout")?.parse()?;
            }
            "--inventory-swap-smoke" => opts.inventory_swap_smoke = true,
            "--inventory-swap-item-entry-a" => {
                opts.inventory_swap_item_entry_a =
                    next_arg(&mut args, "--inventory-swap-item-entry-a")?.parse()?;
            }
            "--inventory-swap-item-entry-b" => {
                opts.inventory_swap_item_entry_b =
                    next_arg(&mut args, "--inventory-swap-item-entry-b")?.parse()?;
            }
            "--inventory-swap-timeout" => {
                opts.inventory_swap_timeout_secs =
                    next_arg(&mut args, "--inventory-swap-timeout")?.parse()?;
            }
            "--vendor-smoke" => opts.vendor_smoke = true,
            "--vendor-entry" => {
                opts.vendor_entry = next_arg(&mut args, "--vendor-entry")?.parse()?;
            }
            "--vendor-spawn-guid" => {
                opts.vendor_spawn_guid = next_arg(&mut args, "--vendor-spawn-guid")?.parse()?;
            }
            "--vendor-runtime-counter" => {
                opts.vendor_runtime_counter =
                    Some(next_arg(&mut args, "--vendor-runtime-counter")?.parse()?);
            }
            "--vendor-item-entry" => {
                opts.vendor_item_entry = next_arg(&mut args, "--vendor-item-entry")?.parse()?;
            }
            "--vendor-extended-cost" => {
                opts.vendor_extended_cost =
                    next_arg(&mut args, "--vendor-extended-cost")?.parse()?;
            }
            "--vendor-currency-id" => {
                opts.vendor_currency_id = next_arg(&mut args, "--vendor-currency-id")?.parse()?;
            }
            "--vendor-currency-cost" => {
                opts.vendor_currency_cost =
                    next_arg(&mut args, "--vendor-currency-cost")?.parse()?;
            }
            "--vendor-currency-quantity" => {
                opts.vendor_currency_quantity =
                    next_arg(&mut args, "--vendor-currency-quantity")?.parse()?;
            }
            "--vendor-timeout" => {
                opts.vendor_timeout_secs = next_arg(&mut args, "--vendor-timeout")?.parse()?;
            }
            "--equipment-set-race-smoke" => opts.equipment_set_race_smoke = true,
            "--equipment-set-account-a" => {
                opts.equipment_set_account_a = next_arg(&mut args, "--equipment-set-account-a")?;
            }
            "--equipment-set-account-b" => {
                opts.equipment_set_account_b = next_arg(&mut args, "--equipment-set-account-b")?;
            }
            "--equipment-set-timeout" => {
                opts.equipment_set_timeout_secs =
                    next_arg(&mut args, "--equipment-set-timeout")?.parse()?;
            }
            "--rested-xp-smoke" => opts.rested_xp_smoke = true,
            arg if parse_ack_disposable_rested_xp_arg(arg, &mut opts.ack_disposable_rested_xp) => {}
            "--rested-xp-creature-entry" => {
                opts.rested_xp_creature_entry =
                    next_arg(&mut args, "--rested-xp-creature-entry")?.parse()?;
            }
            "--rested-xp-creature-guid" => {
                opts.rested_xp_creature_guid =
                    Some(next_arg(&mut args, "--rested-xp-creature-guid")?.parse()?);
            }
            "--rested-xp-runtime-counter" => {
                opts.rested_xp_runtime_counter =
                    Some(next_arg(&mut args, "--rested-xp-runtime-counter")?.parse()?);
            }
            "--rested-xp-offline-secs" => {
                opts.rested_xp_offline_secs =
                    next_arg(&mut args, "--rested-xp-offline-secs")?.parse()?;
            }
            "--rested-xp-timeout" => {
                opts.rested_xp_timeout_secs =
                    next_arg(&mut args, "--rested-xp-timeout")?.parse()?;
            }
            "--loot-race-smoke" => opts.loot_race_smoke = true,
            "--loot-item-capture" => opts.loot_item_capture = true,
            arg if arg == loot_race::ACK_FLAG => opts.ack_disposable_overworld_loot_race = true,
            "--loot-race-account-a" => {
                opts.loot_race_account_a = next_arg(&mut args, "--loot-race-account-a")?;
            }
            "--loot-race-account-b" => {
                opts.loot_race_account_b = next_arg(&mut args, "--loot-race-account-b")?;
            }
            flag @ ("--loot-race-gameobject-entry" | "--loot-race-creature-entry") => {
                opts.loot_race_creature_entry = next_arg(&mut args, flag)?.parse()?;
            }
            flag @ ("--loot-race-gameobject-spawn-guid" | "--loot-race-creature-spawn-guid") => {
                opts.loot_race_creature_spawn_guid = next_arg(&mut args, flag)?.parse()?;
            }
            "--loot-race-runtime-counter" => {
                opts.loot_race_runtime_counter =
                    next_arg(&mut args, "--loot-race-runtime-counter")?.parse()?;
            }
            "--loot-race-item-entry" => {
                opts.loot_race_item_entry =
                    next_arg(&mut args, "--loot-race-item-entry")?.parse()?;
            }
            "--loot-race-timeout" => {
                opts.loot_race_timeout_secs =
                    next_arg(&mut args, "--loot-race-timeout")?.parse()?;
            }
            "--loot-workflow-deadline" => {
                opts.loot_workflow_deadline_secs =
                    next_arg(&mut args, "--loot-workflow-deadline")?.parse()?;
            }
            "--recover-loot-fixture" => opts.recover_loot_fixture = true,
            "--group-capacity-race-smoke" => opts.group_capacity_race_smoke = true,
            "--group-capacity-leader" => {
                opts.group_capacity_leader_account =
                    next_arg(&mut args, "--group-capacity-leader")?;
            }
            "--group-capacity-candidate-a" => {
                opts.group_capacity_candidate_a_account =
                    next_arg(&mut args, "--group-capacity-candidate-a")?;
            }
            "--group-capacity-candidate-b" => {
                opts.group_capacity_candidate_b_account =
                    next_arg(&mut args, "--group-capacity-candidate-b")?;
            }
            "--group-capacity-group-id" => {
                opts.group_capacity_group_id =
                    next_arg(&mut args, "--group-capacity-group-id")?.parse()?;
            }
            "--group-capacity-timeout" => {
                opts.group_capacity_timeout_secs =
                    next_arg(&mut args, "--group-capacity-timeout")?.parse()?;
            }
            "--quest-smoke" => opts.quest_smoke = true,
            "--quest-creature-entry" => {
                opts.quest_creature_entry =
                    Some(next_arg(&mut args, "--quest-creature-entry")?.parse()?);
            }
            "--quest-creature-guid" => {
                opts.quest_creature_guid =
                    Some(next_arg(&mut args, "--quest-creature-guid")?.parse()?);
            }
            "--quest-guid-counter" => {
                opts.quest_guid_counter =
                    Some(next_arg(&mut args, "--quest-guid-counter")?.parse()?);
            }
            "--quest-runtime-counter" => {
                opts.quest_guid_counter =
                    Some(next_arg(&mut args, "--quest-runtime-counter")?.parse()?);
            }
            "--quest-map" => opts.quest_map_id = Some(next_arg(&mut args, "--quest-map")?.parse()?),
            "--expect-quest" => {
                opts.quest_expected_id = Some(next_arg(&mut args, "--expect-quest")?.parse()?);
            }
            "--forbid-quest" => {
                opts.quest_forbidden_id = Some(next_arg(&mut args, "--forbid-quest")?.parse()?);
            }
            "--forbid-title" => {
                opts.quest_forbidden_title = Some(next_arg(&mut args, "--forbid-title")?)
            }
            "--quest-query-details" => opts.quest_query_details = true,
            "--no-quest-query-details" => opts.quest_query_details = false,
            "--quest-accept" => opts.quest_accept = true,
            "--quest-no-accept" => opts.quest_accept = false,
            "--quest-reset" => opts.quest_reset = true,
            "--quest-relocate" => opts.quest_relocate = true,
            "--quest-set-level" => {
                opts.quest_set_level = Some(next_arg(&mut args, "--quest-set-level")?.parse()?);
            }
            "--quest-set-race" => {
                opts.quest_set_race = Some(next_arg(&mut args, "--quest-set-race")?.parse()?);
            }
            "--quest-set-class" => {
                opts.quest_set_class = Some(next_arg(&mut args, "--quest-set-class")?.parse()?);
            }
            "--quest-objective-persist" => opts.quest_objective_persist = true,
            "--quest-objectives" => {
                opts.quest_objectives =
                    parse_quest_objective_rows(&next_arg(&mut args, "--quest-objectives")?)?;
            }
            "--quest-objective-status" => {
                opts.quest_objective_status =
                    next_arg(&mut args, "--quest-objective-status")?.parse()?;
            }
            "--gossip-select-option-id" => {
                opts.gossip_select_option_id =
                    Some(next_arg(&mut args, "--gossip-select-option-id")?.parse()?);
            }
            "--expect-trainer-list" => opts.expect_trainer_list = true,
            "--expect-trainer-id" => {
                opts.expect_trainer_id = Some(next_arg(&mut args, "--expect-trainer-id")?.parse()?);
            }
            "--quest-timeout" => {
                opts.quest_timeout_secs = next_arg(&mut args, "--quest-timeout")?.parse()?;
            }
            "--report" => opts.report_path = Some(next_arg(&mut args, "--report")?),
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => bail!("Unknown argument `{}`. Use --help.", arg),
        }
    }
    if opts.loot_item_capture
        && opts.loot_race_creature_entry == loot_race::DEFAULT_CREATURE_ENTRY
        && opts.loot_race_creature_spawn_guid == loot_race::DEFAULT_CREATURE_SPAWN_GUID
        && opts.loot_race_runtime_counter == loot_race::DEFAULT_RUNTIME_COUNTER
        && opts.loot_race_item_entry == loot_race::DEFAULT_ITEM_ENTRY
    {
        opts.loot_race_creature_entry = loot_race::DEFAULT_CAPTURE_CREATURE_ENTRY;
        opts.loot_race_creature_spawn_guid = loot_race::DEFAULT_CAPTURE_CREATURE_SPAWN_GUID;
        opts.loot_race_runtime_counter = loot_race::DEFAULT_CAPTURE_RUNTIME_COUNTER;
        opts.loot_race_item_entry = loot_race::DEFAULT_CAPTURE_ITEM_ENTRY;
    }
    Ok(opts)
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next().ok_or_else(|| anyhow!("{} needs a value", flag))
}

fn parse_ack_disposable_rested_xp_arg(arg: &str, acknowledged: &mut bool) -> bool {
    if arg != ACK_DISPOSABLE_RESTED_XP_FLAG {
        return false;
    }

    *acknowledged = true;
    true
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}

fn validate_provisioning_mode(guarded_mode: bool, ensure_test_accounts: bool) -> Result<()> {
    if guarded_mode && ensure_test_accounts {
        bail!(
            "guarded multi-client workflows forbid --ensure-test-accounts/WOW_BOT_ENSURE_TEST_ACCOUNTS; provision fixtures separately, then run the read-only identity preflight"
        );
    }
    Ok(())
}

fn install_loot_termination_token() -> Result<CancellationToken> {
    let token = CancellationToken::new();
    let signal_token = token.clone();
    #[cfg(unix)]
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .context("Install loot-fixture SIGINT handler before mutation")?;
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("Install loot-fixture SIGTERM handler before mutation")?;
    if std::env::var("WOW_BOT_REQUIRE_PARENT_DEATH_GUARD").is_ok_and(|value| is_truthy(&value)) {
        #[cfg(target_os = "linux")]
        {
            // SAFETY: getppid takes no pointers and has no preconditions.
            let parent_before = unsafe { libc::getppid() };
            if parent_before <= 1 {
                bail!("loot parent-death guard has no live supervising parent");
            }
            // SAFETY: PR_SET_PDEATHSIG accepts an integer signal number. SIGTERM
            // is handled by the streams registered synchronously above.
            let result = unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) };
            if result != 0 {
                return Err(std::io::Error::last_os_error())
                    .context("Arm Linux parent-death SIGTERM before fixture mutation");
            }
            // Close the documented prctl race: if the supervisor disappeared
            // before PR_SET_PDEATHSIG was armed, refuse to enter the fixture.
            // SAFETY: getppid takes no pointers and has no preconditions.
            let parent_after = unsafe { libc::getppid() };
            if parent_after != parent_before {
                bail!("loot supervisor changed while arming parent-death guard");
            }
        }
        #[cfg(not(target_os = "linux"))]
        bail!("WOW_BOT_REQUIRE_PARENT_DEATH_GUARD is supported only on Linux");
    }
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = terminate.recv() => {}
            }
        }
        #[cfg(not(unix))]
        if let Err(error) = tokio::signal::ctrl_c().await {
            error!("Termination handler failed: {error}");
        }
        signal_token.cancel();
    });
    Ok(token)
}

async fn finish_guarded_loot_result(
    result: Result<Vec<BotRunResult>>,
) -> Result<Vec<BotRunResult>> {
    match result {
        Ok(results) => Ok(results),
        Err(primary) => match loot_race::recover_pending_fixture_if_present().await {
            Ok(true) => Err(primary.context(
                "loot workflow failed; the durable fixture journal was recovered before exit",
            )),
            Ok(false) => Err(primary),
            Err(recovery) => bail!(
                "loot workflow failed ({primary:#}) and durable fixture recovery also failed ({recovery:#}); leave the normal world stopped and run --recover-loot-fixture"
            ),
        },
    }
}

fn validate_rested_xp_cli_values(
    enabled: bool,
    acknowledged_disposable: bool,
    bot_count: usize,
    creature_entry: u32,
    offline_secs: u64,
    timeout_secs: u64,
    now_secs: u64,
) -> Result<()> {
    if !enabled {
        if acknowledged_disposable {
            bail!("{ACK_DISPOSABLE_RESTED_XP_FLAG} is only valid with --rested-xp-smoke");
        }
        return Ok(());
    }
    if !acknowledged_disposable {
        bail!(
            "--rested-xp-smoke requires {ACK_DISPOSABLE_RESTED_XP_FLAG}; this acknowledges that the selected account, character, and Battle.net identity are disposable QA fixtures"
        );
    }
    if bot_count != 1 {
        bail!("--rested-xp-smoke requires exactly one bot; select it with --single");
    }
    if creature_entry == 0 {
        bail!("--rested-xp-creature-entry must be nonzero");
    }
    if offline_secs == 0 {
        bail!("--rested-xp-offline-secs must be greater than zero");
    }
    if u32::try_from(offline_secs).is_err() {
        bail!("--rested-xp-offline-secs must fit the legacy C++ uint32 interval");
    }
    if offline_secs >= now_secs {
        bail!(
            "--rested-xp-offline-secs ({offline_secs}) must be smaller than the current Unix timestamp ({now_secs})"
        );
    }
    if timeout_secs == 0 {
        bail!("--rested-xp-timeout must be greater than zero");
    }
    Ok(())
}

fn validate_rested_xp_fixture_safety_state(state: &RestedXpFixtureSafetyState) -> Result<()> {
    if state.at_login != 0 {
        bail!(
            "rested-XP fixture requires characters.at_login = 0, found 0x{:X}; login flags can reset or create non-restored character state",
            state.at_login
        );
    }
    if state.game_account_online != 0 {
        bail!(
            "rested-XP fixture game account is marked online; refusing concurrent or stale account state"
        );
    }
    if !state.bnet_email_matches_configured_account {
        bail!(
            "rested-XP fixture account ID does not belong to the configured @bot.local Battle.net email"
        );
    }
    if state.characters_on_game_account != 1 {
        bail!(
            "rested-XP fixture requires a dedicated game account with exactly one character, found {}",
            state.characters_on_game_account
        );
    }
    if state.game_accounts_on_bnet_account != 1 {
        bail!(
            "rested-XP fixture requires a dedicated Battle.net identity with exactly one game account, found {}",
            state.game_accounts_on_bnet_account
        );
    }
    if !state.nonempty_side_state.is_empty() {
        let summary = state
            .nonempty_side_state
            .iter()
            .map(|(table, rows)| format!("{table}={rows}"))
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "rested-XP fixture has non-restored high-risk side state ({summary}); use a clean disposable QA character/account"
        );
    }
    Ok(())
}

fn parse_quest_objective_rows(value: &str) -> Result<Vec<QuestObjectiveDbRow>> {
    let mut rows = Vec::new();
    for raw in value.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (objective, data) = raw
            .split_once(':')
            .or_else(|| raw.split_once('='))
            .ok_or_else(|| anyhow!("Quest objective row `{raw}` must use storage:data"))?;
        rows.push(QuestObjectiveDbRow {
            objective: objective
                .trim()
                .parse()
                .map_err(|e| anyhow!("Invalid objective storage index `{objective}`: {e}"))?,
            data: data
                .trim()
                .parse()
                .map_err(|e| anyhow!("Invalid objective data `{data}`: {e}"))?,
        });
    }
    if rows.is_empty() {
        bail!("Quest objective rows cannot be empty");
    }
    rows.sort();
    rows.dedup_by_key(|row| row.objective);
    Ok(rows)
}

fn print_help() {
    println!("wow-test-bot options:");
    println!("  --config <path>          Config JSON path (default: config.json)");
    println!("  --dungeon <id>           LFG dungeon id override");
    println!("  --timeout <secs>         LFG read window override");
    println!("  --single <account>       Run one configured account only");
    println!("  --parallel               Run enabled bots concurrently (default)");
    println!("  --sequential             Run enabled bots one after another");
    println!("  --auto-teleport          Send CMSG_DF_TELEPORT after group formation");
    println!(
        "  --cleanup-groups         Delete stale group rows for configured bot GUIDs before run"
    );
    println!("  --require-group          Treat missing party info/group formation as failure");
    println!("  --ensure-test-accounts   Create missing local TESTBOT auth rows; validate existing rows without rewriting them");
    println!("  --login-only             Stop after SMSG_LOGIN_VERIFY_WORLD; do not run LFG");
    println!("  --stand-state-smoke      After login, verify Sit then Stand state round-trips");
    println!(
        "  --stand-state <n>        Verify one state instead (0=Stand, 1=Sit, 3=Sleep, 8=Kneel)"
    );
    println!("  --stand-state-timeout <secs>  Per-state response timeout (default: 5)");
    println!(
        "                           Env: WOW_BOT_STAND_STATE_SMOKE, WOW_BOT_STAND_STATE, WOW_BOT_STAND_STATE_TIMEOUT_SECS"
    );
    println!(
        "  --bank-smoke             Deposit, logout/relogin, withdraw, logout, and verify DB persistence"
    );
    println!("  --bank-item-entry <id>   Controlled fixture item (default: 2589)");
    println!("  --bank-runtime-counter <n> Live ObjectGuid low counter for the banker");
    println!("  --bank-timeout <secs>    Per bank phase timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_BANK_SMOKE, WOW_BOT_BANK_ITEM_ENTRY, WOW_BOT_BANK_RUNTIME_COUNTER, WOW_BOT_BANK_TIMEOUT_SECS"
    );
    println!(
        "  --void-storage-smoke     Unlock, deposit, relog/swap, relog/withdraw, and verify CharacterDB"
    );
    println!(
        "  --void-storage-query-capture  Query one seeded void item for a narrow C++/Rust wire capture"
    );
    println!("  --void-storage-item-entry <id> Controlled fixture item (default: 2589)");
    println!(
        "  --void-storage-runtime-counter <n> Optional checked live ObjectGuid counter override"
    );
    println!("  --void-storage-timeout <secs> Per action/DB timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_VOID_STORAGE_SMOKE, WOW_BOT_VOID_STORAGE_QUERY_CAPTURE, WOW_BOT_VOID_STORAGE_ITEM_ENTRY, WOW_BOT_VOID_STORAGE_RUNTIME_COUNTER, WOW_BOT_VOID_STORAGE_RUNTIME_REALM_ID, WOW_BOT_VOID_STORAGE_TIMEOUT_SECS"
    );
    println!(
        "  --vendor-smoke           Buy one extended-cost vendor item, relog, verify DB persistence, and restore the fixture"
    );
    println!("  --vendor-entry <id>      Vendor creature entry (default: 18525 G'eras)");
    println!("  --vendor-spawn-guid <n>  Exact world.creature spawn (default: 96654)");
    println!(
        "  --vendor-runtime-counter <n> Optional checked live ObjectGuid low-counter override"
    );
    println!("  --vendor-item-entry <id> Item to buy (default: 30183)");
    println!("  --vendor-extended-cost <id> Required vendor extended cost (default: 1642)");
    println!("  --vendor-currency-id <id> Cost currency (default: 42)");
    println!("  --vendor-currency-cost <n> Cost for one purchase (default: 15)");
    println!("  --vendor-currency-quantity <n> Seeded quantity (default: 30)");
    println!("  --vendor-timeout <secs>  Vendor response timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_VENDOR_SMOKE, WOW_BOT_VENDOR_ENTRY, WOW_BOT_VENDOR_SPAWN_GUID, WOW_BOT_VENDOR_RUNTIME_COUNTER, WOW_BOT_VENDOR_ITEM_ENTRY, WOW_BOT_VENDOR_EXTENDED_COST, WOW_BOT_VENDOR_CURRENCY_ID, WOW_BOT_VENDOR_CURRENCY_COST, WOW_BOT_VENDOR_CURRENCY_QUANTITY, WOW_BOT_VENDOR_TIMEOUT_SECS"
    );
    println!(
        "  --equipment-set-race-smoke  Concurrently save equipment/transmog sets, logout, relog, and verify shared GUID persistence"
    );
    println!(
        "  --equipment-set-account-a <account>  Equipment-set bot (default TESTBOT2@bot.local)"
    );
    println!(
        "  --equipment-set-account-b <account>  Transmog-set bot (default TESTBOT3@bot.local)"
    );
    println!("  --equipment-set-timeout <secs>  Per response/barrier timeout (default: 10)");
    println!(
        "                           Env: WOW_BOT_EQUIPMENT_SET_RACE_SMOKE, WOW_BOT_EQUIPMENT_SET_ACCOUNT_A, WOW_BOT_EQUIPMENT_SET_ACCOUNT_B, WOW_BOT_EQUIPMENT_SET_TIMEOUT_SECS"
    );
    println!(
        "  --homebind-smoke         Bind at an innkeeper, relog, and verify response packets plus DB persistence"
    );
    println!(
        "  --homebind-runtime-counter <n> Optional ObjectGuid low-counter override for the innkeeper"
    );
    println!("  --homebind-timeout <secs> Bind response timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_HOMEBIND_SMOKE, WOW_BOT_HOMEBIND_RUNTIME_COUNTER, WOW_BOT_HOMEBIND_TIMEOUT_SECS"
    );
    println!(
        "  --rested-xp-smoke       Compare offline rest rates, kill one mob, and verify XP/rest persistence"
    );
    println!(
        "  {ACK_DISPOSABLE_RESTED_XP_FLAG} Acknowledge the rested-XP account/character/BNet fixture is disposable"
    );
    println!("  --rested-xp-creature-entry <id>  Low-level XP target (default: 15274 Mana Wyrm)");
    println!("  --rested-xp-creature-guid <guid> Optional exact world.creature spawn GUID");
    println!(
        "  --rested-xp-runtime-counter <n> Optional live counter; must match the target's discovered CREATE_OBJECT"
    );
    println!("  --rested-xp-offline-secs <n> Simulated offline interval (default: 86400)");
    println!("  --rested-xp-timeout <secs> Combat/DB response timeout (default: 120)");
    println!(
        "                           Env: WOW_BOT_RESTED_XP_SMOKE, WOW_BOT_RESTED_XP_CREATURE_ENTRY, WOW_BOT_RESTED_XP_CREATURE_GUID, WOW_BOT_RESTED_XP_RUNTIME_COUNTER, WOW_BOT_RESTED_XP_OFFLINE_SECS, WOW_BOT_RESTED_XP_TIMEOUT_SECS"
    );
    println!(
        "  --loot-race-smoke       Race ITEM and MONEY claims on one shared chest from two real sessions"
    );
    println!(
        "  --loot-item-capture     One real session kills/opens/claims only the item and emits a fixed capture fence"
    );
    println!(
        "  {}  REQUIRED acknowledgement: mutates disposable loot fixtures (capture also kills its creature)",
        loot_race::ACK_FLAG
    );
    println!(
        "  --loot-race-account-a <account>  First disposable contender/capture killer (default TESTBOT2@bot.local)"
    );
    println!(
        "  --loot-race-account-b <account>  Second disposable bot (default TESTBOT3@bot.local)"
    );
    println!("  --loot-race-gameobject-entry <id> Exact chest entry (default 2846 Tattered Chest)");
    println!(
        "  --loot-race-gameobject-spawn-guid <id> Exact wrapper-owned world.gameobject spawn (default 9106001)"
    );
    println!("                           Legacy aliases: --loot-race-creature-entry / --loot-race-creature-spawn-guid");
    println!("  --loot-race-runtime-counter <n>  Optional strict live counter override (default 0=auto-discover full GUID)");
    println!("  --loot-race-item-entry <id>      Exact shared-chest loot item (default 38)");
    println!("  --loot-race-timeout <secs>       Per coordination/loot timeout (capture also uses it for combat; default 30)");
    println!("  --loot-workflow-deadline <secs>  Hard end-to-end loot deadline before guarded cleanup (default 900)");
    println!("  --recover-loot-fixture           Restore one pending WOW_BOT_FIXTURE_JOURNAL; no login/service actions");
    println!(
        "                           Race uses a guarded temporary chest; capture keeps its separate Doctor creature fixture"
    );
    println!(
        "                           Env capture mode: WOW_BOT_LOOT_ITEM_CAPTURE=1 (uses account A; account B remains offline as a guarded fixture snapshot)"
    );
    println!(
        "  --group-capacity-race-smoke  Race two invite accepts for the fifth slot of a preloaded four-member party"
    );
    println!("  --group-capacity-leader <account>     Preloaded party leader (default TESTBOT1)");
    println!("  --group-capacity-candidate-a <account> First invitee (default TESTBOT2)");
    println!("  --group-capacity-candidate-b <account> Second invitee (default TESTBOT3)");
    println!("  --group-capacity-group-id <id>         Required preloaded CharacterDB group id");
    println!("  --group-capacity-timeout <secs>        Per barrier/packet timeout (default 30)");
    println!("  --quest-smoke            After login, right-click/query one questgiver NPC");
    println!("  --quest-creature-entry <id>  Creature entry to resolve from world.creature");
    println!("  --quest-creature-guid <guid> Optional world.creature spawn guid override");
    println!("  --quest-runtime-counter <n> Live ObjectGuid low counter for the target");
    println!("  --quest-guid-counter <n> Legacy alias for --quest-runtime-counter");
    println!("  --quest-map <id>         Optional map id override for GUID construction");
    println!("  --expect-quest <id>      Require this quest id in gossip/list/details");
    println!("  --forbid-quest <id>      Fail if this quest id is offered");
    println!("  --forbid-title <text>    Fail if an offered quest title contains this text");
    println!("  --no-quest-query-details Skip the QuestGiverQueryQuest details probe");
    println!("  --quest-accept           Accept the selected quest and verify DB persistence");
    println!(
        "  --quest-reset            Delete the selected quest from bot quest tables before run"
    );
    println!(
        "  --quest-relocate         Move the bot character near the target spawn before login"
    );
    println!(
        "  --quest-set-level <n>    Set bot character level before login for deterministic QA"
    );
    println!("  --quest-set-race <n>     Set bot character race before login");
    println!("  --quest-set-class <n>    Set bot character class before login");
    println!(
        "  --quest-objective-persist Seed expected quest objectives, logout, and verify DB rows"
    );
    println!("  --quest-objectives <rows> Objective rows as storage:data,storage:data");
    println!("  --quest-objective-status <n> Status for seeded quest row (default: 3)");
    println!("  --gossip-select-option-id <id> Select this gossip option after GossipMessage");
    println!("  --expect-trainer-list     Require SMSG_TRAINER_LIST after gossip select");
    println!("  --expect-trainer-id <id>  Require this TrainerID in SMSG_TRAINER_LIST");
    println!("  --quest-timeout <secs>   Quest smoke read window after sending hello");
    println!("  --report <path>          Write JSON report");
}

fn apply_password_overrides(bots: &mut [config::BotConfig]) {
    let shared_password = std::env::var("WOW_BOT_PASSWORD").ok();
    for bot in bots {
        if let Ok(password) = std::env::var(password_env_name(&bot.account)) {
            bot.password = password;
        } else if bot.password.is_empty() {
            if let Some(password) = &shared_password {
                bot.password = password.clone();
            }
        }
    }
}

fn password_env_name(account: &str) -> String {
    let suffix: String = account
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("WOW_BOT_PASSWORD_{suffix}")
}

fn ensure_test_accounts(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let auth_db = auth_db_url()?;
    let char_db = characters_db_url()?;
    let auth_opts = qa_mysql_opts(&auth_db, "auth")?;
    let char_opts = qa_mysql_opts(&char_db, "characters")?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut char_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    // Validate every existing character before the first auth write. Account
    // provisioning is intentionally create-only: an ID collision must never
    // become authority to rewrite credentials or character ownership.
    for bot in bots {
        validate_local_bot_character_owner(&mut char_conn, bot)?;
    }
    for bot in bots {
        let character_count: u64 = char_conn
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE account = ?",
                (bot.account_id,),
            )
            .map_err(|e| anyhow!("Count characters for account {}: {e}", bot.account_id))?
            .unwrap_or(0);
        provision_local_bot_account_create_only(&mut auth_conn, bot, character_count)?;
    }

    Ok(())
}

fn qa_mysql_opts(url: &str, label: &str) -> Result<mysql::Opts> {
    let opts = mysql::Opts::from_url(url).map_err(|e| anyhow!("Bad {label} DB URL: {e}"))?;
    Ok(mysql::OptsBuilder::from_opts(opts)
        .tcp_connect_timeout(Some(Duration::from_secs(10)))
        .read_timeout(Some(Duration::from_secs(30)))
        .write_timeout(Some(Duration::from_secs(30)))
        .into())
}

fn provision_local_bot_account_create_only(
    conn: &mut mysql::Conn,
    bot: &config::BotConfig,
    character_count: u64,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let (email, bnet_salt, bnet_verifier) =
        bot_srp6::bnet_v1_registration_material_like_cpp(&bot.account, &bot.password);
    let allow_nonlocal =
        std::env::var("WOW_BOT_ALLOW_NONLOCAL_ACCOUNT_BOOTSTRAP").is_ok_and(|v| is_truthy(&v));
    if !email.ends_with("@BOT.LOCAL") && !allow_nonlocal {
        bail!(
            "Refusing to bootstrap non-local bot account {email}; set WOW_BOT_ALLOW_NONLOCAL_ACCOUNT_BOOTSTRAP=1 if this is intentional"
        );
    }
    let game_username = game_account_username(&bot.account)?;
    let bnet_rows: Vec<(u32, String)> = conn
        .exec(
            "SELECT id, email FROM battlenet_accounts WHERE email = ? ORDER BY id",
            (&email,),
        )
        .map_err(|e| anyhow!("Lookup BNet account {email}: {e}"))?;
    if bnet_rows.len() > 1 {
        bail!(
            "Refusing account provisioning: BNet email {email} has {} rows",
            bnet_rows.len()
        );
    }
    let game_account_exists = conn
        .exec_first::<u32, _, _>("SELECT id FROM account WHERE id = ?", (bot.account_id,))
        .map_err(|e| anyhow!("Lookup game account id {}: {e}", bot.account_id))?
        .is_some();

    // Existing identities are validation-only. This makes repeated
    // provisioning idempotent while forbidding credential rewrites.
    match create_only_provisioning_plan(!bnet_rows.is_empty(), game_account_exists)? {
        CreateOnlyProvisioningPlan::ValidateExisting => {
            validate_exact_bot_identity(conn, None, bot)?;
            validate_realm_character_count(conn, bot, character_count)?;
            info!(
                "[Bot {}] existing local auth fixture validated without mutation",
                bot.account_id
            );
            return Ok(());
        }
        CreateOnlyProvisioningPlan::CreateBoth => {}
    }

    let colliding_accounts: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE username = ? OR email = ? OR reg_mail = ?",
            (&game_username, &email, &email),
        )
        .map_err(|e| anyhow!("Check game-account identity collisions: {e}"))?
        .unwrap_or(0);
    if !game_account_exists && colliding_accounts != 0 {
        bail!(
            "Refusing account provisioning: username/email for {} already belongs to another game account",
            bot.account
        );
    }

    let expected_numchars = u8::try_from(character_count).map_err(|_| {
        anyhow!("Character count {character_count} exceeds realmcharacters capacity")
    })?;
    let mut tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start create-only account transaction: {e}"))?;
    let bnet_id = if let Some((id, _)) = bnet_rows.first() {
        *id
    } else {
        tx.exec_drop(
            "INSERT INTO battlenet_accounts (email, srp_version, salt, verifier) VALUES (?, 1, ?, ?)",
            (&email, bnet_salt.to_vec(), bnet_verifier),
        )
        .map_err(|e| anyhow!("Insert BNet account {email}: {e}"))?;
        u32::try_from(
            tx.last_insert_id()
                .ok_or_else(|| anyhow!("BNet account insert returned no id"))?,
        )
        .map_err(|_| anyhow!("BNet account id overflow"))?
    };

    if !game_account_exists {
        // The 3.4.3 world login path authenticates through
        // account.session_key_bnet. Legacy Grunt fields remain NOT NULL.
        tx.exec_drop(
            "INSERT INTO account \
             (id, username, salt, verifier, reg_mail, email, joindate, battlenet_account, battlenet_index, expansion) \
             VALUES (?, ?, ?, ?, ?, ?, NOW(), ?, 1, 9)",
            (
                bot.account_id,
                &game_username,
                random_32().to_vec(),
                fixed_le_32(Vec::new()),
                &email,
                &email,
                bnet_id,
            ),
        )
        .map_err(|e| anyhow!("Insert game account {}: {e}", bot.account_id))?;
    }

    let existing_realm_count: Option<u8> = tx
        .exec_first(
            "SELECT numchars FROM realmcharacters WHERE acctid = ? AND realmid = ?",
            (bot.account_id, realm_id()),
        )
        .map_err(|e| anyhow!("Load realmcharacters for account {}: {e}", bot.account_id))?;
    match existing_realm_count {
        Some(actual) if actual != expected_numchars => bail!(
            "Refusing to rewrite realmcharacters for account {}: expected {}, found {}",
            bot.account_id,
            expected_numchars,
            actual
        ),
        Some(_) => {}
        None => tx
            .exec_drop(
                "INSERT INTO realmcharacters (numchars, acctid, realmid) VALUES (?, ?, ?)",
                (expected_numchars, bot.account_id, realm_id()),
            )
            .map_err(|e| anyhow!("Insert realmcharacters for account {}: {e}", bot.account_id))?,
    }
    tx.commit()
        .map_err(|e| anyhow!("Commit create-only account transaction: {e}"))?;
    validate_exact_bot_identity(conn, None, bot)?;
    validate_realm_character_count(conn, bot, character_count)?;
    info!(
        "[Bot {}] created missing local auth rows without rewriting existing identities",
        bot.account_id
    );
    Ok(())
}

fn validate_realm_character_count(
    auth_conn: &mut mysql::Conn,
    bot: &config::BotConfig,
    character_count: u64,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let expected = u8::try_from(character_count).map_err(|_| {
        anyhow!("Character count {character_count} exceeds realmcharacters capacity")
    })?;
    let actual: Option<u8> = auth_conn
        .exec_first(
            "SELECT numchars FROM realmcharacters WHERE acctid = ? AND realmid = ?",
            (bot.account_id, realm_id()),
        )
        .map_err(|e| anyhow!("Load realmcharacters for account {}: {e}", bot.account_id))?;
    if actual != Some(expected) {
        bail!(
            "realmcharacters for account {} must remain {}, found {actual:?}; create-only provisioning will not rewrite it",
            bot.account_id,
            expected
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateOnlyProvisioningPlan {
    ValidateExisting,
    CreateBoth,
}

fn create_only_provisioning_plan(
    bnet_exists: bool,
    game_account_exists: bool,
) -> Result<CreateOnlyProvisioningPlan> {
    match (bnet_exists, game_account_exists) {
        (true, true) => Ok(CreateOnlyProvisioningPlan::ValidateExisting),
        (false, false) => Ok(CreateOnlyProvisioningPlan::CreateBoth),
        (true, false) => bail!(
            "Refusing create-only provisioning: a BNet identity already exists without the configured game account"
        ),
        (false, true) => bail!(
            "Refusing create-only provisioning: the numeric game-account ID already exists without the configured BNet identity"
        ),
    }
}

fn game_account_username(account: &str) -> Result<String> {
    let local_part = account
        .split('@')
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| anyhow!("Bot account {account} has no local part"))?;
    let username = bot_srp6::utf8_to_upper_only_latin_like_cpp(local_part);
    if username.len() > 32 {
        bail!("Bot username {username} exceeds Trinity account.username limit");
    }
    Ok(username)
}

fn fixed_le_32(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.resize(32, 0);
    bytes.truncate(32);
    bytes
}

fn random_32() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

fn validate_local_bot_character_owner(
    conn: &mut mysql::Conn,
    bot: &config::BotConfig,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let (owner, online, at_login) = conn
        .exec_first::<(u32, u8, u16), _, _>(
            "SELECT account, online, at_login FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Lookup character {}: {e}", bot.character_guid))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;

    if owner != bot.account_id {
        bail!(
            "Refusing to reassign character GUID {} from account {} to configured test account {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 || at_login != 0 {
        bail!(
            "Character GUID {} is not an offline clean fixture (online={online}, at_login={at_login})",
            bot.character_guid
        );
    }

    Ok(())
}

fn validate_exact_bot_identity(
    auth_conn: &mut mysql::Conn,
    mut character_conn: Option<&mut mysql::Conn>,
    bot: &config::BotConfig,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let expected_email = bot_srp6::utf8_to_upper_only_latin_like_cpp(&bot.account);
    let expected_username = game_account_username(&bot.account)?;
    let bnet_rows: Vec<(u32, String, i8, Vec<u8>, Vec<u8>, u32, u8, u8)> = auth_conn
        .exec(
            "SELECT id, email, srp_version, salt, verifier, failed_logins, locked, online \
             FROM battlenet_accounts WHERE email = ? ORDER BY id",
            (&expected_email,),
        )
        .map_err(|e| anyhow!("Load exact BNet fixture {}: {e}", bot.account))?;
    if bnet_rows.len() != 1 {
        bail!(
            "Expected exactly one BNet row for {}, found {}",
            bot.account,
            bnet_rows.len()
        );
    }
    let (bnet_id, email, srp_version, salt, verifier, failed_logins, locked, bnet_online) =
        &bnet_rows[0];
    let (_, expected_verifier) =
        bot_srp6::bnet_v1_verifier_for_salt_like_cpp(&bot.account, &bot.password, salt);
    if !email.eq_ignore_ascii_case(&expected_email)
        || *srp_version != 1
        || salt.len() != 32
        || *verifier != expected_verifier
        || *failed_logins != 0
        || *locked != 0
        || *bnet_online != 0
    {
        bail!(
            "BNet fixture {} does not exactly match configured credentials/offline state",
            bot.account
        );
    }

    let account = auth_conn
        .exec_first::<(
            String,
            String,
            String,
            Option<u32>,
            Option<u8>,
            u8,
            u32,
            u8,
            u8,
        ), _, _>(
            "SELECT username, reg_mail, email, battlenet_account, battlenet_index, expansion, \
                    failed_logins, locked, online FROM account WHERE id = ?",
            (bot.account_id,),
        )
        .map_err(|e| anyhow!("Load exact game-account fixture {}: {e}", bot.account_id))?
        .ok_or_else(|| anyhow!("No game-account row for id {}", bot.account_id))?;
    if !account.0.eq_ignore_ascii_case(&expected_username)
        || !account.1.eq_ignore_ascii_case(&expected_email)
        || !account.2.eq_ignore_ascii_case(&expected_email)
        || account.3 != Some(*bnet_id)
        || account.4 != Some(1)
        || account.5 != 9
        || account.6 != 0
        || account.7 != 0
        || account.8 != 0
    {
        bail!(
            "Game account {} does not exactly match username/email/BNet/offline fixture contract",
            bot.account_id
        );
    }
    let game_accounts_on_bnet: u64 = auth_conn
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE battlenet_account = ?",
            (*bnet_id,),
        )
        .map_err(|e| anyhow!("Count game accounts on BNet fixture: {e}"))?
        .unwrap_or(0);
    if game_accounts_on_bnet != 1 {
        bail!(
            "BNet fixture {} must own exactly one game account, found {game_accounts_on_bnet}",
            bot.account
        );
    }
    let bnet_bans: u64 = auth_conn
        .exec_first(
            "SELECT COUNT(*) FROM battlenet_account_bans WHERE id = ?",
            (*bnet_id,),
        )
        .map_err(|e| anyhow!("Check BNet fixture bans: {e}"))?
        .unwrap_or(0);
    let game_bans: u64 = auth_conn
        .exec_first(
            "SELECT COUNT(*) FROM account_banned WHERE id = ? AND active <> 0",
            (bot.account_id,),
        )
        .map_err(|e| anyhow!("Check game-account fixture bans: {e}"))?
        .unwrap_or(0);
    if bnet_bans != 0 || game_bans != 0 {
        bail!(
            "Configured bot fixture is banned (bnet rows={bnet_bans}, active game rows={game_bans}); provisioning will not clear bans"
        );
    }

    if let Some(char_conn) = character_conn.as_mut() {
        validate_local_bot_character_owner(char_conn, bot)?;
        let count: u64 = char_conn
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE account = ?",
                (bot.account_id,),
            )
            .map_err(|e| anyhow!("Count dedicated fixture characters: {e}"))?
            .unwrap_or(0);
        if count != 1 {
            bail!(
                "Loot fixture account {} must own exactly one character, found {count}",
                bot.account_id
            );
        }
        let realm_count: Option<u8> = auth_conn
            .exec_first(
                "SELECT numchars FROM realmcharacters WHERE acctid = ? AND realmid = ?",
                (bot.account_id, realm_id()),
            )
            .map_err(|e| anyhow!("Load realmcharacters fixture count: {e}"))?;
        if realm_count != Some(1) {
            bail!(
                "realmcharacters fixture count for account {} must be exactly 1, found {realm_count:?}",
                bot.account_id
            );
        }
    }
    Ok(())
}

fn validate_exact_loot_bot_identities(bots: &[config::BotConfig]) -> Result<()> {
    let auth_opts = qa_mysql_opts(&auth_db_url()?, "auth")?;
    let char_opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut character_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    for bot in bots {
        validate_exact_bot_identity(&mut auth_conn, Some(&mut character_conn), bot)?;
    }
    Ok(())
}

fn validate_linked_group_capacity_bot_identities(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let auth_opts = qa_mysql_opts(&auth_db_url()?, "auth")?;
    let char_opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut character_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    for bot in bots {
        validate_local_bot_character_owner(&mut character_conn, bot)?;
        let expected_email = bot_srp6::utf8_to_upper_only_latin_like_cpp(&bot.account);
        let expected_username = game_account_username(&bot.account)?;
        let game_account = auth_conn
            .exec_first::<(
                String,
                String,
                String,
                Option<u32>,
                Option<u8>,
                u8,
                u32,
                u8,
                u8,
            ), _, _>(
                "SELECT username, reg_mail, email, battlenet_account, battlenet_index, expansion, \
                        failed_logins, locked, online FROM account WHERE id = ?",
                (bot.account_id,),
            )
            .map_err(|e| {
                anyhow!(
                    "Load linked group-capacity game account {}: {e}",
                    bot.account_id
                )
            })?
            .ok_or_else(|| {
                anyhow!(
                    "No linked group-capacity game account for id {}",
                    bot.account_id
                )
            })?;
        let bnet_id = game_account.3.ok_or_else(|| {
            anyhow!(
                "Group-capacity game account {} has no linked BNet identity",
                bot.account_id
            )
        })?;
        if !game_account.0.eq_ignore_ascii_case(&expected_username)
            || !game_account.1.eq_ignore_ascii_case(&expected_email)
            || !game_account.2.eq_ignore_ascii_case(&expected_email)
            || game_account.4 != Some(1)
            || game_account.5 != 9
            || game_account.6 != 0
            || game_account.7 != 0
            || game_account.8 != 0
        {
            bail!(
                "Linked group-capacity game account {} does not match configured identity/offline state",
                bot.account_id
            );
        }

        let bnet_account = auth_conn
            .exec_first::<(String, i8, Vec<u8>, Vec<u8>, u32, u8, u8), _, _>(
                "SELECT email, srp_version, salt, verifier, failed_logins, locked, online \
                 FROM battlenet_accounts WHERE id = ?",
                (bnet_id,),
            )
            .map_err(|e| anyhow!("Load linked group-capacity BNet identity {bnet_id}: {e}"))?
            .ok_or_else(|| anyhow!("No linked group-capacity BNet identity for id {bnet_id}"))?;
        // These long-lived group fixtures predate the current create-only SRP
        // provisioning helper, so their stored verifier is not reproducible by
        // `bnet_v1_verifier_for_salt_like_cpp`. Pin the exact linked identity,
        // SRP shape, offline state, and bans here. The World authentication that
        // follows proves possession of either the configured 64-byte fixture
        // key or a key derived by the live BNet fallback.
        if !bnet_account.0.eq_ignore_ascii_case(&expected_email)
            || bnet_account.1 != 1
            || bnet_account.2.len() != 32
            || bnet_account.3.len() != 128
            || bnet_account.4 != 0
            || bnet_account.5 != 0
            || bnet_account.6 != 0
        {
            bail!(
                "Linked group-capacity BNet identity {} does not match configured credentials/offline state",
                bot.account
            );
        }

        let bnet_bans: u64 = auth_conn
            .exec_first(
                "SELECT COUNT(*) FROM battlenet_account_bans WHERE id = ?",
                (bnet_id,),
            )
            .map_err(|e| anyhow!("Check linked group-capacity BNet bans: {e}"))?
            .unwrap_or(0);
        let game_bans: u64 = auth_conn
            .exec_first(
                "SELECT COUNT(*) FROM account_banned WHERE id = ? AND active <> 0",
                (bot.account_id,),
            )
            .map_err(|e| anyhow!("Check linked group-capacity game-account bans: {e}"))?
            .unwrap_or(0);
        if bnet_bans != 0 || game_bans != 0 {
            bail!(
                "Configured group-capacity bot is banned (bnet rows={bnet_bans}, active game rows={game_bans})"
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("🎮 WoW Test Bot - TrinityCore 3.4.3 SRP6 + AES-GCM");
    info!("═══════════════════════════════════════════════════");

    let cli = parse_cli()?;
    if cli.recover_loot_fixture {
        let conflicting_mode = cli.ensure_test_accounts
            || cli.login_only
            || cli.stand_state_smoke
            || cli.bank_smoke
            || cli.void_storage_smoke
            || cli.void_storage_query_capture
            || cli.homebind_smoke
            || cli.inventory_swap_smoke
            || cli.vendor_smoke
            || cli.equipment_set_race_smoke
            || cli.rested_xp_smoke
            || cli.loot_race_smoke
            || cli.loot_item_capture
            || cli.group_capacity_race_smoke
            || cli.quest_smoke
            || cli.single_account.is_some();
        if conflicting_mode {
            bail!("--recover-loot-fixture must be used alone");
        }
        loot_race::recover_pending_fixture().await?;
        info!("Pending loot fixture recovered and cleanup marker written");
        return Ok(());
    }
    let app_config = config::AppConfig::load_or_create(&cli.config_path)?;
    let mut bots: Vec<config::BotConfig> =
        app_config.get_enabled_bots().into_iter().cloned().collect();

    if let Some(account) = &cli.single_account {
        bots.retain(|bot| bot.account.eq_ignore_ascii_case(account));
    }
    if cli.loot_race_smoke || cli.loot_item_capture {
        if cli.single_account.is_some() {
            bail!(
                "--single is incompatible with the loot workflows; select the guarded fixture with the loot account flags"
            );
        }
        bots.retain(|bot| {
            bot.account.eq_ignore_ascii_case(&cli.loot_race_account_a)
                || bot.account.eq_ignore_ascii_case(&cli.loot_race_account_b)
        });
    }
    if cli.group_capacity_race_smoke {
        if cli.single_account.is_some() {
            bail!("--single is incompatible with --group-capacity-race-smoke");
        }
        bots.retain(|bot| {
            bot.account
                .eq_ignore_ascii_case(&cli.group_capacity_leader_account)
                || bot
                    .account
                    .eq_ignore_ascii_case(&cli.group_capacity_candidate_a_account)
                || bot
                    .account
                    .eq_ignore_ascii_case(&cli.group_capacity_candidate_b_account)
        });
    }
    if cli.equipment_set_race_smoke {
        if cli.single_account.is_some() {
            bail!("--single is incompatible with --equipment-set-race-smoke");
        }
        bots.retain(|bot| {
            bot.account
                .eq_ignore_ascii_case(&cli.equipment_set_account_a)
                || bot
                    .account
                    .eq_ignore_ascii_case(&cli.equipment_set_account_b)
        });
    }
    apply_password_overrides(&mut bots);

    if bots.is_empty() {
        bail!("No enabled bots matched the current config/filter");
    }
    let missing_passwords: Vec<&str> = bots
        .iter()
        .filter(|bot| {
            bot.password.is_empty()
                && !(cli.group_capacity_race_smoke && !bot.session_key_bnet.trim().is_empty())
                && !cli.equipment_set_race_smoke
        })
        .map(|bot| bot.account.as_str())
        .collect();
    if !missing_passwords.is_empty() {
        bail!(
            "Missing bot password for {}. Set WOW_BOT_PASSWORD, set {}, or use an ignored local config.json.",
            missing_passwords.join(", "),
            password_env_name(missing_passwords[0])
        );
    }
    let loot_mode = cli.loot_race_smoke || cli.loot_item_capture;
    let guarded_identity_mode =
        loot_mode || cli.group_capacity_race_smoke || cli.equipment_set_race_smoke;
    validate_provisioning_mode(guarded_identity_mode, cli.ensure_test_accounts)?;
    let post_login_mode_count = [
        cli.stand_state_smoke,
        cli.bank_smoke,
        cli.void_storage_smoke,
        cli.void_storage_query_capture,
        cli.homebind_smoke,
        cli.inventory_swap_smoke,
        cli.vendor_smoke,
        cli.equipment_set_race_smoke,
        cli.rested_xp_smoke,
        cli.loot_race_smoke,
        cli.loot_item_capture,
        cli.group_capacity_race_smoke,
        cli.quest_smoke,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if post_login_mode_count > 1 {
        bail!("stand-state, bank, void-storage, homebind, inventory-swap, vendor, equipment-set-race, rested-xp, loot-race, loot-item-capture, group-capacity-race, and quest smoke are separate post-login modes");
    }
    if cli.bank_smoke && bots.len() != 1 {
        bail!("--bank-smoke requires exactly one bot; select it with --single");
    }
    if cli.bank_smoke && cli.bank_timeout_secs == 0 {
        bail!("--bank-timeout must be greater than zero");
    }
    if cli.bank_smoke && cli.bank_runtime_counter.is_none() {
        bail!(
            "--bank-smoke requires --bank-runtime-counter or WOW_BOT_BANK_RUNTIME_COUNTER for the live banker ObjectGuid"
        );
    }
    if (cli.void_storage_smoke || cli.void_storage_query_capture) && bots.len() != 1 {
        bail!("void-storage modes require exactly one bot; select it with --single");
    }
    if (cli.void_storage_smoke || cli.void_storage_query_capture)
        && cli.void_storage_timeout_secs == 0
    {
        bail!("--void-storage-timeout must be greater than zero");
    }
    if cli.homebind_smoke && bots.len() != 1 {
        bail!("--homebind-smoke requires exactly one bot; select it with --single");
    }
    if cli.homebind_smoke && cli.homebind_timeout_secs == 0 {
        bail!("--homebind-timeout must be greater than zero");
    }
    if cli.inventory_swap_smoke && bots.len() != 1 {
        bail!("--inventory-swap-smoke requires exactly one bot; select it with --single");
    }
    if cli.inventory_swap_smoke && cli.inventory_swap_timeout_secs == 0 {
        bail!("--inventory-swap-timeout must be greater than zero");
    }
    if cli.inventory_swap_smoke
        && cli.inventory_swap_item_entry_a == cli.inventory_swap_item_entry_b
    {
        bail!("inventory-swap fixture item entries must be different to avoid stack merging");
    }
    if cli.vendor_smoke && bots.len() != 1 {
        bail!("--vendor-smoke requires exactly one bot; select it with --single");
    }
    if cli.vendor_smoke && cli.vendor_timeout_secs == 0 {
        bail!("--vendor-timeout must be greater than zero");
    }
    if cli.vendor_smoke
        && (cli.vendor_entry == 0
            || cli.vendor_spawn_guid == 0
            || cli.vendor_item_entry == 0
            || cli.vendor_extended_cost == 0
            || cli.vendor_currency_id == 0
            || cli.vendor_currency_cost == 0
            || cli.vendor_currency_quantity <= cli.vendor_currency_cost)
    {
        bail!("vendor smoke requires nonzero fixture identifiers/cost and a seeded currency quantity greater than one purchase cost");
    }
    if cli.equipment_set_race_smoke {
        if bots.len() != 2 {
            bail!("--equipment-set-race-smoke requires exactly its two configured bots");
        }
        if cli.equipment_set_timeout_secs == 0 {
            bail!("--equipment-set-timeout must be greater than zero");
        }
        if cli
            .equipment_set_account_a
            .eq_ignore_ascii_case(&cli.equipment_set_account_b)
        {
            bail!("equipment-set race accounts must be distinct");
        }
    }
    validate_rested_xp_cli_values(
        cli.rested_xp_smoke,
        cli.ack_disposable_rested_xp,
        bots.len(),
        cli.rested_xp_creature_entry,
        cli.rested_xp_offline_secs,
        cli.rested_xp_timeout_secs,
        current_epoch_secs(),
    )?;
    let loot_race_cli = loot_race::LootRaceCli {
        account_a: cli.loot_race_account_a.clone(),
        account_b: cli.loot_race_account_b.clone(),
        entry: cli.loot_race_creature_entry,
        spawn_guid: cli.loot_race_creature_spawn_guid,
        runtime_counter: cli.loot_race_runtime_counter,
        item_entry: cli.loot_race_item_entry,
        timeout_secs: cli.loot_race_timeout_secs,
        workflow_deadline_secs: cli.loot_workflow_deadline_secs,
    };
    let group_capacity_cli = loot_race::GroupCapacityRaceCli {
        leader_account: cli.group_capacity_leader_account.clone(),
        candidate_a_account: cli.group_capacity_candidate_a_account.clone(),
        candidate_b_account: cli.group_capacity_candidate_b_account.clone(),
        group_db_store_id: cli.group_capacity_group_id,
        timeout_secs: cli.group_capacity_timeout_secs,
    };
    loot_race::validate_cli(
        cli.loot_race_smoke,
        cli.loot_item_capture,
        cli.ack_disposable_overworld_loot_race,
        &bots,
        &loot_race_cli,
    )?;
    if loot_mode {
        loot_race::validate_journal_contract()?;
        let bots_for_validation = bots.clone();
        tokio::task::spawn_blocking(move || {
            validate_exact_loot_bot_identities(&bots_for_validation)
        })
        .await
        .map_err(|e| anyhow!("Loot identity-preflight DB worker join failed: {e}"))??;
    } else if cli.group_capacity_race_smoke {
        let bots_for_validation = bots.clone();
        tokio::task::spawn_blocking(move || {
            validate_linked_group_capacity_bot_identities(&bots_for_validation)
        })
        .await
        .map_err(|e| anyhow!("Group-capacity identity preflight DB worker failed: {e}"))??;
    } else if cli.ensure_test_accounts {
        let bots_for_db = bots.clone();
        tokio::task::spawn_blocking(move || ensure_test_accounts(&bots_for_db))
            .await
            .map_err(|e| anyhow!("DB worker join failed while provisioning test accounts: {e}"))?
            .map_err(|e| anyhow!("Failed to provision test accounts: {e}"))?;
    }
    let stand_state_options = if cli.stand_state_smoke {
        Some(stand_state_smoke_options_from_cli(&cli)?)
    } else {
        None
    };
    let quest_options = if cli.quest_smoke {
        Some(quest_smoke_options_from_cli(&cli)?)
    } else {
        None
    };

    let dungeon_id = cli
        .dungeon_id
        .unwrap_or_else(|| test_dungeon_id(&app_config));
    let timeout_secs = cli
        .timeout_secs
        .unwrap_or(app_config.test_config.wait_for_proposal_timeout_secs);
    let auto_teleport = cli
        .auto_teleport
        .unwrap_or(app_config.test_config.auto_teleport);
    let cleanup_groups = cli
        .cleanup_groups
        .unwrap_or(app_config.test_config.cleanup_groups);
    let require_proposal = app_config.test_config.tests.lfg_proposal;
    let require_group = cli.require_group || app_config.test_config.require_group;

    info!("Target dungeon: {}", dungeon_id);
    info!("Enabled bots: {}", bots.len());
    if let Some(options) = &stand_state_options {
        info!(
            "Stand-state sequence: {:?}; per-state timeout: {}s",
            options.states, options.timeout_secs
        );
    }
    info!(
        "Mode: {}; client_build={}; LFG timeout: {}s; auto_teleport={}; require_proposal={}; require_group={}",
        if cli.stand_state_smoke {
            "stand-state-smoke"
        } else if cli.bank_smoke {
            "bank-smoke"
        } else if cli.void_storage_smoke {
            "void-storage-smoke"
        } else if cli.void_storage_query_capture {
            "void-storage-query-capture"
        } else if cli.homebind_smoke {
            "homebind-smoke"
        } else if cli.inventory_swap_smoke {
            "inventory-swap-smoke"
        } else if cli.vendor_smoke {
            "vendor-smoke"
        } else if cli.equipment_set_race_smoke {
            "equipment-set-race-smoke"
        } else if cli.rested_xp_smoke {
            "rested-xp-smoke"
        } else if cli.loot_race_smoke {
            "loot-race-smoke"
        } else if cli.loot_item_capture {
            "loot-item-capture"
        } else if cli.group_capacity_race_smoke {
            "group-capacity-race-smoke"
        } else if cli.quest_smoke {
            "quest-smoke"
        } else if cli.login_only {
            "login-only"
        } else {
            "lfg"
        },
        client_build(),
        timeout_secs,
        auto_teleport,
        require_proposal,
        require_group
    );

    if cleanup_groups
        && !cli.login_only
        && !cli.stand_state_smoke
        && !cli.bank_smoke
        && !cli.void_storage_smoke
        && !cli.void_storage_query_capture
        && !cli.homebind_smoke
        && !cli.inventory_swap_smoke
        && !cli.vendor_smoke
        && !cli.equipment_set_race_smoke
        && !cli.rested_xp_smoke
        && !cli.loot_race_smoke
        && !cli.loot_item_capture
        && !cli.group_capacity_race_smoke
    {
        cleanup_bot_group_state(&bots)?;
    }

    let expected_bot_count = if cli.loot_item_capture { 1 } else { bots.len() };
    let mut results = Vec::new();
    if cli.loot_item_capture {
        let shutdown = install_loot_termination_token()?;
        results = finish_guarded_loot_result(
            loot_race::run_single_item_capture_workflow(
                bots,
                loot_race_cli,
                dungeon_id,
                timeout_secs,
                auto_teleport,
                shutdown,
            )
            .await,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.loot_race_smoke {
        let shutdown = install_loot_termination_token()?;
        results = finish_guarded_loot_result(
            loot_race::run_workflow(
                bots,
                loot_race_cli,
                dungeon_id,
                timeout_secs,
                auto_teleport,
                shutdown,
            )
            .await,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.group_capacity_race_smoke {
        let shutdown = install_loot_termination_token()?;
        results = loot_race::run_group_capacity_workflow(
            bots,
            group_capacity_cli,
            dungeon_id,
            timeout_secs,
            auto_teleport,
            shutdown,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.equipment_set_race_smoke {
        results = run_equipment_set_race_workflow(
            bots,
            dungeon_id,
            timeout_secs,
            auto_teleport,
            cli.equipment_set_account_a.clone(),
            cli.equipment_set_account_b.clone(),
            cli.equipment_set_timeout_secs,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.sequential || bots.len() == 1 {
        for bot in bots {
            info!("\n[Bot {}] Starting...", bot.account);
            let run = if cli.bank_smoke {
                run_bank_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.bank_item_entry,
                    cli.bank_runtime_counter,
                    cli.bank_timeout_secs,
                )
                .await
            } else if cli.void_storage_smoke {
                run_void_storage_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.void_storage_item_entry,
                    cli.void_storage_runtime_counter,
                    cli.void_storage_timeout_secs,
                )
                .await
            } else if cli.void_storage_query_capture {
                run_void_storage_query_capture_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.void_storage_item_entry,
                    cli.void_storage_runtime_counter,
                    cli.void_storage_timeout_secs,
                )
                .await
            } else if cli.homebind_smoke {
                run_homebind_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.homebind_runtime_counter,
                    cli.homebind_timeout_secs,
                )
                .await
            } else if cli.inventory_swap_smoke {
                run_inventory_swap_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.inventory_swap_item_entry_a,
                    cli.inventory_swap_item_entry_b,
                    cli.inventory_swap_timeout_secs,
                )
                .await
            } else if cli.vendor_smoke {
                run_vendor_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.vendor_entry,
                    cli.vendor_spawn_guid,
                    cli.vendor_runtime_counter,
                    cli.vendor_item_entry,
                    cli.vendor_extended_cost,
                    cli.vendor_currency_id,
                    cli.vendor_currency_cost,
                    cli.vendor_currency_quantity,
                    cli.vendor_timeout_secs,
                )
                .await
            } else if cli.rested_xp_smoke {
                run_rested_xp_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.rested_xp_creature_entry,
                    cli.rested_xp_creature_guid,
                    cli.rested_xp_runtime_counter,
                    cli.rested_xp_offline_secs,
                    cli.rested_xp_timeout_secs,
                )
                .await
            } else {
                run_bot(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.login_only,
                    stand_state_options.clone(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    quest_options.clone(),
                )
                .await
            };
            match run {
                Ok(result) => {
                    log_bot_summary(&result, require_proposal, require_group, cli.login_only);
                    results.push(result);
                }
                Err(e) => error!("❌ Bot run ERROR: {}", e),
            }
        }
    } else {
        let mut handles = Vec::new();
        for (idx, bot) in bots.into_iter().enumerate() {
            let delay_ms = app_config
                .test_config
                .launch_delay_ms
                .saturating_mul(idx as u64);
            let quest_options_for_bot = quest_options.clone();
            let stand_state_options_for_bot = stand_state_options.clone();
            handles.push(tokio::spawn(async move {
                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                let account = bot.account.clone();
                let run = run_bot(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.login_only,
                    stand_state_options_for_bot,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    quest_options_for_bot,
                )
                .await;
                (account, run)
            }));
        }

        for handle in handles {
            match handle.await {
                Ok((_account, Ok(result))) => {
                    log_bot_summary(&result, require_proposal, require_group, cli.login_only);
                    results.push(result);
                }
                Ok((account, Err(e))) => error!("❌ Bot {} ERROR: {}", account, e),
                Err(e) => error!("❌ Bot task join error: {}", e),
            }
        }
    }

    results.sort_by_key(|r| r.account_id);
    write_report_if_requested(
        &cli,
        dungeon_id,
        timeout_secs,
        require_proposal,
        require_group,
        auto_teleport,
        cli.login_only,
        cli.stand_state_smoke,
        cli.bank_smoke,
        cli.void_storage_smoke,
        cli.void_storage_query_capture,
        cli.homebind_smoke,
        cli.inventory_swap_smoke,
        cli.vendor_smoke,
        cli.equipment_set_race_smoke,
        cli.rested_xp_smoke,
        cli.loot_race_smoke,
        cli.loot_item_capture,
        cli.group_capacity_race_smoke,
        cli.quest_smoke,
        &results,
    )?;

    let assertion_failures = results
        .iter()
        .filter(|result| !result.success(require_proposal, require_group, cli.login_only))
        .count();
    let task_failures = expected_bot_count.saturating_sub(results.len());
    let failures = assertion_failures + task_failures;
    if failures > 0 {
        bail!(
            "{} bot(s) failed ({} task errors, {} assertion failures)",
            failures,
            task_failures,
            assertion_failures
        );
    }

    info!("\n🎯 All tests completed");
    Ok(())
}

fn stand_state_smoke_options_from_cli(cli: &CliOptions) -> Result<StandStateSmokeOptions> {
    if cli.stand_state_timeout_secs == 0 {
        bail!("--stand-state-timeout must be greater than zero");
    }

    let states = match cli.stand_state {
        Some(state) if is_client_stand_state_like_cpp(state) => vec![state],
        Some(state) => bail!(
            "unsupported stand state {state}; expected 0 (Stand), 1 (Sit), 3 (Sleep), or 8 (Kneel)"
        ),
        None => vec![UNIT_STAND_STATE_SIT, UNIT_STAND_STATE_STAND],
    };

    Ok(StandStateSmokeOptions {
        states,
        timeout_secs: cli.stand_state_timeout_secs,
    })
}

fn is_client_stand_state_like_cpp(state: u8) -> bool {
    matches!(
        state,
        UNIT_STAND_STATE_STAND
            | UNIT_STAND_STATE_SIT
            | UNIT_STAND_STATE_SLEEP
            | UNIT_STAND_STATE_KNEEL
    )
}

fn quest_smoke_options_from_cli(cli: &CliOptions) -> Result<QuestSmokeOptions> {
    let creature_entry = cli.quest_creature_entry.ok_or_else(|| {
        anyhow!("--quest-smoke requires --quest-creature-entry or WOW_BOT_QUEST_CREATURE_ENTRY")
    })?;
    if (cli.quest_accept || cli.quest_reset) && cli.quest_expected_id.is_none() {
        bail!("--quest-accept/--quest-reset require --expect-quest or WOW_BOT_QUEST_EXPECT_ID");
    }
    if cli.quest_objective_persist {
        if cli.quest_expected_id.is_none() {
            bail!("--quest-objective-persist requires --expect-quest or WOW_BOT_QUEST_EXPECT_ID");
        }
        if cli.quest_objectives.is_empty() {
            bail!("--quest-objective-persist requires --quest-objectives storage:data");
        }
    }
    if let Some(level) = cli.quest_set_level {
        if !(1..=80).contains(&level) {
            bail!("--quest-set-level must be in the 1..=80 player level range");
        }
    }
    if matches!(cli.quest_set_race, Some(0)) {
        bail!("--quest-set-race must be nonzero");
    }
    if matches!(cli.quest_set_class, Some(0)) {
        bail!("--quest-set-class must be nonzero");
    }

    Ok(QuestSmokeOptions {
        creature_entry,
        creature_spawn_guid: cli.quest_creature_guid,
        creature_guid_counter: cli.quest_guid_counter,
        map_id: cli.quest_map_id,
        expected_quest_id: cli.quest_expected_id,
        forbidden_quest_id: cli.quest_forbidden_id,
        forbidden_title_contains: cli.quest_forbidden_title.clone(),
        query_details: cli.quest_query_details || cli.quest_accept,
        accept: cli.quest_accept,
        reset_before_run: cli.quest_reset,
        relocate_before_login: cli.quest_relocate,
        set_level_before_login: cli.quest_set_level,
        set_race_before_login: cli.quest_set_race,
        set_class_before_login: cli.quest_set_class,
        objective_persist: cli.quest_objective_persist,
        objective_seed: cli.quest_objectives.clone(),
        objective_status: cli.quest_objective_status,
        gossip_select_option_id: cli.gossip_select_option_id,
        expect_trainer_list: cli.expect_trainer_list,
        expect_trainer_id: cli.expect_trainer_id,
        timeout_secs: cli.quest_timeout_secs,
    })
}

fn quest_smoke_needs_prelogin_db_setup(quest_options: &QuestSmokeOptions) -> bool {
    quest_options.reset_before_run
        || quest_options.relocate_before_login
        || quest_options.set_level_before_login.is_some()
        || quest_options.set_race_before_login.is_some()
        || quest_options.set_class_before_login.is_some()
        || quest_options.objective_persist
}

/// Run a single bot through the full SRP6 → World → LFG flow
async fn run_bot(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    login_only: bool,
    stand_state_options: Option<StandStateSmokeOptions>,
    bank_options: Option<BankSmokeOptions>,
    homebind_options: Option<HomebindSmokeOptions>,
    inventory_swap_options: Option<InventorySwapSmokeOptions>,
    vendor_options: Option<VendorSmokeOptions>,
    rested_xp_options: Option<RestedXpSmokeOptions>,
    loot_race_options: Option<loot_race::LootRaceOptions>,
    group_capacity_options: Option<loot_race::GroupCapacityRaceOptions>,
    equipment_set_options: Option<EquipmentSetSmokeOptions>,
    quest_options: Option<QuestSmokeOptions>,
) -> Result<BotRunResult> {
    run_bot_with_void_storage(
        bot,
        dungeon_id,
        lfg_secs,
        auto_teleport,
        login_only,
        stand_state_options,
        bank_options,
        homebind_options,
        inventory_swap_options,
        vendor_options,
        rested_xp_options,
        loot_race_options,
        group_capacity_options,
        equipment_set_options,
        quest_options,
        None,
    )
    .await
}

async fn run_bot_with_void_storage(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    login_only: bool,
    stand_state_options: Option<StandStateSmokeOptions>,
    bank_options: Option<BankSmokeOptions>,
    homebind_options: Option<HomebindSmokeOptions>,
    inventory_swap_options: Option<InventorySwapSmokeOptions>,
    vendor_options: Option<VendorSmokeOptions>,
    rested_xp_options: Option<RestedXpSmokeOptions>,
    loot_race_options: Option<loot_race::LootRaceOptions>,
    group_capacity_options: Option<loot_race::GroupCapacityRaceOptions>,
    equipment_set_options: Option<EquipmentSetSmokeOptions>,
    quest_options: Option<QuestSmokeOptions>,
    mut void_storage_options: Option<VoidStorageSmokeOptions>,
) -> Result<BotRunResult> {
    let bot_index = bot.account_id as usize;
    let void_storage_query_capture = void_storage_options
        .as_ref()
        .is_some_and(|options| options.phase == VoidStorageSmokePhase::QueryCapture);
    let mut result = BotRunResult {
        account: bot.account.clone(),
        account_id: bot.account_id,
        character_guid: bot.character_guid,
        dungeon_id,
        role: bot.lfg_role,
        join_result: None,
        join_detail: None,
        got_proposal: false,
        accepted_proposal: false,
        got_ready_check: false,
        group_formed: false,
        teleport_denied_reason: None,
        entered_world: false,
        world_auth: false,
        enum_characters: false,
        player_login_verified: false,
        login_only,
        stand_state_smoke: stand_state_options.is_some(),
        stand_state_smoke_passed: None,
        stand_states_requested: stand_state_options
            .as_ref()
            .map(|options| options.states.clone())
            .unwrap_or_default(),
        stand_states_confirmed: Vec::new(),
        stand_state_failure: None,
        bank_smoke: bank_options.is_some(),
        bank_smoke_passed: None,
        bank_banker_entry: bank_options.as_ref().map(|options| options.banker.entry),
        bank_banker_spawn_guid: bank_options
            .as_ref()
            .map(|options| options.banker.spawn_guid),
        bank_banker_guid_counter: bank_options
            .as_ref()
            .map(|options| options.banker.guid_counter),
        bank_item_guid: bank_options.as_ref().map(|options| options.item_guid),
        bank_item_entry: bank_options.as_ref().map(|options| options.item_entry),
        bank_inventory_slot: bank_options.as_ref().map(|options| options.inventory_slot),
        bank_bank_slot: bank_options.as_ref().map(|options| options.bank_slot),
        bank_open_confirmed: false,
        bank_deposit_persisted: false,
        bank_relogin_after_deposit: false,
        bank_withdraw_persisted: false,
        bank_failure: None,
        void_storage_smoke: void_storage_options.is_some() && !void_storage_query_capture,
        void_storage_smoke_passed: None,
        void_storage_query_capture,
        void_storage_query_capture_passed: None,
        void_storage_unlock_persisted: false,
        void_storage_deposit_persisted: false,
        void_storage_deposit_relogin_verified: false,
        void_storage_swap_persisted: false,
        void_storage_swap_relogin_verified: false,
        void_storage_withdraw_persisted: false,
        void_storage_withdraw_relogin_verified: false,
        void_storage_item_id: void_storage_options
            .as_ref()
            .and_then(|options| options.expected_void_item_id),
        void_storage_failure: None,
        homebind_smoke: homebind_options.is_some(),
        homebind_smoke_passed: None,
        homebind_innkeeper_entry: homebind_options
            .as_ref()
            .map(|options| options.innkeeper.entry),
        homebind_innkeeper_spawn_guid: homebind_options
            .as_ref()
            .map(|options| options.innkeeper.spawn_guid),
        homebind_innkeeper_guid_counter: homebind_options
            .as_ref()
            .map(|options| options.innkeeper.guid_counter),
        homebind_spell_go_seen: false,
        homebind_bind_point_update_seen: false,
        homebind_player_bound_seen: false,
        homebind_gossip_complete_seen: false,
        homebind_db_persisted: false,
        homebind_relogin_verified: false,
        homebind_failure: None,
        inventory_swap_smoke: inventory_swap_options.is_some(),
        inventory_swap_smoke_passed: None,
        inventory_swap_item_guid_a: inventory_swap_options
            .as_ref()
            .map(|options| options.item_guid_a),
        inventory_swap_item_guid_b: inventory_swap_options
            .as_ref()
            .map(|options| options.item_guid_b),
        inventory_swap_item_entry_a: inventory_swap_options
            .as_ref()
            .map(|options| options.item_entry_a),
        inventory_swap_item_entry_b: inventory_swap_options
            .as_ref()
            .map(|options| options.item_entry_b),
        inventory_swap_slot_a: inventory_swap_options
            .as_ref()
            .map(|options| options.slot_a),
        inventory_swap_slot_b: inventory_swap_options
            .as_ref()
            .map(|options| options.slot_b),
        inventory_swap_forward_persisted: false,
        inventory_swap_relogin_after_forward: false,
        inventory_swap_reverse_persisted: false,
        inventory_swap_failure: None,
        vendor_smoke: vendor_options.is_some(),
        vendor_smoke_passed: None,
        vendor_entry: vendor_options.as_ref().map(|options| options.vendor.entry),
        vendor_spawn_guid: vendor_options
            .as_ref()
            .map(|options| options.vendor.spawn_guid),
        vendor_runtime_counter: vendor_options.as_ref().and_then(|options| {
            (options.vendor.guid_counter != 0).then_some(options.vendor.guid_counter)
        }),
        vendor_item_entry: vendor_options.as_ref().map(|options| options.item_entry),
        vendor_extended_cost: vendor_options.as_ref().map(|options| options.extended_cost),
        vendor_currency_id: vendor_options.as_ref().map(|options| options.currency_id),
        vendor_currency_before: vendor_options
            .as_ref()
            .map(|options| options.currency_before),
        vendor_currency_after: None,
        vendor_item_total_after: None,
        vendor_inventory_seen: false,
        vendor_buy_succeeded_seen: false,
        vendor_set_currency_seen: false,
        vendor_item_push_seen: false,
        vendor_relogin_verified: false,
        vendor_failure: None,
        equipment_set_smoke: equipment_set_options.is_some(),
        equipment_set_smoke_passed: None,
        equipment_set_type: equipment_set_options
            .as_ref()
            .map(|options| options.set_type),
        equipment_set_id: equipment_set_options.as_ref().map(|options| options.set_id),
        equipment_set_generated_guid: equipment_set_options
            .as_ref()
            .and_then(|options| options.expected_guid),
        equipment_set_login_count: None,
        equipment_set_load_seen: false,
        equipment_set_db_persisted: false,
        equipment_set_relogin_verified: false,
        equipment_set_failure: None,
        rested_xp_smoke: rested_xp_options.is_some(),
        rested_xp_smoke_passed: None,
        rested_xp_offline_wilderness_bonus: None,
        rested_xp_offline_resting_bonus: None,
        rested_xp_target_entry: rested_xp_options
            .as_ref()
            .map(|options| options.target.entry),
        rested_xp_target_spawn_guid: rested_xp_options
            .as_ref()
            .map(|options| options.target.spawn_guid),
        rested_xp_target_guid_counter: rested_xp_options.as_ref().and_then(|options| {
            (options.target.guid_counter != 0).then_some(options.target.guid_counter)
        }),
        rested_xp_packet_amount: None,
        rested_xp_packet_original: None,
        rested_xp_db_xp_before: None,
        rested_xp_db_xp_after: None,
        rested_xp_db_rest_before: None,
        rested_xp_db_rest_after: None,
        rested_xp_relog_verified: false,
        rested_xp_failure: None,
        loot_race_smoke: loot_race_options.is_some(),
        loot_race_smoke_passed: None,
        loot_race_target_entry: loot_race_options
            .as_ref()
            .map(|options| options.target.entry),
        loot_race_target_spawn_guid: loot_race_options
            .as_ref()
            .map(|options| options.target.spawn_guid),
        loot_race_target_runtime_counter: loot_race_options
            .as_ref()
            .and_then(|options| options.resolved_runtime_counter().ok()),
        loot_race_party_confirmed: false,
        loot_race_target_discovered: false,
        loot_race_loot_opened: false,
        loot_race_loot_list_id: None,
        loot_race_loot_coins: None,
        loot_race_item_push_seen: false,
        loot_race_loot_removed_seen: false,
        loot_race_money_notify_amount: None,
        loot_race_coin_removed_seen: false,
        loot_race_db_item_total: None,
        loot_race_db_money_delta: None,
        loot_race_relog_verified: false,
        loot_race_failure: None,
        group_capacity_race_smoke: group_capacity_options.is_some(),
        group_capacity_race_smoke_passed: None,
        group_capacity_group_id: group_capacity_options
            .as_ref()
            .map(|options| options.group_db_store_id),
        group_capacity_outcome: None,
        group_capacity_final_member_count: None,
        group_capacity_failure: None,
        quest_smoke: quest_options.is_some(),
        quest_smoke_passed: None,
        quest_target_entry: None,
        quest_target_spawn_guid: None,
        quest_target_guid_counter: None,
        quest_target_map_id: None,
        quest_gossip_hello_sent: false,
        quest_questgiver_hello_sent: false,
        quest_gossip_id_seen: None,
        quest_gossip_select_sent: false,
        quest_gossip_message_seen: false,
        quest_quest_list_seen: false,
        quest_details_seen: false,
        quest_request_items_seen: false,
        trainer_list_seen: false,
        trainer_id_seen: None,
        trainer_spell_count_seen: None,
        quest_accept_sent: false,
        quest_accept_confirm_seen: false,
        quest_db_verified: false,
        quest_db_status: None,
        quest_objective_persist: quest_options
            .as_ref()
            .is_some_and(|options| options.objective_persist),
        quest_objective_seeded: quest_options
            .as_ref()
            .map(|options| options.objective_seed.clone())
            .unwrap_or_default(),
        quest_objective_db_before: Vec::new(),
        quest_objective_db_after: Vec::new(),
        quest_objective_db_verified: false,
        quest_objective_update_seen: false,
        quest_objective_update_has_expected: false,
        quest_ids_seen: Vec::new(),
        quest_titles_seen: Vec::new(),
        quest_failure: None,
        seen_opcodes: Vec::new(),
    };

    // ── Step 1: Prepare the World session key ────────────────────────────────
    // The guarded equipment-set QA writes a fresh 64-byte fixture key for each
    // verified disposable account. Group-capacity QA may instead reuse a
    // configured 64-byte fixture key. Otherwise live BNet SRP6 computes
    // (login_ticket, K_32), where
    // K = SHA256(broken_evidence_le(S)), and expands it to K || SHA256(K).
    // Either path writes account.session_key_bnet before CMSG_AUTH_SESSION.
    info!(
        "[Bot {}] Step 1: Preparing World session key (configured group fixture or live SRP6 via {}:{})",
        bot_index,
        bnet_host(),
        bnet_port()
    );

    let configured_group_session_key = group_capacity_options
        .as_ref()
        .filter(|_| !bot.session_key_bnet.trim().is_empty())
        .map(|_| {
            hex::decode(bot.session_key_bnet.trim()).map_err(|error| {
                anyhow!(
                    "Configured group-capacity session_key_bnet for {} is not hex: {error}",
                    bot.account
                )
            })
        })
        .transpose()?;
    let generated_equipment_session_key = equipment_set_options.as_ref().map(|_| {
        let mut session_key = vec![0u8; 64];
        rand::thread_rng().fill_bytes(&mut session_key);
        session_key
    });
    let (session_key, used_fixture_session_key) =
        if let Some(session_key) = generated_equipment_session_key {
            (session_key, true)
        } else if let Some(session_key) = configured_group_session_key {
            if session_key.len() != 64 {
                bail!(
                    "Configured group-capacity session_key_bnet for {} has {} bytes, expected 64",
                    bot.account,
                    session_key.len()
                );
            }
            (session_key, true)
        } else {
            // Rusty's current BNet bot endpoint keeps challenge state that
            // cannot safely serve multiple fallback logins at once. Serialize
            // only this authentication exchange; all World connections and
            // the actual group accept race remain concurrent.
            let group_capacity_auth_guard = if let Some(options) = group_capacity_options.as_ref() {
                Some(options.auth_serial.lock().await)
            } else {
                None
            };
            let bnet_url = format!("https://{}:{}", bnet_host(), bnet_port());
            let (login_ticket, session_key_32) =
                bot_srp6::authenticate_bot(&bnet_url, &bot.account, &bot.password)
                    .await
                    .map_err(|e| anyhow!("Bot SRP6 failed: {}", e))?;
            drop(group_capacity_auth_guard);
            let _ = login_ticket; // BNet proof only; World auth uses the derived key.
            if session_key_32.len() != 32 {
                bail!(
                    "Bot SRP6 returned K of unexpected length: {}",
                    session_key_32.len()
                );
            }
            (expand_session_key(&session_key_32).to_vec(), false)
        };

    let account_for_db = bot.account.clone();
    let session_key_for_db = session_key.clone();
    let realm_id_for_db = realm_id();
    let world_auth_context = tokio::task::spawn_blocking(move || {
        prepare_world_auth_context(&account_for_db, &session_key_for_db, realm_id_for_db)
    })
    .await
    .map_err(|e| anyhow!("DB worker join failed for {}: {}", bot.account, e))?
    .map_err(|e| {
        anyhow!(
            "Failed to prepare world auth context for {}: {}",
            bot.account,
            e
        )
    })?;
    let wow_username = world_auth_context.username.clone();

    if used_fixture_session_key {
        info!(
            "[Bot {}] ✅ guarded fixture session key prepared (64B)",
            bot_index
        );
    } else {
        info!("[Bot {}] ✅ LoginTicket received", bot_index);
        info!("[Bot {}] ✅ K (live, 32B) received", bot_index);
    }
    info!(
        "[Bot {}] ✅ session_key_bnet (64B) written to account `{}`; realm build {} auth seed loaded",
        bot_index, wow_username, world_auth_context.realm_build
    );

    if let Some(quest_options) = &quest_options {
        if quest_smoke_needs_prelogin_db_setup(quest_options) {
            let bot_for_db = bot.clone();
            let options_for_db = quest_options.clone();
            tokio::task::spawn_blocking(move || {
                prepare_quest_smoke_before_login(&bot_for_db, &options_for_db)
            })
            .await
            .map_err(|e| anyhow!("Quest smoke setup DB worker join failed: {}", e))??;
        }
        if quest_options.objective_persist {
            let bot_for_db = bot.clone();
            let quest_id = quest_options
                .expected_quest_id
                .ok_or_else(|| anyhow!("Objective persistence requested without quest id"))?;
            result.quest_objective_db_before = tokio::task::spawn_blocking(move || {
                load_bot_quest_objectives(&bot_for_db, quest_id)
            })
            .await
            .map_err(|e| anyhow!("Quest objective before-load worker join failed: {}", e))??;
        }
    }

    // ── Step 2: Connect to World Server ─────────────────────────────────────
    info!(
        "[Bot {}] Step 2: Connecting to World Server {}:{}",
        bot_index,
        world_host(),
        world_port()
    );
    let world_addr = format!("{}:{}", world_host(), world_port());
    let mut stream =
        tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, TcpStream::connect(&world_addr))
            .await
            .map_err(|_| anyhow!("Timed out connecting to world server {world_addr}"))?
            .map_err(|e| anyhow!("Failed to connect to world server: {}", e))?;
    info!("[Bot {}] ✅ TCP connected", bot_index);

    // ── Step 3: World Server Handshake ──────────────────────────────────────
    info!("[Bot {}] Step 3: Handshake...", bot_index);
    let mut init_buf = vec![0u8; 256];
    let n = tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, stream.read(&mut init_buf))
        .await
        .map_err(|_| anyhow!("Timed out reading SERVER_INIT"))??;
    if !init_buf[..n].starts_with(&SERVER_INIT[..SERVER_INIT.len().min(n)]) {
        bail!(
            "Unexpected server init: {:?}",
            String::from_utf8_lossy(&init_buf[..n])
        );
    }
    info!("[Bot {}] ✅ SERVER_INIT received", bot_index);

    tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, async {
        stream.write_all(CLIENT_INIT).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| anyhow!("Timed out writing CLIENT_INIT"))??;
    info!("[Bot {}] ✅ CLIENT_INIT sent", bot_index);

    // ── Step 4: Read SMSG_AUTH_CHALLENGE ────────────────────────────────────
    info!("[Bot {}] Step 4: Reading SMSG_AUTH_CHALLENGE...", bot_index);
    let (opcode, challenge_data) = tokio::time::timeout(
        INITIAL_NETWORK_IO_TIMEOUT,
        read_unencrypted_packet(&mut stream),
    )
    .await
    .map_err(|_| anyhow!("Timed out reading SMSG_AUTH_CHALLENGE"))??;
    if opcode != 0x3048 {
        bail!(
            "Expected SMSG_AUTH_CHALLENGE (0x3048), got 0x{:04X}",
            opcode
        );
    }
    if challenge_data.len() < 48 {
        bail!(
            "SMSG_AUTH_CHALLENGE too short: {} bytes",
            challenge_data.len()
        );
    }
    let server_challenge: [u8; 16] = challenge_data[32..48].try_into()?;
    info!("[Bot {}] ✅ SMSG_AUTH_CHALLENGE received", bot_index);

    // ── Step 5: Send CMSG_AUTH_SESSION ──────────────────────────────────────
    info!(
        "[Bot {}] Step 5: Sending CMSG_AUTH_SESSION (build={})...",
        bot_index,
        client_build()
    );
    let local_challenge: [u8; 16] = rand::random();
    let digest = compute_auth_digest(
        &local_challenge,
        &server_challenge,
        &session_key,
        &world_auth_context.win64_auth_seed,
    );
    let derived_session_key =
        derive_realm_session_key(&session_key, &local_challenge, &server_challenge);

    // RealmJoinTicket on the worldserver side is the WoW account name (account.username),
    // World auth uses the game-account username and `session_key_bnet`, not a
    // BNet login ticket (sending that ticket yields "unknown account").
    let auth_data = build_cmsg_auth_session(realm_id(), &local_challenge, &digest, &wow_username);
    send_unencrypted_packet(&mut stream, 0x3765, &auth_data).await?;
    info!("[Bot {}] ✅ CMSG_AUTH_SESSION sent", bot_index);

    // ── Step 6: Wait for SMSG_AUTH_RESPONSE & encryption activation ─────────
    info!(
        "[Bot {}] Step 6: Waiting for auth response & encryption...",
        bot_index
    );
    let mut world_crypt: Option<WorldCrypt> = None;
    let mut encrypted = false;

    for _ in 0..10 {
        match tokio::time::timeout(Duration::from_secs(5), read_unencrypted_packet(&mut stream))
            .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);

                if op == 0x256D {
                    // SMSG_AUTH_RESPONSE
                    info!("[Bot {}] ✅ SMSG_AUTH_RESPONSE received", bot_index);
                } else if op == 0x3049 {
                    // SMSG_ENTER_ENCRYPTED_MODE
                    info!("[Bot {}] ✅ SMSG_ENTER_ENCRYPTED_MODE received", bot_index);

                    let enc_key =
                        derive_encryption_key(&session_key, &local_challenge, &server_challenge);
                    info!(
                        "[Bot {}] Encryption key derived: {:02x}{:02x}...",
                        bot_index, enc_key[0], enc_key[1]
                    );

                    // Server's WorldPacketCrypt increments _clientCounter / _serverCounter
                    // on every packet — including the unencrypted SMSG_AUTH_CHALLENGE,
                    // SMSG_ENTER_ENCRYPTED_MODE, CMSG_AUTH_SESSION, and CMSG_ENTER_ENCRYPTED_MODE_ACK
                    // exchanges that happen before _authCrypt.Init() is called. By the
                    // time the first AES-GCM packet flies, both counters are at 2.
                    world_crypt = Some(WorldCrypt::new_with_counters(&enc_key, 2, 2));
                    encrypted = true;

                    // Send ACK
                    send_unencrypted_packet(&mut stream, 0x3767, &[]).await?;
                    info!("[Bot {}] ✅ CMSG_ENTER_ENCRYPTED_MODE_ACK sent", bot_index);
                    break;
                } else if op == 0x256E {
                    // SMSG_AUTH_RESPONSE (error variant)
                    warn!(
                        "[Bot {}] ⚠️ Auth response error code: {:?}",
                        bot_index,
                        payload.get(0)
                    );
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Error reading packet: {}", bot_index, e);
                break;
            }
            Err(_) => {
                warn!("[Bot {}] Timeout waiting for encryption", bot_index);
                break;
            }
        }
    }

    if !encrypted {
        bail!("Encryption not established");
    }
    result.world_auth = true;
    let mut crypt = world_crypt.take().unwrap();
    let mut server_inflater = ServerPacketInflater::default();
    let mut realm_connection: Option<EncryptedWorldConnection> = None;

    // ── Step 7a: Enumerate Characters ──────────────────────────────────────
    // The worldserver gates HandlePlayerLoginOpcode on `_legitCharacters` being
    // populated, which only happens after CMSG_ENUM_CHARACTERS is processed.
    // Skipping this step results in "Trying to login with a character of another account".
    info!(
        "[Bot {}] Step 7a: Sending CMSG_ENUM_CHARACTERS...",
        bot_index
    );
    send_encrypted_packet(&mut stream, &mut crypt, 0x35E9, &[]).await?;

    let mut enum_ok = false;
    for _ in 0..20 {
        match tokio::time::timeout(
            Duration::from_secs(3),
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, _payload))) => {
                if op == 0x2583 {
                    // SMSG_ENUM_CHARACTERS_RESULT
                    info!(
                        "[Bot {}] ✅ SMSG_ENUM_CHARACTERS_RESULT received",
                        bot_index
                    );
                    enum_ok = true;
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Enum read error: {}", bot_index, e);
                break;
            }
            Err(_) => { /* fall through to retry */ }
        }
    }
    if !enum_ok {
        bail!("Did not receive SMSG_ENUM_CHARACTERS_RESULT");
    }
    result.enum_characters = true;

    // ── Step 7b: Player Login ──────────────────────────────────────────────
    info!(
        "[Bot {}] Step 7b: Sending CMSG_PLAYER_LOGIN (guid={})...",
        bot_index, bot.character_guid
    );
    let login_data = build_player_login(bot.character_guid, realm_id(), 500.0);
    send_encrypted_packet(&mut stream, &mut crypt, 0x35EB, &login_data).await?;
    info!("[Bot {}] ✅ CMSG_PLAYER_LOGIN sent", bot_index);

    let mut login_ok = false;
    let preserve_realm_connection = stand_state_options.is_some()
        || homebind_options.is_some()
        || vendor_options.is_some()
        || rested_xp_options.is_some()
        || loot_race_options.is_some()
        || group_capacity_options.is_some()
        || equipment_set_options.is_some()
        || void_storage_options.is_some();
    let mut loot_race_target_seen = false;
    let mut vendor_target_seen: Option<DiscoveredCreatureGuid> = None;
    let mut void_storage_target_seen: Option<DiscoveredCreatureGuid> = None;
    let login_budget = LoginVerifyBudget::new(LOGIN_VERIFY_TIMEOUT);
    while let Some(read_timeout) = login_budget.next_read_timeout() {
        match tokio::time::timeout(
            read_timeout,
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                if let Some(options) = loot_race_options.as_ref() {
                    if let Some(counter) = loot_race::target_seen_in_update(options, op, &payload)?
                    {
                        loot_race_target_seen = true;
                        result.loot_race_target_runtime_counter = Some(counter);
                    }
                }
                if let Some(options) = vendor_options.as_ref() {
                    let candidate = (op == SMSG_UPDATE_OBJECT)
                        .then(|| {
                            find_creature_guid_near_position_in_update_object(
                                &payload,
                                options.vendor.map_id,
                                options.vendor.entry,
                                options.vendor.x as f32,
                                options.vendor.y as f32,
                                options.vendor.z as f32,
                                options.target_match_radius,
                                (options.vendor.guid_counter != 0).then_some(
                                    options.vendor.guid_counter & OBJECT_GUID_COUNTER_MASK,
                                ),
                            )
                        })
                        .flatten();
                    if let Some(candidate) = candidate {
                        match vendor_target_seen {
                            Some(previous)
                                if (previous.low, previous.high)
                                    != (candidate.low, candidate.high) =>
                            {
                                bail!(
                                    "vendor login discovery produced two different live candidates near SQL spawn {}",
                                    options.vendor.spawn_guid
                                );
                            }
                            _ => vendor_target_seen = Some(candidate),
                        }
                    }
                }
                if let Some(options) = void_storage_options.as_ref() {
                    let expected_counter = (!options.discover_runtime_guid)
                        .then_some(options.vault_keeper.guid_counter & OBJECT_GUID_COUNTER_MASK);
                    let candidate = (op == SMSG_UPDATE_OBJECT)
                        .then(|| {
                            find_creature_guid_near_position_in_update_object(
                                &payload,
                                options.vault_keeper.map_id,
                                options.vault_keeper.entry,
                                options.vault_keeper.x as f32,
                                options.vault_keeper.y as f32,
                                options.vault_keeper.z as f32,
                                10.0,
                                expected_counter,
                            )
                        })
                        .flatten();
                    if let Some(candidate) = candidate {
                        match void_storage_target_seen {
                            Some(previous)
                                if (previous.low, previous.high)
                                    != (candidate.low, candidate.high) =>
                            {
                                bail!(
                                    "void-storage login discovery produced two different live candidates near SQL spawn {}",
                                    options.vault_keeper.spawn_guid
                                );
                            }
                            _ => void_storage_target_seen = Some(candidate),
                        }
                    }
                }
                if let Some(options) = quest_options.as_ref() {
                    record_quest_objective_login_signal(op, &payload, options, &mut result);
                }
                if let Some(options) = equipment_set_options.as_ref() {
                    record_equipment_set_login_signal(op, &payload, options, &mut result)?;
                }
                if op == 0x2597 {
                    // SMSG_LOGIN_VERIFY_WORLD
                    info!("[Bot {}] ✅ SMSG_LOGIN_VERIFY_WORLD received", bot_index);
                    login_ok = true;
                    let equipment_set_login_ready = equipment_set_options
                        .as_ref()
                        .is_none_or(|_| result.equipment_set_load_seen);
                    let void_storage_login_ready =
                        void_storage_options.as_ref().is_none_or(|options| {
                            void_storage_login_target_ready(
                                options.discover_runtime_guid,
                                void_storage_target_seen.is_some(),
                            )
                        });
                    if (!preserve_realm_connection || realm_connection.is_some())
                        && equipment_set_login_ready
                        && void_storage_login_ready
                    {
                        break;
                    }
                    // Routing-sensitive captures validate both connections, so
                    // keep reading the realm socket until SMSG_CONNECT_TO has
                    // created and authenticated a distinct instance socket.
                    continue;
                }

                if op == 0x304D {
                    let connect_to = parse_connect_to(&payload)
                        .ok_or_else(|| anyhow!("Unable to parse SMSG_CONNECT_TO payload"))?;
                    info!(
                        "[Bot {}] SMSG_CONNECT_TO: {}:{} serial={} con={} key={}",
                        bot_index,
                        connect_to.address,
                        connect_to.port,
                        connect_to.serial,
                        connect_to.connection_type,
                        connect_to.key
                    );
                    let (instance_stream, instance_crypt) =
                        connect_to_instance(bot_index, &connect_to, &derived_session_key).await?;
                    if preserve_realm_connection {
                        if realm_connection.is_some() {
                            bail!("Routing smoke received more than one SMSG_CONNECT_TO");
                        }
                        let realm_stream = std::mem::replace(&mut stream, instance_stream);
                        let realm_crypt = std::mem::replace(&mut crypt, instance_crypt);
                        let realm_inflater = std::mem::take(&mut server_inflater);
                        realm_connection = Some(EncryptedWorldConnection {
                            stream: realm_stream,
                            crypt: realm_crypt,
                            inflater: realm_inflater,
                        });
                    } else {
                        stream = instance_stream;
                        crypt = instance_crypt;
                        server_inflater = ServerPacketInflater::default();
                    }
                    info!("[Bot {}] ✅ Instance socket authenticated", bot_index);
                    let void_storage_login_ready =
                        void_storage_options.as_ref().is_none_or(|options| {
                            void_storage_login_target_ready(
                                options.discover_runtime_guid,
                                void_storage_target_seen.is_some(),
                            )
                        });
                    if login_ok && preserve_realm_connection && void_storage_login_ready {
                        break;
                    }
                } else if op == 0x304B {
                    // SMSG_RESUME_COMMS
                    info!("[Bot {}] ✅ SMSG_RESUME_COMMS received", bot_index);
                }
                if equipment_set_options.is_some()
                    && login_ok
                    && result.equipment_set_load_seen
                    && (!preserve_realm_connection || realm_connection.is_some())
                {
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Login read error: {}", bot_index, e);
                break;
            }
            // A quiet five-second slice does not invalidate an otherwise
            // healthy login. The absolute deadline above remains the guard.
            Err(_) => continue,
        }
    }
    if !login_ok {
        bail!("Login verification failed");
    }
    result.player_login_verified = true;

    if let Some(stand_state_options) = stand_state_options {
        run_stand_state_smoke(
            bot_index,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &stand_state_options,
            &mut result,
        )
        .await;
        return Ok(result);
    }

    if let Some(quest_options) = quest_options {
        run_quest_smoke(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &quest_options,
            &mut result,
        )
        .await;
        if quest_options.objective_persist {
            if let Err(e) = logout_and_verify_quest_objectives(
                bot_index,
                &bot,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                &quest_options,
                &mut result,
            )
            .await
            {
                result.quest_failure = Some(format!("Quest objective persist QA failed: {e}"));
                result.quest_smoke_passed = Some(false);
            } else {
                result.quest_smoke_passed = Some(quest_smoke_passes(&quest_options, &mut result));
            }
        }
        return Ok(result);
    }

    if let Some(bank_options) = bank_options {
        if let Err(error) = run_bank_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &bank_options,
            &mut result,
        )
        .await
        {
            result.bank_failure = Some(error.to_string());
            result.bank_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(mut void_storage_options) = void_storage_options.take() {
        if void_storage_options.discover_runtime_guid {
            let discovered = void_storage_target_seen.ok_or_else(|| {
                anyhow!(
                    "void-storage vault keeper entry {} spawn {} was not discovered in login object updates",
                    void_storage_options.vault_keeper.entry,
                    void_storage_options.vault_keeper.spawn_guid
                )
            })?;
            void_storage_options.vault_keeper.guid_counter = discovered.low;
            void_storage_options.vault_keeper.packed_guid =
                build_packed_guid(discovered.low, discovered.high);
        }
        if let Err(error) = run_void_storage_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &void_storage_options,
            &mut result,
        )
        .await
        {
            result.void_storage_failure = Some(error.to_string());
            if result.void_storage_query_capture {
                result.void_storage_query_capture_passed = Some(false);
            } else {
                result.void_storage_smoke_passed = Some(false);
            }
        }
        return Ok(result);
    }

    if let Some(homebind_options) = homebind_options {
        if let Err(error) = run_homebind_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &homebind_options,
            &mut result,
        )
        .await
        {
            result.homebind_failure = Some(error.to_string());
            result.homebind_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(inventory_swap_options) = inventory_swap_options {
        if let Err(error) = run_inventory_swap_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &inventory_swap_options,
            &mut result,
        )
        .await
        {
            result.inventory_swap_failure = Some(error.to_string());
            result.inventory_swap_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(vendor_options) = vendor_options {
        if let Err(error) = run_vendor_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &vendor_options,
            vendor_target_seen,
            &mut result,
        )
        .await
        {
            let mut failure = error.to_string();
            if let Err(logout_error) = loot_race::logout_and_wait_routed_like_cpp(
                bot_index,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                realm_connection.as_mut(),
                bot.character_guid,
                &mut result,
            )
            .await
            {
                failure.push_str(&format!(
                    "; graceful logout after failure also failed: {logout_error}"
                ));
            }
            result.vendor_failure = Some(failure);
            result.vendor_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(rested_xp_options) = rested_xp_options {
        if let Err(error) = run_rested_xp_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &rested_xp_options,
            &mut result,
        )
        .await
        {
            result.rested_xp_failure = Some(error.to_string());
            result.rested_xp_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if let Some(loot_race_options) = loot_race_options {
        if let Err(error) = loot_race::run_phase(
            bot_index,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &loot_race_options,
            loot_race_target_seen,
            &mut result,
        )
        .await
        {
            result.loot_race_failure = Some(error.to_string());
            result.loot_race_smoke_passed = Some(false);
            loot_race::best_effort_close(
                bot_index,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                &mut realm_connection,
                loot_race_options.character_guid,
                &mut result,
            )
            .await;
        }
        return Ok(result);
    }

    if let Some(group_capacity_options) = group_capacity_options {
        if let Err(error) = loot_race::run_group_capacity_phase(
            bot_index,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            &mut realm_connection,
            &group_capacity_options,
            &mut result,
        )
        .await
        {
            result.group_capacity_failure = Some(error.to_string());
            result.group_capacity_race_smoke_passed = Some(false);
            loot_race::best_effort_logout_preserving_group(
                bot_index,
                &mut stream,
                &mut crypt,
                &mut server_inflater,
                &mut realm_connection,
                group_capacity_options.character_guid,
                &mut result,
            )
            .await;
        }
        return Ok(result);
    }

    if let Some(equipment_set_options) = equipment_set_options {
        if let Err(error) = run_equipment_set_smoke_phase(
            bot_index,
            &bot,
            &mut stream,
            &mut crypt,
            &mut server_inflater,
            realm_connection.as_mut(),
            &equipment_set_options,
            &mut result,
        )
        .await
        {
            result.equipment_set_failure = Some(error.to_string());
            result.equipment_set_smoke_passed = Some(false);
        }
        return Ok(result);
    }

    if login_only {
        info!(
            "[Bot {}] ✅ Login-only smoke passed: world_auth=true enum_characters=true player_login=true",
            bot_index
        );
        return Ok(result);
    }

    // ── Step 8: LFG Setup ───────────────────────────────────────────────────
    // Opcodes verified against src/server/game/Server/Protocol/Opcodes.h:
    //   CMSG_DF_SET_ROLES = 0x3617    (was incorrectly 0x35EE)
    //   CMSG_DF_JOIN      = 0x360B
    //   CMSG_DF_READY_CHECK_RESPONSE = 0x361C  (was 0x360D)
    //   SMSG_LFG_JOIN_RESULT         = 0x2A1C  (was 0x2F0C)
    //   SMSG_LFG_READY_CHECK_UPDATE  = 0x2A22  (was 0x2F0E)
    info!(
        "[Bot {}] Step 8: Setting up LFG (role={})...",
        bot_index, bot.lfg_role
    );
    tokio::time::sleep(Duration::from_secs(1)).await;

    // CMSG_DF_SET_ROLES wire format (per LFGPackets.cpp::DFSetRoles::Read):
    //   bit:  hasPartyIndex (false here → 0)
    //   u8:   RolesDesired
    // ByteBuffer aligns to byte after bits, so the wire is [bit_byte=0x00, role].
    let role_data = vec![0x00, bot.lfg_role];
    send_encrypted_packet(&mut stream, &mut crypt, 0x3617, &role_data).await?;
    info!("[Bot {}] ✅ CMSG_DF_SET_ROLES sent", bot_index);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // ── Step 9: Join Defense Protocol Alpha (259) ───────────────────────────
    info!(
        "[Bot {}] Step 9: Joining LFG dungeon {}...",
        bot_index, dungeon_id
    );
    let join_data = build_lfg_join(dungeon_id, bot.lfg_role);
    send_encrypted_packet(&mut stream, &mut crypt, 0x360B, &join_data).await?;
    info!("[Bot {}] ✅ CMSG_DF_JOIN sent", bot_index);

    // ── Step 10: Wait for LFG result ────────────────────────────────────────
    let mut result_code = 255u8;
    let mut detail_code = 255u8;

    // Read until we hit the configured overall budget. We stay in the queue past the initial
    // SMSG_LFG_JOIN_RESULT so LFGMgr has time to match all 5 bots, send a proposal,
    // collect accepts, and run the ready check — which is what proves group formation.
    // the queue and needs time to log in, set roles, and accept the proposal.
    info!("[Bot {}] LFG read window: {}s", bot_index, lfg_secs);
    let lfg_deadline = tokio::time::Instant::now() + Duration::from_secs(lfg_secs);
    let mut got_proposal = false;
    let mut group_formed = false;
    while tokio::time::Instant::now() < lfg_deadline {
        let remaining = lfg_deadline - tokio::time::Instant::now();
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);

                if op == 0x2A1C {
                    // SMSG_LFG_JOIN_RESULT
                    // Use the proper parser — the result/detail bytes live AFTER the
                    // RideTicket prefix (PackedGuid + 12 bytes + 1 bit pad), not at offset 0.
                    if let Some(r) = packet_parser::parse_lfg_join_result(&payload) {
                        result_code = r.result;
                        detail_code = r.result_detail;
                        result.join_result = Some(r.result);
                        result.join_detail = Some(r.result_detail);
                        info!(
                            "[Bot {}] 🎯 LFG RESULT: result={}, detail={}",
                            bot_index, result_code, detail_code
                        );
                    }
                }

                if op == 0x2A2D {
                    // SMSG_LFG_PROPOSAL_UPDATE
                    info!(
                        "[Bot {}] 📜 SMSG_LFG_PROPOSAL_UPDATE ({} bytes)",
                        bot_index,
                        payload.len()
                    );
                    if let Some(resp) = build_proposal_response(&payload) {
                        send_encrypted_packet(&mut stream, &mut crypt, 0x3609, &resp).await?;
                        info!(
                            "[Bot {}] ✅ CMSG_DF_PROPOSAL_RESPONSE sent (Accepted=true)",
                            bot_index
                        );
                        got_proposal = true;
                        result.got_proposal = true;
                        result.accepted_proposal = true;
                    } else {
                        warn!(
                            "[Bot {}] Proposal payload too short to parse Ticket prefix",
                            bot_index
                        );
                    }
                }

                if op == 0x2A22 {
                    // SMSG_LFG_READY_CHECK_UPDATE
                    info!("[Bot {}] 📢 LFG Ready Check received", bot_index);
                    result.got_ready_check = true;
                    let ready_data = vec![1u8];
                    send_encrypted_packet(&mut stream, &mut crypt, 0x361C, &ready_data).await?;
                    info!(
                        "[Bot {}] ✅ CMSG_DF_READY_CHECK_RESPONSE sent (Ready=true)",
                        bot_index
                    );
                }

                if op == 0x2A36 {
                    // SMSG_LFG_PARTY_INFO — sent when group is committed
                    info!("[Bot {}] 🎉 SMSG_LFG_PARTY_INFO — group formed!", bot_index);
                    group_formed = true;
                    result.group_formed = true;
                    if auto_teleport {
                        send_encrypted_packet(&mut stream, &mut crypt, 0x3619, &[1u8]).await?;
                        info!(
                            "[Bot {}] ✅ CMSG_DF_TELEPORT sent (teleport_out=false)",
                            bot_index
                        );
                    }
                }

                if op == 0x2594 {
                    // SMSG_NEW_WORLD
                    info!("[Bot {}] 🌍 SMSG_NEW_WORLD ({} bytes) — replying with CMSG_WORLD_PORT_RESPONSE", bot_index, payload.len());
                    send_encrypted_packet(&mut stream, &mut crypt, 0x35FA, &[]).await?;
                    info!(
                        "[Bot {}] ✅ CMSG_WORLD_PORT_RESPONSE sent — should now be inside map",
                        bot_index
                    );
                    result.entered_world = true;
                }

                if op == 0x2A32 {
                    // SMSG_LFG_TELEPORT_DENIED (1 byte body, reason code high nibble)
                    let reason_byte = payload.first().copied().unwrap_or(0);
                    let reason = reason_byte >> 4;
                    warn!(
                        "[Bot {}] ⛔ SMSG_LFG_TELEPORT_DENIED reason={} (raw=0x{:02X})",
                        bot_index, reason, reason_byte
                    );
                    result.teleport_denied_reason = Some(reason);
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Error: {}", bot_index, e);
                break;
            }
            Err(_) => {
                debug!("[Bot {}] LFG read budget exhausted", bot_index);
                break;
            }
        }
    }
    let _ = (got_proposal, group_formed); // mirrored in result; keep locals for readable logs while debugging

    tokio::time::sleep(Duration::from_secs(3)).await;
    info!(
        "[Bot {}] 🏁 Completed: result={}, detail={}",
        bot_index, result_code, detail_code
    );
    Ok(result)
}

fn log_bot_summary(
    result: &BotRunResult,
    require_proposal: bool,
    require_group: bool,
    login_only: bool,
) {
    if result.success(require_proposal, require_group, login_only) {
        if result.stand_state_smoke {
            info!(
                "✅ Bot {}: SUCCESS stand_state_smoke requested={:?} confirmed={:?} failure={:?}",
                result.account,
                result.stand_states_requested,
                result.stand_states_confirmed,
                result.stand_state_failure
            );
            return;
        }
        if result.quest_smoke {
            info!(
                "✅ Bot {}: SUCCESS quest_smoke target={:?}/{:?} ids={:?} details={} request_items={} accept_sent={} db_verified={} db_status={:?} obj_verified={} obj_before={:?} obj_after={:?} failure={:?}",
                result.account,
                result.quest_target_entry,
                result.quest_target_spawn_guid,
                result.quest_ids_seen,
                result.quest_details_seen,
                result.quest_request_items_seen,
                result.quest_accept_sent,
                result.quest_db_verified,
                result.quest_db_status,
                result.quest_objective_db_verified,
                result.quest_objective_db_before,
                result.quest_objective_db_after,
                result.quest_failure
            );
            return;
        }
        if result.bank_smoke {
            info!(
                "✅ Bot {}: SUCCESS bank_smoke banker={:?}/{:?} item={:?}/entry={:?} slots={:?}->{:?} open={} deposit={} relog={} withdraw={} failure={:?}",
                result.account,
                result.bank_banker_entry,
                result.bank_banker_spawn_guid,
                result.bank_item_guid,
                result.bank_item_entry,
                result.bank_inventory_slot,
                result.bank_bank_slot,
                result.bank_open_confirmed,
                result.bank_deposit_persisted,
                result.bank_relogin_after_deposit,
                result.bank_withdraw_persisted,
                result.bank_failure
            );
            return;
        }
        if result.void_storage_smoke {
            info!(
                "✅ Bot {}: SUCCESS void_storage item_id={:?} unlock={} deposit={} deposit_relog={} swap={} swap_relog={} withdraw={} withdraw_relog={} failure={:?}",
                result.account,
                result.void_storage_item_id,
                result.void_storage_unlock_persisted,
                result.void_storage_deposit_persisted,
                result.void_storage_deposit_relogin_verified,
                result.void_storage_swap_persisted,
                result.void_storage_swap_relogin_verified,
                result.void_storage_withdraw_persisted,
                result.void_storage_withdraw_relogin_verified,
                result.void_storage_failure
            );
            return;
        }
        if result.void_storage_query_capture {
            info!(
                "✅ Bot {}: SUCCESS void_storage_query_capture item_id={:?} failure={:?}",
                result.account, result.void_storage_item_id, result.void_storage_failure
            );
            return;
        }
        if result.homebind_smoke {
            info!(
                "✅ Bot {}: SUCCESS homebind_smoke innkeeper={:?}/{:?} spell_go={} bind_update={} player_bound={} gossip_complete={} db_persisted={} relog={} failure={:?}",
                result.account,
                result.homebind_innkeeper_entry,
                result.homebind_innkeeper_spawn_guid,
                result.homebind_spell_go_seen,
                result.homebind_bind_point_update_seen,
                result.homebind_player_bound_seen,
                result.homebind_gossip_complete_seen,
                result.homebind_db_persisted,
                result.homebind_relogin_verified,
                result.homebind_failure
            );
            return;
        }
        if result.inventory_swap_smoke {
            info!(
                "✅ Bot {}: SUCCESS inventory_swap_smoke items={:?}/{:?} entries={:?}/{:?} slots={:?}<->{:?} forward={} relog={} reverse={} failure={:?}",
                result.account,
                result.inventory_swap_item_guid_a,
                result.inventory_swap_item_guid_b,
                result.inventory_swap_item_entry_a,
                result.inventory_swap_item_entry_b,
                result.inventory_swap_slot_a,
                result.inventory_swap_slot_b,
                result.inventory_swap_forward_persisted,
                result.inventory_swap_relogin_after_forward,
                result.inventory_swap_reverse_persisted,
                result.inventory_swap_failure
            );
            return;
        }
        if result.vendor_smoke {
            info!(
                "✅ Bot {}: SUCCESS vendor_smoke vendor={:?}/{:?}/counter={:?} item={:?}/cost={:?} currency={:?} {:?}->{:?} item_total={:?} list={} buy={} set_currency={} item_push={} relog={} failure={:?}",
                result.account,
                result.vendor_entry,
                result.vendor_spawn_guid,
                result.vendor_runtime_counter,
                result.vendor_item_entry,
                result.vendor_extended_cost,
                result.vendor_currency_id,
                result.vendor_currency_before,
                result.vendor_currency_after,
                result.vendor_item_total_after,
                result.vendor_inventory_seen,
                result.vendor_buy_succeeded_seen,
                result.vendor_set_currency_seen,
                result.vendor_item_push_seen,
                result.vendor_relogin_verified,
                result.vendor_failure,
            );
            return;
        }
        if result.equipment_set_smoke {
            info!(
                "✅ Bot {}: SUCCESS equipment_set_smoke type={:?} set_id={:?} guid={:?} login_count={:?} load={} db={} relog={} failure={:?}",
                result.account,
                result.equipment_set_type,
                result.equipment_set_id,
                result.equipment_set_generated_guid,
                result.equipment_set_login_count,
                result.equipment_set_load_seen,
                result.equipment_set_db_persisted,
                result.equipment_set_relogin_verified,
                result.equipment_set_failure,
            );
            return;
        }
        if result.rested_xp_smoke {
            info!(
                "✅ Bot {}: SUCCESS rested_xp_smoke offline={:?}/{:?} target={:?}/{:?}/counter={:?} xp={:?}+{:?} rest={:?}->{:?} relog={} failure={:?}",
                result.account,
                result.rested_xp_offline_wilderness_bonus,
                result.rested_xp_offline_resting_bonus,
                result.rested_xp_target_entry,
                result.rested_xp_target_spawn_guid,
                result.rested_xp_target_guid_counter,
                result.rested_xp_packet_amount,
                result.rested_xp_packet_original,
                result.rested_xp_db_rest_before,
                result.rested_xp_db_rest_after,
                result.rested_xp_relog_verified,
                result.rested_xp_failure,
            );
            return;
        }
        if result.loot_race_smoke {
            info!(
                "✅ Bot {}: SUCCESS loot_race target={:?}/{:?}/counter={:?} party={} discovered={} opened={} list={:?} coins={:?} item_push={} removed={} money_notify={:?} coin_removed={} db_item={:?} db_money_delta={:?} relog={} failure={:?}",
                result.account,
                result.loot_race_target_entry,
                result.loot_race_target_spawn_guid,
                result.loot_race_target_runtime_counter,
                result.loot_race_party_confirmed,
                result.loot_race_target_discovered,
                result.loot_race_loot_opened,
                result.loot_race_loot_list_id,
                result.loot_race_loot_coins,
                result.loot_race_item_push_seen,
                result.loot_race_loot_removed_seen,
                result.loot_race_money_notify_amount,
                result.loot_race_coin_removed_seen,
                result.loot_race_db_item_total,
                result.loot_race_db_money_delta,
                result.loot_race_relog_verified,
                result.loot_race_failure,
            );
            return;
        }
        if result.group_capacity_race_smoke {
            info!(
                "✅ Bot {}: SUCCESS group_capacity_race group={:?} outcome={:?} final_members={:?} failure={:?}",
                result.account,
                result.group_capacity_group_id,
                result.group_capacity_outcome,
                result.group_capacity_final_member_count,
                result.group_capacity_failure,
            );
            return;
        }
        info!(
            "✅ Bot {}: SUCCESS login={{auth:{}, enum:{}, player:{}}} join={:?}/{:?} proposal={} group={} teleport_denied={:?}",
            result.account,
            result.world_auth,
            result.enum_characters,
            result.player_login_verified,
            result.join_result,
            result.join_detail,
            result.got_proposal,
            result.group_formed,
            result.teleport_denied_reason
        );
    } else {
        if result.stand_state_smoke {
            error!(
                "❌ Bot {}: FAILED stand_state_smoke requested={:?} confirmed={:?} failure={:?}",
                result.account,
                result.stand_states_requested,
                result.stand_states_confirmed,
                result.stand_state_failure
            );
            return;
        }
        if result.quest_smoke {
            error!(
                "❌ Bot {}: FAILED quest_smoke target={:?}/{:?} ids={:?} details={} request_items={} accept_sent={} db_verified={} db_status={:?} obj_verified={} obj_before={:?} obj_after={:?} failure={:?}",
                result.account,
                result.quest_target_entry,
                result.quest_target_spawn_guid,
                result.quest_ids_seen,
                result.quest_details_seen,
                result.quest_request_items_seen,
                result.quest_accept_sent,
                result.quest_db_verified,
                result.quest_db_status,
                result.quest_objective_db_verified,
                result.quest_objective_db_before,
                result.quest_objective_db_after,
                result.quest_failure
            );
            return;
        }
        if result.bank_smoke {
            error!(
                "❌ Bot {}: FAILED bank_smoke banker={:?}/{:?} item={:?}/entry={:?} slots={:?}->{:?} open={} deposit={} relog={} withdraw={} failure={:?}",
                result.account,
                result.bank_banker_entry,
                result.bank_banker_spawn_guid,
                result.bank_item_guid,
                result.bank_item_entry,
                result.bank_inventory_slot,
                result.bank_bank_slot,
                result.bank_open_confirmed,
                result.bank_deposit_persisted,
                result.bank_relogin_after_deposit,
                result.bank_withdraw_persisted,
                result.bank_failure
            );
            return;
        }
        if result.void_storage_smoke {
            error!(
                "❌ Bot {}: FAILED void_storage item_id={:?} unlock={} deposit={} deposit_relog={} swap={} swap_relog={} withdraw={} withdraw_relog={} failure={:?}",
                result.account,
                result.void_storage_item_id,
                result.void_storage_unlock_persisted,
                result.void_storage_deposit_persisted,
                result.void_storage_deposit_relogin_verified,
                result.void_storage_swap_persisted,
                result.void_storage_swap_relogin_verified,
                result.void_storage_withdraw_persisted,
                result.void_storage_withdraw_relogin_verified,
                result.void_storage_failure
            );
            return;
        }
        if result.void_storage_query_capture {
            error!(
                "❌ Bot {}: FAILED void_storage_query_capture item_id={:?} failure={:?}",
                result.account, result.void_storage_item_id, result.void_storage_failure
            );
            return;
        }
        if result.homebind_smoke {
            error!(
                "❌ Bot {}: FAILED homebind_smoke innkeeper={:?}/{:?} spell_go={} bind_update={} player_bound={} gossip_complete={} db_persisted={} relog={} failure={:?}",
                result.account,
                result.homebind_innkeeper_entry,
                result.homebind_innkeeper_spawn_guid,
                result.homebind_spell_go_seen,
                result.homebind_bind_point_update_seen,
                result.homebind_player_bound_seen,
                result.homebind_gossip_complete_seen,
                result.homebind_db_persisted,
                result.homebind_relogin_verified,
                result.homebind_failure
            );
            return;
        }
        if result.inventory_swap_smoke {
            error!(
                "❌ Bot {}: FAILED inventory_swap_smoke items={:?}/{:?} entries={:?}/{:?} slots={:?}<->{:?} forward={} relog={} reverse={} failure={:?}",
                result.account,
                result.inventory_swap_item_guid_a,
                result.inventory_swap_item_guid_b,
                result.inventory_swap_item_entry_a,
                result.inventory_swap_item_entry_b,
                result.inventory_swap_slot_a,
                result.inventory_swap_slot_b,
                result.inventory_swap_forward_persisted,
                result.inventory_swap_relogin_after_forward,
                result.inventory_swap_reverse_persisted,
                result.inventory_swap_failure
            );
            return;
        }
        if result.vendor_smoke {
            error!(
                "❌ Bot {}: FAILED vendor_smoke vendor={:?}/{:?}/counter={:?} item={:?}/cost={:?} currency={:?} {:?}->{:?} item_total={:?} list={} buy={} set_currency={} item_push={} relog={} failure={:?}",
                result.account,
                result.vendor_entry,
                result.vendor_spawn_guid,
                result.vendor_runtime_counter,
                result.vendor_item_entry,
                result.vendor_extended_cost,
                result.vendor_currency_id,
                result.vendor_currency_before,
                result.vendor_currency_after,
                result.vendor_item_total_after,
                result.vendor_inventory_seen,
                result.vendor_buy_succeeded_seen,
                result.vendor_set_currency_seen,
                result.vendor_item_push_seen,
                result.vendor_relogin_verified,
                result.vendor_failure,
            );
            return;
        }
        if result.equipment_set_smoke {
            error!(
                "❌ Bot {}: FAILED equipment_set_smoke type={:?} set_id={:?} guid={:?} login_count={:?} load={} db={} relog={} failure={:?}",
                result.account,
                result.equipment_set_type,
                result.equipment_set_id,
                result.equipment_set_generated_guid,
                result.equipment_set_login_count,
                result.equipment_set_load_seen,
                result.equipment_set_db_persisted,
                result.equipment_set_relogin_verified,
                result.equipment_set_failure,
            );
            return;
        }
        if result.rested_xp_smoke {
            error!(
                "❌ Bot {}: FAILED rested_xp_smoke offline={:?}/{:?} target={:?}/{:?}/counter={:?} packet={:?}/{:?} db_xp={:?}->{:?} db_rest={:?}->{:?} relog={} failure={:?}",
                result.account,
                result.rested_xp_offline_wilderness_bonus,
                result.rested_xp_offline_resting_bonus,
                result.rested_xp_target_entry,
                result.rested_xp_target_spawn_guid,
                result.rested_xp_target_guid_counter,
                result.rested_xp_packet_amount,
                result.rested_xp_packet_original,
                result.rested_xp_db_xp_before,
                result.rested_xp_db_xp_after,
                result.rested_xp_db_rest_before,
                result.rested_xp_db_rest_after,
                result.rested_xp_relog_verified,
                result.rested_xp_failure,
            );
            return;
        }
        if result.loot_race_smoke {
            error!(
                "❌ Bot {}: FAILED loot_race target={:?}/{:?}/counter={:?} party={} discovered={} opened={} list={:?} coins={:?} item_push={} removed={} money_notify={:?} coin_removed={} db_item={:?} db_money_delta={:?} relog={} failure={:?}",
                result.account,
                result.loot_race_target_entry,
                result.loot_race_target_spawn_guid,
                result.loot_race_target_runtime_counter,
                result.loot_race_party_confirmed,
                result.loot_race_target_discovered,
                result.loot_race_loot_opened,
                result.loot_race_loot_list_id,
                result.loot_race_loot_coins,
                result.loot_race_item_push_seen,
                result.loot_race_loot_removed_seen,
                result.loot_race_money_notify_amount,
                result.loot_race_coin_removed_seen,
                result.loot_race_db_item_total,
                result.loot_race_db_money_delta,
                result.loot_race_relog_verified,
                result.loot_race_failure,
            );
            return;
        }
        if result.group_capacity_race_smoke {
            error!(
                "❌ Bot {}: FAILED group_capacity_race group={:?} outcome={:?} final_members={:?} failure={:?}",
                result.account,
                result.group_capacity_group_id,
                result.group_capacity_outcome,
                result.group_capacity_final_member_count,
                result.group_capacity_failure,
            );
            return;
        }
        error!(
            "❌ Bot {}: FAILED login={{auth:{}, enum:{}, player:{}}} join={:?}/{:?} proposal={} group={} teleport_denied={:?}",
            result.account,
            result.world_auth,
            result.enum_characters,
            result.player_login_verified,
            result.join_result,
            result.join_detail,
            result.got_proposal,
            result.group_formed,
            result.teleport_denied_reason
        );
    }
}

fn write_report_if_requested(
    cli: &CliOptions,
    dungeon_id: u32,
    timeout_secs: u64,
    require_proposal: bool,
    require_group: bool,
    auto_teleport: bool,
    login_only: bool,
    stand_state_smoke: bool,
    bank_smoke: bool,
    void_storage_smoke: bool,
    void_storage_query_capture: bool,
    homebind_smoke: bool,
    inventory_swap_smoke: bool,
    vendor_smoke: bool,
    equipment_set_race_smoke: bool,
    rested_xp_smoke: bool,
    loot_race_smoke: bool,
    loot_item_capture: bool,
    group_capacity_race_smoke: bool,
    quest_smoke: bool,
    results: &[BotRunResult],
) -> Result<()> {
    let path = cli.report_path.clone().unwrap_or_else(|| {
        format!(
            "/tmp/wow-bot-run-{}.json",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        )
    });
    let report = RunReport {
        dungeon_id,
        timeout_secs,
        require_proposal,
        require_group,
        auto_teleport,
        login_only,
        stand_state_smoke,
        bank_smoke,
        void_storage_smoke,
        void_storage_query_capture,
        homebind_smoke,
        inventory_swap_smoke,
        vendor_smoke,
        equipment_set_race_smoke,
        rested_xp_smoke,
        loot_race_smoke,
        loot_item_capture,
        group_capacity_race_smoke,
        quest_smoke,
        results: results.to_vec(),
    };
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&path, json)?;
    info!("Report written: {}", path);
    Ok(())
}

fn cleanup_bot_group_state(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let guids: Vec<u64> = bots.iter().map(|bot| bot.character_guid).collect();
    if guids.is_empty() {
        return Ok(());
    }

    let db_url = characters_db_url()?;
    let opts =
        mysql::Opts::from_url(&db_url).map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;

    let placeholders = std::iter::repeat("?")
        .take(guids.len())
        .collect::<Vec<_>>()
        .join(",");
    let params = mysql::Params::Positional(guids.iter().copied().map(mysql::Value::from).collect());

    conn.exec_drop(
        format!(
            "DELETE FROM group_member WHERE memberGuid IN ({})",
            placeholders
        ),
        params.clone(),
    )
    .map_err(|e| anyhow!("DELETE group_member for bots: {}", e))?;

    conn.exec_drop(
        format!("DELETE FROM groups WHERE leaderGuid IN ({})", placeholders),
        params,
    )
    .map_err(|e| anyhow!("DELETE groups for bots: {}", e))?;

    info!(
        "Cleaned stale group rows for {} configured bot GUIDs",
        bots.len()
    );
    Ok(())
}

async fn run_stand_state_smoke(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &StandStateSmokeOptions,
    result: &mut BotRunResult,
) {
    match run_stand_state_smoke_inner(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options,
        result,
    )
    .await
    {
        Ok(()) => {
            result.stand_state_smoke_passed = Some(true);
            info!(
                "[Bot {}] ✅ Stand-state smoke passed: {:?}",
                bot_index, result.stand_states_confirmed
            );
        }
        Err(error) => {
            result.stand_state_smoke_passed = Some(false);
            result.stand_state_failure = Some(error.to_string());
            warn!("[Bot {}] Stand-state smoke failed: {}", bot_index, error);
        }
    }
}

async fn run_stand_state_smoke_inner(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &StandStateSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    validate_stand_state_socket_topology(realm_connection.is_some())?;

    let _ = drain_connections_until_quiet_for_stand_state_smoke(
        bot_index,
        "login burst",
        STAND_STATE_LOGIN_QUIET_PERIOD,
        STAND_STATE_LOGIN_DRAIN_LIMIT,
        false,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        result,
    )
    .await?;

    let mut ack_side_effects = StandStateDrainSummary::default();
    for &expected_state in &options.states {
        let request = build_stand_state_change(expected_state);
        send_encrypted_packet(stream, crypt, CMSG_STAND_STATE_CHANGE, &request).await?;
        info!(
            "[Bot {}] ✅ CMSG_STAND_STATE_CHANGE sent on instance (state={})",
            bot_index, expected_state
        );

        let realm = realm_connection
            .as_mut()
            .context("stand-state realm socket disappeared after topology validation")?;
        let summary = wait_for_stand_state_update(
            bot_index,
            "realm",
            true,
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            options.timeout_secs,
            expected_state,
            result,
        )
        .await?;
        ack_side_effects.active_update_objects += summary.active_update_objects;
        ack_side_effects.active_aura_updates += summary.active_aura_updates;
    }

    // Keep both sockets alive until deferred UpdateObject/aura fanout has been
    // observed, then write a deterministic CMSG_PING fence. Capture-diff trims
    // at that CMSG, so the isolated action cannot end at the earlier realm ACK
    // and silently omit instance-side state deltas.
    let post_action = drain_connections_until_quiet_for_stand_state_smoke(
        bot_index,
        "stand-state side effects",
        STAND_STATE_POST_ACTION_QUIET_PERIOD,
        STAND_STATE_POST_ACTION_DRAIN_LIMIT,
        true,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        result,
    )
    .await?;
    info!(
        "[Bot {}] ✅ stand side effects on active connection: UpdateObject={}, AuraUpdate={}",
        bot_index,
        ack_side_effects.active_update_objects + post_action.active_update_objects,
        ack_side_effects.active_aura_updates + post_action.active_aura_updates
    );
    if options
        .states
        .iter()
        .any(|state| *state != UNIT_STAND_STATE_STAND)
        && ack_side_effects.active_update_objects + post_action.active_update_objects == 0
    {
        bail!(
            "changed stand-state smoke received no instance SMSG_UPDATE_OBJECT before capture fence"
        );
    }
    send_and_verify_stand_state_capture_fence(
        bot_index,
        stream,
        crypt,
        server_inflater,
        options.timeout_secs,
        result,
    )
    .await?;

    Ok(())
}

fn validate_stand_state_socket_topology(has_separate_realm_connection: bool) -> Result<()> {
    if !has_separate_realm_connection {
        bail!(
            "stand-state smoke requires SMSG_CONNECT_TO and distinct realm/instance sockets; single-socket login cannot validate opcode routing"
        );
    }
    Ok(())
}

/// Opcodes.cpp registers SMSG_STAND_STATE_UPDATE on CONNECTION_TYPE_REALM.
/// When login created a separate instance socket, accepting this opcode from
/// that instance connection would hide a routing-parity bug.
async fn wait_for_stand_state_update(
    bot_index: usize,
    connection_name: &str,
    separate_realm_connection: bool,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    expected_state: u8,
    result: &mut BotRunResult,
) -> Result<StandStateDrainSummary> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut side_effects = StandStateDrainSummary::default();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for realm SMSG_STAND_STATE_UPDATE state {}",
                expected_state
            );
        }

        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "timed out waiting for realm SMSG_STAND_STATE_UPDATE state {}",
                expected_state
            )
        })??;
        result.seen_opcodes.push(format!("0x{:04X}", opcode));
        info!(
            "[Bot {}] 📦 {} {}",
            bot_index,
            connection_name,
            parse_packet(opcode, &payload)
        );

        if opcode != SMSG_STAND_STATE_UPDATE {
            if matches!(opcode, SMSG_UPDATE_OBJECT | SMSG_AURA_UPDATE) {
                if separate_realm_connection {
                    bail!(
                        "{} arrived on the separate realm connection before StandStateUpdate; C++ routes stand side effects on instance",
                        parse_packet(opcode, &payload)
                    );
                }
                match opcode {
                    SMSG_UPDATE_OBJECT => side_effects.active_update_objects += 1,
                    SMSG_AURA_UPDATE => side_effects.active_aura_updates += 1,
                    _ => unreachable!(),
                }
            }
            continue;
        }

        validate_stand_state_update(&payload, expected_state)?;
        result.stand_states_confirmed.push(expected_state);
        info!(
            "[Bot {}] ✅ realm SMSG_STAND_STATE_UPDATE matched (animKitID=0, state={})",
            bot_index, expected_state
        );
        return Ok(side_effects);
    }
}

#[derive(Debug, Default)]
struct StandStateDrainSummary {
    active_update_objects: usize,
    active_aura_updates: usize,
}

fn stand_state_quiet_drain_ambient_opcode(opcode: u16) -> bool {
    matches!(opcode, SMSG_TIME_SYNC_REQUEST | SMSG_ON_MONSTER_MOVE)
}

async fn drain_connections_until_quiet_for_stand_state_smoke(
    bot_index: usize,
    phase: &str,
    quiet_period: Duration,
    drain_limit: Duration,
    enforce_instance_side_effect_routing: bool,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<StandStateDrainSummary> {
    enum DrainReady {
        Active,
        Realm,
        Quiet,
    }

    let deadline = tokio::time::Instant::now() + drain_limit;
    let mut quiet_deadline = tokio::time::Instant::now() + quiet_period;
    let mut drained = 0usize;
    let mut summary = StandStateDrainSummary::default();

    loop {
        let now = tokio::time::Instant::now();
        let remaining = deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            bail!(
                "{} did not become quiet within {} ms",
                phase,
                drain_limit.as_millis()
            );
        }
        let quiet_remaining = quiet_deadline.saturating_duration_since(now);
        if quiet_remaining.is_zero() {
            info!(
                "[Bot {}] ✅ {} drained across realm/instance: {} packet(s), {} ms relevant quiet",
                bot_index,
                phase,
                drained,
                quiet_period.as_millis()
            );
            return Ok(summary);
        }
        let quiet_wait = remaining.min(quiet_remaining);

        let quiet = tokio::time::sleep(quiet_wait);
        tokio::pin!(quiet);
        // `peek` waits for real bytes without consuming them. It is safe to
        // cancel when the other socket or quiet timer wins, unlike selecting
        // directly on the `read_exact`-based packet reader; it also avoids a
        // stale Tokio readiness bit causing a full read to wait past quiet.
        let mut active_peek = [0u8; 1];
        let mut realm_peek = [0u8; 1];
        let ready = if let Some(realm) = realm_connection.as_ref() {
            tokio::select! {
                result = stream.peek(&mut active_peek) => {
                    if result.with_context(|| format!("active {phase} drain peek failed"))? == 0 {
                        bail!("active connection closed during {phase} drain");
                    }
                    DrainReady::Active
                }
                result = realm.stream.peek(&mut realm_peek) => {
                    if result.with_context(|| format!("realm {phase} drain peek failed"))? == 0 {
                        bail!("realm connection closed during {phase} drain");
                    }
                    DrainReady::Realm
                }
                _ = &mut quiet => DrainReady::Quiet,
            }
        } else {
            tokio::select! {
                result = stream.peek(&mut active_peek) => {
                    if result.with_context(|| format!("primary realm {phase} drain peek failed"))? == 0 {
                        bail!("primary realm connection closed during {phase} drain");
                    }
                    DrainReady::Active
                }
                _ = &mut quiet => DrainReady::Quiet,
            }
        };

        match ready {
            DrainReady::Quiet => {
                if tokio::time::Instant::now() >= quiet_deadline {
                    info!(
                        "[Bot {}] ✅ {} drained across realm/instance: {} packet(s), {} ms relevant quiet",
                        bot_index,
                        phase,
                        drained,
                        quiet_period.as_millis()
                    );
                    return Ok(summary);
                }
                bail!(
                    "{} did not become quiet within {} ms",
                    phase,
                    drain_limit.as_millis()
                );
            }
            DrainReady::Active => {
                let read_remaining =
                    deadline.saturating_duration_since(tokio::time::Instant::now());
                let packet = tokio::time::timeout(
                    read_remaining,
                    read_encrypted_packet(stream, crypt, server_inflater),
                )
                .await
                .map_err(|_| anyhow!("active {phase} drain packet read timed out"))?;
                let (opcode, payload) = packet
                    .map_err(|error| anyhow!("active {phase} drain packet read failed: {error}"))?;
                drained += 1;
                if !stand_state_quiet_drain_ambient_opcode(opcode) {
                    quiet_deadline = tokio::time::Instant::now() + quiet_period;
                }
                if enforce_instance_side_effect_routing {
                    match opcode {
                        SMSG_UPDATE_OBJECT => summary.active_update_objects += 1,
                        SMSG_AURA_UPDATE => summary.active_aura_updates += 1,
                        _ => {}
                    }
                }
                result.seen_opcodes.push(format!("0x{:04X}", opcode));
                let connection_name = if realm_connection.is_some() {
                    "instance"
                } else {
                    "primary realm"
                };
                info!(
                    "[Bot {}] 📦 {} {} drain {}",
                    bot_index,
                    connection_name,
                    phase,
                    parse_packet(opcode, &payload)
                );
            }
            DrainReady::Realm => {
                let read_remaining =
                    deadline.saturating_duration_since(tokio::time::Instant::now());
                let realm = realm_connection.as_mut().with_context(|| {
                    format!("realm connection disappeared during {phase} drain")
                })?;
                let packet = tokio::time::timeout(
                    read_remaining,
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("realm {phase} drain packet read timed out"))?;
                let (opcode, payload) = packet
                    .map_err(|error| anyhow!("realm {phase} drain packet read failed: {error}"))?;
                drained += 1;
                if !stand_state_quiet_drain_ambient_opcode(opcode) {
                    quiet_deadline = tokio::time::Instant::now() + quiet_period;
                }
                if enforce_instance_side_effect_routing
                    && matches!(opcode, SMSG_UPDATE_OBJECT | SMSG_AURA_UPDATE)
                {
                    bail!(
                        "{} arrived on realm during {} drain; C++ routes stand side effects on instance",
                        parse_packet(opcode, &payload),
                        phase
                    );
                }
                result.seen_opcodes.push(format!("0x{:04X}", opcode));
                info!(
                    "[Bot {}] 📦 realm {} drain {}",
                    bot_index,
                    phase,
                    parse_packet(opcode, &payload)
                );
            }
        }
    }
}

async fn send_and_verify_stand_state_capture_fence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let payload = build_ping_payload(STAND_STATE_CAPTURE_FENCE_SERIAL);
    send_encrypted_packet(stream, crypt, CMSG_PING, &payload).await?;
    info!(
        "[Bot {}] ✅ deterministic CMSG_PING capture fence sent (serial=0x{:08X})",
        bot_index, STAND_STATE_CAPTURE_FENCE_SERIAL
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for stand-state capture-fence SMSG_PONG");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for stand-state capture-fence SMSG_PONG"))??;
        result.seen_opcodes.push(format!("0x{:04X}", opcode));
        info!(
            "[Bot {}] 📦 active capture-fence {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode != SMSG_PONG {
            continue;
        }
        if payload != STAND_STATE_CAPTURE_FENCE_SERIAL.to_le_bytes() {
            bail!(
                "capture-fence SMSG_PONG mismatch: expected serial 0x{:08X}, got {:02X?}",
                STAND_STATE_CAPTURE_FENCE_SERIAL,
                payload
            );
        }
        return Ok(());
    }
}

/// C++ `WorldPackets::Auth::Ping::Read`: uint32 serial then uint32 latency.
fn build_ping_payload(serial: u32) -> [u8; 8] {
    let mut payload = [0u8; 8];
    payload[..4].copy_from_slice(&serial.to_le_bytes());
    payload
}

/// C++ `TimeSyncResponse::Read`: request sequence then client ticks in ms.
fn build_time_sync_response_payload(sequence_index: u32, client_time: u32) -> [u8; 8] {
    let mut payload = [0u8; 8];
    payload[..4].copy_from_slice(&sequence_index.to_le_bytes());
    payload[4..].copy_from_slice(&client_time.to_le_bytes());
    payload
}

fn parse_time_sync_request_sequence(payload: &[u8]) -> Result<u32> {
    let bytes: [u8; 4] = payload
        .try_into()
        .map_err(|_| anyhow!("SMSG_TIME_SYNC_REQUEST payload must contain exactly 4 bytes"))?;
    Ok(u32::from_le_bytes(bytes))
}

/// C++ WorldPackets::Misc::StandStateChange::Read: one little-endian uint32.
fn build_stand_state_change(state: u8) -> [u8; 4] {
    u32::from(state).to_le_bytes()
}

/// C++ StandStateUpdate::Write: uint32 AnimKitID followed by uint8 State.
fn validate_stand_state_update(payload: &[u8], expected_state: u8) -> Result<()> {
    if payload.len() != 5 {
        bail!(
            "SMSG_STAND_STATE_UPDATE payload length mismatch: expected 5, got {}",
            payload.len()
        );
    }

    let anim_kit_id = u32::from_le_bytes(payload[0..4].try_into()?);
    let state = payload[4];
    if anim_kit_id != 0 {
        bail!(
            "SMSG_STAND_STATE_UPDATE AnimKitID mismatch: expected 0, got {}",
            anim_kit_id
        );
    }
    if state != expected_state {
        bail!(
            "SMSG_STAND_STATE_UPDATE state mismatch: expected {}, got {}",
            expected_state,
            state
        );
    }

    Ok(())
}

async fn run_quest_smoke(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) {
    match run_quest_smoke_inner(
        bot_index,
        bot,
        stream,
        crypt,
        server_inflater,
        quest_options,
        result,
    )
    .await
    {
        Ok(()) => {
            let pass = quest_smoke_passes(quest_options, result);
            result.quest_smoke_passed = Some(pass);
            if pass {
                info!(
                    "[Bot {}] ✅ Quest smoke passed: ids={:?} titles={:?}",
                    bot_index, result.quest_ids_seen, result.quest_titles_seen
                );
            } else if result.quest_failure.is_none() {
                result.quest_failure = Some("Quest response expectations were not met".to_string());
            }
        }
        Err(e) => {
            result.quest_smoke_passed = Some(false);
            result.quest_failure = Some(e.to_string());
            warn!("[Bot {}] Quest smoke failed: {}", bot_index, e);
        }
    }
}

fn sorted_quest_objective_rows(mut rows: Vec<QuestObjectiveDbRow>) -> Vec<QuestObjectiveDbRow> {
    rows.sort();
    rows
}

fn record_quest_objective_login_signal(
    op: u16,
    payload: &[u8],
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) {
    if !quest_options.objective_persist || op != 0x27CB {
        return;
    }
    let Some(quest_id) = quest_options.expected_quest_id else {
        return;
    };
    if !payload
        .windows(4)
        .any(|window| window == quest_id.to_le_bytes())
    {
        return;
    }

    result.quest_objective_update_seen = true;
    result.quest_objective_update_has_expected = quest_options
        .objective_seed
        .iter()
        .filter(|row| row.data > 0)
        .all(|row| {
            u16::try_from(row.data).is_ok_and(|data| {
                payload
                    .windows(2)
                    .any(|window| window == data.to_le_bytes())
            })
        });
}

async fn logout_and_verify_quest_objectives(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let quest_id = quest_options
        .expected_quest_id
        .ok_or_else(|| anyhow!("Objective persistence requested without quest id"))?;

    send_encrypted_packet(stream, crypt, 0x34D6, &[0]).await?;
    info!("[Bot {}] ✅ CMSG_LOGOUT_REQUEST sent", bot_index);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);
                if op == 0x2684 {
                    info!("[Bot {}] ✅ SMSG_LOGOUT_COMPLETE received", bot_index);
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Logout read error: {}", bot_index, e);
                break;
            }
            Err(_) => break,
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    let bot_for_db = bot.clone();
    let after =
        tokio::task::spawn_blocking(move || load_bot_quest_objectives(&bot_for_db, quest_id))
            .await
            .map_err(|e| anyhow!("Quest objective after-load worker join failed: {}", e))??;
    let expected = sorted_quest_objective_rows(quest_options.objective_seed.clone());
    let actual = sorted_quest_objective_rows(after);
    result.quest_objective_db_after = actual.clone();
    result.quest_objective_db_verified = actual == expected;
    if !result.quest_objective_db_verified {
        bail!(
            "objective rows changed across logout: expected {:?}, got {:?}",
            expected,
            actual
        );
    }

    Ok(())
}

async fn run_rested_xp_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    creature_entry: u32,
    creature_spawn_guid: Option<u64>,
    runtime_counter: Option<u64>,
    offline_secs: u64,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_rested_xp_smoke_fixture(
            &bot_for_setup,
            creature_entry,
            creature_spawn_guid,
            runtime_counter,
            offline_secs,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Rested-XP fixture setup worker failed: {error}"))??;

    let workflow = run_rested_xp_smoke_workflow_inner(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        &fixture,
    )
    .await;

    // A successful XP packet proves this workflow killed the selected target.
    // On earlier protocol/discovery failures no respawn transition is expected;
    // skipping that wait avoids masking the real error. If an ambiguous failed
    // attack did create a timer, the next preflight still rejects it safely.
    let verify_target_respawn = workflow
        .as_ref()
        .is_ok_and(|result| result.rested_xp_packet_amount.is_some());

    // Cleanup is deliberately outside the workflow result so every login,
    // protocol, assertion, and DB error path attempts the bounded selected-field
    // restore after the server has completed its disconnect save.
    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_rested_xp_smoke_fixture(
            &bot_for_cleanup,
            &fixture_for_cleanup,
            verify_target_respawn,
        )
    })
    .await
    .map_err(|error| anyhow!("Rested-XP cleanup worker failed: {error}"))?;

    match (workflow, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(mut result), Err(error)) => {
            result.rested_xp_smoke_passed = Some(false);
            result.rested_xp_failure = Some(format!("fixture cleanup failed: {error}"));
            Ok(result)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "Rested-XP workflow failed: {workflow_error}; fixture cleanup failed: {cleanup_error}"
        )),
    }
}

async fn run_rested_xp_smoke_workflow_inner(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    fixture: &RestedXpSmokeFixture,
) -> Result<BotRunResult> {
    let mut wilderness_options = fixture.options.clone();
    wilderness_options.phase = RestedXpSmokePhase::OfflineWilderness;
    prepare_rested_xp_phase_async(
        bot.clone(),
        fixture.clone(),
        RestedXpSmokePhase::OfflineWilderness,
    )
    .await?;
    let mut combined = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        Some(wilderness_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    if !combined.rested_xp_smoke_passed.unwrap_or(false) {
        return Ok(combined);
    }
    let wilderness = load_rested_xp_db_state_async(bot.clone()).await?;
    let expected_wilderness = offline_rest_bonus_like_cpp(
        fixture.options.next_level_xp,
        fixture.offline_secs,
        REST_OFFLINE_WILDERNESS_BUBBLE,
        fixture.wilderness_rate,
    );
    if let Err(error) = validate_rested_xp_saved_state_shape(
        wilderness,
        fixture.test_level,
        0,
        0,
        "wilderness offline accrual",
    ) {
        set_rested_xp_failure(&mut combined, error.to_string());
        return Ok(combined);
    }
    if !offline_rest_bonus_matches_like_cpp(
        wilderness.rest_bonus,
        expected_wilderness,
        fixture.options.next_level_xp,
        REST_OFFLINE_WILDERNESS_BUBBLE,
        fixture.wilderness_rate,
        fixture.options.timeout_secs,
    ) {
        set_rested_xp_failure(
            &mut combined,
            format!(
                "wilderness offline bonus mismatch: expected approximately {expected_wilderness:.4}, got {:.4}",
                wilderness.rest_bonus
            ),
        );
        return Ok(combined);
    }
    combined.rested_xp_offline_wilderness_bonus = Some(wilderness.rest_bonus);

    let mut resting_options = fixture.options.clone();
    resting_options.phase = RestedXpSmokePhase::OfflineResting;
    prepare_rested_xp_phase_async(
        bot.clone(),
        fixture.clone(),
        RestedXpSmokePhase::OfflineResting,
    )
    .await?;
    let resting_result = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        Some(resting_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    merge_rested_xp_results(&mut combined, resting_result);
    if !combined.rested_xp_smoke_passed.unwrap_or(false) {
        return Ok(combined);
    }
    let resting = load_rested_xp_db_state_async(bot.clone()).await?;
    let expected_resting = offline_rest_bonus_like_cpp(
        fixture.options.next_level_xp,
        fixture.offline_secs,
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE,
        fixture.resting_rate,
    );
    if let Err(error) = validate_rested_xp_saved_state_shape(
        resting,
        fixture.test_level,
        0,
        0,
        "resting offline accrual",
    ) {
        set_rested_xp_failure(&mut combined, error.to_string());
        return Ok(combined);
    }
    if !offline_rest_bonus_matches_like_cpp(
        resting.rest_bonus,
        expected_resting,
        fixture.options.next_level_xp,
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE,
        fixture.resting_rate,
        fixture.options.timeout_secs,
    ) {
        set_rested_xp_failure(
            &mut combined,
            format!(
                "resting offline bonus mismatch: expected approximately {expected_resting:.4}, got {:.4}",
                resting.rest_bonus
            ),
        );
        return Ok(combined);
    }
    combined.rested_xp_offline_resting_bonus = Some(resting.rest_bonus);
    if resting.rest_bonus <= wilderness.rest_bonus {
        set_rested_xp_failure(
            &mut combined,
            format!(
                "resting offline bonus {:.4} was not greater than wilderness {:.4}",
                resting.rest_bonus, wilderness.rest_bonus
            ),
        );
        return Ok(combined);
    }

    let mut consume_options = fixture.options.clone();
    consume_options.phase = RestedXpSmokePhase::ConsumeKill;
    prepare_rested_xp_phase_async(
        bot.clone(),
        fixture.clone(),
        RestedXpSmokePhase::ConsumeKill,
    )
    .await?;
    let consume_result = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        Some(consume_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    merge_rested_xp_results(&mut combined, consume_result);
    if !combined.rested_xp_smoke_passed.unwrap_or(false) {
        return Ok(combined);
    }

    let expected_xp = combined
        .rested_xp_db_xp_after
        .context("rested-XP consume phase omitted persisted XP")?;
    let expected_rest_bonus = combined
        .rested_xp_db_rest_after
        .context("rested-XP consume phase omitted persisted rest bonus")?;
    let mut verify_options = fixture.options.clone();
    verify_options.phase = RestedXpSmokePhase::VerifyRelog;
    verify_options.expected_xp = Some(expected_xp);
    verify_options.expected_rest_bonus = Some(expected_rest_bonus);
    let verify_result = run_bot(
        bot,
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        Some(verify_options),
        None,
        None,
        None,
        None,
    )
    .await?;
    merge_rested_xp_results(&mut combined, verify_result);
    combined.rested_xp_smoke_passed =
        Some(combined.rested_xp_smoke_passed.unwrap_or(false) && combined.rested_xp_relog_verified);
    Ok(combined)
}

async fn prepare_rested_xp_phase_async(
    bot: config::BotConfig,
    fixture: RestedXpSmokeFixture,
    phase: RestedXpSmokePhase,
) -> Result<()> {
    tokio::task::spawn_blocking(move || prepare_rested_xp_character_phase(&bot, &fixture, phase))
        .await
        .map_err(|error| anyhow!("Rested-XP phase setup worker failed: {error}"))?
}

async fn load_rested_xp_db_state_async(bot: config::BotConfig) -> Result<RestedXpDbState> {
    tokio::task::spawn_blocking(move || load_rested_xp_db_state(&bot))
        .await
        .map_err(|error| anyhow!("Rested-XP DB state worker failed: {error}"))?
}

fn set_rested_xp_failure(result: &mut BotRunResult, message: String) {
    result.rested_xp_smoke_passed = Some(false);
    result.rested_xp_failure = Some(message);
}

fn merge_rested_xp_results(combined: &mut BotRunResult, next: BotRunResult) {
    combined.world_auth &= next.world_auth;
    combined.enum_characters &= next.enum_characters;
    combined.player_login_verified &= next.player_login_verified;
    combined.seen_opcodes.extend(next.seen_opcodes);
    combined.rested_xp_smoke_passed = Some(
        combined.rested_xp_smoke_passed.unwrap_or(false)
            && next.rested_xp_smoke_passed.unwrap_or(false),
    );
    if next.rested_xp_target_guid_counter.is_some() {
        combined.rested_xp_target_guid_counter = next.rested_xp_target_guid_counter;
    }
    if next.rested_xp_packet_amount.is_some() {
        combined.rested_xp_packet_amount = next.rested_xp_packet_amount;
        combined.rested_xp_packet_original = next.rested_xp_packet_original;
        combined.rested_xp_db_xp_before = next.rested_xp_db_xp_before;
        combined.rested_xp_db_xp_after = next.rested_xp_db_xp_after;
        combined.rested_xp_db_rest_before = next.rested_xp_db_rest_before;
        combined.rested_xp_db_rest_after = next.rested_xp_db_rest_after;
    }
    combined.rested_xp_relog_verified |= next.rested_xp_relog_verified;
    if next.rested_xp_failure.is_some() {
        combined.rested_xp_failure = next.rested_xp_failure;
    }
}

fn offline_rest_bonus_like_cpp(
    next_level_xp: u32,
    offline_secs: u64,
    bubble: f32,
    rate: f32,
) -> f32 {
    let extra = offline_secs as f32 * next_level_xp as f32 / 72_000.0 * bubble * rate;
    extra.clamp(0.0, next_level_xp as f32 * REST_BONUS_CAP_NEXT_LEVEL_FACTOR)
}

fn offline_rest_bonus_matches_like_cpp(
    actual: f32,
    expected: f32,
    next_level_xp: u32,
    bubble: f32,
    rate: f32,
    timeout_secs: u64,
) -> bool {
    let per_second = next_level_xp as f32 / 72_000.0 * bubble * rate.abs();
    let timing_slop = timeout_secs.saturating_add(30) as f32 * per_second;
    (actual - expected).abs() <= timing_slop.max(0.05)
}

fn prepare_equipment_set_smoke_fixture(
    bots: &[config::BotConfig],
) -> Result<EquipmentSetSmokeFixture> {
    use mysql::prelude::Queryable;

    if bots.len() != 2 {
        bail!("equipment-set fixture requires exactly two bots");
    }
    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;

    for bot in bots {
        if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
            bail!(
                "refusing equipment-set fixture setup for non-local account {}",
                bot.account
            );
        }
        let row: Option<(u32, u8)> = conn
            .exec_first(
                "SELECT account, online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Load equipment-set bot character: {error}"))?;
        let (owner, online) = row.ok_or_else(|| {
            anyhow!(
                "No characters row for equipment-set bot guid {}",
                bot.character_guid
            )
        })?;
        if owner != bot.account_id || online != 0 {
            bail!(
                "equipment-set bot {} ownership/online mismatch: owner={owner}, expected={}, online={online}",
                bot.character_guid,
                bot.account_id
            );
        }
        let equipment_rows: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM character_equipmentsets WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Count equipment-set fixture rows: {error}"))?
            .unwrap_or(0);
        let transmog_rows: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM character_transmog_outfits WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Count transmog-set fixture rows: {error}"))?
            .unwrap_or(0);
        if equipment_rows != 0 || transmog_rows != 0 {
            bail!(
                "equipment-set bot {} is not an empty disposable fixture (equipment={equipment_rows}, transmog={transmog_rows})",
                bot.character_guid
            );
        }
    }

    let initial_max_guid: Option<Option<u64>> = conn
        .query_first(SHARED_EQUIPMENT_SET_GUID_MAX_QUERY)
        .map_err(|error| anyhow!("Load shared equipment/transmog maximum: {error}"))?;
    Ok(EquipmentSetSmokeFixture {
        initial_max_guid: initial_max_guid.flatten().unwrap_or(0),
    })
}

fn verify_equipment_set_db_row(
    bot: &config::BotConfig,
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
) -> Result<bool> {
    use mysql::prelude::Queryable;

    let opts = mysql::Opts::from_url(&characters_db_url()?)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let equipment_rows: Vec<mysql::Row> = conn
        .exec(
            "SELECT CAST(setguid AS UNSIGNED) AS setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18 FROM character_equipmentsets WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load persisted equipment sets: {error}"))?;
    let equipment_rows = equipment_rows
        .iter()
        .map(equipment_set_db_row_from_mysql)
        .collect::<Result<Vec<_>>>()?;
    let transmog_rows: Vec<mysql::Row> = conn
        .exec(
            "SELECT CAST(setguid AS UNSIGNED) AS setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant FROM character_transmog_outfits WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load persisted transmog outfits: {error}"))?;
    let transmog_rows = transmog_rows
        .iter()
        .map(transmog_outfit_db_row_from_mysql)
        .collect::<Result<Vec<_>>>()?;

    Ok(equipment_set_db_rows_match(
        options,
        expected_guid,
        &equipment_rows,
        &transmog_rows,
    ))
}

fn equipment_set_db_rows_match(
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
    equipment_rows: &[EquipmentSetDbRow],
    transmog_rows: &[TransmogOutfitDbRow],
) -> bool {
    match options.set_type {
        0 => {
            equipment_rows == [expected_equipment_set_db_row(options, expected_guid)]
                && transmog_rows.is_empty()
        }
        1 => {
            equipment_rows.is_empty()
                && transmog_rows == [expected_transmog_outfit_db_row(options, expected_guid)]
        }
        _ => false,
    }
}

fn equipment_set_db_row_from_mysql(row: &mysql::Row) -> Result<EquipmentSetDbRow> {
    Ok(EquipmentSetDbRow {
        set_guid: required_row_value(row, "setguid")?,
        set_index: required_row_value(row, "setindex")?,
        name: required_row_value(row, "name")?,
        icon_name: required_row_value(row, "iconname")?,
        ignore_mask: required_row_value(row, "ignore_mask")?,
        assigned_spec_index: required_row_value(row, "AssignedSpecIndex")?,
        items: required_indexed_row_values(row, "item")?,
    })
}

fn transmog_outfit_db_row_from_mysql(row: &mysql::Row) -> Result<TransmogOutfitDbRow> {
    Ok(TransmogOutfitDbRow {
        set_guid: required_row_value(row, "setguid")?,
        set_index: required_row_value(row, "setindex")?,
        name: required_row_value(row, "name")?,
        icon_name: required_row_value(row, "iconname")?,
        ignore_mask: required_row_value(row, "ignore_mask")?,
        appearances: required_indexed_row_values(row, "appearance")?,
        main_hand_enchant: required_row_value(row, "mainHandEnchant")?,
        off_hand_enchant: required_row_value(row, "offHandEnchant")?,
    })
}

fn required_indexed_row_values<T, const N: usize>(row: &mysql::Row, prefix: &str) -> Result<[T; N]>
where
    T: mysql::prelude::FromValue,
{
    let mut values = Vec::with_capacity(N);
    for index in 0..N {
        values.push(required_row_value(row, &format!("{prefix}{index}"))?);
    }
    values
        .try_into()
        .map_err(|_| anyhow!("Expected exactly {N} `{prefix}` columns in QA fixture query"))
}

fn expected_equipment_set_db_row(
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
) -> EquipmentSetDbRow {
    EquipmentSetDbRow {
        set_guid: expected_guid,
        set_index: options.set_id,
        name: options.set_name.clone(),
        icon_name: options.set_icon.clone(),
        ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
        assigned_spec_index: -1,
        items: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    }
}

fn expected_transmog_outfit_db_row(
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
) -> TransmogOutfitDbRow {
    TransmogOutfitDbRow {
        set_guid: expected_guid,
        set_index: options.set_id,
        name: options.set_name.clone(),
        icon_name: options.set_icon.clone(),
        ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
        appearances: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
        main_hand_enchant: 0,
        off_hand_enchant: 0,
    }
}

fn cleanup_equipment_set_smoke_fixture(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let opts = mysql::Opts::from_url(&characters_db_url()?)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    for bot in bots {
        let offline_deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let online: u8 = conn
                .exec_first(
                    "SELECT online FROM characters WHERE guid = ?",
                    (bot.character_guid,),
                )
                .map_err(|error| anyhow!("Check equipment-set bot offline state: {error}"))?
                .ok_or_else(|| anyhow!("Equipment-set bot character disappeared"))?;
            if online == 0 {
                break;
            }
            if std::time::Instant::now() >= offline_deadline {
                bail!(
                    "equipment-set bot {} remained online before cleanup",
                    bot.character_guid
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        conn.exec_drop(
            "DELETE FROM character_equipmentsets WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clean equipment-set fixture rows: {error}"))?;
        conn.exec_drop(
            "DELETE FROM character_transmog_outfits WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clean transmog-set fixture rows: {error}"))?;
        let remaining: u64 = conn
            .exec_first(
                "SELECT (SELECT COUNT(*) FROM character_equipmentsets WHERE guid = ?) + (SELECT COUNT(*) FROM character_transmog_outfits WHERE guid = ?)",
                (bot.character_guid, bot.character_guid),
            )
            .map_err(|error| anyhow!("Verify equipment-set fixture cleanup: {error}"))?
            .unwrap_or(u64::MAX);
        if remaining != 0 {
            bail!(
                "equipment-set fixture cleanup left {remaining} rows for character {}",
                bot.character_guid
            );
        }
    }
    Ok(())
}

fn push_msb_bits(data: &mut Vec<u8>, bit_offset: &mut usize, value: u32, count: usize) {
    for shift in (0..count).rev() {
        if *bit_offset % 8 == 0 {
            data.push(0);
        }
        if (value >> shift) & 1 != 0 {
            let byte = data.len() - 1;
            data[byte] |= 1 << (7 - (*bit_offset % 8));
        }
        *bit_offset += 1;
    }
}

fn build_save_equipment_set_payload(options: &EquipmentSetSmokeOptions) -> Result<Vec<u8>> {
    let name_len = u32::try_from(options.set_name.len())?;
    let icon_len = u32::try_from(options.set_icon.len())?;
    if name_len > u8::MAX.into() || icon_len >= (1 << 9) {
        bail!("equipment-set fixture name/icon exceeds the packet bit width");
    }
    let mut data = Vec::with_capacity(512);
    data.extend_from_slice(&options.set_type.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.extend_from_slice(&options.set_id.to_le_bytes());
    data.extend_from_slice(&EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP.to_le_bytes());
    for _ in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
        data.extend_from_slice(&[0; 16]);
        data.extend_from_slice(&0_i32.to_le_bytes());
    }
    for _ in 0..6 {
        data.extend_from_slice(&0_i32.to_le_bytes());
    }
    let mut bit_offset = 0;
    push_msb_bits(&mut data, &mut bit_offset, 0, 1);
    push_msb_bits(&mut data, &mut bit_offset, name_len, 8);
    push_msb_bits(&mut data, &mut bit_offset, icon_len, 9);
    data.extend_from_slice(options.set_name.as_bytes());
    data.extend_from_slice(options.set_icon.as_bytes());
    Ok(data)
}

fn take_equipment_bytes<'a>(
    payload: &'a [u8],
    offset: &mut usize,
    count: usize,
) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(count)
        .ok_or_else(|| anyhow!("equipment-set packet offset overflow"))?;
    let bytes = payload
        .get(*offset..end)
        .ok_or_else(|| anyhow!("equipment-set packet truncated at byte {}", *offset))?;
    *offset = end;
    Ok(bytes)
}

fn read_equipment_u32(payload: &[u8], offset: &mut usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        take_equipment_bytes(payload, offset, 4)?.try_into()?,
    ))
}

fn read_equipment_i32(payload: &[u8], offset: &mut usize) -> Result<i32> {
    Ok(i32::from_le_bytes(
        take_equipment_bytes(payload, offset, 4)?.try_into()?,
    ))
}

fn read_equipment_u64(payload: &[u8], offset: &mut usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        take_equipment_bytes(payload, offset, 8)?.try_into()?,
    ))
}

fn read_equipment_msb_bits(payload: &[u8], bit_offset: &mut usize, count: usize) -> Result<u32> {
    let mut value = 0_u32;
    for _ in 0..count {
        let byte = *payload
            .get(*bit_offset / 8)
            .ok_or_else(|| anyhow!("equipment-set bit section truncated"))?;
        value = (value << 1) | u32::from((byte >> (7 - (*bit_offset % 8))) & 1);
        *bit_offset += 1;
    }
    Ok(value)
}

fn parse_load_equipment_sets(payload: &[u8]) -> Result<Vec<EquipmentSetWire>> {
    let mut offset = 0;
    let count = read_equipment_u32(payload, &mut offset)?;
    let mut sets = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let set_type = read_equipment_i32(payload, &mut offset)?;
        let guid = read_equipment_u64(payload, &mut offset)?;
        let set_id = read_equipment_u32(payload, &mut offset)?;
        let ignore_mask = read_equipment_u32(payload, &mut offset)?;
        let mut pieces = [[0_u8; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        let mut appearances = [0_i32; EQUIPMENT_SET_SLOTS_LIKE_CPP];
        for index in 0..EQUIPMENT_SET_SLOTS_LIKE_CPP {
            pieces[index] = take_equipment_bytes(payload, &mut offset, 16)?.try_into()?;
            appearances[index] = read_equipment_i32(payload, &mut offset)?;
        }
        let enchants = [
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
        ];
        let secondary_appearances_and_slots = [
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
            read_equipment_i32(payload, &mut offset)?,
        ];
        let mut bit_offset = offset * 8;
        let has_spec = read_equipment_msb_bits(payload, &mut bit_offset, 1)? != 0;
        let name_len = read_equipment_msb_bits(payload, &mut bit_offset, 8)? as usize;
        let icon_len = read_equipment_msb_bits(payload, &mut bit_offset, 9)? as usize;
        offset = bit_offset.div_ceil(8);
        let assigned_spec_index = if has_spec {
            read_equipment_i32(payload, &mut offset)?
        } else {
            -1
        };
        let set_name =
            std::str::from_utf8(take_equipment_bytes(payload, &mut offset, name_len)?)?.to_string();
        let set_icon =
            std::str::from_utf8(take_equipment_bytes(payload, &mut offset, icon_len)?)?.to_string();
        sets.push(EquipmentSetWire {
            set_type,
            guid,
            set_id,
            ignore_mask,
            pieces,
            appearances,
            enchants,
            secondary_appearances_and_slots,
            assigned_spec_index,
            set_name,
            set_icon,
        });
    }
    if offset != payload.len() {
        bail!(
            "SMSG_LOAD_EQUIPMENT_SET left {} trailing bytes",
            payload.len() - offset
        );
    }
    Ok(sets)
}

fn parse_equipment_set_id(payload: &[u8]) -> Result<(u64, i32, u32)> {
    if payload.len() != 16 {
        bail!(
            "SMSG_EQUIPMENT_SET_ID payload length mismatch: expected 16, got {}",
            payload.len()
        );
    }
    let mut offset = 0;
    Ok((
        read_equipment_u64(payload, &mut offset)?,
        read_equipment_i32(payload, &mut offset)?,
        read_equipment_u32(payload, &mut offset)?,
    ))
}

fn validate_equipment_set_id_response(
    on_realm: bool,
    payload: &[u8],
    options: &EquipmentSetSmokeOptions,
) -> Result<u64> {
    if on_realm {
        bail!("SMSG_EQUIPMENT_SET_ID arrived on realm instead of instance");
    }
    let (guid, set_type, set_id) = parse_equipment_set_id(payload)?;
    if guid == 0 || set_type != options.set_type || set_id != options.set_id {
        bail!(
            "SMSG_EQUIPMENT_SET_ID mismatch: got {guid}/{set_type}/{set_id}, expected nonzero/{}/{}",
            options.set_type,
            options.set_id
        );
    }
    Ok(guid)
}

fn record_equipment_set_login_signal(
    opcode: u16,
    payload: &[u8],
    options: &EquipmentSetSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if opcode != SMSG_LOAD_EQUIPMENT_SET {
        return Ok(());
    }
    if result.equipment_set_login_count.is_some() {
        bail!("received duplicate SMSG_LOAD_EQUIPMENT_SET during one login");
    }
    let sets = parse_load_equipment_sets(payload)?;
    result.equipment_set_login_count = Some(u32::try_from(sets.len())?);
    result.equipment_set_load_seen = true;
    if options.phase == EquipmentSetSmokePhase::VerifyRelog {
        let expected_guid = options
            .expected_guid
            .context("equipment-set relog phase missing expected GUID")?;
        let expected = EquipmentSetWire {
            set_type: options.set_type,
            guid: expected_guid,
            set_id: options.set_id,
            ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
            pieces: [[0; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP],
            appearances: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_appearances_and_slots: [0; 4],
            assigned_spec_index: -1,
            set_name: options.set_name.clone(),
            set_icon: options.set_icon.clone(),
        };
        result.equipment_set_relogin_verified = sets.as_slice() == std::slice::from_ref(&expected);
        if !result.equipment_set_relogin_verified {
            warn!(
                "Equipment-set relog mismatch for {}: expected {:?}, loaded {:?}",
                result.account, expected, sets
            );
        }
    }
    Ok(())
}

async fn wait_for_equipment_set_id_routed(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    mut realm: Option<&mut EncryptedWorldConnection>,
    options: &EquipmentSetSmokeOptions,
    result: &mut BotRunResult,
) -> Result<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for SMSG_EQUIPMENT_SET_ID");
        }
        let routed = if let Some(realm_connection) = realm.as_deref_mut() {
            read_encrypted_packet_if_ready(
                &mut realm_connection.stream,
                &mut realm_connection.crypt,
                &mut realm_connection.inflater,
                remaining.min(Duration::from_millis(5)),
                remaining,
                "equipment-set realm response",
            )
            .await?
            .map(|(opcode, payload)| (true, opcode, payload))
        } else {
            None
        };
        let routed = if routed.is_some() {
            routed
        } else {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            read_encrypted_packet_if_ready(
                stream,
                crypt,
                inflater,
                remaining.min(Duration::from_millis(5)),
                remaining,
                "equipment-set instance response",
            )
            .await?
            .map(|(opcode, payload)| (false, opcode, payload))
        };
        let Some((on_realm, opcode, payload)) = routed else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            if on_realm {
                bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during equipment-set save");
            }
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        if opcode != SMSG_EQUIPMENT_SET_ID {
            continue;
        }
        let guid = validate_equipment_set_id_response(on_realm, &payload, options)?;
        info!(
            "[Bot {}] ✅ SMSG_EQUIPMENT_SET_ID guid={} type={} set_id={} route=instance",
            bot_index, guid, options.set_type, options.set_id
        );
        return Ok(guid);
    }
}

async fn run_equipment_set_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    mut realm: Option<&mut EncryptedWorldConnection>,
    options: &EquipmentSetSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if !result.equipment_set_load_seen {
        bail!("login omitted SMSG_LOAD_EQUIPMENT_SET");
    }
    match options.phase {
        EquipmentSetSmokePhase::Save => {
            if result.equipment_set_login_count != Some(0) {
                bail!(
                    "disposable equipment-set fixture loaded {:?} pre-existing sets",
                    result.equipment_set_login_count
                );
            }
            let barrier = options
                .save_barrier
                .as_ref()
                .context("equipment-set save phase missing race barrier")?;
            tokio::time::timeout(Duration::from_secs(options.timeout_secs), barrier.wait())
                .await
                .map_err(|_| anyhow!("timed out at equipment-set concurrent save barrier"))?;
            let payload = build_save_equipment_set_payload(options)?;
            send_encrypted_packet(stream, crypt, CMSG_SAVE_EQUIPMENT_SET, &payload).await?;
            info!(
                "[Bot {}] ✅ CMSG_SAVE_EQUIPMENT_SET sent type={} set_id={}",
                bot_index, options.set_type, options.set_id
            );
            let guid = wait_for_equipment_set_id_routed(
                bot_index,
                stream,
                crypt,
                inflater,
                realm.as_deref_mut(),
                options,
                result,
            )
            .await?;
            result.equipment_set_generated_guid = Some(guid);
            loot_race::logout_and_wait_routed_like_cpp(
                bot_index,
                stream,
                crypt,
                inflater,
                realm.as_deref_mut(),
                bot.character_guid,
                result,
            )
            .await?;
            let bot_for_db = bot.clone();
            let options_for_db = options.clone();
            result.equipment_set_db_persisted = tokio::task::spawn_blocking(move || {
                verify_equipment_set_db_row(&bot_for_db, &options_for_db, guid)
            })
            .await
            .map_err(|error| anyhow!("Equipment-set persistence worker failed: {error}"))??;
            if !result.equipment_set_db_persisted {
                bail!("equipment/transmog set did not persist exactly after logout");
            }
            result.equipment_set_smoke_passed = Some(true);
        }
        EquipmentSetSmokePhase::VerifyRelog => {
            if result.equipment_set_login_count != Some(1) || !result.equipment_set_relogin_verified
            {
                bail!(
                    "fresh relog did not load exactly the expected set (count={:?}, matched={})",
                    result.equipment_set_login_count,
                    result.equipment_set_relogin_verified
                );
            }
            let guid = options
                .expected_guid
                .context("equipment-set relog phase missing expected GUID")?;
            loot_race::logout_and_wait_routed_like_cpp(
                bot_index,
                stream,
                crypt,
                inflater,
                realm.as_deref_mut(),
                bot.character_guid,
                result,
            )
            .await?;
            let bot_for_db = bot.clone();
            let options_for_db = options.clone();
            result.equipment_set_db_persisted = tokio::task::spawn_blocking(move || {
                verify_equipment_set_db_row(&bot_for_db, &options_for_db, guid)
            })
            .await
            .map_err(|error| anyhow!("Equipment-set relog DB worker failed: {error}"))??;
            result.equipment_set_smoke_passed = Some(result.equipment_set_db_persisted);
        }
    }
    Ok(())
}

async fn run_equipment_set_race_workflow(
    mut bots: Vec<config::BotConfig>,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    account_a: String,
    account_b: String,
    timeout_secs: u64,
) -> Result<Vec<BotRunResult>> {
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&account_a) {
            0
        } else if bot.account.eq_ignore_ascii_case(&account_b) {
            1
        } else {
            2
        }
    });
    if bots.len() != 2
        || !bots[0].account.eq_ignore_ascii_case(&account_a)
        || !bots[1].account.eq_ignore_ascii_case(&account_b)
    {
        bail!("configured equipment-set race accounts were not both found exactly once");
    }
    let bots_for_setup = bots.clone();
    let fixture =
        tokio::task::spawn_blocking(move || prepare_equipment_set_smoke_fixture(&bots_for_setup))
            .await
            .map_err(|error| anyhow!("Equipment-set fixture setup worker failed: {error}"))??;

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let base = [
        EquipmentSetSmokeOptions {
            phase: EquipmentSetSmokePhase::Save,
            set_type: 0,
            set_id: 7,
            set_name: "QA Equipment".to_string(),
            set_icon: "INV_Sword_01".to_string(),
            expected_guid: None,
            save_barrier: Some(std::sync::Arc::clone(&barrier)),
            timeout_secs,
        },
        EquipmentSetSmokeOptions {
            phase: EquipmentSetSmokePhase::Save,
            set_type: 1,
            set_id: 8,
            set_name: "QA Transmog".to_string(),
            set_icon: "INV_Chest_Cloth_01".to_string(),
            expected_guid: None,
            save_barrier: Some(barrier),
            timeout_secs,
        },
    ];

    let first = tokio::join!(
        run_bot(
            bots[0].clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(base[0].clone()),
            None,
        ),
        run_bot(
            bots[1].clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(base[1].clone()),
            None,
        )
    );

    let workflow = async {
        let mut saved = vec![first.0?, first.1?];
        if saved
            .iter()
            .any(|result| !result.equipment_set_smoke_passed.unwrap_or(false))
        {
            let failures = saved
                .iter()
                .filter_map(|result| {
                    (!result.equipment_set_smoke_passed.unwrap_or(false)).then(|| {
                        format!(
                            "{}: {}",
                            result.account,
                            result
                                .equipment_set_failure
                                .as_deref()
                                .unwrap_or("missing success verdict")
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!("concurrent equipment-set save phase failed: {failures}");
        }
        let guids = [
            saved[0]
                .equipment_set_generated_guid
                .context("equipment-set bot omitted generated GUID")?,
            saved[1]
                .equipment_set_generated_guid
                .context("transmog-set bot omitted generated GUID")?,
        ];
        if guids[0] == guids[1] || guids.iter().any(|guid| *guid <= fixture.initial_max_guid) {
            bail!(
                "shared allocator proof failed: initial max={}, generated={guids:?}",
                fixture.initial_max_guid
            );
        }

        let mut verify = base.clone();
        for index in 0..2 {
            verify[index].phase = EquipmentSetSmokePhase::VerifyRelog;
            verify[index].expected_guid = Some(guids[index]);
            verify[index].save_barrier = None;
        }
        let relog = tokio::join!(
            run_bot(
                bots[0].clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(verify[0].clone()),
                None,
            ),
            run_bot(
                bots[1].clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(verify[1].clone()),
                None,
            )
        );
        let reloaded = [relog.0?, relog.1?];
        for index in 0..2 {
            saved[index].world_auth &= reloaded[index].world_auth;
            saved[index].enum_characters &= reloaded[index].enum_characters;
            saved[index].player_login_verified &= reloaded[index].player_login_verified;
            saved[index].equipment_set_login_count = reloaded[index].equipment_set_login_count;
            saved[index].equipment_set_load_seen = reloaded[index].equipment_set_load_seen;
            saved[index].equipment_set_relogin_verified =
                reloaded[index].equipment_set_relogin_verified;
            saved[index].equipment_set_db_persisted &= reloaded[index].equipment_set_db_persisted;
            saved[index]
                .seen_opcodes
                .extend(reloaded[index].seen_opcodes.clone());
            saved[index].equipment_set_failure = reloaded[index].equipment_set_failure.clone();
            saved[index].equipment_set_smoke_passed = Some(
                saved[index].equipment_set_db_persisted
                    && saved[index].equipment_set_relogin_verified
                    && reloaded[index].equipment_set_smoke_passed.unwrap_or(false),
            );
        }
        Ok::<_, anyhow::Error>(saved)
    }
    .await;

    let bots_for_cleanup = bots.clone();
    let cleanup =
        tokio::task::spawn_blocking(move || cleanup_equipment_set_smoke_fixture(&bots_for_cleanup))
            .await
            .map_err(|error| anyhow!("Equipment-set cleanup worker failed: {error}"))?;
    match (workflow, cleanup) {
        (Ok(results), Ok(())) => Ok(results),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(workflow_error), Ok(())) => Err(workflow_error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "equipment-set workflow failed: {workflow_error}; cleanup failed: {cleanup_error}"
        )),
    }
}

async fn run_bank_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_bank_smoke_fixture(&bot_for_setup, item_entry, runtime_counter, timeout_secs)
    })
    .await
    .map_err(|e| anyhow!("Bank smoke setup DB worker join failed: {e}"))??;

    let mut deposit_options = fixture.options.clone();
    deposit_options.phase = BankSmokePhase::Deposit;
    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        Some(deposit_options),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            let bot_for_cleanup = bot.clone();
            let fixture_for_cleanup = fixture.clone();
            let _ = tokio::task::spawn_blocking(move || {
                cleanup_bank_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await;
            return Err(error.context("Bank smoke deposit login/phase failed"));
        }
    };

    if combined.bank_smoke_passed.unwrap_or(false) {
        let mut withdraw_options = fixture.options.clone();
        withdraw_options.phase = BankSmokePhase::Withdraw;
        match run_bot(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            Some(withdraw_options),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(second) => {
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.bank_open_confirmed &= second.bank_open_confirmed;
                combined.bank_relogin_after_deposit = second.bank_relogin_after_deposit;
                combined.bank_withdraw_persisted = second.bank_withdraw_persisted;
                combined.seen_opcodes.extend(second.seen_opcodes);
                combined.bank_failure = second.bank_failure;
                combined.bank_smoke_passed = Some(
                    combined.bank_deposit_persisted
                        && combined.bank_relogin_after_deposit
                        && combined.bank_withdraw_persisted
                        && second.bank_smoke_passed.unwrap_or(false),
                );
            }
            Err(error) => {
                combined.bank_failure = Some(format!("Withdrawal relog/phase failed: {error}"));
                combined.bank_smoke_passed = Some(false);
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_bank_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|e| anyhow!("Bank smoke cleanup DB worker join failed: {e}"))?;
    if let Err(error) = cleanup {
        combined.bank_failure = Some(format!("Bank fixture cleanup failed: {error}"));
        combined.bank_smoke_passed = Some(false);
    }

    Ok(combined)
}

async fn run_void_storage_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_void_storage_smoke_fixture(
            &bot_for_setup,
            item_entry,
            runtime_counter,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Void-storage fixture setup worker failed: {error}"))??;

    let workflow = async {
        let mut unlock_deposit = fixture.options.clone();
        unlock_deposit.phase = VoidStorageSmokePhase::UnlockDeposit;
        let mut combined = run_bot_with_void_storage(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(unlock_deposit),
        )
        .await?;
        if !combined.void_storage_smoke_passed.unwrap_or(false) {
            return Ok(combined);
        }
        let void_item_id = combined
            .void_storage_item_id
            .context("void-storage deposit response omitted its generated item ID")?;

        let phases = [
            VoidStorageSmokePhase::VerifyDepositSwap,
            VoidStorageSmokePhase::VerifySwapWithdraw,
            VoidStorageSmokePhase::VerifyWithdraw,
        ];
        for phase in phases {
            let mut options = fixture.options.clone();
            options.phase = phase;
            options.expected_void_item_id = Some(void_item_id);
            options.expected_void_slot = match phase {
                VoidStorageSmokePhase::VerifyDepositSwap => 0,
                VoidStorageSmokePhase::VerifySwapWithdraw => 5,
                VoidStorageSmokePhase::VerifyWithdraw => 0,
                VoidStorageSmokePhase::UnlockDeposit | VoidStorageSmokePhase::QueryCapture => {
                    unreachable!()
                }
            };
            let next = run_bot_with_void_storage(
                bot.clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(options),
            )
            .await?;
            combined.world_auth &= next.world_auth;
            combined.enum_characters &= next.enum_characters;
            combined.player_login_verified &= next.player_login_verified;
            combined.void_storage_deposit_relogin_verified |=
                next.void_storage_deposit_relogin_verified;
            combined.void_storage_swap_persisted |= next.void_storage_swap_persisted;
            combined.void_storage_swap_relogin_verified |= next.void_storage_swap_relogin_verified;
            combined.void_storage_withdraw_persisted |= next.void_storage_withdraw_persisted;
            combined.void_storage_withdraw_relogin_verified |=
                next.void_storage_withdraw_relogin_verified;
            combined.seen_opcodes.extend(next.seen_opcodes);
            if !next.void_storage_smoke_passed.unwrap_or(false) {
                combined.void_storage_failure = next.void_storage_failure;
                combined.void_storage_smoke_passed = Some(false);
                break;
            }
        }
        combined.void_storage_smoke_passed = Some(
            combined.void_storage_unlock_persisted
                && combined.void_storage_deposit_persisted
                && combined.void_storage_deposit_relogin_verified
                && combined.void_storage_swap_persisted
                && combined.void_storage_swap_relogin_verified
                && combined.void_storage_withdraw_persisted
                && combined.void_storage_withdraw_relogin_verified,
        );
        Ok::<_, anyhow::Error>(combined)
    }
    .await;

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_void_storage_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|error| anyhow!("Void-storage cleanup worker failed: {error}"))?;

    match (workflow, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(mut result), Err(error)) => {
            result.void_storage_smoke_passed = Some(false);
            result.void_storage_failure = Some(format!("fixture cleanup failed: {error}"));
            Ok(result)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "Void-storage workflow failed: {workflow_error}; cleanup failed: {cleanup_error}"
        )),
    }
}

async fn run_void_storage_query_capture_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_void_storage_query_capture_fixture(
            &bot_for_setup,
            item_entry,
            runtime_counter,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Void-storage query fixture setup worker failed: {error}"))??;

    let workflow = run_bot_with_void_storage(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(fixture.options.clone()),
    )
    .await;

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_void_storage_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|error| anyhow!("Void-storage query cleanup worker failed: {error}"))?;

    match (workflow, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Ok(mut result), Err(error)) => {
            result.void_storage_query_capture_passed = Some(false);
            result.void_storage_failure = Some(format!("fixture cleanup failed: {error}"));
            Ok(result)
        }
        (Err(error), Ok(())) => Err(error),
        (Err(workflow_error), Err(cleanup_error)) => Err(anyhow!(
            "Void-storage query workflow failed: {workflow_error}; cleanup failed: {cleanup_error}"
        )),
    }
}

async fn run_homebind_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_homebind_smoke_fixture(&bot_for_setup, runtime_counter, timeout_secs)
    })
    .await
    .map_err(|e| anyhow!("Homebind smoke setup DB worker join failed: {e}"))??;

    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        Some(fixture.options.clone()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            let bot_for_cleanup = bot.clone();
            let fixture_for_cleanup = fixture.clone();
            let cleanup = tokio::task::spawn_blocking(move || {
                cleanup_homebind_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await
            .map_err(|join_error| {
                anyhow!(
                    "Homebind smoke bind login/phase failed: {error}; cleanup worker failed: {join_error}"
                )
            })?;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "Homebind smoke bind login/phase failed: {error}; fixture cleanup failed: {cleanup_error}"
                );
            }
            return Err(error.context("Homebind smoke bind login/phase failed"));
        }
    };

    if combined.homebind_smoke_passed.unwrap_or(false) {
        let mut relog_options = fixture.options.clone();
        relog_options.phase = HomebindSmokePhase::VerifyRelog;
        let bot_for_db = bot.clone();
        relog_options.expected_homebind =
            match tokio::task::spawn_blocking(move || load_homebind_row(&bot_for_db)).await {
                Ok(Ok(Some(row))) => Some(row),
                Ok(Ok(None)) => {
                    combined.homebind_failure =
                        Some("character_homebind disappeared before relog".to_string());
                    combined.homebind_smoke_passed = Some(false);
                    None
                }
                Ok(Err(error)) => {
                    combined.homebind_failure =
                        Some(format!("Homebind expected-row query failed: {error}"));
                    combined.homebind_smoke_passed = Some(false);
                    None
                }
                Err(error) => {
                    combined.homebind_failure =
                        Some(format!("Homebind expected-row worker join failed: {error}"));
                    combined.homebind_smoke_passed = Some(false);
                    None
                }
            };
        if relog_options.expected_homebind.is_some() {
            match run_bot(
                bot.clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                Some(relog_options),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            {
                Ok(second) => {
                    combined.world_auth &= second.world_auth;
                    combined.enum_characters &= second.enum_characters;
                    combined.player_login_verified &= second.player_login_verified;
                    combined.homebind_relogin_verified = second.homebind_relogin_verified;
                    combined.seen_opcodes.extend(second.seen_opcodes);
                    combined.homebind_failure = second.homebind_failure;
                    combined.homebind_smoke_passed = Some(
                        combined.homebind_spell_go_seen
                            && combined.homebind_bind_point_update_seen
                            && combined.homebind_player_bound_seen
                            && combined.homebind_gossip_complete_seen
                            && combined.homebind_db_persisted
                            && combined.homebind_relogin_verified
                            && second.homebind_smoke_passed.unwrap_or(false),
                    );
                }
                Err(error) => {
                    combined.homebind_failure = Some(format!("Homebind relog failed: {error}"));
                    combined.homebind_smoke_passed = Some(false);
                }
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_homebind_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|e| anyhow!("Homebind smoke cleanup DB worker join failed: {e}"))?;
    if let Err(error) = cleanup {
        combined.homebind_failure = Some(format!("Homebind fixture cleanup failed: {error}"));
        combined.homebind_smoke_passed = Some(false);
    }

    Ok(combined)
}

async fn run_homebind_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &HomebindSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.phase == HomebindSmokePhase::Bind {
        let mut runtime_components = (!options.discover_runtime_guid).then(|| {
            let (_, low, high) = parse_packed_guid(&options.innkeeper.packed_guid)
                .expect("fixture packed GUID was built locally");
            (low, high)
        });
        let drain_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < drain_deadline {
            match tokio::time::timeout(
                Duration::from_millis(250),
                read_encrypted_packet(stream, crypt, server_inflater),
            )
            .await
            {
                Ok(Ok((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    if opcode == SMSG_UPDATE_OBJECT {
                        if let Some((low, high)) = find_creature_guid_in_update_object(
                            &payload,
                            options.innkeeper.map_id,
                            options.innkeeper.entry,
                        ) {
                            runtime_components = Some((low, high));
                            result.homebind_innkeeper_guid_counter = Some(low & 0xFF_FFFF_FFFF);
                        }
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
        }
        let (runtime_low, runtime_high) = runtime_components.ok_or_else(|| {
            anyhow!(
                "innkeeper entry {} was not discovered in login SMSG_UPDATE_OBJECT packets",
                options.innkeeper.entry
            )
        })?;
        let realm = realm_connection.as_mut().context(
            "homebind smoke requires distinct realm/instance sockets to validate C++ routing",
        )?;
        loop {
            match tokio::time::timeout(
                Duration::from_millis(250),
                read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
            )
            .await
            {
                Ok(Ok((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    info!(
                        "[Bot {}] 📦 realm login drain {}",
                        bot_index,
                        parse_packet(opcode, &payload)
                    );
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
        }
        let runtime_guid = build_packed_guid(runtime_low, runtime_high);
        send_encrypted_packet(stream, crypt, CMSG_BINDER_ACTIVATE, &runtime_guid).await?;
        info!(
            "[Bot {}] ✅ CMSG_BINDER_ACTIVATE sent to entry={} spawn={}",
            bot_index, options.innkeeper.entry, options.innkeeper.spawn_guid
        );

        let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
        let mut bind_packet_homebind = None;
        let mut pending_player_bound = None;
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            enum HomebindReady {
                Instance,
                Realm,
            }
            let mut instance_peek = [0u8; 1];
            let mut realm_peek = [0u8; 1];
            let ready = tokio::time::timeout(remaining, async {
                tokio::select! {
                    result = stream.peek(&mut instance_peek) => {
                        if result.context("homebind instance peek failed")? == 0 {
                            bail!("instance connection closed during homebind smoke");
                        }
                        Ok(HomebindReady::Instance)
                    }
                    result = realm.stream.peek(&mut realm_peek) => {
                        if result.context("homebind realm peek failed")? == 0 {
                            bail!("realm connection closed during homebind smoke");
                        }
                        Ok(HomebindReady::Realm)
                    }
                }
            })
            .await;
            let ready = match ready {
                Ok(result) => result?,
                Err(_) => break,
            };
            let (connection, opcode, payload) = match ready {
                HomebindReady::Instance => {
                    let (opcode, payload) = tokio::time::timeout(
                        deadline.saturating_duration_since(tokio::time::Instant::now()),
                        read_encrypted_packet(stream, crypt, server_inflater),
                    )
                    .await
                    .map_err(|_| anyhow!("homebind instance packet read timed out"))??;
                    ("instance", opcode, payload)
                }
                HomebindReady::Realm => {
                    let (opcode, payload) = tokio::time::timeout(
                        deadline.saturating_duration_since(tokio::time::Instant::now()),
                        read_encrypted_packet(
                            &mut realm.stream,
                            &mut realm.crypt,
                            &mut realm.inflater,
                        ),
                    )
                    .await
                    .map_err(|_| anyhow!("homebind realm packet read timed out"))??;
                    ("realm", opcode, payload)
                }
            };
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            info!(
                "[Bot {}] 📦 {} {}",
                bot_index,
                connection,
                parse_packet(opcode, &payload)
            );
            match (connection, opcode) {
                ("instance", SMSG_SPELL_GO) => {
                    let player_high = (2u64 << 58) | ((u64::from(realm_id()) & 0x1FFF) << 42);
                    result.homebind_spell_go_seen = homebind_spell_go_seen_after_packet(
                        result.homebind_spell_go_seen,
                        &payload,
                        runtime_low,
                        runtime_high,
                        bot.character_guid,
                        player_high,
                    );
                }
                ("instance", SMSG_BIND_POINT_UPDATE) => {
                    if let Some(homebind) =
                        parse_bind_point_update(&payload, options.innkeeper.orientation)
                    {
                        result.homebind_bind_point_update_seen = true;
                        if let Some(player_bound) = pending_player_bound.as_deref() {
                            result.homebind_player_bound_seen = player_bound_matches(
                                player_bound,
                                runtime_low,
                                runtime_high,
                                u32::from(homebind.zone_id),
                            );
                        }
                        bind_packet_homebind = Some(homebind);
                    }
                }
                ("realm", SMSG_PLAYER_BOUND) => {
                    if let Some(expected) = bind_packet_homebind.as_ref() {
                        result.homebind_player_bound_seen = player_bound_matches(
                            &payload,
                            runtime_low,
                            runtime_high,
                            u32::from(expected.zone_id),
                        );
                    }
                    pending_player_bound = Some(payload);
                }
                ("realm", SMSG_GOSSIP_COMPLETE) => result.homebind_gossip_complete_seen = true,
                ("instance", SMSG_PLAYER_BOUND | SMSG_GOSSIP_COMPLETE) => {
                    bail!(
                        "{} arrived on instance; C++ routes it on realm",
                        parse_packet(opcode, &payload)
                    );
                }
                ("realm", SMSG_SPELL_GO | SMSG_BIND_POINT_UPDATE) => {
                    bail!(
                        "{} arrived on realm; C++ routes it on instance",
                        parse_packet(opcode, &payload)
                    );
                }
                _ => {}
            }
            if result.homebind_spell_go_seen
                && result.homebind_bind_point_update_seen
                && result.homebind_player_bound_seen
                && result.homebind_gossip_complete_seen
            {
                break;
            }
        }
        if !(result.homebind_spell_go_seen
            && result.homebind_bind_point_update_seen
            && result.homebind_player_bound_seen
            && result.homebind_gossip_complete_seen)
        {
            bail!(
                "missing bind responses: spell_go={} bind_update={} player_bound={} gossip_complete={}",
                result.homebind_spell_go_seen,
                result.homebind_bind_point_update_seen,
                result.homebind_player_bound_seen,
                result.homebind_gossip_complete_seen
            );
        }
        let expected_homebind = bind_packet_homebind
            .ok_or_else(|| anyhow!("BindPointUpdate payload could not be decoded"))?;
        let bot_for_db = bot.clone();
        let persistence_timeout = Duration::from_secs(options.timeout_secs.clamp(1, 10));
        result.homebind_db_persisted = tokio::task::spawn_blocking(move || {
            wait_for_homebind_row(&bot_for_db, &expected_homebind, persistence_timeout)
        })
        .await
        .map_err(|e| anyhow!("Homebind DB verification worker join failed: {e}"))??;
        if !result.homebind_db_persisted {
            bail!("character_homebind did not persist the live bind location");
        }
    } else {
        let expected_homebind = options
            .expected_homebind
            .clone()
            .ok_or_else(|| anyhow!("Homebind relog phase missing expected complete row"))?;
        let bot_for_db = bot.clone();
        result.homebind_relogin_verified = tokio::task::spawn_blocking(move || {
            Ok::<_, anyhow::Error>(
                load_homebind_row(&bot_for_db)?.as_ref() == Some(&expected_homebind),
            )
        })
        .await
        .map_err(|e| anyhow!("Homebind relog DB worker join failed: {e}"))??;
        if !result.homebind_relogin_verified {
            bail!("character_homebind changed before the verification relog completed");
        }
    }

    logout_and_wait(bot_index, stream, crypt, server_inflater, result).await?;
    result.homebind_smoke_passed = Some(true);
    Ok(())
}

async fn run_rested_xp_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &RestedXpSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    match options.phase {
        RestedXpSmokePhase::OfflineWilderness | RestedXpSmokePhase::OfflineResting => {
            disconnect_rested_xp_and_wait(
                bot_index,
                bot,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options.timeout_secs,
                result,
            )
            .await?;
            result.rested_xp_smoke_passed = Some(true);
            return Ok(());
        }
        RestedXpSmokePhase::VerifyRelog => {
            let expected_xp = options
                .expected_xp
                .context("rested-XP relog phase missing expected XP")?;
            let expected_rest_bonus = options
                .expected_rest_bonus
                .context("rested-XP relog phase missing expected rest bonus")?;
            let before_logout = load_rested_xp_db_state_async(bot.clone()).await?;
            validate_rested_xp_persistence_state(
                before_logout,
                options.test_level,
                expected_xp,
                expected_rest_bonus,
                1,
                "after relog",
            )?;
            disconnect_rested_xp_and_wait(
                bot_index,
                bot,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options.timeout_secs,
                result,
            )
            .await?;
            let after_logout = load_rested_xp_db_state_async(bot.clone()).await?;
            validate_rested_xp_persistence_state(
                after_logout,
                options.test_level,
                expected_xp,
                expected_rest_bonus,
                0,
                "after relog logout",
            )?;
            result.rested_xp_relog_verified = true;
            result.rested_xp_smoke_passed = Some(true);
            return Ok(());
        }
        RestedXpSmokePhase::ConsumeKill => {}
    }

    let realm = realm_connection.as_mut().context(
        "rested-XP smoke requires distinct realm/instance sockets to validate XP routing",
    )?;
    let realm_drain_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < realm_drain_deadline {
        let remaining = realm_drain_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "rested-XP realm login drain",
        )
        .await?
        else {
            break;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 realm rested-XP login drain {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
    }

    // C++ Player::CanNeverSee returns true until the client acknowledges that
    // its active mover is initialized. Send the ACK before discovery because
    // C++ may defer the target's visibility update until this packet, whereas
    // Rust currently can have queued the CREATE slightly earlier.
    let active_mover_complete = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &active_mover_complete,
    )
    .await?;
    info!(
        "[Bot {}] ✅ CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE sent before live target discovery",
        bot_index
    );

    // Search until the declared deadline, not merely until the first 250 ms
    // gap. This also drains the visibility work caused by the active-mover ACK.
    // Stop as soon as the CREATE_OBJECT candidate is decoded so its live
    // position is still fresh enough for deterministic engagement below.
    let expected_runtime_counter = (options.target.guid_counter != 0)
        .then_some(options.target.guid_counter & OBJECT_GUID_COUNTER_MASK);
    let mut discovered = None;
    let discovery_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while discovered.is_none() && tokio::time::Instant::now() < discovery_deadline {
        let remaining = discovery_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "rested-XP instance login discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_UPDATE_OBJECT {
            discovered = find_creature_guid_near_position_in_update_object(
                &payload,
                options.target.map_id,
                options.target.entry,
                options.target.x as f32,
                options.target.y as f32,
                options.target.z as f32,
                options.target_match_radius,
                expected_runtime_counter,
            );
        }
    }

    let candidate = resolve_rested_xp_runtime_target(&options.target, discovered)?;
    let player_x = candidate.x + 1.0;
    let player_y = candidate.y;
    let player_z = candidate.z;
    let player_orientation = (candidate.y - player_y).atan2(candidate.x - player_x);
    let player_distance = ((candidate.x - player_x).powi(2)
        + (candidate.y - player_y).powi(2)
        + (candidate.z - player_z).powi(2))
    .sqrt();
    if player_distance > NOMINAL_MELEE_RANGE_LIKE_CPP {
        bail!(
            "rested-XP live engagement placement remained {player_distance:.2} yards from the target (C++ nominal melee range is {NOMINAL_MELEE_RANGE_LIKE_CPP:.2})"
        );
    }
    let (player_low, player_high) = create_player_guid_raw(bot.character_guid, realm_id());
    let movement = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x,
        player_y,
        player_z,
        player_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &movement).await?;
    info!(
        "[Bot {}] ✅ CMSG_MOVE_HEARTBEAT placed the bot {:.2} yards from the live target and facing it",
        bot_index, player_distance
    );
    let (runtime_low, runtime_high) = (candidate.low, candidate.high);
    result.rested_xp_target_guid_counter = Some(runtime_low & OBJECT_GUID_COUNTER_MASK);
    let packed_target = build_packed_guid(runtime_low, runtime_high);
    send_encrypted_packet(stream, crypt, CMSG_ATTACK_SWING, &packed_target).await?;
    info!(
        "[Bot {}] ✅ CMSG_ATTACK_SWING sent on instance to entry={} spawn={} counter={}",
        bot_index,
        options.target.entry,
        options.target.spawn_guid,
        runtime_low & OBJECT_GUID_COUNTER_MASK
    );

    enum RestedXpReady {
        Instance,
        Realm,
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    let client_clock_origin = tokio::time::Instant::now();
    let mut player_damage_observed = 0i64;
    let mut target_death_observed = false;
    let xp_gain = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for realm SMSG_LOG_XP_GAIN after observing {player_damage_observed} player damage (target_death={target_death_observed})"
            );
        }
        let mut instance_peek = [0u8; 1];
        let mut realm_peek = [0u8; 1];
        let ready = tokio::time::timeout(remaining, async {
            tokio::select! {
                ready = stream.peek(&mut instance_peek) => {
                    if ready.context("rested-XP instance peek failed")? == 0 {
                        bail!("instance connection closed during rested-XP combat");
                    }
                    Ok(RestedXpReady::Instance)
                }
                ready = realm.stream.peek(&mut realm_peek) => {
                    if ready.context("rested-XP realm peek failed")? == 0 {
                        bail!("realm connection closed during rested-XP combat");
                    }
                    Ok(RestedXpReady::Realm)
                }
            }
        })
        .await
        .map_err(|_| {
            anyhow!(
                "timed out waiting for rested-XP combat packets after observing {player_damage_observed} player damage (target_death={target_death_observed})"
            )
        })??;
        let (connection, opcode, payload) = match ready {
            RestedXpReady::Instance => {
                let (opcode, payload) = tokio::time::timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(stream, crypt, server_inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP instance packet read timed out"))??;
                ("instance", opcode, payload)
            }
            RestedXpReady::Realm => {
                let (opcode, payload) = tokio::time::timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP realm packet read timed out"))??;
                ("realm", opcode, payload)
            }
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 {} rested-XP {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            if connection != "instance" {
                bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during rested-XP combat");
            }
            let sequence_index = parse_time_sync_request_sequence(&payload)?;
            let client_time = client_clock_origin.elapsed().as_millis() as u32;
            let response = build_time_sync_response_payload(sequence_index, client_time);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            info!(
                "[Bot {}] ✅ rested-XP CMSG_TIME_SYNC_RESPONSE sent (sequence={}, client_time={})",
                bot_index, sequence_index, client_time
            );
            continue;
        }
        if opcode == SMSG_ATTACKER_STATE_UPDATE {
            let update = parse_attacker_state_update_summary(&payload)
                .context("malformed SMSG_ATTACKER_STATE_UPDATE during rested-XP combat")?;
            if (update.attacker_guid_low, update.attacker_guid_high) == (player_low, player_high)
                && (update.victim_guid_low, update.victim_guid_high) == (runtime_low, runtime_high)
            {
                if update.damage < 0 {
                    bail!(
                        "rested-XP player auto-attack reported negative damage {}",
                        update.damage
                    );
                }
                // C++ can emit zero-damage MISS/DODGE/PARRY swings. They are
                // valid combat progress, but only positive damage contributes
                // to the proof that this bot killed the selected target.
                if update.damage > 0 {
                    player_damage_observed += i64::from(update.damage);
                }
                target_death_observed |= update.over_damage >= 0;
            }
        }
        if opcode == SMSG_ATTACK_STOP {
            let stop = parse_attack_stop_summary(&payload)
                .context("malformed SMSG_ATTACK_STOP payload during rested-XP combat")?;
            let attacker = (stop.attacker_guid_low, stop.attacker_guid_high);
            let victim = (stop.victim_guid_low, stop.victim_guid_high);
            if attacker == (runtime_low, runtime_high) && victim == (player_low, player_high) {
                if stop.now_dead {
                    bail!("the rested-XP target killed the bot before XP was awarded");
                }
                // C++ CombatStop can emit the reciprocal creature->player stop
                // before the player's target-death stop.
                continue;
            }
            if attacker != (player_low, player_high) || victim != (runtime_low, runtime_high) {
                continue;
            }
            if stop.now_dead {
                // C++ sends AttackStop when the victim dies. XP can arrive on
                // the realm connection immediately before or after this
                // instance packet, so keep polling both sockets.
                target_death_observed = true;
                continue;
            }
            bail!("server stopped the rested-XP attack while the target was still alive");
        }
        if opcode != SMSG_LOG_XP_GAIN {
            continue;
        }
        if connection != "realm" {
            bail!("SMSG_LOG_XP_GAIN arrived on instance; C++ routes it on realm");
        }
        let gain =
            parse_log_xp_gain_summary(&payload).context("malformed SMSG_LOG_XP_GAIN payload")?;
        break gain;
    };

    if (xp_gain.victim_guid_low, xp_gain.victim_guid_high) != (runtime_low, runtime_high) {
        bail!("SMSG_LOG_XP_GAIN victim did not match the attacked creature");
    }
    if player_damage_observed <= 0 {
        bail!("rested-XP reward arrived without any positive player damage to the target");
    }
    if xp_gain.reason != 0 || xp_gain.amount <= 0 {
        bail!(
            "SMSG_LOG_XP_GAIN was not a positive kill reward: reason={} amount={}",
            xp_gain.reason,
            xp_gain.amount
        );
    }
    if xp_gain.original != xp_gain.amount.saturating_mul(2) {
        bail!(
            "rested kill was not exactly 200% XP: amount={} original={}",
            xp_gain.amount,
            xp_gain.original
        );
    }
    if (xp_gain.group_bonus - 1.0).abs() > f32::EPSILON {
        bail!("unexpected rested-XP group bonus: {}", xp_gain.group_bonus);
    }
    let expected_xp = u32::try_from(xp_gain.original)
        .context("positive SMSG_LOG_XP_GAIN original did not fit u32")?;
    let spent = xp_gain.amount as f32;
    let expected_rest_bonus = options.seeded_rest_bonus - spent;
    if expected_rest_bonus < 0.0 {
        bail!("fixture rest bonus was smaller than the awarded base XP");
    }
    observe_instance_after_realm_xp(bot_index, stream, crypt, server_inflater, result).await?;
    // C++ Player::GiveXP mutates update fields in memory; persistence occurs
    // during the later character save. Disconnect both sockets and wait for a
    // stable offline row before asserting DB state so this workflow is valid
    // against both the legacy server and RustyCore. This also avoids coupling
    // rested-XP QA to C++'s unrelated 20-second wilderness logout delay.
    disconnect_rested_xp_and_wait(
        bot_index,
        bot,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options.timeout_secs,
        result,
    )
    .await?;
    let persisted = load_rested_xp_db_state_async(bot.clone()).await?;
    validate_rested_xp_persistence_state(
        persisted,
        options.test_level,
        expected_xp,
        expected_rest_bonus,
        0,
        "after rested-XP disconnect save",
    )?;
    result.rested_xp_packet_amount = Some(xp_gain.amount);
    result.rested_xp_packet_original = Some(xp_gain.original);
    result.rested_xp_db_xp_before = Some(0);
    result.rested_xp_db_xp_after = Some(persisted.xp);
    result.rested_xp_db_rest_before = Some(options.seeded_rest_bonus);
    result.rested_xp_db_rest_after = Some(persisted.rest_bonus);
    result.rested_xp_smoke_passed = Some(true);
    Ok(())
}

fn validate_rested_xp_instance_post_realm_opcode(opcode: u16) -> Result<()> {
    if opcode == SMSG_LOG_XP_GAIN {
        bail!(
            "SMSG_LOG_XP_GAIN was duplicated/misrouted to instance after it arrived on realm; C++ routes it only on realm"
        );
    }
    Ok(())
}

async fn read_encrypted_packet_if_ready(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    readiness_wait: Duration,
    frame_timeout: Duration,
    context: &str,
) -> Result<Option<(u16, Vec<u8>)>> {
    // Waiting on `peek` is cancellation-safe: a readiness timeout cannot
    // consume a partial encrypted frame and desynchronize framing/crypt state.
    let mut peek = [0u8; 1];
    match tokio::time::timeout(readiness_wait, stream.peek(&mut peek)).await {
        Err(_) => return Ok(None),
        Ok(Ok(0)) => bail!("{context}: connection closed"),
        Ok(Ok(_)) => {}
        Ok(Err(error)) => return Err(anyhow!("{context}: peek failed: {error}")),
    }

    // Once data is ready, finish the whole frame. A timeout is terminal for
    // this workflow, so a partially consumed frame is never reused.
    let packet = tokio::time::timeout(
        frame_timeout,
        read_encrypted_packet(stream, crypt, server_inflater),
    )
    .await
    .map_err(|_| anyhow!("{context}: encrypted frame read timed out"))??;
    Ok(Some(packet))
}

async fn observe_instance_after_realm_xp(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + RESTED_XP_INSTANCE_OBSERVATION_WINDOW;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            remaining,
            Duration::from_secs(5),
            "rested-XP post-realm instance observation",
        )
        .await?
        else {
            return Ok(());
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance post-realm rested-XP drain {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        validate_rested_xp_instance_post_realm_opcode(opcode)?;
    }
}

fn validate_rested_xp_persistence_state(
    state: RestedXpDbState,
    expected_level: u8,
    expected_xp: u32,
    expected_rest_bonus: f32,
    expected_online: u8,
    phase: &str,
) -> Result<()> {
    validate_rested_xp_saved_state_shape(
        state,
        expected_level,
        expected_xp,
        expected_online,
        phase,
    )?;
    if (state.rest_bonus - expected_rest_bonus).abs() > 0.05 {
        bail!(
            "rested-XP persistence mismatch {phase}: expected xp/rest {expected_xp}/{expected_rest_bonus:.4}, got {}/{:.4}",
            state.xp,
            state.rest_bonus
        );
    }
    Ok(())
}

fn validate_rested_xp_saved_state_shape(
    state: RestedXpDbState,
    expected_level: u8,
    expected_xp: u32,
    expected_online: u8,
    phase: &str,
) -> Result<()> {
    // Rest bonus accrual is time-sensitive within a small tolerance. Derive
    // the state relation from the persisted value itself so a legitimate
    // boundary crossing near 1.0 is not rejected while still enforcing C++'s
    // exact SetRestBonus state rule.
    let expected_rest_state = if state.rest_bonus >= 1.0 {
        REST_STATE_RESTED
    } else {
        REST_STATE_NORMAL
    };
    if state.level != expected_level
        || state.xp != expected_xp
        || state.rest_state != expected_rest_state
        || state.online != expected_online
    {
        bail!(
            "rested-XP saved-state mismatch {phase}: expected level/xp/state/online {expected_level}/{expected_xp}/{expected_rest_state}/{expected_online}, got {}/{}/{}/{}",
            state.level,
            state.xp,
            state.rest_state,
            state.online
        );
    }
    Ok(())
}

fn validate_rested_xp_target_template(
    entry: u32,
    creature_type: u8,
    vehicle_id: u32,
) -> Result<()> {
    if creature_type == CREATURE_TYPE_CRITTER {
        bail!("rested-XP target entry {entry} is a critter; C++ dynamically marks critters NO_XP");
    }
    if vehicle_id != 0 {
        bail!(
            "rested-XP target entry {entry} has VehicleId {vehicle_id}; C++ creates it with HighGuid::Vehicle, which this creature-only smoke does not support"
        );
    }
    Ok(())
}

async fn disconnect_rested_xp_and_wait(
    bot_index: usize,
    bot: &config::BotConfig,
    instance_stream: &mut TcpStream,
    instance_crypt: &mut WorldCrypt,
    instance_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    // A bare socket loss keeps a stock C++ WorldSession alive for 60 seconds
    // (`expireTime` in WorldSession). Exercise the real logout opcode instead:
    // wilderness logout completes after C++'s 20-second countdown, while Rust
    // may complete immediately. The DB-stability wait below remains the final
    // persistence proof for both runtimes.
    send_encrypted_packet(instance_stream, instance_crypt, CMSG_LOGOUT_REQUEST, &[0])
        .await
        .context("send rested-XP CMSG_LOGOUT_REQUEST")?;
    info!("[Bot {}] ✅ rested-XP CMSG_LOGOUT_REQUEST sent", bot_index);
    let logout_wait_secs = timeout_secs.min(NORMAL_LOGOUT_COMPLETE_WAIT_SECS);
    let logout_deadline = tokio::time::Instant::now() + Duration::from_secs(logout_wait_secs);
    let client_clock_origin = tokio::time::Instant::now();
    let mut logout_complete = false;
    let mut instance_open = true;
    let mut realm_open = realm_connection.is_some();
    enum RestedXpLogoutReady {
        Instance,
        Realm,
        InstanceClosed,
        RealmClosed,
    }
    while tokio::time::Instant::now() < logout_deadline {
        let remaining = logout_deadline.saturating_duration_since(tokio::time::Instant::now());
        if !instance_open && !realm_open {
            break;
        }
        let ready = match (instance_open, realm_open) {
            (true, true) => {
                let realm = realm_connection
                    .as_mut()
                    .context("rested-XP logout lost its realm connection")?;
                tokio::time::timeout(remaining, async {
                    let mut instance_peek = [0u8; 1];
                    let mut realm_peek = [0u8; 1];
                    tokio::select! {
                        ready = instance_stream.peek(&mut instance_peek) => {
                            if ready.context("rested-XP logout instance peek failed")? == 0 {
                                Ok(RestedXpLogoutReady::InstanceClosed)
                            } else {
                                Ok(RestedXpLogoutReady::Instance)
                            }
                        }
                        ready = realm.stream.peek(&mut realm_peek) => {
                            if ready.context("rested-XP logout realm peek failed")? == 0 {
                                Ok(RestedXpLogoutReady::RealmClosed)
                            } else {
                                Ok(RestedXpLogoutReady::Realm)
                            }
                        }
                    }
                })
                .await
            }
            (true, false) => {
                tokio::time::timeout(remaining, async {
                    let mut peek = [0u8; 1];
                    if instance_stream
                        .peek(&mut peek)
                        .await
                        .context("rested-XP logout instance peek failed")?
                        == 0
                    {
                        Ok(RestedXpLogoutReady::InstanceClosed)
                    } else {
                        Ok(RestedXpLogoutReady::Instance)
                    }
                })
                .await
            }
            (false, true) => {
                let realm = realm_connection
                    .as_mut()
                    .context("rested-XP logout lost its realm connection")?;
                tokio::time::timeout(remaining, async {
                    let mut peek = [0u8; 1];
                    if realm
                        .stream
                        .peek(&mut peek)
                        .await
                        .context("rested-XP logout realm peek failed")?
                        == 0
                    {
                        Ok(RestedXpLogoutReady::RealmClosed)
                    } else {
                        Ok(RestedXpLogoutReady::Realm)
                    }
                })
                .await
            }
            (false, false) => unreachable!(),
        };
        let ready = match ready {
            Ok(Ok(ready)) => ready,
            Ok(Err(error)) => return Err(error),
            Err(_) => break,
        };
        match ready {
            RestedXpLogoutReady::InstanceClosed => {
                instance_open = false;
                continue;
            }
            RestedXpLogoutReady::RealmClosed => {
                realm_open = false;
                continue;
            }
            RestedXpLogoutReady::Instance | RestedXpLogoutReady::Realm => {}
        }
        let (connection, opcode, payload) = match ready {
            RestedXpLogoutReady::Instance => {
                let (opcode, payload) = tokio::time::timeout(
                    logout_deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(instance_stream, instance_crypt, instance_inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP logout instance packet read timed out"))??;
                ("instance", opcode, payload)
            }
            RestedXpLogoutReady::Realm => {
                let realm = realm_connection
                    .as_mut()
                    .context("rested-XP logout lost its realm connection")?;
                let (opcode, payload) = tokio::time::timeout(
                    logout_deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP logout realm packet read timed out"))??;
                ("realm", opcode, payload)
            }
            RestedXpLogoutReady::InstanceClosed | RestedXpLogoutReady::RealmClosed => {
                unreachable!()
            }
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 {} rested-XP logout {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            if connection != "instance" {
                bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during rested-XP logout");
            }
            let sequence_index = parse_time_sync_request_sequence(&payload)?;
            let client_time = client_clock_origin.elapsed().as_millis() as u32;
            let response = build_time_sync_response_payload(sequence_index, client_time);
            send_encrypted_packet(
                instance_stream,
                instance_crypt,
                CMSG_TIME_SYNC_RESPONSE,
                &response,
            )
            .await?;
            continue;
        }
        if opcode == SMSG_LOGOUT_COMPLETE {
            if connection != "realm" {
                warn!(
                    "[Bot {}] SMSG_LOGOUT_COMPLETE arrived on instance; stock C++ routes it on realm",
                    bot_index
                );
            }
            logout_complete = true;
            break;
        }
    }
    if !logout_complete {
        warn!(
            "[Bot {}] rested-XP graceful logout did not emit SMSG_LOGOUT_COMPLETE within {}s; closing both sockets and relying on the bounded DB-state proof",
            bot_index, logout_wait_secs
        );
    }

    instance_stream
        .shutdown()
        .await
        .context("shut down rested-XP instance socket")?;
    if let Some(mut realm) = realm_connection.take() {
        realm
            .stream
            .shutdown()
            .await
            .context("shut down rested-XP realm socket")?;
    }

    let bot_for_wait = bot.clone();
    tokio::task::spawn_blocking(move || {
        wait_for_rested_xp_character_offline_and_stable(&bot_for_wait, timeout_secs)
    })
    .await
    .map_err(|error| anyhow!("Rested-XP disconnect-save worker failed: {error}"))??;
    info!(
        "[Bot {}] ✅ rested-XP sockets disconnected and character save reached a stable offline row",
        bot_index
    );
    Ok(())
}

fn build_full_guid(low: u64, high: u64) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&low.to_le_bytes());
    payload.extend_from_slice(&high.to_le_bytes());
    payload
}

fn item_guid_raw(db_guid: u64) -> (u64, u64) {
    let high = (3u64 << 58) | ((u64::from(realm_id()) & 0x1FFF) << 42);
    (db_guid & OBJECT_GUID_COUNTER_MASK, high)
}

fn vault_keeper_full_guid(target: &ResolvedCreatureTarget, runtime_realm_id: u16) -> Vec<u8> {
    let (low, high) = create_void_storage_creature_guid_raw(
        target.map_id,
        target.entry,
        target.guid_counter,
        runtime_realm_id,
    );
    build_full_guid(low, high)
}

fn build_void_storage_transfer_payload(
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
    deposits: &[(u64, u64)],
    withdrawals: &[(u64, u64)],
) -> Vec<u8> {
    let mut payload = vault_keeper_full_guid(target, runtime_realm_id);
    payload.extend_from_slice(&(deposits.len() as u32).to_le_bytes());
    payload.extend_from_slice(&(withdrawals.len() as u32).to_le_bytes());
    for &(low, high) in deposits.iter().chain(withdrawals) {
        payload.extend_from_slice(&low.to_le_bytes());
        payload.extend_from_slice(&high.to_le_bytes());
    }
    payload
}

fn build_void_storage_swap_payload(
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
    void_item_id: u64,
    dst_slot: u32,
) -> Vec<u8> {
    let mut payload = vault_keeper_full_guid(target, runtime_realm_id);
    let (low, high) = item_guid_raw(void_item_id);
    payload.extend_from_slice(&low.to_le_bytes());
    payload.extend_from_slice(&high.to_le_bytes());
    payload.extend_from_slice(&dst_slot.to_le_bytes());
    payload
}

fn parse_void_item_wire(payload: &[u8], cursor: &mut usize) -> Result<VoidStorageItemWire> {
    let fixed_end = cursor
        .checked_add(50)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| anyhow!("truncated void-storage item"))?;
    let item_id = u64::from_le_bytes(payload[*cursor..*cursor + 8].try_into()?);
    *cursor += 16; // full void-item ObjectGuid
    *cursor += 16; // full creator ObjectGuid
    let slot = u32::from_le_bytes(payload[*cursor..*cursor + 4].try_into()?);
    *cursor += 4;
    let item_entry = i32::from_le_bytes(payload[*cursor..*cursor + 4].try_into()?);
    if item_entry <= 0 {
        bail!("void-storage item carried invalid entry {item_entry}");
    }
    *cursor += 12; // item id + random seed + random property id
    let has_bonus = payload[*cursor] & 0x80 != 0;
    *cursor += 1;
    let modifier_count = usize::from(payload[*cursor] >> 2);
    *cursor += 1;
    let modifier_bytes = modifier_count
        .checked_mul(5)
        .ok_or_else(|| anyhow!("void-storage modifier length overflow"))?;
    *cursor = cursor
        .checked_add(modifier_bytes)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| anyhow!("truncated void-storage item modifiers"))?;
    if has_bonus {
        if *cursor + 5 > payload.len() {
            bail!("truncated void-storage item bonuses");
        }
        *cursor += 1;
        let bonus_count = u32::from_le_bytes(payload[*cursor..*cursor + 4].try_into()?) as usize;
        *cursor += 4;
        let bonus_bytes = bonus_count
            .checked_mul(4)
            .ok_or_else(|| anyhow!("void-storage bonus length overflow"))?;
        *cursor = cursor
            .checked_add(bonus_bytes)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("truncated void-storage bonus list"))?;
    }
    debug_assert!(*cursor >= fixed_end);
    Ok(VoidStorageItemWire {
        item_id,
        slot,
        item_entry: item_entry as u32,
    })
}

fn parse_void_storage_contents(payload: &[u8]) -> Result<Vec<VoidStorageItemWire>> {
    let count = usize::from(
        *payload
            .first()
            .ok_or_else(|| anyhow!("empty SMSG_VOID_STORAGE_CONTENTS"))?,
    );
    let mut cursor = 1;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(parse_void_item_wire(payload, &mut cursor)?);
    }
    if cursor != payload.len() {
        bail!(
            "SMSG_VOID_STORAGE_CONTENTS left {} trailing bytes",
            payload.len() - cursor
        );
    }
    Ok(items)
}

async fn read_void_storage_packet(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    deadline: tokio::time::Instant,
    result: &mut BotRunResult,
) -> Result<(u16, Vec<u8>)> {
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for void-storage response");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for void-storage response"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 void-storage {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        return Ok((opcode, payload));
    }
}

async fn query_void_storage_contents(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<Vec<VoidStorageItemWire>> {
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_QUERY_VOID_STORAGE,
        &vault_keeper_full_guid(target, runtime_realm_id),
    )
    .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let (opcode, payload) =
            read_void_storage_packet(bot_index, stream, crypt, server_inflater, deadline, result)
                .await?;
        match opcode {
            SMSG_VOID_STORAGE_CONTENTS => return parse_void_storage_contents(&payload),
            SMSG_VOID_STORAGE_FAILED => bail!("void-storage query returned failure"),
            _ => {}
        }
    }
}

async fn wait_for_void_storage_transfer(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    expect_added: bool,
    expected_item_id: Option<u64>,
    result: &mut BotRunResult,
) -> Result<Option<VoidStorageItemWire>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut changed_item = None;
    let mut changes_seen = false;
    let mut success_seen = false;
    while !changes_seen || !success_seen {
        let (opcode, payload) =
            read_void_storage_packet(bot_index, stream, crypt, server_inflater, deadline, result)
                .await?;
        match opcode {
            SMSG_VOID_STORAGE_TRANSFER_CHANGES => {
                let counts = *payload
                    .first()
                    .ok_or_else(|| anyhow!("empty void-storage transfer changes"))?;
                let added_count = usize::from(counts >> 4);
                let removed_count = usize::from(counts & 0x0F);
                let expected_counts = if expect_added { (1, 0) } else { (0, 1) };
                if (added_count, removed_count) != expected_counts {
                    bail!(
                        "unexpected void-storage change counts {added_count}/{removed_count}, expected {}/{}",
                        expected_counts.0,
                        expected_counts.1
                    );
                }
                let mut cursor = 1;
                if expect_added {
                    changed_item = Some(parse_void_item_wire(&payload, &mut cursor)?);
                } else {
                    if cursor + 16 != payload.len() {
                        bail!("withdrawal change packet has invalid GUID length");
                    }
                    let removed_id = u64::from_le_bytes(payload[cursor..cursor + 8].try_into()?);
                    if Some(removed_id) != expected_item_id {
                        bail!(
                            "withdrawal removed void item {removed_id}, expected {:?}",
                            expected_item_id
                        );
                    }
                    cursor += 16;
                }
                if cursor != payload.len() {
                    bail!("void-storage transfer changes left trailing bytes");
                }
                changes_seen = true;
            }
            SMSG_VOID_TRANSFER_RESULT => {
                if payload.len() != 4 || i32::from_le_bytes(payload[..4].try_into()?) != 0 {
                    bail!("void-storage transfer returned nonzero or malformed result");
                }
                success_seen = true;
            }
            _ => {}
        }
    }
    Ok(changed_item)
}

async fn wait_for_void_storage_swap(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    expected_item_id: u64,
    expected_slot: u32,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let (opcode, payload) =
            read_void_storage_packet(bot_index, stream, crypt, server_inflater, deadline, result)
                .await?;
        if opcode == SMSG_VOID_STORAGE_FAILED || opcode == SMSG_VOID_TRANSFER_RESULT {
            bail!("void-storage swap returned failure opcode 0x{opcode:04X}");
        }
        if opcode != SMSG_VOID_ITEM_SWAP_RESPONSE {
            continue;
        }
        if payload.len() != 40 {
            bail!(
                "void-storage swap response has {} bytes, expected 40",
                payload.len()
            );
        }
        let item_id = u64::from_le_bytes(payload[..8].try_into()?);
        let slot = u32::from_le_bytes(payload[16..20].try_into()?);
        let destination_low = u64::from_le_bytes(payload[20..28].try_into()?);
        let destination_slot = u32::from_le_bytes(payload[36..40].try_into()?);
        if item_id != expected_item_id
            || slot != expected_slot
            || destination_low != 0
            || destination_slot != 0
        {
            bail!(
                "unexpected void-storage swap response item/slot/destination {item_id}/{slot}/{destination_low}/{destination_slot}"
            );
        }
        return Ok(());
    }
}

async fn wait_for_void_storage_db_state<F>(
    bot: &config::BotConfig,
    item_entry: u32,
    timeout_secs: u64,
    description: &str,
    predicate: F,
) -> Result<VoidStorageDbState>
where
    F: Fn(&VoidStorageDbState) -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let bot_for_db = bot.clone();
        let state = tokio::task::spawn_blocking(move || {
            load_void_storage_db_state(&bot_for_db, item_entry)
        })
        .await
        .map_err(|error| anyhow!("Void-storage DB worker failed: {error}"))??;
        if predicate(&state) {
            return Ok(state);
        }
        if tokio::time::Instant::now() >= deadline {
            bail!("timed out waiting for {description}; last state: {state:?}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn run_void_storage_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    options: &VoidStorageSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    match options.phase {
        VoidStorageSmokePhase::UnlockDeposit => {
            let before = wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "seeded void-storage fixture",
                |state| {
                    state.player_flags & PLAYER_FLAGS_VOID_UNLOCKED == 0
                        && state.void_items.is_empty()
                        && state.inventory_items
                            == vec![(options.fixture_item_guid, options.inventory_slot, 0)]
                },
            )
            .await?;
            send_encrypted_packet(
                stream,
                crypt,
                CMSG_UNLOCK_VOID_STORAGE,
                &vault_keeper_full_guid(&options.vault_keeper, options.runtime_realm_id),
            )
            .await?;
            let after_unlock = wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage unlock",
                |state| {
                    state.player_flags & PLAYER_FLAGS_VOID_UNLOCKED != 0
                        && state.money == before.money.saturating_sub(VOID_STORAGE_UNLOCK_COST)
                },
            )
            .await?;
            result.void_storage_unlock_persisted = true;

            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if !contents.is_empty() {
                bail!("freshly unlocked void storage was not empty: {contents:?}");
            }

            let deposit_guid = item_guid_raw(options.fixture_item_guid);
            let payload = build_void_storage_transfer_payload(
                &options.vault_keeper,
                options.runtime_realm_id,
                &[deposit_guid],
                &[],
            );
            send_encrypted_packet(stream, crypt, CMSG_VOID_STORAGE_TRANSFER, &payload).await?;
            let added = wait_for_void_storage_transfer(
                bot_index,
                stream,
                crypt,
                server_inflater,
                options.timeout_secs,
                true,
                None,
                result,
            )
            .await?
            .context("deposit transfer omitted added void item")?;
            if added.item_entry != options.item_entry || added.slot != 0 || added.item_id == 0 {
                bail!("unexpected deposited void item: {added:?}");
            }
            let expected_money = after_unlock
                .money
                .saturating_sub(VOID_STORAGE_STORE_ITEM_COST);
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage deposit",
                |state| {
                    state.money == expected_money
                        && state.void_items == vec![(added.item_id, options.item_entry, 0)]
                        && state.inventory_items.is_empty()
                },
            )
            .await?;
            result.void_storage_item_id = Some(added.item_id);
            result.void_storage_deposit_persisted = true;
        }
        VoidStorageSmokePhase::VerifyDepositSwap => {
            let expected_id = options
                .expected_void_item_id
                .context("deposit-relog phase omitted void item ID")?;
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if contents
                != vec![VoidStorageItemWire {
                    item_id: expected_id,
                    slot: u32::from(options.expected_void_slot),
                    item_entry: options.item_entry,
                }]
            {
                bail!("deposit relog query mismatch: {contents:?}");
            }
            result.void_storage_deposit_relogin_verified = true;
            let payload = build_void_storage_swap_payload(
                &options.vault_keeper,
                options.runtime_realm_id,
                expected_id,
                5,
            );
            send_encrypted_packet(stream, crypt, CMSG_SWAP_VOID_ITEM, &payload).await?;
            wait_for_void_storage_swap(
                bot_index,
                stream,
                crypt,
                server_inflater,
                options.timeout_secs,
                expected_id,
                5,
                result,
            )
            .await?;
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage slot swap",
                |state| state.void_items == vec![(expected_id, options.item_entry, 5)],
            )
            .await?;
            result.void_storage_swap_persisted = true;
        }
        VoidStorageSmokePhase::VerifySwapWithdraw => {
            let expected_id = options
                .expected_void_item_id
                .context("swap-relog phase omitted void item ID")?;
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if contents
                != vec![VoidStorageItemWire {
                    item_id: expected_id,
                    slot: u32::from(options.expected_void_slot),
                    item_entry: options.item_entry,
                }]
            {
                bail!("swap relog query mismatch: {contents:?}");
            }
            result.void_storage_swap_relogin_verified = true;
            let withdrawal_guid = item_guid_raw(expected_id);
            let payload = build_void_storage_transfer_payload(
                &options.vault_keeper,
                options.runtime_realm_id,
                &[],
                &[withdrawal_guid],
            );
            send_encrypted_packet(stream, crypt, CMSG_VOID_STORAGE_TRANSFER, &payload).await?;
            wait_for_void_storage_transfer(
                bot_index,
                stream,
                crypt,
                server_inflater,
                options.timeout_secs,
                false,
                Some(expected_id),
                result,
            )
            .await?;
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "atomic void-storage withdrawal",
                |state| {
                    state.void_items.is_empty()
                        && state.inventory_items.len() == 1
                        && state.inventory_items[0].2 & 1 != 0
                },
            )
            .await?;
            result.void_storage_withdraw_persisted = true;
        }
        VoidStorageSmokePhase::VerifyWithdraw => {
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            if !contents.is_empty() {
                bail!("withdraw relog query was not empty: {contents:?}");
            }
            wait_for_void_storage_db_state(
                bot,
                options.item_entry,
                options.timeout_secs,
                "withdrawn item after relog",
                |state| {
                    state.void_items.is_empty()
                        && state.inventory_items.len() == 1
                        && state.inventory_items[0].2 & 1 != 0
                },
            )
            .await?;
            result.void_storage_withdraw_relogin_verified = true;
        }
        VoidStorageSmokePhase::QueryCapture => {
            let expected_id = options
                .expected_void_item_id
                .context("void-storage query capture omitted seeded item ID")?;
            let contents = query_void_storage_contents(
                bot_index,
                stream,
                crypt,
                server_inflater,
                &options.vault_keeper,
                options.runtime_realm_id,
                options.timeout_secs,
                result,
            )
            .await?;
            let expected = vec![VoidStorageItemWire {
                item_id: expected_id,
                slot: u32::from(options.expected_void_slot),
                item_entry: options.item_entry,
            }];
            if contents != expected {
                bail!("void-storage query capture mismatch: {contents:?}, expected {expected:?}");
            }
            result.void_storage_item_id = Some(expected_id);
            result.void_storage_query_capture_passed = Some(true);
        }
    }
    logout_and_wait(bot_index, stream, crypt, server_inflater, result).await?;
    if options.phase != VoidStorageSmokePhase::QueryCapture {
        result.void_storage_smoke_passed = Some(true);
    }
    Ok(())
}

async fn run_bank_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    options: &BankSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let expected_before = match options.phase {
        BankSmokePhase::Deposit => options.inventory_slot,
        BankSmokePhase::Withdraw => options.bank_slot,
    };
    let bot_for_before = bot.clone();
    let item_guid = options.item_guid;
    let before = tokio::task::spawn_blocking(move || {
        verify_bank_fixture_location(&bot_for_before, item_guid, expected_before)
    })
    .await
    .map_err(|e| anyhow!("Bank pre-phase DB worker join failed: {e}"))??;
    if !before {
        bail!(
            "fixture item {} was not in expected slot {} before {:?}",
            options.item_guid,
            expected_before,
            options.phase
        );
    }
    if options.phase == BankSmokePhase::Withdraw {
        result.bank_relogin_after_deposit = true;
    }

    send_encrypted_packet(
        stream,
        crypt,
        CMSG_BANKER_ACTIVATE,
        &options.banker.packed_guid,
    )
    .await?;
    info!(
        "[Bot {}] ✅ CMSG_BANKER_ACTIVATE sent to entry={} spawn={}",
        bot_index, options.banker.entry, options.banker.spawn_guid
    );

    wait_for_bank_open(
        bot_index,
        stream,
        crypt,
        server_inflater,
        options.timeout_secs,
        result,
    )
    .await?;

    let (opcode, source_slot, expected_after) = match options.phase {
        BankSmokePhase::Deposit => (
            CMSG_AUTOBANK_ITEM,
            options.inventory_slot,
            options.bank_slot,
        ),
        BankSmokePhase::Withdraw => (
            CMSG_AUTOSTORE_BANK_ITEM,
            options.bank_slot,
            options.inventory_slot,
        ),
    };
    let payload = build_auto_bank_item_payload(source_slot);
    send_encrypted_packet(stream, crypt, opcode, &payload).await?;
    info!(
        "[Bot {}] ✅ {} sent from bag={} slot={}",
        bot_index,
        if options.phase == BankSmokePhase::Deposit {
            "CMSG_AUTOBANK_ITEM"
        } else {
            "CMSG_AUTOSTORE_BANK_ITEM"
        },
        INVENTORY_SLOT_BAG_0,
        source_slot
    );

    wait_for_bank_item_location(
        bot_index,
        bot,
        options.item_guid,
        expected_after,
        options.timeout_secs,
    )
    .await?;
    logout_and_wait(bot_index, stream, crypt, server_inflater, result).await?;

    let bot_for_after = bot.clone();
    let item_guid = options.item_guid;
    let persisted = tokio::task::spawn_blocking(move || {
        verify_bank_fixture_location(&bot_for_after, item_guid, expected_after)
    })
    .await
    .map_err(|e| anyhow!("Bank post-logout DB worker join failed: {e}"))??;
    if !persisted {
        bail!(
            "fixture item {} did not persist in slot {} after logout",
            options.item_guid,
            expected_after
        );
    }

    match options.phase {
        BankSmokePhase::Deposit => result.bank_deposit_persisted = true,
        BankSmokePhase::Withdraw => result.bank_withdraw_persisted = true,
    }
    result.bank_smoke_passed = Some(true);
    Ok(())
}

async fn run_inventory_swap_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    item_entry_a: u32,
    item_entry_b: u32,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_inventory_swap_smoke_fixture(
            &bot_for_setup,
            item_entry_a,
            item_entry_b,
            timeout_secs,
        )
    })
    .await
    .map_err(|e| anyhow!("Inventory swap smoke setup DB worker join failed: {e}"))??;

    let mut forward_options = fixture.options.clone();
    forward_options.phase = InventorySwapSmokePhase::Forward;
    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        Some(forward_options),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            let bot_for_cleanup = bot.clone();
            let fixture_for_cleanup = fixture.clone();
            let _ = tokio::task::spawn_blocking(move || {
                cleanup_inventory_swap_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await;
            return Err(error.context("Inventory swap forward login/phase failed"));
        }
    };

    if combined.inventory_swap_smoke_passed.unwrap_or(false) {
        let mut reverse_options = fixture.options.clone();
        reverse_options.phase = InventorySwapSmokePhase::Reverse;
        match run_bot(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            Some(reverse_options),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(second) => {
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.inventory_swap_relogin_after_forward =
                    second.inventory_swap_relogin_after_forward;
                combined.inventory_swap_reverse_persisted = second.inventory_swap_reverse_persisted;
                combined.seen_opcodes.extend(second.seen_opcodes);
                combined.inventory_swap_failure = second.inventory_swap_failure;
                combined.inventory_swap_smoke_passed = Some(
                    combined.inventory_swap_forward_persisted
                        && combined.inventory_swap_relogin_after_forward
                        && combined.inventory_swap_reverse_persisted
                        && second.inventory_swap_smoke_passed.unwrap_or(false),
                );
            }
            Err(error) => {
                combined.inventory_swap_failure = Some(format!(
                    "Inventory swap reverse relog/phase failed: {error}"
                ));
                combined.inventory_swap_smoke_passed = Some(false);
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_inventory_swap_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|e| anyhow!("Inventory swap cleanup DB worker join failed: {e}"))?;
    if let Err(error) = cleanup {
        combined.inventory_swap_failure = Some(format!("Inventory swap cleanup failed: {error}"));
        combined.inventory_swap_smoke_passed = Some(false);
    }

    Ok(combined)
}

async fn run_vendor_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    vendor_entry: u32,
    vendor_spawn_guid: u64,
    runtime_counter: Option<u64>,
    item_entry: u32,
    extended_cost: u32,
    currency_id: u32,
    currency_cost: u32,
    currency_quantity: u32,
    timeout_secs: u64,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_vendor_smoke_fixture(
            &bot_for_setup,
            vendor_entry,
            vendor_spawn_guid,
            runtime_counter,
            item_entry,
            extended_cost,
            currency_id,
            currency_cost,
            currency_quantity,
            timeout_secs,
        )
    })
    .await
    .map_err(|error| anyhow!("Vendor smoke setup DB worker join failed: {error}"))??;

    let first = run_bot(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        false,
        None,
        None,
        None,
        None,
        Some(fixture.options.clone()),
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            let bot_for_cleanup = bot.clone();
            let fixture_for_cleanup = fixture.clone();
            let cleanup = tokio::task::spawn_blocking(move || {
                cleanup_vendor_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
            })
            .await
            .map_err(|join_error| {
                anyhow!(
                    "Vendor smoke purchase login/phase failed: {error}; cleanup worker failed: {join_error}"
                )
            })?;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "Vendor smoke purchase login/phase failed: {error}; fixture cleanup failed: {cleanup_error}"
                );
            }
            return Err(error.context("Vendor smoke purchase login/phase failed"));
        }
    };

    if combined.vendor_smoke_passed.unwrap_or(false) {
        let mut relog_options = fixture.options.clone();
        relog_options.phase = VendorSmokePhase::VerifyRelog;
        match run_bot(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            Some(relog_options),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        {
            Ok(second) => {
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.vendor_relogin_verified = second.vendor_relogin_verified;
                combined.vendor_currency_after = second.vendor_currency_after;
                combined.vendor_item_total_after = second.vendor_item_total_after;
                combined.seen_opcodes.extend(second.seen_opcodes);
                combined.vendor_failure = second.vendor_failure;
                combined.vendor_smoke_passed = Some(
                    combined.vendor_inventory_seen
                        && combined.vendor_buy_succeeded_seen
                        && combined.vendor_set_currency_seen
                        && combined.vendor_item_push_seen
                        && combined.vendor_relogin_verified
                        && second.vendor_smoke_passed.unwrap_or(false),
                );
            }
            Err(error) => {
                combined.vendor_failure =
                    Some(format!("Vendor persistence relog/phase failed: {error}"));
                combined.vendor_smoke_passed = Some(false);
            }
        }
    }

    let bot_for_cleanup = bot.clone();
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || {
        cleanup_vendor_smoke_fixture(&bot_for_cleanup, &fixture_for_cleanup)
    })
    .await
    .map_err(|error| anyhow!("Vendor smoke cleanup DB worker join failed: {error}"))?;
    if let Err(error) = cleanup {
        let cleanup_failure = format!("Vendor fixture cleanup failed: {error}");
        combined.vendor_failure = Some(match combined.vendor_failure.take() {
            Some(previous) => format!("{previous}; {cleanup_failure}"),
            None => cleanup_failure,
        });
        combined.vendor_smoke_passed = Some(false);
    }

    Ok(combined)
}

async fn run_vendor_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &VendorSmokeOptions,
    login_discovered_target: Option<DiscoveredCreatureGuid>,
    result: &mut BotRunResult,
) -> Result<()> {
    let expected_currency_after = options
        .currency_before
        .checked_sub(options.currency_cost)
        .ok_or_else(|| anyhow!("Vendor currency fixture underflow"))?;

    if options.phase == VendorSmokePhase::VerifyRelog {
        let bot_for_db = bot.clone();
        let currency_id = options.currency_id;
        let item_entry = options.item_entry;
        let (currency, item_total) = tokio::task::spawn_blocking(move || {
            load_vendor_smoke_db_state(&bot_for_db, currency_id, item_entry)
        })
        .await
        .map_err(|error| anyhow!("Vendor relog DB worker join failed: {error}"))??;
        result.vendor_currency_after = Some(currency);
        result.vendor_item_total_after = Some(item_total);
        if currency != expected_currency_after || item_total != options.expected_item_total {
            bail!(
                "vendor state after relog is currency/item {currency}/{item_total}, expected {expected_currency_after}/{}",
                options.expected_item_total
            );
        }
        result.vendor_relogin_verified = true;
        loot_race::logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection.as_mut(),
            bot.character_guid,
            result,
        )
        .await?;
        result.vendor_smoke_passed = Some(true);
        return Ok(());
    }

    let bot_for_before = bot.clone();
    let currency_id = options.currency_id;
    let item_entry = options.item_entry;
    let (currency_before, item_before) = tokio::task::spawn_blocking(move || {
        load_vendor_smoke_db_state(&bot_for_before, currency_id, item_entry)
    })
    .await
    .map_err(|error| anyhow!("Vendor pre-purchase DB worker join failed: {error}"))??;
    if currency_before != options.currency_before || item_before != 0 {
        bail!(
            "vendor fixture drifted before purchase: currency/item {currency_before}/{item_before}, expected {}/0",
            options.currency_before
        );
    }

    // C++ Player::CanNeverSee keeps nearby world objects hidden until the
    // client acknowledges that its active mover is initialized. Rust may have
    // queued the vendor CREATE earlier, so send the canonical ACK before the
    // cross-server discovery window in both cases.
    let active_mover_complete = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &active_mover_complete,
    )
    .await?;
    info!(
        "[Bot {}] ✅ CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE sent before vendor discovery",
        bot_index
    );

    let expected_runtime_counter = (options.vendor.guid_counter != 0)
        .then_some(options.vendor.guid_counter & OBJECT_GUID_COUNTER_MASK);
    let mut discovered = login_discovered_target;
    let discovery_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while discovered.is_none() && tokio::time::Instant::now() < discovery_deadline {
        let remaining = discovery_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "vendor instance login discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
        } else if opcode == SMSG_UPDATE_OBJECT {
            discovered = find_creature_guid_near_position_in_update_object(
                &payload,
                options.vendor.map_id,
                options.vendor.entry,
                options.vendor.x as f32,
                options.vendor.y as f32,
                options.vendor.z as f32,
                options.target_match_radius,
                expected_runtime_counter,
            );
        }
    }
    let runtime_target = resolve_vendor_runtime_target(&options.vendor, discovered)?;
    let runtime_counter = runtime_target.low & OBJECT_GUID_COUNTER_MASK;
    let runtime_vendor_guid = build_packed_guid(runtime_target.low, runtime_target.high);
    result.vendor_runtime_counter = Some(runtime_counter);

    send_encrypted_packet(stream, crypt, CMSG_LIST_INVENTORY, &runtime_vendor_guid).await?;
    info!(
        "[Bot {}] ✅ CMSG_LIST_INVENTORY sent to entry={} spawn={} counter={}",
        bot_index, options.vendor.entry, options.vendor.spawn_guid, runtime_counter
    );
    let vendor_item = wait_for_vendor_inventory_item(
        bot_index,
        stream,
        crypt,
        server_inflater,
        &runtime_vendor_guid,
        options,
        result,
    )
    .await?;

    let buy_payload = build_vendor_buy_item_payload(
        &runtime_vendor_guid,
        bot.character_guid,
        vendor_item.muid,
        options.item_entry,
    );
    send_encrypted_packet(stream, crypt, CMSG_BUY_ITEM, &buy_payload).await?;
    info!(
        "[Bot {}] ✅ CMSG_BUY_ITEM sent item={} muid={} cost={}/currency={}",
        bot_index, options.item_entry, vendor_item.muid, options.currency_cost, options.currency_id
    );
    wait_for_vendor_purchase_result(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        &runtime_vendor_guid,
        vendor_item.muid,
        options,
        expected_currency_after,
        result,
    )
    .await?;

    loot_race::logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection.as_mut(),
        bot.character_guid,
        result,
    )
    .await?;
    let bot_for_after = bot.clone();
    let currency_id = options.currency_id;
    let item_entry = options.item_entry;
    let (currency_after, item_after) = tokio::task::spawn_blocking(move || {
        load_vendor_smoke_db_state(&bot_for_after, currency_id, item_entry)
    })
    .await
    .map_err(|error| anyhow!("Vendor post-purchase DB worker join failed: {error}"))??;
    result.vendor_currency_after = Some(currency_after);
    result.vendor_item_total_after = Some(item_after);
    if currency_after != expected_currency_after || item_after != options.expected_item_total {
        bail!(
            "vendor purchase persisted currency/item {currency_after}/{item_after}, expected {expected_currency_after}/{}",
            options.expected_item_total
        );
    }
    result.vendor_smoke_passed = Some(true);
    Ok(())
}

async fn run_inventory_swap_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    options: &InventorySwapSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let (expected_before_a, expected_before_b, expected_after_a, expected_after_b) =
        match options.phase {
            InventorySwapSmokePhase::Forward => (
                options.slot_a,
                options.slot_b,
                options.slot_b,
                options.slot_a,
            ),
            InventorySwapSmokePhase::Reverse => {
                result.inventory_swap_relogin_after_forward = true;
                (
                    options.slot_b,
                    options.slot_a,
                    options.slot_a,
                    options.slot_b,
                )
            }
        };

    let bot_for_before = bot.clone();
    let options_for_before = options.clone();
    let before = tokio::task::spawn_blocking(move || {
        verify_inventory_swap_fixture_locations(
            &bot_for_before,
            &options_for_before,
            expected_before_a,
            expected_before_b,
        )
    })
    .await
    .map_err(|e| anyhow!("Inventory swap pre-phase DB worker join failed: {e}"))??;
    if !before {
        bail!(
            "inventory swap fixture was not in expected slots {expected_before_a}/{expected_before_b} before {:?}",
            options.phase
        );
    }

    let payload = build_swap_inv_item_payload(options.slot_a, options.slot_b);
    send_encrypted_packet(stream, crypt, CMSG_SWAP_INV_ITEM, &payload).await?;
    info!(
        "[Bot {}] ✅ CMSG_SWAP_INV_ITEM sent slot {} -> {} ({:?})",
        bot_index, options.slot_a, options.slot_b, options.phase
    );

    wait_for_inventory_swap_locations(
        bot,
        options,
        expected_after_a,
        expected_after_b,
        options.timeout_secs,
    )
    .await?;
    logout_and_wait(bot_index, stream, crypt, server_inflater, result).await?;

    let bot_for_after = bot.clone();
    let options_for_after = options.clone();
    let persisted = tokio::task::spawn_blocking(move || {
        verify_inventory_swap_fixture_locations(
            &bot_for_after,
            &options_for_after,
            expected_after_a,
            expected_after_b,
        )
    })
    .await
    .map_err(|e| anyhow!("Inventory swap post-logout DB worker join failed: {e}"))??;
    if !persisted {
        bail!(
            "inventory swap fixture did not persist in expected slots {expected_after_a}/{expected_after_b} after logout"
        );
    }

    match options.phase {
        InventorySwapSmokePhase::Forward => result.inventory_swap_forward_persisted = true,
        InventorySwapSmokePhase::Reverse => result.inventory_swap_reverse_persisted = true,
    }
    result.inventory_swap_smoke_passed = Some(true);
    Ok(())
}

async fn wait_for_inventory_swap_locations(
    bot: &config::BotConfig,
    options: &InventorySwapSmokeOptions,
    expected_slot_a: u8,
    expected_slot_b: u8,
    timeout_secs: u64,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let bot_for_check = bot.clone();
        let options_for_check = options.clone();
        let matches = tokio::task::spawn_blocking(move || {
            verify_inventory_swap_fixture_locations(
                &bot_for_check,
                &options_for_check,
                expected_slot_a,
                expected_slot_b,
            )
        })
        .await
        .map_err(|e| anyhow!("Inventory swap polling DB worker join failed: {e}"))??;
        if matches {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("timed out waiting for inventory swap DB slots {expected_slot_a}/{expected_slot_b}")
}

async fn wait_for_bank_open(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        {
            Ok(Ok((opcode, payload))) => {
                result.seen_opcodes.push(format!("0x{opcode:04X}"));
                if opcode == SMSG_INVENTORY_CHANGE_FAILURE {
                    bail!("bank activation returned inventory failure payload {payload:?}");
                }
                if opcode == SMSG_NPC_INTERACTION_OPEN_RESULT {
                    result.bank_open_confirmed = true;
                    info!("[Bot {}] ✅ banker interaction open confirmed", bot_index);
                    return Ok(());
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => break,
        }
    }
    bail!("timed out waiting for SMSG_NPC_INTERACTION_OPEN_RESULT")
}

async fn wait_for_bank_item_location(
    bot_index: usize,
    bot: &config::BotConfig,
    item_guid: u64,
    expected_slot: u8,
    timeout_secs: u64,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let bot_for_db = bot.clone();
        let located = tokio::task::spawn_blocking(move || {
            verify_bank_fixture_location(&bot_for_db, item_guid, expected_slot)
        })
        .await
        .map_err(|e| anyhow!("Bank location DB worker join failed: {e}"))??;
        if located {
            info!(
                "[Bot {}] ✅ fixture item {} reached persisted slot {}",
                bot_index, item_guid, expected_slot
            );
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!(
        "timed out waiting for fixture item {} in slot {}",
        item_guid,
        expected_slot
    )
}

async fn logout_and_wait(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    result: &mut BotRunResult,
) -> Result<()> {
    send_encrypted_packet(stream, crypt, CMSG_LOGOUT_REQUEST, &[0]).await?;
    info!("[Bot {}] ✅ CMSG_LOGOUT_REQUEST sent", bot_index);
    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(NORMAL_LOGOUT_COMPLETE_WAIT_SECS);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        {
            Ok(Ok((opcode, _))) => {
                result.seen_opcodes.push(format!("0x{opcode:04X}"));
                if opcode == SMSG_LOGOUT_COMPLETE {
                    info!("[Bot {}] ✅ SMSG_LOGOUT_COMPLETE received", bot_index);
                    return Ok(());
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => break,
        }
    }
    bail!("timed out waiting for SMSG_LOGOUT_COMPLETE")
}

async fn wait_for_vendor_inventory_item(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    expected_vendor_guid: &[u8],
    options: &VendorSmokeOptions,
    result: &mut BotRunResult,
) -> Result<VendorInventoryItemWire> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for SMSG_VENDOR_INVENTORY");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for SMSG_VENDOR_INVENTORY"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 vendor-list {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        if opcode != SMSG_VENDOR_INVENTORY {
            continue;
        }

        let items = parse_vendor_inventory(&payload, expected_vendor_guid)?;
        let item = items
            .into_iter()
            .find(|item| {
                item.item_id == options.item_entry as i32
                    && item.extended_cost == options.extended_cost as i32
            })
            .ok_or_else(|| {
                anyhow!(
                    "vendor inventory omitted expected item/cost {}/{}",
                    options.item_entry,
                    options.extended_cost
                )
            })?;
        if item.item_type != 1 || item.muid <= 0 || item.price != 0 || item.stack_count != 1 {
            bail!(
                "vendor item {} wire row is not the deterministic fixture shape: {item:?}",
                options.item_entry
            );
        }
        result.vendor_inventory_seen = true;
        return Ok(item);
    }
}

async fn wait_for_vendor_purchase_result(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    expected_vendor_guid: &[u8],
    expected_muid: i32,
    options: &VendorSmokeOptions,
    expected_currency_after: u32,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection
        .as_mut()
        .context("vendor purchase requires the preserved realm connection")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    let mut fence_sent = false;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for vendor purchase result (buy={}, currency={}, item_push={}, fence={})",
                result.vendor_buy_succeeded_seen,
                result.vendor_set_currency_seen,
                result.vendor_item_push_seen,
                fence_sent
            );
        }
        // C++ splits this result across both encrypted connections. Poll for
        // readiness, then finish one selected frame without cancellation so
        // a losing `select!` branch cannot consume a partial encrypted frame.
        let routed_packet = if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(5)),
            remaining,
            "vendor realm purchase result",
        )
        .await?
        {
            Some(("realm", true, opcode, payload))
        } else {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            read_encrypted_packet_if_ready(
                stream,
                crypt,
                server_inflater,
                remaining.min(Duration::from_millis(5)),
                remaining,
                "vendor instance purchase result",
            )
            .await?
            .map(|(opcode, payload)| ("instance", false, opcode, payload))
        };
        let Some((connection, on_realm, opcode, payload)) = routed_packet else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 {} vendor-buy {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );

        match opcode {
            SMSG_TIME_SYNC_REQUEST => {
                if on_realm {
                    bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during vendor purchase");
                }
                let sequence = parse_time_sync_request_sequence(&payload)?;
                let response = build_time_sync_response_payload(sequence, 0);
                send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            }
            SMSG_BUY_FAILED => {
                let reason = payload.last().copied();
                bail!("vendor purchase returned SMSG_BUY_FAILED reason={reason:?}");
            }
            SMSG_INVENTORY_CHANGE_FAILURE => {
                bail!("vendor purchase returned SMSG_INVENTORY_CHANGE_FAILURE");
            }
            SMSG_BUY_SUCCEEDED => {
                if !on_realm {
                    bail!("SMSG_BUY_SUCCEEDED arrived on instance; C++ routes it on realm");
                }
                parse_vendor_buy_succeeded(&payload, expected_vendor_guid, expected_muid, 1, -1)?;
                result.vendor_buy_succeeded_seen = true;
            }
            SMSG_SET_CURRENCY => {
                if on_realm {
                    bail!("SMSG_SET_CURRENCY arrived on realm; C++ routes it on instance");
                }
                let (currency_id, quantity) = parse_set_currency_identity(&payload)?;
                if currency_id == options.currency_id {
                    if quantity != expected_currency_after {
                        bail!(
                            "SMSG_SET_CURRENCY quantity for {} is {}, expected {}",
                            currency_id,
                            quantity,
                            expected_currency_after
                        );
                    }
                    result.vendor_set_currency_seen = true;
                }
            }
            SMSG_ITEM_PUSH_RESULT => {
                if !on_realm {
                    bail!("SMSG_ITEM_PUSH_RESULT arrived on instance; C++ routes it on realm");
                }
                loot_race::validate_vendor_item_push_result_like_cpp(
                    &payload,
                    result.character_guid,
                    options.item_entry,
                    1,
                    realm_id(),
                )?;
                result.vendor_item_push_seen = true;
            }
            SMSG_PONG if fence_sent => {
                if on_realm {
                    bail!("vendor capture-fence SMSG_PONG arrived on realm");
                }
                if payload != VENDOR_CAPTURE_FENCE_SERIAL.to_le_bytes() {
                    bail!(
                        "vendor capture-fence SMSG_PONG mismatch: expected 0x{:08X}, got {:02X?}",
                        VENDOR_CAPTURE_FENCE_SERIAL,
                        payload
                    );
                }
                return Ok(());
            }
            _ => {}
        }

        if !fence_sent
            && result.vendor_buy_succeeded_seen
            && result.vendor_set_currency_seen
            && result.vendor_item_push_seen
        {
            let ping = build_ping_payload(VENDOR_CAPTURE_FENCE_SERIAL);
            send_encrypted_packet(stream, crypt, CMSG_PING, &ping).await?;
            fence_sent = true;
            info!(
                "[Bot {}] ✅ vendor capture-fence CMSG_PING serial=0x{:08X}",
                bot_index, VENDOR_CAPTURE_FENCE_SERIAL
            );
        }
    }
}

fn parse_set_currency_identity(payload: &[u8]) -> Result<(u32, u32)> {
    if payload.len() < 8 {
        bail!("SMSG_SET_CURRENCY payload is shorter than type/quantity");
    }
    let currency_id = i32::from_le_bytes(payload[0..4].try_into()?);
    let quantity = i32::from_le_bytes(payload[4..8].try_into()?);
    Ok((
        u32::try_from(currency_id).map_err(|_| anyhow!("negative currency id {currency_id}"))?,
        u32::try_from(quantity).map_err(|_| anyhow!("negative currency quantity {quantity}"))?,
    ))
}

fn parse_vendor_buy_succeeded(
    payload: &[u8],
    expected_vendor_guid: &[u8],
    expected_muid: i32,
    expected_quantity_bought: u32,
    expected_new_quantity: i32,
) -> Result<()> {
    let (guid_len, low, high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("SMSG_BUY_SUCCEEDED has an invalid packed vendor GUID"))?;
    let (expected_guid_len, expected_low, expected_high) = parse_packed_guid(expected_vendor_guid)
        .ok_or_else(|| anyhow!("fixture has an invalid packed vendor GUID"))?;
    if guid_len != expected_guid_len || low != expected_low || high != expected_high {
        bail!("SMSG_BUY_SUCCEEDED names a different vendor GUID");
    }

    let mut cursor = guid_len;
    let muid = take_vendor_u32(payload, &mut cursor)?;
    let new_quantity = take_vendor_i32(payload, &mut cursor)?;
    let quantity_bought = take_vendor_u32(payload, &mut cursor)?;
    let expected_muid = u32::try_from(expected_muid)
        .map_err(|_| anyhow!("fixture has invalid negative vendor MUID {expected_muid}"))?;
    if cursor != payload.len() {
        bail!(
            "SMSG_BUY_SUCCEEDED has {} trailing bytes",
            payload.len() - cursor
        );
    }
    if muid != expected_muid
        || new_quantity != expected_new_quantity
        || quantity_bought != expected_quantity_bought
    {
        bail!(
            "SMSG_BUY_SUCCEEDED fields are muid/new_quantity/quantity_bought {muid}/{new_quantity}/{quantity_bought}, expected {expected_muid}/{expected_new_quantity}/{expected_quantity_bought}"
        );
    }
    Ok(())
}

fn parse_vendor_inventory(
    payload: &[u8],
    expected_vendor_guid: &[u8],
) -> Result<Vec<VendorInventoryItemWire>> {
    let (guid_len, low, high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("SMSG_VENDOR_INVENTORY has an invalid packed vendor GUID"))?;
    let (expected_len, expected_low, expected_high) = parse_packed_guid(expected_vendor_guid)
        .ok_or_else(|| anyhow!("fixture has an invalid packed vendor GUID"))?;
    if guid_len != expected_len || low != expected_low || high != expected_high {
        bail!("SMSG_VENDOR_INVENTORY names a different vendor GUID");
    }
    let mut cursor = guid_len;
    let reason = *payload
        .get(cursor)
        .ok_or_else(|| anyhow!("SMSG_VENDOR_INVENTORY omitted reason"))?;
    cursor += 1;
    if reason != 0 {
        bail!("SMSG_VENDOR_INVENTORY returned reason {reason}");
    }
    let count = take_vendor_u32(payload, &mut cursor)?;
    let count = usize::try_from(count).map_err(|_| anyhow!("vendor item count overflow"))?;
    if count > 300 {
        bail!("SMSG_VENDOR_INVENTORY item count {count} exceeds C++ vendor bound");
    }
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let price = take_vendor_u64(payload, &mut cursor)?;
        let muid = take_vendor_i32(payload, &mut cursor)?;
        let item_type = take_vendor_i32(payload, &mut cursor)?;
        let _durability = take_vendor_i32(payload, &mut cursor)?;
        let stack_count = take_vendor_i32(payload, &mut cursor)?;
        let _quantity = take_vendor_i32(payload, &mut cursor)?;
        let extended_cost = take_vendor_i32(payload, &mut cursor)?;
        let _player_condition_failed = take_vendor_i32(payload, &mut cursor)?;
        cursor = cursor
            .checked_add(1)
            .filter(|next| *next <= payload.len())
            .ok_or_else(|| anyhow!("SMSG_VENDOR_INVENTORY omitted vendor flags"))?;
        let item_id = take_vendor_i32(payload, &mut cursor)?;
        let _random_seed = take_vendor_i32(payload, &mut cursor)?;
        let _random_property = take_vendor_i32(payload, &mut cursor)?;
        let has_bonus_bits = *payload
            .get(cursor)
            .ok_or_else(|| anyhow!("vendor ItemInstance omitted bonus bit"))?;
        cursor += 1;
        let mod_count_bits = *payload
            .get(cursor)
            .ok_or_else(|| anyhow!("vendor ItemInstance omitted modifier count"))?;
        cursor += 1;
        if has_bonus_bits != 0 || mod_count_bits != 0 {
            bail!(
                "vendor ItemInstance for item {item_id} has unsupported bonus/modifier bits 0x{has_bonus_bits:02X}/0x{mod_count_bits:02X}"
            );
        }
        items.push(VendorInventoryItemWire {
            muid,
            item_id,
            item_type,
            price,
            stack_count,
            extended_cost,
        });
    }
    if cursor != payload.len() {
        bail!(
            "SMSG_VENDOR_INVENTORY has {} trailing bytes after {} items",
            payload.len() - cursor,
            count
        );
    }
    Ok(items)
}

fn take_vendor_u32(payload: &[u8], cursor: &mut usize) -> Result<u32> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| anyhow!("vendor packet cursor overflow"))?;
    let bytes: [u8; 4] = payload
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("vendor packet truncated at byte {}", *cursor))?
        .try_into()?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes))
}

fn take_vendor_i32(payload: &[u8], cursor: &mut usize) -> Result<i32> {
    Ok(i32::from_le_bytes(
        take_vendor_u32(payload, cursor)?.to_le_bytes(),
    ))
}

fn take_vendor_u64(payload: &[u8], cursor: &mut usize) -> Result<u64> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| anyhow!("vendor packet cursor overflow"))?;
    let bytes: [u8; 8] = payload
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("vendor packet truncated at byte {}", *cursor))?
        .try_into()?;
    *cursor = end;
    Ok(u64::from_le_bytes(bytes))
}

fn build_vendor_buy_item_payload(
    vendor_guid: &[u8],
    character_guid: u64,
    muid: i32,
    item_entry: u32,
) -> Vec<u8> {
    let (player_low, player_high) = create_player_guid_raw(character_guid, realm_id());
    let mut payload = Vec::with_capacity(vendor_guid.len() + 48);
    payload.extend_from_slice(vendor_guid);
    payload.extend(build_packed_guid(player_low, player_high));
    payload.extend_from_slice(&1i32.to_le_bytes());
    payload.extend_from_slice(&muid.to_le_bytes());
    payload.extend_from_slice(&i32::from(u8::MAX).to_le_bytes());
    payload.extend_from_slice(&1i32.to_le_bytes());
    payload.extend_from_slice(&(item_entry as i32).to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.extend_from_slice(&0i32.to_le_bytes());
    payload.push(0);
    payload.push(0);
    payload
}

fn build_auto_bank_item_payload(slot: u8) -> [u8; 5] {
    // C++ InvUpdate count=1 is two MSB-first bits `01`, followed by the
    // affected position and then the packet's source bag/slot.
    [0x40, INVENTORY_SLOT_BAG_0, slot, INVENTORY_SLOT_BAG_0, slot]
}

fn build_swap_inv_item_payload(src_slot: u8, dst_slot: u8) -> [u8; 7] {
    // C++ InvUpdate count=2 is two MSB-first bits `10`. The real 3.4.3
    // client lists destination then source, followed by Slot2/Slot1.
    [
        0x80,
        INVENTORY_SLOT_BAG_0,
        dst_slot,
        INVENTORY_SLOT_BAG_0,
        src_slot,
        dst_slot,
        src_slot,
    ]
}

async fn run_quest_smoke_inner(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    quest_options: &QuestSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let options_for_db = quest_options.clone();
    let bot_for_db = bot.clone();
    let target = tokio::task::spawn_blocking(move || {
        resolve_quest_target_for_bot(&bot_for_db, &options_for_db)
    })
    .await
    .map_err(|e| anyhow!("Quest target DB worker join failed: {}", e))??;

    result.quest_target_entry = Some(target.entry);
    result.quest_target_spawn_guid = Some(target.spawn_guid);
    result.quest_target_guid_counter = Some(target.guid_counter);
    result.quest_target_map_id = Some(target.map_id);

    info!(
        "[Bot {}] Quest smoke target: entry={} spawn_guid={} guid_counter={} map={}",
        bot_index, target.entry, target.spawn_guid, target.guid_counter, target.map_id
    );

    send_encrypted_packet(stream, crypt, 0x349C, &target.packed_guid).await?;
    send_encrypted_packet(stream, crypt, 0x3492, &target.packed_guid).await?;
    result.quest_gossip_hello_sent = true;
    info!("[Bot {}] ✅ CMSG_GOSSIP_HELLO sent", bot_index);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(quest_options.timeout_secs);
    let mut questgiver_hello_sent = false;
    let mut details_query_sent_for: Option<u32> = None;
    let mut accept_sent_for: Option<u32> = None;

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let slice = remaining.min(Duration::from_millis(500));
        match tokio::time::timeout(slice, read_encrypted_packet(stream, crypt, server_inflater))
            .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                let parsed = parse_packet(op, &payload);
                info!("[Bot {}] 📦 {}", bot_index, parsed);
                record_quest_objective_login_signal(op, &payload, quest_options, result);
                handle_quest_smoke_packet(op, &payload, result);

                if let Some(gossip_option_id) = quest_options.gossip_select_option_id {
                    if !result.quest_gossip_select_sent {
                        if let Some(gossip_id) = result.quest_gossip_id_seen {
                            let select = build_gossip_select_option(
                                &target.packed_guid,
                                gossip_id,
                                gossip_option_id,
                            );
                            send_encrypted_packet(stream, crypt, 0x3494, &select).await?;
                            result.quest_gossip_select_sent = true;
                            info!(
                                "[Bot {}] ✅ CMSG_GOSSIP_SELECT_OPTION sent: gossip_id={} option_id={}",
                                bot_index, gossip_id, gossip_option_id
                            );
                        }
                    }
                }

                if quest_options.query_details
                    && details_query_sent_for.is_none()
                    && !quest_details_or_request_items_seen(result)
                {
                    if let Some(quest_id) = select_quest_to_query(quest_options, result) {
                        let query = build_quest_giver_query_quest(&target.packed_guid, quest_id);
                        send_encrypted_packet(stream, crypt, 0x3497, &query).await?;
                        details_query_sent_for = Some(quest_id);
                        info!(
                            "[Bot {}] ✅ CMSG_QUEST_GIVER_QUERY_QUEST sent for quest {}",
                            bot_index, quest_id
                        );
                    }
                }

                if quest_options.accept && result.quest_details_seen && accept_sent_for.is_none() {
                    if let Some(quest_id) = select_quest_to_query(quest_options, result) {
                        let accept = build_quest_giver_accept_quest(&target.packed_guid, quest_id);
                        send_encrypted_packet(stream, crypt, 0x3498, &accept).await?;
                        result.quest_accept_sent = true;
                        accept_sent_for = Some(quest_id);
                        info!(
                            "[Bot {}] ✅ CMSG_QUEST_GIVER_ACCEPT_QUEST sent for quest {}",
                            bot_index, quest_id
                        );
                    }
                }

                if quest_smoke_has_enough_signal(quest_options, result) {
                    break;
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Quest smoke read error: {}", bot_index, e);
                break;
            }
            Err(_) => {
                if !questgiver_hello_sent {
                    send_encrypted_packet(stream, crypt, 0x3496, &target.packed_guid).await?;
                    result.quest_questgiver_hello_sent = true;
                    questgiver_hello_sent = true;
                    info!("[Bot {}] ✅ CMSG_QUEST_GIVER_HELLO sent", bot_index);
                }
            }
        }
    }

    if !quest_options.objective_persist
        && !result.quest_gossip_message_seen
        && !result.quest_quest_list_seen
        && !result.quest_details_seen
        && !result.quest_request_items_seen
        && !result.trainer_list_seen
    {
        bail!("No GossipMessage, QuestList, QuestDetails, RequestItems, or TrainerList response received");
    }

    if quest_options.accept {
        let quest_id = quest_options
            .expected_quest_id
            .or(accept_sent_for)
            .ok_or_else(|| {
                anyhow!("Quest accept requested but no selected quest id was available")
            })?;
        let bot_for_db = bot.clone();
        let (verified, status) =
            tokio::task::spawn_blocking(move || verify_quest_accepted_in_db(&bot_for_db, quest_id))
                .await
                .map_err(|e| anyhow!("Quest DB verification worker join failed: {}", e))??;
        result.quest_db_verified = verified;
        result.quest_db_status = status;
    }

    Ok(())
}

fn handle_quest_smoke_packet(op: u16, payload: &[u8], result: &mut BotRunResult) {
    match op {
        0x2A98 => {
            result.quest_gossip_message_seen = true;
            result.quest_gossip_id_seen = packet_parser::parse_gossip_id(payload);
            if let Some(offers) = packet_parser::parse_gossip_quest_offers(payload) {
                record_quest_offers(offers, result);
            }
        }
        0x2A9A => {
            result.quest_quest_list_seen = true;
            if let Some(offers) = packet_parser::parse_quest_list_offers(payload) {
                record_quest_offers(offers, result);
            }
        }
        0x2A92 => {
            result.quest_details_seen = true;
            if let Some(details) = packet_parser::parse_quest_details_summary(payload) {
                record_quest_id(details.quest_id, result);
            }
        }
        0x2A93 => {
            result.quest_request_items_seen = true;
            if let Some(summary) = packet_parser::parse_quest_request_items_summary(payload) {
                record_quest_id(summary.quest_id, result);
            }
        }
        0x2A83 => {
            result.quest_accept_confirm_seen = true;
            if payload.len() >= 4 {
                record_quest_id(
                    u32::from_le_bytes(payload[0..4].try_into().unwrap()),
                    result,
                );
            }
        }
        0x26DF => {
            result.trainer_list_seen = true;
            if let Some(summary) = packet_parser::parse_trainer_list_summary(payload) {
                result.trainer_id_seen = Some(summary.trainer_id);
                result.trainer_spell_count_seen = Some(summary.spell_count);
            }
        }
        _ => {}
    }
}

fn record_quest_offers(offers: Vec<packet_parser::QuestOffer>, result: &mut BotRunResult) {
    for offer in offers {
        record_quest_id(offer.quest_id, result);
        if !offer.title.is_empty() && !result.quest_titles_seen.contains(&offer.title) {
            result.quest_titles_seen.push(offer.title);
        }
    }
}

fn record_quest_id(quest_id: u32, result: &mut BotRunResult) {
    if quest_id != 0 && !result.quest_ids_seen.contains(&quest_id) {
        result.quest_ids_seen.push(quest_id);
    }
}

fn select_quest_to_query(quest_options: &QuestSmokeOptions, result: &BotRunResult) -> Option<u32> {
    if let Some(expected) = quest_options.expected_quest_id {
        if result.quest_ids_seen.contains(&expected) {
            return Some(expected);
        }
    }
    result.quest_ids_seen.first().copied()
}

fn quest_smoke_has_enough_signal(quest_options: &QuestSmokeOptions, result: &BotRunResult) -> bool {
    if quest_options.expect_trainer_list {
        return result.trainer_list_seen;
    }
    if quest_options.accept {
        return result.quest_accept_sent
            && (result.quest_accept_confirm_seen || result.quest_db_verified);
    }
    if quest_options.query_details {
        quest_details_or_request_items_seen(result)
    } else {
        result.quest_gossip_message_seen
            || result.quest_quest_list_seen
            || result.quest_details_seen
            || result.quest_request_items_seen
    }
}

fn quest_details_or_request_items_seen(result: &BotRunResult) -> bool {
    result.quest_details_seen || result.quest_request_items_seen
}

fn quest_smoke_passes(quest_options: &QuestSmokeOptions, result: &mut BotRunResult) -> bool {
    result.quest_failure = None;

    if !quest_options.objective_persist
        && !result.quest_gossip_message_seen
        && !result.quest_quest_list_seen
        && !result.quest_details_seen
        && !result.quest_request_items_seen
        && !result.trainer_list_seen
    {
        result.quest_failure = Some("No questgiver or trainer response was received".to_string());
        return false;
    }

    if quest_options.expect_trainer_list && !result.trainer_list_seen {
        result.quest_failure = Some("TrainerList was not received".to_string());
        return false;
    }

    if let Some(expected) = quest_options.expect_trainer_id {
        if result.trainer_id_seen != Some(expected) {
            result.quest_failure = Some(format!(
                "Expected trainer id {}, got {:?}",
                expected, result.trainer_id_seen
            ));
            return false;
        }
    }

    if quest_options.query_details
        && !quest_options.objective_persist
        && !quest_details_or_request_items_seen(result)
    {
        result.quest_failure = Some("QuestDetails or RequestItems was not received".to_string());
        return false;
    }

    if quest_options.accept {
        if !result.quest_accept_sent {
            result.quest_failure = Some("Quest accept packet was not sent".to_string());
            return false;
        }
        if !result.quest_db_verified {
            result.quest_failure = Some(format!(
                "Accepted quest was not verified in DB (status={:?})",
                result.quest_db_status
            ));
            return false;
        }
    }

    if quest_options.objective_persist && !result.quest_objective_db_verified {
        result.quest_failure = Some(format!(
            "Quest objective DB rows did not survive logout (before={:?}, after={:?})",
            result.quest_objective_db_before, result.quest_objective_db_after
        ));
        return false;
    }

    if !quest_options.objective_persist {
        if let Some(expected) = quest_options.expected_quest_id {
            if !result.quest_ids_seen.contains(&expected) {
                result.quest_failure = Some(format!("Expected quest {} was not seen", expected));
                return false;
            }
        }
    }

    if let Some(forbidden) = quest_options.forbidden_quest_id {
        if result.quest_ids_seen.contains(&forbidden) {
            result.quest_failure = Some(format!("Forbidden quest {} was offered", forbidden));
            return false;
        }
    }

    if let Some(forbidden_title) = &quest_options.forbidden_title_contains {
        let needle = forbidden_title.to_ascii_lowercase();
        if result
            .quest_titles_seen
            .iter()
            .any(|title| title.to_ascii_lowercase().contains(&needle))
        {
            result.quest_failure = Some(format!(
                "Forbidden quest title fragment `{}` was offered",
                forbidden_title
            ));
            return false;
        }
    }

    true
}

fn resolve_quest_target_for_bot(
    bot: &config::BotConfig,
    quest_options: &QuestSmokeOptions,
) -> Result<ResolvedCreatureTarget> {
    use mysql::prelude::Queryable;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|e| anyhow!("Bad world DB URL: {}", e))?;
    let mut world =
        mysql::Conn::new(world_opts).map_err(|e| anyhow!("Connect to world DB failed: {}", e))?;

    let (spawn_guid, entry, map_id, x, y, z, orientation) = if let Some(spawn_guid) =
        quest_options.creature_spawn_guid
    {
        let row: Option<(u64, u32, u32, f64, f64, f64, f32)> = world
            .exec_first(
                "SELECT guid, id, map, position_x, position_y, position_z, orientation \
                 FROM creature WHERE guid = ?",
                (spawn_guid,),
            )
            .map_err(|e| anyhow!("Lookup creature spawn {}: {}", spawn_guid, e))?;
        let (guid, entry, map_id, x, y, z, orientation) =
            row.ok_or_else(|| anyhow!("No world.creature row for guid {}", spawn_guid))?;
        if entry != quest_options.creature_entry {
            bail!(
                "Creature guid {} has entry {}, expected {}",
                guid,
                entry,
                quest_options.creature_entry
            );
        }
        (
            guid,
            entry,
            quest_options.map_id.unwrap_or(map_id as u16),
            x,
            y,
            z,
            orientation,
        )
    } else {
        let player_position = load_bot_position(bot.character_guid).ok();
        let map_id = quest_options
            .map_id
            .or_else(|| player_position.as_ref().map(|p| p.0))
            .ok_or_else(|| {
                anyhow!("Set WOW_BOT_QUEST_MAP_ID or use a bot character with a saved map position")
            })?;

        let row: Option<(u64, u32, u32, f64, f64, f64, f32)> = if let Some((player_map, x, y, z)) =
            player_position
        {
            let query_map = if quest_options.map_id.is_some() {
                map_id
            } else {
                player_map
            };
            world
                .exec_first(
                    "SELECT guid, id, map, position_x, position_y, position_z, orientation FROM creature \
                     WHERE id = ? AND map = ? \
                     ORDER BY POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2) \
                     LIMIT 1",
                    (quest_options.creature_entry, query_map, x, y, z),
                )
                .map_err(|e| anyhow!("Lookup nearest creature target: {}", e))?
        } else {
            world
                .exec_first(
                    "SELECT guid, id, map, position_x, position_y, position_z, orientation FROM creature \
                     WHERE id = ? AND map = ? \
                     ORDER BY guid LIMIT 1",
                    (quest_options.creature_entry, map_id),
                )
                .map_err(|e| anyhow!("Lookup creature target by entry/map: {}", e))?
        };

        row.ok_or_else(|| {
            anyhow!(
                "No world.creature row for entry {} on map {}",
                quest_options.creature_entry,
                map_id
            )
        })
        .map(|(guid, entry, row_map, x, y, z, orientation)| {
            (guid, entry, row_map as u16, x, y, z, orientation)
        })?
    };

    let guid_counter = resolve_quest_runtime_counter(
        quest_options.creature_guid_counter,
        spawn_guid,
        quest_options.creature_entry,
    )?;
    let (low, high) = create_creature_guid_raw(map_id, entry, guid_counter);
    Ok(ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id,
        x,
        y,
        z,
        orientation,
        packed_guid: build_packed_guid(low, high),
    })
}

fn load_bot_position(character_guid: u64) -> Result<(u16, f64, f64, f64)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;
    let row: Option<(u32, f64, f64, f64)> = conn
        .exec_first(
            "SELECT map, position_x, position_y, position_z FROM characters WHERE guid = ?",
            (character_guid,),
        )
        .map_err(|e| anyhow!("Lookup character {} position: {}", character_guid, e))?;
    row.map(|(map, x, y, z)| (map as u16, x, y, z))
        .ok_or_else(|| anyhow!("No characters row for guid {}", character_guid))
}

fn prepare_quest_smoke_before_login(
    bot: &config::BotConfig,
    quest_options: &QuestSmokeOptions,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let target = resolve_quest_target_for_bot(bot, quest_options)?;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;

    if quest_options.reset_before_run {
        let quest_id = quest_options
            .expected_quest_id
            .ok_or_else(|| anyhow!("Quest reset requested without expected quest id"))?;
        reset_bot_quest_state(&mut conn, bot.character_guid, quest_id)?;
        info!(
            "Quest smoke reset character {} quest {}",
            bot.character_guid, quest_id
        );
    }

    if quest_options.relocate_before_login {
        let offset_x = 2.0_f64;
        conn.exec_drop(
            "UPDATE characters \
             SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                u32::from(target.map_id),
                target.x + offset_x,
                target.y,
                target.z,
                target.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| {
            anyhow!(
                "Relocate character {} near quest target: {}",
                bot.character_guid,
                e
            )
        })?;
        info!(
            "Quest smoke relocated character {} near creature {} ({}, {}, {})",
            bot.character_guid,
            target.spawn_guid,
            target.x + offset_x,
            target.y,
            target.z
        );
    }

    if let Some(level) = quest_options.set_level_before_login {
        set_bot_character_level(&mut conn, bot.character_guid, level)?;
        info!(
            "Quest smoke set character {} level to {}",
            bot.character_guid, level
        );
    }

    if quest_options.set_race_before_login.is_some()
        || quest_options.set_class_before_login.is_some()
    {
        set_bot_character_race_class(
            &mut conn,
            bot.character_guid,
            quest_options.set_race_before_login,
            quest_options.set_class_before_login,
        )?;
        info!(
            "Quest smoke set character {} race={:?} class={:?}",
            bot.character_guid,
            quest_options.set_race_before_login,
            quest_options.set_class_before_login
        );
    }

    if quest_options.objective_persist {
        let quest_id = quest_options
            .expected_quest_id
            .ok_or_else(|| anyhow!("Quest objective persistence requested without quest id"))?;
        seed_bot_quest_objective_state(
            &mut conn,
            bot.character_guid,
            quest_id,
            quest_options.objective_status,
            &quest_options.objective_seed,
        )?;
        info!(
            "Quest smoke seeded character {} quest {} objectives {:?}",
            bot.character_guid, quest_id, quest_options.objective_seed
        );
    }

    Ok(())
}

const RESTED_XP_RESTORE_CHARACTER_SQL: &str =
    "UPDATE characters SET level = ?, xp = ?, restState = ?, playerFlags = ?, rest_bonus = ?, \
     logout_time = ?, is_logout_resting = ?, map = ?, zone = ?, instance_id = ?, \
     position_x = ?, position_y = ?, position_z = ?, orientation = ?, health = ?, \
     power1 = ?, power2 = ?, power3 = ?, power4 = ?, power5 = ?, power6 = ?, power7 = ?, \
     power8 = ?, power9 = ?, power10 = ?, totalKills = ?, todayKills = ?, yesterdayKills = ?, \
     totaltime = ?, leveltime = ?, latency = ?, lastLoginBuild = ? \
     WHERE guid = ? AND online = 0";

const RESTED_XP_SELECT_TRAIT_CONFIGS_SQL: &str =
    "SELECT traitConfigId, type, chrSpecializationId, combatConfigFlags, localIdentifier, \
     skillLineId, traitSystemId, name FROM character_trait_config \
     WHERE guid = ? ORDER BY traitConfigId";
const RESTED_XP_SELECT_TRAIT_ENTRIES_SQL: &str =
    "SELECT traitConfigId, traitNodeId, traitNodeEntryId, rank, grantedRanks \
     FROM character_trait_entry WHERE guid = ? \
     ORDER BY traitConfigId, traitNodeId, traitNodeEntryId";

// Stock C++ login/save materializes these defaults for an otherwise clean
// character. The rested-XP fixture requires them empty before the smoke, so
// cleanup must remove the same bounded, character-owned rows afterwards.
const RESTED_XP_CPP_GENERATED_CHARACTER_ROWS: &[(&str, &str, &str)] = &[
    (
        "character_glyphs",
        "DELETE FROM character_glyphs WHERE guid = ?",
        "SELECT COUNT(*) FROM character_glyphs WHERE guid = ?",
    ),
    (
        "character_reputation",
        "DELETE FROM character_reputation WHERE guid = ?",
        "SELECT COUNT(*) FROM character_reputation WHERE guid = ?",
    ),
    (
        "character_skills",
        "DELETE FROM character_skills WHERE guid = ?",
        "SELECT COUNT(*) FROM character_skills WHERE guid = ?",
    ),
];

fn prepare_rested_xp_smoke_fixture(
    bot: &config::BotConfig,
    creature_entry: u32,
    creature_spawn_guid: Option<u64>,
    runtime_counter: Option<u64>,
    offline_secs: u64,
    timeout_secs: u64,
) -> Result<RestedXpSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing rested-XP fixture setup for non-local account {}",
            bot.account
        );
    }
    if let Some(counter) = runtime_counter {
        if counter == 0 || counter > OBJECT_GUID_COUNTER_MASK {
            bail!(
                "rested-XP runtime counter override {counter} must fit the nonzero 40-bit ObjectGuid counter field"
            );
        }
    }
    let stat_save_min_level = worldserver_config_u32("PlayerSave.Stats.MinLevel", 0)?;
    if stat_save_min_level != 0 {
        bail!(
            "rested-XP fixture requires PlayerSave.Stats.MinLevel=0, got {stat_save_min_level}; stock C++ otherwise destructively rewrites character_stats on logout"
        );
    }
    let start_all_spells = worldserver_config_u32("PlayerStart.AllSpells", 0)?;
    if start_all_spells != 0 {
        bail!(
            "rested-XP fixture requires PlayerStart.AllSpells=0, got {start_all_spells}; otherwise stock C++ can populate character_spell during login"
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let character_row: mysql::Row = characters
        .exec_first(
            "SELECT account, online, at_login, level, xp, restState, playerFlags, rest_bonus, logout_time, \
             is_logout_resting, map, zone, instance_id, position_x, position_y, position_z, \
             orientation, health, power1, power2, power3, power4, power5, power6, power7, power8, \
             power9, power10, totalKills, todayKills, yesterdayKills, totaltime, leveltime, \
             latency, lastLoginBuild \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load rested-XP bot character: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let owner: u32 = required_row_value(&character_row, "account")?;
    let online: u8 = required_row_value(&character_row, "online")?;
    let at_login: u16 = required_row_value(&character_row, "at_login")?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; refusing rested-XP fixture setup",
            bot.character_guid
        );
    }
    let characters_on_game_account = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM characters WHERE account = ?",
        u64::from(bot.account_id),
        "characters on game account",
    )?;
    let mut safety_state = RestedXpFixtureSafetyState {
        at_login,
        characters_on_game_account,
        ..RestedXpFixtureSafetyState::default()
    };
    for (table, sql) in [
        (
            "character_inventory",
            "SELECT COUNT(*) FROM character_inventory WHERE guid = ?",
        ),
        (
            "character_pet",
            "SELECT COUNT(*) FROM character_pet WHERE owner = ?",
        ),
        (
            "character_aura",
            "SELECT COUNT(*) FROM character_aura WHERE guid = ?",
        ),
        (
            "character_aura_effect",
            "SELECT COUNT(*) FROM character_aura_effect WHERE guid = ?",
        ),
        (
            "character_spell_cooldown",
            "SELECT COUNT(*) FROM character_spell_cooldown WHERE guid = ?",
        ),
        (
            "character_spell_charges",
            "SELECT COUNT(*) FROM character_spell_charges WHERE guid = ?",
        ),
        (
            "character_skills",
            "SELECT COUNT(*) FROM character_skills WHERE guid = ?",
        ),
        (
            "character_glyphs",
            "SELECT COUNT(*) FROM character_glyphs WHERE guid = ?",
        ),
        (
            "character_talent",
            "SELECT COUNT(*) FROM character_talent WHERE guid = ?",
        ),
        (
            "character_spell",
            "SELECT COUNT(*) FROM character_spell WHERE guid = ?",
        ),
        (
            "character_spell_favorite",
            "SELECT COUNT(*) FROM character_spell_favorite WHERE guid = ?",
        ),
        (
            "character_action",
            "SELECT COUNT(*) FROM character_action WHERE guid = ?",
        ),
        (
            "character_reputation",
            "SELECT COUNT(*) FROM character_reputation WHERE guid = ?",
        ),
        (
            "character_equipmentsets",
            "SELECT COUNT(*) FROM character_equipmentsets WHERE guid = ?",
        ),
        (
            "character_transmog_outfits",
            "SELECT COUNT(*) FROM character_transmog_outfits WHERE guid = ?",
        ),
        (
            "character_cuf_profiles",
            "SELECT COUNT(*) FROM character_cuf_profiles WHERE guid = ?",
        ),
        (
            "character_void_storage",
            "SELECT COUNT(*) FROM character_void_storage WHERE playerGuid = ?",
        ),
        (
            "guild_member",
            "SELECT COUNT(*) FROM guild_member WHERE guid = ?",
        ),
        ("corpse", "SELECT COUNT(*) FROM corpse WHERE guid = ?"),
    ] {
        let rows = rested_xp_count_rows(&mut characters, sql, bot.character_guid, table)?;
        if rows != 0 {
            safety_state
                .nonempty_side_state
                .push((table.to_string(), rows));
        }
    }
    let instance_lock_rows = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM account_instance_times WHERE accountId = ?",
        u64::from(bot.account_id),
        "account_instance_times",
    )?;
    if instance_lock_rows != 0 {
        safety_state
            .nonempty_side_state
            .push(("account_instance_times".to_string(), instance_lock_rows));
    }
    let tutorial_rows = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM account_tutorial WHERE accountId = ?",
        u64::from(bot.account_id),
        "account_tutorial",
    )?;
    if tutorial_rows != 0 {
        safety_state
            .nonempty_side_state
            .push(("account_tutorial".to_string(), tutorial_rows));
    }
    let original = rested_xp_character_restore_point_from_row(&character_row)?;
    let original_achievements: Vec<(u32, i64)> = characters
        .exec(
            "SELECT achievement, date FROM character_achievement WHERE guid = ? ORDER BY achievement",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character achievements: {error}"))?;
    let original_achievement_progress: Vec<(u32, u64, i64)> = characters
        .exec(
            "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ? ORDER BY criteria",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character achievement progress: {error}"))?;
    // Stock C++ can materialize missing specialization trait configs during
    // login/save. Preserve these tables exactly instead of assuming the
    // disposable fixture started with no trait state.
    let original_trait_configs: Vec<RestedXpTraitConfigSnapshot> = characters
        .exec(RESTED_XP_SELECT_TRAIT_CONFIGS_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Snapshot rested-XP character trait configs: {error}"))?;
    let original_trait_entries: Vec<RestedXpTraitEntrySnapshot> = characters
        .exec(RESTED_XP_SELECT_TRAIT_ENTRIES_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Snapshot rested-XP character trait entries: {error}"))?;
    let original_homebind: Option<RestedXpHomebindSnapshot> = characters
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation \
             FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character homebind: {error}"))?;
    let original_fishing_steps: Option<u8> = characters
        .exec_first(
            "SELECT fishingSteps FROM character_fishingsteps WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character fishing steps: {error}"))?;
    let original_battleground_data: Option<RestedXpBattlegroundDataSnapshot> = characters
        .exec_first(
            "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, \
                    taxiStart, taxiEnd, mountSpell, queueId \
             FROM character_battleground_data WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP character battleground data: {error}"))?;

    let active_quests: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_queststatus WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check rested-XP active quests: {error}"))?
        .unwrap_or(0);
    let active_objectives: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_queststatus_objectives WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check rested-XP quest objectives: {error}"))?
        .unwrap_or(0);
    let active_criteria = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM character_queststatus_objectives_criteria WHERE guid = ?",
        bot.character_guid,
        "character_queststatus_objectives_criteria",
    )?;
    let active_criteria_progress = rested_xp_count_rows(
        &mut characters,
        "SELECT COUNT(*) FROM character_queststatus_objectives_criteria_progress WHERE guid = ?",
        bot.character_guid,
        "character_queststatus_objectives_criteria_progress",
    )?;
    if active_quests != 0
        || active_objectives != 0
        || active_criteria != 0
        || active_criteria_progress != 0
    {
        bail!(
            "character {} has active quest state ({active_quests} quests/{active_objectives} objectives/{active_criteria} criteria/{active_criteria_progress} criteria progress); use a clean @bot.local character so the kill cannot mutate quest progress",
            bot.character_guid
        );
    }
    let group_rows: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM group_member WHERE memberGuid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check rested-XP group membership: {error}"))?
        .unwrap_or(0);
    if group_rows != 0 {
        bail!(
            "character {} is in a persisted group; refusing ambiguous rested/RAF XP QA",
            bot.character_guid
        );
    }

    let auth_url = auth_db_url()?;
    let auth_opts =
        mysql::Opts::from_url(&auth_url).map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut auth = mysql::Conn::new(auth_opts)
        .map_err(|error| anyhow!("Connect to auth DB failed: {error}"))?;
    let (recruiter, battlenet_account_id, game_account_online, battlenet_email): (
        u32,
        Option<u32>,
        u8,
        Option<String>,
    ) = auth
        .exec_first(
            "SELECT a.recruiter, a.battlenet_account, a.online, ba.email \
             FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
             WHERE a.id = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Check rested-XP account scope: {error}"))?
        .ok_or_else(|| anyhow!("No auth.account row for id {}", bot.account_id))?;
    let battlenet_account_id = battlenet_account_id.ok_or_else(|| {
        anyhow!(
            "auth.account {} has no Battle.net identity; refusing disposable rested-XP fixture",
            bot.account_id
        )
    })?;
    let original_last_played_characters: Vec<RestedXpLastPlayedCharacterSnapshot> = auth
        .exec(
            "SELECT region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime \
             FROM account_last_played_character WHERE accountId = ? \
             ORDER BY region, battlegroup",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP last-played character rows: {error}"))?;
    let original_battle_pet_slots: Vec<RestedXpBattlePetSlotSnapshot> = auth
        .exec(
            "SELECT id, battlePetGuid, locked FROM battle_pet_slots \
             WHERE battlenetAccountId = ? ORDER BY id",
            (battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Snapshot rested-XP Battle.net pet slots: {error}"))?;
    safety_state.game_account_online = game_account_online;
    safety_state.bnet_email_matches_configured_account = battlenet_email
        .as_deref()
        .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account));
    safety_state.game_accounts_on_bnet_account = rested_xp_count_rows(
        &mut auth,
        "SELECT COUNT(*) FROM account WHERE battlenet_account = ?",
        u64::from(battlenet_account_id),
        "game accounts on Battle.net identity",
    )?;
    for (table, sql) in [
        (
            "battlenet_account_mounts",
            "SELECT COUNT(*) FROM battlenet_account_mounts WHERE battlenetAccountId = ?",
        ),
        (
            "battlenet_account_toys",
            "SELECT COUNT(*) FROM battlenet_account_toys WHERE accountId = ?",
        ),
        (
            "battlenet_account_heirlooms",
            "SELECT COUNT(*) FROM battlenet_account_heirlooms WHERE accountId = ?",
        ),
        (
            "battlenet_item_appearances",
            "SELECT COUNT(*) FROM battlenet_item_appearances WHERE battlenetAccountId = ?",
        ),
        (
            "battlenet_item_favorite_appearances",
            "SELECT COUNT(*) FROM battlenet_item_favorite_appearances WHERE battlenetAccountId = ?",
        ),
        (
            "battlenet_account_transmog_illusions",
            "SELECT COUNT(*) FROM battlenet_account_transmog_illusions WHERE battlenetAccountId = ?",
        ),
        (
            "battle_pets",
            "SELECT COUNT(*) FROM battle_pets WHERE battlenetAccountId = ?",
        ),
    ] {
        let rows = rested_xp_count_rows(
            &mut auth,
            sql,
            u64::from(battlenet_account_id),
            table,
        )?;
        if rows != 0 {
            safety_state
                .nonempty_side_state
                .push((table.to_string(), rows));
        }
    }
    let recruited_accounts: u64 = auth
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE recruiter = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Check rested-XP recruited accounts: {error}"))?
        .unwrap_or(0);
    if recruiter != 0 || recruited_accounts != 0 {
        bail!(
            "account {} participates in Recruit-A-Friend; refusing a rested-XP test that could award 300% XP",
            bot.account_id
        );
    }
    validate_rested_xp_fixture_safety_state(&safety_state)?;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let target_row: mysql::Row = if let Some(spawn_guid) = creature_spawn_guid {
        world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation, \
                 c.wander_distance, c.spawntimesecs AS SpawnTimeSecs, COALESCE(d.MinLevel, 1) AS MinLevel, \
                 COALESCE(d.MaxLevel, 1) AS MaxLevel, ct.type AS CreatureType, \
                 ct.VehicleId, ct.flags_extra, \
                 COALESCE(d.StaticFlags1, 0) AS StaticFlags1 \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 LEFT JOIN creature_template_difficulty d ON d.Entry = c.id AND d.DifficultyID = 0 \
                 WHERE c.guid = ? AND c.id = ?",
                (spawn_guid, creature_entry),
            )
            .map_err(|error| anyhow!("Resolve rested-XP target spawn: {error}"))?
    } else {
        world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation, \
                 c.wander_distance, c.spawntimesecs AS SpawnTimeSecs, COALESCE(d.MinLevel, 1) AS MinLevel, \
                 COALESCE(d.MaxLevel, 1) AS MaxLevel, ct.type AS CreatureType, \
                 ct.VehicleId, ct.flags_extra, \
                 COALESCE(d.StaticFlags1, 0) AS StaticFlags1 \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 LEFT JOIN creature_template_difficulty d ON d.Entry = c.id AND d.DifficultyID = 0 \
                 WHERE c.id = ? ORDER BY c.guid LIMIT 1",
                (creature_entry,),
            )
            .map_err(|error| anyhow!("Resolve rested-XP target entry: {error}"))?
    }
    .ok_or_else(|| {
        anyhow!(
            "No world.creature spawn for rested-XP entry {}{}",
            creature_entry,
            creature_spawn_guid
                .map(|guid| format!(" and guid {guid}"))
                .unwrap_or_default()
        )
    })?;
    let spawn_guid: u64 = required_row_value(&target_row, "guid")?;
    let entry: u32 = required_row_value(&target_row, "id")?;
    let map_id_u32: u32 = required_row_value(&target_row, "map")?;
    let map_id = u16::try_from(map_id_u32)
        .map_err(|_| anyhow!("rested-XP target map {map_id_u32} does not fit protocol u16"))?;
    let x: f64 = required_row_value(&target_row, "position_x")?;
    let y: f64 = required_row_value(&target_row, "position_y")?;
    let z: f64 = required_row_value(&target_row, "position_z")?;
    let orientation: f32 = required_row_value(&target_row, "orientation")?;
    let wander_distance: f32 = required_row_value(&target_row, "wander_distance")?;
    let target_match_radius = wander_distance.max(0.0) + 2.0;
    let target_respawn_secs: u32 = required_row_value(&target_row, "SpawnTimeSecs")?;
    let min_level: u8 = required_row_value(&target_row, "MinLevel")?;
    let max_level: u8 = required_row_value(&target_row, "MaxLevel")?;
    let creature_type: u8 = required_row_value(&target_row, "CreatureType")?;
    let vehicle_id: u32 = required_row_value(&target_row, "VehicleId")?;
    let flags_extra: u32 = required_row_value(&target_row, "flags_extra")?;
    let static_flags_1: u32 = required_row_value(&target_row, "StaticFlags1")?;
    if runtime_counter.is_none() {
        let overlapping_spawn: Option<u64> = world
            .exec_first(
                "SELECT guid FROM creature \
                 WHERE id = ? AND map = ? AND guid <> ? \
                   AND SQRT(POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) \
                       <= ? + GREATEST(wander_distance, 0) \
                 ORDER BY guid LIMIT 1",
                (entry, map_id_u32, spawn_guid, x, y, z, target_match_radius),
            )
            .map_err(|error| anyhow!("Check rested-XP target spawn ambiguity: {error}"))?;
        if let Some(overlapping_spawn) = overlapping_spawn {
            bail!(
                "rested-XP SQL spawn {spawn_guid} has an overlapping same-entry movement radius with spawn {overlapping_spawn}; set --rested-xp-runtime-counter from a trusted live discovery or choose an isolated spawn"
            );
        }
    }
    validate_rested_xp_target_template(entry, creature_type, vehicle_id)?;
    if !(MIN_RESTED_XP_TARGET_RESPAWN_SECS..=MAX_RESTED_XP_TARGET_RESPAWN_SECS)
        .contains(&target_respawn_secs)
    {
        bail!(
            "rested-XP target entry {entry} has an unsuitable {target_respawn_secs}s respawn; choose a disposable target in {MIN_RESTED_XP_TARGET_RESPAWN_SECS}..={MAX_RESTED_XP_TARGET_RESPAWN_SECS}s so the harness can observe the persisted timer before it clears"
        );
    }
    if min_level == 0 || max_level == 0 || min_level > max_level || max_level > 6 {
        bail!(
            "rested-XP target {} has nondeterministic/unsafe level range {}..={}; choose a level 1..=6 creature",
            entry,
            min_level,
            max_level
        );
    }
    if flags_extra & CREATURE_FLAG_EXTRA_NO_XP != 0
        || static_flags_1 & CREATURE_STATIC_FLAG_NO_XP != 0
    {
        bail!("rested-XP target entry {entry} is marked NO_XP");
    }
    let reputation_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM creature_onkill_reputation WHERE creature_id = ?",
            (entry,),
        )
        .map_err(|error| anyhow!("Check rested-XP target reputation side effects: {error}"))?
        .unwrap_or(0);
    if reputation_rows != 0 {
        bail!("rested-XP target entry {entry} has on-kill reputation; choose an isolated target");
    }

    let pending_respawn: Option<u64> = characters
        .exec_first(
            "SELECT respawnTime FROM respawn WHERE type = 0 AND spawnId = ? AND mapId = ? AND instanceId = 0",
            (spawn_guid, map_id_u32),
        )
        .map_err(|error| anyhow!("Check rested-XP target respawn timer: {error}"))?;
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    if let Some(respawn_time) = pending_respawn {
        bail!(
            "rested-XP target spawn {spawn_guid} already has persisted respawn row {respawn_time} (now={now}); refusing to overwrite world state"
        );
    }

    let test_level = max_level;
    let next_level_xp: u32 = world
        .exec_first(
            "SELECT Experience FROM player_xp_for_level WHERE Level = ?",
            (test_level,),
        )
        .map_err(|error| anyhow!("Load rested-XP next-level threshold: {error}"))?
        .ok_or_else(|| anyhow!("No player_xp_for_level row for level {test_level}"))?;
    if next_level_xp == 0 {
        bail!("level {test_level} has zero next-level XP; cannot test rested XP");
    }
    let wilderness_rate = worldserver_config_f32("Rate.Rest.Offline.InWilderness", 1.0)?;
    let resting_rate = worldserver_config_f32("Rate.Rest.Offline.InTavernOrCity", 1.0)?;
    let expected_wilderness = offline_rest_bonus_like_cpp(
        next_level_xp,
        offline_secs,
        REST_OFFLINE_WILDERNESS_BUBBLE,
        wilderness_rate,
    );
    let expected_resting = offline_rest_bonus_like_cpp(
        next_level_xp,
        offline_secs,
        REST_OFFLINE_TAVERN_OR_CITY_BUBBLE,
        resting_rate,
    );
    if expected_wilderness <= 0.0 || expected_resting <= expected_wilderness {
        bail!(
            "configured rest rates/interval cannot prove resting>wilderness: wilderness={expected_wilderness:.4}, resting={expected_resting:.4}"
        );
    }

    let guid_counter = runtime_counter.unwrap_or(0);
    let packed_guid = if guid_counter == 0 {
        Vec::new()
    } else {
        let (low, high) = create_creature_guid_raw(map_id, entry, guid_counter);
        build_packed_guid(low, high)
    };
    let target = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id,
        x,
        y,
        z,
        orientation,
        packed_guid,
    };
    let seeded_rest_bonus = next_level_xp as f32 * REST_BONUS_CAP_NEXT_LEVEL_FACTOR;
    info!(
        "Rested-XP fixture ready: character={} target={}/{} map={} level={} nextXP={} rates={}/{}",
        bot.character_guid,
        entry,
        spawn_guid,
        map_id,
        test_level,
        next_level_xp,
        wilderness_rate,
        resting_rate
    );
    Ok(RestedXpSmokeFixture {
        options: RestedXpSmokeOptions {
            phase: RestedXpSmokePhase::OfflineWilderness,
            target,
            target_match_radius,
            test_level,
            next_level_xp,
            seeded_rest_bonus,
            expected_xp: None,
            expected_rest_bonus: None,
            timeout_secs,
        },
        original,
        original_achievements,
        original_achievement_progress,
        original_trait_configs,
        original_trait_entries,
        original_homebind,
        original_fishing_steps,
        original_battleground_data,
        original_last_played_characters,
        original_battle_pet_slots,
        battlenet_account_id,
        target_respawn_secs,
        test_level,
        offline_secs,
        wilderness_rate,
        resting_rate,
    })
}

fn required_row_value<T>(row: &mysql::Row, column: &str) -> Result<T>
where
    T: mysql::prelude::FromValue,
{
    row.get(column)
        .ok_or_else(|| anyhow!("Missing/invalid `{column}` in QA fixture query"))
}

fn rested_xp_count_rows(conn: &mut mysql::Conn, sql: &str, key: u64, label: &str) -> Result<u64> {
    use mysql::prelude::Queryable;

    conn.exec_first(sql, (key,))
        .map_err(|error| anyhow!("Check rested-XP fixture state in {label}: {error}"))
        .map(|count| count.unwrap_or(0))
}

fn rested_xp_character_restore_point_from_row(
    row: &mysql::Row,
) -> Result<RestedXpCharacterRestorePoint> {
    Ok(RestedXpCharacterRestorePoint {
        level: required_row_value(row, "level")?,
        xp: required_row_value(row, "xp")?,
        rest_state: required_row_value(row, "restState")?,
        player_flags: required_row_value(row, "playerFlags")?,
        rest_bonus: required_row_value(row, "rest_bonus")?,
        logout_time: required_row_value(row, "logout_time")?,
        is_logout_resting: required_row_value(row, "is_logout_resting")?,
        map_id: required_row_value(row, "map")?,
        zone_id: required_row_value(row, "zone")?,
        instance_id: required_row_value(row, "instance_id")?,
        x: required_row_value(row, "position_x")?,
        y: required_row_value(row, "position_y")?,
        z: required_row_value(row, "position_z")?,
        orientation: required_row_value(row, "orientation")?,
        health: required_row_value(row, "health")?,
        powers: [
            required_row_value(row, "power1")?,
            required_row_value(row, "power2")?,
            required_row_value(row, "power3")?,
            required_row_value(row, "power4")?,
            required_row_value(row, "power5")?,
            required_row_value(row, "power6")?,
            required_row_value(row, "power7")?,
            required_row_value(row, "power8")?,
            required_row_value(row, "power9")?,
            required_row_value(row, "power10")?,
        ],
        total_kills: required_row_value(row, "totalKills")?,
        today_kills: required_row_value(row, "todayKills")?,
        yesterday_kills: required_row_value(row, "yesterdayKills")?,
        total_time: required_row_value(row, "totaltime")?,
        level_time: required_row_value(row, "leveltime")?,
        latency: required_row_value(row, "latency")?,
        last_login_build: required_row_value(row, "lastLoginBuild")?,
    })
}

fn prepare_rested_xp_character_phase(
    bot: &config::BotConfig,
    fixture: &RestedXpSmokeFixture,
    phase: RestedXpSmokePhase,
) -> Result<()> {
    use mysql::prelude::Queryable;

    if phase == RestedXpSmokePhase::VerifyRelog {
        return wait_for_rested_xp_character_offline_and_stable(bot, fixture.options.timeout_secs);
    }
    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start rested-XP phase transaction: {error}"))?;
    let online: u8 = transaction
        .exec_first(
            "SELECT online FROM characters WHERE guid = ? FOR UPDATE",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Lock rested-XP character row: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if online != 0 {
        bail!(
            "character {} remained online before {:?}; refusing DB mutation",
            bot.character_guid,
            phase
        );
    }
    let (rest_state, rest_bonus, logout_time, is_logout_resting) = match phase {
        RestedXpSmokePhase::OfflineWilderness => (
            REST_STATE_NORMAL,
            0.0,
            current_epoch_secs().saturating_sub(fixture.offline_secs),
            0u8,
        ),
        RestedXpSmokePhase::OfflineResting => (
            REST_STATE_NORMAL,
            0.0,
            current_epoch_secs().saturating_sub(fixture.offline_secs),
            1u8,
        ),
        RestedXpSmokePhase::ConsumeKill => (
            REST_STATE_RESTED,
            fixture.options.seeded_rest_bonus,
            current_epoch_secs(),
            0,
        ),
        RestedXpSmokePhase::VerifyRelog => unreachable!(),
    };
    let player_flags =
        fixture.original.player_flags & !(PLAYER_FLAGS_RESTING | PLAYER_FLAGS_NO_XP_GAIN);
    let player_x = fixture.options.target.x + 1.0;
    let player_y = fixture.options.target.y;
    let player_z = fixture.options.target.z;
    let player_orientation =
        (fixture.options.target.y - player_y).atan2(fixture.options.target.x - player_x) as f32;
    transaction
        .exec_drop(
            "UPDATE characters SET level = ?, xp = 0, restState = ?, playerFlags = ?, rest_bonus = ?, \
             logout_time = ?, is_logout_resting = ?, map = ?, zone = 0, instance_id = 0, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ?, health = ? \
             WHERE guid = ? AND online = 0",
            mysql::Params::Positional(vec![
                fixture.test_level.into(),
                rest_state.into(),
                player_flags.into(),
                rest_bonus.into(),
                logout_time.into(),
                is_logout_resting.into(),
                u32::from(fixture.options.target.map_id).into(),
                player_x.into(),
                player_y.into(),
                player_z.into(),
                player_orientation.into(),
                u32::MAX.into(),
                bot.character_guid.into(),
            ]),
        )
        .map_err(|error| anyhow!("Prepare rested-XP character phase {phase:?}: {error}"))?;
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit rested-XP phase {phase:?}: {error}"))?;
    Ok(())
}

fn current_epoch_secs() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

fn load_rested_xp_db_state(bot: &config::BotConfig) -> Result<RestedXpDbState> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let row: Option<(u8, u32, u8, f32, u8)> = conn
        .exec_first(
            "SELECT level, xp, restState, rest_bonus, online FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load rested-XP persistence state: {error}"))?;
    row.map(
        |(level, xp, rest_state, rest_bonus, online)| RestedXpDbState {
            level,
            xp,
            rest_state,
            rest_bonus,
            online,
        },
    )
    .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))
}

fn rested_xp_restore_params(
    restore_point: &RestedXpCharacterRestorePoint,
    character_guid: u64,
) -> Vec<mysql::Value> {
    let mut values = vec![
        restore_point.level.into(),
        restore_point.xp.into(),
        restore_point.rest_state.into(),
        restore_point.player_flags.into(),
        restore_point.rest_bonus.into(),
        restore_point.logout_time.into(),
        restore_point.is_logout_resting.into(),
        restore_point.map_id.into(),
        restore_point.zone_id.into(),
        restore_point.instance_id.into(),
        restore_point.x.into(),
        restore_point.y.into(),
        restore_point.z.into(),
        restore_point.orientation.into(),
        restore_point.health.into(),
    ];
    values.extend(restore_point.powers.iter().copied().map(mysql::Value::from));
    values.extend([
        restore_point.total_kills.into(),
        restore_point.today_kills.into(),
        restore_point.yesterday_kills.into(),
        restore_point.total_time.into(),
        restore_point.level_time.into(),
        restore_point.latency.into(),
        restore_point.last_login_build.into(),
        character_guid.into(),
    ]);
    values
}

fn wait_for_rested_xp_character_offline_and_stable(
    bot: &config::BotConfig,
    timeout_secs: u64,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let deadline = std::time::Instant::now()
        + Duration::from_secs(timeout_secs.clamp(10, RESTED_XP_DISCONNECT_SAVE_MAX_WAIT_SECS));
    let mut previous_offline_marker: Option<(u32, u32, u64, u8)> = None;
    loop {
        let row: Option<(u8, u32, f32, u64, u8)> = conn
            .exec_first(
                "SELECT online, xp, rest_bonus, logout_time, is_logout_resting FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Wait for rested-XP disconnect save: {error}"))?;
        let (online, xp, rest_bonus, logout_time, is_logout_resting) =
            row.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
        if online == 0 {
            let marker = (xp, rest_bonus.to_bits(), logout_time, is_logout_resting);
            if previous_offline_marker == Some(marker) {
                return Ok(());
            }
            previous_offline_marker = Some(marker);
        } else {
            previous_offline_marker = None;
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "character {} did not reach a stable offline DB state; refusing selected-field restore",
                bot.character_guid
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn wait_for_rested_xp_game_account_offline(
    bot: &config::BotConfig,
    fixture: &RestedXpSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let auth_url = auth_db_url()?;
    let opts =
        mysql::Opts::from_url(&auth_url).map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|error| anyhow!("Connect to auth DB failed: {error}"))?;
    let deadline = std::time::Instant::now()
        + Duration::from_secs(
            fixture
                .options
                .timeout_secs
                .clamp(10, RESTED_XP_DISCONNECT_SAVE_MAX_WAIT_SECS),
        );
    loop {
        let row: Option<(Option<u32>, u8, Option<String>)> = conn
            .exec_first(
                "SELECT a.battlenet_account, a.online, ba.email \
                 FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
                 WHERE a.id = ?",
                (bot.account_id,),
            )
            .map_err(|error| anyhow!("Wait for rested-XP game account offline: {error}"))?;
        let (battlenet_account_id, online, email) =
            row.ok_or_else(|| anyhow!("No auth.account row for id {}", bot.account_id))?;
        if battlenet_account_id != Some(fixture.battlenet_account_id)
            || !email
                .as_deref()
                .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account))
        {
            bail!(
                "rested-XP fixture identity changed while waiting for account {} to disconnect",
                bot.account_id
            );
        }
        if online == 0 {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "game account {} remained online; refusing rested-XP cleanup mutation",
                bot.account_id
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn wait_for_rested_xp_target_respawn_cleanup(
    conn: &mut mysql::Conn,
    fixture: &RestedXpSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let target = &fixture.options.target;
    let wait_secs = rested_xp_respawn_cleanup_wait_secs(
        fixture.options.timeout_secs,
        fixture.target_respawn_secs,
    );
    let mut deadline = std::time::Instant::now() + Duration::from_secs(wait_secs);
    let mut absent_since = None;
    let mut last_respawn_time = None;
    let mut saw_persisted_respawn = false;
    loop {
        let respawn_time: Option<u64> = conn
            .exec_first(
                "SELECT respawnTime FROM respawn WHERE type = 0 AND spawnId = ? AND mapId = ? AND instanceId = 0",
                (target.spawn_guid, u32::from(target.map_id)),
            )
            .map_err(|error| anyhow!("Wait for rested-XP target respawn cleanup: {error}"))?;
        if let Some(respawn_time) = respawn_time {
            last_respawn_time = Some(respawn_time);
            absent_since = None;
            if !saw_persisted_respawn {
                info!(
                    "Rested-XP target persisted respawn row observed for spawn {} map {} at {}",
                    target.spawn_guid, target.map_id, respawn_time
                );
            }
            saw_persisted_respawn = true;
            let remaining =
                rested_xp_observed_respawn_remaining_secs(respawn_time, current_epoch_secs())?;
            deadline = deadline.max(std::time::Instant::now() + Duration::from_secs(remaining));
        } else {
            if saw_persisted_respawn {
                let absent_since = absent_since.get_or_insert_with(std::time::Instant::now);
                if absent_since.elapsed() >= Duration::from_secs(1) {
                    info!(
                        "Rested-XP target respawn row cleared naturally for spawn {} map {} after the persisted timer was observed",
                        target.spawn_guid, target.map_id
                    );
                    return Ok(());
                }
            }
        }

        if std::time::Instant::now() >= deadline {
            bail!(
                "rested-XP target respawn transition was not observed within the bounded wait (initial_wait={wait_secs}s, spawn={}, map={}, saw_persisted_respawn={}, last_respawnTime={:?}); the harness did not delete it, so wait for the runtime respawn before retrying",
                target.spawn_guid,
                target.map_id,
                saw_persisted_respawn,
                last_respawn_time
            );
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn rested_xp_observed_respawn_remaining_secs(respawn_time: u64, now: u64) -> Result<u64> {
    let remaining = respawn_time
        .saturating_sub(now)
        .saturating_add(RESTED_XP_RESPAWN_GRACE_SECS);
    if remaining > MAX_RESTED_XP_RESPAWN_CLEANUP_WAIT_SECS {
        bail!(
            "observed rested-XP respawn timer requires {remaining}s, exceeding the {MAX_RESTED_XP_RESPAWN_CLEANUP_WAIT_SECS}s safety bound"
        );
    }
    Ok(remaining)
}

fn rested_xp_respawn_cleanup_wait_secs(protocol_timeout_secs: u64, respawn_secs: u32) -> u64 {
    protocol_timeout_secs
        .max(u64::from(respawn_secs).saturating_add(RESTED_XP_RESPAWN_GRACE_SECS))
        .clamp(
            10,
            u64::from(MAX_RESTED_XP_TARGET_RESPAWN_SECS)
                .saturating_add(RESTED_XP_RESPAWN_GRACE_SECS),
        )
}

fn cleanup_rested_xp_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &RestedXpSmokeFixture,
    verify_target_respawn: bool,
) -> Result<()> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing rested-XP fixture cleanup for non-local account {}",
            bot.account
        );
    }
    wait_for_rested_xp_character_offline_and_stable(bot, fixture.options.timeout_secs)?;
    wait_for_rested_xp_game_account_offline(bot, fixture)?;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let mut character_tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start rested-XP character cleanup transaction: {error}"))?;
    let (owner, online): (u32, u8) = character_tx
        .exec_first(
            "SELECT account, online FROM characters WHERE guid = ? FOR UPDATE",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Lock rested-XP cleanup character: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id || online != 0 {
        bail!(
            "rested-XP cleanup character ownership/online state changed (owner={owner}, online={online})"
        );
    }
    let characters_on_game_account: u64 = character_tx
        .exec_first(
            "SELECT COUNT(*) FROM characters WHERE account = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Recheck rested-XP character exclusivity: {error}"))?
        .unwrap_or(0);
    if characters_on_game_account != 1 {
        bail!(
            "rested-XP cleanup requires the game account to remain exclusive; found {characters_on_game_account} characters"
        );
    }

    let auth_url = auth_db_url()?;
    let auth_opts =
        mysql::Opts::from_url(&auth_url).map_err(|error| anyhow!("Bad auth DB URL: {error}"))?;
    let mut auth = mysql::Conn::new(auth_opts)
        .map_err(|error| anyhow!("Connect to auth DB failed: {error}"))?;
    let mut auth_tx = auth
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start rested-XP auth cleanup transaction: {error}"))?;
    let (battlenet_account_id, game_account_online, battlenet_email): (
        Option<u32>,
        u8,
        Option<String>,
    ) = auth_tx
        .exec_first(
            "SELECT a.battlenet_account, a.online, ba.email \
             FROM account a LEFT JOIN battlenet_accounts ba ON ba.id = a.battlenet_account \
             WHERE a.id = ? FOR UPDATE",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Lock rested-XP cleanup game account: {error}"))?
        .ok_or_else(|| anyhow!("No auth.account row for id {}", bot.account_id))?;
    if battlenet_account_id != Some(fixture.battlenet_account_id)
        || game_account_online != 0
        || !battlenet_email
            .as_deref()
            .is_some_and(|email| email.eq_ignore_ascii_case(&bot.account))
    {
        bail!("rested-XP cleanup Battle.net identity or online state changed");
    }
    let game_accounts_on_bnet: u64 = auth_tx
        .exec_first(
            "SELECT COUNT(*) FROM account WHERE battlenet_account = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Recheck rested-XP Battle.net exclusivity: {error}"))?
        .unwrap_or(0);
    if game_accounts_on_bnet != 1 {
        bail!(
            "rested-XP cleanup requires the Battle.net identity to remain exclusive; found {game_accounts_on_bnet} game accounts"
        );
    }

    character_tx
        .exec_drop(
            RESTED_XP_RESTORE_CHARACTER_SQL,
            mysql::Params::Positional(rested_xp_restore_params(
                &fixture.original,
                bot.character_guid,
            )),
        )
        .map_err(|error| anyhow!("Restore rested-XP selected character fields: {error}"))?;
    // A real C++ kill can update both achievement tables. Restore their exact
    // pre-smoke snapshots rather than assuming this disposable character had
    // no existing criteria or completed achievements.
    character_tx
        .exec_drop(
            "DELETE FROM character_achievement WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character achievements: {error}"))?;
    for (achievement, date) in &fixture.original_achievements {
        character_tx
            .exec_drop(
                "INSERT INTO character_achievement (guid, achievement, date) VALUES (?, ?, ?)",
                (bot.character_guid, achievement, date),
            )
            .map_err(|error| anyhow!("Restore rested-XP character achievement: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_achievement_progress WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character achievement progress: {error}"))?;
    for (criteria, counter, date) in &fixture.original_achievement_progress {
        character_tx
            .exec_drop(
                "INSERT INTO character_achievement_progress (guid, criteria, counter, date) VALUES (?, ?, ?, ?)",
                (bot.character_guid, criteria, counter, date),
            )
            .map_err(|error| anyhow!("Restore rested-XP achievement progress: {error}"))?;
    }
    // C++ Player::_LoadTraits may create missing per-specialization configs,
    // and SaveToDB persists them. Restore the exact pre-smoke snapshot so both
    // pre-existing builds and newly materialized defaults are handled safely.
    character_tx
        .exec_drop(
            "DELETE FROM character_trait_entry WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character trait entries: {error}"))?;
    character_tx
        .exec_drop(
            "DELETE FROM character_trait_config WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character trait configs: {error}"))?;
    for (
        trait_config_id,
        config_type,
        chr_specialization_id,
        combat_config_flags,
        local_identifier,
        skill_line_id,
        trait_system_id,
        name,
    ) in &fixture.original_trait_configs
    {
        character_tx
            .exec_drop(
                "INSERT INTO character_trait_config \
                 (guid, traitConfigId, type, chrSpecializationId, combatConfigFlags, \
                  localIdentifier, skillLineId, traitSystemId, name) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    trait_config_id,
                    config_type,
                    chr_specialization_id,
                    combat_config_flags,
                    local_identifier,
                    skill_line_id,
                    trait_system_id,
                    name,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP character trait config: {error}"))?;
    }
    for (trait_config_id, trait_node_id, trait_node_entry_id, rank, granted_ranks) in
        &fixture.original_trait_entries
    {
        character_tx
            .exec_drop(
                "INSERT INTO character_trait_entry \
                 (guid, traitConfigId, traitNodeId, traitNodeEntryId, rank, grantedRanks) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    trait_config_id,
                    trait_node_id,
                    trait_node_entry_id,
                    rank,
                    granted_ranks,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP character trait entry: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character homebind: {error}"))?;
    if let Some((map_id, zone_id, x, y, z, orientation)) = fixture.original_homebind {
        character_tx
            .exec_drop(
                "INSERT INTO character_homebind \
                 (guid, mapId, zoneId, posX, posY, posZ, orientation) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                (bot.character_guid, map_id, zone_id, x, y, z, orientation),
            )
            .map_err(|error| anyhow!("Restore rested-XP character homebind: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_fishingsteps WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character fishing steps: {error}"))?;
    if let Some(fishing_steps) = fixture.original_fishing_steps {
        character_tx
            .exec_drop(
                "INSERT INTO character_fishingsteps (guid, fishingSteps) VALUES (?, ?)",
                (bot.character_guid, fishing_steps),
            )
            .map_err(|error| anyhow!("Restore rested-XP character fishing steps: {error}"))?;
    }
    character_tx
        .exec_drop(
            "DELETE FROM character_battleground_data WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Clear rested-XP character battleground data: {error}"))?;
    if let Some((
        instance_id,
        team,
        join_x,
        join_y,
        join_z,
        join_o,
        join_map_id,
        taxi_start,
        taxi_end,
        mount_spell,
        queue_id,
    )) = fixture.original_battleground_data
    {
        character_tx
            .exec_drop(
                "INSERT INTO character_battleground_data \
                 (guid, instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, \
                  taxiStart, taxiEnd, mountSpell, queueId) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    instance_id,
                    team,
                    join_x,
                    join_y,
                    join_z,
                    join_o,
                    join_map_id,
                    taxi_start,
                    taxi_end,
                    mount_spell,
                    queue_id,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP character battleground data: {error}"))?;
    }
    // Preflight proved these tables were empty. C++ login/save deterministically
    // creates them, so remove only the rows scoped to this disposable fixture.
    for (label, delete_sql, _) in RESTED_XP_CPP_GENERATED_CHARACTER_ROWS {
        character_tx
            .exec_drop(*delete_sql, (bot.character_guid,))
            .map_err(|error| anyhow!("Remove rested-XP fixture {label} rows: {error}"))?;
    }
    for (label, _, count_sql) in RESTED_XP_CPP_GENERATED_CHARACTER_ROWS {
        let rows: u64 = character_tx
            .exec_first(*count_sql, (bot.character_guid,))
            .map_err(|error| anyhow!("Verify rested-XP cleanup {label}: {error}"))?
            .unwrap_or(0);
        if rows != 0 {
            bail!("rested-XP cleanup left {rows} rows in {label}");
        }
    }
    let restored_row: mysql::Row = character_tx
        .exec_first(
            "SELECT level, xp, restState, playerFlags, rest_bonus, logout_time, is_logout_resting, \
             map, zone, instance_id, position_x, position_y, position_z, orientation, health, \
             power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, \
             totalKills, todayKills, yesterdayKills, totaltime, leveltime, latency, lastLoginBuild \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Reload restored rested-XP selected fields: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let restored = rested_xp_character_restore_point_from_row(&restored_row)?;
    if restored != fixture.original {
        bail!("rested-XP cleanup verification did not reproduce the selected character fields");
    }
    let restored_achievements: Vec<(u32, i64)> = character_tx
        .exec(
            "SELECT achievement, date FROM character_achievement WHERE guid = ? ORDER BY achievement",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character achievements: {error}"))?;
    if restored_achievements != fixture.original_achievements {
        bail!("rested-XP cleanup verification did not restore character achievements");
    }
    let restored_achievement_progress: Vec<(u32, u64, i64)> = character_tx
        .exec(
            "SELECT criteria, counter, date FROM character_achievement_progress WHERE guid = ? ORDER BY criteria",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP achievement progress: {error}"))?;
    if restored_achievement_progress != fixture.original_achievement_progress {
        bail!("rested-XP cleanup verification did not restore achievement progress");
    }
    let restored_trait_configs: Vec<RestedXpTraitConfigSnapshot> = character_tx
        .exec(RESTED_XP_SELECT_TRAIT_CONFIGS_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Verify rested-XP character trait configs: {error}"))?;
    if restored_trait_configs != fixture.original_trait_configs {
        bail!("rested-XP cleanup verification did not restore character trait configs");
    }
    let restored_trait_entries: Vec<RestedXpTraitEntrySnapshot> = character_tx
        .exec(RESTED_XP_SELECT_TRAIT_ENTRIES_SQL, (bot.character_guid,))
        .map_err(|error| anyhow!("Verify rested-XP character trait entries: {error}"))?;
    if restored_trait_entries != fixture.original_trait_entries {
        bail!("rested-XP cleanup verification did not restore character trait entries");
    }
    let restored_homebind: Option<RestedXpHomebindSnapshot> = character_tx
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation \
             FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character homebind: {error}"))?;
    if restored_homebind != fixture.original_homebind {
        bail!("rested-XP cleanup verification did not restore character homebind");
    }
    let restored_fishing_steps: Option<u8> = character_tx
        .exec_first(
            "SELECT fishingSteps FROM character_fishingsteps WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character fishing steps: {error}"))?;
    if restored_fishing_steps != fixture.original_fishing_steps {
        bail!("rested-XP cleanup verification did not restore character fishing steps");
    }
    let restored_battleground_data: Option<RestedXpBattlegroundDataSnapshot> = character_tx
        .exec_first(
            "SELECT instanceId, team, joinX, joinY, joinZ, joinO, joinMapId, \
                    taxiStart, taxiEnd, mountSpell, queueId \
             FROM character_battleground_data WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Verify rested-XP character battleground data: {error}"))?;
    if restored_battleground_data != fixture.original_battleground_data {
        bail!("rested-XP cleanup verification did not restore character battleground data");
    }

    auth_tx
        .exec_drop(
            "DELETE FROM account_last_played_character WHERE accountId = ?",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Clear rested-XP last-played character rows: {error}"))?;
    for (region, battlegroup, realm_id, name, character_guid, last_played_time) in
        &fixture.original_last_played_characters
    {
        auth_tx
            .exec_drop(
                "INSERT INTO account_last_played_character \
                 (accountId, region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.account_id,
                    region,
                    battlegroup,
                    realm_id,
                    name,
                    character_guid,
                    last_played_time,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP last-played character row: {error}"))?;
    }
    auth_tx
        .exec_drop(
            "DELETE FROM battle_pet_slots WHERE battlenetAccountId = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Clear rested-XP Battle.net pet slots: {error}"))?;
    for (slot_id, battle_pet_guid, locked) in &fixture.original_battle_pet_slots {
        auth_tx
            .exec_drop(
                "INSERT INTO battle_pet_slots \
                 (id, battlenetAccountId, battlePetGuid, locked) VALUES (?, ?, ?, ?)",
                (
                    slot_id,
                    fixture.battlenet_account_id,
                    battle_pet_guid,
                    locked,
                ),
            )
            .map_err(|error| anyhow!("Restore rested-XP Battle.net pet slot: {error}"))?;
    }
    auth_tx
        .exec_drop(
            "DELETE FROM battlenet_account_transmog_illusions WHERE battlenetAccountId = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Remove rested-XP fixture illusion rows: {error}"))?;
    let illusion_rows: u64 = auth_tx
        .exec_first(
            "SELECT COUNT(*) FROM battlenet_account_transmog_illusions WHERE battlenetAccountId = ?",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Verify rested-XP illusion cleanup: {error}"))?
        .unwrap_or(0);
    if illusion_rows != 0 {
        bail!("rested-XP cleanup left {illusion_rows} transmog illusion rows");
    }
    let restored_last_played_characters: Vec<RestedXpLastPlayedCharacterSnapshot> = auth_tx
        .exec(
            "SELECT region, battlegroup, realmId, characterName, characterGUID, lastPlayedTime \
             FROM account_last_played_character WHERE accountId = ? \
             ORDER BY region, battlegroup",
            (bot.account_id,),
        )
        .map_err(|error| anyhow!("Verify rested-XP last-played character rows: {error}"))?;
    if restored_last_played_characters != fixture.original_last_played_characters {
        bail!("rested-XP cleanup verification did not restore last-played character rows");
    }
    let restored_battle_pet_slots: Vec<RestedXpBattlePetSlotSnapshot> = auth_tx
        .exec(
            "SELECT id, battlePetGuid, locked FROM battle_pet_slots \
             WHERE battlenetAccountId = ? ORDER BY id",
            (fixture.battlenet_account_id,),
        )
        .map_err(|error| anyhow!("Verify rested-XP Battle.net pet slots: {error}"))?;
    if restored_battle_pet_slots != fixture.original_battle_pet_slots {
        bail!("rested-XP cleanup verification did not restore Battle.net pet slots");
    }

    character_tx
        .commit()
        .map_err(|error| anyhow!("Commit rested-XP character cleanup: {error}"))?;
    auth_tx
        .commit()
        .map_err(|error| anyhow!("Commit rested-XP auth cleanup: {error}"))?;
    info!(
        "Rested-XP fixture restored character/account snapshots and deterministic glyph/reputation/skill/illusion save rows for character {}",
        bot.character_guid,
    );
    if verify_target_respawn {
        wait_for_rested_xp_target_respawn_cleanup(&mut conn, fixture)?;
    } else {
        info!(
            "Rested-XP workflow did not prove a target kill; skipped the inapplicable respawn-transition wait"
        );
    }
    Ok(())
}

fn prepare_inventory_swap_smoke_fixture(
    bot: &config::BotConfig,
    item_entry_a: u32,
    item_entry_b: u32,
    timeout_secs: u64,
) -> Result<InventorySwapSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive inventory swap fixture setup for non-local account {}",
            bot.account
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let character: Option<(u32, u8)> = characters
        .exec_first(
            "SELECT account, online FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load inventory swap bot character: {e}"))?;
    let (owner, online) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; log it out before inventory swap smoke setup",
            bot.character_guid
        );
    }

    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|e| anyhow!("Load occupied inventory swap slots: {e}"))?;
    let free_slots: Vec<u8> = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .filter(|slot| !occupied_slots.contains(slot))
        .take(2)
        .collect();
    if free_slots.len() != 2 {
        bail!("Two empty default backpack slots are required for inventory swap smoke");
    }
    let slot_a = free_slots[0];
    let slot_b = free_slots[1];

    for item_entry in [item_entry_a, item_entry_b] {
        let owned_count: u64 = characters
            .exec_first(
                "SELECT COUNT(*) FROM character_inventory ci \
                 JOIN item_instance ii ON ii.guid = ci.item \
                 WHERE ci.guid = ? AND ii.itemEntry = ?",
                (bot.character_guid, item_entry),
            )
            .map_err(|e| anyhow!("Check existing inventory swap item entry: {e}"))?
            .unwrap_or(0);
        if owned_count != 0 {
            bail!(
                "bot character already owns item entry {item_entry}; choose isolated inventory-swap item entries"
            );
        }
    }

    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|e| anyhow!("Load max item guid: {e}"))?
        .unwrap_or(0);
    let item_guid_a = max_item_guid
        .checked_add(20_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving inventory swap fixture"))?;
    let item_guid_b = item_guid_a
        .checked_add(1)
        .ok_or_else(|| anyhow!("item guid overflow while reserving inventory swap fixture"))?;

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start inventory swap fixture transaction: {e}"))?;
    for (item_guid, item_entry, slot) in [
        (item_guid_a, item_entry_a, slot_a),
        (item_guid_b, item_entry_b, slot_b),
    ] {
        transaction
            .exec_drop(
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
                  enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
                (item_guid, item_entry, bot.character_guid),
            )
            .map_err(|e| anyhow!("Insert inventory swap fixture item: {e}"))?;
        transaction
            .exec_drop(
                "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
                (bot.character_guid, slot, item_guid),
            )
            .map_err(|e| anyhow!("Insert inventory swap fixture inventory row: {e}"))?;
    }
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit inventory swap fixture transaction: {e}"))?;

    info!(
        "Inventory swap fixture: character={} items={}/{} entries={}/{} slots={}/{}",
        bot.character_guid, item_guid_a, item_guid_b, item_entry_a, item_entry_b, slot_a, slot_b
    );
    Ok(InventorySwapSmokeFixture {
        options: InventorySwapSmokeOptions {
            phase: InventorySwapSmokePhase::Forward,
            item_guid_a,
            item_guid_b,
            item_entry_a,
            item_entry_b,
            slot_a,
            slot_b,
            timeout_secs,
        },
    })
}

fn verify_inventory_swap_fixture_locations(
    bot: &config::BotConfig,
    options: &InventorySwapSmokeOptions,
    expected_slot_a: u8,
    expected_slot_b: u8,
) -> Result<bool> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let load = |conn: &mut mysql::Conn, item_guid: u64| -> Result<Option<(u64, u8, u32, u64)>> {
        conn.exec_first(
            "SELECT ci.bag, ci.slot, ii.itemEntry, ii.owner_guid \
             FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ci.item = ? AND ii.count = 1",
            (bot.character_guid, item_guid),
        )
        .map_err(|e| anyhow!("Load inventory swap fixture location: {e}"))
    };
    let row_a = load(&mut conn, options.item_guid_a)?;
    let row_b = load(&mut conn, options.item_guid_b)?;
    Ok(matches!(
        row_a,
        Some((0, slot, entry, owner))
            if slot == expected_slot_a
                && entry == options.item_entry_a
                && owner == bot.character_guid
    ) && matches!(
        row_b,
        Some((0, slot, entry, owner))
            if slot == expected_slot_b
                && entry == options.item_entry_b
                && owner == bot.character_guid
    ))
}

fn cleanup_inventory_swap_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &InventorySwapSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check inventory swap bot offline state before cleanup: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => {
                bail!(
                    "character {} remained online; refusing inventory swap cleanup before disconnect save",
                    bot.character_guid
                );
            }
            None => bail!(
                "No characters row for guid {} during inventory swap cleanup",
                bot.character_guid
            ),
        }
    }

    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start inventory swap cleanup transaction: {e}"))?;
    for item_guid in [fixture.options.item_guid_a, fixture.options.item_guid_b] {
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, item_guid),
            )
            .map_err(|e| anyhow!("Delete inventory swap fixture inventory row: {e}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
                (item_guid, bot.character_guid),
            )
            .map_err(|e| anyhow!("Delete inventory swap fixture item: {e}"))?;
    }
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit inventory swap cleanup transaction: {e}"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn prepare_vendor_smoke_fixture(
    bot: &config::BotConfig,
    vendor_entry: u32,
    vendor_spawn_guid: u64,
    runtime_counter: Option<u64>,
    item_entry: u32,
    extended_cost: u32,
    currency_id: u32,
    currency_cost: u32,
    currency_quantity: u32,
    timeout_secs: u64,
) -> Result<VendorSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive vendor fixture setup for non-local account {}",
            bot.account
        );
    }
    if item_entry > i32::MAX as u32
        || extended_cost > i32::MAX as u32
        || currency_id > u16::MAX as u32
        || currency_quantity <= currency_cost
    {
        bail!("vendor fixture identifiers/quantity do not fit the 3.4.3 wire/database shape");
    }
    if runtime_counter.is_some_and(|counter| counter == 0 || counter > OBJECT_GUID_COUNTER_MASK) {
        bail!("vendor runtime counter override must fit the nonzero 40-bit ObjectGuid counter");
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let character: Option<(u32, u8, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load vendor bot character: {error}"))?;
    let (owner, online, map_id, zone_id, instance_id, x, y, z, orientation) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; log it out before vendor smoke setup",
            bot.character_guid
        );
    }
    let original_position = CharacterPositionSnapshot {
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    };
    let original_currency: Option<VendorCurrencyRowSnapshot> = characters
        .exec_first::<(u32, u32, u32, u32, u32, u8), _, _>(
            "SELECT Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags \
             FROM character_currency WHERE CharacterGuid = ? AND Currency = ?",
            (bot.character_guid, currency_id),
        )
        .map_err(|error| anyhow!("Load original vendor currency row: {error}"))?
        .map(
            |(
                quantity,
                weekly_quantity,
                tracked_quantity,
                increased_cap_quantity,
                earned_quantity,
                flags,
            )| VendorCurrencyRowSnapshot {
                quantity,
                weekly_quantity,
                tracked_quantity,
                increased_cap_quantity,
                earned_quantity,
                flags,
            },
        );
    let existing_item_total: u64 = characters
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0) FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Check existing vendor fixture item: {error}"))?
        .unwrap_or(0);
    if existing_item_total != 0 {
        bail!(
            "bot character already owns {} of vendor item {}; choose an isolated fixture item",
            existing_item_total,
            item_entry
        );
    }
    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|error| anyhow!("Load vendor bot occupied backpack slots: {error}"))?;
    if !(INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .any(|slot| !occupied_slots.contains(&slot))
    {
        bail!("No empty default backpack slot for vendor smoke");
    }

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let spawn: Option<(u32, u32, f64, f64, f64, f32, f32, u32, u32, String)> = world
        .exec_first(
            "SELECT c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation, \
                    c.wander_distance, c.phaseId, c.phaseGroup, c.spawnDifficulties \
             FROM creature c WHERE c.guid = ?",
            (vendor_spawn_guid,),
        )
        .map_err(|error| anyhow!("Load exact vendor spawn: {error}"))?;
    let (
        spawn_entry,
        vendor_map,
        vendor_x,
        vendor_y,
        vendor_z,
        vendor_o,
        wander_distance,
        phase_id,
        phase_group,
        spawn_difficulties,
    ) = spawn.ok_or_else(|| anyhow!("No world.creature row for guid {vendor_spawn_guid}"))?;
    if spawn_entry != vendor_entry {
        bail!(
            "vendor spawn {} has entry {}, expected {}",
            vendor_spawn_guid,
            spawn_entry,
            vendor_entry
        );
    }
    if phase_id != 0 || phase_group != 0 || !spawn_difficulties.split(',').any(|id| id == "0") {
        bail!(
            "vendor spawn {} is not a deterministic base-phase difficulty-0 fixture",
            vendor_spawn_guid
        );
    }
    let target_match_radius = wander_distance.max(0.0) + 2.0;
    if runtime_counter.is_none() {
        let overlapping_spawn: Option<u64> = world
            .exec_first(
                "SELECT guid FROM creature \
                 WHERE id = ? AND map = ? AND guid <> ? \
                   AND SQRT(POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) \
                       <= ? + GREATEST(wander_distance, 0) \
                 ORDER BY guid LIMIT 1",
                (
                    vendor_entry,
                    vendor_map,
                    vendor_spawn_guid,
                    vendor_x,
                    vendor_y,
                    vendor_z,
                    target_match_radius,
                ),
            )
            .map_err(|error| anyhow!("Check vendor spawn ambiguity: {error}"))?;
        if let Some(overlapping_spawn) = overlapping_spawn {
            bail!(
                "vendor SQL spawn {vendor_spawn_guid} overlaps same-entry spawn {overlapping_spawn}; supply a trusted live runtime counter or choose an isolated spawn"
            );
        }
    }
    let vendor_row_count: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM npc_vendor \
             WHERE entry = ? AND item = ? AND ExtendedCost = ? AND type = 1",
            (vendor_entry, item_entry, extended_cost),
        )
        .map_err(|error| anyhow!("Validate exact npc_vendor row: {error}"))?
        .unwrap_or(0);
    if vendor_row_count != 1 {
        bail!(
            "expected one npc_vendor row for vendor/item/extended-cost {vendor_entry}/{item_entry}/{extended_cost}, found {vendor_row_count}"
        );
    }
    let vendor_map = u16::try_from(vendor_map)
        .map_err(|_| anyhow!("vendor map id does not fit protocol: {vendor_map}"))?;
    let guid_counter = runtime_counter.unwrap_or(0);
    let packed_guid = if guid_counter == 0 {
        Vec::new()
    } else {
        let (low, high) = create_creature_guid_raw(vendor_map, vendor_entry, guid_counter);
        build_packed_guid(low, high)
    };
    let vendor = ResolvedCreatureTarget {
        entry: vendor_entry,
        spawn_guid: vendor_spawn_guid,
        guid_counter,
        map_id: vendor_map,
        x: vendor_x,
        y: vendor_y,
        z: vendor_z,
        orientation: vendor_o,
        packed_guid,
    };

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start vendor fixture transaction: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_currency \
             (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) \
             VALUES (?, ?, ?, 0, 0, 0, 0, 0) \
             ON DUPLICATE KEY UPDATE Quantity = VALUES(Quantity), WeeklyQuantity = 0, \
                 TrackedQuantity = 0, IncreasedCapQuantity = 0, EarnedQuantity = 0, Flags = 0",
            (bot.character_guid, currency_id, currency_quantity),
        )
        .map_err(|error| anyhow!("Seed vendor currency fixture: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = 0, instance_id = 0, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                u32::from(vendor_map),
                vendor_x + 2.0,
                vendor_y,
                vendor_z,
                vendor_o,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Relocate vendor bot near vendor: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("vendor character relocation lost its offline ownership guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit vendor fixture transaction: {error}"))?;

    info!(
        "Vendor fixture: character={} vendor={}/{} counter={} item={} extended_cost={} currency={} quantity/cost={}/{}",
        bot.character_guid,
        vendor_entry,
        vendor_spawn_guid,
        guid_counter,
        item_entry,
        extended_cost,
        currency_id,
        currency_quantity,
        currency_cost
    );
    Ok(VendorSmokeFixture {
        options: VendorSmokeOptions {
            phase: VendorSmokePhase::Purchase,
            vendor,
            target_match_radius,
            item_entry,
            extended_cost,
            currency_id,
            currency_before: currency_quantity,
            currency_cost,
            expected_item_total: 1,
            timeout_secs,
        },
        original_position,
        original_currency,
    })
}

fn load_vendor_smoke_db_state(
    bot: &config::BotConfig,
    currency_id: u32,
    item_entry: u32,
) -> Result<(u32, u64)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let currency = conn
        .exec_first(
            "SELECT Quantity FROM character_currency WHERE CharacterGuid = ? AND Currency = ?",
            (bot.character_guid, currency_id),
        )
        .map_err(|error| anyhow!("Load vendor currency DB state: {error}"))?
        .unwrap_or(0);
    let item_total = conn
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0) FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Load vendor item DB state: {error}"))?
        .unwrap_or(0);
    Ok((currency, item_total))
}

fn cleanup_vendor_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &VendorSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    // Stock C++ may defer its disconnected-session save/offline transition
    // substantially longer than Rust. A failed phase already attempts a
    // graceful logout, but retain a bounded disconnect fallback as well.
    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Check vendor bot offline before cleanup: {error}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online; refusing vendor cleanup",
                bot.character_guid
            ),
            None => bail!(
                "No characters row for guid {} during vendor cleanup",
                bot.character_guid
            ),
        }
    }

    let purchased_guids: Vec<u64> = conn
        .exec_map(
            "SELECT ii.guid FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ? ORDER BY ii.guid",
            (bot.character_guid, fixture.options.item_entry),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Load purchased vendor fixture item GUIDs: {error}"))?;
    if purchased_guids.len() > fixture.options.expected_item_total as usize {
        bail!(
            "vendor cleanup found {} fixture item stacks, expected at most {}",
            purchased_guids.len(),
            fixture.options.expected_item_total
        );
    }

    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start vendor cleanup transaction: {error}"))?;
    for item_guid in purchased_guids {
        transaction
            .exec_drop(
                "DELETE FROM item_refund_instance WHERE item_guid = ?",
                (item_guid,),
            )
            .map_err(|error| anyhow!("Delete vendor refund metadata: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, item_guid),
            )
            .map_err(|error| anyhow!("Delete vendor inventory row: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
                (item_guid, bot.character_guid),
            )
            .map_err(|error| anyhow!("Delete vendor item instance: {error}"))?;
    }
    transaction
        .exec_drop(
            "DELETE FROM character_currency WHERE CharacterGuid = ? AND Currency = ?",
            (bot.character_guid, fixture.options.currency_id),
        )
        .map_err(|error| anyhow!("Clear vendor fixture currency row: {error}"))?;
    if let Some(currency) = fixture.original_currency {
        transaction
            .exec_drop(
                "INSERT INTO character_currency \
                 (CharacterGuid, Currency, Quantity, WeeklyQuantity, TrackedQuantity, IncreasedCapQuantity, EarnedQuantity, Flags) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    bot.character_guid,
                    fixture.options.currency_id,
                    currency.quantity,
                    currency.weekly_quantity,
                    currency.tracked_quantity,
                    currency.increased_cap_quantity,
                    currency.earned_quantity,
                    currency.flags,
                ),
            )
            .map_err(|error| anyhow!("Restore vendor currency snapshot: {error}"))?;
    }
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                fixture.original_position.map_id,
                fixture.original_position.zone_id,
                fixture.original_position.instance_id,
                fixture.original_position.x,
                fixture.original_position.y,
                fixture.original_position.z,
                fixture.original_position.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Restore vendor bot position: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("vendor cleanup lost its offline character guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit vendor fixture cleanup: {error}"))?;

    let (restored_currency, restored_item_total) =
        load_vendor_smoke_db_state(bot, fixture.options.currency_id, fixture.options.item_entry)?;
    let expected_currency = fixture
        .original_currency
        .map(|currency| currency.quantity)
        .unwrap_or(0);
    if restored_currency != expected_currency || restored_item_total != 0 {
        bail!(
            "vendor cleanup verification found currency/item {restored_currency}/{restored_item_total}, expected {expected_currency}/0"
        );
    }
    Ok(())
}

fn load_void_storage_db_state(
    bot: &config::BotConfig,
    item_entry: u32,
) -> Result<VoidStorageDbState> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let (money, player_flags): (u64, u32) = conn
        .exec_first(
            "SELECT money, playerFlags FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load void-storage character state: {error}"))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    let void_items = conn
        .exec_map(
            "SELECT itemId, itemEntry, slot FROM character_void_storage WHERE playerGuid = ? ORDER BY slot",
            (bot.character_guid,),
            |(item_id, entry, slot): (u64, u32, u8)| (item_id, entry, slot),
        )
        .map_err(|error| anyhow!("Load character_void_storage state: {error}"))?;
    let inventory_items = conn
        .exec_map(
            "SELECT ii.guid, ci.slot, ii.flags FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ? ORDER BY ii.guid",
            (bot.character_guid, item_entry),
            |(guid, slot, flags): (u64, u8, u32)| (guid, slot, flags),
        )
        .map_err(|error| anyhow!("Load void-storage fixture inventory state: {error}"))?;
    Ok(VoidStorageDbState {
        money,
        player_flags,
        void_items,
        inventory_items,
    })
}

fn prepare_void_storage_smoke_fixture(
    bot: &config::BotConfig,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<VoidStorageSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive void-storage fixture setup for non-local account {}",
            bot.account
        );
    }
    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let character: Option<(u32, u8, u64, u32, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, money, playerFlags, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load void-storage bot character: {error}"))?;
    let (
        owner,
        online,
        original_money,
        original_player_flags,
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    ) = character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; log it out before void-storage smoke setup",
            bot.character_guid
        );
    }
    let existing_void_items: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_void_storage WHERE playerGuid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Check existing void-storage rows: {error}"))?
        .unwrap_or(0);
    if existing_void_items != 0 {
        bail!(
            "character {} already has {existing_void_items} void-storage rows; use an empty disposable bot",
            bot.character_guid
        );
    }
    let same_entry_count: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Check existing void-storage fixture item entry: {error}"))?
        .unwrap_or(0);
    if same_entry_count != 0 {
        bail!(
            "bot character already owns item entry {item_entry}; choose another --void-storage-item-entry"
        );
    }
    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|error| anyhow!("Load occupied void-storage bot slots: {error}"))?;
    let inventory_slot = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty default backpack slot for void-storage smoke"))?;
    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|error| anyhow!("Load max item guid: {error}"))?
        .unwrap_or(0);
    let item_guid = max_item_guid
        .checked_add(10_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving void-storage fixture"))?;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|error| anyhow!("Bad world DB URL: {error}"))?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let neutral: Option<(u64, u32, u32, f64, f64, f64, f32)> = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id \
             WHERE ct.faction = 35 \
               AND ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
               AND c.phaseid = 0 AND c.phasegroup = 0 \
               AND FIND_IN_SET('0', c.spawnDifficulties) > 0 \
               AND ct.VehicleId = 0 ORDER BY c.guid LIMIT 1",
            (NPC_FLAG_VAULT_KEEPER,),
        )
        .map_err(|error| anyhow!("Resolve neutral vault keeper: {error}"))?;
    let vault_row = match neutral {
        Some(row) => row,
        None => world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 WHERE ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
                 ORDER BY c.guid LIMIT 1",
                (NPC_FLAG_VAULT_KEEPER,),
            )
            .map_err(|error| anyhow!("Resolve fallback vault keeper: {error}"))?
            .ok_or_else(|| anyhow!("No vault-keeper creature spawn exists in world DB"))?,
    };
    let (spawn_guid, entry, vault_map, vault_x, vault_y, vault_z, vault_orientation) = vault_row;
    let vault_map = u16::try_from(vault_map)
        .map_err(|_| anyhow!("vault-keeper map id does not fit protocol: {vault_map}"))?;
    let runtime_realm_id = void_storage_runtime_realm_id()?;
    let discover_runtime_guid = runtime_counter.is_none();
    let guid_counter = runtime_counter.unwrap_or(spawn_guid);
    let (low, high) =
        create_void_storage_creature_guid_raw(vault_map, entry, guid_counter, runtime_realm_id);
    let vault_keeper = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id: vault_map,
        x: vault_x,
        y: vault_y,
        z: vault_z,
        orientation: vault_orientation,
        packed_guid: build_packed_guid(low, high),
    };

    let fixture_money = original_money.max(
        VOID_STORAGE_UNLOCK_COST
            .saturating_add(VOID_STORAGE_STORE_ITEM_COST)
            .saturating_add(10_000),
    );
    let fixture_flags = original_player_flags & !PLAYER_FLAGS_VOID_UNLOCKED;
    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start void-storage fixture transaction: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO item_instance \
             (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
              enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
             VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
            (item_guid, item_entry, bot.character_guid),
        )
        .map_err(|error| anyhow!("Insert void-storage fixture item: {error}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
            (bot.character_guid, inventory_slot, item_guid),
        )
        .map_err(|error| anyhow!("Insert void-storage fixture inventory row: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET money = ?, playerFlags = ?, map = ?, zone = 0, instance_id = 0, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                fixture_money,
                fixture_flags,
                u32::from(vault_map),
                vault_x + 2.0,
                vault_y,
                vault_z,
                vault_orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Relocate and seed void-storage bot: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("void-storage fixture lost its offline character guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit void-storage fixture transaction: {error}"))?;

    info!(
        "Void-storage fixture: character={} item={}/entry={} slot={} vault={}/{} runtime_counter={}",
        bot.character_guid,
        item_guid,
        item_entry,
        inventory_slot,
        entry,
        spawn_guid,
        guid_counter
    );
    Ok(VoidStorageSmokeFixture {
        options: VoidStorageSmokeOptions {
            phase: VoidStorageSmokePhase::UnlockDeposit,
            vault_keeper,
            runtime_realm_id,
            discover_runtime_guid,
            fixture_item_guid: item_guid,
            item_entry,
            inventory_slot,
            expected_void_item_id: None,
            expected_void_slot: 0,
            timeout_secs,
        },
        original_position: CharacterPositionSnapshot {
            map_id,
            zone_id,
            instance_id,
            x,
            y,
            z,
            orientation,
        },
        original_money,
        original_player_flags,
    })
}

fn prepare_void_storage_query_capture_fixture(
    bot: &config::BotConfig,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<VoidStorageSmokeFixture> {
    use mysql::prelude::Queryable;

    let mut fixture = prepare_void_storage_smoke_fixture(
        bot,
        item_entry,
        runtime_counter,
        timeout_secs,
    )?;
    let setup = (|| {
        let characters_url = characters_db_url()?;
        let opts = mysql::Opts::from_url(&characters_url)
            .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
        let mut conn = mysql::Conn::new(opts)
            .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
        let max_void_item_id: u64 = conn
            .query_first("SELECT COALESCE(MAX(itemId), 0) FROM character_void_storage")
            .map_err(|error| anyhow!("Load max void item ID for query capture: {error}"))?
            .unwrap_or(0);
        let void_item_id = max_void_item_id
            .checked_add(10_000)
            .ok_or_else(|| anyhow!("void item ID overflow while reserving query fixture"))?;
        let mut transaction = conn
            .start_transaction(mysql::TxOpts::default())
            .map_err(|error| anyhow!("Start void-storage query fixture transaction: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, fixture.options.fixture_item_guid),
            )
            .map_err(|error| anyhow!("Delete query fixture inventory row: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ? AND itemEntry = ?",
                (
                    fixture.options.fixture_item_guid,
                    bot.character_guid,
                    item_entry,
                ),
            )
            .map_err(|error| anyhow!("Delete query fixture item instance: {error}"))?;
        transaction
            .exec_drop(
                "INSERT INTO character_void_storage \
                 (itemId, playerGuid, itemEntry, slot, creatorGuid, fixedScalingLevel, \
                  randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, 0, 0, 0, 0)",
                (void_item_id, bot.character_guid, item_entry),
            )
            .map_err(|error| anyhow!("Insert void-storage query fixture row: {error}"))?;
        transaction
            .exec_drop(
                "UPDATE characters SET playerFlags = playerFlags | ? WHERE guid = ? AND online = 0",
                (PLAYER_FLAGS_VOID_UNLOCKED, bot.character_guid),
            )
            .map_err(|error| anyhow!("Unlock void-storage query fixture: {error}"))?;
        if transaction.affected_rows() != 1 {
            bail!("void-storage query fixture lost its offline character guard");
        }
        transaction
            .commit()
            .map_err(|error| anyhow!("Commit void-storage query fixture: {error}"))?;
        Ok::<u64, anyhow::Error>(void_item_id)
    })();

    let void_item_id = match setup {
        Ok(void_item_id) => void_item_id,
        Err(error) => {
            return match cleanup_void_storage_smoke_fixture(bot, &fixture) {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(anyhow!(
                    "Void-storage query fixture setup failed: {error}; cleanup failed: {cleanup_error}"
                )),
            };
        }
    };
    fixture.options.phase = VoidStorageSmokePhase::QueryCapture;
    fixture.options.expected_void_item_id = Some(void_item_id);
    fixture.options.expected_void_slot = 0;
    Ok(fixture)
}

fn cleanup_void_storage_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &VoidStorageSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Check void-storage bot offline state: {error}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online; refusing void-storage fixture cleanup",
                bot.character_guid
            ),
            None => bail!(
                "No characters row for guid {} during void-storage cleanup",
                bot.character_guid
            ),
        }
    }

    let item_guids: Vec<u64> = conn
        .exec_map(
            "SELECT ii.guid FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, fixture.options.item_entry),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Resolve void-storage cleanup items: {error}"))?;
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start void-storage cleanup transaction: {error}"))?;
    for item_guid in item_guids {
        transaction
            .exec_drop(
                "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
                (bot.character_guid, item_guid),
            )
            .map_err(|error| anyhow!("Delete void-storage fixture inventory row: {error}"))?;
        transaction
            .exec_drop(
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ? AND itemEntry = ?",
                (item_guid, bot.character_guid, fixture.options.item_entry),
            )
            .map_err(|error| anyhow!("Delete void-storage fixture item: {error}"))?;
    }
    transaction
        .exec_drop(
            "DELETE FROM character_void_storage WHERE playerGuid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Delete void-storage fixture rows: {error}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET money = ?, playerFlags = ?, map = ?, zone = ?, instance_id = ?, \
             position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ? AND online = 0",
            (
                fixture.original_money,
                fixture.original_player_flags,
                fixture.original_position.map_id,
                fixture.original_position.zone_id,
                fixture.original_position.instance_id,
                fixture.original_position.x,
                fixture.original_position.y,
                fixture.original_position.z,
                fixture.original_position.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|error| anyhow!("Restore void-storage bot character: {error}"))?;
    if transaction.affected_rows() != 1 {
        bail!("void-storage cleanup lost its offline character guard");
    }
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit void-storage fixture cleanup: {error}"))?;
    let restored = load_void_storage_db_state(bot, fixture.options.item_entry)?;
    if restored.money != fixture.original_money
        || restored.player_flags != fixture.original_player_flags
        || !restored.void_items.is_empty()
        || !restored.inventory_items.is_empty()
    {
        bail!("void-storage cleanup verification failed: {restored:?}");
    }
    Ok(())
}

fn prepare_bank_smoke_fixture(
    bot: &config::BotConfig,
    item_entry: u32,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<BankSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive bank fixture setup for non-local account {}",
            bot.account
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    let character: Option<(u32, u8, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load bank bot character: {e}"))?;
    let (owner, online, map_id, zone_id, instance_id, x, y, z, orientation) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; log it out before bank smoke setup",
            bot.character_guid
        );
    }
    let original_position = CharacterPositionSnapshot {
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    };

    let occupied_slots: Vec<u8> = characters
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (bot.character_guid,),
            |slot: u8| slot,
        )
        .map_err(|e| anyhow!("Load occupied bank bot slots: {e}"))?;
    let inventory_slot = (INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty default backpack slot for bank smoke"))?;
    let bank_slot = (BANK_SLOT_ITEM_START..BANK_SLOT_ITEM_END)
        .find(|slot| !occupied_slots.contains(slot))
        .ok_or_else(|| anyhow!("No empty personal bank slot for bank smoke"))?;

    let same_entry_count: u64 = characters
        .exec_first(
            "SELECT COUNT(*) FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ii.itemEntry = ?",
            (bot.character_guid, item_entry),
        )
        .map_err(|e| anyhow!("Check existing fixture item entry: {e}"))?
        .unwrap_or(0);
    if same_entry_count != 0 {
        bail!(
            "bot character already owns item entry {}; choose another --bank-item-entry to keep the fixture isolated",
            item_entry
        );
    }

    let max_item_guid: u64 = characters
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|e| anyhow!("Load max item guid: {e}"))?
        .unwrap_or(0);
    let item_guid = max_item_guid
        .checked_add(10_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving bank fixture"))?;

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|e| anyhow!("Bad world DB URL: {e}"))?;
    let mut world =
        mysql::Conn::new(world_opts).map_err(|e| anyhow!("Connect to world DB failed: {e}"))?;
    // Use a neutral banker fixture. Picking the geometrically nearest banker can
    // cross faction boundaries on continent maps (for example Exodar vs.
    // Silvermoon), and C++ `GetNPCIfCanInteractWith` correctly rejects hostile
    // NPCs even when their BANKER flag and distance are valid.
    let neutral: Option<(u64, u32, u32, f64, f64, f64, f32)> = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id \
             WHERE ct.faction = 35 \
               AND ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
               AND c.phaseid = 0 AND c.phasegroup = 0 \
               AND FIND_IN_SET('0', c.spawnDifficulties) > 0 \
               AND ct.VehicleId = 0 \
             ORDER BY c.guid LIMIT 1",
            (NPC_FLAG_BANKER,),
        )
        .map_err(|e| anyhow!("Resolve neutral banker: {e}"))?;
    let banker_row = match neutral {
        Some(row) => row,
        None => world
            .exec_first(
                "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
                 FROM creature c JOIN creature_template ct ON ct.entry = c.id \
                 WHERE ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
                 ORDER BY c.guid LIMIT 1",
                (NPC_FLAG_BANKER,),
            )
            .map_err(|e| anyhow!("Resolve fallback banker: {e}"))?
            .ok_or_else(|| anyhow!("No banker creature spawn exists in world DB"))?,
    };
    let (spawn_guid, entry, banker_map, banker_x, banker_y, banker_z, banker_orientation) =
        banker_row;
    let banker_map = u16::try_from(banker_map)
        .map_err(|_| anyhow!("banker map id does not fit protocol: {banker_map}"))?;
    let guid_counter = runtime_counter.ok_or_else(|| {
        anyhow!(
            "banker entry {entry} resolved world.creature guid {spawn_guid}, but needs the live ObjectGuid low counter"
        )
    })?;
    let (low, high) = create_creature_guid_raw(banker_map, entry, guid_counter);
    let banker = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id: banker_map,
        x: banker_x,
        y: banker_y,
        z: banker_z,
        orientation: banker_orientation,
        packed_guid: build_packed_guid(low, high),
    };

    let mut transaction = characters
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start bank fixture transaction: {e}"))?;
    transaction
        .exec_drop(
            "INSERT INTO item_instance \
             (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
              enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
             VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
            (item_guid, item_entry, bot.character_guid),
        )
        .map_err(|e| anyhow!("Insert bank fixture item: {e}"))?;
    transaction
        .exec_drop(
            "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
            (bot.character_guid, inventory_slot, item_guid),
        )
        .map_err(|e| anyhow!("Insert bank fixture inventory row: {e}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                u32::from(banker_map),
                banker_x + 2.0,
                banker_y,
                banker_z,
                banker_orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Relocate bank bot near banker: {e}"))?;
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit bank fixture transaction: {e}"))?;

    info!(
        "Bank smoke fixture: character={} item={}/entry={} inventory_slot={} bank_slot={} banker={}/{} runtime_counter={}",
        bot.character_guid,
        item_guid,
        item_entry,
        inventory_slot,
        bank_slot,
        entry,
        spawn_guid,
        guid_counter
    );
    Ok(BankSmokeFixture {
        options: BankSmokeOptions {
            phase: BankSmokePhase::Deposit,
            banker,
            item_guid,
            item_entry,
            inventory_slot,
            bank_slot,
            timeout_secs,
        },
        original_position,
    })
}

fn prepare_homebind_smoke_fixture(
    bot: &config::BotConfig,
    runtime_counter: Option<u64>,
    timeout_secs: u64,
) -> Result<HomebindSmokeFixture> {
    use mysql::prelude::Queryable;

    if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!(
            "refusing destructive homebind fixture setup for non-local account {}",
            bot.account
        );
    }

    let characters_url = characters_db_url()?;
    let character_opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut characters = mysql::Conn::new(character_opts)
        .map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let character: Option<(u32, u8, u8, u32, u32, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, race, map, zone, instance_id, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load homebind bot character: {e}"))?;
    let (owner, online, race, map_id, zone_id, instance_id, x, y, z, orientation) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; log it out before homebind smoke setup",
            bot.character_guid
        );
    }
    let original_position = CharacterPositionSnapshot {
        map_id,
        zone_id,
        instance_id,
        x,
        y,
        z,
        orientation,
    };
    let original_homebind = characters
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load original character_homebind: {e}"))?
        .map(|(map_id, zone_id, x, y, z, orientation)| HomebindRowSnapshot {
            map_id,
            zone_id,
            x,
            y,
            z,
            orientation,
        });

    let world_url = world_db_url()?;
    let world_opts =
        mysql::Opts::from_url(&world_url).map_err(|e| anyhow!("Bad world DB URL: {e}"))?;
    let mut world =
        mysql::Conn::new(world_opts).map_err(|e| anyhow!("Connect to world DB failed: {e}"))?;
    let preferred_faction = if matches!(race, 2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 35 | 36) {
        29u32
    } else {
        12u32
    };
    let innkeeper_row: (u64, u32, u32, f64, f64, f64, f32) = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.position_x, c.position_y, c.position_z, c.orientation \
             FROM creature c JOIN creature_template ct ON ct.entry = c.id \
             WHERE c.map IN (0, 1) \
               AND ct.faction = ? \
               AND ((IF(c.npcflag <> 0, c.npcflag, ct.npcflag) & ?) <> 0) \
               AND c.phaseid = 0 AND c.phasegroup = 0 \
               AND FIND_IN_SET('0', c.spawnDifficulties) > 0 \
               AND ct.VehicleId = 0 \
               AND NOT EXISTS (SELECT 1 FROM creature duplicate \
                               WHERE duplicate.map = c.map AND duplicate.id = c.id \
                                 AND duplicate.guid <> c.guid) \
             ORDER BY c.guid LIMIT 1",
            (preferred_faction, NPC_FLAG_INNKEEPER),
        )
        .map_err(|e| anyhow!("Resolve unique faction-friendly innkeeper: {e}"))?
        .ok_or_else(|| anyhow!("No unique faction-friendly continent innkeeper exists"))?;
    let (spawn_guid, entry, innkeeper_map, innkeeper_x, innkeeper_y, innkeeper_z, innkeeper_o) =
        innkeeper_row;
    let innkeeper_map = u16::try_from(innkeeper_map)
        .map_err(|_| anyhow!("innkeeper map id does not fit protocol: {innkeeper_map}"))?;
    let discover_runtime_guid = runtime_counter.is_none();
    // The placeholder is replaced from the login UpdateObject stream. An
    // explicit override remains useful for narrow packet captures.
    let guid_counter = runtime_counter.unwrap_or(spawn_guid);
    let (low, high) = create_creature_guid_raw(innkeeper_map, entry, guid_counter);
    let innkeeper = ResolvedCreatureTarget {
        entry,
        spawn_guid,
        guid_counter,
        map_id: innkeeper_map,
        x: innkeeper_x,
        y: innkeeper_y,
        z: innkeeper_z,
        orientation: innkeeper_o,
        packed_guid: build_packed_guid(low, high),
    };

    characters
        .exec_drop(
            "UPDATE characters SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
            (
                u32::from(innkeeper_map),
                innkeeper_x + 2.0,
                innkeeper_y,
                innkeeper_z,
                innkeeper_o,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Relocate homebind bot near innkeeper: {e}"))?;

    Ok(HomebindSmokeFixture {
        options: HomebindSmokeOptions {
            phase: HomebindSmokePhase::Bind,
            innkeeper,
            discover_runtime_guid,
            expected_homebind: None,
            timeout_secs,
        },
        original_position,
        original_homebind,
    })
}

fn load_homebind_row(bot: &config::BotConfig) -> Result<Option<HomebindRowSnapshot>> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let row: Option<(u16, u16, f32, f32, f32, f32)> = conn
        .exec_first(
            "SELECT mapId, zoneId, posX, posY, posZ, orientation FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load bound character_homebind: {e}"))?;
    Ok(row.map(
        |(map_id, zone_id, x, y, z, orientation)| HomebindRowSnapshot {
            map_id,
            zone_id,
            x,
            y,
            z,
            orientation,
        },
    ))
}

fn wait_for_homebind_row(
    bot: &config::BotConfig,
    expected: &HomebindRowSnapshot,
    timeout: Duration,
) -> Result<bool> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if load_homebind_row(bot)?.as_ref() == Some(expected) {
            return Ok(true);
        }
        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn cleanup_homebind_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &HomebindSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check homebind bot offline state before cleanup: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {} remained online during homebind cleanup",
                bot.character_guid
            ),
            None => bail!(
                "No characters row for guid {} during homebind cleanup",
                bot.character_guid
            ),
        }
    }

    let mut tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start homebind cleanup transaction: {e}"))?;
    tx.exec_drop(
        "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ?",
        (
            fixture.original_position.map_id,
            fixture.original_position.zone_id,
            fixture.original_position.instance_id,
            fixture.original_position.x,
            fixture.original_position.y,
            fixture.original_position.z,
            fixture.original_position.orientation,
            bot.character_guid,
        ),
    )
    .map_err(|e| anyhow!("Restore homebind bot position: {e}"))?;
    if let Some(homebind) = &fixture.original_homebind {
        tx.exec_drop(
            "INSERT INTO character_homebind (guid, mapId, zoneId, posX, posY, posZ, orientation) \
             VALUES (?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE mapId=VALUES(mapId), zoneId=VALUES(zoneId), posX=VALUES(posX), posY=VALUES(posY), posZ=VALUES(posZ), orientation=VALUES(orientation)",
            (
                bot.character_guid,
                homebind.map_id,
                homebind.zone_id,
                homebind.x,
                homebind.y,
                homebind.z,
                homebind.orientation,
            ),
        )
        .map_err(|e| anyhow!("Restore original character_homebind: {e}"))?;
    } else {
        tx.exec_drop(
            "DELETE FROM character_homebind WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Delete homebind fixture row: {e}"))?;
    }
    tx.commit()
        .map_err(|e| anyhow!("Commit homebind fixture cleanup: {e}"))?;
    Ok(())
}

fn verify_bank_fixture_location(
    bot: &config::BotConfig,
    item_guid: u64,
    expected_slot: u8,
) -> Result<bool> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    let row: Option<(u64, u8, u64, u64)> = conn
        .exec_first(
            "SELECT ci.bag, ci.slot, ii.owner_guid, ii.count \
             FROM character_inventory ci JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ci.guid = ? AND ci.item = ?",
            (bot.character_guid, item_guid),
        )
        .map_err(|e| anyhow!("Load bank fixture location: {e}"))?;
    Ok(matches!(
        row,
        Some((0, slot, owner, 1)) if slot == expected_slot && owner == bot.character_guid
    ))
}

fn cleanup_bank_smoke_fixture(bot: &config::BotConfig, fixture: &BankSmokeFixture) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {e}"))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    // A failed packet phase can drop the socket before the normal logout path.
    // Wait until the world server has finished its disconnect save before
    // deleting the fixture, otherwise that late save could recreate it or
    // overwrite the restored character position.
    let offline_deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|e| anyhow!("Check bank bot offline state before cleanup: {e}"))?;
        match online {
            Some(0) => break,
            Some(_) if std::time::Instant::now() < offline_deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => {
                bail!(
                    "character {} remained online; refusing bank fixture cleanup before disconnect save",
                    bot.character_guid
                );
            }
            None => bail!(
                "No characters row for guid {} during cleanup",
                bot.character_guid
            ),
        }
    }

    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|e| anyhow!("Start bank cleanup transaction: {e}"))?;
    transaction
        .exec_drop(
            "DELETE FROM character_inventory WHERE guid = ? AND item = ?",
            (bot.character_guid, fixture.options.item_guid),
        )
        .map_err(|e| anyhow!("Delete bank fixture inventory row: {e}"))?;
    transaction
        .exec_drop(
            "DELETE FROM item_instance WHERE guid = ? AND owner_guid = ?",
            (fixture.options.item_guid, bot.character_guid),
        )
        .map_err(|e| anyhow!("Delete bank fixture item: {e}"))?;
    transaction
        .exec_drop(
            "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                fixture.original_position.map_id,
                fixture.original_position.zone_id,
                fixture.original_position.instance_id,
                fixture.original_position.x,
                fixture.original_position.y,
                fixture.original_position.z,
                fixture.original_position.orientation,
                bot.character_guid,
            ),
        )
        .map_err(|e| anyhow!("Restore bank bot position: {e}"))?;
    transaction
        .commit()
        .map_err(|e| anyhow!("Commit bank fixture cleanup: {e}"))?;
    info!(
        "Bank smoke fixture cleaned: character={} item={}",
        bot.character_guid, fixture.options.item_guid
    );
    Ok(())
}

fn set_bot_character_level(conn: &mut mysql::Conn, character_guid: u64, level: u8) -> Result<()> {
    use mysql::prelude::Queryable;

    conn.exec_drop(
        "UPDATE characters SET level = ? WHERE guid = ?",
        (level, character_guid),
    )
    .map_err(|e| anyhow!("UPDATE characters.level: {}", e))?;
    Ok(())
}

fn set_bot_character_race_class(
    conn: &mut mysql::Conn,
    character_guid: u64,
    race: Option<u8>,
    class: Option<u8>,
) -> Result<()> {
    use mysql::prelude::Queryable;

    match (race, class) {
        (Some(race), Some(class)) => conn
            .exec_drop(
                "UPDATE characters SET race = ?, class = ? WHERE guid = ?",
                (race, class, character_guid),
            )
            .map_err(|e| anyhow!("UPDATE characters.race/class: {}", e))?,
        (Some(race), None) => conn
            .exec_drop(
                "UPDATE characters SET race = ? WHERE guid = ?",
                (race, character_guid),
            )
            .map_err(|e| anyhow!("UPDATE characters.race: {}", e))?,
        (None, Some(class)) => conn
            .exec_drop(
                "UPDATE characters SET class = ? WHERE guid = ?",
                (class, character_guid),
            )
            .map_err(|e| anyhow!("UPDATE characters.class: {}", e))?,
        (None, None) => {}
    }
    Ok(())
}

fn reset_bot_quest_state(conn: &mut mysql::Conn, character_guid: u64, quest_id: u32) -> Result<()> {
    use mysql::prelude::Queryable;

    conn.exec_drop(
        "DELETE FROM character_queststatus WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus: {}", e))?;
    conn.exec_drop(
        "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus_objectives: {}", e))?;
    conn.exec_drop(
        "DELETE FROM character_queststatus_rewarded WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus_rewarded: {}", e))?;
    Ok(())
}

fn seed_bot_quest_objective_state(
    conn: &mut mysql::Conn,
    character_guid: u64,
    quest_id: u32,
    status: u8,
    rows: &[QuestObjectiveDbRow],
) -> Result<()> {
    use mysql::prelude::Queryable;

    let accept_time = chrono::Utc::now().timestamp();
    conn.exec_drop(
        "REPLACE INTO character_queststatus \
         (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, 0, ?, 0)",
        (character_guid, quest_id, status, accept_time),
    )
    .map_err(|e| anyhow!("REPLACE character_queststatus: {}", e))?;
    conn.exec_drop(
        "DELETE FROM character_queststatus_objectives WHERE guid = ? AND quest = ?",
        (character_guid, quest_id),
    )
    .map_err(|e| anyhow!("DELETE character_queststatus_objectives: {}", e))?;

    for row in rows.iter().filter(|row| row.data != 0) {
        conn.exec_drop(
            "REPLACE INTO character_queststatus_objectives \
             (guid, quest, objective, data) VALUES (?, ?, ?, ?)",
            (character_guid, quest_id, row.objective, row.data),
        )
        .map_err(|e| anyhow!("REPLACE character_queststatus_objectives: {}", e))?;
    }
    Ok(())
}

fn load_bot_quest_objectives(
    bot: &config::BotConfig,
    quest_id: u32,
) -> Result<Vec<QuestObjectiveDbRow>> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;
    let mut rows: Vec<QuestObjectiveDbRow> = conn
        .exec_map(
            "SELECT objective, data FROM character_queststatus_objectives \
             WHERE guid = ? AND quest = ? ORDER BY objective",
            (bot.character_guid, quest_id),
            |(objective, data)| QuestObjectiveDbRow { objective, data },
        )
        .map_err(|e| anyhow!("SELECT character_queststatus_objectives: {}", e))?;
    rows.sort();
    Ok(rows)
}

fn verify_quest_accepted_in_db(
    bot: &config::BotConfig,
    quest_id: u32,
) -> Result<(bool, Option<u8>)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut last_status = None;
    loop {
        let row: Option<(u8,)> = conn
            .exec_first(
                "SELECT status FROM character_queststatus WHERE guid = ? AND quest = ?",
                (bot.character_guid, quest_id),
            )
            .map_err(|e| anyhow!("SELECT accepted quest status: {}", e))?;
        if let Some((status,)) = row {
            last_status = Some(status);
            if status != 0 {
                return Ok((true, last_status));
            }
        }

        if std::time::Instant::now() >= deadline {
            return Ok((false, last_status));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn database_url(env_name: &str, conf_key: &str) -> Result<String> {
    if let Ok(value) = std::env::var(env_name) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    database_url_from_worldserver_conf(conf_key).map_err(|e| {
        anyhow!(
            "{} is not set and {} could not be read from worldserver config: {}",
            env_name,
            conf_key,
            e
        )
    })
}

fn worldserver_config_f32(key: &str, default: f32) -> Result<f32> {
    let path = std::env::var("WOW_BOT_DB_CONF")
        .unwrap_or_else(|_| "/home/server/trinity-legacy-install/etc/worldserver.conf".to_string());
    let contents =
        std::fs::read_to_string(&path).map_err(|error| anyhow!("Read {path} failed: {error}"))?;
    worldserver_config_f32_from_contents(&contents, key, default)
        .with_context(|| format!("Read {key} from {path}"))
}

fn worldserver_config_u32(key: &str, default: u32) -> Result<u32> {
    let path = std::env::var("WOW_BOT_DB_CONF")
        .unwrap_or_else(|_| "/home/server/trinity-legacy-install/etc/worldserver.conf".to_string());
    let contents =
        std::fs::read_to_string(&path).map_err(|error| anyhow!("Read {path} failed: {error}"))?;
    worldserver_config_u32_from_contents(&contents, key, default)
        .with_context(|| format!("Read {key} from {path}"))
}

fn worldserver_config_u32_from_contents(contents: &str, key: &str, default: u32) -> Result<u32> {
    let mut effective = None;
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let Some((candidate_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if !candidate_key.trim().eq_ignore_ascii_case(key) {
            continue;
        }
        effective = Some(raw_value.trim().trim_matches('"'));
    }
    let Some(value) = effective else {
        return Ok(default);
    };
    value
        .parse::<u32>()
        .map_err(|error| anyhow!("invalid {key} value `{value}`: {error}"))
}

fn worldserver_config_f32_from_contents(contents: &str, key: &str, default: f32) -> Result<f32> {
    let mut effective = None;
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let Some((candidate_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if !candidate_key.trim().eq_ignore_ascii_case(key) {
            continue;
        }
        effective = Some(raw_value.trim().trim_matches('"'));
    }
    let Some(value) = effective else {
        return Ok(default);
    };
    value
        .parse::<f32>()
        .map_err(|error| anyhow!("invalid {key} value `{value}`: {error}"))
}

fn database_url_from_worldserver_conf(conf_key: &str) -> Result<String> {
    let path = std::env::var("WOW_BOT_DB_CONF")
        .unwrap_or_else(|_| "/home/server/trinity-legacy-install/etc/worldserver.conf".to_string());
    let contents =
        std::fs::read_to_string(&path).map_err(|e| anyhow!("Read {} failed: {}", path, e))?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(conf_key) {
            continue;
        }
        let value = trimmed
            .split_once('=')
            .map(|(_, v)| v.trim())
            .ok_or_else(|| anyhow!("Malformed {} line", conf_key))?;
        let value = value.trim_matches('"');
        let mut parts = value.split(';');
        let host = parts.next().ok_or_else(|| anyhow!("Missing DB host"))?;
        let port = parts.next().ok_or_else(|| anyhow!("Missing DB port"))?;
        let user = parts.next().ok_or_else(|| anyhow!("Missing DB user"))?;
        let password = parts.next().ok_or_else(|| anyhow!("Missing DB password"))?;
        let database = parts.next().ok_or_else(|| anyhow!("Missing DB name"))?;
        let mut url = String::from("mysql://");
        url.push_str(user);
        url.push(':');
        url.push_str(password);
        url.push('@');
        url.push_str(host);
        url.push(':');
        url.push_str(port);
        url.push('/');
        url.push_str(database);
        return Ok(url);
    }
    bail!("{} not found in {}", conf_key, path)
}

// ═════════════════════════════════════════════════════════════════════════════
// Session key plumbing (DB sync for worldserver)
// ═════════════════════════════════════════════════════════════════════════════

/// Expand 32-byte K (from SRP6) to the 64-byte session_key_bnet expected by the
/// worldserver: K || SHA256(K). Mirrors main_srp6_complete.rs::expand_session_key.
fn expand_session_key(k: &[u8]) -> [u8; 64] {
    use sha2::{Digest, Sha256};
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(k);
    out[32..].copy_from_slice(&Sha256::digest(k));
    out
}

/// Resolve the WoW account for a battle.net email, write `K_64` into
/// account.session_key_bnet, and load the same build auth seed the worldserver
/// uses for CMSG_AUTH_SESSION verification.
///
/// Uses synchronous mysql crate (one short transaction per bot); kept off the
/// runtime via spawn_blocking is unnecessary because run_bot is sequential.
fn prepare_world_auth_context(
    email: &str,
    session_key: &[u8],
    realm_id: u32,
) -> Result<WorldAuthDbContext> {
    use mysql::prelude::Queryable;
    let db_url = auth_db_url()?;
    let opts = mysql::Opts::from_url(&db_url).map_err(|e| anyhow!("Bad DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to auth DB failed: {}", e))?;

    let username: Option<String> = conn
        .exec_first(
            "SELECT a.username FROM account a \
             JOIN battlenet_accounts ba ON a.battlenet_account = ba.id \
             WHERE ba.email = ?",
            (email,),
        )
        .map_err(|e| anyhow!("Lookup username for {}: {}", email, e))?;
    let username = username.ok_or_else(|| anyhow!("No account for email {}", email))?;

    conn.exec_drop(
        "UPDATE account SET session_key_bnet = ? WHERE username = ?",
        (session_key, &username),
    )
    .map_err(|e| anyhow!("UPDATE session_key_bnet for {}: {}", username, e))?;

    let realm_build: Option<(u32,)> = conn
        .exec_first("SELECT gamebuild FROM realmlist WHERE id = ?", (realm_id,))
        .map_err(|e| anyhow!("Lookup realmlist.gamebuild for realm {}: {}", realm_id, e))?;
    let realm_build = realm_build
        .map(|(build,)| build)
        .ok_or_else(|| anyhow!("No realmlist row for realm {}", realm_id))?;

    let seed_row: Option<(Option<String>,)> = conn
        .exec_first(
            "SELECT win64AuthSeed FROM build_info WHERE build = ?",
            (realm_build,),
        )
        .map_err(|e| {
            anyhow!(
                "Lookup build_info.win64AuthSeed for build {}: {}",
                realm_build,
                e
            )
        })?;
    let seed_hex = seed_row
        .and_then(|(seed,)| seed)
        .ok_or_else(|| anyhow!("No win64AuthSeed for build {}", realm_build))?;
    let win64_auth_seed = parse_win64_auth_seed(&seed_hex, realm_build)?;

    Ok(WorldAuthDbContext {
        username,
        realm_build,
        win64_auth_seed,
    })
}

fn parse_win64_auth_seed(seed_hex: &str, build: u32) -> Result<[u8; 16]> {
    if seed_hex.len() != 32 {
        bail!(
            "Invalid win64AuthSeed for build {}: expected 32 hex chars, got {}",
            build,
            seed_hex.len()
        );
    }

    let mut seed = [0u8; 16];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&seed_hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| anyhow!("Invalid win64AuthSeed hex for build {}: {}", build, e))?;
    }
    Ok(seed)
}

// ═════════════════════════════════════════════════════════════════════════════
// Packet I/O helpers
// ═════════════════════════════════════════════════════════════════════════════

/// Read unencrypted packet (18-byte header: size + tag placeholder + opcode)
async fn read_unencrypted_packet(stream: &mut TcpStream) -> Result<(u16, Vec<u8>)> {
    let mut header = [0u8; 18];
    stream.read_exact(&mut header).await?;

    let size = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if size == 0 || size > 0x10000 {
        bail!("Invalid packet size: {}", size);
    }

    let payload_size = size.saturating_sub(2);
    let mut payload = vec![0u8; payload_size];
    if payload_size > 0 {
        stream.read_exact(&mut payload).await?;
    }

    let opcode = u16::from_le_bytes([header[16], header[17]]);
    Ok((opcode, payload))
}

/// Send unencrypted packet
async fn send_unencrypted_packet(stream: &mut TcpStream, opcode: u16, data: &[u8]) -> Result<()> {
    let mut packet = vec![0u8; 18];
    let size = (2 + data.len()) as u32;
    packet[0..4].copy_from_slice(&size.to_le_bytes());
    // bytes 4-15 are zeros (tag placeholder for unencrypted phase)
    packet[16..18].copy_from_slice(&opcode.to_le_bytes());

    stream.write_all(&packet).await?;
    if !data.is_empty() {
        stream.write_all(data).await?;
    }
    stream.flush().await?;
    Ok(())
}

/// Read encrypted packet (16-byte header: size + tag)
async fn read_encrypted_packet(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
) -> Result<(u16, Vec<u8>)> {
    let mut header = [0u8; 16];
    stream.read_exact(&mut header).await?;

    let size = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    const MAX_ENCRYPTED_PACKET_SIZE: usize = 8 * 1024 * 1024;
    if size == 0 || size > MAX_ENCRYPTED_PACKET_SIZE {
        bail!("Invalid encrypted packet size: {}", size);
    }

    let mut tag = [0u8; 12];
    tag.copy_from_slice(&header[4..16]);

    let mut ciphertext = vec![0u8; size];
    stream.read_exact(&mut ciphertext).await?;

    let plaintext = crypt
        .decrypt_server(&ciphertext, &tag, &[])
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    if plaintext.len() < 2 {
        bail!("Decrypted payload too short");
    }

    let opcode = u16::from_le_bytes([plaintext[0], plaintext[1]]);
    let payload = plaintext[2..].to_vec();
    if opcode == SMSG_COMPRESSED_PACKET {
        return decompress_server_packet_like_cpp(&payload, server_inflater);
    }
    Ok((opcode, payload))
}

fn decompress_server_packet_like_cpp(
    payload: &[u8],
    inflater: &mut ServerPacketInflater,
) -> Result<(u16, Vec<u8>)> {
    if payload.len() < 12 {
        bail!("Compressed packet too short: {} bytes", payload.len());
    }
    let uncompressed_size = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    if uncompressed_size < 2 {
        bail!("Invalid compressed packet size: {uncompressed_size}");
    }
    let uncompressed_size = uncompressed_size as usize;
    let deflated = &payload[12..];
    let mut output = vec![0u8; uncompressed_size + 256];
    let base_out = inflater.decompressor.total_out() as usize;

    inflater
        .decompressor
        .decompress(deflated, &mut output, FlushDecompress::Sync)
        .map_err(|e| anyhow!("Compressed packet inflate failed: {e}"))?;
    let produced = inflater.decompressor.total_out() as usize - base_out;
    if produced != uncompressed_size {
        bail!(
            "Compressed packet size mismatch: expected {}, got {}",
            uncompressed_size,
            produced
        );
    }
    output.truncate(produced);
    let opcode = u16::from_le_bytes([output[0], output[1]]);
    Ok((opcode, output[2..].to_vec()))
}

/// Send encrypted packet
async fn send_encrypted_packet(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    opcode: u16,
    data: &[u8],
) -> Result<()> {
    let mut plaintext = Vec::with_capacity(2 + data.len());
    plaintext.extend_from_slice(&opcode.to_le_bytes());
    plaintext.extend_from_slice(data);

    let (ciphertext, tag) = crypt
        .encrypt_client(&plaintext, &[])
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Build header: size + tag + encrypted_opcode_hint
    let mut header = [0u8; 18];
    let size = ciphertext.len() as u32;
    header[0..4].copy_from_slice(&size.to_le_bytes());
    header[4..16].copy_from_slice(&tag);
    if ciphertext.len() >= 2 {
        header[16..18].copy_from_slice(&ciphertext[0..2]);
    }

    stream.write_all(&header).await?;
    if ciphertext.len() > 2 {
        stream.write_all(&ciphertext[2..]).await?;
    }
    stream.flush().await?;
    Ok(())
}

// ═════════════════════════════════════════════════════════════════════════════
// Auth & crypto helpers
// ═════════════════════════════════════════════════════════════════════════════

/// Compute the 24-byte auth digest used in CMSG_AUTH_SESSION
fn compute_auth_digest(
    local: &[u8; 16],
    server: &[u8; 16],
    session_key: &[u8],
    auth_seed: &[u8; 16],
) -> [u8; 24] {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    type HmacSha256 = Hmac<sha2::Sha256>;

    // SHA256(session_key || build_info.win64AuthSeed)
    let mut hasher = Sha256::new();
    hasher.update(session_key);
    hasher.update(auth_seed);
    let digest_key = hasher.finalize();

    // HMAC(digest_key, local || server || AUTH_CHECK_SEED)
    let mut mac = HmacSha256::new_from_slice(&digest_key).unwrap();
    mac.update(local);
    mac.update(server);
    mac.update(&AUTH_CHECK_SEED);
    let result = mac.finalize().into_bytes();

    let mut digest = [0u8; 24];
    digest.copy_from_slice(&result[..24]);
    digest
}

/// Derive the 16-byte AES-GCM encryption key the same way TrinityCore's worldserver
/// does: HMAC the 64-byte session_key_bnet to a 32-byte seed, expand that seed to 40
/// bytes through the SessionKeyGenerator cascade, then HMAC again for the final key.
/// Falling back to a "32 bytes || 8 zero bytes" shortcut (the previous implementation)
/// produced an HMAC key that disagreed with the server, so the very first server
/// packet failed AES-GCM tag verification.
fn derive_encryption_key(session_key: &[u8], local: &[u8; 16], server: &[u8; 16]) -> [u8; 16] {
    let session_key_40 = derive_realm_session_key(session_key, local, server);
    srp6_auth::calculate_encrypt_key(&session_key_40, local, server)
}

fn derive_realm_session_key(session_key: &[u8], local: &[u8; 16], server: &[u8; 16]) -> [u8; 40] {
    let key_data: [u8; 64] = session_key
        .try_into()
        .expect("session_key must be 64 bytes");
    srp6_auth::calculate_session_key(&key_data, server, local)
}

fn compute_continued_auth_digest(
    key: i64,
    local: &[u8; 16],
    server: &[u8; 16],
    session_key: &[u8],
) -> [u8; 24] {
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(session_key).unwrap();
    mac.update(&key.to_le_bytes());
    mac.update(local);
    mac.update(server);
    mac.update(&CONTINUED_SESSION_SEED);
    let result = mac.finalize().into_bytes();

    let mut digest = [0u8; 24];
    digest.copy_from_slice(&result[..24]);
    digest
}

fn derive_instance_encryption_key(
    session_key: &[u8],
    local: &[u8; 16],
    server: &[u8; 16],
) -> [u8; 16] {
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(session_key).unwrap();
    mac.update(local);
    mac.update(server);
    mac.update(&ENCRYPTION_KEY_SEED);
    let result = mac.finalize().into_bytes();

    let mut key = [0u8; 16];
    key.copy_from_slice(&result[..16]);
    key
}

fn build_cmsg_auth_continued_session(
    connect_to_key: i64,
    local: &[u8; 16],
    digest: &[u8; 24],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(56);
    data.extend_from_slice(&0i64.to_le_bytes());
    data.extend_from_slice(&connect_to_key.to_le_bytes());
    data.extend_from_slice(local);
    data.extend_from_slice(digest);
    data
}

/// Build CMSG_AUTH_SESSION packet data
fn build_cmsg_auth_session(
    realm_id: u32,
    local: &[u8; 16],
    digest: &[u8; 24],
    ticket: &str,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(128);

    // dos_response (u64)
    data.extend_from_slice(&0u64.to_le_bytes());
    // region_id (u32)
    data.extend_from_slice(&0u32.to_le_bytes());
    // battlegroup_id (u32)
    data.extend_from_slice(&0u32.to_le_bytes());
    // realm_id (u32)
    data.extend_from_slice(&realm_id.to_le_bytes());
    // local_challenge (16 bytes)
    data.extend_from_slice(local);
    // digest (24 bytes)
    data.extend_from_slice(digest);
    // UseIPv6 flag (1 byte)
    data.push(0x00);
    // ticket (string with length prefix)
    let ticket_bytes = ticket.as_bytes();
    data.extend_from_slice(&(ticket_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(ticket_bytes);

    data
}

/// Build player login data (packed GUID + farClip)
fn build_player_login(guid: u64, realm_id: u32, far_clip: f32) -> Vec<u8> {
    let (low, high) = create_player_guid_raw(guid, realm_id);
    let (low_mask, low_bytes) = pack_u64(low);
    let (high_mask, high_bytes) = pack_u64(high);

    let mut data = Vec::with_capacity(2 + low_bytes.len() + high_bytes.len() + 4);
    data.push(low_mask);
    data.push(high_mask);
    data.extend_from_slice(&low_bytes);
    data.extend_from_slice(&high_bytes);
    data.extend_from_slice(&far_clip.to_le_bytes());

    data
}

fn create_player_guid_raw(guid: u64, realm_id: u32) -> (u64, u64) {
    // C++ ObjectGuid::Create<HighGuid::Player>(realmId, guid).
    let high = (2u64 << 58) | ((u64::from(realm_id) & 0xFFFF) << 42);
    (guid, high)
}

fn build_move_heartbeat_payload(
    player_low: u64,
    player_high: u64,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
) -> Vec<u8> {
    // C++ MovementInfo wire order, mirrored by wow-packet::MovementInfo::read.
    let mut data = build_packed_guid(player_low, player_high);
    data.extend_from_slice(&0u32.to_le_bytes()); // MovementFlags
    data.extend_from_slice(&0u32.to_le_bytes()); // MovementFlags2
    data.extend_from_slice(&0u32.to_le_bytes()); // MovementFlags3
    data.extend_from_slice(&0u32.to_le_bytes()); // client time; server uses its fallback clock
    data.extend_from_slice(&x.to_le_bytes());
    data.extend_from_slice(&y.to_le_bytes());
    data.extend_from_slice(&z.to_le_bytes());
    data.extend_from_slice(&orientation.to_le_bytes());
    data.extend_from_slice(&0f32.to_le_bytes()); // pitch
    data.extend_from_slice(&0f32.to_le_bytes()); // step-up start elevation
    data.extend_from_slice(&0u32.to_le_bytes()); // remove movement forces count
    data.extend_from_slice(&0u32.to_le_bytes()); // move index
    data.push(0); // no transport/fall/spline/inertia/advanced-flying bits
    data
}

fn build_move_init_active_mover_complete_payload(ticks: u32) -> [u8; 4] {
    // C++ WorldPackets::Movement::MoveInitActiveMoverComplete::Read reads one
    // little-endian uint32. Zero is valid for this non-transport fixture.
    ticks.to_le_bytes()
}

fn resolve_quest_runtime_counter(
    runtime_counter: Option<u64>,
    spawn_guid: u64,
    entry: u32,
) -> Result<u64> {
    runtime_counter.ok_or_else(|| {
        anyhow!(
            "Quest smoke for entry {entry} resolved world.creature guid {spawn_guid}, but needs the live ObjectGuid low counter. Set WOW_BOT_QUEST_RUNTIME_COUNTER or pass --quest-runtime-counter."
        )
    })
}

fn create_creature_guid_raw(map_id: u16, entry: u32, counter: u64) -> (u64, u64) {
    let high = (8u64 << 58) | ((map_id as u64 & 0x1FFF) << 29) | ((entry as u64 & 0x7F_FFFF) << 6);
    let low = counter & OBJECT_GUID_COUNTER_MASK;
    (low, high)
}

fn create_void_storage_creature_guid_raw(
    map_id: u16,
    entry: u32,
    counter: u64,
    runtime_realm_id: u16,
) -> (u64, u64) {
    let (low, high) = create_creature_guid_raw(map_id, entry, counter);
    (low, high | (u64::from(runtime_realm_id) << 42))
}

fn void_storage_runtime_realm_id() -> Result<u16> {
    let configured = std::env::var("WOW_BOT_VOID_STORAGE_RUNTIME_REALM_ID")
        .ok()
        .map(|value| value.parse::<u16>())
        .transpose()
        .map_err(|error| anyhow!("Invalid WOW_BOT_VOID_STORAGE_RUNTIME_REALM_ID: {error}"))?
        .unwrap_or_else(|| u16::try_from(realm_id()).unwrap_or(u16::MAX));
    if configured > 0x1FFF {
        bail!("void-storage runtime realm ID {configured} exceeds the 13-bit ObjectGuid field");
    }
    Ok(configured)
}

fn resolve_vendor_runtime_target(
    target: &ResolvedCreatureTarget,
    discovered: Option<DiscoveredCreatureGuid>,
) -> Result<DiscoveredCreatureGuid> {
    let candidate = discovered.ok_or_else(|| {
        if target.guid_counter == 0 {
            anyhow!(
                "vendor entry {} spawn {} was not discovered near its SQL position in login SMSG_UPDATE_OBJECT packets",
                target.entry,
                target.spawn_guid
            )
        } else {
            anyhow!(
                "vendor runtime counter {} was not discovered near SQL spawn {}; the override cannot be linked safely",
                target.guid_counter & OBJECT_GUID_COUNTER_MASK,
                target.spawn_guid
            )
        }
    })?;
    if target.guid_counter != 0 {
        let expected = create_creature_guid_raw(target.map_id, target.entry, target.guid_counter);
        if (candidate.low, candidate.high) != expected {
            bail!(
                "vendor runtime counter override {} did not match discovered counter {} for SQL spawn {}",
                expected.0,
                candidate.low & OBJECT_GUID_COUNTER_MASK,
                target.spawn_guid
            );
        }
    }
    Ok(candidate)
}

fn resolve_rested_xp_runtime_target(
    target: &ResolvedCreatureTarget,
    discovered: Option<DiscoveredCreatureGuid>,
) -> Result<DiscoveredCreatureGuid> {
    let candidate = discovered.ok_or_else(|| {
        if target.guid_counter == 0 {
            anyhow!(
                "target entry {} spawn {} was not discovered near its SQL position in login SMSG_UPDATE_OBJECT packets; restart the QA world and retry",
                target.entry,
                target.spawn_guid
            )
        } else {
            anyhow!(
                "runtime counter {} for target entry {} spawn {} was not discovered near that spawn's SQL position; the override cannot be linked safely",
                target.guid_counter & OBJECT_GUID_COUNTER_MASK,
                target.entry,
                target.spawn_guid
            )
        }
    })?;

    if target.guid_counter != 0 {
        let expected = create_creature_guid_raw(target.map_id, target.entry, target.guid_counter);
        if (candidate.low, candidate.high) != expected {
            bail!(
                "runtime counter override {} did not match discovered counter {} for SQL spawn {}",
                expected.0,
                candidate.low & OBJECT_GUID_COUNTER_MASK,
                target.spawn_guid
            );
        }
    }

    Ok(candidate)
}

fn find_creature_guid_in_update_object(
    payload: &[u8],
    map_id: u16,
    entry: u32,
) -> Option<(u64, u64)> {
    // CreateObject blocks start with UpdateType (1/2) followed by a packed
    // ObjectGuid. Scanning is intentional: blocks are variable-sized, while
    // the GUID's high fields make a false match for type/map/entry negligible.
    for offset in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[offset], 1 | 2) {
            continue;
        }
        let Some((_, low, high)) = parse_packed_guid(&payload[offset + 1..]) else {
            continue;
        };
        if ((high >> 58) & 0x3F) == 8
            && ((high >> 29) & 0x1FFF) == u64::from(map_id)
            && ((high >> 6) & 0x7F_FFFF) == u64::from(entry)
        {
            return Some((low, high));
        }
    }
    None
}

fn find_creature_guid_near_position_in_update_object(
    payload: &[u8],
    map_id: u16,
    entry: u32,
    expected_x: f32,
    expected_y: f32,
    expected_z: f32,
    max_distance: f32,
    expected_counter: Option<u64>,
) -> Option<DiscoveredCreatureGuid> {
    let mut nearest: Option<(f32, DiscoveredCreatureGuid)> = None;
    for offset in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[offset], 1 | 2) {
            continue;
        }
        let Some(guid_bytes) = payload.get(offset + 1..) else {
            continue;
        };
        let Some((guid_len, low, high)) = parse_packed_guid(guid_bytes) else {
            continue;
        };
        if ((high >> 58) & 0x3F) != 8
            || ((high >> 29) & 0x1FFF) != u64::from(map_id)
            || ((high >> 6) & 0x7F_FFFF) != u64::from(entry)
            || expected_counter.is_some_and(|expected| low != expected & OBJECT_GUID_COUNTER_MASK)
        {
            continue;
        }
        let Some(mut position) = offset.checked_add(1 + guid_len) else {
            continue;
        };
        if payload.get(position).copied() != Some(5) {
            continue;
        }
        position += 1;
        // C++ CreateObjectBits are 18 MSB-first bits, flushed to three bytes.
        let Some(next_position) = position.checked_add(3) else {
            continue;
        };
        position = next_position;
        let Some(mover_bytes) = payload.get(position..) else {
            continue;
        };
        let Some((mover_len, mover_low, mover_high)) = parse_packed_guid(mover_bytes) else {
            continue;
        };
        if (mover_low, mover_high) != (low, high) {
            continue;
        }
        let Some(next_position) = position.checked_add(mover_len + 12 + 4) else {
            continue;
        };
        position = next_position;
        let (Some(x), Some(y), Some(z)) = (
            read_f32_at(payload, position),
            read_f32_at(payload, position + 4),
            read_f32_at(payload, position + 8),
        ) else {
            continue;
        };
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            continue;
        }
        let distance =
            ((x - expected_x).powi(2) + (y - expected_y).powi(2) + (z - expected_z).powi(2)).sqrt();
        if distance > max_distance.max(0.0) {
            continue;
        }
        let candidate = DiscoveredCreatureGuid { low, high, x, y, z };
        if nearest
            .as_ref()
            .is_none_or(|(nearest_distance, _)| distance < *nearest_distance)
        {
            nearest = Some((distance, candidate));
        }
    }
    nearest.map(|(_, candidate)| candidate)
}

fn read_f32_at(data: &[u8], position: usize) -> Option<f32> {
    let bytes: [u8; 4] = data
        .get(position..position.checked_add(4)?)?
        .try_into()
        .ok()?;
    Some(f32::from_le_bytes(bytes))
}

fn parse_bind_point_update(
    payload: &[u8],
    expected_orientation: f32,
) -> Option<HomebindRowSnapshot> {
    if payload.len() != 20 {
        return None;
    }
    let x = f32::from_le_bytes(payload[0..4].try_into().ok()?);
    let y = f32::from_le_bytes(payload[4..8].try_into().ok()?);
    let z = f32::from_le_bytes(payload[8..12].try_into().ok()?);
    let map_id = u16::try_from(i32::from_le_bytes(payload[12..16].try_into().ok()?)).ok()?;
    let zone_id = u16::try_from(i32::from_le_bytes(payload[16..20].try_into().ok()?)).ok()?;
    Some(HomebindRowSnapshot {
        map_id,
        zone_id,
        x,
        y,
        z,
        orientation: expected_orientation,
    })
}

fn read_msb_bits(data: &[u8], bit_offset: usize, bit_count: usize) -> Option<u32> {
    if bit_count > 32 || bit_offset.checked_add(bit_count)? > data.len().checked_mul(8)? {
        return None;
    }
    let mut value = 0u32;
    for bit in bit_offset..bit_offset + bit_count {
        value = (value << 1) | u32::from((data[bit / 8] >> (7 - bit % 8)) & 1);
    }
    Some(value)
}

fn take_packed_guid(data: &[u8], position: &mut usize) -> Option<(u64, u64)> {
    let (consumed, low, high) = parse_packed_guid(data.get(*position..)?)?;
    *position = position.checked_add(consumed)?;
    Some((low, high))
}

fn take_u32(data: &[u8], position: &mut usize) -> Option<u32> {
    let bytes: [u8; 4] = data
        .get(*position..position.checked_add(4)?)?
        .try_into()
        .ok()?;
    *position = position.checked_add(4)?;
    Some(u32::from_le_bytes(bytes))
}

fn spell_go_matches_bind(
    payload: &[u8],
    expected_caster_low: u64,
    expected_caster_high: u64,
    expected_player_low: u64,
    expected_player_high: u64,
) -> bool {
    let mut position = 0usize;
    let Some(caster) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    let Some(caster_unit) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    if caster != (expected_caster_low, expected_caster_high) || caster_unit != caster {
        return false;
    }
    if take_packed_guid(payload, &mut position).is_none()
        || take_packed_guid(payload, &mut position).is_none()
        || take_u32(payload, &mut position) != Some(3286)
    {
        return false;
    }

    // Visual, CastFlags, CastFlagsEx, CastTime, trajectory, destination index,
    // immunities, heal prediction and empty prediction beacon GUID.
    if take_u32(payload, &mut position).is_none()
        || take_u32(payload, &mut position) != Some(0x0004_0101)
        || take_u32(payload, &mut position) != Some(0)
        || take_u32(payload, &mut position).is_none()
    {
        return false;
    }
    position = match position.checked_add(8 + 1 + 8 + 4 + 1) {
        Some(end) if end <= payload.len() => end,
        _ => return false,
    };
    let Some(beacon) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    if beacon != (0, 0) {
        return false;
    }

    let Some(counts) = payload.get(position..position.saturating_add(10)) else {
        return false;
    };
    if read_msb_bits(counts, 0, 16) != Some(1)
        || read_msb_bits(counts, 16, 16) != Some(0)
        || read_msb_bits(counts, 32, 16) != Some(0)
        || read_msb_bits(counts, 48, 9) != Some(0)
        || read_msb_bits(counts, 57, 1) != Some(0)
        || read_msb_bits(counts, 58, 16) != Some(0)
        || read_msb_bits(counts, 74, 1) != Some(0)
        || read_msb_bits(counts, 75, 1) != Some(0)
    {
        return false;
    }
    position += 10;

    let Some(target_header) = payload.get(position..position.saturating_add(5)) else {
        return false;
    };
    if read_msb_bits(target_header, 0, 28) != Some(0x2)
        || read_msb_bits(target_header, 28, 4) != Some(0)
        || read_msb_bits(target_header, 32, 7) != Some(0)
    {
        return false;
    }
    position += 5;
    let Some(target) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    let Some(item) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    let Some(hit_target) = take_packed_guid(payload, &mut position) else {
        return false;
    };
    target == (expected_player_low, expected_player_high) && hit_target == target && item == (0, 0)
}

fn homebind_spell_go_seen_after_packet(
    already_seen: bool,
    payload: &[u8],
    expected_caster_low: u64,
    expected_caster_high: u64,
    expected_player_low: u64,
    expected_player_high: u64,
) -> bool {
    already_seen
        || spell_go_matches_bind(
            payload,
            expected_caster_low,
            expected_caster_high,
            expected_player_low,
            expected_player_high,
        )
}

fn player_bound_matches(
    payload: &[u8],
    expected_low: u64,
    expected_high: u64,
    expected_area_id: u32,
) -> bool {
    let Some((consumed, low, high)) = parse_packed_guid(payload) else {
        return false;
    };
    payload
        .get(consumed..consumed + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        == Some(expected_area_id)
        && low == expected_low
        && high == expected_high
        && payload.len() == consumed + 4
}

fn build_packed_guid(low: u64, high: u64) -> Vec<u8> {
    let (low_mask, low_bytes) = pack_u64(low);
    let (high_mask, high_bytes) = pack_u64(high);
    let mut data = Vec::with_capacity(2 + low_bytes.len() + high_bytes.len());
    data.push(low_mask);
    data.push(high_mask);
    data.extend_from_slice(&low_bytes);
    data.extend_from_slice(&high_bytes);
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_result_requires_verified_relog_for_success() {
        let mut result = BotRunResult {
            world_auth: true,
            enum_characters: true,
            player_login_verified: true,
            loot_race_smoke: true,
            loot_race_smoke_passed: Some(true),
            ..BotRunResult::default()
        };

        assert!(!result.success(false, false, false));

        result.loot_race_relog_verified = true;
        assert!(result.success(false, false, false));
    }

    #[test]
    fn vendor_result_requires_verified_relog_for_success() {
        let mut result = BotRunResult {
            world_auth: true,
            enum_characters: true,
            player_login_verified: true,
            vendor_smoke: true,
            vendor_smoke_passed: Some(true),
            ..BotRunResult::default()
        };

        assert!(!result.success(false, false, false));
        result.vendor_relogin_verified = true;
        assert!(result.success(false, false, false));
    }

    fn equipment_set_test_options() -> EquipmentSetSmokeOptions {
        EquipmentSetSmokeOptions {
            phase: EquipmentSetSmokePhase::Save,
            set_type: 0,
            set_id: 7,
            set_name: "QA Equipment".to_string(),
            set_icon: "INV_Sword_01".to_string(),
            expected_guid: None,
            save_barrier: None,
            timeout_secs: 10,
        }
    }

    #[test]
    fn equipment_set_result_requires_db_and_fresh_relog_proof() {
        let mut result = BotRunResult {
            world_auth: true,
            enum_characters: true,
            player_login_verified: true,
            equipment_set_smoke: true,
            equipment_set_smoke_passed: Some(true),
            ..BotRunResult::default()
        };

        assert!(!result.success(false, false, false));
        result.equipment_set_db_persisted = true;
        assert!(!result.success(false, false, false));
        result.equipment_set_relogin_verified = true;
        assert!(result.success(false, false, false));
    }

    #[test]
    fn equipment_set_smoke_indices_stay_within_cpp_client_limit() {
        assert!(7 < MAX_EQUIPMENT_SET_INDEX_LIKE_CPP);
        assert!(8 < MAX_EQUIPMENT_SET_INDEX_LIKE_CPP);
    }

    #[test]
    fn equipment_set_fixture_max_query_pins_unsigned_wire_type() {
        assert!(SHARED_EQUIPMENT_SET_GUID_MAX_QUERY
            .starts_with("SELECT CAST(MAX(maxguid) AS UNSIGNED)"));
    }

    #[test]
    fn equipment_set_db_verifier_requires_one_row_in_the_expected_table() {
        let options = equipment_set_test_options();
        let equipment_row = expected_equipment_set_db_row(&options, 42);
        let transmog_row = expected_transmog_outfit_db_row(&options, 42);

        assert!(equipment_set_db_rows_match(
            &options,
            42,
            std::slice::from_ref(&equipment_row),
            &[],
        ));
        assert!(!equipment_set_db_rows_match(
            &options,
            42,
            &[equipment_row.clone(), equipment_row.clone()],
            &[],
        ));
        assert!(!equipment_set_db_rows_match(
            &options,
            42,
            &[],
            std::slice::from_ref(&transmog_row),
        ));
        let mut equipment_with_wrong_item = equipment_row;
        equipment_with_wrong_item.items[3] = 99;
        assert!(!equipment_set_db_rows_match(
            &options,
            42,
            std::slice::from_ref(&equipment_with_wrong_item),
            &[],
        ));

        let mut transmog_options = options.clone();
        transmog_options.set_type = 1;
        assert!(equipment_set_db_rows_match(
            &transmog_options,
            42,
            &[],
            std::slice::from_ref(&transmog_row),
        ));
        assert!(!equipment_set_db_rows_match(
            &transmog_options,
            42,
            &[],
            &[transmog_row.clone(), transmog_row.clone()],
        ));
        let mut transmog_with_wrong_appearance = transmog_row.clone();
        transmog_with_wrong_appearance.appearances[4] = 1;
        assert!(!equipment_set_db_rows_match(
            &transmog_options,
            42,
            &[],
            std::slice::from_ref(&transmog_with_wrong_appearance),
        ));
        let mut transmog_with_wrong_enchant = transmog_row;
        transmog_with_wrong_enchant.main_hand_enchant = 7;
        assert!(!equipment_set_db_rows_match(
            &transmog_options,
            42,
            &[],
            std::slice::from_ref(&transmog_with_wrong_enchant),
        ));
    }

    #[test]
    fn equipment_set_save_builder_and_load_parser_share_cpp_shape() {
        let options = equipment_set_test_options();
        let save = build_save_equipment_set_payload(&options).unwrap();
        let mut load = Vec::with_capacity(4 + save.len());
        load.extend_from_slice(&1_u32.to_le_bytes());
        load.extend_from_slice(&save);
        let guid = 0x0102_0304_0506_0708_u64;
        load[8..16].copy_from_slice(&guid.to_le_bytes());

        assert_eq!(
            parse_load_equipment_sets(&load).unwrap(),
            vec![EquipmentSetWire {
                set_type: 0,
                guid,
                set_id: 7,
                ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
                pieces: [[0; 16]; EQUIPMENT_SET_SLOTS_LIKE_CPP],
                appearances: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
                enchants: [0; 2],
                secondary_appearances_and_slots: [0; 4],
                assigned_spec_index: -1,
                set_name: "QA Equipment".to_string(),
                set_icon: "INV_Sword_01".to_string(),
            }]
        );

        let mut nonzero_fields = load.clone();
        let first_piece_offset = 4 + 4 + 8 + 4 + 4;
        nonzero_fields[first_piece_offset] = 1;
        let first_appearance_offset = first_piece_offset + 16;
        nonzero_fields[first_appearance_offset..first_appearance_offset + 4]
            .copy_from_slice(&2_i32.to_le_bytes());
        let first_enchant_offset = first_piece_offset + EQUIPMENT_SET_SLOTS_LIKE_CPP * (16 + 4);
        nonzero_fields[first_enchant_offset..first_enchant_offset + 4]
            .copy_from_slice(&3_i32.to_le_bytes());
        let first_secondary_offset = first_enchant_offset + 2 * 4;
        nonzero_fields[first_secondary_offset..first_secondary_offset + 4]
            .copy_from_slice(&4_i32.to_le_bytes());
        let parsed = parse_load_equipment_sets(&nonzero_fields).unwrap();
        assert_eq!(parsed[0].pieces[0][0], 1);
        assert_eq!(parsed[0].appearances[0], 2);
        assert_eq!(parsed[0].enchants[0], 3);
        assert_eq!(parsed[0].secondary_appearances_and_slots[0], 4);

        load.push(0);
        assert!(parse_load_equipment_sets(&load).is_err());
    }

    #[test]
    fn equipment_set_id_parser_requires_exact_cpp_payload() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&42_u64.to_le_bytes());
        payload.extend_from_slice(&1_i32.to_le_bytes());
        payload.extend_from_slice(&72_u32.to_le_bytes());
        assert_eq!(parse_equipment_set_id(&payload).unwrap(), (42, 1, 72));
        payload.push(0);
        assert!(parse_equipment_set_id(&payload).is_err());
    }

    #[test]
    fn equipment_set_id_validator_requires_instance_route() {
        let options = equipment_set_test_options();
        let mut payload = Vec::new();
        payload.extend_from_slice(&42_u64.to_le_bytes());
        payload.extend_from_slice(&options.set_type.to_le_bytes());
        payload.extend_from_slice(&options.set_id.to_le_bytes());

        assert_eq!(
            validate_equipment_set_id_response(false, &payload, &options).unwrap(),
            42
        );
        assert!(validate_equipment_set_id_response(true, &payload, &options).is_err());
    }

    fn vendor_inventory_fixture(has_bonus: u8, modifier_count: u8) -> (Vec<u8>, Vec<u8>) {
        let vendor_guid = build_packed_guid(0x1234, 0xF130_0000_485D_0001);
        let mut payload = vendor_guid.clone();
        payload.push(0); // VendorInventoryReason::None.
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes()); // Price.
        payload.extend_from_slice(&37i32.to_le_bytes()); // MUID.
        payload.extend_from_slice(&1i32.to_le_bytes()); // Item type.
        payload.extend_from_slice(&0i32.to_le_bytes()); // Durability.
        payload.extend_from_slice(&1i32.to_le_bytes()); // Stack count.
        payload.extend_from_slice(&(-1i32).to_le_bytes()); // Unlimited quantity.
        payload.extend_from_slice(&1642i32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes()); // Player condition failure.
        payload.push(0); // Vendor flags.
        payload.extend_from_slice(&30183i32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes()); // Random seed.
        payload.extend_from_slice(&0i32.to_le_bytes()); // Random property.
        payload.push(has_bonus);
        payload.push(modifier_count);
        (payload, vendor_guid)
    }

    #[test]
    fn vendor_inventory_parser_reads_cpp_plain_item_row_exactly() {
        let (payload, vendor_guid) = vendor_inventory_fixture(0, 0);
        let items = parse_vendor_inventory(&payload, &vendor_guid).unwrap();

        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.muid, 37);
        assert_eq!(item.item_id, 30183);
        assert_eq!(item.item_type, 1);
        assert_eq!(item.price, 0);
        assert_eq!(item.stack_count, 1);
        assert_eq!(item.extended_cost, 1642);
    }

    #[test]
    fn vendor_inventory_parser_fails_closed_on_unimplemented_item_instance_shapes() {
        let (bonus_payload, vendor_guid) = vendor_inventory_fixture(1, 0);
        assert!(parse_vendor_inventory(&bonus_payload, &vendor_guid).is_err());

        let (modifier_payload, vendor_guid) = vendor_inventory_fixture(0, 1);
        assert!(parse_vendor_inventory(&modifier_payload, &vendor_guid).is_err());

        let (mut trailing_payload, vendor_guid) = vendor_inventory_fixture(0, 0);
        trailing_payload.push(0);
        assert!(parse_vendor_inventory(&trailing_payload, &vendor_guid).is_err());
    }

    #[test]
    fn vendor_buy_payload_uses_cpp_field_order_and_wire_item_instance() {
        let vendor_guid = build_packed_guid(0x1234, 0xF130_0000_485D_0001);
        let payload = build_vendor_buy_item_payload(&vendor_guid, 15, 37, 30183);
        assert!(payload.starts_with(&vendor_guid));

        let mut cursor = vendor_guid.len();
        let (player_guid_len, player_low, player_high) =
            parse_packed_guid(&payload[cursor..]).unwrap();
        let expected_player = create_player_guid_raw(15, realm_id());
        assert_eq!((player_low, player_high), expected_player);
        cursor += player_guid_len;

        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 1);
        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 37);
        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 255);
        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 1);
        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 30183);
        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 0);
        assert_eq!(take_vendor_i32(&payload, &mut cursor).unwrap(), 0);
        assert_eq!(&payload[cursor..], &[0, 0]);
    }

    #[test]
    fn vendor_buy_succeeded_parser_requires_exact_cpp_fields_and_no_tail() {
        let vendor_guid = build_packed_guid(0x1234, 0xF130_0000_485D_0001);
        let mut payload = vendor_guid.clone();
        payload.extend_from_slice(&59u32.to_le_bytes());
        payload.extend_from_slice(&(-1i32).to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());
        assert!(parse_vendor_buy_succeeded(&payload, &vendor_guid, 59, 1, -1).is_ok());

        let mut wrong_quantity = payload.clone();
        let quantity_offset = wrong_quantity.len() - 4;
        wrong_quantity[quantity_offset..].copy_from_slice(&2u32.to_le_bytes());
        assert!(parse_vendor_buy_succeeded(&wrong_quantity, &vendor_guid, 59, 1, -1).is_err());

        let mut trailing = payload;
        trailing.push(0);
        assert!(parse_vendor_buy_succeeded(&trailing, &vendor_guid, 59, 1, -1).is_err());
    }

    #[test]
    fn vendor_item_push_validator_requires_exact_cpp_purchase_shape() {
        let realm = 1;
        let character_guid = 15;
        let (player_low, player_high) = create_player_guid_raw(character_guid, realm);
        let item_high = (3u64 << 58) | (u64::from(realm) << 42);
        let mut payload = build_packed_guid(player_low, player_high);
        payload.push(INVENTORY_SLOT_BAG_0);
        payload.extend_from_slice(&i32::from(INVENTORY_SLOT_ITEM_START).to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes()); // QuestLogItemID.
        payload.extend_from_slice(&1i32.to_le_bytes()); // Quantity.
        payload.extend_from_slice(&1i32.to_le_bytes()); // QuantityInInventory.
        payload.extend_from_slice(&0i32.to_le_bytes()); // DungeonEncounterID.
        payload.extend_from_slice(&[0; 16]); // Battle-pet fields.
        payload.extend(build_packed_guid(500, item_high));
        payload.push(0x88); // Pushed + normal display, not created.
        payload.extend_from_slice(&30183i32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes()); // Random seed.
        payload.extend_from_slice(&0i32.to_le_bytes()); // Random property.
        payload.push(0); // No ItemBonus.
        payload.push(0); // No modifiers.

        assert!(loot_race::validate_vendor_item_push_result_like_cpp(
            &payload,
            character_guid,
            30183,
            1,
            realm,
        )
        .is_ok());
        payload.push(0);
        assert!(loot_race::validate_vendor_item_push_result_like_cpp(
            &payload,
            character_guid,
            30183,
            1,
            realm,
        )
        .is_err());
    }

    #[test]
    fn set_currency_parser_rejects_negative_wire_values() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&42i32.to_le_bytes());
        payload.extend_from_slice(&15i32.to_le_bytes());
        assert_eq!(parse_set_currency_identity(&payload).unwrap(), (42, 15));

        payload[4..8].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(parse_set_currency_identity(&payload).is_err());
    }

    #[test]
    fn login_verify_budget_is_time_based_not_packet_count() {
        let budget = LoginVerifyBudget::new(Duration::from_secs(1));

        // The former `for _ in 0..30` guard disconnected a second concurrent
        // client after 30 fast CREATE/broadcast packets. Observing packets must
        // not consume the wall-clock budget.
        for _ in 0..64 {
            assert!(budget.next_read_timeout().is_some());
        }

        assert_eq!(
            LoginVerifyBudget::new(Duration::ZERO).next_read_timeout(),
            None
        );
    }

    fn pack_msb_fields(fields: &[(u32, usize)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut current = 0u8;
        let mut used = 0usize;
        for &(value, width) in fields {
            for bit in (0..width).rev() {
                current |= (((value >> bit) & 1) as u8) << (7 - used);
                used += 1;
                if used == 8 {
                    bytes.push(current);
                    current = 0;
                    used = 0;
                }
            }
        }
        if used != 0 {
            bytes.push(current);
        }
        bytes
    }

    fn bind_spell_go_fixture(
        caster_low: u64,
        caster_high: u64,
        player_low: u64,
        player_high: u64,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend(build_packed_guid(caster_low, caster_high));
        payload.extend(build_packed_guid(caster_low, caster_high));
        payload.extend(build_packed_guid(1, 0));
        payload.extend(build_packed_guid(1, 0));
        payload.extend_from_slice(&3286u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0x0004_0101u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&1234u32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes());
        payload.extend_from_slice(&0f32.to_le_bytes());
        payload.push(0);
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.push(0);
        payload.extend(build_packed_guid(0, 0));
        payload.extend(pack_msb_fields(&[
            (1, 16),
            (0, 16),
            (0, 16),
            (0, 9),
            (0, 1),
            (0, 16),
            (0, 1),
            (0, 1),
        ]));
        payload.extend(pack_msb_fields(&[(0x2, 28), (0, 4), (0, 7)]));
        payload.extend(build_packed_guid(player_low, player_high));
        payload.extend(build_packed_guid(0, 0));
        payload.extend(build_packed_guid(player_low, player_high));
        payload.push(0);
        payload
    }

    fn rested_xp_create_object_fixture(
        map_id: u16,
        entry: u32,
        counter: u64,
        x: f32,
        y: f32,
        z: f32,
    ) -> Vec<u8> {
        let (low, high) = create_creature_guid_raw(map_id, entry, counter);
        let mut payload = vec![1]; // CreateObject1
        payload.extend(build_packed_guid(low, high));
        payload.push(5); // TypeId::Unit
        payload.extend_from_slice(&[0x10, 0, 0]); // MovementUpdate bit 3, MSB-first.
        payload.extend(build_packed_guid(low, high)); // movement MoverGUID
        payload.extend_from_slice(&[0; 12]); // movement flags
        payload.extend_from_slice(&123u32.to_le_bytes()); // MoveTime
        payload.extend_from_slice(&x.to_le_bytes());
        payload.extend_from_slice(&y.to_le_bytes());
        payload.extend_from_slice(&z.to_le_bytes());
        payload
    }

    fn rested_xp_target_fixture(guid_counter: u64) -> ResolvedCreatureTarget {
        let (low, high) = create_creature_guid_raw(530, 15_274, guid_counter);
        ResolvedCreatureTarget {
            entry: 15_274,
            spawn_guid: 54_931,
            guid_counter,
            map_id: 530,
            x: 10_187.8,
            y: -6_347.56,
            z: 30.459,
            orientation: 0.0,
            packed_guid: build_packed_guid(low, high),
        }
    }

    #[test]
    fn auto_bank_payload_uses_cpp_inv_update_then_source_position() {
        assert_eq!(build_auto_bank_item_payload(35), [0x40, 255, 35, 255, 35]);
        assert_eq!(build_auto_bank_item_payload(59), [0x40, 255, 59, 255, 59]);
    }

    #[test]
    fn inventory_swap_payload_matches_real_cpp_client_layout() {
        assert_eq!(
            build_swap_inv_item_payload(36, 40),
            [0x80, 255, 40, 255, 36, 40, 36]
        );
    }

    #[test]
    fn stand_state_change_uses_cpp_uint32_wire_layout() {
        assert_eq!(build_stand_state_change(UNIT_STAND_STATE_SIT), [1, 0, 0, 0]);
        assert_eq!(
            build_stand_state_change(UNIT_STAND_STATE_STAND),
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn stand_state_update_requires_exact_cpp_wire_layout() {
        assert!(validate_stand_state_update(&[0, 0, 0, 0, UNIT_STAND_STATE_SIT], 1).is_ok());

        let short = validate_stand_state_update(&[0, 0, 0, 0], UNIT_STAND_STATE_STAND)
            .expect_err("four-byte response must fail");
        assert!(short.to_string().contains("expected 5"));

        let wrong_anim = validate_stand_state_update(
            &[1, 0, 0, 0, UNIT_STAND_STATE_STAND],
            UNIT_STAND_STATE_STAND,
        )
        .expect_err("nonzero AnimKitID must fail");
        assert!(wrong_anim.to_string().contains("AnimKitID"));

        let wrong_state = validate_stand_state_update(
            &[0, 0, 0, 0, UNIT_STAND_STATE_STAND],
            UNIT_STAND_STATE_SIT,
        )
        .expect_err("unexpected state must fail");
        assert!(wrong_state.to_string().contains("state mismatch"));
    }

    #[test]
    fn stand_state_smoke_only_accepts_states_allowed_by_cpp_handler() {
        for state in [
            UNIT_STAND_STATE_STAND,
            UNIT_STAND_STATE_SIT,
            UNIT_STAND_STATE_SLEEP,
            UNIT_STAND_STATE_KNEEL,
        ] {
            assert!(is_client_stand_state_like_cpp(state));
        }
        assert!(!is_client_stand_state_like_cpp(7));
        assert!(!is_client_stand_state_like_cpp(9));
    }

    #[test]
    fn stand_state_smoke_requires_distinct_realm_and_instance_sockets() {
        assert!(validate_stand_state_socket_topology(true).is_ok());
        let error = validate_stand_state_socket_topology(false).unwrap_err();
        assert!(error
            .to_string()
            .contains("distinct realm/instance sockets"));
    }

    #[test]
    fn stand_state_quiet_drain_ignores_only_periodic_world_traffic() {
        assert!(stand_state_quiet_drain_ambient_opcode(SMSG_ON_MONSTER_MOVE));
        assert!(stand_state_quiet_drain_ambient_opcode(
            SMSG_TIME_SYNC_REQUEST
        ));
        assert!(!stand_state_quiet_drain_ambient_opcode(SMSG_UPDATE_OBJECT));
        assert!(!stand_state_quiet_drain_ambient_opcode(SMSG_AURA_UPDATE));
        assert!(!stand_state_quiet_drain_ambient_opcode(
            SMSG_STAND_STATE_UPDATE
        ));
    }

    #[test]
    fn player_login_guid_uses_configured_realm() {
        let guid = 0x1234;
        let realm_id = 7;
        let far_clip = 500.0f32;
        let mut expected = Vec::new();
        let (low_mask, low_bytes) = pack_u64(guid);
        let high = (2u64 << 58) | ((u64::from(realm_id) & 0x1FFF) << 42);
        let (high_mask, high_bytes) = pack_u64(high);
        expected.push(low_mask);
        expected.push(high_mask);
        expected.extend_from_slice(&low_bytes);
        expected.extend_from_slice(&high_bytes);
        expected.extend_from_slice(&far_clip.to_le_bytes());

        assert_eq!(build_player_login(guid, realm_id, far_clip), expected);
    }

    #[test]
    fn quest_smoke_requires_live_runtime_counter() {
        let error = resolve_quest_runtime_counter(None, 12_345, 15_513).unwrap_err();

        assert!(error.to_string().contains("WOW_BOT_QUEST_RUNTIME_COUNTER"));
    }

    #[test]
    fn legacy_creature_guid_constructor_keeps_counter_map_and_entry_fields() {
        let (low, high) = create_creature_guid_raw(571, 15_513, 77_001);

        assert_eq!(low, 77_001);
        assert_eq!((high >> 58) & 0x3F, 8);
        assert_eq!((high >> 42) & 0x1FFF, 0);
        assert_eq!((high >> 29) & 0x1FFF, 571);
        assert_eq!((high >> 6) & 0x7F_FFFF, 15_513);
    }

    #[test]
    fn rested_xp_move_heartbeat_uses_live_position_and_cpp_movement_layout() {
        let (player_low, player_high) = create_player_guid_raw(14, 1);
        let target = DiscoveredCreatureGuid {
            low: 77_001,
            high: create_creature_guid_raw(530, 15_274, 77_001).1,
            x: 10_188.0,
            y: -6_347.5,
            z: 30.5,
        };
        let player_x = target.x + 1.0;
        let player_y = target.y;
        let player_z = target.z;
        let orientation = (target.y - player_y).atan2(target.x - player_x);
        let payload = build_move_heartbeat_payload(
            player_low,
            player_high,
            player_x,
            player_y,
            player_z,
            orientation,
        );

        let (guid_len, low, high) = parse_packed_guid(&payload).expect("packed player GUID");
        assert_eq!((low, high), (player_low, player_high));
        let movement = &payload[guid_len..];
        assert_eq!(movement.len(), 49);
        assert_eq!(&movement[..16], &[0; 16]);
        assert_eq!(read_f32_at(movement, 16), Some(player_x));
        assert_eq!(read_f32_at(movement, 20), Some(player_y));
        assert_eq!(read_f32_at(movement, 24), Some(player_z));
        assert_eq!(read_f32_at(movement, 28), Some(orientation));
        assert_eq!(&movement[32..48], &[0; 16]);
        assert_eq!(movement[48], 0);

        let distance = ((target.x - player_x).powi(2)
            + (target.y - player_y).powi(2)
            + (target.z - player_z).powi(2))
        .sqrt();
        assert!(distance < NOMINAL_MELEE_RANGE_LIKE_CPP);
        assert!((orientation - std::f32::consts::PI).abs() < f32::EPSILON);
    }

    #[test]
    fn rested_xp_active_mover_ack_matches_cpp_wire_layout() {
        assert_eq!(CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, 0x3A46);
        assert_eq!(
            build_move_init_active_mover_complete_payload(0x1234_5678),
            [0x78, 0x56, 0x34, 0x12]
        );
    }

    #[test]
    fn homebind_smoke_discovers_loaded_creature_guid_from_update_object() {
        let (low, high) = create_creature_guid_raw(1, 12_196, 733);
        let mut payload = vec![0, 0, 0, 0, 1, 0, 0, 0];
        payload.push(1);
        payload.extend(build_packed_guid(low, high));
        payload.push(5);

        assert_eq!(
            find_creature_guid_in_update_object(&payload, 1, 12_196),
            Some((low, high))
        );
        assert_eq!(
            find_creature_guid_in_update_object(&payload, 1, 12_197),
            None
        );
    }

    #[test]
    fn homebind_smoke_decodes_complete_bind_point_update() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&1.25f32.to_le_bytes());
        payload.extend_from_slice(&(-2.5f32).to_le_bytes());
        payload.extend_from_slice(&3.75f32.to_le_bytes());
        payload.extend_from_slice(&571i32.to_le_bytes());
        payload.extend_from_slice(&4395i32.to_le_bytes());

        assert_eq!(
            parse_bind_point_update(&payload, 4.5),
            Some(HomebindRowSnapshot {
                map_id: 571,
                zone_id: 4395,
                x: 1.25,
                y: -2.5,
                z: 3.75,
                orientation: 4.5,
            })
        );
        assert_eq!(parse_bind_point_update(&payload[..19], 4.5), None);
    }

    #[test]
    fn homebind_smoke_validates_exact_player_bound_payload() {
        let (low, high) = create_creature_guid_raw(1, 12_196, 733);
        let mut payload = build_packed_guid(low, high);
        payload.extend_from_slice(&3430u32.to_le_bytes());

        assert!(player_bound_matches(&payload, low, high, 3430));
        assert!(!player_bound_matches(&payload, low, high, 3431));
        assert!(!player_bound_matches(&payload, low + 1, high, 3430));

        payload.push(0);
        assert!(!player_bound_matches(&payload, low, high, 3430));
    }

    #[test]
    fn homebind_smoke_requires_exact_bind_spell_go() {
        let (caster_low, caster_high) = create_creature_guid_raw(1, 12_196, 733);
        let player_low = 99;
        let player_high = (2u64 << 58) | (1u64 << 42);
        let mut payload = bind_spell_go_fixture(caster_low, caster_high, player_low, player_high);

        assert!(spell_go_matches_bind(
            &payload,
            caster_low,
            caster_high,
            player_low,
            player_high,
        ));
        assert!(!spell_go_matches_bind(
            &payload,
            caster_low + 1,
            caster_high,
            player_low,
            player_high,
        ));

        let spell_offset = 2 * build_packed_guid(caster_low, caster_high).len()
            + 2 * build_packed_guid(1, 0).len();
        payload[spell_offset..spell_offset + 4].copy_from_slice(&1u32.to_le_bytes());
        assert!(!spell_go_matches_bind(
            &payload,
            caster_low,
            caster_high,
            player_low,
            player_high,
        ));
    }

    #[test]
    fn homebind_smoke_keeps_match_after_later_unrelated_spell_go() {
        let (caster_low, caster_high) = create_creature_guid_raw(1, 12_196, 733);
        let player_low = 99;
        let player_high = (2u64 << 58) | (1u64 << 42);
        let matching = bind_spell_go_fixture(caster_low, caster_high, player_low, player_high);
        let mut unrelated = matching.clone();
        let spell_offset = 2 * build_packed_guid(caster_low, caster_high).len()
            + 2 * build_packed_guid(1, 0).len();
        unrelated[spell_offset..spell_offset + 4].copy_from_slice(&1u32.to_le_bytes());

        let seen = homebind_spell_go_seen_after_packet(
            false,
            &matching,
            caster_low,
            caster_high,
            player_low,
            player_high,
        );
        assert!(homebind_spell_go_seen_after_packet(
            seen,
            &unrelated,
            caster_low,
            caster_high,
            player_low,
            player_high,
        ));
    }

    #[test]
    fn rested_xp_smoke_cli_validation_is_fail_closed_only_when_enabled() {
        const NOW: u64 = 2_000_000_000;
        assert!(validate_rested_xp_cli_values(false, false, 0, 0, 0, 0, NOW).is_ok());
        assert!(validate_rested_xp_cli_values(true, true, 1, 15_274, 86_400, 45, NOW).is_ok());

        let stray_ack = validate_rested_xp_cli_values(false, true, 0, 0, 0, 0, NOW)
            .expect_err("the destructive ACK must not apply to other modes");
        assert!(stray_ack.to_string().contains("only valid"));

        let missing_ack = validate_rested_xp_cli_values(true, false, 1, 15_274, 86_400, 45, NOW)
            .expect_err("rested-XP smoke must require an explicit destructive ACK");
        assert!(missing_ack
            .to_string()
            .contains(ACK_DISPOSABLE_RESTED_XP_FLAG));

        let bot_count = validate_rested_xp_cli_values(true, true, 2, 15_274, 86_400, 45, NOW)
            .expect_err("multiple bots must be rejected");
        assert!(bot_count.to_string().contains("exactly one bot"));

        let entry = validate_rested_xp_cli_values(true, true, 1, 0, 86_400, 45, NOW)
            .expect_err("zero creature entry must be rejected");
        assert!(entry.to_string().contains("must be nonzero"));

        let offline = validate_rested_xp_cli_values(true, true, 1, 15_274, 0, 45, NOW)
            .expect_err("zero offline duration must be rejected");
        assert!(offline.to_string().contains("greater than zero"));

        let overflow = validate_rested_xp_cli_values(
            true,
            true,
            1,
            15_274,
            u64::from(u32::MAX) + 1,
            45,
            u64::MAX,
        )
        .expect_err("legacy C++ cannot represent a wider offline interval");
        assert!(overflow.to_string().contains("uint32"));

        let current_or_future = validate_rested_xp_cli_values(true, true, 1, 15_274, NOW, 45, NOW)
            .expect_err("logout_time=0/future fixtures must be rejected");
        assert!(current_or_future.to_string().contains("Unix timestamp"));

        let timeout = validate_rested_xp_cli_values(true, true, 1, 15_274, 86_400, 0, NOW)
            .expect_err("zero timeout must be rejected");
        assert!(timeout.to_string().contains("greater than zero"));
    }

    #[test]
    fn rested_xp_destructive_ack_parser_accepts_only_the_exact_cli_flag() {
        let mut acknowledged = false;
        assert!(!parse_ack_disposable_rested_xp_arg(
            "--ack-disposable-rested-xp=true",
            &mut acknowledged,
        ));
        assert!(!acknowledged);
        assert!(parse_ack_disposable_rested_xp_arg(
            ACK_DISPOSABLE_RESTED_XP_FLAG,
            &mut acknowledged,
        ));
        assert!(acknowledged);
    }

    #[test]
    fn rested_xp_fixture_safety_requires_exclusive_clean_disposable_scope() {
        let safe = RestedXpFixtureSafetyState {
            bnet_email_matches_configured_account: true,
            characters_on_game_account: 1,
            game_accounts_on_bnet_account: 1,
            ..RestedXpFixtureSafetyState::default()
        };
        assert!(validate_rested_xp_fixture_safety_state(&safe).is_ok());

        let mut at_login = RestedXpFixtureSafetyState {
            bnet_email_matches_configured_account: true,
            characters_on_game_account: 1,
            game_accounts_on_bnet_account: 1,
            at_login: 0x20,
            ..RestedXpFixtureSafetyState::default()
        };
        let error = validate_rested_xp_fixture_safety_state(&at_login)
            .expect_err("first-login state must be rejected");
        assert!(error.to_string().contains("at_login"));

        at_login.at_login = 0;
        at_login.characters_on_game_account = 2;
        let error = validate_rested_xp_fixture_safety_state(&at_login)
            .expect_err("shared game accounts must be rejected");
        assert!(error.to_string().contains("exactly one character"));

        let dirty = RestedXpFixtureSafetyState {
            bnet_email_matches_configured_account: true,
            characters_on_game_account: 1,
            game_accounts_on_bnet_account: 1,
            nonempty_side_state: vec![("character_inventory".to_string(), 3)],
            ..RestedXpFixtureSafetyState::default()
        };
        let error = validate_rested_xp_fixture_safety_state(&dirty)
            .expect_err("non-restored side tables must be rejected");
        assert!(error.to_string().contains("character_inventory=3"));

        let crossed_identity = RestedXpFixtureSafetyState {
            characters_on_game_account: 1,
            game_accounts_on_bnet_account: 1,
            ..RestedXpFixtureSafetyState::default()
        };
        let error = validate_rested_xp_fixture_safety_state(&crossed_identity)
            .expect_err("a configured bot email must own the selected game account");
        assert!(error.to_string().contains("configured @bot.local"));
    }

    #[test]
    fn rested_xp_cleanup_covers_cpp_generated_character_rows() {
        let labels: Vec<_> = RESTED_XP_CPP_GENERATED_CHARACTER_ROWS
            .iter()
            .map(|(label, _, _)| *label)
            .collect();
        assert_eq!(
            labels,
            [
                "character_glyphs",
                "character_reputation",
                "character_skills"
            ]
        );

        for (label, delete_sql, count_sql) in RESTED_XP_CPP_GENERATED_CHARACTER_ROWS {
            assert!(
                delete_sql.starts_with(&format!("DELETE FROM {label} ")),
                "cleanup for {label} must delete only from its own table"
            );
            assert!(
                delete_sql.ends_with("WHERE guid = ?"),
                "cleanup for {label} must remain scoped to the fixture character"
            );
            assert!(
                count_sql.starts_with(&format!("SELECT COUNT(*) FROM {label} ")),
                "verification for {label} must query the same table"
            );
            assert!(
                count_sql.ends_with("WHERE guid = ?"),
                "verification for {label} must remain scoped to the fixture character"
            );
        }

        assert!(
            RESTED_XP_SELECT_TRAIT_CONFIGS_SQL.contains("WHERE guid = ?"),
            "trait config snapshot must remain scoped to the fixture character"
        );
        assert!(
            RESTED_XP_SELECT_TRAIT_CONFIGS_SQL.ends_with("ORDER BY traitConfigId"),
            "trait config snapshot order must be deterministic"
        );
        assert!(
            RESTED_XP_SELECT_TRAIT_ENTRIES_SQL.contains("WHERE guid = ?"),
            "trait entry snapshot must remain scoped to the fixture character"
        );
        assert!(
            RESTED_XP_SELECT_TRAIT_ENTRIES_SQL
                .ends_with("ORDER BY traitConfigId, traitNodeId, traitNodeEntryId"),
            "trait entry snapshot order must be deterministic"
        );
    }

    #[test]
    fn rested_xp_respawn_cleanup_wait_covers_the_selected_spawn_timer() {
        assert_eq!(rested_xp_respawn_cleanup_wait_secs(120, 300), 315);
        assert_eq!(rested_xp_respawn_cleanup_wait_secs(180, 30), 180);
        assert_eq!(rested_xp_respawn_cleanup_wait_secs(1, 1), 16);
        assert_eq!(
            rested_xp_observed_respawn_remaining_secs(1_360, 1_000).unwrap(),
            375
        );
        assert_eq!(
            rested_xp_observed_respawn_remaining_secs(900, 1_000).unwrap(),
            15
        );
        assert!(rested_xp_observed_respawn_remaining_secs(2_000, 1_000).is_err());
    }

    #[test]
    fn rested_xp_time_sync_response_matches_cpp_wire_layout() {
        assert_eq!(
            build_time_sync_response_payload(0x1122_3344, 0x5566_7788),
            [0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55]
        );
        assert_eq!(
            parse_time_sync_request_sequence(&[4, 3, 2, 1]).unwrap(),
            0x0102_0304
        );
        assert!(parse_time_sync_request_sequence(&[0, 1, 2]).is_err());
    }

    #[test]
    fn rested_xp_create_discovery_filters_position_and_runtime_counter() {
        let payload =
            rested_xp_create_object_fixture(530, 15_274, 77_001, 10_188.0, -6_347.5, 30.5);
        let discovered = find_creature_guid_near_position_in_update_object(
            &payload,
            530,
            15_274,
            10_187.8,
            -6_347.56,
            30.459,
            2.0,
            Some(77_001),
        )
        .expect("matching CREATE_OBJECT must be discovered");
        assert_eq!(discovered.low, 77_001);
        assert!((discovered.x - 10_188.0).abs() < f32::EPSILON);

        assert!(
            find_creature_guid_near_position_in_update_object(
                &payload,
                530,
                15_274,
                10_187.8,
                -6_347.56,
                30.459,
                2.0,
                Some(77_002),
            )
            .is_none(),
            "an override for another runtime object must not bind this SQL-position candidate"
        );
        assert!(
            find_creature_guid_near_position_in_update_object(
                &payload, 530, 15_274, 10_000.0, -6_347.56, 30.459, 2.0, None,
            )
            .is_none(),
            "a same-entry runtime object away from the selected SQL spawn must be rejected"
        );
    }

    #[test]
    fn rested_xp_runtime_override_must_match_discovered_sql_position_candidate() {
        let target = rested_xp_target_fixture(77_001);
        let (low, high) = create_creature_guid_raw(target.map_id, target.entry, 77_001);
        let candidate = DiscoveredCreatureGuid {
            low,
            high,
            x: target.x as f32,
            y: target.y as f32,
            z: target.z as f32,
        };
        assert_eq!(
            resolve_rested_xp_runtime_target(&target, Some(candidate)).unwrap(),
            candidate
        );

        let (wrong_low, wrong_high) = create_creature_guid_raw(target.map_id, target.entry, 77_002);
        let wrong_candidate = DiscoveredCreatureGuid {
            low: wrong_low,
            high: wrong_high,
            ..candidate
        };
        let mismatch = resolve_rested_xp_runtime_target(&target, Some(wrong_candidate))
            .expect_err("a counter from another spawn must fail closed");
        assert!(mismatch
            .to_string()
            .contains("did not match discovered counter"));

        let missing = resolve_rested_xp_runtime_target(&target, None)
            .expect_err("an unobserved override cannot prove SQL spawn identity");
        assert!(missing.to_string().contains("cannot be linked safely"));
    }

    #[test]
    fn rested_xp_realm_routing_rejects_any_instance_duplicate() {
        assert!(validate_rested_xp_instance_post_realm_opcode(SMSG_UPDATE_OBJECT).is_ok());
        let duplicate = validate_rested_xp_instance_post_realm_opcode(SMSG_LOG_XP_GAIN)
            .expect_err("instance XP must invalidate a realm observation");
        assert!(duplicate.to_string().contains("duplicated/misrouted"));
        assert_eq!(NOMINAL_MELEE_RANGE_LIKE_CPP, 5.0);
    }

    #[test]
    fn rested_xp_target_rejects_cpp_dynamic_no_xp_critters_and_vehicle_guids() {
        assert!(validate_rested_xp_target_template(15_274, 1, 0).is_ok());

        let critter = validate_rested_xp_target_template(15_274, CREATURE_TYPE_CRITTER, 0)
            .expect_err("critters must be rejected even without a persisted NO_XP flag");
        assert!(critter.to_string().contains("critter"));

        let vehicle = validate_rested_xp_target_template(15_274, 1, 123)
            .expect_err("vehicles use a different C++ HighGuid and must fail closed");
        assert!(vehicle.to_string().contains("HighGuid::Vehicle"));
    }

    #[test]
    fn rested_xp_offline_math_and_cap_match_both_cpp_references() {
        let wilderness =
            offline_rest_bonus_like_cpp(400, 86_400, REST_OFFLINE_WILDERNESS_BUBBLE, 1.0);
        let resting =
            offline_rest_bonus_like_cpp(400, 86_400, REST_OFFLINE_TAVERN_OR_CITY_BUBBLE, 1.0);

        assert!((wilderness - 14.88).abs() < 0.001);
        assert!((resting - 60.0).abs() < 0.001);
        assert_eq!(offline_rest_bonus_like_cpp(400, u64::MAX, 1.0, 1.0), 300.0);
        assert!((REST_BONUS_CAP_NEXT_LEVEL_FACTOR - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn rested_xp_saved_state_distinguishes_active_relog_from_offline_save() {
        let active = RestedXpDbState {
            level: 1,
            xp: 100,
            rest_state: REST_STATE_RESTED,
            rest_bonus: 250.0,
            online: 1,
        };

        assert!(
            validate_rested_xp_persistence_state(active, 1, 100, 250.0, 1, "active relog",).is_ok()
        );
        assert!(
            validate_rested_xp_persistence_state(active, 1, 100, 250.0, 0, "offline save",)
                .is_err()
        );
    }

    #[test]
    fn rested_xp_world_config_rate_is_case_insensitive_and_last_wins() {
        let contents = r#"
            Rate.Rest.Offline.InWilderness = 1.0
            rate.rest.offline.inwilderness = "1.5" # effective value
        "#;
        assert_eq!(
            worldserver_config_f32_from_contents(contents, "Rate.Rest.Offline.InWilderness", 0.25,)
                .unwrap(),
            1.5
        );
        assert_eq!(
            worldserver_config_f32_from_contents(contents, "Missing.Rate", 0.25).unwrap(),
            0.25
        );

        let error = worldserver_config_f32_from_contents("Rate.Rest = nope", "Rate.Rest", 1.0)
            .expect_err("malformed configured rate must not fall back silently");
        assert!(error.to_string().contains("invalid Rate.Rest value"));

        let stats = r#"
            PlayerSave.Stats.MinLevel = 80
            playersave.stats.minlevel = "0" # effective value
        "#;
        assert_eq!(
            worldserver_config_u32_from_contents(stats, "PlayerSave.Stats.MinLevel", 1).unwrap(),
            0
        );
        assert_eq!(
            worldserver_config_u32_from_contents(stats, "Missing.Integer", 7).unwrap(),
            7
        );
        let error = worldserver_config_u32_from_contents(
            "PlayerSave.Stats.MinLevel = nope",
            "PlayerSave.Stats.MinLevel",
            0,
        )
        .expect_err("malformed stats gate must not fall back silently");
        assert!(error
            .to_string()
            .contains("invalid PlayerSave.Stats.MinLevel value"));
    }

    #[test]
    fn pinned_instance_port_rejects_invalid_or_different_connect_to_target() {
        assert!(validate_pinned_instance_port(8086, None).is_ok());
        assert!(validate_pinned_instance_port(8086, Some("8086")).is_ok());

        let different = validate_pinned_instance_port(9000, Some("8086"))
            .expect_err("a different advertised instance port must fail closed");
        assert!(different
            .to_string()
            .contains("advertised instance port 9000"));
        assert!(validate_pinned_instance_port(8086, Some("0")).is_err());
        assert!(validate_pinned_instance_port(8086, Some("not-a-port")).is_err());
    }

    #[test]
    fn loot_mode_rejects_generic_account_provisioning() {
        assert!(validate_provisioning_mode(false, false).is_ok());
        assert!(validate_provisioning_mode(false, true).is_ok());
        assert!(validate_provisioning_mode(true, false).is_ok());
        let error = validate_provisioning_mode(true, true)
            .expect_err("loot mode must reject provisioning before any DB mutation");
        assert!(error.to_string().contains("forbid --ensure-test-accounts"));
    }

    #[test]
    fn create_only_provisioning_rejects_partial_identity_collisions() {
        assert_eq!(
            create_only_provisioning_plan(false, false).unwrap(),
            CreateOnlyProvisioningPlan::CreateBoth
        );
        assert_eq!(
            create_only_provisioning_plan(true, true).unwrap(),
            CreateOnlyProvisioningPlan::ValidateExisting
        );
        assert!(create_only_provisioning_plan(true, false).is_err());
        assert!(create_only_provisioning_plan(false, true).is_err());
    }

    #[test]
    fn void_storage_contents_parser_matches_cpp_fixed_guid_layout() {
        let mut payload = vec![1];
        payload.extend_from_slice(&77u64.to_le_bytes());
        payload.extend_from_slice(&0x0C00_0400_0000_0000u64.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&0u64.to_le_bytes());
        payload.extend_from_slice(&5u32.to_le_bytes());
        payload.extend_from_slice(&2589i32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes());
        payload.extend_from_slice(&0i32.to_le_bytes());
        payload.push(0); // no bonus list
        payload.push(0); // zero 6-bit item modifiers

        assert_eq!(
            parse_void_storage_contents(&payload).unwrap(),
            vec![VoidStorageItemWire {
                item_id: 77,
                slot: 5,
                item_entry: 2589,
            }]
        );
        payload.push(0);
        assert!(parse_void_storage_contents(&payload).is_err());
    }

    #[test]
    fn void_storage_success_requires_every_fresh_login_checkpoint() {
        let mut result = BotRunResult {
            world_auth: true,
            enum_characters: true,
            player_login_verified: true,
            void_storage_smoke: true,
            void_storage_smoke_passed: Some(true),
            void_storage_unlock_persisted: true,
            void_storage_deposit_persisted: true,
            void_storage_deposit_relogin_verified: true,
            void_storage_swap_persisted: true,
            void_storage_swap_relogin_verified: true,
            void_storage_withdraw_persisted: true,
            ..BotRunResult::default()
        };
        assert!(!result.success(false, false, false));
        result.void_storage_withdraw_relogin_verified = true;
        assert!(result.success(false, false, false));
    }

    #[test]
    fn void_storage_query_capture_has_its_own_success_contract() {
        let mut result = BotRunResult {
            world_auth: true,
            enum_characters: true,
            player_login_verified: true,
            void_storage_query_capture: true,
            ..BotRunResult::default()
        };
        assert!(!result.success(false, false, false));
        result.void_storage_query_capture_passed = Some(true);
        assert!(result.success(false, false, false));
        assert!(!result.void_storage_smoke);
    }

    #[test]
    fn void_storage_explicit_runtime_guid_does_not_require_login_discovery() {
        assert!(void_storage_login_target_ready(false, false));
        assert!(!void_storage_login_target_ready(true, false));
        assert!(void_storage_login_target_ready(true, true));
    }

    #[test]
    fn explicit_creature_guid_includes_active_realm_like_cpp() {
        assert_eq!(
            create_void_storage_creature_guid_raw(571, 31_810, 24, 1),
            (24, 0x2000_0447_601F_1080)
        );
        assert_eq!(
            create_void_storage_creature_guid_raw(530, 18_525, 111, 0),
            create_creature_guid_raw(530, 18_525, 111)
        );
    }

    #[test]
    fn void_storage_wire_guid_keeps_active_realm_like_runtime() {
        let target = ResolvedCreatureTarget {
            entry: 31_810,
            spawn_guid: 24,
            guid_counter: 24,
            map_id: 571,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            orientation: 0.0,
            packed_guid: Vec::new(),
        };
        assert_eq!(
            vault_keeper_full_guid(&target, 1),
            [24, 0, 0, 0, 0, 0, 0, 0, 0x80, 0x10, 0x1F, 0x60, 0x47, 0x04, 0, 0x20,]
        );
    }
}

fn build_quest_giver_query_quest(packed_guid: &[u8], quest_id: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(packed_guid.len() + 5);
    data.extend_from_slice(packed_guid);
    data.extend_from_slice(&quest_id.to_le_bytes());
    data.push(0x80); // RespondToGiver=true, MSB-first WriteBit/ReadBit.
    data
}

fn build_quest_giver_accept_quest(packed_guid: &[u8], quest_id: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(packed_guid.len() + 5);
    data.extend_from_slice(packed_guid);
    data.extend_from_slice(&quest_id.to_le_bytes());
    data.push(0x00); // StartCheat=false, padded to one bit byte.
    data
}

fn build_gossip_select_option(
    packed_guid: &[u8],
    gossip_id: i32,
    gossip_option_id: i32,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(packed_guid.len() + 9);
    data.extend_from_slice(packed_guid);
    data.extend_from_slice(&gossip_id.to_le_bytes());
    data.extend_from_slice(&gossip_option_id.to_le_bytes());
    data.push(0x00); // PromotionCode length = 0, written as 8 MSB-first bits.
    data
}

fn parse_connect_to(payload: &[u8]) -> Option<ConnectToTarget> {
    // RustyCore ConnectTo payload:
    // signature[256], address_type u8, address (4/16), port u16, serial u32,
    // connection_type u8, key i64.
    let address_type = *payload.get(256)?;
    let address_offset = 257;
    let (address, address_len) = match address_type {
        1 => {
            let bytes: [u8; 4] = payload
                .get(address_offset..address_offset + 4)?
                .try_into()
                .ok()?;
            (IpAddr::from(bytes), 4)
        }
        2 => {
            let bytes: [u8; 16] = payload
                .get(address_offset..address_offset + 16)?
                .try_into()
                .ok()?;
            (IpAddr::from(bytes), 16)
        }
        _ => return None,
    };
    let serial_offset = 256 + 1 + address_len + 2;
    let port_offset = 256 + 1 + address_len;
    let port = u16::from_le_bytes(payload.get(port_offset..port_offset + 2)?.try_into().ok()?);
    let serial = u32::from_le_bytes(
        payload
            .get(serial_offset..serial_offset + 4)?
            .try_into()
            .ok()?,
    );
    let connection_type_offset = serial_offset + 4;
    let connection_type = *payload.get(connection_type_offset)?;
    let key_offset = connection_type_offset + 1;
    let key = i64::from_le_bytes(payload.get(key_offset..key_offset + 8)?.try_into().ok()?);

    Some(ConnectToTarget {
        address,
        port,
        serial,
        connection_type,
        key,
    })
}

async fn connect_to_instance(
    bot_index: usize,
    connect_to: &ConnectToTarget,
    session_key: &[u8],
) -> Result<(TcpStream, WorldCrypt)> {
    if connect_to.connection_type != 1 {
        bail!(
            "SMSG_CONNECT_TO requested unsupported connection type {}",
            connect_to.connection_type
        );
    }

    let expected_port = std::env::var("INSTANCE_PORT").ok();
    validate_pinned_instance_port(connect_to.port, expected_port.as_deref())?;

    let target_host =
        std::env::var("INSTANCE_HOST").unwrap_or_else(|_| connect_to.address.to_string());
    let addr = format!("{}:{}", target_host, connect_to.port);
    info!(
        "[Bot {}] Connecting to instance socket {}...",
        bot_index, addr
    );
    let mut stream = tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, TcpStream::connect(&addr))
        .await
        .map_err(|_| anyhow!("Timed out connecting to instance socket {addr}"))?
        .map_err(|e| anyhow!("Failed to connect to instance socket {}: {}", addr, e))?;

    let mut init_buf = vec![0u8; 256];
    let n = tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, stream.read(&mut init_buf))
        .await
        .map_err(|_| anyhow!("Timed out reading instance SERVER_INIT"))??;
    if !init_buf[..n].starts_with(&SERVER_INIT[..SERVER_INIT.len().min(n)]) {
        bail!(
            "Unexpected instance server init: {:?}",
            String::from_utf8_lossy(&init_buf[..n])
        );
    }

    tokio::time::timeout(INITIAL_NETWORK_IO_TIMEOUT, async {
        stream.write_all(CLIENT_INIT).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| anyhow!("Timed out writing instance CLIENT_INIT"))??;

    let (opcode, challenge_data) = tokio::time::timeout(
        INITIAL_NETWORK_IO_TIMEOUT,
        read_unencrypted_packet(&mut stream),
    )
    .await
    .map_err(|_| anyhow!("Timed out reading instance SMSG_AUTH_CHALLENGE"))??;
    if opcode != 0x3048 {
        bail!(
            "Expected instance SMSG_AUTH_CHALLENGE (0x3048), got 0x{:04X}",
            opcode
        );
    }
    if challenge_data.len() < 48 {
        bail!(
            "Instance SMSG_AUTH_CHALLENGE too short: {} bytes",
            challenge_data.len()
        );
    }

    let server_challenge: [u8; 16] = challenge_data[32..48].try_into()?;
    let local_challenge: [u8; 16] = rand::random();
    let digest = compute_continued_auth_digest(
        connect_to.key,
        &local_challenge,
        &server_challenge,
        session_key,
    );
    let auth_data = build_cmsg_auth_continued_session(connect_to.key, &local_challenge, &digest);
    send_unencrypted_packet(&mut stream, 0x3766, &auth_data).await?;

    let mut got_encryption = false;
    for _ in 0..10 {
        let (op, _payload) =
            tokio::time::timeout(Duration::from_secs(5), read_unencrypted_packet(&mut stream))
                .await
                .map_err(|_| anyhow!("Timeout waiting for instance encrypted mode"))??;

        if op == 0x3049 {
            got_encryption = true;
            break;
        }

        info!(
            "[Bot {}] Instance pre-encryption packet 0x{:04X}",
            bot_index, op
        );
    }

    if !got_encryption {
        bail!("Instance socket did not send SMSG_ENTER_ENCRYPTED_MODE");
    }

    let enc_key = derive_instance_encryption_key(session_key, &local_challenge, &server_challenge);
    send_unencrypted_packet(&mut stream, 0x3767, &[]).await?;

    Ok((stream, WorldCrypt::new_with_counters(&enc_key, 2, 2)))
}

fn validate_pinned_instance_port(advertised_port: u16, expected_port: Option<&str>) -> Result<()> {
    let Some(expected_port) = expected_port else {
        return Ok(());
    };
    let expected_port = expected_port
        .parse::<u16>()
        .with_context(|| "INSTANCE_PORT must be a valid nonzero TCP port")?;
    if expected_port == 0 {
        bail!("INSTANCE_PORT must be a valid nonzero TCP port");
    }
    if advertised_port != expected_port {
        bail!(
            "SMSG_CONNECT_TO advertised instance port {}, expected pinned INSTANCE_PORT {}",
            advertised_port,
            expected_port
        );
    }
    Ok(())
}

/// Pack u64 into WoW's packed format (mask + non-zero bytes)
fn pack_u64(value: u64) -> (u8, Vec<u8>) {
    let mut mask = 0u8;
    let mut bytes = Vec::new();

    for i in 0..8 {
        let b = (value >> (i * 8)) as u8;
        if b != 0 {
            mask |= 1 << i;
            bytes.push(b);
        }
    }

    (mask, bytes)
}

/// Echo the Ticket+InstanceID+ProposalID prefix from a SMSG_LFG_PROPOSAL_UPDATE
/// payload back into a CMSG_DF_PROPOSAL_RESPONSE body, with `Accepted=true`.
///
/// SMSG_LFG_PROPOSAL_UPDATE layout (LFGPackets.cpp:375):
///   Ticket {
///     ObjectGuid RequesterGuid  // 1 lowMask + 1 highMask + popcnt(low)+popcnt(high) bytes
///     uint32 Id, uint32 Type, uint64 Time
///     bit Unknown925, FlushBits  // 1 byte
///   }
///   uint64 InstanceID, uint32 ProposalID, ... rest we ignore
///
/// CMSG_DF_PROPOSAL_RESPONSE just wants the same Ticket+InstanceID+ProposalID
/// prefix plus a single Accepted bit.
fn build_proposal_response(payload: &[u8]) -> Option<Vec<u8>> {
    if payload.len() < 2 {
        return None;
    }
    let low_mask = payload[0];
    let high_mask = payload[1];
    let guid_len = 2 + (low_mask.count_ones() + high_mask.count_ones()) as usize;
    let prefix_len = guid_len
        + 4  // Id
        + 4  // Type
        + 8  // Time
        + 1  // bit byte (Unknown925 + flush)
        + 8  // InstanceID
        + 4; // ProposalID
    if payload.len() < prefix_len {
        return None;
    }
    let mut response = Vec::with_capacity(prefix_len + 1);
    response.extend_from_slice(&payload[..prefix_len]);
    response.push(0x80); // Accepted=true bit, MSB-first per Trinity's WriteBit
    Some(response)
}

/// Build CMSG_DF_JOIN body — layout matches main_srp6_complete.rs::build_lfg_join_packet,
/// which is what the 3.4.3 server's DF_JOIN handler parses.
///
///   bits[8]:  QueueAsGroup | hasPartyIndex | Unknown | 5b padding → all zero
///   u8:       Roles bitmask (2=Tank, 4=Healer, 8=DPS)
///   u32 LE:   NumDungeons (always 1)
///   u32 LE:   DungeonID
fn build_lfg_join(dungeon_id: u32, roles: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(10);
    data.push(0x00);
    data.push(roles);
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&dungeon_id.to_le_bytes());
    data
}
