//! WoW Test Bot - TrinityCore 3.4.3 Modern Protocol with Full SRP6
//! Combines BNet SRP6 Auth + World Server AES-GCM Encryption + LFG

use anyhow::{anyhow, bail, Result};
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
        if self.quest_smoke {
            return self.world_auth
                && self.enum_characters
                && self.player_login_verified
                && self.quest_smoke_passed.unwrap_or(false);
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
        quest_guid_counter: std::env::var("WOW_BOT_QUEST_GUID_COUNTER")
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
    println!("  --quest-smoke            After login, right-click/query one questgiver NPC");
    println!("  --quest-creature-entry <id>  Creature entry to resolve from world.creature");
    println!("  --quest-creature-guid <guid> Optional world.creature spawn guid override");
    println!("  --quest-guid-counter <n> Optional live ObjectGuid low counter override");
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
    info!(
        "Mode: {}; client_build={}; LFG timeout: {}s; auto_teleport={}; require_proposal={}; require_group={}",
        if cli.quest_smoke {
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

    if cleanup_groups && !cli.login_only {
        cleanup_bot_group_state(&bots)?;
    }

    let expected_bot_count = bots.len();
    let mut results = Vec::new();
    if cli.sequential || bots.len() == 1 {
        for bot in bots {
            info!("\n[Bot {}] Starting...", bot.account);
            match run_bot(
                bot,
                dungeon_id,
                timeout_secs,
                auto_teleport,
                cli.login_only,
                quest_options.clone(),
            )
            .await
            {
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
    let login_data = build_player_login(bot.character_guid, 500.0);
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
                    break;
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
                    stream = instance_stream;
                    crypt = instance_crypt;
                    server_inflater = ServerPacketInflater::default();
                    info!("[Bot {}] ✅ Instance socket authenticated", bot_index);
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

    let guid_counter = quest_options.creature_guid_counter.unwrap_or(spawn_guid);
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
fn build_player_login(guid: u64, far_clip: f32) -> Vec<u8> {
    let (low_mask, low_bytes) = pack_u64(guid);

    // High part: (HighGuid::Player << 58) | (realmId << 42)
    let realm_id = 1u64;
    let high = (2u64 << 58) | (realm_id << 42);
    let (high_mask, high_bytes) = pack_u64(high);

    let mut data = Vec::with_capacity(2 + low_bytes.len() + high_bytes.len() + 4);
    data.push(low_mask);
    data.push(high_mask);
    data.extend_from_slice(&low_bytes);
    data.extend_from_slice(&high_bytes);
    data.extend_from_slice(&far_clip.to_le_bytes());

    data
}

fn create_creature_guid_raw(map_id: u16, entry: u32, counter: u64) -> (u64, u64) {
    let high = (8u64 << 58)
        | (1u64 << 42)
        | ((map_id as u64 & 0x1FFF) << 29)
        | ((entry as u64 & 0x7F_FFFF) << 6);
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
