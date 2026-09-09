//! Loot-race misc operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) const DEFAULT_ACCOUNT_A: &str = "TESTBOT2@bot.local";
pub(crate) const DEFAULT_ACCOUNT_B: &str = "TESTBOT3@bot.local";
// Race and strict capture intentionally use different fixture kinds. The
// wrapper installs exactly one non-pooled Tattered Chest spawn before the
// world starts; Doctor Maleficus remains the creature-only capture fixture.
// The public constant names remain stable for existing callers; main.rs now
// exposes GameObject spellings and retains `--loot-race-creature-*` aliases.
pub(crate) const DEFAULT_CREATURE_ENTRY: u32 = 2_846;
pub(crate) const DEFAULT_CREATURE_SPAWN_GUID: u64 = 9_106_001;
pub(crate) const DEFAULT_RUNTIME_COUNTER: u64 = 0;
pub(crate) const DEFAULT_ITEM_ENTRY: u32 = 38;
pub(crate) const DEFAULT_CAPTURE_CREATURE_ENTRY: u32 = 21_779;
pub(crate) const DEFAULT_CAPTURE_CREATURE_SPAWN_GUID: u64 = 1_117;
pub(crate) const DEFAULT_CAPTURE_RUNTIME_COUNTER: u64 = 0;
pub(crate) const DEFAULT_CAPTURE_ITEM_ENTRY: u32 = 30_712;
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub(crate) const DEFAULT_WORKFLOW_DEADLINE_SECS: u64 = 900;
pub(crate) const ACK_FLAG: &str = "--ack-disposable-overworld-loot-race";
pub(crate) const DEFAULT_GROUP_CAPACITY_LEADER: &str = "TESTBOT1@bot.local";
pub(crate) const DEFAULT_GROUP_CAPACITY_CANDIDATE_A: &str = "TESTBOT2@bot.local";
pub(crate) const DEFAULT_GROUP_CAPACITY_CANDIDATE_B: &str = "TESTBOT3@bot.local";
pub(crate) const DEFAULT_GROUP_CAPACITY_TIMEOUT_SECS: u64 = 30;
pub(crate) const GUARDED_FIXTURE_HEALTH_MODIFIER: f32 = 0.0001;
pub(crate) const RACE_GAMEOBJECT_MAP_ID: u16 = 0;
pub(crate) const RACE_GAMEOBJECT_X: f64 = -8_946.95;
pub(crate) const RACE_GAMEOBJECT_Y: f64 = -132.493;
pub(crate) const RACE_GAMEOBJECT_Z: f64 = 83.5312;
pub(crate) const RACE_GAMEOBJECT_LOOT_ID: u32 = 2_278;
pub(crate) const RACE_GAMEOBJECT_MONEY: u32 = 10;
pub(crate) const RACE_GAMEOBJECT_ADDON_FACTION: u16 = 101;
pub(crate) const RACE_GAMEOBJECT_RESPAWN_SECS: u32 = 300;
pub(crate) const RACE_GAMEOBJECT_STATE: u8 = 1;
pub(crate) const RACE_GAMEOBJECT_ANIM_PROGRESS: u8 = 255;
pub(crate) const RACE_GAMEOBJECT_TEMPLATE_DATA: [i32; 35] = [
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
pub(crate) const PERSONAL_LOOT_METHOD_LIKE_CPP: u8 = 5;
pub(crate) const HIGH_GUID_CREATURE: u64 = 8;
pub(crate) const HIGH_GUID_GAMEOBJECT: u64 = 11;

pub(crate) const CMSG_PARTY_INVITE: u16 = 0x3604;
pub(crate) const CMSG_PARTY_INVITE_RESPONSE: u16 = 0x3606;
pub(crate) const CMSG_LEAVE_GROUP: u16 = 0x364C;
pub(crate) const CMSG_GAME_OBJ_USE: u16 = 0x34EE;
pub(crate) const CMSG_LOOT_UNIT: u16 = 0x320F;
pub(crate) const CMSG_LOOT_ITEM: u16 = 0x3211;
pub(crate) const CMSG_LOOT_MONEY: u16 = 0x3210;
pub(crate) const SMSG_PARTY_INVITE: u16 = 0x25BD;
pub(crate) const SMSG_PARTY_UPDATE: u16 = 0x25F4;
pub(crate) const SMSG_PARTY_COMMAND_RESULT: u16 = 0x2796;
pub(crate) const SMSG_PARTY_MEMBER_FULL_STATE: u16 = 0x2759;
pub(crate) const SMSG_LOOT_RESPONSE: u16 = 0x2614;
pub(crate) const SMSG_LOOT_REMOVED: u16 = 0x2615;
pub(crate) const SMSG_LOOT_MONEY_NOTIFY: u16 = 0x261C;
pub(crate) const SMSG_COIN_REMOVED: u16 = 0x2617;
pub(crate) const SMSG_ITEM_PUSH_RESULT: u16 = 0x2623;
pub(crate) const RESPONSE_SETTLE: Duration = Duration::from_secs(2);
pub(crate) const LOOT_ITEM_CAPTURE_FENCE_SERIAL: u32 = 0x4C4F_4F54;
// The Doctor's Key (30712) is a key and C++ stores this exact fixture in the
// first keyring destination represented by the committed capture.
pub(crate) const LOOT_ITEM_CAPTURE_KEYRING_SLOT: u8 = 106;
// Normal C++ logout consumes 20 seconds before SMSG_LOGOUT_COMPLETE. Leave a
// bounded disconnect-save margin before any destructive fixture restoration.
pub(crate) const LOOT_FIXTURE_OFFLINE_WAIT_SECS: u64 = 90;
pub(crate) const LOOT_LOGOUT_DB_CONFIRM_WAIT_SECS: u64 = 5;
pub(crate) const LOOT_DB_OPERATION_TIMEOUT_SECS: u64 = 30;
pub(crate) const LOOT_CLEANUP_TIMEOUT_SECS: u64 = 180;
pub(crate) const FIXTURE_JOURNAL_VERSION: u32 = 1;
pub(crate) const FIXTURE_JOURNAL_ENV: &str = "WOW_BOT_FIXTURE_JOURNAL";
pub(crate) const HIGH_GUID_ITEM: u64 = 3;
pub(crate) const HIGH_GUID_LOOT_OBJECT: u64 = 15;
pub(crate) const GUID_HIGH_TYPE_MASK: u64 = 0x3F;
pub(crate) const GUID_REALM_SPECIFIC_MASK: u64 = 0xFFFF;
pub(crate) const GUID_REALM_MASK: u64 = 0x1FFF;
pub(crate) const GUID_MAP_MASK: u64 = 0x1FFF;
pub(crate) const GUID_ENTRY_MASK: u64 = 0x7F_FFFF;
pub(crate) const GUID_SUBTYPE_MASK: u64 = 0x3F;
pub(crate) const GUID_SERVER_MASK: u64 = 0xFF_FFFF;
pub(crate) const GUID_COUNTER_MASK: u64 = 0xFF_FFFF_FFFF;
// C++ Player.cpp `MAX_MONEY_AMOUNT` and Rust's represented player cap.
pub(crate) const MAX_PLAYER_MONEY_LIKE_CPP: u64 = 99_999_999_999;
pub(crate) const CHARACTER_PROGRESS_TABLES: &[&str] = &[
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
pub(crate) struct LootRaceCli {
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
pub(crate) enum LootRacePhase {
    Race,
    CaptureItem,
    VerifyRelog,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum LootRaceTargetKind {
    Creature,
    GameObject,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LootFixturePurpose {
    Race,
    CaptureItem,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogoutCompletionRoute {
    Realm,
    Instance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartyPacketRoute {
    Realm,
    Instance,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LootRaceTarget {
    pub(crate) kind: LootRaceTargetKind,
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
pub(crate) struct LootRaceOptions {
    pub phase: LootRacePhase,
    pub participant: usize,
    pub character_guid: u64,
    pub peer_name: String,
    pub peer_character_guid: u64,
    pub killer_character_guid: u64,
    pub target: LootRaceTarget,
    pub timeout_secs: u64,
    pub(crate) sync: Arc<LootRaceSync>,
}
#[derive(Debug, Clone)]
pub(crate) struct GroupCapacityRaceCli {
    pub leader_account: String,
    pub candidate_a_account: String,
    pub candidate_b_account: String,
    pub group_db_store_id: u32,
    pub timeout_secs: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupCapacityRaceRole {
    Leader,
    CandidateA,
    CandidateB,
}
#[derive(Debug, Clone)]
pub(crate) struct GroupCapacityRaceOptions {
    pub role: GroupCapacityRaceRole,
    pub character_guid: u64,
    pub(crate) leader_guid: u64,
    pub(crate) candidate_names: [String; 2],
    pub(crate) candidate_guids: [u64; 2],
    pub(crate) initial_member_guids: [u64; 4],
    pub(crate) party_settings: GroupCapacityPartySettings,
    pub group_db_store_id: u32,
    pub timeout_secs: u64,
    pub(crate) auth_serial: Arc<Mutex<()>>,
    pub(crate) sync: Arc<GroupCapacityRaceSync>,
}
#[derive(Debug)]
pub(crate) struct GroupCapacityRaceSync {
    pub(crate) logged_in: Barrier,
    pub(crate) invitations_sent: Barrier,
    pub(crate) accepts_ready: Barrier,
    pub(crate) outcomes_observed: Barrier,
    pub(crate) cancelled: CancellationToken,
    pub(crate) failure: StdMutex<Option<String>>,
}
impl GroupCapacityRaceSync {
    pub(crate) fn new() -> Self {
        Self {
            logged_in: Barrier::new(3),
            invitations_sent: Barrier::new(3),
            accepts_ready: Barrier::new(2),
            outcomes_observed: Barrier::new(3),
            cancelled: CancellationToken::new(),
            failure: StdMutex::new(None),
        }
    }

    pub(crate) fn cancel(&self, message: impl Into<String>) {
        let message = message.into();
        let mut failure = self.failure.lock().expect("group-capacity failure lock");
        if failure.is_none() {
            *failure = Some(message);
        }
        self.cancelled.cancel();
    }

    pub(crate) fn cancellation_error(&self) -> Result<()> {
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
pub(crate) struct CharacterFixture {
    pub(crate) bot: config::BotConfig,
    pub(crate) name: String,
    pub(crate) race: u8,
    pub(crate) money: u64,
    pub(crate) core: CharacterCoreSnapshot,
    pub(crate) position: CharacterPositionSnapshot,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CharacterCoreSnapshot {
    pub(crate) level: u8,
    pub(crate) xp: u32,
    pub(crate) health: u32,
    pub(crate) powers: [u32; 10],
    pub(crate) rest_state: u8,
    pub(crate) rest_bonus: f32,
    pub(crate) explored_zones: Option<String>,
    pub(crate) known_titles: Option<String>,
    pub(crate) chosen_title: u32,
}
/// Persistent rows that a creature kill, item acquisition, or the resulting
/// C++ criteria fanout can mutate.  These are snapshotted for both dedicated
/// disposable characters and restored transactionally after every run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CharacterProgressSnapshot {
    pub(crate) achievements: Vec<(u64, u32, i64)>,
    pub(crate) achievement_progress: Vec<(u64, u32, u64, i64)>,
    pub(crate) quest_status: Vec<(u64, u32, u8, u8, i64, i64)>,
    pub(crate) quest_daily: Vec<(u64, u32, i64)>,
    pub(crate) quest_monthly: Vec<(u64, u32)>,
    pub(crate) quest_objectives: Vec<(u64, u32, u8, i32)>,
    pub(crate) quest_objective_criteria: Vec<(u64, u32)>,
    pub(crate) quest_objective_criteria_progress: Vec<(u64, u32, u64, i64)>,
    pub(crate) quest_rewarded: Vec<(u64, u32, u8)>,
    pub(crate) quest_seasonal: Vec<(u64, u32, u32, i64)>,
    pub(crate) quest_weekly: Vec<(u64, u32)>,
    pub(crate) reputation: Vec<(u64, u16, i32, u16)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RespawnSnapshot {
    pub(crate) respawn_time: i64,
    pub(crate) map_id: u16,
    pub(crate) instance_id: u32,
}
#[derive(Debug, Clone)]
pub(crate) struct LootRaceFixture {
    pub(crate) characters: [CharacterFixture; 2],
    pub(crate) target: LootRaceTarget,
    pub(crate) respawn: Option<RespawnSnapshot>,
    pub(crate) respawn_type: u16,
    pub(crate) gameobject_state: Option<u8>,
    pub(crate) progress: CharacterProgressSnapshot,
    pub(crate) journal: FixtureJournal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct JournalCharacterFixture {
    pub(crate) account: String,
    pub(crate) account_id: u32,
    pub(crate) character_guid: u64,
    pub(crate) name: String,
    pub(crate) race: u8,
    pub(crate) money: u64,
    pub(crate) core: CharacterCoreSnapshot,
    pub(crate) position: CharacterPositionSnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FixtureJournalRecord {
    pub(crate) version: u32,
    pub(crate) created_by_pid: u32,
    pub(crate) characters: [JournalCharacterFixture; 2],
    pub(crate) target: LootRaceTarget,
    pub(crate) respawn: Option<RespawnSnapshot>,
    pub(crate) respawn_type: u16,
    pub(crate) gameobject_state: Option<u8>,
    pub(crate) progress: CharacterProgressSnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CleanupMarkerRecord {
    pub(crate) version: u32,
    pub(crate) journal_sha256: String,
    pub(crate) cleanup_pid: u32,
}
#[derive(Debug, Clone)]
pub(crate) struct FixtureJournal {
    pub(crate) path: PathBuf,
}
pub(crate) fn validate_journal_contract() -> Result<()> {
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
    pub(crate) fn from_fixture(fixture: &LootRaceFixture) -> Self {
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

    pub(crate) fn into_fixture(self, journal: FixtureJournal) -> Result<LootRaceFixture> {
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
    pub(crate) fn configured() -> Result<Self> {
        Ok(Self {
            path: configured_fixture_journal_path()?,
        })
    }

    pub(crate) fn persist(&self, fixture: &LootRaceFixture) -> Result<()> {
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

    pub(crate) fn load(path: PathBuf) -> Result<LootRaceFixture> {
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

    pub(crate) fn complete(&self) -> Result<()> {
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
pub(crate) fn sync_parent_directory(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path {} has no parent", path.display()))?;
    let directory = fs::File::open(parent)
        .with_context(|| format!("Open directory {} for fsync", parent.display()))?;
    directory
        .sync_all()
        .with_context(|| format!("fsync directory {}", parent.display()))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LootWindow {
    pub(crate) owner_low: u64,
    pub(crate) owner_high: u64,
    pub(crate) loot_low: u64,
    pub(crate) loot_high: u64,
    pub(crate) coins: u32,
    pub(crate) item_entry: u32,
    pub(crate) quantity: u32,
    pub(crate) loot_list_id: u8,
    pub(crate) loot_method: u8,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct WireEvidence {
    pub(crate) item_pushes: Vec<ItemPush>,
    pub(crate) loot_removed: Vec<LootRemovedEvidence>,
    pub(crate) money_notifies: Vec<MoneyNotify>,
    pub(crate) coin_removed: Vec<(u64, u64)>,
    pub(crate) inventory_failures: Vec<InventoryFailure>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LootRemovedEvidence {
    pub(crate) owner_low: u64,
    pub(crate) owner_high: u64,
    pub(crate) loot_low: u64,
    pub(crate) loot_high: u64,
    pub(crate) loot_list_id: u8,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ItemPush {
    pub(crate) player_low: u64,
    pub(crate) player_high: u64,
    pub(crate) slot: u8,
    pub(crate) slot_in_bag: i32,
    pub(crate) quest_log_item_id: i32,
    pub(crate) quantity: i32,
    pub(crate) quantity_in_inventory: i32,
    pub(crate) dungeon_encounter_id: i32,
    pub(crate) item_guid_low: u64,
    pub(crate) item_guid_high: u64,
    pub(crate) pushed: bool,
    pub(crate) created: bool,
    pub(crate) display_text: u8,
    pub(crate) is_bonus_roll: bool,
    pub(crate) is_encounter_loot: bool,
    pub(crate) item_entry: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WireItemGrant {
    pub(crate) participant: usize,
    pub(crate) push: ItemPush,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedPersistedItemGrant {
    pub(crate) owner_guid: u64,
    pub(crate) push: ItemPush,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExpectedPersistedMoneyGrant {
    pub(crate) owner_guid: u64,
    pub(crate) amount: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PersistedItemGrantRow {
    pub(crate) item_guid: u64,
    pub(crate) owner_guid: u64,
    pub(crate) item_entry: u32,
    pub(crate) count: u32,
    pub(crate) inventory_owner: Option<u64>,
    pub(crate) bag_guid: Option<u64>,
    pub(crate) slot: Option<u8>,
    pub(crate) bag_slot: Option<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventoryFailure {
    pub(crate) result: i32,
    pub(crate) item_0_low: u64,
    pub(crate) item_0_high: u64,
    pub(crate) item_1_low: u64,
    pub(crate) item_1_high: u64,
    pub(crate) container_b_slot: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoneyNotify {
    pub(crate) money: u64,
    pub(crate) money_mod: u64,
    pub(crate) sole_looter: bool,
}
#[derive(Debug)]
pub(crate) struct LootRaceSync {
    pub(crate) logged_in: Barrier,
    pub(crate) party_ready: Barrier,
    pub(crate) positioned: Barrier,
    pub(crate) use_ready: Barrier,
    pub(crate) response_received: Barrier,
    pub(crate) windows_ready: Barrier,
    pub(crate) item_claim: Barrier,
    pub(crate) item_observed: Barrier,
    pub(crate) money_claim: Barrier,
    pub(crate) money_observed: Barrier,
    pub(crate) before_leave: Barrier,
    pub(crate) windows: Mutex<[Option<LootWindow>; 2]>,
    pub(crate) evidence: Mutex<[WireEvidence; 2]>,
    pub(crate) cancel_reason: StdMutex<Option<String>>,
    pub(crate) cancellation: CancellationToken,
    /// Complete live ObjectGuid observed on the wire. C++ includes realm bits
    /// in the high half, so reconstructing it from the SQL spawn/counter is
    /// not equivalent to preserving this value.
    pub(crate) runtime_guid: StdMutex<Option<(u64, u64)>>,
}
impl LootRaceSync {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn cancel(&self, reason: impl Into<String>) {
        let reason = reason.into();
        if let Ok(mut stored) = self.cancel_reason.lock() {
            if stored.is_none() {
                *stored = Some(reason);
            }
        }
        self.cancellation.cancel();
    }

    pub(crate) fn cancellation_error(&self) -> Result<()> {
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

    pub(crate) async fn cancelled(&self) {
        self.cancellation.cancelled().await;
    }
}
impl LootRaceOptions {
    pub(crate) fn resolved_runtime_guid(&self) -> Result<(u64, u64)> {
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

    pub(crate) fn resolved_runtime_counter(&self) -> Result<u64> {
        Ok(self.resolved_runtime_guid()?.0 & OBJECT_GUID_COUNTER_MASK)
    }

    pub(crate) fn resolved_packed_guid(&self) -> Result<Vec<u8>> {
        let (low, high) = self.resolved_runtime_guid()?;
        Ok(build_packed_guid(low, high))
    }
}
pub(crate) fn validate_cli(
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
pub(crate) fn record_discovered_runtime_guid(
    options: &LootRaceOptions,
    candidate: (u64, u64),
) -> Result<u64> {
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
pub(crate) fn target_seen_in_update(
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
