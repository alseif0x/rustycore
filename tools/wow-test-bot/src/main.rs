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
use tracing::{debug, error, info, warn};

mod bot_srp6;
mod config;
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
const SMSG_ON_MONSTER_MOVE: u16 = 0x2DD4;
const CMSG_BANKER_ACTIVATE: u16 = 0x34B3;
const CMSG_AUTOBANK_ITEM: u16 = 0x3997;
const CMSG_AUTOSTORE_BANK_ITEM: u16 = 0x3996;
const SMSG_NPC_INTERACTION_OPEN_RESULT: u16 = 0x288A;
const SMSG_INVENTORY_CHANGE_FAILURE: u16 = 0x2DA5;
const CMSG_LOGOUT_REQUEST: u16 = 0x34D6;
const SMSG_LOGOUT_COMPLETE: u16 = 0x2684;
const INVENTORY_SLOT_BAG_0: u8 = 255;
const INVENTORY_SLOT_ITEM_START: u8 = 35;
const BANK_SLOT_ITEM_START: u8 = 59;
const BANK_SLOT_ITEM_END: u8 = 87;
const NPC_FLAG_BANKER: u32 = 0x20000;
const DEFAULT_BANK_SMOKE_ITEM_ENTRY: u32 = 2589;
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

#[derive(Debug, Clone, Copy)]
struct CharacterPositionSnapshot {
    map_id: u32,
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
    Ok(opts)
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next().ok_or_else(|| anyhow!("{} needs a value", flag))
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
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
    println!("  --ensure-test-accounts   Upsert local TESTBOT auth rows before login");
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
    let auth_db = auth_db_url()?;
    let char_db = characters_db_url()?;
    let auth_opts = mysql::Opts::from_url(&auth_db).map_err(|e| anyhow!("Bad auth DB URL: {e}"))?;
    let char_opts =
        mysql::Opts::from_url(&char_db).map_err(|e| anyhow!("Bad character DB URL: {e}"))?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut char_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    for bot in bots {
        ensure_local_bot_account(&mut auth_conn, bot)?;
        ensure_local_bot_character_owner(&mut char_conn, bot)?;
        sync_realm_character_count(&mut auth_conn, &mut char_conn, bot)?;
    }

    Ok(())
}

fn ensure_local_bot_account(conn: &mut mysql::Conn, bot: &config::BotConfig) -> Result<()> {
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

    let bnet_id = if let Some(id) = conn
        .exec_first::<u32, _, _>(
            "SELECT id FROM battlenet_accounts WHERE email = ?",
            (&email,),
        )
        .map_err(|e| anyhow!("Lookup BNet account {email}: {e}"))?
    {
        conn.exec_drop(
            "UPDATE battlenet_accounts \
             SET srp_version = 1, salt = ?, verifier = ?, failed_logins = 0, locked = 0, lock_country = '00' \
             WHERE id = ?",
            (bnet_salt.to_vec(), bnet_verifier, id),
        )
        .map_err(|e| anyhow!("Update BNet account {email}: {e}"))?;
        id
    } else {
        conn.exec_drop(
            "INSERT INTO battlenet_accounts (email, srp_version, salt, verifier) VALUES (?, 1, ?, ?)",
            (&email, bnet_salt.to_vec(), bnet_verifier),
        )
        .map_err(|e| anyhow!("Insert BNet account {email}: {e}"))?;
        u32::try_from(conn.last_insert_id()).map_err(|_| anyhow!("BNet account id overflow"))?
    };

    let game_username = game_account_username(&bot.account)?;
    // The 3.4.3 world login path below authenticates through
    // account.session_key_bnet. These legacy Grunt fields are still NOT NULL in
    // the auth schema, so keep them initialized for local bot rows.
    let account_salt = random_32();
    let account_verifier = fixed_le_32(Vec::new());
    let account_exists = conn
        .exec_first::<u32, _, _>("SELECT id FROM account WHERE id = ?", (bot.account_id,))
        .map_err(|e| anyhow!("Lookup game account id {}: {e}", bot.account_id))?
        .is_some();

    if account_exists {
        conn.exec_drop(
            "UPDATE account \
             SET username = ?, salt = ?, verifier = ?, reg_mail = ?, email = ?, battlenet_account = ?, \
                 battlenet_index = 1, expansion = 9, failed_logins = 0, locked = 0, lock_country = '00', online = 0 \
             WHERE id = ?",
            (
                &game_username,
                account_salt.to_vec(),
                account_verifier,
                &email,
                &email,
                bnet_id,
                bot.account_id,
            ),
        )
        .map_err(|e| anyhow!("Update game account {}: {e}", bot.account_id))?;
    } else {
        conn.exec_drop(
            "INSERT INTO account \
             (id, username, salt, verifier, reg_mail, email, joindate, battlenet_account, battlenet_index, expansion) \
             VALUES (?, ?, ?, ?, ?, ?, NOW(), ?, 1, 9)",
            (
                bot.account_id,
                &game_username,
                account_salt.to_vec(),
                account_verifier,
                &email,
                &email,
                bnet_id,
            ),
        )
        .map_err(|e| anyhow!("Insert game account {}: {e}", bot.account_id))?;
    }

    conn.exec_drop(
        "DELETE FROM battlenet_account_bans WHERE id = ?",
        (bnet_id,),
    )
    .map_err(|e| anyhow!("Clear BNet bans for {email}: {e}"))?;
    conn.exec_drop(
        "UPDATE account_banned SET active = 0 WHERE id = ?",
        (bot.account_id,),
    )
    .map_err(|e| anyhow!("Clear game account bans for {}: {e}", bot.account_id))?;

    info!(
        "[Bot {}] ensured local auth account {} / character GUID {}",
        bot.account_id, email, bot.character_guid
    );
    Ok(())
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

fn ensure_local_bot_character_owner(conn: &mut mysql::Conn, bot: &config::BotConfig) -> Result<()> {
    use mysql::prelude::Queryable;

    let owner = conn
        .exec_first::<u32, _, _>(
            "SELECT account FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Lookup character {}: {e}", bot.character_guid))?
        .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;

    if owner != bot.account_id {
        warn!(
            "[Bot {}] character GUID {} belonged to account {}; reassigning to test account",
            bot.account_id, bot.character_guid, owner
        );
        conn.exec_drop(
            "UPDATE characters SET account = ? WHERE guid = ?",
            (bot.account_id, bot.character_guid),
        )
        .map_err(|e| anyhow!("Update owner for character {}: {e}", bot.character_guid))?;
    }

    Ok(())
}

fn sync_realm_character_count(
    auth_conn: &mut mysql::Conn,
    char_conn: &mut mysql::Conn,
    bot: &config::BotConfig,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let count = char_conn
        .exec_first::<u32, _, _>(
            "SELECT COUNT(*) FROM characters WHERE account = ?",
            (bot.account_id,),
        )
        .map_err(|e| anyhow!("Count characters for account {}: {e}", bot.account_id))?
        .unwrap_or(0);
    auth_conn
        .exec_drop(
            "REPLACE INTO realmcharacters (numchars, acctid, realmid) VALUES (?, ?, ?)",
            (count, bot.account_id, realm_id()),
        )
        .map_err(|e| anyhow!("Sync realmcharacters for account {}: {e}", bot.account_id))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("🎮 WoW Test Bot - TrinityCore 3.4.3 SRP6 + AES-GCM");
    info!("═══════════════════════════════════════════════════");

    let cli = parse_cli()?;
    let app_config = config::AppConfig::load_or_create(&cli.config_path)?;
    let mut bots: Vec<config::BotConfig> =
        app_config.get_enabled_bots().into_iter().cloned().collect();

    if let Some(account) = &cli.single_account {
        bots.retain(|bot| bot.account.eq_ignore_ascii_case(account));
    }
    apply_password_overrides(&mut bots);

    if bots.is_empty() {
        bail!("No enabled bots matched the current config/filter");
    }
    let missing_passwords: Vec<&str> = bots
        .iter()
        .filter(|bot| bot.password.is_empty())
        .map(|bot| bot.account.as_str())
        .collect();
    if !missing_passwords.is_empty() {
        bail!(
            "Missing bot password for {}. Set WOW_BOT_PASSWORD, set {}, or use an ignored local config.json.",
            missing_passwords.join(", "),
            password_env_name(missing_passwords[0])
        );
    }
    if cli.ensure_test_accounts {
        let bots_for_db = bots.clone();
        tokio::task::spawn_blocking(move || ensure_test_accounts(&bots_for_db))
            .await
            .map_err(|e| anyhow!("DB worker join failed while ensuring test accounts: {}", e))?
            .map_err(|e| anyhow!("Failed to ensure test accounts: {}", e))?;
    }
    let post_login_mode_count = [cli.stand_state_smoke, cli.bank_smoke, cli.quest_smoke]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if post_login_mode_count > 1 {
        bail!("stand-state, bank, and quest smoke are separate post-login modes");
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

    if cleanup_groups && !cli.login_only && !cli.stand_state_smoke && !cli.bank_smoke {
        cleanup_bot_group_state(&bots)?;
    }

    let expected_bot_count = bots.len();
    let mut results = Vec::new();
    if cli.sequential || bots.len() == 1 {
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
            } else {
                run_bot(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.login_only,
                    stand_state_options.clone(),
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
    quest_options: Option<QuestSmokeOptions>,
) -> Result<BotRunResult> {
    let bot_index = bot.account_id as usize;
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

    // ── Step 1: Live SRP6 against bnetserver bot endpoint ────────────────────
    // Computes (login_ticket, K_32) where K = SHA256(broken_evidence_le(S)).
    // We expand to 64 bytes (K || SHA256(K)) to match worldserver's expected
    // session_key_bnet width, then push it into account.session_key_bnet so the
    // worldserver picks up the live key when validating CMSG_AUTH_SESSION.
    info!(
        "[Bot {}] Step 1: Live SRP6 against bnetserver {}:{}",
        bot_index,
        bnet_host(),
        bnet_port()
    );

    let bnet_url = format!("https://{}:{}", bnet_host(), bnet_port());
    let (login_ticket, session_key_32) =
        bot_srp6::authenticate_bot(&bnet_url, &bot.account, &bot.password)
            .await
            .map_err(|e| anyhow!("Bot SRP6 failed: {}", e))?;
    if session_key_32.len() != 32 {
        bail!(
            "Bot SRP6 returned K of unexpected length: {}",
            session_key_32.len()
        );
    }
    let session_key = expand_session_key(&session_key_32).to_vec();

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

    info!("[Bot {}] ✅ LoginTicket received", bot_index);
    info!("[Bot {}] ✅ K (live, 32B) received", bot_index);
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
    let mut stream = TcpStream::connect(&world_addr)
        .await
        .map_err(|e| anyhow!("Failed to connect to world server: {}", e))?;
    info!("[Bot {}] ✅ TCP connected", bot_index);

    // ── Step 3: World Server Handshake ──────────────────────────────────────
    info!("[Bot {}] Step 3: Handshake...", bot_index);
    let mut init_buf = vec![0u8; 256];
    let n = stream.read(&mut init_buf).await?;
    if !init_buf[..n].starts_with(&SERVER_INIT[..SERVER_INIT.len().min(n)]) {
        bail!(
            "Unexpected server init: {:?}",
            String::from_utf8_lossy(&init_buf[..n])
        );
    }
    info!("[Bot {}] ✅ SERVER_INIT received", bot_index);

    stream.write_all(CLIENT_INIT).await?;
    stream.flush().await?;
    info!("[Bot {}] ✅ CLIENT_INIT sent", bot_index);

    // ── Step 4: Read SMSG_AUTH_CHALLENGE ────────────────────────────────────
    info!("[Bot {}] Step 4: Reading SMSG_AUTH_CHALLENGE...", bot_index);
    let (opcode, challenge_data) = read_unencrypted_packet(&mut stream).await?;
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
    // NOT the bnet login_ticket — sending the bnet ticket here yields "unknown account".
    let _ = &login_ticket; // ticket only needed for the bnetserver REST proof
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
    for _ in 0..30 {
        match tokio::time::timeout(
            Duration::from_secs(5),
            read_encrypted_packet(&mut stream, &mut crypt, &mut server_inflater),
        )
        .await
        {
            Ok(Ok((op, payload))) => {
                result.seen_opcodes.push(format!("0x{:04X}", op));
                if let Some(options) = quest_options.as_ref() {
                    record_quest_objective_login_signal(op, &payload, options, &mut result);
                }
                if op == 0x2597 {
                    // SMSG_LOGIN_VERIFY_WORLD
                    info!("[Bot {}] ✅ SMSG_LOGIN_VERIFY_WORLD received", bot_index);
                    login_ok = true;
                    if stand_state_options.is_none() || realm_connection.is_some() {
                        break;
                    }
                    // A stand-state capture validates connection routing, so
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
                    if stand_state_options.is_some() {
                        if realm_connection.is_some() {
                            bail!("Stand-state smoke received more than one SMSG_CONNECT_TO");
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
                    if login_ok && stand_state_options.is_some() {
                        break;
                    }
                } else if op == 0x304B {
                    // SMSG_RESUME_COMMS
                    info!("[Bot {}] ✅ SMSG_RESUME_COMMS received", bot_index);
                }
            }
            Ok(Err(e)) => {
                warn!("[Bot {}] Login read error: {}", bot_index, e);
                break;
            }
            Err(_) => break,
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
    let mut payload = [0u8; 8];
    payload[..4].copy_from_slice(&STAND_STATE_CAPTURE_FENCE_SERIAL.to_le_bytes());
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
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

fn build_auto_bank_item_payload(slot: u8) -> [u8; 5] {
    // C++ InvUpdate count=1 is two MSB-first bits `01`, followed by the
    // affected position and then the packet's source bag/slot.
    [0x40, INVENTORY_SLOT_BAG_0, slot, INVENTORY_SLOT_BAG_0, slot]
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

    let character: Option<(u32, u8, u32, f64, f64, f64, f32)> = characters
        .exec_first(
            "SELECT account, online, map, position_x, position_y, position_z, orientation \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|e| anyhow!("Load bank bot character: {e}"))?;
    let (owner, online, map_id, x, y, z, orientation) =
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
            "UPDATE characters SET map = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
             WHERE guid = ?",
            (
                fixture.original_position.map_id,
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
    let (low_mask, low_bytes) = pack_u64(guid);

    // High part: (HighGuid::Player << 58) | (realmId << 42)
    let high = (2u64 << 58) | ((u64::from(realm_id) & 0x1FFF) << 42);
    let (high_mask, high_bytes) = pack_u64(high);

    let mut data = Vec::with_capacity(2 + low_bytes.len() + high_bytes.len() + 4);
    data.push(low_mask);
    data.push(high_mask);
    data.extend_from_slice(&low_bytes);
    data.extend_from_slice(&high_bytes);
    data.extend_from_slice(&far_clip.to_le_bytes());

    data
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
    let low = counter & 0xFF_FFFF_FFFF;
    (low, high)
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
    fn auto_bank_payload_uses_cpp_inv_update_then_source_position() {
        assert_eq!(build_auto_bank_item_payload(35), [0x40, 255, 35, 255, 35]);
        assert_eq!(build_auto_bank_item_payload(59), [0x40, 255, 59, 255, 59]);
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
    fn creature_guid_uses_runtime_counter_and_zero_realm_like_cpp() {
        let (low, high) = create_creature_guid_raw(571, 15_513, 77_001);

        assert_eq!(low, 77_001);
        assert_eq!((high >> 58) & 0x3F, 8);
        assert_eq!((high >> 42) & 0x1FFF, 0);
        assert_eq!((high >> 29) & 0x1FFF, 571);
        assert_eq!((high >> 6) & 0x7F_FFFF, 15_513);
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

    let target_host =
        std::env::var("INSTANCE_HOST").unwrap_or_else(|_| connect_to.address.to_string());
    let addr = format!("{}:{}", target_host, connect_to.port);
    info!(
        "[Bot {}] Connecting to instance socket {}...",
        bot_index, addr
    );
    let mut stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| anyhow!("Failed to connect to instance socket {}: {}", addr, e))?;

    let mut init_buf = vec![0u8; 256];
    let n = stream.read(&mut init_buf).await?;
    if !init_buf[..n].starts_with(&SERVER_INIT[..SERVER_INIT.len().min(n)]) {
        bail!(
            "Unexpected instance server init: {:?}",
            String::from_utf8_lossy(&init_buf[..n])
        );
    }

    stream.write_all(CLIENT_INIT).await?;
    stream.flush().await?;

    let (opcode, challenge_data) = read_unencrypted_packet(&mut stream).await?;
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
