//! Multi-client live smokes for atomic shared world-loot and group claims.
//!
//! These are deliberately mutating QA-only workflows. The two-session loot
//! race uses a wrapper-installed, shared Tattered Chest GameObject, while the
//! strict one-session capture continues to use stationary Doctor Maleficus.
//! The group-capacity race consumes the fifth slot of a preloaded party. Each
//! fixture requires explicit restoration and a world restart before reuse.

use super::*;
use mysql::params;
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Barrier, Mutex};
use tokio_util::sync::CancellationToken;

pub(super) const DEFAULT_ACCOUNT_A: &str = "TESTBOT2@bot.local";
pub(super) const DEFAULT_ACCOUNT_B: &str = "TESTBOT3@bot.local";
// Race and strict capture intentionally use different fixture kinds. The
// wrapper installs exactly one non-pooled Tattered Chest spawn before the
// world starts; Doctor Maleficus remains the creature-only capture fixture.
// The public constant names remain stable for existing callers; main.rs now
// exposes GameObject spellings and retains `--loot-race-creature-*` aliases.
pub(super) const DEFAULT_CREATURE_ENTRY: u32 = 2_846;
pub(super) const DEFAULT_CREATURE_SPAWN_GUID: u64 = 9_106_001;
pub(super) const DEFAULT_RUNTIME_COUNTER: u64 = 0;
pub(super) const DEFAULT_ITEM_ENTRY: u32 = 38;
pub(super) const DEFAULT_CAPTURE_CREATURE_ENTRY: u32 = 21_779;
pub(super) const DEFAULT_CAPTURE_CREATURE_SPAWN_GUID: u64 = 1_117;
pub(super) const DEFAULT_CAPTURE_RUNTIME_COUNTER: u64 = 0;
pub(super) const DEFAULT_CAPTURE_ITEM_ENTRY: u32 = 30_712;
pub(super) const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub(super) const DEFAULT_WORKFLOW_DEADLINE_SECS: u64 = 900;
pub(super) const ACK_FLAG: &str = "--ack-disposable-overworld-loot-race";
pub(super) const DEFAULT_GROUP_CAPACITY_LEADER: &str = "TESTBOT1@bot.local";
pub(super) const DEFAULT_GROUP_CAPACITY_CANDIDATE_A: &str = "TESTBOT2@bot.local";
pub(super) const DEFAULT_GROUP_CAPACITY_CANDIDATE_B: &str = "TESTBOT3@bot.local";
pub(super) const DEFAULT_GROUP_CAPACITY_TIMEOUT_SECS: u64 = 30;
const GUARDED_FIXTURE_HEALTH_MODIFIER: f32 = 0.0001;
const RACE_GAMEOBJECT_MAP_ID: u16 = 0;
const RACE_GAMEOBJECT_X: f64 = -8_946.95;
const RACE_GAMEOBJECT_Y: f64 = -132.493;
const RACE_GAMEOBJECT_Z: f64 = 83.5312;
const RACE_GAMEOBJECT_LOOT_ID: u32 = 2_278;
const RACE_GAMEOBJECT_MONEY: u32 = 10;
const RACE_GAMEOBJECT_ADDON_FACTION: u16 = 101;
const RACE_GAMEOBJECT_RESPAWN_SECS: u32 = 300;
const RACE_GAMEOBJECT_STATE: u8 = 1;
const RACE_GAMEOBJECT_ANIM_PROGRESS: u8 = 255;
const RACE_GAMEOBJECT_TEMPLATE_DATA: [i32; 35] = [
    57,
    RACE_GAMEOBJECT_LOOT_ID as i32,
    0,
    1,
    0,
    0,
    0,
    0,
    0,
    0,
    1,
    0,
    1,
    0,
    0,
    1,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];
const PERSONAL_LOOT_METHOD_LIKE_CPP: u8 = 5;
const HIGH_GUID_CREATURE: u64 = 8;
const HIGH_GUID_GAMEOBJECT: u64 = 11;

const CMSG_PARTY_INVITE: u16 = 0x3604;
const CMSG_PARTY_INVITE_RESPONSE: u16 = 0x3606;
const CMSG_LEAVE_GROUP: u16 = 0x364C;
const CMSG_GAME_OBJ_USE: u16 = 0x34EE;
const CMSG_LOOT_UNIT: u16 = 0x320F;
const CMSG_LOOT_ITEM: u16 = 0x3211;
const CMSG_LOOT_MONEY: u16 = 0x3210;
const SMSG_PARTY_INVITE: u16 = 0x25BD;
const SMSG_PARTY_UPDATE: u16 = 0x25F4;
const SMSG_PARTY_COMMAND_RESULT: u16 = 0x2796;
const SMSG_PARTY_MEMBER_FULL_STATE: u16 = 0x2759;
const SMSG_LOOT_RESPONSE: u16 = 0x2614;
const SMSG_LOOT_REMOVED: u16 = 0x2615;
const SMSG_LOOT_MONEY_NOTIFY: u16 = 0x261C;
const SMSG_COIN_REMOVED: u16 = 0x2617;
const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;
const RESPONSE_SETTLE: Duration = Duration::from_secs(2);
const LOOT_ITEM_CAPTURE_FENCE_SERIAL: u32 = 0x4C4F_4F54;
// The Doctor's Key (30712) is a key and C++ stores this exact fixture in the
// first keyring destination represented by the committed capture.
const LOOT_ITEM_CAPTURE_KEYRING_SLOT: u8 = 106;
// Normal C++ logout consumes 20 seconds before SMSG_LOGOUT_COMPLETE. Leave a
// bounded disconnect-save margin before any destructive fixture restoration.
const LOOT_FIXTURE_OFFLINE_WAIT_SECS: u64 = 90;
const LOOT_LOGOUT_DB_CONFIRM_WAIT_SECS: u64 = 5;
const LOOT_DB_OPERATION_TIMEOUT_SECS: u64 = 30;
const LOOT_CLEANUP_TIMEOUT_SECS: u64 = 180;
const FIXTURE_JOURNAL_VERSION: u32 = 1;
const FIXTURE_JOURNAL_ENV: &str = "WOW_BOT_FIXTURE_JOURNAL";
const HIGH_GUID_ITEM: u64 = 3;
const HIGH_GUID_LOOT_OBJECT: u64 = 15;
const GUID_HIGH_TYPE_MASK: u64 = 0x3F;
const GUID_REALM_SPECIFIC_MASK: u64 = 0xFFFF;
const GUID_REALM_MASK: u64 = 0x1FFF;
const GUID_MAP_MASK: u64 = 0x1FFF;
const GUID_ENTRY_MASK: u64 = 0x7F_FFFF;
const GUID_SUBTYPE_MASK: u64 = 0x3F;
const GUID_SERVER_MASK: u64 = 0xFF_FFFF;
const GUID_COUNTER_MASK: u64 = 0xFF_FFFF_FFFF;
// C++ Player.cpp `MAX_MONEY_AMOUNT` and Rust's represented player cap.
const MAX_PLAYER_MONEY_LIKE_CPP: u64 = 99_999_999_999;
const CHARACTER_PROGRESS_TABLES: &[&str] = &[
    "character_achievement_progress",
    "character_achievement",
    "character_queststatus_objectives_criteria_progress",
    "character_queststatus_objectives_criteria",
    "character_queststatus_objectives",
    "character_queststatus_daily",
    "character_queststatus_monthly",
    "character_queststatus_rewarded",
    "character_queststatus_seasonal",
    "character_queststatus_weekly",
    "character_queststatus",
    "character_reputation",
];

#[derive(Debug, Clone)]
pub(super) struct LootRaceCli {
    pub account_a: String,
    pub account_b: String,
    pub entry: u32,
    pub spawn_guid: u64,
    pub runtime_counter: u64,
    pub item_entry: u32,
    pub timeout_secs: u64,
    pub workflow_deadline_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LootRacePhase {
    Race,
    CaptureItem,
    VerifyRelog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum LootRaceTargetKind {
    Creature,
    GameObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LootFixturePurpose {
    Race,
    CaptureItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogoutCompletionRoute {
    Realm,
    Instance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartyPacketRoute {
    Realm,
    Instance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LootRaceTarget {
    kind: LootRaceTargetKind,
    pub entry: u32,
    pub spawn_guid: u64,
    /// Optional operator override. Zero means discover the complete live GUID
    /// from SMSG_UPDATE_OBJECT after the exact SQL spawn has passed preflight.
    pub runtime_counter_override: u64,
    pub map_id: u16,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub item_entry: u32,
}

#[derive(Debug, Clone)]
pub(super) struct LootRaceOptions {
    pub phase: LootRacePhase,
    pub participant: usize,
    pub character_guid: u64,
    pub peer_name: String,
    pub peer_character_guid: u64,
    pub killer_character_guid: u64,
    pub target: LootRaceTarget,
    pub timeout_secs: u64,
    sync: Arc<LootRaceSync>,
}

#[derive(Debug, Clone)]
pub(super) struct GroupCapacityRaceCli {
    pub leader_account: String,
    pub candidate_a_account: String,
    pub candidate_b_account: String,
    pub group_db_store_id: u32,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GroupCapacityRaceRole {
    Leader,
    CandidateA,
    CandidateB,
}

#[derive(Debug, Clone)]
pub(super) struct GroupCapacityRaceOptions {
    pub role: GroupCapacityRaceRole,
    pub character_guid: u64,
    leader_guid: u64,
    candidate_names: [String; 2],
    candidate_guids: [u64; 2],
    initial_member_guids: [u64; 4],
    party_settings: GroupCapacityPartySettings,
    pub group_db_store_id: u32,
    pub timeout_secs: u64,
    pub(super) auth_serial: Arc<Mutex<()>>,
    sync: Arc<GroupCapacityRaceSync>,
}

#[derive(Debug)]
struct GroupCapacityRaceSync {
    logged_in: Barrier,
    invitations_sent: Barrier,
    accepts_ready: Barrier,
    outcomes_observed: Barrier,
    cancelled: CancellationToken,
    failure: StdMutex<Option<String>>,
}

impl GroupCapacityRaceSync {
    fn new() -> Self {
        Self {
            logged_in: Barrier::new(3),
            invitations_sent: Barrier::new(3),
            accepts_ready: Barrier::new(2),
            outcomes_observed: Barrier::new(3),
            cancelled: CancellationToken::new(),
            failure: StdMutex::new(None),
        }
    }

    fn cancel(&self, message: impl Into<String>) {
        let message = message.into();
        let mut failure = self.failure.lock().expect("group-capacity failure lock");
        if failure.is_none() {
            *failure = Some(message);
        }
        self.cancelled.cancel();
    }

    fn cancellation_error(&self) -> Result<()> {
        if !self.cancelled.is_cancelled() {
            return Ok(());
        }
        let failure = self
            .failure
            .lock()
            .expect("group-capacity failure lock")
            .clone()
            .unwrap_or_else(|| "peer cancelled the group-capacity race".to_string());
        bail!("group-capacity race cancelled: {failure}")
    }
}

#[derive(Debug, Clone)]
struct CharacterFixture {
    bot: config::BotConfig,
    name: String,
    race: u8,
    money: u64,
    core: CharacterCoreSnapshot,
    position: CharacterPositionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CharacterCoreSnapshot {
    level: u8,
    xp: u32,
    health: u32,
    powers: [u32; 10],
    rest_state: u8,
    rest_bonus: f32,
    explored_zones: Option<String>,
    known_titles: Option<String>,
    chosen_title: u32,
}

/// Persistent rows that a creature kill, item acquisition, or the resulting
/// C++ criteria fanout can mutate.  These are snapshotted for both dedicated
/// disposable characters and restored transactionally after every run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CharacterProgressSnapshot {
    achievements: Vec<(u64, u32, i64)>,
    achievement_progress: Vec<(u64, u32, u64, i64)>,
    quest_status: Vec<(u64, u32, u8, u8, i64, i64)>,
    quest_daily: Vec<(u64, u32, i64)>,
    quest_monthly: Vec<(u64, u32)>,
    quest_objectives: Vec<(u64, u32, u8, i32)>,
    quest_objective_criteria: Vec<(u64, u32)>,
    quest_objective_criteria_progress: Vec<(u64, u32, u64, i64)>,
    quest_rewarded: Vec<(u64, u32, u8)>,
    quest_seasonal: Vec<(u64, u32, u32, i64)>,
    quest_weekly: Vec<(u64, u32)>,
    reputation: Vec<(u64, u16, i32, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RespawnSnapshot {
    respawn_time: i64,
    map_id: u16,
    instance_id: u32,
}

#[derive(Debug, Clone)]
struct LootRaceFixture {
    characters: [CharacterFixture; 2],
    target: LootRaceTarget,
    respawn: Option<RespawnSnapshot>,
    respawn_type: u16,
    gameobject_state: Option<u8>,
    progress: CharacterProgressSnapshot,
    journal: FixtureJournal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalCharacterFixture {
    account: String,
    account_id: u32,
    character_guid: u64,
    name: String,
    race: u8,
    money: u64,
    core: CharacterCoreSnapshot,
    position: CharacterPositionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FixtureJournalRecord {
    version: u32,
    created_by_pid: u32,
    characters: [JournalCharacterFixture; 2],
    target: LootRaceTarget,
    respawn: Option<RespawnSnapshot>,
    respawn_type: u16,
    gameobject_state: Option<u8>,
    progress: CharacterProgressSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CleanupMarkerRecord {
    version: u32,
    journal_sha256: String,
    cleanup_pid: u32,
}

#[derive(Debug, Clone)]
struct FixtureJournal {
    path: PathBuf,
}

fn configured_fixture_journal_path() -> Result<PathBuf> {
    let raw = std::env::var(FIXTURE_JOURNAL_ENV).map_err(|_| {
        anyhow!("loot workflows require {FIXTURE_JOURNAL_ENV}=<absolute recovery-journal path>")
    })?;
    let path = PathBuf::from(raw);
    if !path.is_absolute() || path.file_name().is_none() {
        bail!("{FIXTURE_JOURNAL_ENV} must name an absolute file path");
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("fixture journal has no parent directory"))?;
    let metadata = fs::symlink_metadata(parent)
        .with_context(|| format!("Inspect fixture-journal directory {}", parent.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!(
            "fixture-journal parent {} must be a real directory",
            parent.display()
        );
    }
    Ok(path)
}

fn cleanup_marker_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.cleanup-complete", path.display()))
}

pub(super) fn validate_journal_contract() -> Result<()> {
    let path = configured_fixture_journal_path()?;
    let marker = cleanup_marker_path(&path);
    if fs::symlink_metadata(&path).is_ok() {
        bail!(
            "pending fixture journal {} exists; run --recover-loot-fixture before another QA run",
            path.display()
        );
    }
    if fs::symlink_metadata(&marker).is_ok() {
        bail!(
            "stale cleanup marker {} exists; each QA run must use a fresh journal path",
            marker.display()
        );
    }
    Ok(())
}

impl FixtureJournalRecord {
    fn from_fixture(fixture: &LootRaceFixture) -> Self {
        Self {
            version: FIXTURE_JOURNAL_VERSION,
            created_by_pid: std::process::id(),
            characters: fixture
                .characters
                .clone()
                .map(|character| JournalCharacterFixture {
                    account: character.bot.account,
                    account_id: character.bot.account_id,
                    character_guid: character.bot.character_guid,
                    name: character.name,
                    race: character.race,
                    money: character.money,
                    core: character.core,
                    position: character.position,
                }),
            target: fixture.target.clone(),
            respawn: fixture.respawn.clone(),
            respawn_type: fixture.respawn_type,
            gameobject_state: fixture.gameobject_state,
            progress: fixture.progress.clone(),
        }
    }

    fn into_fixture(self, journal: FixtureJournal) -> Result<LootRaceFixture> {
        if self.version != FIXTURE_JOURNAL_VERSION {
            bail!(
                "unsupported fixture-journal version {}; expected {}",
                self.version,
                FIXTURE_JOURNAL_VERSION
            );
        }
        Ok(LootRaceFixture {
            characters: self.characters.map(|character| CharacterFixture {
                bot: config::BotConfig {
                    account: character.account,
                    password: String::new(),
                    character_guid: character.character_guid,
                    account_id: character.account_id,
                    lfg_role: 0,
                    class: String::new(),
                    enabled: false,
                    session_key_bnet: String::new(),
                },
                name: character.name,
                race: character.race,
                money: character.money,
                core: character.core,
                position: character.position,
            }),
            target: self.target,
            respawn: self.respawn,
            respawn_type: self.respawn_type,
            gameobject_state: self.gameobject_state,
            progress: self.progress,
            journal,
        })
    }
}

impl FixtureJournal {
    fn configured() -> Result<Self> {
        Ok(Self {
            path: configured_fixture_journal_path()?,
        })
    }

    fn persist(&self, fixture: &LootRaceFixture) -> Result<()> {
        let record = FixtureJournalRecord::from_fixture(fixture);
        let payload = serde_json::to_vec_pretty(&record)
            .context("Serialize durable loot-fixture recovery journal")?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "Create durable loot-fixture journal {}",
                    self.path.display()
                )
            })?;
        file.write_all(&payload)
            .context("Write durable loot-fixture recovery journal")?;
        file.write_all(b"\n")
            .context("Terminate durable loot-fixture recovery journal")?;
        file.sync_all()
            .context("fsync durable loot-fixture recovery journal")?;
        sync_parent_directory(&self.path)?;
        Ok(())
    }

    fn load(path: PathBuf) -> Result<LootRaceFixture> {
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("Inspect pending fixture journal {}", path.display()))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            bail!("pending fixture journal must be a regular non-symlink file");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.mode() & 0o777 != 0o600 {
                bail!("pending fixture journal permissions must be exactly 0600");
            }
        }
        let bytes = fs::read(&path)
            .with_context(|| format!("Read pending fixture journal {}", path.display()))?;
        let record: FixtureJournalRecord =
            serde_json::from_slice(&bytes).context("Parse pending fixture journal")?;
        record.into_fixture(Self { path })
    }

    fn complete(&self) -> Result<()> {
        use sha2::{Digest, Sha256};

        let journal_bytes = fs::read(&self.path)
            .with_context(|| format!("Read fixture journal {}", self.path.display()))?;
        let digest = hex::encode(Sha256::digest(&journal_bytes));
        let marker = cleanup_marker_path(&self.path);
        if fs::symlink_metadata(&marker).is_ok() {
            validate_cleanup_marker(&marker, Some(&digest))?;
        } else {
            let temp_marker =
                PathBuf::from(format!("{}.tmp.{}", marker.display(), std::process::id()));
            let marker_payload = CleanupMarkerRecord {
                version: FIXTURE_JOURNAL_VERSION,
                journal_sha256: digest.clone(),
                cleanup_pid: std::process::id(),
            };
            let write_result = (|| -> Result<()> {
                let mut marker_file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&temp_marker)
                    .with_context(|| {
                        format!("Create temporary cleanup marker {}", temp_marker.display())
                    })?;
                serde_json::to_writer_pretty(&mut marker_file, &marker_payload)
                    .context("Write fixture cleanup marker")?;
                marker_file
                    .write_all(b"\n")
                    .context("Terminate fixture cleanup marker")?;
                marker_file
                    .sync_all()
                    .context("fsync fixture cleanup marker")?;
                fs::hard_link(&temp_marker, &marker).with_context(|| {
                    format!(
                        "Atomically publish cleanup marker without replacement {}",
                        marker.display()
                    )
                })?;
                fs::remove_file(&temp_marker).with_context(|| {
                    format!("Remove temporary cleanup marker {}", temp_marker.display())
                })?;
                sync_parent_directory(&marker)
            })();
            if write_result.is_err() {
                let _ = fs::remove_file(&temp_marker);
            }
            write_result?;
            validate_cleanup_marker(&marker, Some(&digest))?;
        }
        fs::remove_file(&self.path)
            .with_context(|| format!("Remove completed journal {}", self.path.display()))?;
        sync_parent_directory(&marker)?;
        Ok(())
    }
}

fn validate_cleanup_marker(path: &Path, expected_digest: Option<&str>) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Inspect cleanup marker {}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("cleanup marker must be a regular non-symlink file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o777 != 0o600 {
            bail!("cleanup marker permissions must be exactly 0600");
        }
    }
    let marker: CleanupMarkerRecord = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Read cleanup marker {}", path.display()))?,
    )
    .context("Parse cleanup marker")?;
    if marker.version != FIXTURE_JOURNAL_VERSION
        || marker.journal_sha256.len() != 64
        || !marker
            .journal_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || expected_digest.is_some_and(|expected| marker.journal_sha256 != expected)
    {
        bail!("cleanup marker does not match the durable journal contract");
    }
    Ok(())
}

fn sync_parent_directory(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path {} has no parent", path.display()))?;
    let directory = fs::File::open(parent)
        .with_context(|| format!("Open directory {} for fsync", parent.display()))?;
    directory
        .sync_all()
        .with_context(|| format!("fsync directory {}", parent.display()))
}

pub(super) async fn recover_pending_fixture() -> Result<()> {
    if !recover_pending_fixture_if_present().await? {
        let path = configured_fixture_journal_path()?;
        bail!("no pending fixture journal exists at {}", path.display());
    }
    Ok(())
}

pub(super) async fn recover_pending_fixture_if_present() -> Result<bool> {
    let path = configured_fixture_journal_path()?;
    if fs::symlink_metadata(&path).is_err() {
        let marker = cleanup_marker_path(&path);
        if fs::symlink_metadata(&marker).is_ok() {
            validate_cleanup_marker(&marker, None)?;
            return Ok(true);
        }
        return Ok(false);
    }
    tokio::time::timeout(
        Duration::from_secs(LOOT_CLEANUP_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || {
            let fixture = FixtureJournal::load(path)?;
            cleanup_fixture(&fixture)
        }),
    )
    .await
    .map_err(|_| anyhow!("fixture recovery exceeded {LOOT_CLEANUP_TIMEOUT_SECS}s"))?
    .map_err(|error| anyhow!("fixture recovery DB worker join failed: {error}"))??;
    Ok(true)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LootWindow {
    owner_low: u64,
    owner_high: u64,
    loot_low: u64,
    loot_high: u64,
    coins: u32,
    item_entry: u32,
    quantity: u32,
    loot_list_id: u8,
    loot_method: u8,
}

#[derive(Debug, Clone, Default)]
struct WireEvidence {
    item_pushes: Vec<ItemPush>,
    loot_removed: Vec<LootRemovedEvidence>,
    money_notifies: Vec<MoneyNotify>,
    coin_removed: Vec<(u64, u64)>,
    inventory_failures: Vec<InventoryFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LootRemovedEvidence {
    owner_low: u64,
    owner_high: u64,
    loot_low: u64,
    loot_high: u64,
    loot_list_id: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ItemPush {
    player_low: u64,
    player_high: u64,
    slot: u8,
    slot_in_bag: i32,
    quest_log_item_id: i32,
    quantity: i32,
    quantity_in_inventory: i32,
    dungeon_encounter_id: i32,
    item_guid_low: u64,
    item_guid_high: u64,
    pushed: bool,
    created: bool,
    display_text: u8,
    is_bonus_roll: bool,
    is_encounter_loot: bool,
    item_entry: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WireItemGrant {
    participant: usize,
    push: ItemPush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpectedPersistedItemGrant {
    owner_guid: u64,
    push: ItemPush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpectedPersistedMoneyGrant {
    owner_guid: u64,
    amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PersistedItemGrantRow {
    item_guid: u64,
    owner_guid: u64,
    item_entry: u32,
    count: u32,
    inventory_owner: Option<u64>,
    bag_guid: Option<u64>,
    slot: Option<u8>,
    bag_slot: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InventoryFailure {
    result: i32,
    item_0_low: u64,
    item_0_high: u64,
    item_1_low: u64,
    item_1_high: u64,
    container_b_slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MoneyNotify {
    money: u64,
    money_mod: u64,
    sole_looter: bool,
}

#[derive(Debug)]
struct LootRaceSync {
    logged_in: Barrier,
    party_ready: Barrier,
    positioned: Barrier,
    use_ready: Barrier,
    response_received: Barrier,
    windows_ready: Barrier,
    item_claim: Barrier,
    item_observed: Barrier,
    money_claim: Barrier,
    money_observed: Barrier,
    before_leave: Barrier,
    windows: Mutex<[Option<LootWindow>; 2]>,
    evidence: Mutex<[WireEvidence; 2]>,
    cancel_reason: StdMutex<Option<String>>,
    cancellation: CancellationToken,
    /// Complete live ObjectGuid observed on the wire. C++ includes realm bits
    /// in the high half, so reconstructing it from the SQL spawn/counter is
    /// not equivalent to preserving this value.
    runtime_guid: StdMutex<Option<(u64, u64)>>,
}

impl LootRaceSync {
    fn new() -> Self {
        Self {
            logged_in: Barrier::new(2),
            party_ready: Barrier::new(2),
            positioned: Barrier::new(2),
            use_ready: Barrier::new(2),
            response_received: Barrier::new(2),
            windows_ready: Barrier::new(2),
            item_claim: Barrier::new(2),
            item_observed: Barrier::new(2),
            money_claim: Barrier::new(2),
            money_observed: Barrier::new(2),
            before_leave: Barrier::new(2),
            windows: Mutex::new([None, None]),
            evidence: Mutex::new([WireEvidence::default(), WireEvidence::default()]),
            cancel_reason: StdMutex::new(None),
            cancellation: CancellationToken::new(),
            runtime_guid: StdMutex::new(None),
        }
    }

    fn cancel(&self, reason: impl Into<String>) {
        let reason = reason.into();
        if let Ok(mut stored) = self.cancel_reason.lock() {
            if stored.is_none() {
                *stored = Some(reason);
            }
        }
        self.cancellation.cancel();
    }

    fn cancellation_error(&self) -> Result<()> {
        if !self.cancellation.is_cancelled() {
            return Ok(());
        }
        let reason = self
            .cancel_reason
            .lock()
            .ok()
            .and_then(|reason| reason.clone())
            .unwrap_or_else(|| "peer failed without a recorded reason".to_string());
        bail!("loot-race cancelled because {reason}")
    }

    async fn cancelled(&self) {
        self.cancellation.cancelled().await;
    }
}

impl LootRaceOptions {
    fn resolved_runtime_guid(&self) -> Result<(u64, u64)> {
        self.sync
            .runtime_guid
            .lock()
            .map_err(|_| anyhow!("loot-race runtime GUID state was poisoned"))?
            .as_ref()
            .copied()
            .ok_or_else(|| {
                anyhow!(
                    "loot-race target entry {} spawn {} has no discovered live ObjectGuid",
                    self.target.entry,
                    self.target.spawn_guid
                )
            })
    }

    pub(super) fn resolved_runtime_counter(&self) -> Result<u64> {
        Ok(self.resolved_runtime_guid()?.0 & OBJECT_GUID_COUNTER_MASK)
    }

    fn resolved_packed_guid(&self) -> Result<Vec<u8>> {
        let (low, high) = self.resolved_runtime_guid()?;
        Ok(build_packed_guid(low, high))
    }
}

pub(super) fn validate_cli(
    race_enabled: bool,
    capture_enabled: bool,
    acknowledged: bool,
    bots: &[config::BotConfig],
    cli: &LootRaceCli,
) -> Result<()> {
    if race_enabled && capture_enabled {
        bail!("--loot-race-smoke and --loot-item-capture are separate workflows");
    }
    let enabled = race_enabled || capture_enabled;
    if !enabled {
        if acknowledged {
            bail!(
                "{} is only valid with --loot-race-smoke or --loot-item-capture",
                ACK_FLAG
            );
        }
        return Ok(());
    }
    if !acknowledged {
        bail!(
            "the selected loot workflow requires {}; this acknowledges consuming a disposable world-loot fixture whose live runtime cannot be restored without a world restart",
            ACK_FLAG
        );
    }
    if cli.account_a.eq_ignore_ascii_case(&cli.account_b) {
        bail!("loot-race account A and B must be different");
    }
    if bots.len() != 2 {
        bail!(
            "the guarded loot fixture requires exactly the two selected configured bots; capture mode logs in only account A and keeps account B offline"
        );
    }
    for expected in [&cli.account_a, &cli.account_b] {
        if !bots
            .iter()
            .any(|bot| bot.account.eq_ignore_ascii_case(expected))
        {
            bail!("loot-race account `{expected}` is not an enabled configured bot");
        }
    }
    if cli.entry == 0 || cli.spawn_guid == 0 || cli.item_entry == 0 {
        bail!("loot-race target entry/spawn/item entry must all be nonzero");
    }
    if race_enabled
        && (cli.entry, cli.spawn_guid, cli.item_entry)
            != (
                DEFAULT_CREATURE_ENTRY,
                DEFAULT_CREATURE_SPAWN_GUID,
                DEFAULT_ITEM_ENTRY,
            )
    {
        bail!(
            "the two-client race is pinned to wrapper-owned Tattered Chest entry/spawn/item {}/{}/{}; custom race fixtures are not cancel-safe",
            DEFAULT_CREATURE_ENTRY,
            DEFAULT_CREATURE_SPAWN_GUID,
            DEFAULT_ITEM_ENTRY
        );
    }
    if cli.runtime_counter & !OBJECT_GUID_COUNTER_MASK != 0 {
        bail!("loot-race runtime counter exceeds the 40-bit ObjectGuid counter field");
    }
    if cli.timeout_secs == 0 {
        bail!("--loot-race-timeout must be greater than zero");
    }
    if cli.workflow_deadline_secs == 0 || cli.workflow_deadline_secs <= cli.timeout_secs {
        bail!("--loot-workflow-deadline must be greater than the per-phase loot timeout");
    }
    Ok(())
}

pub(super) async fn run_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: LootRaceCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let workflow_deadline =
        tokio::time::Instant::now() + Duration::from_secs(cli.workflow_deadline_secs);
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled before fixture setup");
    }
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&cli.account_a) {
            0
        } else {
            1
        }
    });
    let setup_bots = bots.clone();
    let setup_cli = cli.clone();
    let setup_shutdown = shutdown.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_fixture(
            &setup_bots,
            &setup_cli,
            LootFixturePurpose::Race,
            &setup_shutdown,
        )
    })
    .await
    .map_err(|error| anyhow!("loot-race fixture DB worker join failed: {error}"))??;

    let sync = Arc::new(LootRaceSync::new());
    let mut handles = tokio::task::JoinSet::new();
    for participant in 0..2 {
        let bot = bots[participant].clone();
        let options = LootRaceOptions {
            phase: LootRacePhase::Race,
            participant,
            character_guid: fixture.characters[participant].bot.character_guid,
            peer_name: fixture.characters[1 - participant].name.clone(),
            peer_character_guid: fixture.characters[1 - participant].bot.character_guid,
            killer_character_guid: fixture.characters[0].bot.character_guid,
            target: fixture.target.clone(),
            timeout_secs: cli.timeout_secs,
            sync: Arc::clone(&sync),
        };
        let task_sync = Arc::clone(&sync);
        handles.spawn(async move {
            let run = run_bot(
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
                None,
                Some(options),
                None,
                None,
                None,
            )
            .await;
            match &run {
                Err(error) => task_sync.cancel(format!(
                    "participant {participant} transport/login failed: {error:#}"
                )),
                Ok(result) if result.loot_race_smoke_passed == Some(false) => {
                    task_sync.cancel(format!(
                        "participant {participant} failed: {}",
                        result
                            .loot_race_failure
                            .as_deref()
                            .unwrap_or("unknown loot-race failure")
                    ));
                }
                _ => {}
            }
            run
        });
    }

    let mut results = Vec::with_capacity(2);
    let mut task_error = None;
    loop {
        let joined = tokio::select! {
            joined = handles.join_next() => joined,
            _ = shutdown.cancelled() => {
                let message = "loot-race received SIGINT/SIGTERM".to_string();
                sync.cancel(message.clone());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                task_error = Some(message);
                break;
            }
            _ = tokio::time::sleep_until(workflow_deadline) => {
                let message = format!(
                    "loot-race exceeded the {}s end-to-end deadline",
                    cli.workflow_deadline_secs
                );
                sync.cancel(message.clone());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                task_error = Some(message);
                break;
            }
        };
        let Some(joined) = joined else { break };
        match joined {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(error)) => {
                let message = error.to_string();
                sync.cancel(message.clone());
                handles.abort_all();
                task_error.get_or_insert(message);
            }
            Err(error) => {
                let message = format!("loot-race task join failed: {error}");
                sync.cancel(message.clone());
                handles.abort_all();
                task_error.get_or_insert(message);
            }
        }
    }
    results.sort_by_key(|result| result.account_id);
    if shutdown.is_cancelled() {
        task_error.get_or_insert_with(|| "loot-race received SIGINT/SIGTERM".to_string());
    } else if tokio::time::Instant::now() >= workflow_deadline {
        task_error.get_or_insert_with(|| {
            format!(
                "loot-race exceeded the {}s end-to-end deadline",
                cli.workflow_deadline_secs
            )
        });
    }

    if task_error.is_none()
        && results.len() == 2
        && results
            .iter()
            .all(|result| result.loot_race_smoke_passed == Some(true))
    {
        let expected_source_coins = results[0]
            .loot_race_loot_coins
            .map(u64::from)
            .ok_or_else(|| anyhow!("loot-race passed without recording source money"));
        let expected_item_grant = expected_persisted_item_grant(&fixture, &sync).await;
        let expected_money_grant = match expected_source_coins.as_ref() {
            Ok(source_coins) => {
                expected_persisted_money_grant(&fixture, &sync, *source_coins).await
            }
            Err(error) => Err(anyhow!(error.to_string())),
        };
        let verification = match (
            expected_source_coins,
            expected_item_grant,
            expected_money_grant,
        ) {
            (Ok(expected_source_coins), Ok(expected_item_grant), Ok(expected_money_grant)) => {
                if expected_source_coins != expected_money_grant.amount {
                    Err(anyhow!(
                        "wire money winner amount {} did not consume exact source pool {expected_source_coins}",
                        expected_money_grant.amount
                    ))
                } else {
                    let fixture_for_db = fixture.clone();
                    match tokio::task::spawn_blocking(move || {
                        verify_persisted_grants(
                            &fixture_for_db,
                            expected_money_grant,
                            expected_item_grant,
                        )
                    })
                    .await
                    {
                        Ok(verification) => verification,
                        Err(error) => Err(anyhow!(
                            "loot-race verification DB worker join failed: {error}"
                        )),
                    }
                }
            }
            (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => Err(error),
        };
        match verification {
            Ok((item_total, money_delta, item_grant, money_grant)) => {
                for result in &mut results {
                    result.loot_race_db_item_total = Some(item_total);
                    result.loot_race_db_money_delta = Some(money_delta);
                }
                let mut relog_ok = true;
                for participant in 0..2 {
                    let options = LootRaceOptions {
                        phase: LootRacePhase::VerifyRelog,
                        participant,
                        character_guid: fixture.characters[participant].bot.character_guid,
                        peer_name: fixture.characters[1 - participant].name.clone(),
                        peer_character_guid: fixture.characters[1 - participant].bot.character_guid,
                        killer_character_guid: fixture.characters[0].bot.character_guid,
                        target: fixture.target.clone(),
                        timeout_secs: cli.timeout_secs,
                        sync: Arc::clone(&sync),
                    };
                    let relog = tokio::select! {
                        _ = shutdown.cancelled() => {
                            Err(anyhow!("loot-race relog cancelled by SIGINT/SIGTERM"))
                        }
                        result = tokio::time::timeout_at(workflow_deadline, run_bot(
                            bots[participant].clone(),
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
                            Some(options),
                            None,
                            None,
                            None,
                        )) => match result {
                            Ok(run) => run,
                            Err(_) => Err(anyhow!(
                                "loot-race relog exceeded the {}s end-to-end deadline",
                                cli.workflow_deadline_secs
                            )),
                        },
                    };
                    match relog {
                        Ok(relog) if relog.loot_race_relog_verified => {
                            if let Some(result) = results
                                .iter_mut()
                                .find(|result| result.account_id == bots[participant].account_id)
                            {
                                result.loot_race_relog_verified = true;
                            }
                        }
                        Ok(_) | Err(_) => relog_ok = false,
                    }
                }
                let fixture_for_relog = fixture.clone();
                let after_relog = match tokio::task::spawn_blocking(move || {
                    verify_persisted_grants(&fixture_for_relog, money_grant, item_grant)
                })
                .await
                {
                    Ok(verification) => verification,
                    Err(error) => Err(anyhow!("loot-race relog DB worker join failed: {error}")),
                };
                if !matches!(
                    after_relog,
                    Ok(persisted) if persisted == (item_total, money_delta, item_grant, money_grant)
                ) {
                    relog_ok = false;
                }
                for result in &mut results {
                    result.loot_race_smoke_passed = Some(
                        result.loot_race_smoke_passed.unwrap_or(false)
                            && relog_ok
                            && result.loot_race_relog_verified,
                    );
                    if !result.loot_race_smoke_passed.unwrap_or(false)
                        && result.loot_race_failure.is_none()
                    {
                        result.loot_race_failure = Some(
                            "loot grants did not survive a clean logout/relogin unchanged".into(),
                        );
                    }
                }
            }
            Err(error) => {
                for result in &mut results {
                    result.loot_race_smoke_passed = Some(false);
                    result.loot_race_failure = Some(error.to_string());
                }
            }
        }
    }

    if let Some(ref error) = task_error {
        for result in &mut results {
            result.loot_race_smoke_passed = Some(false);
            result.loot_race_failure.get_or_insert(error.clone());
        }
    }

    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || cleanup_fixture(&fixture_for_cleanup))
        .await
        .map_err(|error| anyhow!("loot-race cleanup DB worker join failed: {error}"))?;
    if let Err(error) = cleanup {
        bail!("loot-race fixture cleanup failed: {error:#}");
    }
    if results.len() != 2 {
        bail!(
            "loot-race produced {} results instead of 2{}",
            results.len(),
            task_error
                .as_deref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        );
    }
    Ok(results)
}

/// Record one deterministic, item-only loot action with exactly one connected
/// client. The second guarded bot remains offline; it is retained only because
/// the shared disposable-fixture snapshot/cleanup predates this capture mode.
pub(super) async fn run_single_item_capture_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: LootRaceCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let workflow_deadline =
        tokio::time::Instant::now() + Duration::from_secs(cli.workflow_deadline_secs);
    if shutdown.is_cancelled() {
        bail!("loot-item capture cancelled before fixture setup");
    }
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&cli.account_a) {
            0
        } else {
            1
        }
    });
    let setup_bots = bots.clone();
    let setup_cli = cli.clone();
    let setup_shutdown = shutdown.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_fixture(
            &setup_bots,
            &setup_cli,
            LootFixturePurpose::CaptureItem,
            &setup_shutdown,
        )
    })
    .await
    .map_err(|error| anyhow!("loot-item capture fixture DB worker join failed: {error}"))??;

    let sync = Arc::new(LootRaceSync::new());
    let options = LootRaceOptions {
        phase: LootRacePhase::CaptureItem,
        participant: 0,
        character_guid: fixture.characters[0].bot.character_guid,
        peer_name: fixture.characters[1].name.clone(),
        peer_character_guid: fixture.characters[1].bot.character_guid,
        killer_character_guid: fixture.characters[0].bot.character_guid,
        target: fixture.target.clone(),
        timeout_secs: cli.timeout_secs,
        sync: Arc::clone(&sync),
    };
    let run = tokio::select! {
        _ = shutdown.cancelled() => Err(anyhow!("loot-item capture received SIGINT/SIGTERM")),
        result = tokio::time::timeout_at(workflow_deadline, run_bot(
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
            Some(options),
            None,
            None,
            None,
        )) => match result {
            Ok(run) => run,
            Err(_) => Err(anyhow!(
                "loot-item capture exceeded the {}s end-to-end deadline",
                cli.workflow_deadline_secs
            )),
        },
    };

    let mut result = match run {
        Ok(result) => Some(result),
        Err(error) => {
            let fixture_for_cleanup = fixture.clone();
            let cleanup =
                tokio::task::spawn_blocking(move || cleanup_fixture(&fixture_for_cleanup))
                    .await
                    .map_err(|join| {
                        anyhow!("loot-item capture cleanup DB worker join failed: {join}")
                    })?;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "loot-item capture failed ({error:#}) and fixture cleanup also failed ({cleanup_error:#})"
                );
            }
            return Err(error);
        }
    };

    if result
        .as_ref()
        .is_some_and(|result| result.loot_race_smoke_passed == Some(true))
    {
        let fixture_for_verification = fixture.clone();
        match tokio::task::spawn_blocking(move || {
            verify_single_item_capture_persistence(&fixture_for_verification)
        })
        .await
        {
            Ok(Ok(item_total)) => {
                let result = result
                    .as_mut()
                    .expect("successful wire result remains available");
                result.loot_race_db_item_total = Some(item_total);
                result.loot_race_db_money_delta = Some(0);
            }
            Ok(Err(error)) => {
                let result = result
                    .as_mut()
                    .expect("successful wire result remains available");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(error.to_string());
            }
            Err(error) => {
                let result = result
                    .as_mut()
                    .expect("successful wire result remains available");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(format!(
                    "loot-item capture verification DB worker join failed: {error}"
                ));
            }
        }
    }

    if result
        .as_ref()
        .is_some_and(|result| result.loot_race_smoke_passed == Some(true))
    {
        let relog_options = LootRaceOptions {
            phase: LootRacePhase::VerifyRelog,
            participant: 0,
            character_guid: fixture.characters[0].bot.character_guid,
            peer_name: fixture.characters[1].name.clone(),
            peer_character_guid: fixture.characters[1].bot.character_guid,
            killer_character_guid: fixture.characters[0].bot.character_guid,
            target: fixture.target.clone(),
            timeout_secs: cli.timeout_secs,
            sync,
        };
        let relog = tokio::select! {
            _ = shutdown.cancelled() => {
                Err(anyhow!("loot-item capture relog cancelled by SIGINT/SIGTERM"))
            }
            relog = tokio::time::timeout_at(workflow_deadline, run_bot(
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
                Some(relog_options),
                None,
                None,
                None,
            )) => match relog {
                Ok(relog) => relog,
                Err(_) => Err(anyhow!(
                    "loot-item capture relog exceeded the {}s end-to-end deadline",
                    cli.workflow_deadline_secs
                )),
            },
        };
        match relog {
            Ok(relog) if relog.loot_race_relog_verified => {
                let fixture_for_verification = fixture.clone();
                let persisted_after_relog = tokio::task::spawn_blocking(move || {
                    verify_single_item_capture_persistence(&fixture_for_verification)
                })
                .await;
                let result = result
                    .as_mut()
                    .expect("successful capture result remains available for relog merge");
                result.world_auth &= relog.world_auth;
                result.enum_characters &= relog.enum_characters;
                result.player_login_verified &= relog.player_login_verified;
                result.seen_opcodes.extend(relog.seen_opcodes);
                match persisted_after_relog {
                    Ok(Ok(item_total)) if Some(item_total) == result.loot_race_db_item_total => {
                        result.loot_race_relog_verified = true;
                    }
                    Ok(Ok(item_total)) => {
                        result.loot_race_smoke_passed = Some(false);
                        result.loot_race_failure = Some(format!(
                            "loot-item capture persisted item total changed across relog: before {:?}, after {item_total}",
                            result.loot_race_db_item_total
                        ));
                    }
                    Ok(Err(error)) => {
                        result.loot_race_smoke_passed = Some(false);
                        result.loot_race_failure = Some(format!(
                            "loot-item capture post-relog persistence verification failed: {error}"
                        ));
                    }
                    Err(error) => {
                        result.loot_race_smoke_passed = Some(false);
                        result.loot_race_failure = Some(format!(
                            "loot-item capture post-relog DB worker join failed: {error}"
                        ));
                    }
                }
            }
            Ok(_) => {
                let result = result
                    .as_mut()
                    .expect("successful capture result remains available for relog failure");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(
                    "loot-item capture relog did not verify a clean logout/login cycle".into(),
                );
            }
            Err(error) => {
                let result = result
                    .as_mut()
                    .expect("successful capture result remains available for relog error");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(error.to_string());
            }
        }
    }

    let deadline_exceeded_before_cleanup = tokio::time::Instant::now() >= workflow_deadline;
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || cleanup_fixture(&fixture_for_cleanup))
        .await
        .map_err(|error| anyhow!("loot-item capture cleanup DB worker join failed: {error}"))?;
    if let Err(error) = cleanup {
        bail!("loot-item capture fixture cleanup failed: {error:#}");
    }

    let workflow_failure = if shutdown.is_cancelled() {
        Some("loot-item capture was cancelled before end-to-end verification completed")
    } else if deadline_exceeded_before_cleanup {
        Some("loot-item capture exceeded its end-to-end deadline during final verification")
    } else {
        None
    };
    if let Some(failure) = workflow_failure {
        let result = result
            .as_mut()
            .expect("successful bot run remains available after cleanup");
        result.loot_race_smoke_passed = Some(false);
        result
            .loot_race_failure
            .get_or_insert_with(|| failure.to_owned());
    }

    Ok(vec![result.expect("successful bot run remains available")])
}

fn record_discovered_runtime_guid(options: &LootRaceOptions, candidate: (u64, u64)) -> Result<u64> {
    let (low, high) = candidate;
    let counter = low & OBJECT_GUID_COUNTER_MASK;
    if counter == 0 {
        bail!(
            "loot-race discovered an empty runtime counter for entry {} spawn {}",
            options.target.entry,
            options.target.spawn_guid
        );
    }
    let high_type = (high >> 58) & GUID_HIGH_TYPE_MASK;
    let expected_high_type = match options.target.kind {
        LootRaceTargetKind::Creature => HIGH_GUID_CREATURE,
        LootRaceTargetKind::GameObject => HIGH_GUID_GAMEOBJECT,
    };
    let high_map = (high >> 29) & GUID_MAP_MASK;
    let high_entry = (high >> 6) & GUID_ENTRY_MASK;
    if high_type != expected_high_type
        || high_map != u64::from(options.target.map_id)
        || high_entry != u64::from(options.target.entry)
    {
        bail!(
            "loot-race discovered malformed {:?} ObjectGuid {low:#018X}/{high:#018X} for entry {} map {}",
            options.target.kind,
            options.target.entry,
            options.target.map_id
        );
    }
    let configured = options.target.runtime_counter_override;
    if configured != 0 && counter != configured {
        bail!(
            "loot-race runtime counter override {configured} did not match discovered counter {counter} for SQL spawn {}",
            options.target.spawn_guid
        );
    }

    let mut resolved = options
        .sync
        .runtime_guid
        .lock()
        .map_err(|_| anyhow!("loot-race runtime GUID state was poisoned"))?;
    if let Some(previous) = *resolved {
        if previous != candidate {
            bail!(
                "loot-race bots discovered different live ObjectGuids for exact SQL spawn {}: first={:#018X}/{:#018X}, current={low:#018X}/{high:#018X}",
                options.target.spawn_guid,
                previous.0,
                previous.1
            );
        }
    } else {
        *resolved = Some(candidate);
    }
    Ok(counter)
}

pub(super) fn target_seen_in_update(
    options: &LootRaceOptions,
    opcode: u16,
    payload: &[u8],
) -> Result<Option<u64>> {
    if options.phase == LootRacePhase::VerifyRelog || opcode != SMSG_UPDATE_OBJECT {
        return Ok(None);
    }
    // Keep the complete GUID seen on the wire: C++ includes realm/server bits
    // that must not be reconstructed from the SQL spawn id. C++
    // `GameObject::LoadFromDB` stores the SQL spawn id in `m_spawnId`, while
    // `GameObject::Create` builds the live ObjectGuid with the map-local
    // `GenerateLowGuid<HighGuid::GameObject>()` counter. The guarded DB
    // preflight therefore proves entry/map uniqueness and wire discovery owns
    // the independent runtime counter.
    let Some(candidate) = find_loot_target_guid_in_update_object(payload, &options.target)? else {
        return Ok(None);
    };
    record_discovered_runtime_guid(options, candidate).map(Some)
}

fn find_loot_target_guid_in_update_object(
    payload: &[u8],
    target: &LootRaceTarget,
) -> Result<Option<(u64, u64)>> {
    let expected_high_type = match target.kind {
        LootRaceTargetKind::Creature => HIGH_GUID_CREATURE,
        LootRaceTargetKind::GameObject => HIGH_GUID_GAMEOBJECT,
    };
    let mut candidates = std::collections::BTreeSet::new();
    for offset in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[offset], 1 | 2) {
            continue;
        }
        let Some((_, low, high)) = parse_packed_guid(&payload[offset + 1..]) else {
            continue;
        };
        if ((high >> 58) & GUID_HIGH_TYPE_MASK) != expected_high_type
            || ((high >> 29) & GUID_MAP_MASK) != u64::from(target.map_id)
            || ((high >> 6) & GUID_ENTRY_MASK) != u64::from(target.entry)
        {
            continue;
        }
        candidates.insert((low, high));
    }

    if candidates.len() > 1 {
        bail!(
            "loot-race update contained {} distinct live ObjectGuid candidates for {:?} entry {} map {}: {candidates:?}",
            candidates.len(),
            target.kind,
            target.entry,
            target.map_id
        );
    }
    Ok(candidates.into_iter().next())
}

pub(super) async fn run_phase(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &LootRaceOptions,
    target_seen: bool,
    result: &mut BotRunResult,
) -> Result<()> {
    let run = run_phase_inner(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection,
        options,
        target_seen,
        result,
    )
    .await;
    if let Err(error) = &run {
        options.sync.cancel(format!(
            "participant {} phase {:?} failed: {error:#}",
            options.participant, options.phase
        ));
    }
    run
}

async fn run_phase_inner(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &LootRaceOptions,
    mut target_seen: bool,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.phase == LootRacePhase::VerifyRelog {
        result.loot_race_target_runtime_counter = Some(options.resolved_runtime_counter()?);
        logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            inflater,
            realm_connection.as_mut(),
            options.character_guid,
            result,
        )
        .await?;
        result.loot_race_relog_verified = true;
        result.loot_race_smoke_passed = Some(true);
        return Ok(());
    }
    let realm = realm_connection
        .as_mut()
        .ok_or_else(|| anyhow!("loot-race requires separate realm and instance sockets"))?;

    // C++ can defer nearby-object visibility until the client confirms that its
    // active mover is initialized. Reuse the same real movement handshake as
    // the rested-XP combat smoke before proving the exact creature is live.
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &build_move_init_active_mover_complete_payload(0),
    )
    .await?;
    target_seen |=
        drain_until_target_or_quiet(bot_index, stream, crypt, inflater, realm, options, result)
            .await?;
    if !target_seen {
        let override_detail = if options.target.runtime_counter_override == 0 {
            "auto-discovery".to_string()
        } else {
            format!("override {}", options.target.runtime_counter_override)
        };
        bail!(
            "world-loot target entry {} spawn {} ({override_detail}) was not present in SMSG_UPDATE_OBJECT; require a fresh world/runtime and matching override",
            options.target.entry,
            options.target.spawn_guid
        );
    }
    result.loot_race_target_runtime_counter = Some(options.resolved_runtime_counter()?);
    result.loot_race_target_discovered = true;
    if options.phase == LootRacePhase::CaptureItem {
        return run_single_item_capture_phase(
            bot_index, stream, crypt, inflater, realm, options, result,
        )
        .await;
    }
    options.sync.cancellation_error()?;
    wait_phase(options, &options.sync.logged_in, "both bots logged in").await?;

    form_party(bot_index, stream, crypt, inflater, realm, options, result).await?;
    result.loot_race_party_confirmed = true;
    wait_phase(options, &options.sync.party_ready, "PERSONAL party formed").await?;

    let (player_low, player_high) = create_player_guid_raw(options.character_guid, realm_id());
    let player_x = options.target.x + 1.0 + options.participant as f64;
    let player_y = options.target.y;
    let player_z = options.target.z;
    let player_orientation =
        (options.target.y - player_y).atan2(options.target.x - player_x) as f32;
    let movement = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x as f32,
        player_y as f32,
        player_z as f32,
        player_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &movement).await?;
    wait_phase(
        options,
        &options.sync.positioned,
        "both bots positioned at the shared chest",
    )
    .await?;

    if options.target.kind != LootRaceTargetKind::GameObject {
        bail!("two-client Race must use the guarded shared GameObject fixture");
    }
    wait_phase(
        options,
        &options.sync.use_ready,
        "simultaneous CMSG_GAME_OBJ_USE",
    )
    .await?;
    // C++ `WorldPackets::GameObject::GameObjUse::Read` consumes one packed
    // GameObject ObjectGuid; both clients deliberately send the same wire GUID.
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_GAME_OBJ_USE,
        &options.resolved_packed_guid()?,
    )
    .await?;
    info!(
        "[Bot {}] loot-race phase: CMSG_GAME_OBJ_USE sent for shared spawn {}",
        bot_index, options.target.spawn_guid
    );
    let window =
        wait_for_loot_window(bot_index, stream, crypt, inflater, realm, options, result).await?;
    result.loot_race_loot_opened = true;
    result.loot_race_loot_list_id = Some(window.loot_list_id);
    result.loot_race_loot_coins = Some(window.coins);
    info!(
        "[Bot {}] loot-race phase: SMSG_LOOT_RESPONSE received (loot_list_id={}, coins={})",
        bot_index, window.loot_list_id, window.coins
    );
    options.sync.windows.lock().await[options.participant] = Some(window.clone());
    wait_phase(
        options,
        &options.sync.response_received,
        "both SMSG_LOOT_RESPONSE packets received",
    )
    .await?;
    wait_phase(
        options,
        &options.sync.windows_ready,
        "shared loot windows recorded",
    )
    .await?;
    validate_shared_windows(options).await?;

    wait_phase(options, &options.sync.item_claim, "simultaneous item claim").await?;
    let item_claim = build_loot_item_claim(&window);
    send_encrypted_packet(stream, crypt, CMSG_LOOT_ITEM, &item_claim).await?;
    let expected_loot_owner = options.resolved_runtime_guid()?;
    let mut evidence = collect_evidence(
        bot_index,
        stream,
        crypt,
        inflater,
        realm,
        expected_loot_owner,
        RESPONSE_SETTLE,
        options,
        result,
    )
    .await?;
    options.sync.evidence.lock().await[options.participant] = evidence.clone();
    wait_phase(
        options,
        &options.sync.item_observed,
        "item outcomes observed",
    )
    .await?;
    validate_item_outcome(options, result).await?;

    wait_phase(
        options,
        &options.sync.money_claim,
        "simultaneous money claim",
    )
    .await?;
    send_encrypted_packet(stream, crypt, CMSG_LOOT_MONEY, &[0]).await?;
    evidence.merge(
        collect_evidence(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            expected_loot_owner,
            RESPONSE_SETTLE,
            options,
            result,
        )
        .await?,
    );
    options.sync.evidence.lock().await[options.participant] = evidence;
    wait_phase(
        options,
        &options.sync.money_observed,
        "money outcomes observed",
    )
    .await?;
    // The money settle window can also surface a delayed item response. Merge
    // first, then re-run the exact item fanout proof so a late duplicate,
    // foreign push/failure, or extra removal cannot escape the earlier fence.
    validate_item_outcome(options, result).await?;
    validate_money_outcome(options, result).await?;

    wait_phase(options, &options.sync.before_leave, "party cleanup").await?;
    send_encrypted_packet(&mut realm.stream, &mut realm.crypt, CMSG_LEAVE_GROUP, &[0]).await?;
    logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        Some(realm),
        options.character_guid,
        result,
    )
    .await?;
    result.loot_race_smoke_passed = Some(true);
    Ok(())
}

async fn run_single_item_capture_phase(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.participant != 0 || options.character_guid != options.killer_character_guid {
        bail!("loot-item capture requires account A as its sole connected killer");
    }

    let (player_low, player_high) = create_player_guid_raw(options.character_guid, realm_id());
    let player_x = options.target.x + 1.0;
    let player_y = options.target.y;
    let player_z = options.target.z;
    let player_orientation =
        (options.target.y - player_y).atan2(options.target.x - player_x) as f32;
    let movement = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x as f32,
        player_y as f32,
        player_z as f32,
        player_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &movement).await?;

    kill_target_once(bot_index, stream, crypt, inflater, realm, options, result).await?;
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_LOOT_UNIT,
        &options.resolved_packed_guid()?,
    )
    .await?;
    let window =
        wait_for_loot_window(bot_index, stream, crypt, inflater, realm, options, result).await?;
    result.loot_race_loot_opened = true;
    result.loot_race_loot_list_id = Some(window.loot_list_id);
    result.loot_race_loot_coins = Some(window.coins);

    // Capture-diff starts at this exact CMSG. Opening the corpse (including its
    // random coin roll) is deliberately outside the item-only window.
    let item_claim = build_loot_item_claim(&window);
    send_encrypted_packet(stream, crypt, CMSG_LOOT_ITEM, &item_claim).await?;
    let evidence = collect_single_item_capture_evidence(
        bot_index,
        stream,
        crypt,
        inflater,
        realm,
        options.resolved_runtime_guid()?,
        options.timeout_secs,
        result,
    )
    .await?;
    let expected_player = create_player_guid_raw(options.character_guid, realm_id());
    validate_single_item_capture_evidence(
        &evidence,
        expected_player,
        options.target.item_entry,
        &window,
    )?;
    result.loot_race_item_push_seen = true;
    result.loot_race_loot_removed_seen = true;

    send_and_verify_loot_item_capture_fence(
        bot_index,
        stream,
        crypt,
        inflater,
        options.timeout_secs,
        result,
    )
    .await?;
    logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        Some(realm),
        options.character_guid,
        result,
    )
    .await?;
    result.loot_race_smoke_passed = Some(true);
    Ok(())
}

/// Read only until the two C++-anchored item-claim responses have arrived.
///
/// The two-client race deliberately keeps a settle window so it can prove the
/// absence/presence of competing outcomes on both sockets.  A capture golden
/// has a different requirement: put the fixed ping fence immediately after
/// the one `LootRemoved` and one `ItemPushResult`, rather than admitting an
/// arbitrary two seconds of periodic traffic into the strict diff window.
async fn collect_single_item_capture_evidence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    expected_loot_owner: (u64, u64),
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<WireEvidence> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut evidence = WireEvidence::default();

    loop {
        if evidence.item_pushes.len() == 1 && evidence.loot_removed.len() == 1 {
            return Ok(evidence);
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for the capture item responses (item pushes={}, loot removals={})",
                evidence.item_pushes.len(),
                evidence.loot_removed.len()
            );
        }

        // Do not `select!` two in-progress encrypted reads: cancelling the
        // losing `read_exact` could consume part of a framed packet.  Poll
        // socket readiness briefly, then finish any selected frame without
        // cancellation.
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(5)),
            remaining,
            "loot-item capture realm response",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
            validate_single_item_capture_candidate(&evidence)?;
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            continue;
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "loot-item capture instance response",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
            validate_single_item_capture_candidate(&evidence)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}

fn validate_single_item_capture_candidate(evidence: &WireEvidence) -> Result<()> {
    if evidence.item_pushes.len() > 1 || evidence.loot_removed.len() > 1 {
        bail!(
            "loot-item capture observed duplicate functional responses before its fence (item pushes={}, loot removals={})",
            evidence.item_pushes.len(),
            evidence.loot_removed.len()
        );
    }
    if !evidence.inventory_failures.is_empty() {
        bail!("loot-item capture observed an inventory failure before its success fence");
    }
    if !evidence.money_notifies.is_empty() || !evidence.coin_removed.is_empty() {
        bail!("loot-item capture observed money-claim traffic before its item-only fence");
    }
    Ok(())
}

fn validate_single_item_capture_evidence(
    evidence: &WireEvidence,
    expected_player: (u64, u64),
    expected_item_entry: u32,
    window: &LootWindow,
) -> Result<()> {
    if evidence.item_pushes.len() != 1 {
        bail!(
            "single-session item capture observed item pushes {:?}; expected exactly one",
            evidence.item_pushes
        );
    }
    let push = &evidence.item_pushes[0];
    if push.item_entry != expected_item_entry
        || push.quantity != 1
        || push.quantity_in_inventory != 1
        || push.slot != INVENTORY_SLOT_BAG_0
        || push.slot_in_bag != i32::from(LOOT_ITEM_CAPTURE_KEYRING_SLOT)
        || (push.player_low, push.player_high) != expected_player
    {
        bail!(
            "single-session item capture observed item pushes {:?}; expected exactly item {} quantity 1 in keyring slot {}/{} for the sole character",
            evidence.item_pushes,
            expected_item_entry,
            INVENTORY_SLOT_BAG_0,
            LOOT_ITEM_CAPTURE_KEYRING_SLOT
        );
    }
    let expected_removal = LootRemovedEvidence {
        owner_low: window.owner_low,
        owner_high: window.owner_high,
        loot_low: window.loot_low,
        loot_high: window.loot_high,
        loot_list_id: window.loot_list_id,
    };
    if evidence.loot_removed != [expected_removal] {
        bail!(
            "single-session item capture observed removals {:?}; expected exactly {:?}",
            evidence.loot_removed,
            expected_removal
        );
    }
    if !evidence.inventory_failures.is_empty() {
        bail!("single-session item capture unexpectedly failed inventory storage");
    }
    if !evidence.money_notifies.is_empty() || !evidence.coin_removed.is_empty() {
        bail!("item-only capture unexpectedly emitted money-claim packets");
    }
    Ok(())
}

async fn send_and_verify_loot_item_capture_fence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let payload = build_ping_payload(LOOT_ITEM_CAPTURE_FENCE_SERIAL);
    send_encrypted_packet(stream, crypt, CMSG_PING, &payload).await?;
    info!(
        "[Bot {}] deterministic loot-item CMSG_PING fence sent (serial=0x{:08X})",
        bot_index, LOOT_ITEM_CAPTURE_FENCE_SERIAL
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for loot-item capture-fence SMSG_PONG");
        }
        let (opcode, payload) =
            tokio::time::timeout(remaining, read_encrypted_packet(stream, crypt, inflater))
                .await
                .map_err(|_| {
                    anyhow!("timed out waiting for loot-item capture-fence SMSG_PONG")
                })??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_PONG {
            if payload != LOOT_ITEM_CAPTURE_FENCE_SERIAL.to_le_bytes() {
                bail!(
                    "loot-item capture-fence SMSG_PONG mismatch: expected 0x{:08X}, got {:02X?}",
                    LOOT_ITEM_CAPTURE_FENCE_SERIAL,
                    payload
                );
            }
            return Ok(());
        }
        handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
    }
}

fn logout_completion_route(
    opcode: u16,
    payload: &[u8],
    route: LogoutCompletionRoute,
) -> Result<Option<LogoutCompletionRoute>> {
    if opcode != SMSG_LOGOUT_COMPLETE {
        return Ok(None);
    }
    if !payload.is_empty() {
        bail!(
            "SMSG_LOGOUT_COMPLETE carried {} bytes; C++ 3.4.3 writes an empty body",
            payload.len()
        );
    }
    Ok(Some(route))
}

fn wait_for_loot_character_offline(character_guid: u64, timeout: Duration) -> Result<()> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let online: u8 = conn
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (character_guid,),
            )
            .map_err(|error| anyhow!("Check loot logout offline state: {error}"))?
            .ok_or_else(|| anyhow!("Loot character {character_guid} disappeared during logout"))?;
        if online == 0 {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "loot character {character_guid} remained online after its bounded logout DB proof"
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Finish a loot workflow across the real 3.4.3 socket topology.
///
/// C++ routes `SMSG_LOGOUT_RESPONSE` on instance and
/// `SMSG_LOGOUT_COMPLETE` on realm. Rust currently may complete on instance,
/// so both sockets are drained safely and the observed route is reported. In
/// every case, success additionally requires the exact character row to be
/// offline; a closed socket by itself is never accepted as logout proof.
pub(super) async fn logout_and_wait_routed_like_cpp(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    mut realm_connection: Option<&mut EncryptedWorldConnection>,
    character_guid: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    send_encrypted_packet(stream, crypt, CMSG_LOGOUT_REQUEST, &[0]).await?;
    info!("[Bot {}] ✅ CMSG_LOGOUT_REQUEST sent", bot_index);

    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(NORMAL_LOGOUT_COMPLETE_WAIT_SECS);
    let mut instance_open = true;
    let mut realm_open = realm_connection.is_some();
    let mut completion_route = None;
    let mut transport_errors = Vec::new();

    while tokio::time::Instant::now() < deadline && completion_route.is_none() {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        if !realm_open && !instance_open {
            tokio::time::sleep(remaining).await;
            break;
        }

        if realm_open {
            let realm = realm_connection
                .as_deref_mut()
                .expect("realm-open state requires the preserved realm connection");
            match read_encrypted_packet_if_ready(
                &mut realm.stream,
                &mut realm.crypt,
                &mut realm.inflater,
                remaining.min(Duration::from_millis(50)),
                remaining,
                "loot logout realm packet",
            )
            .await
            {
                Ok(Some((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    info!(
                        "[Bot {}] 📦 realm loot logout {}",
                        bot_index,
                        parse_packet(opcode, &payload)
                    );
                    completion_route =
                        logout_completion_route(opcode, &payload, LogoutCompletionRoute::Realm)?;
                }
                Ok(None) => {}
                Err(error) => {
                    realm_open = false;
                    transport_errors.push(format!("realm: {error}"));
                }
            }
        }

        if completion_route.is_some() {
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        if instance_open {
            let readiness_wait = if realm_open {
                Duration::from_millis(1)
            } else {
                remaining.min(Duration::from_millis(50))
            };
            match read_encrypted_packet_if_ready(
                stream,
                crypt,
                inflater,
                readiness_wait,
                remaining,
                "loot logout instance packet",
            )
            .await
            {
                Ok(Some((opcode, payload))) => {
                    result.seen_opcodes.push(format!("0x{opcode:04X}"));
                    info!(
                        "[Bot {}] 📦 instance loot logout {}",
                        bot_index,
                        parse_packet(opcode, &payload)
                    );
                    completion_route =
                        logout_completion_route(opcode, &payload, LogoutCompletionRoute::Instance)?;
                    if completion_route.is_none() {
                        handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload)
                            .await?;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    instance_open = false;
                    transport_errors.push(format!("instance: {error}"));
                }
            }
        }
    }

    let offline = tokio::task::spawn_blocking(move || {
        wait_for_loot_character_offline(
            character_guid,
            Duration::from_secs(LOOT_LOGOUT_DB_CONFIRM_WAIT_SECS),
        )
    })
    .await
    .map_err(|error| anyhow!("loot logout DB worker join failed: {error}"))?;
    if let Err(error) = offline {
        let route = completion_route
            .map(|route| format!("{route:?}"))
            .unwrap_or_else(|| "none".to_string());
        let transport = if transport_errors.is_empty() {
            "none".to_string()
        } else {
            transport_errors.join("; ")
        };
        bail!(
            "loot logout was not proven for character {character_guid} (LogoutComplete route={route}, transport errors={transport}): {error}"
        );
    }

    match completion_route {
        Some(LogoutCompletionRoute::Realm) => info!(
            "[Bot {}] ✅ realm SMSG_LOGOUT_COMPLETE and exact offline DB state confirmed",
            bot_index
        ),
        Some(LogoutCompletionRoute::Instance) => warn!(
            "[Bot {}] SMSG_LOGOUT_COMPLETE arrived on instance instead of the C++ realm route; exact offline DB state confirmed",
            bot_index
        ),
        None => warn!(
            "[Bot {}] no SMSG_LOGOUT_COMPLETE arrived within {}s; exact offline DB state confirmed as the bounded fallback{}",
            bot_index,
            NORMAL_LOGOUT_COMPLETE_WAIT_SECS,
            if transport_errors.is_empty() {
                String::new()
            } else {
                format!(" ({})", transport_errors.join("; "))
            }
        ),
    }
    Ok(())
}

pub(super) async fn best_effort_close(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    character_guid: u64,
    result: &mut BotRunResult,
) {
    if let Some(realm) = realm_connection.as_mut() {
        let _ = send_encrypted_packet(&mut realm.stream, &mut realm.crypt, CMSG_LEAVE_GROUP, &[0])
            .await;
    }
    let _ = logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection.as_mut(),
        character_guid,
        result,
    )
    .await;
}

pub(super) async fn best_effort_logout_preserving_group(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    character_guid: u64,
    result: &mut BotRunResult,
) {
    let _ = logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection.as_mut(),
        character_guid,
        result,
    )
    .await;
}

impl WireEvidence {
    fn merge(&mut self, other: Self) {
        self.item_pushes.extend(other.item_pushes);
        self.loot_removed.extend(other.loot_removed);
        self.money_notifies.extend(other.money_notifies);
        self.coin_removed.extend(other.coin_removed);
        self.inventory_failures.extend(other.inventory_failures);
    }
}

async fn wait_phase(options: &LootRaceOptions, barrier: &Barrier, label: &str) -> Result<()> {
    options.sync.cancellation_error()?;
    info!(
        "[Bot {}] loot-race phase: waiting for {label}",
        options.participant + 1
    );
    tokio::time::timeout(Duration::from_secs(options.timeout_secs), async {
        tokio::select! {
            _ = barrier.wait() => Ok(()),
            _ = options.sync.cancelled() => options.sync.cancellation_error(),
        }
    })
    .await
    .map_err(|_| anyhow!("loot-race timed out waiting for {label}"))??;
    info!(
        "[Bot {}] loot-race phase: {label} complete",
        options.participant + 1
    );
    Ok(())
}

/// C++ `Opcodes.cpp` maps these party packets to
/// `CONNECTION_TYPE_REALM`. Seeing one on the instance socket is a routing
/// defect, not harmless traffic that the atomic-loot harness may discard.
fn validate_party_packet_route_like_cpp(opcode: u16, route: PartyPacketRoute) -> Result<()> {
    if route == PartyPacketRoute::Instance
        && matches!(
            opcode,
            SMSG_PARTY_INVITE
                | SMSG_PARTY_UPDATE
                | SMSG_PARTY_COMMAND_RESULT
                | SMSG_PARTY_MEMBER_FULL_STATE
        )
    {
        bail!(
            "realm-only party opcode 0x{opcode:04X} arrived on instance while forming the loot-race party; C++ Opcodes.cpp requires CONNECTION_TYPE_REALM"
        );
    }
    Ok(())
}

struct PartyUpdateCursor<'a> {
    payload: &'a [u8],
    offset: usize,
}

impl<'a> PartyUpdateCursor<'a> {
    fn new(payload: &'a [u8]) -> Self {
        Self { payload, offset: 0 }
    }

    fn take(&mut self, len: usize, field: &str) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| anyhow!("PartyUpdate {field} offset overflow"))?;
        let bytes = self.payload.get(self.offset..end).ok_or_else(|| {
            anyhow!(
                "malformed PartyUpdate: {field} needs {len} byte(s) at offset {}, payload has {}",
                self.offset,
                self.payload.len()
            )
        })?;
        self.offset = end;
        Ok(bytes)
    }

    fn read_u8(&mut self, field: &str) -> Result<u8> {
        Ok(self.take(1, field)?[0])
    }

    fn read_u16(&mut self, field: &str) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2, field)?.try_into().expect("exact u16 slice"),
        ))
    }

    fn read_u32(&mut self, field: &str) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4, field)?.try_into().expect("exact u32 slice"),
        ))
    }

    fn read_i32(&mut self, field: &str) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.take(4, field)?.try_into().expect("exact i32 slice"),
        ))
    }

    fn read_packed_guid(&mut self, field: &str) -> Result<(u64, u64)> {
        let (consumed, low, high) = parse_packed_guid(&self.payload[self.offset..])
            .ok_or_else(|| anyhow!("malformed PartyUpdate {field} packed ObjectGuid"))?;
        self.offset = self
            .offset
            .checked_add(consumed)
            .ok_or_else(|| anyhow!("PartyUpdate {field} offset overflow"))?;
        Ok((low, high))
    }
}

/// Decode the C++ `PartyPackets.cpp::PartyUpdate::Write` prefix and each
/// `PartyPlayerInfo`. The race must prove an actual two-player HOME party; a
/// stray, destroyed, raid, LFG, or malformed PartyUpdate is not coordination.
fn validate_party_update_like_cpp(
    payload: &[u8],
    receiver_guid: (u64, u64),
    peer_guid: (u64, u64),
    expected_leader_guid: (u64, u64),
) -> Result<()> {
    const GROUP_CATEGORY_HOME_LIKE_CPP: u8 = 0;
    const GROUP_TYPE_NORMAL_LIKE_CPP: u8 = 1;
    const EXPECTED_PARTY_MEMBERS: u32 = 2;

    let mut cursor = PartyUpdateCursor::new(payload);
    let party_flags = cursor.read_u16("PartyFlags")?;
    let party_index = cursor.read_u8("PartyIndex")?;
    let party_type = cursor.read_u8("PartyType")?;
    let my_index = cursor.read_i32("MyIndex")?;
    let party_guid = cursor.read_packed_guid("PartyGUID")?;
    let _sequence_num = cursor.read_u32("SequenceNum")?;
    let leader_guid = cursor.read_packed_guid("LeaderGUID")?;
    let _leader_faction_group = cursor.read_u8("LeaderFactionGroup")?;
    let player_count = cursor.read_u32("PlayerList.size")?;
    let optional_bits = cursor.read_u8("optional-value bits")?;
    let has_lfg_info = optional_bits & 0x80 != 0;
    let has_loot_settings = optional_bits & 0x40 != 0;
    let has_difficulty_settings = optional_bits & 0x20 != 0;

    if party_flags != 0
        || party_index != GROUP_CATEGORY_HOME_LIKE_CPP
        || party_type != GROUP_TYPE_NORMAL_LIKE_CPP
    {
        bail!(
            "loot-race PartyUpdate was not a normal HOME party: flags={party_flags:#06X} index={party_index} type={party_type}"
        );
    }
    if party_guid == (0, 0) {
        bail!("loot-race PartyUpdate carried an empty PartyGUID");
    }
    if leader_guid != expected_leader_guid {
        bail!(
            "loot-race PartyUpdate leader {:#018X}/{:#018X} did not match inviter {:#018X}/{:#018X}",
            leader_guid.0,
            leader_guid.1,
            expected_leader_guid.0,
            expected_leader_guid.1
        );
    }
    if player_count != EXPECTED_PARTY_MEMBERS {
        bail!(
            "loot-race PartyUpdate carried {player_count} player(s), expected {EXPECTED_PARTY_MEMBERS}"
        );
    }
    if optional_bits & 0x1F != 0 {
        bail!(
            "malformed PartyUpdate optional-value bit padding {:#04X}",
            optional_bits & 0x1F
        );
    }
    // C++ `Group::SendUpdateToPlayer` includes loot and difficulty settings
    // for a non-LFG group with more than one member.
    if has_lfg_info || !has_loot_settings || !has_difficulty_settings {
        bail!(
            "loot-race PartyUpdate optional values did not describe a normal two-player group: lfg={has_lfg_info} loot={has_loot_settings} difficulty={has_difficulty_settings}"
        );
    }

    let mut roster = Vec::with_capacity(EXPECTED_PARTY_MEMBERS as usize);
    for player_index in 0..player_count {
        // `PartyPlayerInfo::operator<<` writes 15 MSB-first bits, then the
        // following packed GUID byte-write aligns to the next byte.
        let info_bits = u16::from_be_bytes(
            cursor
                .take(2, "PartyPlayerInfo bit fields")?
                .try_into()
                .expect("exact bit-field slice"),
        );
        if info_bits & 1 != 0 {
            bail!("malformed PartyUpdate player {player_index}: nonzero aligned bit padding");
        }
        let info_bits = info_bits >> 1;
        let name_len = usize::from((info_bits >> 9) & 0x3F);
        let voice_len_plus_one = usize::from((info_bits >> 3) & 0x3F);
        let connected = info_bits & 0x04 != 0;
        if name_len == 0 || voice_len_plus_one == 0 {
            bail!(
                "malformed PartyUpdate player {player_index}: name_len={name_len} voice_len_plus_one={voice_len_plus_one}"
            );
        }

        let guid = cursor.read_packed_guid("PartyPlayerInfo.GUID")?;
        let subgroup = cursor.read_u8("PartyPlayerInfo.Subgroup")?;
        let _flags = cursor.read_u8("PartyPlayerInfo.Flags")?;
        let _roles = cursor.read_u8("PartyPlayerInfo.RolesAssigned")?;
        let _class = cursor.read_u8("PartyPlayerInfo.Class")?;
        let _faction = cursor.read_u8("PartyPlayerInfo.FactionGroup")?;
        let _name = cursor.take(name_len, "PartyPlayerInfo.Name")?;
        let _voice_state = cursor.take(voice_len_plus_one - 1, "PartyPlayerInfo.VoiceStateID")?;

        if !connected || subgroup != 0 {
            bail!(
                "loot-race PartyUpdate player {player_index} was not a connected HOME subgroup member: connected={connected} subgroup={subgroup}"
            );
        }
        roster.push(guid);
    }

    let receiver_index = usize::try_from(my_index)
        .ok()
        .filter(|index| *index < roster.len())
        .ok_or_else(|| {
            anyhow!("loot-race PartyUpdate MyIndex {my_index} was outside the roster")
        })?;
    if roster[receiver_index] != receiver_guid {
        bail!(
            "loot-race PartyUpdate MyIndex {my_index} identified {:#018X}/{:#018X}, expected receiver {:#018X}/{:#018X}",
            roster[receiver_index].0,
            roster[receiver_index].1,
            receiver_guid.0,
            receiver_guid.1
        );
    }
    let mut expected_roster = [receiver_guid, peer_guid];
    expected_roster.sort_unstable();
    roster.sort_unstable();
    if roster.as_slice() != expected_roster {
        bail!(
            "loot-race PartyUpdate roster {roster:?} did not contain exactly receiver/peer {expected_roster:?}"
        );
    }

    let loot_method = cursor.read_u8("PartyLootSettings.Method")?;
    let _loot_master = cursor.read_packed_guid("PartyLootSettings.LootMaster")?;
    let _loot_threshold = cursor.read_u8("PartyLootSettings.Threshold")?;
    if loot_method != PERSONAL_LOOT_METHOD_LIKE_CPP {
        bail!(
            "loot-race PartyUpdate loot method {loot_method} was not C++ PERSONAL_LOOT ({PERSONAL_LOOT_METHOD_LIKE_CPP})"
        );
    }
    let _difficulty_settings = cursor.take(12, "PartyDifficultySettings")?;
    if cursor.offset != payload.len() {
        bail!(
            "malformed PartyUpdate left {} trailing byte(s)",
            payload.len() - cursor.offset
        );
    }
    Ok(())
}

async fn form_party(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    if options.participant == 0 {
        let (peer_low, peer_high) = create_player_guid_raw(options.peer_character_guid, realm_id());
        let payload = build_party_invite(&options.peer_name, peer_low, peer_high)?;
        send_encrypted_packet(
            &mut realm.stream,
            &mut realm.crypt,
            CMSG_PARTY_INVITE,
            &payload,
        )
        .await?;
        wait_for_realm_opcode(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            options,
            SMSG_PARTY_UPDATE,
            result,
        )
        .await?;
    } else {
        wait_for_realm_opcode(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            options,
            SMSG_PARTY_INVITE,
            result,
        )
        .await?;
        send_encrypted_packet(
            &mut realm.stream,
            &mut realm.crypt,
            CMSG_PARTY_INVITE_RESPONSE,
            &[0x40],
        )
        .await?;
        wait_for_realm_opcode(
            bot_index,
            stream,
            crypt,
            inflater,
            realm,
            options,
            SMSG_PARTY_UPDATE,
            result,
        )
        .await?;
    }
    Ok(())
}

async fn wait_for_realm_opcode(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    expected: u16,
    result: &mut BotRunResult,
) -> Result<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for realm opcode 0x{expected:04X}");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "loot-race realm packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Realm)?;
            if opcode == SMSG_PARTY_UPDATE {
                validate_party_update_like_cpp(
                    &payload,
                    create_player_guid_raw(options.character_guid, realm_id()),
                    create_player_guid_raw(options.peer_character_guid, realm_id()),
                    create_player_guid_raw(options.killer_character_guid, realm_id()),
                )?;
            }
            if opcode == expected {
                return Ok(payload);
            }
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race instance packet while forming party",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}

async fn drain_until_target_or_quiet(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<bool> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut seen = false;
    while tokio::time::Instant::now() < deadline {
        options.sync.cancellation_error()?;
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(100),
            Duration::from_secs(2),
            "loot-race target discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if let Some(counter) = target_seen_in_update(options, opcode, &payload)? {
            result.loot_race_target_runtime_counter = Some(counter);
            seen = true;
        }
        handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        if seen {
            break;
        }
    }
    // Do not let unrelated queued realm traffic grow without bound.
    let _ = read_encrypted_packet_if_ready(
        &mut realm.stream,
        &mut realm.crypt,
        &mut realm.inflater,
        Duration::from_millis(1),
        Duration::from_secs(1),
        "loot-race realm discovery drain",
    )
    .await?;
    Ok(seen)
}

async fn kill_target_once(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let (target_low, target_high) = options.resolved_runtime_guid()?;
    let (killer_low, killer_high) =
        create_player_guid_raw(options.killer_character_guid, realm_id());
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_ATTACK_SWING,
        &build_packed_guid(target_low, target_high),
    )
    .await?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    let mut positive_damage = 0i64;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for the disposable loot-race creature to die after {positive_damage} observed damage"
            );
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "loot-race killer combat packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
            if opcode == SMSG_ATTACKER_STATE_UPDATE {
                let update = parse_attacker_state_update_summary(&payload)
                    .context("malformed loot-race SMSG_ATTACKER_STATE_UPDATE")?;
                if (update.attacker_guid_low, update.attacker_guid_high)
                    == (killer_low, killer_high)
                    && (update.victim_guid_low, update.victim_guid_high)
                        == (target_low, target_high)
                {
                    if update.damage < 0 {
                        bail!(
                            "loot-race killer reported negative damage {}",
                            update.damage
                        );
                    }
                    positive_damage += i64::from(update.damage);
                    if update.over_damage >= 0 {
                        if positive_damage == 0 {
                            bail!("loot-race target death had no positive killer damage evidence");
                        }
                        return Ok(());
                    }
                }
            } else if opcode == SMSG_ATTACK_STOP {
                let stop = parse_attack_stop_summary(&payload)
                    .context("malformed loot-race SMSG_ATTACK_STOP")?;
                if (stop.attacker_guid_low, stop.attacker_guid_high) == (killer_low, killer_high)
                    && (stop.victim_guid_low, stop.victim_guid_high) == (target_low, target_high)
                {
                    if !stop.now_dead {
                        bail!("server stopped the loot-race attack before the target died");
                    }
                    if positive_damage == 0 {
                        bail!("loot-race target death had no positive killer damage evidence");
                    }
                    return Ok(());
                }
            }
        }
        if let Some((opcode, _payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race killer realm combat packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
        }
    }
}

async fn wait_for_loot_window(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<LootWindow> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for SMSG_LOOT_RESPONSE");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "loot-race loot response",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            if opcode == SMSG_LOOT_RESPONSE {
                let response = parse_loot_response(&payload)?;
                let expected_items = response
                    .items
                    .iter()
                    .filter(|item| {
                        item.item_entry == options.target.item_entry && item.quantity == 1
                    })
                    .collect::<Vec<_>>();
                let wrong_item_shape = expected_items.len() != 1
                    || (options.phase == LootRacePhase::CaptureItem && response.items.len() != 1);
                if wrong_item_shape {
                    bail!(
                        "loot response contained {} total rows and {} matching single-item rows for expected entry {}; strict capture requires one of each and race requires exactly one expected row",
                        response.items.len(),
                        expected_items.len(),
                        options.target.item_entry,
                    );
                }
                let item = expected_items[0];
                let (owner_low, owner_high) = options.resolved_runtime_guid()?;
                if (response.owner_low, response.owner_high) != (owner_low, owner_high) {
                    bail!("loot response owner did not match the exact acknowledged world spawn");
                }
                validate_loot_object_guid_like_cpp(
                    response.loot_low,
                    response.loot_high,
                    options.target.map_id,
                    realm_id(),
                )?;
                let wrong_money_shape = match options.phase {
                    LootRacePhase::CaptureItem => response.coins != 0,
                    LootRacePhase::Race => response.coins != RACE_GAMEOBJECT_MONEY,
                    LootRacePhase::VerifyRelog => true,
                };
                let wrong_method = options.phase == LootRacePhase::Race
                    && response.loot_method != PERSONAL_LOOT_METHOD_LIKE_CPP;
                if !response.acquired
                    || response.failure_reason != 17
                    || wrong_money_shape
                    || wrong_method
                {
                    bail!(
                        "loot response did not match the acquired item/money/method contract for {:?} (failure={}, acquired={}, coins={}, method={})",
                        options.phase,
                        response.failure_reason,
                        response.acquired,
                        response.coins,
                        response.loot_method
                    );
                }
                return Ok(LootWindow {
                    owner_low: response.owner_low,
                    owner_high: response.owner_high,
                    loot_low: response.loot_low,
                    loot_high: response.loot_high,
                    coins: response.coins,
                    item_entry: item.item_entry,
                    quantity: item.quantity,
                    loot_list_id: item.loot_list_id,
                    loot_method: response.loot_method,
                });
            }
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
        let _ = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race realm while opening",
        )
        .await?;
    }
}

async fn collect_evidence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    expected_loot_owner: (u64, u64),
    window: Duration,
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<WireEvidence> {
    let deadline = tokio::time::Instant::now() + window;
    let mut evidence = WireEvidence::default();
    while tokio::time::Instant::now() < deadline {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(20)),
            remaining,
            "loot-race realm evidence",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race instance evidence",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            record_evidence(opcode, &payload, expected_loot_owner, &mut evidence)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
    Ok(evidence)
}

fn record_evidence(
    opcode: u16,
    payload: &[u8],
    expected_loot_owner: (u64, u64),
    evidence: &mut WireEvidence,
) -> Result<()> {
    match opcode {
        SMSG_ITEM_PUSH_RESULT => evidence.item_pushes.push(parse_item_push(payload)?),
        SMSG_LOOT_REMOVED => {
            let (owner_used, owner_low, owner_high) = parse_packed_guid(payload)
                .ok_or_else(|| anyhow!("malformed SMSG_LOOT_REMOVED owner guid"))?;
            if (owner_low, owner_high) != expected_loot_owner {
                bail!(
                    "SMSG_LOOT_REMOVED owner ({owner_low:#x}, {owner_high:#x}) did not match discovered world-object GUID ({:#x}, {:#x})",
                    expected_loot_owner.0,
                    expected_loot_owner.1
                );
            }
            let (loot_used, loot_low, loot_high) = parse_packed_guid(&payload[owner_used..])
                .ok_or_else(|| anyhow!("malformed SMSG_LOOT_REMOVED loot guid"))?;
            let list_id = *payload
                .get(owner_used + loot_used)
                .ok_or_else(|| anyhow!("malformed SMSG_LOOT_REMOVED list id"))?;
            if owner_used + loot_used + 1 != payload.len() {
                bail!("SMSG_LOOT_REMOVED has unexpected trailing bytes");
            }
            evidence.loot_removed.push(LootRemovedEvidence {
                owner_low,
                owner_high,
                loot_low,
                loot_high,
                loot_list_id: list_id,
            });
        }
        SMSG_LOOT_MONEY_NOTIFY => {
            if payload.len() != 17 {
                bail!("malformed SMSG_LOOT_MONEY_NOTIFY");
            }
            evidence.money_notifies.push(MoneyNotify {
                money: u64::from_le_bytes(payload[0..8].try_into()?),
                money_mod: u64::from_le_bytes(payload[8..16].try_into()?),
                sole_looter: payload[16] & 0x80 != 0,
            });
        }
        SMSG_COIN_REMOVED => {
            let (used, low, high) =
                parse_packed_guid(payload).ok_or_else(|| anyhow!("malformed SMSG_COIN_REMOVED"))?;
            if used != payload.len() {
                bail!("SMSG_COIN_REMOVED has unexpected trailing bytes");
            }
            evidence.coin_removed.push((low, high));
        }
        SMSG_INVENTORY_CHANGE_FAILURE => evidence
            .inventory_failures
            .push(parse_inventory_failure(payload)?),
        _ => {}
    }
    Ok(())
}

async fn validate_shared_windows(options: &LootRaceOptions) -> Result<()> {
    let windows = options.sync.windows.lock().await;
    let left = windows[0]
        .as_ref()
        .ok_or_else(|| anyhow!("bot A did not record a loot window"))?;
    let right = windows[1]
        .as_ref()
        .ok_or_else(|| anyhow!("bot B did not record a loot window"))?;
    if left != right {
        bail!("two bots did not receive the same shared loot authority: {left:?} vs {right:?}");
    }
    Ok(())
}

/// Validate the exact `ObjectGuid::Create<HighGuid::LootObject>` wire shape.
///
/// C++ `ObjectGuidFactory::CreateWorldObject` substitutes the active realm
/// when its realm argument is zero, then encodes a map-local LootObject with
/// zero subtype/server/entry and a nonzero map sequence counter.
fn validate_loot_object_guid_like_cpp(
    low: u64,
    high: u64,
    expected_map_id: u16,
    expected_realm_id: u32,
) -> Result<()> {
    let high_type = (high >> 58) & GUID_HIGH_TYPE_MASK;
    let realm = (high >> 42) & GUID_REALM_MASK;
    let map = (high >> 29) & GUID_MAP_MASK;
    let entry = (high >> 6) & GUID_ENTRY_MASK;
    let subtype = high & GUID_SUBTYPE_MASK;
    let server = (low >> 40) & GUID_SERVER_MASK;
    let counter = low & GUID_COUNTER_MASK;
    let expected_realm = u64::from(expected_realm_id) & GUID_REALM_MASK;
    let expected_map = u64::from(expected_map_id) & GUID_MAP_MASK;

    if high_type != HIGH_GUID_LOOT_OBJECT
        || realm != expected_realm
        || map != expected_map
        || entry != 0
        || subtype != 0
        || server != 0
        || counter == 0
    {
        bail!(
            "loot response LootObject GUID has invalid C++ structure: type={high_type}, realm={realm}, map={map}, entry={entry}, subtype={subtype}, server={server}, counter={counter}; expected type={HIGH_GUID_LOOT_OBJECT}, realm={expected_realm}, map={expected_map}, entry/subtype/server=0 and nonzero counter"
        );
    }

    Ok(())
}

async fn validate_item_outcome(options: &LootRaceOptions, result: &mut BotRunResult) -> Result<()> {
    let window = options.sync.windows.lock().await[options.participant]
        .clone()
        .ok_or_else(|| anyhow!("loot-race participant has no recorded loot window"))?;
    let (owner_low, owner_high) = options.resolved_runtime_guid()?;
    let expected_removal = LootRemovedEvidence {
        owner_low,
        owner_high,
        loot_low: window.loot_low,
        loot_high: window.loot_high,
        loot_list_id: window.loot_list_id,
    };
    let evidence = options.sync.evidence.lock().await;
    let character_guids = if options.participant == 0 {
        [options.character_guid, options.peer_character_guid]
    } else {
        [options.peer_character_guid, options.character_guid]
    };
    let grant = validate_atomic_item_wire_outcome_like_cpp(
        &evidence,
        character_guids,
        options.target.item_entry,
        window.quantity,
        expected_removal,
        realm_id(),
    )?;
    // `ItemPushResult::PlayerGUID`, rather than the receiving socket, names
    // the winner. This remains correct when #55 adds C++'s group broadcast.
    result.loot_race_item_push_seen = grant.owner_guid == options.character_guid;
    result.loot_race_loot_removed_seen =
        evidence[options.participant].loot_removed == [expected_removal];
    Ok(())
}

/// Prove one logical item grant without mistaking C++ party fanout for a
/// duplicate grant.
///
/// Issue #106 owns atomic claim authority, while the complete
/// `StoreLootItem` side-effect cascade remains #55. The current Rust path may
/// send only to the winner; C++ broadcasts the same packet to both group
/// members for the default item. Both shapes represent one grant, but any
/// second packet on one socket or any divergent packet fails closed.
fn validate_atomic_item_wire_outcome_like_cpp(
    evidence: &[WireEvidence; 2],
    character_guids: [u64; 2],
    expected_item_entry: u32,
    expected_quantity: u32,
    expected_removal: LootRemovedEvidence,
    expected_realm_id: u32,
) -> Result<ExpectedPersistedItemGrant> {
    for (participant, entry) in evidence.iter().enumerate() {
        if entry.loot_removed.as_slice() != [expected_removal] {
            bail!(
                "participant {participant} observed LootRemoved fanout {:?}; expected exactly {:?}",
                entry.loot_removed,
                expected_removal
            );
        }
        if entry.item_pushes.len() > 1 {
            bail!(
                "participant {participant} observed {} ItemPush packets; one logical grant permits at most one observation per socket",
                entry.item_pushes.len()
            );
        }
    }

    // Deliberately inspect every ItemPush before checking the expected entry.
    // Filtering by entry would hide a foreign or late duplicate grant.
    let wire_grants = evidence
        .iter()
        .enumerate()
        .flat_map(|(participant, entry)| {
            entry
                .item_pushes
                .iter()
                .copied()
                .map(move |push| WireItemGrant { participant, push })
        })
        .collect::<Vec<_>>();
    let first = wire_grants
        .first()
        .copied()
        .ok_or_else(|| anyhow!("atomic ITEM race emitted no ItemPush result"))?;
    if wire_grants.iter().any(|grant| grant.push != first.push) {
        bail!("atomic ITEM race emitted divergent ItemPush observations: {wire_grants:?}");
    }

    let push = first.push;
    let expected_quantity = i32::try_from(expected_quantity)
        .map_err(|_| anyhow!("loot-race expected item quantity exceeds i32"))?;
    if push.item_entry != expected_item_entry
        || push.quantity != expected_quantity
        || push.quantity_in_inventory != expected_quantity
    {
        bail!(
            "atomic ITEM push entry/quantity/inventory {:?} did not match expected {expected_item_entry}/{expected_quantity}/{expected_quantity}",
            (push.item_entry, push.quantity, push.quantity_in_inventory)
        );
    }
    if push.slot != INVENTORY_SLOT_BAG_0
        || !(i32::from(INVENTORY_SLOT_ITEM_START)..i32::from(INVENTORY_SLOT_ITEM_START + 16))
            .contains(&push.slot_in_bag)
    {
        bail!(
            "atomic ITEM push slot {}/{} was not one of the preflight-guaranteed base-backpack destinations",
            push.slot,
            push.slot_in_bag
        );
    }
    if push.pushed
        || push.created
        || push.display_text != 1
        || push.is_bonus_roll
        || push.is_encounter_loot
        || push.dungeon_encounter_id != 0
    {
        bail!(
            "atomic ITEM push flags were not the ordinary overworld StoreLootItem shape: {push:?}"
        );
    }
    let expected_item_high =
        (HIGH_GUID_ITEM << 58) | ((u64::from(expected_realm_id) & GUID_REALM_SPECIFIC_MASK) << 42);
    if push.item_guid_low == 0
        || push.item_guid_low & !GUID_COUNTER_MASK != 0
        || push.item_guid_high != expected_item_high
    {
        bail!(
            "atomic ITEM push GUID {:#018X}/{:#018X} was not a nonempty C++ Item GUID for realm {}",
            push.item_guid_low,
            push.item_guid_high,
            expected_realm_id
        );
    }

    let winner = character_guids
        .iter()
        .position(|guid| {
            create_player_guid_raw(*guid, expected_realm_id) == (push.player_low, push.player_high)
        })
        .ok_or_else(|| {
            anyhow!(
                "atomic ITEM push winner {:#018X}/{:#018X} was neither disposable character",
                push.player_low,
                push.player_high
            )
        })?;
    let loser = 1 - winner;
    if evidence[winner].item_pushes.as_slice() != [push] {
        bail!("atomic ITEM winner socket did not receive exactly its direct ItemPush");
    }
    if !evidence[loser].item_pushes.is_empty() && evidence[loser].item_pushes.as_slice() != [push] {
        bail!("atomic ITEM loser observed a non-identical group ItemPush");
    }

    let loot_gone = InventoryFailure {
        result: 50,
        item_0_low: 0,
        item_0_high: 0,
        item_1_low: 0,
        item_1_high: 0,
        container_b_slot: 0,
    };
    if !evidence[winner].inventory_failures.is_empty()
        || evidence[loser].inventory_failures.as_slice() != [loot_gone]
    {
        bail!(
            "atomic ITEM failure fanout was winner={:?}, loser={:?}; expected no winner failure and one exact EQUIP_ERR_LOOT_GONE (50)",
            evidence[winner].inventory_failures,
            evidence[loser].inventory_failures
        );
    }

    Ok(ExpectedPersistedItemGrant {
        owner_guid: character_guids[winner],
        push,
    })
}

async fn validate_money_outcome(
    options: &LootRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let window = options.sync.windows.lock().await[options.participant]
        .clone()
        .ok_or_else(|| anyhow!("loot-race participant has no recorded loot window"))?;
    let evidence = options.sync.evidence.lock().await;
    let source_coins = u64::from(window.coins);
    let expected_source = (window.loot_low, window.loot_high);
    let winner = validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &evidence,
        expected_source,
        source_coins,
    )?;
    result.loot_race_money_notify_amount = Some(if winner == options.participant {
        source_coins
    } else {
        0
    });
    result.loot_race_coin_removed_seen = evidence[options.participant]
        .coin_removed
        .contains(&expected_source);
    Ok(())
}

/// Validate the two serialized C++ `HandleLootMoneyOpcode` observations.
///
/// Both clients send `CMSG_LOOT_MONEY`. For `LOOT_CHEST`, C++ deliberately
/// keeps `shareMoney=false`: the first serialized requester receives the whole
/// pool with `SoleLooter=true`, then `Loot::LootMoney()` sets gold to zero. The
/// second requester receives one zero notification. Each request still calls
/// `Loot::NotifyMoneyRemoved`, so both active viewers observe two removals.
fn validate_serialized_gameobject_money_wire_outcome_like_cpp(
    evidence: &[WireEvidence; 2],
    expected_source: (u64, u64),
    expected_pool: u64,
) -> Result<usize> {
    if expected_pool == 0 {
        bail!("loot-race cannot prove a positive serialized money winner");
    }

    let positive = MoneyNotify {
        money: expected_pool,
        money_mod: 0,
        sole_looter: true,
    };
    let zero = MoneyNotify {
        money: 0,
        money_mod: 0,
        sole_looter: true,
    };

    let mut winner = None;
    for (participant, entry) in evidence.iter().enumerate() {
        match entry.money_notifies.as_slice() {
            [notify] if *notify == positive => {
                if winner.replace(participant).is_some() {
                    bail!("loot-race observed more than one positive chest-money winner");
                }
            }
            [notify] if *notify == zero => {}
            _ => {
                bail!(
                    "loot-race participant {participant} observed money notifications {:?}; C++ LOOT_CHEST requires one requester-local whole-pool {expected_pool} or serialized zero notification, both SoleLooter=true",
                entry.money_notifies
            );
            }
        }

        let matching_removals = entry
            .coin_removed
            .iter()
            .filter(|source| **source == expected_source)
            .count();
        if entry.coin_removed.len() != 2 || matching_removals != 2 {
            bail!(
                "loot-race participant {participant} observed CoinRemoved sources {:?}; C++ calls NotifyMoneyRemoved once for each of the two serialized requests",
                entry.coin_removed
            );
        }
    }

    winner.ok_or_else(|| anyhow!("loot-race emitted no positive chest-money winner"))
}

async fn handle_instance_housekeeping(
    _bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    opcode: u16,
    payload: &[u8],
) -> Result<()> {
    if opcode == SMSG_TIME_SYNC_REQUEST {
        let sequence = parse_time_sync_request_sequence(payload)?;
        let response = build_time_sync_response_payload(sequence, current_millis_u32());
        send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
    }
    Ok(())
}

fn build_party_invite(name: &str, target_low: u64, target_high: u64) -> Result<Vec<u8>> {
    if name.len() > 0x1FF {
        bail!("party invite name is too long");
    }
    // C++ reads HasPartyIndex as one bit and then ResetBitPos(), so the two
    // string lengths begin at the next byte rather than sharing that bit byte.
    let mut payload = vec![0];
    payload.extend_from_slice(&pack_msb_fields(&[(name.len() as u32, 9), (0, 9)]));
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(target_low, target_high));
    payload.extend_from_slice(name.as_bytes());
    Ok(payload)
}

fn build_loot_item_claim(window: &LootWindow) -> Vec<u8> {
    let mut payload = 1u32.to_le_bytes().to_vec();
    payload.extend_from_slice(&build_packed_guid(window.loot_low, window.loot_high));
    payload.push(window.loot_list_id);
    payload.push(0); // IsSoftInteract=false, flushed to a complete byte.
    payload
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

#[derive(Debug)]
struct ParsedLootItem {
    item_entry: u32,
    quantity: u32,
    loot_list_id: u8,
}

#[derive(Debug)]
struct ParsedLootResponse {
    owner_low: u64,
    owner_high: u64,
    loot_low: u64,
    loot_high: u64,
    failure_reason: u8,
    loot_method: u8,
    coins: u32,
    acquired: bool,
    items: Vec<ParsedLootItem>,
}

fn parse_loot_response(payload: &[u8]) -> Result<ParsedLootResponse> {
    let (owner_used, owner_low, owner_high) =
        parse_packed_guid(payload).ok_or_else(|| anyhow!("malformed loot response owner guid"))?;
    let (loot_used, loot_low, loot_high) = parse_packed_guid(&payload[owner_used..])
        .ok_or_else(|| anyhow!("malformed loot response loot guid"))?;
    let mut offset = owner_used + loot_used;
    let failure_reason = take_u8(payload, &mut offset)?;
    let _acquire_reason = take_u8(payload, &mut offset)?;
    let loot_method = take_u8(payload, &mut offset)?;
    let _threshold = take_u8(payload, &mut offset)?;
    let coins = take_u32(payload, &mut offset)?;
    let item_count = take_u32(payload, &mut offset)? as usize;
    let currency_count = take_u32(payload, &mut offset)? as usize;
    let bits = take_u8(payload, &mut offset)?;
    let acquired = bits & 0x80 != 0;
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        let _item_bits = take_u8(payload, &mut offset)?;
        let item_entry = take_i32(payload, &mut offset)?;
        if item_entry <= 0 {
            bail!("loot response has nonpositive item entry {item_entry}");
        }
        let _random_seed = take_i32(payload, &mut offset)?;
        let _random_property = take_i32(payload, &mut offset)?;
        let has_bonus = take_u8(payload, &mut offset)? & 0x80 != 0;
        let mod_count = usize::from(take_u8(payload, &mut offset)? >> 2);
        offset = offset
            .checked_add(mod_count * 5)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("loot response item modifications are truncated"))?;
        if has_bonus {
            let _context = take_u8(payload, &mut offset)?;
            let bonus_count = take_u32(payload, &mut offset)? as usize;
            offset = offset
                .checked_add(bonus_count * 4)
                .filter(|end| *end <= payload.len())
                .ok_or_else(|| anyhow!("loot response item bonuses are truncated"))?;
        }
        let quantity = take_u32(payload, &mut offset)?;
        let _loot_item_type = take_u8(payload, &mut offset)?;
        let loot_list_id = take_u8(payload, &mut offset)?;
        items.push(ParsedLootItem {
            item_entry: item_entry as u32,
            quantity,
            loot_list_id,
        });
    }
    // The fixture forbids currencies; parsing their variable bit tail is not
    // needed and accepting it would make the preflight weaker.
    if currency_count != 0 {
        bail!("loot-race fixture unexpectedly produced {currency_count} currencies");
    }
    if offset != payload.len() {
        bail!(
            "loot response has {} unexpected trailing bytes",
            payload.len() - offset
        );
    }
    Ok(ParsedLootResponse {
        owner_low,
        owner_high,
        loot_low,
        loot_high,
        failure_reason,
        loot_method,
        coins,
        acquired,
        items,
    })
}

fn parse_item_push(payload: &[u8]) -> Result<ItemPush> {
    let (player_used, player_low, player_high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("malformed SMSG_ITEM_PUSH_RESULT player guid"))?;
    let mut offset = player_used;
    let slot = take_u8(payload, &mut offset)?;
    let slot_in_bag = take_i32(payload, &mut offset)?;
    let quest_log_item_id = take_i32(payload, &mut offset)?;
    let quantity = take_i32(payload, &mut offset)?;
    let quantity_in_inventory = take_i32(payload, &mut offset)?;
    let dungeon_encounter_id = take_i32(payload, &mut offset)?;
    // Battle-pet metadata is part of the 3.4.3 layout but is unrelated to
    // this ordinary item fixture. Consume it so the following GUID/bit fields
    // remain aligned.
    for _ in 0..4 {
        let _ = take_i32(payload, &mut offset)?;
    }
    let (item_guid_used, item_guid_low, item_guid_high) = parse_packed_guid(&payload[offset..])
        .ok_or_else(|| anyhow!("malformed SMSG_ITEM_PUSH_RESULT item guid"))?;
    offset += item_guid_used;
    let flags = take_u8(payload, &mut offset)?;
    if flags & 0x01 != 0 {
        bail!("SMSG_ITEM_PUSH_RESULT has a nonzero reserved padding bit");
    }
    let pushed = flags & 0x80 != 0;
    let created = flags & 0x40 != 0;
    let display_text = (flags >> 3) & 0x07;
    let is_bonus_roll = flags & 0x04 != 0;
    let is_encounter_loot = flags & 0x02 != 0;
    let item_entry = take_i32(payload, &mut offset)?;
    if item_entry <= 0 {
        bail!("item push has nonpositive item entry {item_entry}");
    }
    let _random_properties_seed = take_i32(payload, &mut offset)?;
    let _random_properties_id = take_i32(payload, &mut offset)?;
    let item_bonus_bits = take_u8(payload, &mut offset)?;
    if item_bonus_bits & 0x7F != 0 {
        bail!("SMSG_ITEM_PUSH_RESULT ItemBonus has nonzero padding bits");
    }
    let has_item_bonus = item_bonus_bits & 0x80 != 0;
    let modification_bits = take_u8(payload, &mut offset)?;
    if modification_bits & 0x03 != 0 {
        bail!("SMSG_ITEM_PUSH_RESULT ItemModList has nonzero padding bits");
    }
    let modification_count = usize::from(modification_bits >> 2);
    for _ in 0..modification_count {
        let _value = take_i32(payload, &mut offset)?;
        let _modifier_type = take_u8(payload, &mut offset)?;
    }
    if has_item_bonus {
        let _context = take_u8(payload, &mut offset)?;
        let bonus_count = usize::try_from(take_u32(payload, &mut offset)?)?;
        let bonus_bytes = bonus_count
            .checked_mul(std::mem::size_of::<u32>())
            .ok_or_else(|| anyhow!("SMSG_ITEM_PUSH_RESULT bonus-list length overflow"))?;
        offset = offset
            .checked_add(bonus_bytes)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| anyhow!("SMSG_ITEM_PUSH_RESULT item bonuses are truncated"))?;
    }
    if offset != payload.len() {
        bail!(
            "SMSG_ITEM_PUSH_RESULT has {} unexpected trailing bytes",
            payload.len() - offset
        );
    }
    Ok(ItemPush {
        player_low,
        player_high,
        slot,
        slot_in_bag,
        quest_log_item_id,
        quantity,
        quantity_in_inventory,
        dungeon_encounter_id,
        item_guid_low,
        item_guid_high,
        pushed,
        created,
        display_text,
        is_bonus_roll,
        is_encounter_loot,
        item_entry: item_entry as u32,
    })
}

pub(super) fn validate_vendor_item_push_result_like_cpp(
    payload: &[u8],
    expected_character_guid: u64,
    expected_item_entry: u32,
    expected_quantity: u32,
    expected_realm_id: u32,
) -> Result<()> {
    let push = parse_item_push(payload)?;
    let expected_quantity = i32::try_from(expected_quantity)
        .map_err(|_| anyhow!("vendor item quantity exceeds i32"))?;
    let expected_player = create_player_guid_raw(expected_character_guid, expected_realm_id);
    if (push.player_low, push.player_high) != expected_player {
        bail!(
            "vendor ItemPush player {:#018X}/{:#018X} did not match character {}",
            push.player_low,
            push.player_high,
            expected_character_guid
        );
    }
    if push.item_entry != expected_item_entry
        || push.quantity != expected_quantity
        || push.quantity_in_inventory != expected_quantity
    {
        bail!(
            "vendor ItemPush entry/quantity/inventory {:?} did not match {expected_item_entry}/{expected_quantity}/{expected_quantity}",
            (push.item_entry, push.quantity, push.quantity_in_inventory)
        );
    }
    if push.slot != INVENTORY_SLOT_BAG_0
        || !(i32::from(INVENTORY_SLOT_ITEM_START)..i32::from(INVENTORY_SLOT_ITEM_START + 16))
            .contains(&push.slot_in_bag)
    {
        bail!(
            "vendor ItemPush slot {}/{} was not a base-backpack destination",
            push.slot,
            push.slot_in_bag
        );
    }
    if push.quest_log_item_id != 0
        || !push.pushed
        || push.created
        || push.display_text != 1
        || push.is_bonus_roll
        || push.is_encounter_loot
        || push.dungeon_encounter_id != 0
    {
        bail!("vendor ItemPush flags were not the ordinary C++ purchase shape: {push:?}");
    }
    let expected_item_high =
        (HIGH_GUID_ITEM << 58) | ((u64::from(expected_realm_id) & GUID_REALM_SPECIFIC_MASK) << 42);
    if push.item_guid_low == 0
        || push.item_guid_low & !GUID_COUNTER_MASK != 0
        || push.item_guid_high != expected_item_high
    {
        bail!(
            "vendor ItemPush GUID {:#018X}/{:#018X} was not a nonempty C++ Item GUID for realm {}",
            push.item_guid_low,
            push.item_guid_high,
            expected_realm_id
        );
    }
    Ok(())
}

fn parse_inventory_failure(payload: &[u8]) -> Result<InventoryFailure> {
    if payload.len() < 4 {
        bail!("malformed SMSG_INVENTORY_CHANGE_FAILURE result");
    }
    let result = i32::from_le_bytes(payload[0..4].try_into()?);
    let (item_0_used, item_0_low, item_0_high) = parse_packed_guid(&payload[4..])
        .ok_or_else(|| anyhow!("malformed SMSG_INVENTORY_CHANGE_FAILURE item 0"))?;
    let item_1_offset = 4 + item_0_used;
    let (item_1_used, item_1_low, item_1_high) = parse_packed_guid(&payload[item_1_offset..])
        .ok_or_else(|| anyhow!("malformed SMSG_INVENTORY_CHANGE_FAILURE item 1"))?;
    let end = item_1_offset + item_1_used;
    let container_b_slot = *payload
        .get(end)
        .ok_or_else(|| anyhow!("malformed SMSG_INVENTORY_CHANGE_FAILURE container slot"))?;
    // EQUIP_ERR_LOOT_GONE has no result-specific tail in C++.
    if result == 50 && end + 1 != payload.len() {
        bail!("EQUIP_ERR_LOOT_GONE has unexpected trailing bytes");
    }
    Ok(InventoryFailure {
        result,
        item_0_low,
        item_0_high,
        item_1_low,
        item_1_high,
        container_b_slot,
    })
}

fn take_u8(payload: &[u8], offset: &mut usize) -> Result<u8> {
    let value = *payload
        .get(*offset)
        .ok_or_else(|| anyhow!("packet truncated at byte {}", *offset))?;
    *offset += 1;
    Ok(value)
}

fn take_u32(payload: &[u8], offset: &mut usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| anyhow!("packet offset overflow"))?;
    let bytes: [u8; 4] = payload
        .get(*offset..end)
        .ok_or_else(|| anyhow!("packet truncated at byte {}", *offset))?
        .try_into()?;
    *offset = end;
    Ok(u32::from_le_bytes(bytes))
}

fn take_i32(payload: &[u8], offset: &mut usize) -> Result<i32> {
    Ok(take_u32(payload, offset)? as i32)
}

fn generated_fixture_health_like_cpp(base_health: u32, health_modifier: f32) -> u32 {
    (base_health as f32 * health_modifier).ceil() as u32
}

fn loot_db_opts(url: &str, label: &str) -> Result<mysql::Opts> {
    let opts =
        mysql::Opts::from_url(url).map_err(|error| anyhow!("Bad {label} DB URL: {error}"))?;
    Ok(mysql::OptsBuilder::from_opts(opts)
        .tcp_connect_timeout(Some(Duration::from_secs(10)))
        .read_timeout(Some(Duration::from_secs(LOOT_DB_OPERATION_TIMEOUT_SECS)))
        .write_timeout(Some(Duration::from_secs(LOOT_DB_OPERATION_TIMEOUT_SECS)))
        .into())
}

fn validate_guarded_fixture_health(health_modifier: f32, generated_max_health: u32) -> Result<()> {
    if (health_modifier - GUARDED_FIXTURE_HEALTH_MODIFIER).abs() > 0.000_000_1
        || generated_max_health != 1
    {
        bail!(
            "loot fixture must be loaded through the pre-start health guard: expected HealthModifier {GUARDED_FIXTURE_HEALTH_MODIFIER} and generated base health 1, got modifier {health_modifier} and health {generated_max_health}; restart the exact world artifact with RUST_CAPTURE_LOOT_FIXTURE_GUARD=1"
        );
    }
    Ok(())
}

fn prepare_gameobject_race_fixture(
    bots: &[config::BotConfig],
    cli: &LootRaceCli,
    shutdown: &CancellationToken,
) -> Result<LootRaceFixture> {
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled before GameObject fixture preflight");
    }
    if (cli.entry, cli.spawn_guid, cli.item_entry)
        != (
            DEFAULT_CREATURE_ENTRY,
            DEFAULT_CREATURE_SPAWN_GUID,
            DEFAULT_ITEM_ENTRY,
        )
    {
        bail!("GameObject race fixture did not match the pinned wrapper-owned contract");
    }
    let world_url = world_db_url()?;
    let world_opts = loot_db_opts(&world_url, "world")?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let spawn: mysql::Row = world
        .exec_first(
            "SELECT guid, id, map, zoneId, areaId, spawnDifficulties, phaseUseFlags, \
                    PhaseId, PhaseGroup, terrainSwapMap, position_x, position_y, position_z, \
                    orientation, rotation0, rotation1, rotation2, rotation3, spawntimesecs, \
                    animprogress, state, ScriptName, StringId, VerifiedBuild \
             FROM gameobject WHERE guid = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Load wrapper-owned GameObject race spawn: {error}"))?
        .ok_or_else(|| {
            anyhow!(
                "wrapper-owned world.gameobject spawn {} is absent; start the world through the loot-race fixture guard",
                cli.spawn_guid
            )
        })?;
    let spawn_guid: u64 = required_row_value(&spawn, "guid")?;
    let entry: u32 = required_row_value(&spawn, "id")?;
    let map_id: u16 = required_row_value(&spawn, "map")?;
    let zone_id: u16 = required_row_value(&spawn, "zoneId")?;
    let area_id: u16 = required_row_value(&spawn, "areaId")?;
    let difficulties: String = required_row_value(&spawn, "spawnDifficulties")?;
    let phase_flags: u8 = required_row_value(&spawn, "phaseUseFlags")?;
    let phase_id: i32 = required_row_value(&spawn, "PhaseId")?;
    let phase_group: i32 = required_row_value(&spawn, "PhaseGroup")?;
    let terrain_swap_map: i32 = required_row_value(&spawn, "terrainSwapMap")?;
    let x: f64 = required_row_value(&spawn, "position_x")?;
    let y: f64 = required_row_value(&spawn, "position_y")?;
    let z: f64 = required_row_value(&spawn, "position_z")?;
    let orientation: f32 = required_row_value(&spawn, "orientation")?;
    let rotations = [
        required_row_value::<f32>(&spawn, "rotation0")?,
        required_row_value::<f32>(&spawn, "rotation1")?,
        required_row_value::<f32>(&spawn, "rotation2")?,
        required_row_value::<f32>(&spawn, "rotation3")?,
    ];
    let spawntime: u32 = required_row_value(&spawn, "spawntimesecs")?;
    let anim_progress: u8 = required_row_value(&spawn, "animprogress")?;
    let state: u8 = required_row_value(&spawn, "state")?;
    let spawn_script: String = required_row_value(&spawn, "ScriptName")?;
    let string_id: Option<String> = required_row_value(&spawn, "StringId")?;
    let verified_build: i32 = required_row_value(&spawn, "VerifiedBuild")?;
    let position_matches = (x - RACE_GAMEOBJECT_X).abs() <= 0.01
        && (y - RACE_GAMEOBJECT_Y).abs() <= 0.01
        && (z - RACE_GAMEOBJECT_Z).abs() <= 0.01;
    if spawn_guid != DEFAULT_CREATURE_SPAWN_GUID
        || entry != DEFAULT_CREATURE_ENTRY
        || map_id != RACE_GAMEOBJECT_MAP_ID
        || zone_id != 0
        || area_id != 0
        || difficulties != "0"
        || phase_flags != 0
        || phase_id != 0
        || phase_group != 0
        || terrain_swap_map != -1
        || !position_matches
        || orientation.abs() > f32::EPSILON
        || rotations
            .iter()
            .any(|rotation| rotation.abs() > f32::EPSILON)
        || spawntime != RACE_GAMEOBJECT_RESPAWN_SECS
        || anim_progress != RACE_GAMEOBJECT_ANIM_PROGRESS
        || state != RACE_GAMEOBJECT_STATE
        || !spawn_script.is_empty()
        || string_id.is_some()
        || verified_build != 0
    {
        bail!(
            "wrapper-owned GameObject spawn drifted from the exact QA contract: guid={spawn_guid} entry={entry} map={map_id} zone={zone_id} area={area_id} difficulties={difficulties:?} phase={phase_flags}/{phase_id}/{phase_group} terrain={terrain_swap_map} pos=({x},{y},{z}) orientation={orientation} rotations={rotations:?} respawn={spawntime} anim={anim_progress} state={state} script={spawn_script:?} string_id={string_id:?} build={verified_build}"
        );
    }
    let same_entry_map_spawns: Vec<u64> = world
        .exec_map(
            "SELECT guid FROM gameobject WHERE id = ? AND map = ? ORDER BY guid",
            (entry, map_id),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Check GameObject race map/entry spawn uniqueness: {error}"))?;
    validate_unique_sql_spawn(&same_entry_map_spawns, spawn_guid, entry, map_id)?;

    let template: mysql::Row = world
        .exec_first(
            "SELECT type, displayId, name, size, \
                    Data0, Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, \
                    Data10, Data11, Data12, Data13, Data14, Data15, Data16, Data17, Data18, Data19, \
                    Data20, Data21, Data22, Data23, Data24, Data25, Data26, Data27, Data28, Data29, \
                    Data30, Data31, Data32, Data33, Data34, ContentTuningId, AIName, ScriptName, \
                    StringId, VerifiedBuild \
             FROM gameobject_template WHERE entry = ?",
            (entry,),
        )
        .map_err(|error| anyhow!("Load Tattered Chest template: {error}"))?
        .ok_or_else(|| anyhow!("Tattered Chest template {entry} is absent"))?;
    let go_type: u8 = required_row_value(&template, "type")?;
    let display_id: u32 = required_row_value(&template, "displayId")?;
    let name: String = required_row_value(&template, "name")?;
    let size: f32 = required_row_value(&template, "size")?;
    let mut template_data = [0_i32; 35];
    for (index, value) in template_data.iter_mut().enumerate() {
        *value = required_row_value(&template, &format!("Data{index}"))?;
    }
    let content_tuning_id: u32 = required_row_value(&template, "ContentTuningId")?;
    let ai_name: String = required_row_value(&template, "AIName")?;
    let template_script: String = required_row_value(&template, "ScriptName")?;
    let template_string_id: Option<String> = required_row_value(&template, "StringId")?;
    let template_build: i32 = required_row_value(&template, "VerifiedBuild")?;
    if go_type != 3
        || display_id != 259
        || name != "Tattered Chest"
        || (size - 1.0).abs() > f32::EPSILON
        || template_data != RACE_GAMEOBJECT_TEMPLATE_DATA
        || content_tuning_id != 0
        || !ai_name.is_empty()
        || !template_script.is_empty()
        || template_string_id.is_some()
        || template_build != 11_723
    {
        bail!(
            "Tattered Chest template/data drifted from the exact shared C++ fixture contract: type={go_type} display={display_id} name={name:?} size={size} data={template_data:?} content_tuning={content_tuning_id} ai={ai_name:?} script={template_script:?} string_id={template_string_id:?} build={template_build}"
        );
    }

    let loot_rows: Vec<(u32, u32, f32, u8, u16, u8, u8, u8)> = world
        .exec(
            "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount \
             FROM gameobject_loot_template WHERE Entry = ? ORDER BY Item, Reference",
            (RACE_GAMEOBJECT_LOOT_ID,),
        )
        .map_err(|error| anyhow!("Load Tattered Chest loot template: {error}"))?;
    if loot_rows.len() != 1
        || loot_rows[0].0 != DEFAULT_ITEM_ENTRY
        || loot_rows[0].1 != 0
        || (loot_rows[0].2 - 100.0).abs() > f32::EPSILON
        || loot_rows[0].3 != 0
        || loot_rows[0].4 != 1
        || loot_rows[0].5 != 0
        || loot_rows[0].6 != 1
        || loot_rows[0].7 != 1
    {
        bail!(
            "GameObject loot id {RACE_GAMEOBJECT_LOOT_ID} was not exactly one unconditional item-{DEFAULT_ITEM_ENTRY} grant: {loot_rows:?}"
        );
    }
    let loot_conditions: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM conditions \
             WHERE SourceTypeOrReferenceId = 4 AND SourceGroup = ?",
            (RACE_GAMEOBJECT_LOOT_ID,),
        )
        .map_err(|error| anyhow!("Check Tattered Chest loot conditions: {error}"))?
        .unwrap_or(0);
    let addon_rows: Vec<(u16, u32, u32, u32, i32, i32, i32, i32, i32, u32, u32)> = world
        .exec(
            "SELECT faction, flags, Mingold, Maxgold, artkit0, artkit1, artkit2, artkit3, artkit4, \
                    WorldEffectID, AIAnimKitID \
             FROM gameobject_template_addon WHERE entry = ?",
            (entry,),
        )
        .map_err(|error| anyhow!("Load guarded Tattered Chest addon: {error}"))?;
    if addon_rows.as_slice()
        != [(
            RACE_GAMEOBJECT_ADDON_FACTION,
            0,
            RACE_GAMEOBJECT_MONEY,
            RACE_GAMEOBJECT_MONEY,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        )]
    {
        bail!(
            "wrapper-owned GameObject addon must match exact faction/flags/artkits/effects and deterministic money {0}/{0}, got {addon_rows:?}",
            RACE_GAMEOBJECT_MONEY
        );
    }
    let event_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM game_event_gameobject WHERE guid = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject event ownership: {error}"))?
        .unwrap_or(0);
    let pool_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM pool_members WHERE type = 1 AND spawnId = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject pool ownership: {error}"))?
        .unwrap_or(0);
    let linked_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM linked_respawn WHERE guid = ? OR linkedGuid = ?",
            (spawn_guid, spawn_guid),
        )
        .map_err(|error| anyhow!("Check GameObject linked respawn: {error}"))?
        .unwrap_or(0);
    let spawn_addon_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM gameobject_addon WHERE guid = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject spawn addon ownership: {error}"))?
        .unwrap_or(0);
    let override_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM gameobject_overrides WHERE spawnId = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject override ownership: {error}"))?
        .unwrap_or(0);
    let spawn_group_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM spawn_group WHERE spawnType = 1 AND spawnId = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject spawn-group ownership: {error}"))?
        .unwrap_or(0);
    if loot_conditions != 0
        || event_rows != 0
        || pool_rows != 0
        || linked_rows != 0
        || spawn_addon_rows != 0
        || override_rows != 0
        || spawn_group_rows != 0
    {
        bail!(
            "GameObject fixture has conditional/spawn metadata: conditions={loot_conditions} event={event_rows} pool={pool_rows} linked={linked_rows} addon={spawn_addon_rows} override={override_rows} spawn_group={spawn_group_rows}"
        );
    }

    let character_url = characters_db_url()?;
    let character_opts = loot_db_opts(&character_url, "characters")?;
    let mut character_db = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    ensure_no_online_characters(&mut character_db, "GameObject loot-race fixture setup")?;
    let respawn_rows: Vec<(i64, u16, u32)> = character_db
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = 1 AND spawnId = ? ORDER BY mapId, instanceId",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Load GameObject respawn snapshot: {error}"))?;
    if !respawn_rows.is_empty() {
        bail!(
            "GameObject spawn {spawn_guid} already has {} type=1 respawn row(s); restart through a clean fixture guard",
            respawn_rows.len()
        );
    }

    let (characters, progress) = snapshot_loot_fixture_characters(
        &mut character_db,
        bots,
        DEFAULT_ITEM_ENTRY,
        0,
        RACE_GAMEOBJECT_MONEY,
        None,
    )?;
    info!(
        "Loot-race shared GameObject fixture: `{name}` entry={entry} spawn={spawn_guid} map={map_id} runtime=wire-discovered loot={RACE_GAMEOBJECT_LOOT_ID} item={DEFAULT_ITEM_ENTRY} money={RACE_GAMEOBJECT_MONEY} respawn={spawntime}s state={state}; C++ GameObject.cpp shared group-rules path"
    );
    let journal = FixtureJournal::configured()?;
    let fixture = LootRaceFixture {
        characters,
        target: LootRaceTarget {
            kind: LootRaceTargetKind::GameObject,
            entry,
            spawn_guid,
            runtime_counter_override: cli.runtime_counter,
            map_id,
            x,
            y,
            z,
            item_entry: DEFAULT_ITEM_ENTRY,
        },
        respawn: None,
        respawn_type: 1,
        gameobject_state: Some(state),
        progress,
        journal,
    };
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled before durable fixture snapshot");
    }
    fixture.journal.persist(&fixture)?;
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled after durable snapshot and before relocation");
    }
    relocate_loot_fixture_characters(&mut character_db, &fixture.characters, map_id, x, y, z)?;
    Ok(fixture)
}

fn prepare_fixture(
    bots: &[config::BotConfig],
    cli: &LootRaceCli,
    purpose: LootFixturePurpose,
    shutdown: &CancellationToken,
) -> Result<LootRaceFixture> {
    if shutdown.is_cancelled() {
        bail!("loot workflow cancelled before fixture preflight");
    }
    if bots.len() != 2 {
        bail!("internal loot-race setup requires exactly two bots");
    }
    for bot in bots {
        if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
            bail!(
                "refusing destructive loot-race setup for non-local account {}",
                bot.account
            );
        }
    }

    if purpose == LootFixturePurpose::Race {
        return prepare_gameobject_race_fixture(bots, cli, shutdown);
    }

    let world_url = world_db_url()?;
    let world_opts = loot_db_opts(&world_url, "world")?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let row: mysql::Row = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.spawnDifficulties, c.spawntimesecs, \
                    c.position_x, c.position_y, c.position_z, c.orientation, \
                    c.phaseUseFlags, c.PhaseId, c.PhaseGroup, c.wander_distance, c.MovementType, \
                    ct.name, ct.type, ct.unit_class, ct.unit_flags, ct.flags_extra, \
                    d.MinLevel, d.MaxLevel, d.HealthScalingExpansion, d.HealthModifier, \
                    d.LootID, d.GoldMin, d.GoldMax, \
                    d.StaticFlags1 \
             FROM creature c \
             JOIN creature_template ct ON ct.entry = c.id \
             JOIN creature_template_difficulty d ON d.Entry = c.id AND d.DifficultyID = 0 \
             WHERE c.guid = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Load loot-race creature fixture: {error}"))?
        .ok_or_else(|| anyhow!("No world.creature row for spawn {}", cli.spawn_guid))?;
    let spawn_guid: u64 = required_row_value(&row, "guid")?;
    let entry: u32 = required_row_value(&row, "id")?;
    let map_id: u16 = required_row_value(&row, "map")?;
    let difficulties: String = required_row_value(&row, "spawnDifficulties")?;
    let spawntime: u32 = required_row_value(&row, "spawntimesecs")?;
    let x: f64 = required_row_value(&row, "position_x")?;
    let y: f64 = required_row_value(&row, "position_y")?;
    let z: f64 = required_row_value(&row, "position_z")?;
    let _orientation: f32 = required_row_value(&row, "orientation")?;
    let phase_flags: u8 = required_row_value(&row, "phaseUseFlags")?;
    let phase_id: i32 = required_row_value(&row, "PhaseId")?;
    let phase_group: i32 = required_row_value(&row, "PhaseGroup")?;
    let wander_distance: f32 = required_row_value(&row, "wander_distance")?;
    let movement_type: u8 = required_row_value(&row, "MovementType")?;
    let name: String = required_row_value(&row, "name")?;
    let creature_type: u8 = required_row_value(&row, "type")?;
    let unit_class: u8 = required_row_value(&row, "unit_class")?;
    let unit_flags: u32 = required_row_value(&row, "unit_flags")?;
    let flags_extra: u32 = required_row_value(&row, "flags_extra")?;
    let min_level: u8 = required_row_value(&row, "MinLevel")?;
    let max_level: u8 = required_row_value(&row, "MaxLevel")?;
    let health_scaling_expansion: i32 = required_row_value(&row, "HealthScalingExpansion")?;
    let health_modifier: f32 = required_row_value(&row, "HealthModifier")?;
    let loot_id: u32 = required_row_value(&row, "LootID")?;
    let min_gold: u32 = required_row_value(&row, "GoldMin")?;
    let max_gold: u32 = required_row_value(&row, "GoldMax")?;
    let static_flags1: u32 = required_row_value(&row, "StaticFlags1")?;
    if spawn_guid != cli.spawn_guid || entry != cli.entry || loot_id == 0 {
        bail!("creature fixture does not match the exact spawn/entry/loot-id contract");
    }
    let same_entry_map_spawns: Vec<u64> = world
        .exec_map(
            "SELECT guid FROM creature WHERE id = ? AND map = ? ORDER BY guid",
            (entry, map_id),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Check loot-race map/entry spawn uniqueness: {error}"))?;
    validate_unique_sql_spawn(&same_entry_map_spawns, cli.spawn_guid, entry, map_id)?;
    if !matches!(map_id, 0 | 1 | 530 | 571) {
        bail!("loot-race creature must be on a known overworld map, got map {map_id}");
    }
    if !difficulties
        .split(',')
        .any(|difficulty| difficulty.trim() == "0")
    {
        bail!("creature fixture is not available on overworld difficulty 0");
    }
    if phase_flags != 0 || phase_id != 0 || phase_group != 0 {
        bail!("creature fixture has phase behavior outside this focused smoke");
    }
    if wander_distance != 0.0 || movement_type != 0 {
        bail!("creature fixture must be stationary for deterministic two-client engagement");
    }
    if spawntime == 0 || spawntime > 3_600 {
        bail!("creature respawn interval {spawntime} is outside the acknowledged QA boundary");
    }
    if purpose != LootFixturePurpose::CaptureItem {
        bail!("internal error: creature fixture is reserved for loot-item capture");
    }
    if min_gold != 0 || max_gold != 0 {
        bail!("loot-item capture creature must have no random money pool");
    }
    if min_level == 0 || max_level < min_level || max_level > 70 {
        bail!("loot-item capture creature exceeds the bounded combat target level");
    }
    let health_expansion_index = match health_scaling_expansion {
        -1 | 2 => 2,
        0 => 0,
        1 => 1,
        value => bail!(
            "loot fixture has unsupported HealthScalingExpansion {value}; expected the C++ -1..=2 domain"
        ),
    };
    let base_health_rows: Vec<(u8, u32, u32, u32)> = world
        .exec(
            "SELECT level, basehp0, basehp1, basehp2 \
             FROM creature_classlevelstats \
             WHERE class = ? AND level BETWEEN ? AND ? ORDER BY level",
            (unit_class, min_level, max_level),
        )
        .map_err(|error| anyhow!("Load loot fixture class-level health: {error}"))?;
    let expected_level_rows = usize::from(max_level - min_level) + 1;
    if base_health_rows.len() != expected_level_rows {
        bail!(
            "loot fixture class {} level range {}..{} resolved {} class-level health row(s), expected {expected_level_rows}",
            unit_class,
            min_level,
            max_level,
            base_health_rows.len()
        );
    }
    let guarded_max_health = base_health_rows
        .iter()
        .map(|(_, hp0, hp1, hp2)| {
            let base_health = [*hp0, *hp1, *hp2][health_expansion_index].max(1);
            generated_fixture_health_like_cpp(base_health, health_modifier)
        })
        .max()
        .unwrap_or(0);
    validate_guarded_fixture_health(health_modifier, guarded_max_health)?;
    const UNATTACKABLE_UNIT_FLAGS: u32 = 0x0000_0002 | 0x0000_0100 | 0x0001_0000 | 0x0200_0000;
    const UNSUITABLE_STATIC_FLAGS1: u32 = 0x0000_0004 | 0x0000_0020 | 0x0000_0200;
    if unit_flags & UNATTACKABLE_UNIT_FLAGS != 0
        || static_flags1 & UNSUITABLE_STATIC_FLAGS1 != 0
        || creature_type == CREATURE_TYPE_CRITTER
    {
        bail!("creature fixture is not a normal attackable shared-loot target");
    }
    const UNSUITABLE_FLAGS_EXTRA: u32 = 0x0000_0080 | 0x0000_0400 | 0x0000_2000 | 0x0000_4000;
    if flags_extra & UNSUITABLE_FLAGS_EXTRA != 0 {
        bail!("creature fixture has trigger/ghost/no-combat/world-event flags");
    }
    let loot_rows: Vec<(u32, u32, f32, u8, u16, u8, u8, u8)> = world
        .exec(
            "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount \
             FROM creature_loot_template WHERE Entry = ? ORDER BY Item, Reference",
            (loot_id,),
        )
        .map_err(|error| anyhow!("Load loot-race creature loot rows: {error}"))?;
    if purpose == LootFixturePurpose::CaptureItem && loot_rows.len() != 1 {
        bail!(
            "creature loot id {loot_id} contains {} rows; strict capture requires exactly one logical item pool",
            loot_rows.len()
        );
    }
    let matching_item_rows = loot_rows
        .iter()
        .filter(|&&(item, _, _, _, _, _, _, _)| item == cli.item_entry)
        .collect::<Vec<_>>();
    if matching_item_rows.len() != 1 {
        bail!(
            "creature loot id {loot_id} contains {} rows for expected item {}; expected exactly one",
            matching_item_rows.len(),
            cli.item_entry
        );
    }
    let &(_item, reference, chance, quest_required, loot_mode, group_id, min_count, max_count) =
        matching_item_rows[0];
    if reference != 0
        || (chance - 100.0).abs() > f32::EPSILON
        || quest_required != 0
        || loot_mode != 1
        || group_id != 0
        || min_count != 1
        || max_count != 1
    {
        bail!("expected creature item row is not one unconditional normal single-item grant");
    }
    let condition_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM conditions \
             WHERE SourceTypeOrReferenceId = 1 AND SourceGroup = ?",
            (loot_id,),
        )
        .map_err(|error| anyhow!("Check loot-race creature loot conditions: {error}"))?
        .unwrap_or(0);
    if condition_rows != 0 {
        bail!("creature loot id {loot_id} has conditions outside this focused smoke");
    }
    let event_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM game_event_creature WHERE guid = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Check loot-race game-event ownership: {error}"))?
        .unwrap_or(0);
    let pool_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM pool_members WHERE type = 0 AND spawnId = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Check loot-race pool ownership: {error}"))?
        .unwrap_or(0);
    let linked_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM linked_respawn WHERE guid = ? OR linkedGuid = ?",
            (cli.spawn_guid, cli.spawn_guid),
        )
        .map_err(|error| anyhow!("Check loot-race linked respawn: {error}"))?
        .unwrap_or(0);
    if event_rows != 0 || pool_rows != 0 || linked_rows != 0 {
        bail!("creature fixture is event/pool/linked managed and cannot be consumed safely");
    }

    let character_url = characters_db_url()?;
    let character_opts = loot_db_opts(&character_url, "characters")?;
    let mut character_db = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    ensure_no_online_characters(&mut character_db, "loot fixture setup")?;
    let respawn_rows: Vec<(i64, u16, u32)> = character_db
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = 0 AND spawnId = ? ORDER BY mapId, instanceId",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Load loot-race respawn snapshot: {error}"))?;
    if !respawn_rows.is_empty() {
        bail!(
            "creature spawn {} already has {} persisted respawn timer row(s); use a fresh runtime/fixture",
            cli.spawn_guid,
            respawn_rows.len()
        );
    }
    let respawn = respawn_rows.into_iter().next();

    let (fixture_characters, progress) = snapshot_loot_fixture_characters(
        &mut character_db,
        bots,
        cli.item_entry,
        max_level,
        max_gold,
        Some((bots[0].character_guid, LOOT_ITEM_CAPTURE_KEYRING_SLOT)),
    )?;

    info!(
        "Loot-race disposable fixture: `{}` entry={} spawn={} map={} runtime_counter={} level={}..{} guarded_base_health={} loot={} item={} gold={}..{} respawn={}s; live creature cannot be restored without restart",
        name,
        entry,
        cli.spawn_guid,
        map_id,
        if cli.runtime_counter == 0 {
            "auto".to_string()
        } else {
            cli.runtime_counter.to_string()
        },
        min_level,
        max_level,
        guarded_max_health,
        loot_id,
        cli.item_entry,
        min_gold,
        max_gold,
        spawntime,
    );
    let journal = FixtureJournal::configured()?;
    let fixture = LootRaceFixture {
        characters: fixture_characters,
        target: LootRaceTarget {
            kind: LootRaceTargetKind::Creature,
            entry,
            spawn_guid: cli.spawn_guid,
            runtime_counter_override: cli.runtime_counter,
            map_id,
            x,
            y,
            z,
            item_entry: cli.item_entry,
        },
        respawn: respawn.map(|(respawn_time, map_id, instance_id)| RespawnSnapshot {
            respawn_time,
            map_id,
            instance_id,
        }),
        respawn_type: 0,
        gameobject_state: None,
        progress,
        journal,
    };
    if shutdown.is_cancelled() {
        bail!("loot-item capture cancelled before durable fixture snapshot");
    }
    fixture.journal.persist(&fixture)?;
    if shutdown.is_cancelled() {
        bail!("loot-item capture cancelled after durable snapshot and before relocation");
    }
    relocate_loot_fixture_characters(&mut character_db, &fixture.characters, map_id, x, y, z)?;
    Ok(fixture)
}

fn snapshot_loot_fixture_characters(
    character_db: &mut mysql::Conn,
    bots: &[config::BotConfig],
    item_entry: u32,
    target_max_level: u8,
    maximum_money_gain: u32,
    required_empty_top_level_slot: Option<(u64, u8)>,
) -> Result<([CharacterFixture; 2], CharacterProgressSnapshot)> {
    let mut fixtures = Vec::with_capacity(2);
    for bot in bots {
        let character: mysql::Row = character_db
            .exec_first(
                "SELECT account, name, race, level, xp, health, \
                        power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, \
                        restState, rest_bonus, exploredZones, knownTitles, chosenTitle, \
                        online, at_login, money, map, zone, instance_id, \
                        position_x, position_y, position_z, orientation \
                 FROM characters WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Load loot-race character {}: {error}", bot.character_guid))?
            .ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
        let account: u32 = required_row_value(&character, "account")?;
        let name: String = required_row_value(&character, "name")?;
        let race: u8 = required_row_value(&character, "race")?;
        let level: u8 = required_row_value(&character, "level")?;
        let xp: u32 = required_row_value(&character, "xp")?;
        let health: u32 = required_row_value(&character, "health")?;
        let powers = [
            required_row_value(&character, "power1")?,
            required_row_value(&character, "power2")?,
            required_row_value(&character, "power3")?,
            required_row_value(&character, "power4")?,
            required_row_value(&character, "power5")?,
            required_row_value(&character, "power6")?,
            required_row_value(&character, "power7")?,
            required_row_value(&character, "power8")?,
            required_row_value(&character, "power9")?,
            required_row_value(&character, "power10")?,
        ];
        let rest_state: u8 = required_row_value(&character, "restState")?;
        let rest_bonus: f32 = required_row_value(&character, "rest_bonus")?;
        let explored_zones: Option<String> = required_row_value(&character, "exploredZones")?;
        let known_titles: Option<String> = required_row_value(&character, "knownTitles")?;
        let chosen_title: u32 = required_row_value(&character, "chosenTitle")?;
        let online: u8 = required_row_value(&character, "online")?;
        let at_login: u16 = required_row_value(&character, "at_login")?;
        let money: u64 = required_row_value(&character, "money")?;
        let old_map: u32 = required_row_value(&character, "map")?;
        let zone: u32 = required_row_value(&character, "zone")?;
        let instance_id: u32 = required_row_value(&character, "instance_id")?;
        let old_x: f64 = required_row_value(&character, "position_x")?;
        let old_y: f64 = required_row_value(&character, "position_y")?;
        let old_z: f64 = required_row_value(&character, "position_z")?;
        let orientation: f32 = required_row_value(&character, "orientation")?;
        if account != bot.account_id || online != 0 || at_login != 0 {
            bail!(
                "loot-race character {} owner/online/at_login safety check failed",
                bot.character_guid
            );
        }
        if health == 0 || level <= target_max_level {
            bail!(
                "loot-race character {} must be alive and above target level {}",
                bot.character_guid,
                target_max_level
            );
        }
        let maximum_safe_start = MAX_PLAYER_MONEY_LIKE_CPP
            .checked_sub(u64::from(maximum_money_gain))
            .ok_or_else(|| anyhow!("loot-race creature maximum gold exceeds the player cap"))?;
        if money > maximum_safe_start {
            bail!(
                "loot-race character {} lacks headroom for the fixture's maximum money roll",
                bot.character_guid
            );
        }
        let account_chars: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE account = ?",
                (bot.account_id,),
            )
            .map_err(|error| anyhow!("Count dedicated loot-race characters: {error}"))?
            .unwrap_or(0);
        if account_chars != 1 {
            bail!(
                "loot-race requires one dedicated character per game account; {} has {account_chars}",
                bot.account
            );
        }
        let group_rows: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM (\
                    SELECT guid FROM group_member WHERE memberGuid = ? \
                    UNION ALL \
                    SELECT guid FROM `groups` WHERE leaderGuid = ?\
                 ) AS persisted_group_state",
                (bot.character_guid, bot.character_guid),
            )
            .map_err(|error| anyhow!("Check loot-race group state: {error}"))?
            .unwrap_or(0);
        if group_rows != 0 {
            bail!(
                "loot-race character {} is already in a persisted group",
                bot.character_guid
            );
        }
        let persistent_auras: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM character_aura WHERE guid = ?",
                (bot.character_guid,),
            )
            .map_err(|error| anyhow!("Check loot-race persistent auras: {error}"))?
            .unwrap_or(0);
        if persistent_auras != 0 {
            bail!(
                "loot-race character {} has {persistent_auras} persistent aura row(s); money modifiers must be absent",
                bot.character_guid
            );
        }
        let owned_expected: u64 = character_db
            .exec_first(
                "SELECT COUNT(*) FROM item_instance WHERE owner_guid = ? AND itemEntry = ?",
                (bot.character_guid, item_entry),
            )
            .map_err(|error| anyhow!("Check existing loot-race item: {error}"))?
            .unwrap_or(0);
        if owned_expected != 0 {
            bail!(
                "loot-race character {} already owns item entry {}",
                bot.character_guid,
                item_entry
            );
        }
        let occupied: Vec<u8> = character_db
            .exec_map(
                "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
                (bot.character_guid,),
                |slot: u8| slot,
            )
            .map_err(|error| anyhow!("Load loot-race backpack slots: {error}"))?;
        if !(INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16)
            .any(|slot| !occupied.contains(&slot))
        {
            bail!(
                "loot-race character {} has no empty backpack slot",
                bot.character_guid
            );
        }
        validate_required_empty_top_level_slot(
            bot.character_guid,
            &occupied,
            required_empty_top_level_slot,
        )?;
        fixtures.push(CharacterFixture {
            bot: bot.clone(),
            name,
            race,
            money,
            core: CharacterCoreSnapshot {
                level,
                xp,
                health,
                powers,
                rest_state,
                rest_bonus,
                explored_zones,
                known_titles,
                chosen_title,
            },
            position: CharacterPositionSnapshot {
                map_id: old_map,
                zone_id: zone,
                instance_id,
                x: old_x,
                y: old_y,
                z: old_z,
                orientation,
            },
        });
    }
    if faction_for_race(fixtures[0].race) != faction_for_race(fixtures[1].race) {
        bail!(
            "loot-race characters must be the same faction so C++ party invite rules permit grouping"
        );
    }
    let fixture_characters: [CharacterFixture; 2] = fixtures
        .try_into()
        .map_err(|_| anyhow!("internal loot-race character count mismatch"))?;
    let guid_a = fixture_characters[0].bot.character_guid;
    let guid_b = fixture_characters[1].bot.character_guid;
    let guild_members: u64 = character_db
        .exec_first(
            "SELECT COUNT(*) FROM guild_member WHERE guid IN (?, ?)",
            (guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Check loot-race guild criteria isolation: {error}"))?
        .unwrap_or(0);
    if guild_members != 0 {
        bail!(
            "loot-race disposable characters have {guild_members} guild membership row(s); a kill/item criteria update could mutate guild-wide state"
        );
    }
    let progress = load_character_progress_snapshot(character_db, guid_a, guid_b)?;
    Ok((fixture_characters, progress))
}

fn validate_required_empty_top_level_slot(
    character_guid: u64,
    occupied: &[u8],
    required: Option<(u64, u8)>,
) -> Result<()> {
    if let Some((required_guid, required_slot)) = required {
        if character_guid == required_guid && occupied.contains(&required_slot) {
            bail!(
                "loot-item capture character {character_guid} requires exact top-level keyring slot {required_slot} empty"
            );
        }
    }
    Ok(())
}

fn relocate_loot_fixture_characters(
    character_db: &mut mysql::Conn,
    fixture_characters: &[CharacterFixture; 2],
    map_id: u16,
    x: f64,
    y: f64,
    z: f64,
) -> Result<()> {
    let mut tx = character_db
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start loot-race relocation transaction: {error}"))?;
    // C++ Player::LoadFromDB and Rust `restored_saved_health_like_cpp` both
    // clamp this sentinel to the recomputed max health on login. The original
    // health remains in CharacterCoreSnapshot and cleanup restores it exactly;
    // powers are intentionally untouched.
    for (index, fixture) in fixture_characters.iter().enumerate() {
        let player_x = x + 1.0 + index as f64;
        let player_orientation = 0.0_f64.atan2(x - player_x);
        tx.exec_drop(
            "UPDATE characters SET map = :new_map, zone = 0, instance_id = 0, \
                    position_x = :new_x, position_y = :new_y, position_z = :new_z, \
                    orientation = :new_orientation, health = :new_health \
             WHERE guid = :guid AND account = :account AND online = 0 AND at_login = 0 \
               AND map = :old_map AND zone = :old_zone AND instance_id = :old_instance \
               AND position_x = :old_x AND position_y = :old_y AND position_z = :old_z \
               AND orientation = :old_orientation AND health = :old_health",
            mysql::params! {
                "new_map" => u32::from(map_id),
                "new_x" => player_x,
                "new_y" => y,
                "new_z" => z,
                "new_orientation" => player_orientation,
                "new_health" => u32::MAX,
                "guid" => fixture.bot.character_guid,
                "account" => fixture.bot.account_id,
                "old_map" => fixture.position.map_id,
                "old_zone" => fixture.position.zone_id,
                "old_instance" => fixture.position.instance_id,
                "old_x" => fixture.position.x,
                "old_y" => fixture.position.y,
                "old_z" => fixture.position.z,
                "old_orientation" => fixture.position.orientation,
                "old_health" => fixture.core.health,
            },
        )
        .map_err(|error| anyhow!("Relocate loot-race character: {error}"))?;
        if tx.affected_rows() != 1 {
            bail!(
                "loot-race character {} drifted after its durable snapshot; relocation applied to {} rows",
                fixture.bot.character_guid,
                tx.affected_rows()
            );
        }
    }
    tx.commit()
        .map_err(|error| anyhow!("Commit loot-race relocation: {error}"))?;
    Ok(())
}

fn validate_unique_sql_spawn(
    spawns: &[u64],
    configured_spawn: u64,
    entry: u32,
    map_id: u16,
) -> Result<()> {
    if spawns.len() != 1 {
        bail!(
            "loot-race target entry {entry} map {map_id} has {} SQL spawns ({spawns:?}); runtime GUID auto-discovery requires exactly one",
            spawns.len()
        );
    }
    if spawns[0] != configured_spawn {
        bail!(
            "loot-race target entry {entry} map {map_id} uniquely resolves SQL spawn {}, not configured spawn {configured_spawn}",
            spawns[0]
        );
    }
    Ok(())
}

fn ensure_no_online_characters(conn: &mut mysql::Conn, stage: &str) -> Result<()> {
    let online: u64 = conn
        .query_first("SELECT COUNT(*) FROM characters WHERE online <> 0")
        .map_err(|error| anyhow!("Check global online-character isolation at {stage}: {error}"))?
        .unwrap_or(0);
    validate_online_character_count(online, stage)
}

fn validate_online_character_count(online: u64, stage: &str) -> Result<()> {
    if online != 0 {
        bail!(
            "loot capture requires exclusive world access at {stage}, but {online} character(s) are marked online"
        );
    }
    Ok(())
}

fn load_character_progress_snapshot(
    conn: &mut mysql::Conn,
    guid_a: u64,
    guid_b: u64,
) -> Result<CharacterProgressSnapshot> {
    let guids = (guid_a, guid_b);
    Ok(CharacterProgressSnapshot {
        achievements: conn
            .exec(
                "SELECT guid, achievement, `date` FROM character_achievement \
                 WHERE guid IN (?, ?) ORDER BY guid, achievement",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character achievements: {error}"))?,
        achievement_progress: conn
            .exec(
                "SELECT guid, criteria, counter, `date` FROM character_achievement_progress \
                 WHERE guid IN (?, ?) ORDER BY guid, criteria",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character achievement criteria: {error}"))?,
        quest_status: conn
            .exec(
                "SELECT guid, quest, status, explored, acceptTime, endTime \
                 FROM character_queststatus WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest status: {error}"))?,
        quest_daily: conn
            .exec(
                "SELECT guid, quest, `time` FROM character_queststatus_daily \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character daily quests: {error}"))?,
        quest_monthly: conn
            .exec(
                "SELECT guid, quest FROM character_queststatus_monthly \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character monthly quests: {error}"))?,
        quest_objectives: conn
            .exec(
                "SELECT guid, quest, objective, data FROM character_queststatus_objectives \
                 WHERE guid IN (?, ?) ORDER BY guid, quest, objective",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest objectives: {error}"))?,
        quest_objective_criteria: conn
            .exec(
                "SELECT guid, questObjectiveId FROM character_queststatus_objectives_criteria \
                 WHERE guid IN (?, ?) ORDER BY guid, questObjectiveId",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest objective criteria: {error}"))?,
        quest_objective_criteria_progress: conn
            .exec(
                "SELECT guid, criteriaId, counter, `date` \
                 FROM character_queststatus_objectives_criteria_progress \
                 WHERE guid IN (?, ?) ORDER BY guid, criteriaId",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character quest criteria progress: {error}"))?,
        quest_rewarded: conn
            .exec(
                "SELECT guid, quest, active FROM character_queststatus_rewarded \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character rewarded quests: {error}"))?,
        quest_seasonal: conn
            .exec(
                "SELECT guid, quest, event, completedTime FROM character_queststatus_seasonal \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character seasonal quests: {error}"))?,
        quest_weekly: conn
            .exec(
                "SELECT guid, quest FROM character_queststatus_weekly \
                 WHERE guid IN (?, ?) ORDER BY guid, quest",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character weekly quests: {error}"))?,
        reputation: conn
            .exec(
                "SELECT guid, faction, standing, flags FROM character_reputation \
                 WHERE guid IN (?, ?) ORDER BY guid, faction",
                guids,
            )
            .map_err(|error| anyhow!("Snapshot character reputation: {error}"))?,
    })
}

fn restore_character_progress_snapshot(
    tx: &mut mysql::Transaction<'_>,
    guid_a: u64,
    guid_b: u64,
    snapshot: &CharacterProgressSnapshot,
) -> Result<()> {
    for table in CHARACTER_PROGRESS_TABLES {
        tx.exec_drop(
            format!("DELETE FROM `{table}` WHERE guid IN (?, ?)"),
            (guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Clear loot-fixture progress table {table}: {error}"))?;
    }

    for &(guid, achievement, date) in &snapshot.achievements {
        tx.exec_drop(
            "INSERT INTO character_achievement (guid, achievement, `date`) VALUES (?, ?, ?)",
            (guid, achievement, date),
        )
        .map_err(|error| anyhow!("Restore character achievements: {error}"))?;
    }
    for &(guid, criteria, counter, date) in &snapshot.achievement_progress {
        tx.exec_drop(
            "INSERT INTO character_achievement_progress (guid, criteria, counter, `date`) \
             VALUES (?, ?, ?, ?)",
            (guid, criteria, counter, date),
        )
        .map_err(|error| anyhow!("Restore character achievement criteria: {error}"))?;
    }
    for &(guid, quest, status, explored, accept_time, end_time) in &snapshot.quest_status {
        tx.exec_drop(
            "INSERT INTO character_queststatus \
             (guid, quest, status, explored, acceptTime, endTime) VALUES (?, ?, ?, ?, ?, ?)",
            (guid, quest, status, explored, accept_time, end_time),
        )
        .map_err(|error| anyhow!("Restore character quest status: {error}"))?;
    }
    for &(guid, quest, time) in &snapshot.quest_daily {
        tx.exec_drop(
            "INSERT INTO character_queststatus_daily (guid, quest, `time`) VALUES (?, ?, ?)",
            (guid, quest, time),
        )
        .map_err(|error| anyhow!("Restore character daily quests: {error}"))?;
    }
    for &(guid, quest) in &snapshot.quest_monthly {
        tx.exec_drop(
            "INSERT INTO character_queststatus_monthly (guid, quest) VALUES (?, ?)",
            (guid, quest),
        )
        .map_err(|error| anyhow!("Restore character monthly quests: {error}"))?;
    }
    for &(guid, quest, objective, data) in &snapshot.quest_objectives {
        tx.exec_drop(
            "INSERT INTO character_queststatus_objectives (guid, quest, objective, data) \
             VALUES (?, ?, ?, ?)",
            (guid, quest, objective, data),
        )
        .map_err(|error| anyhow!("Restore character quest objectives: {error}"))?;
    }
    for &(guid, objective) in &snapshot.quest_objective_criteria {
        tx.exec_drop(
            "INSERT INTO character_queststatus_objectives_criteria (guid, questObjectiveId) \
             VALUES (?, ?)",
            (guid, objective),
        )
        .map_err(|error| anyhow!("Restore character quest objective criteria: {error}"))?;
    }
    for &(guid, criteria, counter, date) in &snapshot.quest_objective_criteria_progress {
        tx.exec_drop(
            "INSERT INTO character_queststatus_objectives_criteria_progress \
             (guid, criteriaId, counter, `date`) VALUES (?, ?, ?, ?)",
            (guid, criteria, counter, date),
        )
        .map_err(|error| anyhow!("Restore character quest criteria progress: {error}"))?;
    }
    for &(guid, quest, active) in &snapshot.quest_rewarded {
        tx.exec_drop(
            "INSERT INTO character_queststatus_rewarded (guid, quest, active) VALUES (?, ?, ?)",
            (guid, quest, active),
        )
        .map_err(|error| anyhow!("Restore character rewarded quests: {error}"))?;
    }
    for &(guid, quest, event, completed_time) in &snapshot.quest_seasonal {
        tx.exec_drop(
            "INSERT INTO character_queststatus_seasonal (guid, quest, event, completedTime) \
             VALUES (?, ?, ?, ?)",
            (guid, quest, event, completed_time),
        )
        .map_err(|error| anyhow!("Restore character seasonal quests: {error}"))?;
    }
    for &(guid, quest) in &snapshot.quest_weekly {
        tx.exec_drop(
            "INSERT INTO character_queststatus_weekly (guid, quest) VALUES (?, ?)",
            (guid, quest),
        )
        .map_err(|error| anyhow!("Restore character weekly quests: {error}"))?;
    }
    for &(guid, faction, standing, flags) in &snapshot.reputation {
        tx.exec_drop(
            "INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ?, ?)",
            (guid, faction, standing, flags),
        )
        .map_err(|error| anyhow!("Restore character reputation: {error}"))?;
    }
    Ok(())
}

fn load_character_core_snapshot(
    conn: &mut mysql::Conn,
    character_guid: u64,
) -> Result<CharacterCoreSnapshot> {
    let row: mysql::Row = conn
        .exec_first(
            "SELECT level, xp, health, \
                    power1, power2, power3, power4, power5, power6, power7, power8, power9, power10, \
                    restState, rest_bonus, exploredZones, knownTitles, chosenTitle \
             FROM characters WHERE guid = ?",
            (character_guid,),
        )
        .map_err(|error| anyhow!("Reload loot-fixture character core state: {error}"))?
        .ok_or_else(|| anyhow!("Loot-fixture character {character_guid} disappeared"))?;
    Ok(CharacterCoreSnapshot {
        level: required_row_value(&row, "level")?,
        xp: required_row_value(&row, "xp")?,
        health: required_row_value(&row, "health")?,
        powers: [
            required_row_value(&row, "power1")?,
            required_row_value(&row, "power2")?,
            required_row_value(&row, "power3")?,
            required_row_value(&row, "power4")?,
            required_row_value(&row, "power5")?,
            required_row_value(&row, "power6")?,
            required_row_value(&row, "power7")?,
            required_row_value(&row, "power8")?,
            required_row_value(&row, "power9")?,
            required_row_value(&row, "power10")?,
        ],
        rest_state: required_row_value(&row, "restState")?,
        rest_bonus: required_row_value(&row, "rest_bonus")?,
        explored_zones: required_row_value(&row, "exploredZones")?,
        known_titles: required_row_value(&row, "knownTitles")?,
        chosen_title: required_row_value(&row, "chosenTitle")?,
    })
}

async fn expected_persisted_item_grant(
    fixture: &LootRaceFixture,
    sync: &LootRaceSync,
) -> Result<ExpectedPersistedItemGrant> {
    let window = sync.windows.lock().await[0]
        .clone()
        .ok_or_else(|| anyhow!("loot-race passed without a retained item window"))?;
    let (owner_low, owner_high) = sync
        .runtime_guid
        .lock()
        .map_err(|_| anyhow!("loot-race runtime GUID state was poisoned"))?
        .as_ref()
        .copied()
        .ok_or_else(|| anyhow!("loot-race passed without a retained world-object ObjectGuid"))?;
    let expected_removal = LootRemovedEvidence {
        owner_low,
        owner_high,
        loot_low: window.loot_low,
        loot_high: window.loot_high,
        loot_list_id: window.loot_list_id,
    };
    let evidence = sync.evidence.lock().await;
    validate_atomic_item_wire_outcome_like_cpp(
        &evidence,
        [
            fixture.characters[0].bot.character_guid,
            fixture.characters[1].bot.character_guid,
        ],
        fixture.target.item_entry,
        window.quantity,
        expected_removal,
        realm_id(),
    )
}

async fn expected_persisted_money_grant(
    fixture: &LootRaceFixture,
    sync: &LootRaceSync,
    source_coins: u64,
) -> Result<ExpectedPersistedMoneyGrant> {
    if source_coins != u64::from(RACE_GAMEOBJECT_MONEY) {
        bail!(
            "shared chest source money was {source_coins}, expected exact guarded pool {RACE_GAMEOBJECT_MONEY}"
        );
    }
    let window = sync.windows.lock().await[0]
        .clone()
        .ok_or_else(|| anyhow!("loot-race passed without a retained money window"))?;
    let evidence = sync.evidence.lock().await;
    let winner = validate_serialized_gameobject_money_wire_outcome_like_cpp(
        &evidence,
        (window.loot_low, window.loot_high),
        source_coins,
    )?;
    Ok(ExpectedPersistedMoneyGrant {
        owner_guid: fixture.characters[winner].bot.character_guid,
        amount: source_coins,
    })
}

fn validate_persisted_item_grant_like_cpp(
    expected: ExpectedPersistedItemGrant,
    persisted: PersistedItemGrantRow,
) -> Result<()> {
    let quantity = u32::try_from(expected.push.quantity)
        .map_err(|_| anyhow!("wire item quantity was negative"))?;
    let quantity_in_inventory = u32::try_from(expected.push.quantity_in_inventory)
        .map_err(|_| anyhow!("wire inventory quantity was negative"))?;
    let slot_in_bag = u8::try_from(expected.push.slot_in_bag)
        .map_err(|_| anyhow!("wire SlotInBag did not fit a persisted inventory slot"))?;

    if persisted.item_guid != expected.push.item_guid_low
        || persisted.owner_guid != expected.owner_guid
        || persisted.owner_guid != expected.push.player_low
        || persisted.item_entry != expected.push.item_entry
        || persisted.count != quantity
        || persisted.count != quantity_in_inventory
    {
        bail!(
            "wire-keyed persisted item {:?} did not match grant owner/item/count {:?}",
            persisted,
            expected
        );
    }
    // For this clean fixture CanStoreNewItem chooses a free top-level
    // backpack slot. C++ serializes that as wire Slot=255 but persists bag=0;
    // those values are intentionally not compared numerically.
    if expected.push.slot != INVENTORY_SLOT_BAG_0
        || persisted.inventory_owner != Some(expected.owner_guid)
        || persisted.bag_guid != Some(0)
        || persisted.slot != Some(slot_in_bag)
        || persisted.bag_slot.is_some()
    {
        bail!(
            "wire slot {}/{} did not bind to the expected top-level character_inventory row {:?}",
            expected.push.slot,
            expected.push.slot_in_bag,
            persisted
        );
    }
    Ok(())
}

fn verify_persisted_grants(
    fixture: &LootRaceFixture,
    expected_money_grant: ExpectedPersistedMoneyGrant,
    expected_item_grant: ExpectedPersistedItemGrant,
) -> Result<(
    u64,
    u64,
    ExpectedPersistedItemGrant,
    ExpectedPersistedMoneyGrant,
)> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_both_offline(&mut conn, fixture)?;
    ensure_no_online_characters(&mut conn, "loot-race persistence verification")?;
    let guids = (
        fixture.characters[0].bot.character_guid,
        fixture.characters[1].bot.character_guid,
    );
    let (item_total, item_rows, inventory_rows, persisted_item_owner): (u64, u64, u64, u64) = conn
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0), COUNT(DISTINCT ii.guid), \
                    COUNT(DISTINCT ci.item), COALESCE(MIN(ii.owner_guid), 0) \
             FROM item_instance ii \
             LEFT JOIN character_inventory ci \
               ON ci.item = ii.guid AND ci.guid = ii.owner_guid \
             WHERE ii.itemEntry = ? AND ii.owner_guid IN (?, ?)",
            (fixture.target.item_entry, guids.0, guids.1),
        )
        .map_err(|error| anyhow!("Verify loot-race item persistence: {error}"))?
        .ok_or_else(|| anyhow!("loot-race item aggregate query returned no row"))?;
    let current_money: Vec<(u64, u64)> = conn
        .exec(
            "SELECT guid, money FROM characters WHERE guid IN (?, ?) ORDER BY guid",
            guids,
        )
        .map_err(|error| anyhow!("Verify loot-race money: {error}"))?;
    if current_money.len() != 2 {
        bail!("loot-race character rows disappeared during verification");
    }
    if item_total != 1 || item_rows != 1 || inventory_rows != 1 {
        bail!(
            "atomic ITEM race persisted quantity/instances/inventory rows {item_total}/{item_rows}/{inventory_rows}; expected 1/1/1"
        );
    }
    if persisted_item_owner != expected_item_grant.owner_guid {
        bail!(
            "atomic ITEM race persisted owner {persisted_item_owner}, but wire winner was character {}",
            expected_item_grant.owner_guid
        );
    }
    let persisted_tuple: Option<(
        u64,
        u64,
        u32,
        u32,
        Option<u64>,
        Option<u64>,
        Option<u8>,
        Option<u8>,
    )> = conn
        .exec_first(
            "SELECT ii.guid, ii.owner_guid, ii.itemEntry, ii.count, \
                    ci.guid, ci.bag, ci.slot, bag_ci.slot \
             FROM item_instance ii \
             LEFT JOIN character_inventory ci \
               ON ci.item = ii.guid \
             LEFT JOIN character_inventory bag_ci \
               ON ci.bag <> 0 AND bag_ci.item = ci.bag AND bag_ci.guid = ci.guid \
             WHERE ii.guid = ?",
            (expected_item_grant.push.item_guid_low,),
        )
        .map_err(|error| anyhow!("Verify wire-keyed loot-race item persistence: {error}"))?;
    let persisted = persisted_tuple
        .map(
            |(
                item_guid,
                owner_guid,
                item_entry,
                count,
                inventory_owner,
                bag_guid,
                slot,
                bag_slot,
            )| PersistedItemGrantRow {
                item_guid,
                owner_guid,
                item_entry,
                count,
                inventory_owner,
                bag_guid,
                slot,
                bag_slot,
            },
        )
        .ok_or_else(|| {
            anyhow!(
                "wire ItemGUID counter {} did not identify a persisted item_instance row",
                expected_item_grant.push.item_guid_low
            )
        })?;
    validate_persisted_item_grant_like_cpp(expected_item_grant, persisted)?;
    let mut money_delta = 0u64;
    for character in &fixture.characters {
        let current = current_money
            .iter()
            .find_map(|(guid, money)| (*guid == character.bot.character_guid).then_some(*money))
            .ok_or_else(|| {
                anyhow!(
                    "loot-race money verification omitted character {}",
                    character.bot.character_guid
                )
            })?;
        let character_delta = if character.bot.character_guid == expected_money_grant.owner_guid {
            expected_money_grant.amount
        } else {
            0
        };
        let expected = character
            .money
            .checked_add(character_delta)
            .ok_or_else(|| anyhow!("loot-race expected money overflow"))?;
        if current != expected {
            bail!(
                "atomic GAMEOBJECT MONEY race persisted character {} money {}; expected {} from wire winner {} and whole-pool amount {}",
                character.bot.character_guid,
                current,
                expected,
                expected_money_grant.owner_guid,
                expected_money_grant.amount
            );
        }
        money_delta = money_delta
            .checked_add(current - character.money)
            .ok_or_else(|| anyhow!("loot-race persisted money delta overflow"))?;
    }
    if money_delta != expected_money_grant.amount {
        bail!(
            "atomic GAMEOBJECT MONEY race persisted total delta {money_delta}; expected one exact C++ whole-pool grant {}",
            expected_money_grant.amount
        );
    }
    Ok((
        item_total,
        money_delta,
        expected_item_grant,
        expected_money_grant,
    ))
}

fn verify_single_item_capture_persistence(fixture: &LootRaceFixture) -> Result<u64> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_both_offline(&mut conn, fixture)?;
    ensure_no_online_characters(&mut conn, "loot-item capture persistence verification")?;

    let owner = fixture.characters[0].bot.character_guid;
    let offline_peer = fixture.characters[1].bot.character_guid;
    let (item_total, item_rows, inventory_rows, persisted_owner): (u64, u64, u64, u64) = conn
        .exec_first(
            "SELECT COALESCE(SUM(ii.count), 0), COUNT(DISTINCT ii.guid), \
                    COUNT(DISTINCT ci.item), COALESCE(MIN(ii.owner_guid), 0) \
             FROM item_instance ii \
             LEFT JOIN character_inventory ci \
               ON ci.item = ii.guid AND ci.guid = ii.owner_guid \
             WHERE ii.itemEntry = ? AND ii.owner_guid IN (?, ?)",
            (fixture.target.item_entry, owner, offline_peer),
        )
        .map_err(|error| anyhow!("Verify loot-item capture persistence: {error}"))?
        .ok_or_else(|| anyhow!("loot-item capture aggregate query returned no row"))?;
    if (item_total, item_rows, inventory_rows, persisted_owner) != (1, 1, 1, owner) {
        bail!(
            "single-session item capture persisted quantity/instances/inventory/owner {item_total}/{item_rows}/{inventory_rows}/{persisted_owner}; expected 1/1/1/{owner}"
        );
    }
    let persisted_slots: Vec<(u64, u8)> = conn
        .exec(
            "SELECT ci.bag, ci.slot FROM character_inventory ci \
             JOIN item_instance ii ON ii.guid = ci.item \
             WHERE ii.itemEntry = ? AND ii.owner_guid = ? AND ci.guid = ?",
            (fixture.target.item_entry, owner, owner),
        )
        .map_err(|error| anyhow!("Verify loot-item capture keyring slot: {error}"))?;
    if persisted_slots.as_slice() != [(0, LOOT_ITEM_CAPTURE_KEYRING_SLOT)] {
        bail!(
            "single-session item capture persisted item in slots {persisted_slots:?}; expected exact top-level keyring slot 0/{LOOT_ITEM_CAPTURE_KEYRING_SLOT}"
        );
    }

    let money_rows: Vec<(u64, u64)> = conn
        .exec(
            "SELECT guid, money FROM characters WHERE guid IN (?, ?) ORDER BY guid",
            (owner, offline_peer),
        )
        .map_err(|error| anyhow!("Verify loot-item capture money isolation: {error}"))?;
    if money_rows.len() != 2 {
        bail!("loot-item capture character rows disappeared during verification");
    }
    for character in &fixture.characters {
        let current = money_rows
            .iter()
            .find_map(|(guid, money)| (*guid == character.bot.character_guid).then_some(*money))
            .ok_or_else(|| {
                anyhow!(
                    "loot-item capture money verification omitted character {}",
                    character.bot.character_guid
                )
            })?;
        if current != character.money {
            bail!(
                "item-only capture changed character {} money from {} to {} without CMSG_LOOT_MONEY",
                character.bot.character_guid,
                character.money,
                current
            );
        }
    }

    Ok(item_total)
}

fn cleanup_fixture(fixture: &LootRaceFixture) -> Result<()> {
    let url = characters_db_url()?;
    let opts = loot_db_opts(&url, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_both_offline(&mut conn, fixture)?;
    let guid_a = fixture.characters[0].bot.character_guid;
    let guid_b = fixture.characters[1].bot.character_guid;
    let gained_items: Vec<u64> = conn
        .exec_map(
            "SELECT guid FROM item_instance WHERE itemEntry = ? AND owner_guid IN (?, ?)",
            (fixture.target.item_entry, guid_a, guid_b),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Load gained loot-race items for cleanup: {error}"))?;
    let group_ids: Vec<u32> = conn
        .exec_map(
            "SELECT DISTINCT guid FROM group_member WHERE memberGuid IN (?, ?) \
             UNION SELECT guid FROM `groups` WHERE leaderGuid IN (?, ?)",
            (guid_a, guid_b, guid_a, guid_b),
            |guid: u32| guid,
        )
        .map_err(|error| anyhow!("Load loot-race groups for cleanup: {error}"))?;
    for group_id in &group_ids {
        let unrelated_members: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM group_member \
                 WHERE guid = ? AND memberGuid NOT IN (?, ?)",
                (*group_id, guid_a, guid_b),
            )
            .map_err(|error| anyhow!("Check loot-race group cleanup scope: {error}"))?
            .unwrap_or(0);
        if unrelated_members != 0 {
            bail!(
                "refusing to delete loot-race group {group_id}: it gained {unrelated_members} unrelated members"
            );
        }
    }
    if let Some(expected_state) = fixture.gameobject_state {
        let world_url = world_db_url()?;
        let world_opts = loot_db_opts(&world_url, "world")?;
        let mut world = mysql::Conn::new(world_opts).map_err(|error| {
            anyhow!("Connect to world DB during GameObject cleanup failed: {error}")
        })?;
        let observed: Option<u8> = world
            .exec_first(
                "SELECT state FROM gameobject WHERE guid = ? AND id = ?",
                (fixture.target.spawn_guid, fixture.target.entry),
            )
            .map_err(|error| anyhow!("Verify GameObject fixture SQL state: {error}"))?;
        if observed != Some(expected_state) {
            bail!(
                "GameObject fixture SQL state drifted: expected {expected_state}, got {observed:?}; refusing to hide drift or restore normal PM2"
            );
        }
    }
    let observed_respawn: Vec<(i64, u16, u32)> = conn
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = ? AND spawnId = ? ORDER BY mapId, instanceId, respawnTime",
            (fixture.respawn_type, fixture.target.spawn_guid),
        )
        .map_err(|error| anyhow!("Inspect loot-fixture respawn rows before cleanup: {error}"))?;
    let baseline_respawn = fixture
        .respawn
        .iter()
        .map(|row| (row.respawn_time, row.map_id, row.instance_id))
        .collect::<Vec<_>>();
    let generated_respawn = validate_respawn_cleanup_scope(
        fixture.target.map_id,
        &baseline_respawn,
        &observed_respawn,
    )?;
    let mut tx = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start loot-race cleanup transaction: {error}"))?;
    for item_guid in gained_items {
        tx.exec_drop(
            "DELETE FROM character_inventory WHERE item = ?",
            (item_guid,),
        )
        .map_err(|error| anyhow!("Delete loot-race inventory row: {error}"))?;
        tx.exec_drop(
            "DELETE FROM item_instance_gems WHERE itemGuid = ?",
            (item_guid,),
        )
        .map_err(|error| anyhow!("Delete loot-race gem row: {error}"))?;
        tx.exec_drop("DELETE FROM item_instance WHERE guid = ?", (item_guid,))
            .map_err(|error| anyhow!("Delete loot-race item row: {error}"))?;
    }
    for group_id in group_ids {
        tx.exec_drop("DELETE FROM group_member WHERE guid = ?", (group_id,))
            .map_err(|error| anyhow!("Delete loot-race group members: {error}"))?;
        tx.exec_drop("DELETE FROM `groups` WHERE guid = ?", (group_id,))
            .map_err(|error| anyhow!("Delete loot-race group row: {error}"))?;
    }
    restore_character_progress_snapshot(&mut tx, guid_a, guid_b, &fixture.progress)?;
    for character in &fixture.characters {
        tx.exec_drop(
            "UPDATE characters SET \
                    money = :money, level = :level, xp = :xp, health = :health, \
                    power1 = :power1, power2 = :power2, power3 = :power3, \
                    power4 = :power4, power5 = :power5, power6 = :power6, \
                    power7 = :power7, power8 = :power8, power9 = :power9, power10 = :power10, \
                    restState = :rest_state, rest_bonus = :rest_bonus, \
                    exploredZones = :explored_zones, knownTitles = :known_titles, \
                    chosenTitle = :chosen_title, \
                    map = :map, zone = :zone, instance_id = :instance_id, \
                    position_x = :position_x, position_y = :position_y, \
                    position_z = :position_z, orientation = :orientation \
             WHERE guid = :guid AND online = 0",
            mysql::params! {
                "money" => character.money,
                "level" => character.core.level,
                "xp" => character.core.xp,
                "health" => character.core.health,
                "power1" => character.core.powers[0],
                "power2" => character.core.powers[1],
                "power3" => character.core.powers[2],
                "power4" => character.core.powers[3],
                "power5" => character.core.powers[4],
                "power6" => character.core.powers[5],
                "power7" => character.core.powers[6],
                "power8" => character.core.powers[7],
                "power9" => character.core.powers[8],
                "power10" => character.core.powers[9],
                "rest_state" => character.core.rest_state,
                "rest_bonus" => character.core.rest_bonus,
                "explored_zones" => character.core.explored_zones.clone(),
                "known_titles" => character.core.known_titles.clone(),
                "chosen_title" => character.core.chosen_title,
                "map" => character.position.map_id,
                "zone" => character.position.zone_id,
                "instance_id" => character.position.instance_id,
                "position_x" => character.position.x,
                "position_y" => character.position.y,
                "position_z" => character.position.z,
                "orientation" => character.position.orientation,
                "guid" => character.bot.character_guid,
            },
        )
        .map_err(|error| anyhow!("Restore loot-race character snapshot: {error}"))?;
        if tx.affected_rows() != 1 {
            bail!(
                "loot-race character {} became online or disappeared during cleanup",
                character.bot.character_guid
            );
        }
    }
    if fixture.target.kind == LootRaceTargetKind::Creature {
        if let Some((respawn_time, map_id, instance_id)) = generated_respawn {
            tx.exec_drop(
                "DELETE FROM respawn \
                 WHERE type = ? AND spawnId = ? AND respawnTime = ? AND mapId = ? AND instanceId = ?",
                (
                    fixture.respawn_type,
                    fixture.target.spawn_guid,
                    respawn_time,
                    map_id,
                    instance_id,
                ),
            )
            .map_err(|error| anyhow!("Delete exact generated loot respawn row: {error}"))?;
            if tx.affected_rows() != 1 {
                bail!(
                    "exact generated respawn row drifted before cleanup; deleted {} rows",
                    tx.affected_rows()
                );
            }
        }
    }
    tx.commit()
        .map_err(|error| anyhow!("Commit loot-race cleanup: {error}"))?;

    ensure_no_online_characters(&mut conn, "loot fixture cleanup")?;
    let remaining_expected_items: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM item_instance WHERE itemEntry = ? AND owner_guid IN (?, ?)",
            (fixture.target.item_entry, guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Verify loot-fixture item cleanup: {error}"))?
        .unwrap_or(0);
    if remaining_expected_items != 0 {
        bail!(
            "loot fixture cleanup left {remaining_expected_items} expected-item instance row(s) behind"
        );
    }
    let remaining_groups: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM (\
                SELECT guid FROM group_member WHERE memberGuid IN (?, ?) \
                UNION ALL \
                SELECT guid FROM `groups` WHERE leaderGuid IN (?, ?)\
             ) AS persisted_group_state",
            (guid_a, guid_b, guid_a, guid_b),
        )
        .map_err(|error| anyhow!("Verify loot-fixture group cleanup: {error}"))?
        .unwrap_or(0);
    if remaining_groups != 0 {
        bail!("loot fixture cleanup left {remaining_groups} persisted group row(s) behind");
    }
    if fixture.target.kind == LootRaceTargetKind::Creature {
        let occupied_capture_slot: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM character_inventory WHERE guid = ? AND bag = 0 AND slot = ?",
                (guid_a, LOOT_ITEM_CAPTURE_KEYRING_SLOT),
            )
            .map_err(|error| anyhow!("Verify loot-item keyring-slot cleanup: {error}"))?
            .unwrap_or(0);
        if occupied_capture_slot != 0 {
            bail!(
                "loot fixture cleanup did not restore exact empty keyring slot 0/{LOOT_ITEM_CAPTURE_KEYRING_SLOT}"
            );
        }
    }
    let restored_respawn: Vec<(i64, u16, u32)> = conn
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = ? AND spawnId = ? ORDER BY mapId, instanceId",
            (fixture.respawn_type, fixture.target.spawn_guid),
        )
        .map_err(|error| anyhow!("Verify loot-fixture respawn cleanup: {error}"))?;
    let expected_respawn = if fixture.target.kind == LootRaceTargetKind::GameObject {
        observed_respawn
    } else {
        baseline_respawn
    };
    if restored_respawn != expected_respawn {
        bail!("loot fixture target respawn rows did not restore exactly");
    }
    let restored_progress = load_character_progress_snapshot(&mut conn, guid_a, guid_b)?;
    if restored_progress != fixture.progress {
        bail!("loot fixture quest/achievement/criteria/reputation rows did not restore exactly");
    }
    for character in &fixture.characters {
        let restored_core = load_character_core_snapshot(&mut conn, character.bot.character_guid)?;
        if restored_core != character.core {
            bail!(
                "loot fixture character {} XP/level/health/rest/exploration/title state did not restore exactly",
                character.bot.character_guid
            );
        }
        let restored_position_and_money: (u64, u32, u32, u32, f64, f64, f64, f32) = conn
            .exec_first(
                "SELECT money, map, zone, instance_id, position_x, position_y, position_z, orientation \
                 FROM characters WHERE guid = ?",
                (character.bot.character_guid,),
            )
            .map_err(|error| anyhow!("Reload loot-fixture money/position state: {error}"))?
            .ok_or_else(|| {
                anyhow!(
                    "Loot-fixture character {} disappeared during cleanup verification",
                    character.bot.character_guid
                )
            })?;
        let expected_position_and_money = (
            character.money,
            character.position.map_id,
            character.position.zone_id,
            character.position.instance_id,
            character.position.x,
            character.position.y,
            character.position.z,
            character.position.orientation,
        );
        if restored_position_and_money != expected_position_and_money {
            bail!(
                "loot fixture character {} money/position state did not restore exactly",
                character.bot.character_guid
            );
        }
    }

    fixture.journal.complete()?;
    Ok(())
}

fn validate_respawn_cleanup_scope(
    target_map: u16,
    baseline: &[(i64, u16, u32)],
    observed: &[(i64, u16, u32)],
) -> Result<Option<(i64, u16, u32)>> {
    if observed == baseline {
        return Ok(None);
    }
    if !baseline.is_empty() || observed.len() != 1 {
        bail!(
            "loot fixture respawn drift: baseline={baseline:?}, observed={observed:?}; refusing broad cleanup"
        );
    }
    let row = observed[0];
    if row.0 <= 0 || row.1 != target_map || row.2 != 0 {
        bail!(
            "loot fixture generated respawn has wrong time/map/instance: {row:?}, expected positive/{target_map}/0"
        );
    }
    Ok(Some(row))
}

fn wait_both_offline(conn: &mut mysql::Conn, fixture: &LootRaceFixture) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(LOOT_FIXTURE_OFFLINE_WAIT_SECS);
    loop {
        let online: u64 = conn
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE guid IN (?, ?) AND online <> 0",
                (
                    fixture.characters[0].bot.character_guid,
                    fixture.characters[1].bot.character_guid,
                ),
            )
            .map_err(|error| anyhow!("Check loot-race offline state: {error}"))?
            .unwrap_or(2);
        if online == 0 {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "loot-race characters remained online; refusing fixture restoration before disconnect saves"
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn faction_for_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 | 11 | 22 | 25 => 0,
        _ => 1,
    }
}

fn current_millis_u32() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

#[derive(Debug, Clone)]
struct GroupCapacityFixture {
    leader_guid: u64,
    candidate_names: [String; 2],
    candidate_guids: [u64; 2],
    initial_member_guids: [u64; 4],
    party_settings: GroupCapacityPartySettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroupCapacityPartySettings {
    loot_method: u8,
    loot_threshold: u8,
    master_looter_guid: u64,
    dungeon_difficulty_id: u32,
    raid_difficulty_id: u32,
    legacy_raid_difficulty_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroupCapacityPersistenceEvidence {
    final_member_count: u64,
    winning_candidate_guid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupCapacityOutcome {
    Added,
    Full,
}

fn validate_group_capacity_initial_member_offline(member_guid: u64, online: u8) -> Result<()> {
    if online != 0 {
        bail!(
            "group-capacity initial member {member_guid} must be offline before the race; online={online}"
        );
    }
    Ok(())
}

fn load_group_capacity_fixture(
    bots: &[config::BotConfig],
    cli: &GroupCapacityRaceCli,
) -> Result<GroupCapacityFixture> {
    if cli.group_db_store_id == 0 {
        bail!("group-capacity race requires a nonzero preloaded --group-capacity-group-id");
    }
    if cli.timeout_secs == 0 {
        bail!("--group-capacity-timeout must be greater than zero");
    }

    let find_bot = |account: &str| {
        bots.iter()
            .find(|bot| bot.account.eq_ignore_ascii_case(account))
            .cloned()
            .ok_or_else(|| anyhow!("group-capacity account {account} was not selected"))
    };
    let leader = find_bot(&cli.leader_account)?;
    let candidate_a = find_bot(&cli.candidate_a_account)?;
    let candidate_b = find_bot(&cli.candidate_b_account)?;
    let selected_guids = [
        leader.character_guid,
        candidate_a.character_guid,
        candidate_b.character_guid,
    ];
    if selected_guids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != 3
    {
        bail!("group-capacity leader and candidates must be three distinct characters");
    }

    let opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let (
        loaded_leader,
        group_type,
        loot_method,
        loot_threshold,
        dungeon_difficulty_id,
        raid_difficulty_id,
        legacy_raid_difficulty_id,
        master_looter_guid,
    ): (u64, u16, u8, u8, u32, u32, u32, u64) = conn
        .exec_first(
            "SELECT leaderGuid, groupType, lootMethod, lootThreshold, difficulty, raidDifficulty, legacyRaidDifficulty, masterLooterGuid FROM `groups` WHERE guid = ?",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Load preseeded group-capacity group: {error}"))?
        .ok_or_else(|| {
            anyhow!(
                "preseeded group-capacity group {} does not exist; seed it before starting world-server",
                cli.group_db_store_id
            )
        })?;
    if loaded_leader != leader.character_guid || group_type != 0 {
        bail!(
            "group-capacity fixture must be a normal party led by GUID {}; found leader={loaded_leader} groupType={group_type}",
            leader.character_guid
        );
    }

    let initial_members: Vec<u64> = conn
        .exec(
            "SELECT memberGuid FROM group_member WHERE guid = ? ORDER BY memberGuid",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Load preseeded group-capacity members: {error}"))?;
    let initial_member_guids: [u64; 4] =
        initial_members.try_into().map_err(|members: Vec<u64>| {
            anyhow!(
                "group-capacity fixture must start with exactly four members, found {:?}",
                members
            )
        })?;
    if !initial_member_guids.contains(&leader.character_guid)
        || initial_member_guids.contains(&candidate_a.character_guid)
        || initial_member_guids.contains(&candidate_b.character_guid)
    {
        bail!(
            "group-capacity fixture must contain the leader and exclude both candidates; members={initial_member_guids:?}"
        );
    }

    let load_character = |conn: &mut mysql::Conn, guid: u64| -> Result<(String, u8, u8)> {
        conn.exec_first(
            "SELECT name, race, online FROM characters WHERE guid = ?",
            (guid,),
        )
        .map_err(|error| anyhow!("Load group-capacity character {guid}: {error}"))?
        .ok_or_else(|| anyhow!("group-capacity character {guid} does not exist"))
    };
    for member_guid in initial_member_guids {
        let (_, _, online) = load_character(&mut conn, member_guid)?;
        validate_group_capacity_initial_member_offline(member_guid, online)?;
    }

    let leader_character = load_character(&mut conn, leader.character_guid)?;
    let candidate_a_character = load_character(&mut conn, candidate_a.character_guid)?;
    let candidate_b_character = load_character(&mut conn, candidate_b.character_guid)?;
    if leader_character.2 != 0 || candidate_a_character.2 != 0 || candidate_b_character.2 != 0 {
        bail!("group-capacity leader and both candidates must be offline before the race");
    }
    let leader_faction = faction_for_race(leader_character.1);
    if faction_for_race(candidate_a_character.1) != leader_faction
        || faction_for_race(candidate_b_character.1) != leader_faction
    {
        bail!(
            "group-capacity leader and candidates must share a faction for deterministic invite acceptance; races were {}/{}/{}",
            leader_character.1,
            candidate_a_character.1,
            candidate_b_character.1
        );
    }

    Ok(GroupCapacityFixture {
        leader_guid: leader.character_guid,
        candidate_names: [candidate_a_character.0, candidate_b_character.0],
        candidate_guids: [candidate_a.character_guid, candidate_b.character_guid],
        initial_member_guids,
        party_settings: GroupCapacityPartySettings {
            loot_method,
            loot_threshold,
            master_looter_guid,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
        },
    })
}

fn validate_group_capacity_persisted_members(
    fixture: &GroupCapacityFixture,
    final_members: &[u64],
) -> Result<GroupCapacityPersistenceEvidence> {
    if final_members.len() != 5
        || !fixture
            .initial_member_guids
            .iter()
            .all(|guid| final_members.contains(guid))
    {
        bail!(
            "group-capacity race persisted {:?}; expected four initial members plus one candidate",
            final_members
        );
    }
    let persisted_candidates: Vec<_> = fixture
        .candidate_guids
        .iter()
        .copied()
        .filter(|guid| final_members.contains(guid))
        .collect();
    let [winning_candidate_guid] = persisted_candidates.as_slice() else {
        bail!(
            "group-capacity race persisted {} candidates instead of exactly one: {:?}",
            persisted_candidates.len(),
            final_members
        );
    };
    Ok(GroupCapacityPersistenceEvidence {
        final_member_count: final_members.len() as u64,
        winning_candidate_guid: *winning_candidate_guid,
    })
}

fn validate_group_capacity_winner_consistency(
    wire_winner_guid: u64,
    persisted_winner_guid: u64,
) -> Result<()> {
    if wire_winner_guid != persisted_winner_guid {
        bail!(
            "group-capacity winner mismatch: wire added GUID {wire_winner_guid}, CharacterDB persisted GUID {persisted_winner_guid}"
        );
    }
    Ok(())
}

fn verify_group_capacity_fixture(
    cli: &GroupCapacityRaceCli,
    fixture: &GroupCapacityFixture,
) -> Result<GroupCapacityPersistenceEvidence> {
    let opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let group_identity: Option<(u64, u16)> = conn
        .exec_first(
            "SELECT leaderGuid, groupType FROM `groups` WHERE guid = ?",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Verify group-capacity group row: {error}"))?;
    if group_identity != Some((fixture.leader_guid, 0)) {
        bail!(
            "group-capacity group row changed during the race: {group_identity:?}; expected leader {} and normal groupType 0",
            fixture.leader_guid
        );
    }
    let final_members: Vec<u64> = conn
        .exec(
            "SELECT memberGuid FROM group_member WHERE guid = ? ORDER BY memberGuid",
            (cli.group_db_store_id,),
        )
        .map_err(|error| anyhow!("Verify group-capacity member rows: {error}"))?;
    validate_group_capacity_persisted_members(fixture, &final_members)
}

async fn wait_group_capacity_barrier(
    options: &GroupCapacityRaceOptions,
    barrier: &Barrier,
    label: &str,
) -> Result<()> {
    options.sync.cancellation_error()?;
    tokio::time::timeout(Duration::from_secs(options.timeout_secs), async {
        tokio::select! {
            _ = barrier.wait() => Ok(()),
            _ = options.sync.cancelled.cancelled() => options.sync.cancellation_error(),
        }
    })
    .await
    .map_err(|_| anyhow!("group-capacity race timed out waiting for {label}"))??;
    Ok(())
}

fn validate_group_capacity_invite(payload: &[u8]) -> Result<()> {
    if read_msb_bits(payload, 0, 1) != Some(1) {
        bail!("group-capacity candidate received a PartyInvite that could not be accepted");
    }
    Ok(())
}

fn validate_group_full_result(payload: &[u8]) -> Result<()> {
    let name_len = read_msb_bits(payload, 0, 9)
        .ok_or_else(|| anyhow!("malformed group-capacity PartyCommandResult name length"))?;
    let operation = read_msb_bits(payload, 9, 4)
        .ok_or_else(|| anyhow!("malformed group-capacity PartyCommandResult operation"))?;
    let result = read_msb_bits(payload, 13, 6)
        .ok_or_else(|| anyhow!("malformed group-capacity PartyCommandResult result"))?;
    if name_len != 0 || operation != 0 || result != 4 {
        bail!(
            "group-capacity loser received PartyCommandResult name_len={name_len} operation={operation} result={result}; expected empty Invite/GROUP_FULL"
        );
    }
    validate_party_command_result_tail(payload, 0, "")?;
    Ok(())
}

fn validate_group_invite_ok_result(payload: &[u8], expected_name: &str) -> Result<()> {
    let name_len = read_msb_bits(payload, 0, 9)
        .ok_or_else(|| anyhow!("malformed group-capacity invite result name length"))?;
    let operation = read_msb_bits(payload, 9, 4)
        .ok_or_else(|| anyhow!("malformed group-capacity invite result operation"))?;
    let result = read_msb_bits(payload, 13, 6)
        .ok_or_else(|| anyhow!("malformed group-capacity invite result code"))?;
    if name_len != expected_name.len() as u32 || operation != 0 || result != 0 {
        bail!(
            "group-capacity invite for {expected_name} returned name_len={name_len} operation={operation} result={result}; expected Invite/OK"
        );
    }
    validate_party_command_result_tail(payload, name_len as usize, expected_name)?;
    Ok(())
}

fn validate_party_command_result_tail(
    payload: &[u8],
    name_len: usize,
    expected_name: &str,
) -> Result<()> {
    const BIT_HEADER_BYTES: usize = 3;
    const RESULT_DATA_END: usize = BIT_HEADER_BYTES + 4;
    if payload.get(2).is_none_or(|byte| byte & 0x1F != 0) {
        bail!("group-capacity PartyCommandResult had malformed bit padding");
    }
    let result_data = u32::from_le_bytes(
        payload
            .get(BIT_HEADER_BYTES..RESULT_DATA_END)
            .ok_or_else(|| anyhow!("group-capacity PartyCommandResult omitted ResultData"))?
            .try_into()
            .expect("exact PartyCommandResult ResultData slice"),
    );
    let (guid_len, guid_low, guid_high) = parse_packed_guid(
        payload
            .get(RESULT_DATA_END..)
            .ok_or_else(|| anyhow!("group-capacity PartyCommandResult omitted ResultGUID"))?,
    )
    .ok_or_else(|| anyhow!("group-capacity PartyCommandResult had malformed ResultGUID"))?;
    let name_start = RESULT_DATA_END + guid_len;
    let name_end = name_start
        .checked_add(name_len)
        .ok_or_else(|| anyhow!("group-capacity PartyCommandResult name length overflow"))?;
    let name = payload
        .get(name_start..name_end)
        .ok_or_else(|| anyhow!("group-capacity PartyCommandResult omitted its declared name"))?;
    if result_data != 0
        || (guid_low, guid_high) != (0, 0)
        || name != expected_name.as_bytes()
        || name_end != payload.len()
    {
        bail!(
            "group-capacity PartyCommandResult tail differed: ResultData={result_data} ResultGUID=({guid_low}, {guid_high}) name={name:?} trailing={} byte(s)",
            payload.len().saturating_sub(name_end)
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupCapacityPartyUpdateEvidence {
    CompleteRoster,
    ConnectedOnlyRoster,
}

fn validate_group_capacity_party_update(
    payload: &[u8],
    options: &GroupCapacityRaceOptions,
) -> Result<GroupCapacityPartyUpdateEvidence> {
    const EXPECTED_MEMBERS: u32 = 5;
    let mut cursor = PartyUpdateCursor::new(payload);
    let party_flags = cursor.read_u16("PartyFlags")?;
    let party_index = cursor.read_u8("PartyIndex")?;
    let party_type = cursor.read_u8("PartyType")?;
    let my_index = cursor.read_i32("MyIndex")?;
    let party_guid = cursor.read_packed_guid("PartyGUID")?;
    let _sequence_num = cursor.read_u32("SequenceNum")?;
    let leader_guid = cursor.read_packed_guid("LeaderGUID")?;
    let _leader_faction = cursor.read_u8("LeaderFactionGroup")?;
    let member_count = cursor.read_u32("PlayerList.size")?;
    let optional_bits = cursor.read_u8("optional-value bits")?;
    let has_lfg_info = optional_bits & 0x80 != 0;
    let has_loot_settings = optional_bits & 0x40 != 0;
    let has_difficulty_settings = optional_bits & 0x20 != 0;
    if party_flags != 0
        || party_index != 0
        || party_type != 1
        || party_guid == (0, 0)
        || leader_guid != create_player_guid_raw(options.leader_guid, realm_id())
        || !matches!(member_count, 2 | EXPECTED_MEMBERS)
        || optional_bits & 0x1F != 0
    {
        bail!(
            "group-capacity winner received invalid normal HOME PartyUpdate: flags={party_flags:#06X} index={party_index} type={party_type} leader={leader_guid:?} members={member_count} optional={optional_bits:#04X}"
        );
    }
    if has_lfg_info || !has_loot_settings || !has_difficulty_settings {
        bail!(
            "group-capacity PartyUpdate optional values differed from a normal non-LFG group: lfg={has_lfg_info} loot={has_loot_settings} difficulty={has_difficulty_settings}"
        );
    }

    let mut roster = Vec::with_capacity(EXPECTED_MEMBERS as usize);
    for member_index in 0..member_count {
        let info_bits = u16::from_be_bytes(
            cursor
                .take(2, "PartyPlayerInfo bit fields")?
                .try_into()
                .expect("exact party-player bit field"),
        );
        if info_bits & 1 != 0 {
            bail!("group-capacity PartyUpdate member {member_index} had nonzero bit padding");
        }
        let info_bits = info_bits >> 1;
        let name_len = usize::from((info_bits >> 9) & 0x3F);
        let voice_len_plus_one = usize::from((info_bits >> 3) & 0x3F);
        if name_len == 0 || voice_len_plus_one == 0 {
            bail!(
                "group-capacity PartyUpdate member {member_index} had invalid name/voice lengths"
            );
        }
        let guid = cursor.read_packed_guid("PartyPlayerInfo.GUID")?;
        let subgroup = cursor.read_u8("PartyPlayerInfo.Subgroup")?;
        let _flags = cursor.read_u8("PartyPlayerInfo.Flags")?;
        let _roles = cursor.read_u8("PartyPlayerInfo.RolesAssigned")?;
        let _class = cursor.read_u8("PartyPlayerInfo.Class")?;
        let _faction = cursor.read_u8("PartyPlayerInfo.FactionGroup")?;
        let _name = cursor.take(name_len, "PartyPlayerInfo.Name")?;
        let _voice = cursor.take(voice_len_plus_one - 1, "PartyPlayerInfo.VoiceStateID")?;
        if subgroup != 0 {
            bail!("group-capacity normal party member {member_index} had subgroup {subgroup}");
        }
        roster.push(guid);
    }

    let receiver = create_player_guid_raw(options.character_guid, realm_id());
    let wire_roster = roster.clone();
    let mut expected: Vec<_> = options
        .initial_member_guids
        .iter()
        .copied()
        .chain(std::iter::once(options.character_guid))
        .map(|guid| create_player_guid_raw(guid, realm_id()))
        .collect();
    expected.sort_unstable();
    roster.sort_unstable();

    let evidence = if member_count == EXPECTED_MEMBERS {
        let receiver_index = usize::try_from(my_index)
            .ok()
            .filter(|index| *index < expected.len())
            .ok_or_else(|| anyhow!("group-capacity PartyUpdate MyIndex {my_index} was invalid"))?;
        if wire_roster[receiver_index] != receiver {
            bail!("group-capacity PartyUpdate MyIndex did not identify the winning candidate");
        }
        if roster != expected {
            bail!(
                "group-capacity PartyUpdate roster {roster:?} did not match initial members plus winner {expected:?}"
            );
        }
        GroupCapacityPartyUpdateEvidence::CompleteRoster
    } else {
        // C++ Group::SendUpdateToPlayer serializes every MemberSlot, including
        // offline players. The current Rust send_party_update path filters its
        // PlayerList through PlayerRegistry, but keeps MyIndex from the complete
        // group. Keep the #110 runtime race scoped to atomic admission by pinning
        // that existing divergence exactly; the post-race DB assertion below is
        // still the authority for the complete five-member persisted roster.
        let mut expected_connected = vec![
            create_player_guid_raw(options.leader_guid, realm_id()),
            receiver,
        ];
        expected_connected.sort_unstable();
        let expected_complete_index = i32::try_from(options.initial_member_guids.len())
            .expect("normal group fixture length fits i32");
        if my_index != expected_complete_index || roster != expected_connected {
            bail!(
                "group-capacity PartyUpdate connected-only roster {roster:?} with MyIndex {my_index} did not match leader plus winner {expected_connected:?} and complete index {expected_complete_index}"
            );
        }
        GroupCapacityPartyUpdateEvidence::ConnectedOnlyRoster
    };

    let loot_method = cursor.read_u8("PartyLootSettings.Method")?;
    let loot_master = cursor.read_packed_guid("PartyLootSettings.LootMaster")?;
    let loot_threshold = cursor.read_u8("PartyLootSettings.Threshold")?;
    let dungeon_difficulty_id = cursor.read_u32("PartyDifficultySettings.DungeonDifficultyID")?;
    let raid_difficulty_id = cursor.read_u32("PartyDifficultySettings.RaidDifficultyID")?;
    let legacy_raid_difficulty_id =
        cursor.read_u32("PartyDifficultySettings.LegacyRaidDifficultyID")?;
    let expected_loot_master = if options.party_settings.loot_method == 2 {
        create_player_guid_raw(options.party_settings.master_looter_guid, realm_id())
    } else {
        (0, 0)
    };
    if loot_method != options.party_settings.loot_method
        || loot_master != expected_loot_master
        || loot_threshold != options.party_settings.loot_threshold
        || dungeon_difficulty_id != options.party_settings.dungeon_difficulty_id
        || raid_difficulty_id != options.party_settings.raid_difficulty_id
        || legacy_raid_difficulty_id != options.party_settings.legacy_raid_difficulty_id
    {
        bail!(
            "group-capacity PartyUpdate settings differed from the preloaded group: loot={loot_method}/{loot_master:?}/{loot_threshold} difficulty={dungeon_difficulty_id}/{raid_difficulty_id}/{legacy_raid_difficulty_id}; expected loot={}/{expected_loot_master:?}/{} difficulty={}/{}/{}",
            options.party_settings.loot_method,
            options.party_settings.loot_threshold,
            options.party_settings.dungeon_difficulty_id,
            options.party_settings.raid_difficulty_id,
            options.party_settings.legacy_raid_difficulty_id,
        );
    }
    if cursor.offset != payload.len() {
        bail!(
            "group-capacity PartyUpdate left {} trailing byte(s)",
            payload.len() - cursor.offset
        );
    }
    Ok(evidence)
}

async fn wait_for_group_capacity_realm_opcode(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &GroupCapacityRaceOptions,
    expected: u16,
    result: &mut BotRunResult,
) -> Result<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for group-capacity realm opcode 0x{expected:04X}");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "group-capacity realm packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Realm)?;
            if opcode == expected {
                return Ok(payload);
            }
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "group-capacity instance packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}

async fn wait_for_group_capacity_outcome(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &GroupCapacityRaceOptions,
    result: &mut BotRunResult,
) -> Result<GroupCapacityOutcome> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for group-capacity Added/GROUP_FULL outcome");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "group-capacity outcome realm packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Realm)?;
            match opcode {
                SMSG_PARTY_UPDATE => {
                    let evidence = validate_group_capacity_party_update(&payload, options)?;
                    if evidence == GroupCapacityPartyUpdateEvidence::ConnectedOnlyRoster {
                        warn!(
                            "[Bot {}] group-capacity PartyUpdate exposed the known Rust connected-only roster boundary; exact five-member persistence will be checked in the DB",
                            bot_index
                        );
                    }
                    return Ok(GroupCapacityOutcome::Added);
                }
                SMSG_PARTY_COMMAND_RESULT => {
                    validate_group_full_result(&payload)?;
                    return Ok(GroupCapacityOutcome::Full);
                }
                _ => {}
            }
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "group-capacity outcome instance packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}

pub(super) async fn run_group_capacity_phase(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &GroupCapacityRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let run = run_group_capacity_phase_inner(
        bot_index,
        stream,
        crypt,
        inflater,
        realm_connection,
        options,
        result,
    )
    .await;
    if let Err(error) = &run {
        options
            .sync
            .cancel(format!("participant {:?} failed: {error:#}", options.role));
    }
    run
}

async fn run_group_capacity_phase_inner(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &GroupCapacityRaceOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection.as_mut().ok_or_else(|| {
        anyhow!("group-capacity race requires separate realm and instance sockets")
    })?;
    wait_group_capacity_barrier(options, &options.sync.logged_in, "all three logins").await?;

    match options.role {
        GroupCapacityRaceRole::Leader => {
            for index in 0..2 {
                let (low, high) =
                    create_player_guid_raw(options.candidate_guids[index], realm_id());
                let payload = build_party_invite(&options.candidate_names[index], low, high)?;
                send_encrypted_packet(
                    &mut realm.stream,
                    &mut realm.crypt,
                    CMSG_PARTY_INVITE,
                    &payload,
                )
                .await?;
                let invite_result = wait_for_group_capacity_realm_opcode(
                    bot_index,
                    stream,
                    crypt,
                    inflater,
                    realm,
                    options,
                    SMSG_PARTY_COMMAND_RESULT,
                    result,
                )
                .await?;
                validate_group_invite_ok_result(&invite_result, &options.candidate_names[index])?;
            }
            wait_group_capacity_barrier(
                options,
                &options.sync.invitations_sent,
                "both invitations sent",
            )
            .await?;
            result.group_capacity_outcome = Some("leader-observer".to_string());
        }
        GroupCapacityRaceRole::CandidateA | GroupCapacityRaceRole::CandidateB => {
            wait_group_capacity_barrier(
                options,
                &options.sync.invitations_sent,
                "both invitations sent",
            )
            .await?;
            let invite = wait_for_group_capacity_realm_opcode(
                bot_index,
                stream,
                crypt,
                inflater,
                realm,
                options,
                SMSG_PARTY_INVITE,
                result,
            )
            .await?;
            validate_group_capacity_invite(&invite)?;
            wait_group_capacity_barrier(
                options,
                &options.sync.accepts_ready,
                "both candidates ready to accept",
            )
            .await?;
            send_encrypted_packet(
                &mut realm.stream,
                &mut realm.crypt,
                CMSG_PARTY_INVITE_RESPONSE,
                &[0x40],
            )
            .await?;
            let outcome = wait_for_group_capacity_outcome(
                bot_index, stream, crypt, inflater, realm, options, result,
            )
            .await?;
            result.group_capacity_outcome = Some(
                match outcome {
                    GroupCapacityOutcome::Added => "added",
                    GroupCapacityOutcome::Full => "full",
                }
                .to_string(),
            );
        }
    }

    wait_group_capacity_barrier(
        options,
        &options.sync.outcomes_observed,
        "both candidate outcomes observed",
    )
    .await?;
    logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        inflater,
        Some(realm),
        options.character_guid,
        result,
    )
    .await?;
    Ok(())
}

pub(super) async fn run_group_capacity_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: GroupCapacityRaceCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let fixture = {
        let bots = bots.clone();
        let cli = cli.clone();
        tokio::task::spawn_blocking(move || load_group_capacity_fixture(&bots, &cli))
            .await
            .map_err(|error| anyhow!("group-capacity fixture preflight worker failed: {error}"))??
    };
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&cli.leader_account) {
            0
        } else if bot.account.eq_ignore_ascii_case(&cli.candidate_a_account) {
            1
        } else {
            2
        }
    });
    if bots.len() != 3 {
        bail!("group-capacity race requires exactly three selected bot accounts");
    }

    let auth_serial = Arc::new(Mutex::new(()));
    let sync = Arc::new(GroupCapacityRaceSync::new());
    let mut handles = tokio::task::JoinSet::new();
    for (index, bot) in bots.iter().cloned().enumerate() {
        let options = GroupCapacityRaceOptions {
            role: match index {
                0 => GroupCapacityRaceRole::Leader,
                1 => GroupCapacityRaceRole::CandidateA,
                _ => GroupCapacityRaceRole::CandidateB,
            },
            character_guid: bot.character_guid,
            leader_guid: fixture.leader_guid,
            candidate_names: fixture.candidate_names.clone(),
            candidate_guids: fixture.candidate_guids,
            initial_member_guids: fixture.initial_member_guids,
            party_settings: fixture.party_settings,
            group_db_store_id: cli.group_db_store_id,
            timeout_secs: cli.timeout_secs,
            auth_serial: Arc::clone(&auth_serial),
            sync: Arc::clone(&sync),
        };
        let task_sync = Arc::clone(&sync);
        handles.spawn(async move {
            let run = run_bot(
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
                None,
                None,
                Some(options),
                None,
                None,
            )
            .await;
            if let Err(error) = &run {
                task_sync.cancel(format!("group-capacity transport/login failed: {error:#}"));
            }
            run
        });
    }

    let deadline = tokio::time::Instant::now()
        + Duration::from_secs(cli.timeout_secs.saturating_mul(4).saturating_add(180));
    let mut results = Vec::with_capacity(3);
    while !handles.is_empty() {
        let joined = tokio::select! {
            joined = handles.join_next() => joined,
            _ = shutdown.cancelled() => {
                sync.cancel("group-capacity race received SIGINT/SIGTERM");
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                bail!("group-capacity race received SIGINT/SIGTERM");
            }
            _ = tokio::time::sleep_until(deadline) => {
                sync.cancel("group-capacity race exceeded its end-to-end deadline");
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                bail!("group-capacity race exceeded its end-to-end deadline");
            }
        };
        match joined {
            Some(Ok(Ok(result))) => results.push(result),
            Some(Ok(Err(error))) => {
                sync.cancel(error.to_string());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                return Err(error);
            }
            Some(Err(error)) => {
                sync.cancel(error.to_string());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                bail!("group-capacity task join failed: {error}");
            }
            None => break,
        }
    }
    results.sort_by_key(|result| result.account_id);

    let participant_failures: Vec<_> = results
        .iter()
        .filter_map(|result| {
            result
                .group_capacity_failure
                .as_deref()
                .map(|failure| format!("{}: {failure}", result.account))
        })
        .collect();
    if !participant_failures.is_empty() {
        bail!(
            "group-capacity participant failure(s): {}",
            participant_failures.join("; ")
        );
    }

    let candidate_outcomes: Vec<_> = results
        .iter()
        .filter_map(|result| result.group_capacity_outcome.as_deref())
        .filter(|outcome| *outcome != "leader-observer")
        .collect();
    if candidate_outcomes
        .iter()
        .filter(|outcome| **outcome == "added")
        .count()
        != 1
        || candidate_outcomes
            .iter()
            .filter(|outcome| **outcome == "full")
            .count()
            != 1
    {
        bail!(
            "group-capacity wire outcomes were {candidate_outcomes:?}; expected one added and one full"
        );
    }
    let wire_winner_guid = results
        .iter()
        .find(|result| result.group_capacity_outcome.as_deref() == Some("added"))
        .map(|result| result.character_guid)
        .ok_or_else(|| anyhow!("group-capacity race did not identify the wire winner"))?;
    let persistence = {
        let cli = cli.clone();
        let fixture = fixture.clone();
        tokio::task::spawn_blocking(move || verify_group_capacity_fixture(&cli, &fixture))
            .await
            .map_err(|error| anyhow!("group-capacity DB verification worker failed: {error}"))??
    };
    validate_group_capacity_winner_consistency(
        wire_winner_guid,
        persistence.winning_candidate_guid,
    )?;
    for result in &mut results {
        result.group_capacity_final_member_count = Some(persistence.final_member_count);
        result.group_capacity_race_smoke_passed = Some(true);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    const ITEM_TEST_REALM: u32 = 7;
    const ITEM_TEST_ENTRY: u32 = 18_610;
    const ITEM_TEST_CHARACTERS: [u64; 2] = [15, 16];

    #[test]
    fn loot_fixture_health_guard_produces_one_health_like_cpp() {
        assert_eq!(
            generated_fixture_health_like_cpp(115, GUARDED_FIXTURE_HEALTH_MODIFIER),
            1
        );
        assert!(validate_guarded_fixture_health(GUARDED_FIXTURE_HEALTH_MODIFIER, 1).is_ok());
    }

    #[tokio::test]
    async fn cancellation_token_cannot_lose_a_preexisting_cancel() {
        let sync = LootRaceSync::new();
        sync.cancel("deterministic peer failure");
        tokio::time::timeout(Duration::from_millis(50), sync.cancelled())
            .await
            .expect("CancellationToken retains cancellation before waiter registration");
        assert!(sync
            .cancellation_error()
            .expect_err("cancelled sync must fail")
            .to_string()
            .contains("deterministic peer failure"));
    }

    #[test]
    fn respawn_cleanup_scope_is_exact_and_fail_closed() {
        assert_eq!(validate_respawn_cleanup_scope(0, &[], &[]).unwrap(), None);
        assert_eq!(
            validate_respawn_cleanup_scope(0, &[], &[(1_700_000_000, 0, 0)]).unwrap(),
            Some((1_700_000_000, 0, 0))
        );
        assert!(validate_respawn_cleanup_scope(
            0,
            &[],
            &[(1_700_000_000, 0, 0), (1_700_000_001, 0, 1)]
        )
        .is_err());
        assert!(validate_respawn_cleanup_scope(0, &[], &[(1_700_000_000, 1, 0)]).is_err());
        assert!(validate_respawn_cleanup_scope(0, &[], &[(1_700_000_000, 0, 1)]).is_err());
        assert!(validate_respawn_cleanup_scope(0, &[], &[(0, 0, 0)]).is_err());
        assert!(validate_respawn_cleanup_scope(
            0,
            &[(1_600_000_000, 0, 0)],
            &[(1_700_000_000, 0, 0)]
        )
        .is_err());
    }

    #[test]
    fn cleanup_marker_publish_is_0600_and_both_present_recovery_is_idempotent() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "wow-test-bot-journal-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("fixture.journal");
        let journal = FixtureJournal { path: path.clone() };
        let payload = b"durable fixture snapshot\n";
        write_test_journal(&path, payload);

        journal.complete().unwrap();
        let marker = cleanup_marker_path(&path);
        assert!(!path.exists());
        assert_eq!(
            fs::metadata(&marker).unwrap().permissions().mode() & 0o777,
            0o600
        );
        validate_cleanup_marker(&marker, None).unwrap();

        // Model SIGKILL after atomic marker rename but before journal unlink.
        // Recovery re-verifies DB state, then accepts only the same digest.
        write_test_journal(&path, payload);
        journal.complete().unwrap();
        assert!(!path.exists());

        // A different pending snapshot must not be hidden by a stale marker.
        write_test_journal(&path, b"different snapshot\n");
        assert!(journal.complete().is_err());
        assert!(path.exists());

        fs::remove_file(&path).unwrap();
        fs::remove_file(&marker).unwrap();
        fs::remove_dir(&directory).unwrap();
    }

    fn write_test_journal(path: &Path, payload: &[u8]) {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap();
        file.write_all(payload).unwrap();
        file.sync_all().unwrap();
    }

    #[test]
    fn loot_fixture_health_guard_rejects_original_or_stale_runtime_data() {
        let original_health = generated_fixture_health_like_cpp(115, 1.5);
        assert_eq!(original_health, 173);
        assert!(validate_guarded_fixture_health(1.5, original_health).is_err());
        assert!(
            validate_guarded_fixture_health(GUARDED_FIXTURE_HEALTH_MODIFIER, 2).is_err(),
            "a non-default classification multiplier or stale runtime must fail closed"
        );
    }

    #[test]
    fn single_item_fixture_requires_exact_keyring_destination_empty() {
        let required = Some((15, LOOT_ITEM_CAPTURE_KEYRING_SLOT));
        validate_required_empty_top_level_slot(15, &[35, 50], required).unwrap();
        assert!(validate_required_empty_top_level_slot(
            15,
            &[LOOT_ITEM_CAPTURE_KEYRING_SLOT],
            required
        )
        .is_err());
        validate_required_empty_top_level_slot(16, &[LOOT_ITEM_CAPTURE_KEYRING_SLOT], required)
            .unwrap();
    }

    fn valid_atomic_item_push() -> ItemPush {
        let (player_low, player_high) =
            create_player_guid_raw(ITEM_TEST_CHARACTERS[0], ITEM_TEST_REALM);
        ItemPush {
            player_low,
            player_high,
            slot: INVENTORY_SLOT_BAG_0,
            slot_in_bag: i32::from(INVENTORY_SLOT_ITEM_START),
            quest_log_item_id: 0,
            quantity: 1,
            quantity_in_inventory: 1,
            dungeon_encounter_id: 0,
            item_guid_low: 0x1234,
            item_guid_high: (HIGH_GUID_ITEM << 58) | (u64::from(ITEM_TEST_REALM) << 42),
            pushed: false,
            created: false,
            display_text: 1,
            is_bonus_roll: false,
            is_encounter_loot: false,
            item_entry: ITEM_TEST_ENTRY,
        }
    }

    fn valid_atomic_item_outcome(
        group_broadcast: bool,
    ) -> ([WireEvidence; 2], LootRemovedEvidence) {
        let push = valid_atomic_item_push();
        let removal = LootRemovedEvidence {
            owner_low: 268,
            owner_high: 0x2000_0442_4015_44C0,
            loot_low: 41,
            loot_high: (HIGH_GUID_LOOT_OBJECT << 58) | (u64::from(ITEM_TEST_REALM) << 42),
            loot_list_id: 3,
        };
        let loot_gone = InventoryFailure {
            result: 50,
            item_0_low: 0,
            item_0_high: 0,
            item_1_low: 0,
            item_1_high: 0,
            container_b_slot: 0,
        };
        (
            [
                WireEvidence {
                    item_pushes: vec![push],
                    loot_removed: vec![removal],
                    ..Default::default()
                },
                WireEvidence {
                    item_pushes: group_broadcast.then_some(push).into_iter().collect(),
                    loot_removed: vec![removal],
                    inventory_failures: vec![loot_gone],
                    ..Default::default()
                },
            ],
            removal,
        )
    }

    fn runtime_discovery_options(override_counter: u64) -> LootRaceOptions {
        LootRaceOptions {
            phase: LootRacePhase::CaptureItem,
            participant: 0,
            character_guid: 15,
            peer_name: "Peer".to_string(),
            peer_character_guid: 16,
            killer_character_guid: 15,
            target: LootRaceTarget {
                kind: LootRaceTargetKind::Creature,
                entry: 21_779,
                spawn_guid: 1_117,
                runtime_counter_override: override_counter,
                map_id: 530,
                x: -2_695.57,
                y: 2_633.82,
                z: 74.6837,
                item_entry: 30_712,
            },
            timeout_secs: 30,
            sync: Arc::new(LootRaceSync::new()),
        }
    }

    fn gameobject_discovery_options(override_counter: u64) -> LootRaceOptions {
        LootRaceOptions {
            phase: LootRacePhase::Race,
            participant: 0,
            character_guid: 15,
            peer_name: "Peer".to_string(),
            peer_character_guid: 16,
            killer_character_guid: 15,
            target: LootRaceTarget {
                kind: LootRaceTargetKind::GameObject,
                entry: DEFAULT_CREATURE_ENTRY,
                spawn_guid: DEFAULT_CREATURE_SPAWN_GUID,
                runtime_counter_override: override_counter,
                map_id: RACE_GAMEOBJECT_MAP_ID,
                x: RACE_GAMEOBJECT_X,
                y: RACE_GAMEOBJECT_Y,
                z: RACE_GAMEOBJECT_Z,
                item_entry: DEFAULT_ITEM_ENTRY,
            },
            timeout_secs: 30,
            sync: Arc::new(LootRaceSync::new()),
        }
    }

    fn gameobject_runtime_high(map_id: u16, entry: u32) -> u64 {
        (HIGH_GUID_GAMEOBJECT << 58) | (u64::from(map_id) << 29) | (u64::from(entry) << 6)
    }

    fn update_object_with_guid(low: u64, high: u64) -> Vec<u8> {
        let mut payload = vec![1];
        payload.extend_from_slice(&build_packed_guid(low, high));
        payload
    }

    fn append_party_player_info_for_test(payload: &mut Vec<u8>, guid: (u64, u64), name: &str) {
        let name_len = u16::try_from(name.len()).unwrap();
        let voice_len_plus_one = 1u16;
        // C++ PartyPackets.cpp writes 15 MSB-first bits and the following
        // packed-GUID write pads the last low bit to the next byte.
        let info_bits = ((name_len << 9) | (voice_len_plus_one << 3) | 0x04) << 1;
        payload.extend_from_slice(&info_bits.to_be_bytes());
        payload.extend_from_slice(&build_packed_guid(guid.0, guid.1));
        payload.extend_from_slice(&[0, 0, 0, 1, 0]);
        payload.extend_from_slice(name.as_bytes());
    }

    fn party_update_for_test(
        party_flags: u16,
        party_index: u8,
        party_type: u8,
        my_index: i32,
        party_guid: (u64, u64),
        leader_guid: (u64, u64),
        roster: [(u64, u64); 2],
        loot_method: u8,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&party_flags.to_le_bytes());
        payload.push(party_index);
        payload.push(party_type);
        payload.extend_from_slice(&my_index.to_le_bytes());
        payload.extend_from_slice(&build_packed_guid(party_guid.0, party_guid.1));
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&build_packed_guid(leader_guid.0, leader_guid.1));
        payload.push(0);
        payload.extend_from_slice(&2u32.to_le_bytes());
        payload.push(0x60); // no LFG, with loot and difficulty settings
        append_party_player_info_for_test(&mut payload, roster[0], "Leader");
        append_party_player_info_for_test(&mut payload, roster[1], "Peer");
        payload.push(loot_method); // PartyLootSettings.Method
        payload.extend_from_slice(&build_packed_guid(0, 0));
        payload.push(2); // PartyLootSettings.Threshold
        payload.extend_from_slice(&[0; 12]); // PartyDifficultySettings
        payload
    }

    fn group_capacity_party_update_for_test(
        receiver_index: i32,
        leader_guid: (u64, u64),
        roster: &[(u64, u64)],
    ) -> Vec<u8> {
        group_capacity_party_update_with_optional_bits_for_test(
            receiver_index,
            leader_guid,
            roster,
            0x60,
        )
    }

    fn group_capacity_party_update_with_optional_bits_for_test(
        receiver_index: i32,
        leader_guid: (u64, u64),
        roster: &[(u64, u64)],
        optional_bits: u8,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0u16.to_le_bytes());
        payload.push(0);
        payload.push(1);
        payload.extend_from_slice(&receiver_index.to_le_bytes());
        payload.extend_from_slice(&build_packed_guid(77, 88));
        payload.extend_from_slice(&1u32.to_le_bytes());
        payload.extend_from_slice(&build_packed_guid(leader_guid.0, leader_guid.1));
        payload.push(0);
        payload.extend_from_slice(&(roster.len() as u32).to_le_bytes());
        payload.push(optional_bits);
        for (index, guid) in roster.iter().copied().enumerate() {
            append_party_player_info_for_test(&mut payload, guid, &format!("Member{index}"));
        }
        let settings = group_capacity_party_settings_for_test();
        payload.push(settings.loot_method);
        payload.extend_from_slice(&build_packed_guid(0, 0));
        payload.push(settings.loot_threshold);
        payload.extend_from_slice(&settings.dungeon_difficulty_id.to_le_bytes());
        payload.extend_from_slice(&settings.raid_difficulty_id.to_le_bytes());
        payload.extend_from_slice(&settings.legacy_raid_difficulty_id.to_le_bytes());
        payload
    }

    fn group_capacity_party_settings_for_test() -> GroupCapacityPartySettings {
        GroupCapacityPartySettings {
            loot_method: 0,
            loot_threshold: 2,
            master_looter_guid: 0,
            dungeon_difficulty_id: 1,
            raid_difficulty_id: 14,
            legacy_raid_difficulty_id: 3,
        }
    }

    fn group_capacity_fixture_for_test() -> GroupCapacityFixture {
        GroupCapacityFixture {
            leader_guid: 14,
            candidate_names: ["CandidateA".into(), "CandidateB".into()],
            candidate_guids: [15, 16],
            initial_member_guids: [13, 14, 17, 18],
            party_settings: group_capacity_party_settings_for_test(),
        }
    }

    fn group_capacity_options_for_test(candidate_guid: u64) -> GroupCapacityRaceOptions {
        GroupCapacityRaceOptions {
            role: GroupCapacityRaceRole::CandidateA,
            character_guid: candidate_guid,
            leader_guid: 14,
            candidate_names: ["CandidateA".into(), "CandidateB".into()],
            candidate_guids: [15, 16],
            initial_member_guids: [13, 14, 17, 18],
            party_settings: group_capacity_party_settings_for_test(),
            group_db_store_id: 99,
            timeout_secs: 1,
            auth_serial: Arc::new(Mutex::new(())),
            sync: Arc::new(GroupCapacityRaceSync::new()),
        }
    }

    #[test]
    fn sql_spawn_uniqueness_is_required_for_entry_map_runtime_auto_discovery() {
        validate_unique_sql_spawn(&[1_117], 1_117, 21_779, 530).unwrap();

        let missing = validate_unique_sql_spawn(&[], 1_117, 21_779, 530)
            .expect_err("a missing SQL spawn must fail closed");
        assert!(missing.to_string().contains("exactly one"));

        let ambiguous = validate_unique_sql_spawn(&[1_117, 2_268], 1_117, 21_779, 530)
            .expect_err("same-entry map ambiguity must fail closed");
        assert!(ambiguous.to_string().contains("2 SQL spawns"));

        let mismatch = validate_unique_sql_spawn(&[1_117], 2_268, 21_779, 530)
            .expect_err("the unique row must equal the configured spawn");
        assert!(mismatch.to_string().contains("not configured spawn"));
    }

    #[test]
    fn runtime_auto_discovery_preserves_cpp_full_guid_including_realm() {
        const CPP_DOCTOR_COUNTER: u64 = 268;
        const CPP_DOCTOR_HIGH: u64 = 0x2000_0442_4015_44C0;
        let options = runtime_discovery_options(0);
        let payload = update_object_with_guid(CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH);

        assert_eq!(
            target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &payload).unwrap(),
            Some(CPP_DOCTOR_COUNTER)
        );
        assert_eq!(
            options.resolved_runtime_guid().unwrap(),
            (CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH)
        );
        assert_eq!(
            options.resolved_packed_guid().unwrap(),
            build_packed_guid(CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH)
        );
        assert_eq!((CPP_DOCTOR_HIGH >> 42) & GUID_REALM_MASK, 1);

        let reconstructed_without_realm = create_creature_guid_raw(530, 21_779, 268);
        assert_ne!(
            reconstructed_without_realm,
            (CPP_DOCTOR_COUNTER, CPP_DOCTOR_HIGH),
            "the SQL spawn/counter helper must not replace the full discovered C++ GUID"
        );
    }

    #[test]
    fn runtime_override_and_two_bot_convergence_fail_closed() {
        const CPP_DOCTOR_HIGH: u64 = 0x2000_0442_4015_44C0;
        let strict = runtime_discovery_options(1_117);
        let mismatch = target_seen_in_update(
            &strict,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(268, CPP_DOCTOR_HIGH),
        )
        .expect_err("a stale SQL-spawn-as-counter override must be rejected");
        assert!(mismatch
            .to_string()
            .contains("did not match discovered counter 268"));

        let first = runtime_discovery_options(0);
        let mut second = first.clone();
        second.participant = 1;
        assert_eq!(
            target_seen_in_update(
                &first,
                SMSG_UPDATE_OBJECT,
                &update_object_with_guid(268, CPP_DOCTOR_HIGH),
            )
            .unwrap(),
            Some(268)
        );
        assert_eq!(
            target_seen_in_update(
                &second,
                SMSG_UPDATE_OBJECT,
                &update_object_with_guid(268, CPP_DOCTOR_HIGH),
            )
            .unwrap(),
            Some(268)
        );

        let different = target_seen_in_update(
            &second,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(269, CPP_DOCTOR_HIGH),
        )
        .expect_err("two bots must not bind different runtime counters");
        assert!(different.to_string().contains("different live ObjectGuids"));
    }

    #[test]
    fn gameobject_discovery_keeps_sql_spawn_and_runtime_counter_distinct_like_cpp() {
        const LIVE_COUNTER: u64 = 40;
        let options = gameobject_discovery_options(0);
        let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
        let exact = update_object_with_guid(LIVE_COUNTER, high);
        assert_eq!(
            target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &exact).unwrap(),
            Some(LIVE_COUNTER)
        );
        assert_eq!(
            options.resolved_runtime_guid().unwrap(),
            (LIVE_COUNTER, high)
        );
        assert_ne!(LIVE_COUNTER, DEFAULT_CREATURE_SPAWN_GUID);
    }

    #[test]
    fn gameobject_discovery_requires_exact_high_type_entry_and_map() {
        const LIVE_COUNTER: u64 = 40;
        let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
        let creature_high = (HIGH_GUID_CREATURE << 58)
            | (u64::from(RACE_GAMEOBJECT_MAP_ID) << 29)
            | (u64::from(DEFAULT_CREATURE_ENTRY) << 6);
        let wrong_entry_high =
            gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY + 1);
        let wrong_map_high =
            gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID + 1, DEFAULT_CREATURE_ENTRY);

        for wrong_high in [creature_high, wrong_entry_high, wrong_map_high] {
            let options = gameobject_discovery_options(0);
            assert_eq!(
                target_seen_in_update(
                    &options,
                    SMSG_UPDATE_OBJECT,
                    &update_object_with_guid(LIVE_COUNTER, wrong_high),
                )
                .unwrap(),
                None
            );
        }

        let wrong_opcode = gameobject_discovery_options(0);
        assert_eq!(
            target_seen_in_update(
                &wrong_opcode,
                SMSG_LOOT_RESPONSE,
                &update_object_with_guid(LIVE_COUNTER, high),
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn gameobject_runtime_override_checks_the_live_counter_not_the_sql_spawn() {
        const LIVE_COUNTER: u64 = 40;
        let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
        let exact = gameobject_discovery_options(LIVE_COUNTER);
        assert_eq!(
            target_seen_in_update(
                &exact,
                SMSG_UPDATE_OBJECT,
                &update_object_with_guid(LIVE_COUNTER, high),
            )
            .unwrap(),
            Some(LIVE_COUNTER)
        );

        let stale = gameobject_discovery_options(LIVE_COUNTER + 1);
        let error = target_seen_in_update(
            &stale,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(LIVE_COUNTER, high),
        )
        .expect_err("a mismatching live GameObject counter override must fail closed");
        assert!(error
            .to_string()
            .contains("did not match discovered counter 40"));
    }

    #[test]
    fn gameobject_discovery_deduplicates_one_guid_and_rejects_packet_ambiguity() {
        const LIVE_COUNTER: u64 = 40;
        let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);

        let mut duplicate = update_object_with_guid(LIVE_COUNTER, high);
        duplicate.extend_from_slice(&update_object_with_guid(LIVE_COUNTER, high));
        let options = gameobject_discovery_options(0);
        assert_eq!(
            target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &duplicate).unwrap(),
            Some(LIVE_COUNTER)
        );

        let mut ambiguous = update_object_with_guid(LIVE_COUNTER, high);
        ambiguous.extend_from_slice(&update_object_with_guid(LIVE_COUNTER + 1, high));
        let options = gameobject_discovery_options(0);
        let error = target_seen_in_update(&options, SMSG_UPDATE_OBJECT, &ambiguous)
            .expect_err("two distinct matching GameObjects in one update must fail closed");
        assert!(error.to_string().contains("2 distinct live ObjectGuid"));
    }

    #[test]
    fn gameobject_discovery_requires_two_bots_to_converge_on_one_full_guid() {
        const LIVE_COUNTER: u64 = 40;
        let high = gameobject_runtime_high(RACE_GAMEOBJECT_MAP_ID, DEFAULT_CREATURE_ENTRY);
        let first = gameobject_discovery_options(0);
        let mut second = first.clone();
        second.participant = 1;

        assert_eq!(
            target_seen_in_update(
                &first,
                SMSG_UPDATE_OBJECT,
                &update_object_with_guid(LIVE_COUNTER, high),
            )
            .unwrap(),
            Some(LIVE_COUNTER)
        );
        assert_eq!(
            target_seen_in_update(
                &second,
                SMSG_UPDATE_OBJECT,
                &update_object_with_guid(LIVE_COUNTER, high),
            )
            .unwrap(),
            Some(LIVE_COUNTER)
        );

        let error = target_seen_in_update(
            &second,
            SMSG_UPDATE_OBJECT,
            &update_object_with_guid(LIVE_COUNTER + 1, high),
        )
        .expect_err("two bots must not bind different live GameObject GUIDs");
        assert!(error.to_string().contains("different live ObjectGuids"));
    }

    #[tokio::test]
    async fn peer_failure_cancels_a_phase_barrier_without_waiting_for_timeout() {
        let options = gameobject_discovery_options(0);
        let sync = options.sync.clone();
        let waiter = tokio::spawn(async move {
            wait_phase(&options, &options.sync.logged_in, "test peer barrier").await
        });
        tokio::task::yield_now().await;
        sync.cancel("peer failed before reaching the barrier");

        let error = tokio::time::timeout(Duration::from_millis(250), waiter)
            .await
            .expect("cancellation must wake the peer promptly")
            .expect("barrier waiter task must not panic")
            .expect_err("the peer barrier must fail after cancellation");
        assert!(error
            .to_string()
            .contains("peer failed before reaching the barrier"));
    }

    #[test]
    fn logout_complete_requires_cpp_empty_body_on_both_routes() {
        assert_eq!(
            logout_completion_route(SMSG_LOGOUT_COMPLETE, &[], LogoutCompletionRoute::Realm)
                .unwrap(),
            Some(LogoutCompletionRoute::Realm)
        );
        assert_eq!(
            logout_completion_route(SMSG_LOGOUT_COMPLETE, &[], LogoutCompletionRoute::Instance)
                .unwrap(),
            Some(LogoutCompletionRoute::Instance)
        );
        assert_eq!(
            logout_completion_route(SMSG_LOOT_RESPONSE, &[1], LogoutCompletionRoute::Realm)
                .unwrap(),
            None
        );
        assert!(
            logout_completion_route(SMSG_LOGOUT_COMPLETE, &[0], LogoutCompletionRoute::Realm)
                .is_err()
        );
    }

    #[test]
    fn cpp_realm_only_party_opcodes_fail_fast_on_instance() {
        for opcode in [
            SMSG_PARTY_INVITE,
            SMSG_PARTY_UPDATE,
            SMSG_PARTY_COMMAND_RESULT,
            SMSG_PARTY_MEMBER_FULL_STATE,
        ] {
            let error = validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)
                .unwrap_err();
            assert!(error.to_string().contains("CONNECTION_TYPE_REALM"));
        }
    }

    #[test]
    fn party_route_guard_accepts_cpp_realm_and_unrelated_instance_packets() {
        validate_party_packet_route_like_cpp(SMSG_PARTY_UPDATE, PartyPacketRoute::Realm).unwrap();
        validate_party_packet_route_like_cpp(SMSG_TIME_SYNC_REQUEST, PartyPacketRoute::Instance)
            .unwrap();
    }

    #[test]
    fn party_update_proves_exact_normal_home_two_player_roster() {
        let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
        let peer = create_player_guid_raw(16, ITEM_TEST_REALM);
        let payload = party_update_for_test(
            0,
            0,
            1,
            0,
            (77, 88),
            leader,
            [leader, peer],
            PERSONAL_LOOT_METHOD_LIKE_CPP,
        );

        validate_party_update_like_cpp(&payload, leader, peer, leader).unwrap();
    }

    #[test]
    fn group_capacity_winner_requires_exact_five_member_roster() {
        let options = group_capacity_options_for_test(15);
        let roster: Vec<_> = [13, 14, 17, 18, 15]
            .into_iter()
            .map(|guid| create_player_guid_raw(guid, realm_id()))
            .collect();
        let payload = group_capacity_party_update_for_test(
            4,
            create_player_guid_raw(14, realm_id()),
            &roster,
        );
        assert_eq!(
            validate_group_capacity_party_update(&payload, &options).unwrap(),
            GroupCapacityPartyUpdateEvidence::CompleteRoster
        );

        let six_member_roster: Vec<_> = [13, 14, 17, 18, 15, 16]
            .into_iter()
            .map(|guid| create_player_guid_raw(guid, realm_id()))
            .collect();
        let six_member_payload = group_capacity_party_update_for_test(
            4,
            create_player_guid_raw(14, realm_id()),
            &six_member_roster,
        );
        assert!(
            validate_group_capacity_party_update(&six_member_payload, &options)
                .unwrap_err()
                .to_string()
                .contains("members=6")
        );
    }

    #[test]
    fn group_capacity_runtime_boundary_accepts_only_exact_connected_pair() {
        let options = group_capacity_options_for_test(15);
        let leader = create_player_guid_raw(14, realm_id());
        let candidate = create_player_guid_raw(15, realm_id());
        let payload = group_capacity_party_update_for_test(4, leader, &[leader, candidate]);
        assert_eq!(
            validate_group_capacity_party_update(&payload, &options).unwrap(),
            GroupCapacityPartyUpdateEvidence::ConnectedOnlyRoster
        );

        let wrong_peer = create_player_guid_raw(16, realm_id());
        let wrong = group_capacity_party_update_for_test(4, leader, &[leader, wrong_peer]);
        assert!(validate_group_capacity_party_update(&wrong, &options).is_err());
    }

    #[test]
    fn group_capacity_fixture_rejects_online_initial_member() {
        validate_group_capacity_initial_member_offline(17, 0).unwrap();
        let error = validate_group_capacity_initial_member_offline(17, 1)
            .expect_err("an online filler would change the connected-only PartyUpdate roster");
        assert!(error.to_string().contains("initial member 17"));
        assert!(error.to_string().contains("offline"));
    }

    #[test]
    fn group_capacity_persistence_identifies_and_matches_wire_winner() {
        let fixture = group_capacity_fixture_for_test();
        let evidence =
            validate_group_capacity_persisted_members(&fixture, &[13, 14, 15, 17, 18]).unwrap();
        assert_eq!(
            evidence,
            GroupCapacityPersistenceEvidence {
                final_member_count: 5,
                winning_candidate_guid: 15,
            }
        );
        validate_group_capacity_winner_consistency(15, evidence.winning_candidate_guid).unwrap();
        let error = validate_group_capacity_winner_consistency(16, evidence.winning_candidate_guid)
            .expect_err("wire and CharacterDB winners must be the same candidate");
        assert!(error.to_string().contains("winner mismatch"));
        assert!(error.to_string().contains("wire added GUID 16"));
        assert!(error.to_string().contains("persisted GUID 15"));
    }

    #[test]
    fn group_capacity_party_update_requires_cpp_optional_tail_exactly() {
        let options = group_capacity_options_for_test(15);
        let leader = create_player_guid_raw(14, realm_id());
        let candidate = create_player_guid_raw(15, realm_id());

        let no_option_bits = group_capacity_party_update_with_optional_bits_for_test(
            4,
            leader,
            &[leader, candidate],
            0,
        );
        let error = validate_group_capacity_party_update(&no_option_bits, &options)
            .expect_err("normal party update must advertise loot and difficulty settings");
        assert!(error.to_string().contains("loot=false"));
        assert!(error.to_string().contains("difficulty=false"));

        let valid = group_capacity_party_update_for_test(4, leader, &[leader, candidate]);
        let empty_guid_len = build_packed_guid(0, 0).len();
        let optional_tail_len = 1 + empty_guid_len + 1 + 12;
        let tail_start = valid.len() - optional_tail_len;

        let mut wrong_method = valid.clone();
        wrong_method[tail_start] = 3;
        assert!(
            validate_group_capacity_party_update(&wrong_method, &options)
                .unwrap_err()
                .to_string()
                .contains("settings differed")
        );

        let mut wrong_threshold = valid.clone();
        wrong_threshold[tail_start + 1 + empty_guid_len] = 4;
        assert!(
            validate_group_capacity_party_update(&wrong_threshold, &options)
                .unwrap_err()
                .to_string()
                .contains("settings differed")
        );

        let difficulty_start = tail_start + 1 + empty_guid_len + 1;
        let mut swapped_difficulties = valid.clone();
        swapped_difficulties[difficulty_start..difficulty_start + 4]
            .copy_from_slice(&14u32.to_le_bytes());
        swapped_difficulties[difficulty_start + 4..difficulty_start + 8]
            .copy_from_slice(&1u32.to_le_bytes());
        assert!(
            validate_group_capacity_party_update(&swapped_difficulties, &options)
                .unwrap_err()
                .to_string()
                .contains("settings differed")
        );

        let mut missing_tail = valid.clone();
        missing_tail.truncate(tail_start);
        assert!(
            validate_group_capacity_party_update(&missing_tail, &options)
                .unwrap_err()
                .to_string()
                .contains("PartyLootSettings.Method")
        );

        let mut trailing = valid;
        trailing.push(0xFF);
        assert!(validate_group_capacity_party_update(&trailing, &options)
            .unwrap_err()
            .to_string()
            .contains("trailing byte"));
    }

    #[test]
    fn group_capacity_full_result_requires_invite_group_full() {
        let mut payload = pack_msb_fields(&[(0, 9), (0, 4), (4, 6)]);
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&build_packed_guid(0, 0));
        validate_group_full_result(&payload).unwrap();

        let wrong = pack_msb_fields(&[(0, 9), (0, 4), (5, 6)]);
        assert!(validate_group_full_result(&wrong).is_err());
    }

    #[test]
    fn group_capacity_invite_result_requires_ok_for_the_exact_candidate() {
        let mut ok = pack_msb_fields(&[(7, 9), (0, 4), (0, 6)]);
        ok.extend_from_slice(&0u32.to_le_bytes());
        ok.extend_from_slice(&build_packed_guid(0, 0));
        ok.extend_from_slice(b"Lfgheal");
        validate_group_invite_ok_result(&ok, "Lfgheal").unwrap();

        let mut wrong_name = pack_msb_fields(&[(7, 9), (0, 4), (0, 6)]);
        wrong_name.extend_from_slice(&0u32.to_le_bytes());
        wrong_name.extend_from_slice(&build_packed_guid(0, 0));
        wrong_name.extend_from_slice(b"Lfgmage");
        assert!(validate_group_invite_ok_result(&wrong_name, "Lfgheal").is_err());

        let mut wrong_faction = pack_msb_fields(&[(7, 9), (0, 4), (8, 6)]);
        wrong_faction.extend_from_slice(&0u32.to_le_bytes());
        wrong_faction.extend_from_slice(&build_packed_guid(0, 0));
        wrong_faction.extend_from_slice(b"Lfgheal");
        let error = validate_group_invite_ok_result(&wrong_faction, "Lfgheal")
            .expect_err("WRONG_FACTION must fail before the accept barrier");
        assert!(error.to_string().contains("result=8"));
    }

    #[test]
    fn party_update_rejects_non_personal_loot_for_shared_chest_race() {
        let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
        let peer = create_player_guid_raw(16, ITEM_TEST_REALM);
        let payload = party_update_for_test(0, 0, 1, 0, (77, 88), leader, [leader, peer], 0);

        let error = validate_party_update_like_cpp(&payload, leader, peer, leader)
            .expect_err("shared chest race must pin C++ PERSONAL_LOOT");
        assert!(error.to_string().contains("PERSONAL_LOOT"));
    }

    #[test]
    fn party_update_rejects_non_home_empty_or_wrong_roster_states() {
        let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
        let peer = create_player_guid_raw(16, ITEM_TEST_REALM);

        let non_home = party_update_for_test(
            0,
            1,
            1,
            0,
            (77, 88),
            leader,
            [leader, peer],
            PERSONAL_LOOT_METHOD_LIKE_CPP,
        );
        assert!(
            validate_party_update_like_cpp(&non_home, leader, peer, leader)
                .unwrap_err()
                .to_string()
                .contains("normal HOME")
        );

        let empty_group = party_update_for_test(
            0,
            0,
            1,
            0,
            (0, 0),
            leader,
            [leader, peer],
            PERSONAL_LOOT_METHOD_LIKE_CPP,
        );
        assert!(
            validate_party_update_like_cpp(&empty_group, leader, peer, leader)
                .unwrap_err()
                .to_string()
                .contains("empty PartyGUID")
        );

        let duplicate = party_update_for_test(
            0,
            0,
            1,
            0,
            (77, 88),
            leader,
            [leader, leader],
            PERSONAL_LOOT_METHOD_LIKE_CPP,
        );
        assert!(
            validate_party_update_like_cpp(&duplicate, leader, peer, leader)
                .unwrap_err()
                .to_string()
                .contains("did not contain exactly")
        );

        let wrong_receiver_index = party_update_for_test(
            0,
            0,
            1,
            1,
            (77, 88),
            leader,
            [leader, peer],
            PERSONAL_LOOT_METHOD_LIKE_CPP,
        );
        assert!(
            validate_party_update_like_cpp(&wrong_receiver_index, leader, peer, leader)
                .unwrap_err()
                .to_string()
                .contains("expected receiver")
        );
    }

    #[test]
    fn party_update_rejects_truncated_or_trailing_payloads() {
        let leader = create_player_guid_raw(15, ITEM_TEST_REALM);
        let peer = create_player_guid_raw(16, ITEM_TEST_REALM);
        let payload = party_update_for_test(
            0,
            0,
            1,
            0,
            (77, 88),
            leader,
            [leader, peer],
            PERSONAL_LOOT_METHOD_LIKE_CPP,
        );

        let truncated = &payload[..payload.len() - 1];
        assert!(validate_party_update_like_cpp(truncated, leader, peer, leader).is_err());

        let mut trailing = payload;
        trailing.push(0);
        assert!(
            validate_party_update_like_cpp(&trailing, leader, peer, leader)
                .unwrap_err()
                .to_string()
                .contains("trailing byte")
        );
    }

    #[test]
    fn party_invite_matches_cpp_bit_and_field_order() {
        let payload = build_party_invite("Peer", 0x11, 0x22).unwrap();
        assert_eq!(&payload[..4], &[0x00, 0x02, 0x00, 0x00]);
        assert_eq!(&payload[4..8], &[0, 0, 0, 0]);
        assert!(payload.ends_with(b"Peer"));
    }

    #[test]
    fn loot_item_claim_contains_one_exact_request_and_soft_interact_false() {
        let window = LootWindow {
            owner_low: 1,
            owner_high: 2,
            loot_low: 0x11,
            loot_high: 0x22,
            coins: 10,
            item_entry: 46_052,
            quantity: 1,
            loot_list_id: 7,
            loot_method: PERSONAL_LOOT_METHOD_LIKE_CPP,
        };
        let payload = build_loot_item_claim(&window);
        assert_eq!(&payload[..4], &1u32.to_le_bytes());
        assert_eq!(&payload[payload.len() - 2..], &[7, 0]);
    }

    #[test]
    fn creature_guid_and_loot_unit_match_cpp_wire() {
        let (low, high) = create_creature_guid_raw(0, 62, 279_748);
        assert_eq!(low, 279_748);
        assert_eq!(high >> 58, 8);
        assert_eq!((high >> 29) & 0x1FFF, 0);
        assert_eq!((high >> 6) & 0x7F_FFFF, 62);
        assert_eq!(
            build_packed_guid(low, high),
            vec![0x07, 0x83, 0xC4, 0x44, 0x04, 0x80, 0x0F, 0x20]
        );
    }

    #[test]
    fn loot_removed_requires_the_full_discovered_creature_owner_guid() {
        const CREATURE_COUNTER: u64 = 268;
        const CREATURE_HIGH: u64 = 0x2000_0442_4015_44C0;
        let expected_owner = (CREATURE_COUNTER, CREATURE_HIGH);
        let loot_obj = (
            41,
            (HIGH_GUID_LOOT_OBJECT << 58) | (1 << 42) | (u64::from(530u16) << 29),
        );
        let removal_payload = |owner: (u64, u64)| {
            let mut payload = build_packed_guid(owner.0, owner.1);
            payload.extend_from_slice(&build_packed_guid(loot_obj.0, loot_obj.1));
            payload.push(3);
            payload
        };

        let mut evidence = WireEvidence::default();
        record_evidence(
            SMSG_LOOT_REMOVED,
            &removal_payload(expected_owner),
            expected_owner,
            &mut evidence,
        )
        .unwrap();
        assert_eq!(
            evidence.loot_removed,
            vec![LootRemovedEvidence {
                owner_low: expected_owner.0,
                owner_high: expected_owner.1,
                loot_low: loot_obj.0,
                loot_high: loot_obj.1,
                loot_list_id: 3,
            }]
        );

        let wrong_runtime_counter = (CREATURE_COUNTER + 1, CREATURE_HIGH);
        let error = record_evidence(
            SMSG_LOOT_REMOVED,
            &removal_payload(wrong_runtime_counter),
            expected_owner,
            &mut evidence,
        )
        .expect_err("a removal for another runtime creature must fail closed");
        assert!(error.to_string().contains("discovered world-object GUID"));
        assert_eq!(evidence.loot_removed.len(), 1);
    }

    #[test]
    fn item_push_parser_consumes_the_complete_cpp_343_shape_and_rejects_a_tail() {
        let mut payload = build_packed_guid(0x0102, 0);
        payload.push(4);
        for value in [-1, 777, 3, 9, 615, 123, 188, 26, 25] {
            payload.extend_from_slice(&i32::to_le_bytes(value));
        }
        payload.extend_from_slice(&build_packed_guid(0x0506, 0));
        // Pushed=true, Created=false, DisplayText=EncounterLoot,
        // IsBonusRoll=false, IsEncounterLoot=true, then byte-align.
        payload.push(0x92);
        payload.extend_from_slice(&9001i32.to_le_bytes());
        payload.extend_from_slice(&12i32.to_le_bytes());
        payload.extend_from_slice(&(-77i32).to_le_bytes());
        payload.push(0x00); // ItemBonus absent, then byte-align.
        payload.push(0x00); // ItemModList has zero 6-bit entries.

        assert_eq!(
            parse_item_push(&payload).unwrap(),
            ItemPush {
                player_low: 0x0102,
                player_high: 0,
                slot: 4,
                slot_in_bag: -1,
                quest_log_item_id: 777,
                quantity: 3,
                quantity_in_inventory: 9,
                dungeon_encounter_id: 615,
                item_guid_low: 0x0506,
                item_guid_high: 0,
                pushed: true,
                created: false,
                display_text: 2,
                is_bonus_roll: false,
                is_encounter_loot: true,
                item_entry: 9001,
            }
        );

        payload.push(0xAA);
        let error = parse_item_push(&payload)
            .expect_err("SMSG_ITEM_PUSH_RESULT trailing bytes must fail closed");
        assert!(error.to_string().contains("unexpected trailing bytes"));
    }

    #[test]
    fn atomic_item_wire_proves_one_logical_grant_for_direct_or_cpp_group_fanout() {
        for group_broadcast in [false, true] {
            let (evidence, removal) = valid_atomic_item_outcome(group_broadcast);
            let grant = validate_atomic_item_wire_outcome_like_cpp(
                &evidence,
                ITEM_TEST_CHARACTERS,
                ITEM_TEST_ENTRY,
                1,
                removal,
                ITEM_TEST_REALM,
            )
            .unwrap();
            assert_eq!(grant.owner_guid, ITEM_TEST_CHARACTERS[0]);
            assert_eq!(grant.push, valid_atomic_item_push());
        }
    }

    #[test]
    fn atomic_item_wire_rejects_foreign_push_extra_removal_and_late_duplicate() {
        let (mut foreign_push, removal) = valid_atomic_item_outcome(true);
        foreign_push[1].item_pushes[0].item_entry += 1;
        assert!(validate_atomic_item_wire_outcome_like_cpp(
            &foreign_push,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .is_err());

        let (mut extra_removal, removal) = valid_atomic_item_outcome(false);
        extra_removal[0].loot_removed.push(removal);
        assert!(validate_atomic_item_wire_outcome_like_cpp(
            &extra_removal,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .is_err());

        let (mut late_duplicate, removal) = valid_atomic_item_outcome(false);
        validate_atomic_item_wire_outcome_like_cpp(
            &late_duplicate,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .unwrap();
        late_duplicate[0].merge(WireEvidence {
            item_pushes: vec![valid_atomic_item_push()],
            ..Default::default()
        });
        assert!(validate_atomic_item_wire_outcome_like_cpp(
            &late_duplicate,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .is_err());
    }

    #[test]
    fn atomic_item_wire_rejects_foreign_or_additional_inventory_failure() {
        let (mut foreign, removal) = valid_atomic_item_outcome(false);
        foreign[1].inventory_failures[0].result = 49;
        assert!(validate_atomic_item_wire_outcome_like_cpp(
            &foreign,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .is_err());

        let (mut additional, removal) = valid_atomic_item_outcome(false);
        additional[0]
            .inventory_failures
            .push(additional[1].inventory_failures[0]);
        assert!(validate_atomic_item_wire_outcome_like_cpp(
            &additional,
            ITEM_TEST_CHARACTERS,
            ITEM_TEST_ENTRY,
            1,
            removal,
            ITEM_TEST_REALM,
        )
        .is_err());
    }

    #[test]
    fn persisted_item_row_binds_wire_guid_owner_entry_count_and_slot() {
        let expected = ExpectedPersistedItemGrant {
            owner_guid: ITEM_TEST_CHARACTERS[0],
            push: valid_atomic_item_push(),
        };
        let valid = PersistedItemGrantRow {
            item_guid: expected.push.item_guid_low,
            owner_guid: expected.owner_guid,
            item_entry: expected.push.item_entry,
            count: 1,
            inventory_owner: Some(expected.owner_guid),
            bag_guid: Some(0),
            slot: Some(INVENTORY_SLOT_ITEM_START),
            bag_slot: None,
        };
        validate_persisted_item_grant_like_cpp(expected, valid).unwrap();

        for malformed in [
            PersistedItemGrantRow {
                item_guid: valid.item_guid + 1,
                ..valid
            },
            PersistedItemGrantRow {
                owner_guid: valid.owner_guid + 1,
                ..valid
            },
            PersistedItemGrantRow {
                item_entry: valid.item_entry + 1,
                ..valid
            },
            PersistedItemGrantRow { count: 2, ..valid },
            PersistedItemGrantRow {
                slot: Some(INVENTORY_SLOT_ITEM_START + 1),
                ..valid
            },
        ] {
            assert!(validate_persisted_item_grant_like_cpp(expected, malformed).is_err());
        }
    }

    #[test]
    fn tattered_chest_template_data_pins_shared_group_rules_and_loot_id() {
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA.len(), 35);
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[0], 57);
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[1], 2_278);
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[3], 1);
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[10], 1);
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[12], 1);
        assert_eq!(RACE_GAMEOBJECT_TEMPLATE_DATA[15], 1);
        assert!(RACE_GAMEOBJECT_TEMPLATE_DATA
            .iter()
            .enumerate()
            .all(|(index, value)| matches!(index, 0 | 1 | 3 | 10 | 12 | 15) || *value == 0));
    }

    #[test]
    fn loot_object_guid_requires_exact_cpp_world_object_structure() {
        let realm_id = 7u32;
        let map_id = 571u16;
        let counter = 41u64;
        let high =
            (HIGH_GUID_LOOT_OBJECT << 58) | (u64::from(realm_id) << 42) | (u64::from(map_id) << 29);

        validate_loot_object_guid_like_cpp(counter, high, map_id, realm_id).unwrap();

        let malformed = [
            (counter, high ^ (1 << 58)),
            (counter, high ^ (1 << 42)),
            (counter, high ^ (1 << 29)),
            (counter, high | (1 << 6)),
            (counter, high | 1),
            (counter | (1 << 40), high),
            (0, high),
        ];
        for (low, high) in malformed {
            assert!(
                validate_loot_object_guid_like_cpp(low, high, map_id, realm_id).is_err(),
                "malformed LootObject unexpectedly passed: low={low:#x}, high={high:#x}"
            );
        }
    }

    #[test]
    fn serialized_money_race_requires_one_positive_and_one_zero_fanout_like_cpp() {
        let source = (0x11, 0x22);
        let positive = MoneyNotify {
            money: 10,
            money_mod: 0,
            sole_looter: true,
        };
        let zero = MoneyNotify {
            money: 0,
            money_mod: 0,
            sole_looter: true,
        };
        let evidence = [
            WireEvidence {
                money_notifies: vec![positive],
                coin_removed: vec![source, source],
                ..Default::default()
            },
            WireEvidence {
                money_notifies: vec![zero],
                coin_removed: vec![source, source],
                ..Default::default()
            },
        ];

        assert_eq!(
            validate_serialized_gameobject_money_wire_outcome_like_cpp(&evidence, source, 10)
                .unwrap(),
            0
        );

        let reversed = [evidence[1].clone(), evidence[0].clone()];
        assert_eq!(
            validate_serialized_gameobject_money_wire_outcome_like_cpp(&reversed, source, 10)
                .unwrap(),
            1
        );
    }

    #[test]
    fn serialized_money_race_rejects_duplicate_positive_or_missing_coin_fanout() {
        let source = (0x11, 0x22);
        let positive = MoneyNotify {
            money: 10,
            money_mod: 0,
            sole_looter: true,
        };
        let zero = MoneyNotify {
            money: 0,
            money_mod: 0,
            sole_looter: true,
        };
        let valid = WireEvidence {
            money_notifies: vec![zero],
            coin_removed: vec![source, source],
            ..Default::default()
        };

        let duplicate_positive = WireEvidence {
            money_notifies: vec![positive],
            coin_removed: vec![source, source],
            ..Default::default()
        };
        assert!(validate_serialized_gameobject_money_wire_outcome_like_cpp(
            &[duplicate_positive.clone(), duplicate_positive],
            source,
            10,
        )
        .is_err());

        let missing_coin = WireEvidence {
            money_notifies: vec![positive],
            coin_removed: vec![source],
            ..Default::default()
        };
        assert!(validate_serialized_gameobject_money_wire_outcome_like_cpp(
            &[valid.clone(), missing_coin],
            source,
            10,
        )
        .is_err());

        let wrong_sole_looter = WireEvidence {
            money_notifies: vec![MoneyNotify {
                money: 10,
                money_mod: 0,
                sole_looter: false,
            }],
            coin_removed: vec![source, source],
            ..Default::default()
        };
        assert!(validate_serialized_gameobject_money_wire_outcome_like_cpp(
            &[valid, wrong_sole_looter],
            source,
            10,
        )
        .is_err());
    }

    #[test]
    fn loot_gone_failure_matches_cpp_wire_and_rejects_a_tail() {
        let mut payload = 50i32.to_le_bytes().to_vec();
        payload.extend_from_slice(&build_packed_guid(0, 0));
        payload.extend_from_slice(&build_packed_guid(0, 0));
        payload.push(0);
        assert_eq!(
            parse_inventory_failure(&payload).unwrap(),
            InventoryFailure {
                result: 50,
                item_0_low: 0,
                item_0_high: 0,
                item_1_low: 0,
                item_1_high: 0,
                container_b_slot: 0,
            }
        );
        payload.push(0);
        assert!(parse_inventory_failure(&payload).is_err());
    }

    #[test]
    fn money_notify_reads_two_u64s_and_msb_sole_looter_bit() {
        let mut payload = 3u64.to_le_bytes().to_vec();
        payload.extend_from_slice(&2u64.to_le_bytes());
        payload.push(0x80);
        let mut evidence = WireEvidence::default();
        record_evidence(SMSG_LOOT_MONEY_NOTIFY, &payload, (0, 0), &mut evidence).unwrap();
        assert_eq!(
            evidence.money_notifies,
            vec![MoneyNotify {
                money: 3,
                money_mod: 2,
                sole_looter: true,
            }]
        );
        payload.push(0);
        assert!(record_evidence(SMSG_LOOT_MONEY_NOTIFY, &payload, (0, 0), &mut evidence).is_err());
    }

    #[test]
    fn single_item_capture_requires_one_owner_push_one_removal_and_no_money() {
        let window = LootWindow {
            owner_low: 1,
            owner_high: 2,
            loot_low: 0x11,
            loot_high: 0x22,
            coins: 7,
            item_entry: 56_147,
            quantity: 1,
            loot_list_id: 3,
            loot_method: 0,
        };
        let expected_player = (0x33, 0x44);
        let evidence = WireEvidence {
            item_pushes: vec![ItemPush {
                player_low: expected_player.0,
                player_high: expected_player.1,
                item_entry: window.item_entry,
                slot: INVENTORY_SLOT_BAG_0,
                slot_in_bag: i32::from(LOOT_ITEM_CAPTURE_KEYRING_SLOT),
                quantity: 1,
                quantity_in_inventory: 1,
                ..Default::default()
            }],
            loot_removed: vec![LootRemovedEvidence {
                owner_low: window.owner_low,
                owner_high: window.owner_high,
                loot_low: window.loot_low,
                loot_high: window.loot_high,
                loot_list_id: window.loot_list_id,
            }],
            ..Default::default()
        };

        validate_single_item_capture_evidence(
            &evidence,
            expected_player,
            window.item_entry,
            &window,
        )
        .unwrap();

        let mut with_money = evidence.clone();
        with_money.money_notifies.push(MoneyNotify {
            money: 7,
            money_mod: 0,
            sole_looter: true,
        });
        assert!(validate_single_item_capture_evidence(
            &with_money,
            expected_player,
            window.item_entry,
            &window,
        )
        .is_err());

        let mut displaced_key = evidence.clone();
        displaced_key.item_pushes[0].slot_in_bag = i32::from(INVENTORY_SLOT_ITEM_START);
        assert!(validate_single_item_capture_evidence(
            &displaced_key,
            expected_player,
            window.item_entry,
            &window,
        )
        .is_err());

        let mut wrong_route_owner = evidence;
        wrong_route_owner.item_pushes[0].player_low ^= 1;
        assert!(validate_single_item_capture_evidence(
            &wrong_route_owner,
            expected_player,
            window.item_entry,
            &window,
        )
        .is_err());
    }

    #[test]
    fn capture_collector_fails_closed_on_duplicate_or_failure_before_fence() {
        let push = ItemPush {
            player_low: 1,
            player_high: 2,
            item_entry: 56_147,
            ..Default::default()
        };
        validate_single_item_capture_candidate(&WireEvidence {
            item_pushes: vec![push],
            ..Default::default()
        })
        .unwrap();

        assert!(validate_single_item_capture_candidate(&WireEvidence {
            item_pushes: vec![push, push],
            ..Default::default()
        })
        .is_err());
        assert!(validate_single_item_capture_candidate(&WireEvidence {
            inventory_failures: vec![InventoryFailure {
                result: 50,
                item_0_low: 0,
                item_0_high: 0,
                item_1_low: 0,
                item_1_high: 0,
                container_b_slot: 0,
            }],
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn capture_exclusivity_requires_zero_globally_online_characters() {
        validate_online_character_count(0, "test preflight").unwrap();
        let error = validate_online_character_count(1, "test preflight")
            .expect_err("one unrelated online character must fail closed");
        assert!(error.to_string().contains("exclusive world access"));
        assert!(error.to_string().contains("1 character(s)"));
    }

    #[test]
    fn progress_restore_table_scope_has_no_duplicates_or_global_tables() {
        let unique = CHARACTER_PROGRESS_TABLES
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), CHARACTER_PROGRESS_TABLES.len());
        assert!(unique.contains("character_achievement"));
        assert!(unique.contains("character_achievement_progress"));
        assert!(unique.contains("character_queststatus_objectives_criteria_progress"));
        assert!(unique.contains("character_reputation"));
        assert!(!unique.iter().any(|table| table.starts_with("guild_")));
    }
}
