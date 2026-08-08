//! Fail-closed provenance for capture imports.
//!
//! Capture wrappers publish a raw artifact and a side-specific manifest only
//! after the accredited process is gone and the runtime has been restored.
//! Import validates those manifests against the raw bytes, keeps exact copies
//! beside the committed fixture, and publishes the complete derived flow with
//! one atomic directory exchange. [`verify_required_lineage`] then binds the
//! copied raw manifests to every filtered output used by `verify-required`.

use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Capture, Direction, PacketBoundary};
use crate::semantic::{
    CorrelatedSpellGuidBody, ISSUE_24_PING_FENCE_SERIAL, SMSG_SPELL_GO, SMSG_SPELL_START,
    decode_spell_go_body, decode_spell_start_body,
};

/// Completion marker for one fully derived capture flow.
pub const LINEAGE_FILE: &str = "capture-lineage.json";
/// Exact raw manifests retained with the derived fixture.
pub const RAW_PROVENANCE_DIR: &str = "capture-provenance";

const CPP_RAW_MANIFEST_FILE: &str = "cpp.capture-manifest.json";
const RUST_RAW_MANIFEST_FILE: &str = "rust.capture-manifest.json";
const CPP_BOT_REPORT_FILE: &str = "cpp.bot-report.json";
const RUST_BOT_REPORT_FILE: &str = "rust.bot-report.json";
const SHA256_HEX_LEN: usize = 64;
const DETOUR_FIXTURE_MANIFEST_SHA256: &str =
    "3a6c2aa6081974ef9cf13b8f63f739c402b18799f9833f80819f5e7e0de8d013";
const DETOUR_FIXTURE_MAP_SHA256: &str =
    "3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a";
const DETOUR_FIXTURE_TILE_SHA256: &str =
    "693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47";
const CREATURE_SPELL_FIXTURE_MANIFEST_SHA256: &str =
    "fe6cea1808e8beb7d648d285ad52b10067611e46c55d30637514508275b63b49";
const CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT: &str =
    "creature-spell-casting-cpp-source-patch-v1";
const CREATURE_SPELL_CPP_REMOTE_URL: &str = "https://github.com/alseif0x/TrinityCoreLegacyTest.git";
const CREATURE_SPELL_CPP_REMOTE_REF: &str = "refs/remotes/origin/3.4.3";
const CREATURE_SPELL_CPP_BASE_HEAD: &str = "a5f8da2ebf5424bf0450ca4e08843ecbf72577bd";
const CREATURE_SPELL_CPP_BASE_TREE: &str = "bb5c4746be7f9944b1a3f7a1eec5ea88d62fff67";
const CREATURE_SPELL_CPP_PATCHED_HEAD: &str = "8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c";
const CREATURE_SPELL_CPP_PATCHED_TREE: &str = "228e91ed36886593c85fb601d00a9f8eb0702137";
const CREATURE_SPELL_CPP_PATCH_PATH: &str =
    "crates/capture-diff/flows/creature-spell-casting/fixture/cpp-reference.patch";
const CREATURE_SPELL_CPP_PATCH_SHA256: &str =
    "ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262";
const CREATURE_SPELL_CPP_CHANGED_PATH: &str = "src/server/game/DataStores/DB2Stores.cpp";
const CREATURE_SPELL_PLAYER_GUID_HIGH: u64 = 0x0800_0400_0000_0000;
const LINEAGE_VERSION: u32 = 3;
const RAW_MANIFEST_VERSION: u32 = 3;

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawSide {
    Cpp,
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArtifact {
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    packet_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tree_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyntheticMmapEvidence {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedReadOnlyDataEvidence {
    name: String,
    target_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureGuardEvidence {
    enabled: bool,
    contract: String,
    account: String,
    account_id: u32,
    character_guid: u64,
    peer_account: String,
    peer_account_id: u32,
    peer_character_guid: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    creature_entry: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    creature_spawn_guid: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gameobject_entry: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gameobject_spawn_guid: Option<u64>,
    item_entry: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    character_account_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    normal_data_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    private_data_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    private_data_dir_removed_before_normal_runtime: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixture_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixture_manifest_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    synthetic_mmaps: Option<Vec<SyntheticMmapEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    linked_read_only_data: Option<Vec<LinkedReadOnlyDataEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    journal_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    database_snapshot_sha256: Option<String>,
    cleanup_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BotReportEvidence {
    contract: String,
    exec_path: String,
    exec_sha256: String,
    report_path: String,
    report_sha256: String,
    account: String,
    account_id: u32,
    character_guid: u64,
    report_validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceDerivationEvidence {
    contract: String,
    remote_url: String,
    remote_ref: String,
    base_head: String,
    base_tree: String,
    patched_head: String,
    patched_tree: String,
    patch_path: String,
    patch_sha256: String,
    changed_paths: Vec<String>,
}

/// Schema emitted by `capture-cpp.sh` and `capture-rust.sh`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCaptureManifest {
    version: u32,
    flow: String,
    side: RawSide,
    completed: bool,
    created_at: String,
    harness_repo_head: String,
    source_repo_head: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_exec_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_derivation: Option<SourceDerivationEvidence>,
    harness_worktree_clean: bool,
    harness_worktree_state_sha256: String,
    source_worktree_dirty: bool,
    source_worktree_state_sha256: String,
    worktree_state_algorithm: String,
    expected_exec_path: String,
    expected_exec_sha256: String,
    source_exec_path: String,
    source_exec_sha256: String,
    live_exec_path: String,
    live_exec_sha256: String,
    executable_pin_enforced: bool,
    pm2_entry_pid: u32,
    pm2_entry_starttime: u64,
    pm2_exec_path: String,
    pm2_exec_sha256: String,
    pm2_profile_redacted_sha256: String,
    listener_runtime_pid: u32,
    listener_runtime_starttime: u64,
    listener_relationship_verified: bool,
    restart_count: u64,
    effective_config_path: String,
    effective_config_redacted_sha256: String,
    effective_config_algorithm: String,
    runtime_cleanup_verified: bool,
    normal_runtime_restored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixture_guard: Option<FixtureGuardEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bot_report: Option<BotReportEvidence>,
    artifact: RawArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedRawSide {
    manifest_bytes: Vec<u8>,
    manifest_sha256: String,
    manifest: RawCaptureManifest,
    bot_report_bytes: Option<Vec<u8>>,
}

/// A pair of raw manifests whose declared artifacts matched the supplied raw
/// capture bytes at import time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRawPair {
    cpp: ValidatedRawSide,
    rust: ValidatedRawSide,
}

/// One boundary recorded in the derived import contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineageBoundary {
    direction: Option<Direction>,
    opcode: u16,
}

impl From<PacketBoundary> for LineageBoundary {
    fn from(value: PacketBoundary) -> Self {
        Self {
            direction: value.direction,
            opcode: value.opcode,
        }
    }
}

/// Exact selection applied symmetrically to the two raw captures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSelection {
    directions: Vec<Direction>,
    from_opcode: Option<LineageBoundary>,
    until_opcode: Option<LineageBoundary>,
    ignored_opcodes: Vec<LineageBoundary>,
    strict: bool,
}

impl ImportSelection {
    /// Record all flags that can change the derived evidence.
    #[must_use]
    pub fn new(
        directions: Vec<Direction>,
        from_opcode: Option<PacketBoundary>,
        until_opcode: Option<PacketBoundary>,
        ignored_opcodes: &[PacketBoundary],
        strict: bool,
    ) -> Self {
        Self {
            directions,
            from_opcode: from_opcode.map(Into::into),
            until_opcode: until_opcode.map(Into::into),
            ignored_opcodes: ignored_opcodes.iter().copied().map(Into::into).collect(),
            strict,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceLineage {
    manifest_path: String,
    manifest_sha256: String,
    raw_artifact_sha256: String,
    raw_artifact_size: Option<u64>,
    raw_packet_count: Option<u64>,
    harness_repo_head: String,
    source_repo_head: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_derivation: Option<SourceDerivationEvidence>,
    harness_worktree_clean: bool,
    harness_worktree_state_sha256: String,
    source_worktree_dirty: bool,
    source_worktree_state_sha256: String,
    worktree_state_algorithm: String,
    expected_exec_path: String,
    expected_exec_sha256: String,
    source_exec_path: String,
    source_exec_sha256: String,
    live_exec_path: String,
    live_exec_sha256: String,
    executable_pin_enforced: bool,
    pm2_entry_pid: u32,
    pm2_entry_starttime: u64,
    pm2_exec_path: String,
    pm2_exec_sha256: String,
    pm2_profile_redacted_sha256: String,
    listener_runtime_pid: u32,
    listener_runtime_starttime: u64,
    listener_relationship_verified: bool,
    restart_count: u64,
    effective_config_path: String,
    effective_config_redacted_sha256: String,
    effective_config_algorithm: String,
    runtime_cleanup_verified: bool,
    normal_runtime_restored: bool,
    fixture_guard: Option<FixtureGuardEvidence>,
    bot_report: Option<BotReportEvidence>,
    retained_bot_report_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePairLineage {
    cpp: SourceLineage,
    rust: SourceLineage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileLineage {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TreeLineage {
    path: String,
    file_count: u64,
    packet_count: u64,
    tree_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DerivedOutputs {
    cpp_pkt: FileLineage,
    rust: TreeLineage,
    expected_divergences: FileLineage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DerivedLineage {
    version: u32,
    flow: String,
    completed: bool,
    sources: SourcePairLineage,
    selection: ImportSelection,
    outputs: DerivedOutputs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeDigest {
    sha256: String,
    file_count: u64,
    packet_count: u64,
}

/// Validate raw capture manifests and bind them to the exact raw artifacts.
/// Required flows additionally require executable pinning on both sides.
pub fn validate_raw_pair(
    flow: &str,
    cpp_capture: &Path,
    cpp_manifest: &Path,
    rust_capture: &Path,
    rust_manifest: &Path,
    require_pinned_execs: bool,
) -> Result<ValidatedRawPair> {
    let mut cpp =
        read_and_validate_raw_manifest(cpp_manifest, flow, RawSide::Cpp, require_pinned_execs)
            .with_context(|| format!("validating C++ raw manifest {}", cpp_manifest.display()))?;
    let mut rust =
        read_and_validate_raw_manifest(rust_manifest, flow, RawSide::Rust, require_pinned_execs)
            .with_context(|| format!("validating Rust raw manifest {}", rust_manifest.display()))?;

    validate_cpp_artifact(&cpp.manifest, cpp_capture)?;
    validate_rust_artifact(&rust.manifest, rust_capture, Some(rust_manifest))?;
    cpp.bot_report_bytes = validate_bot_report_artifact(&cpp.manifest)?;
    rust.bot_report_bytes = validate_bot_report_artifact(&rust.manifest)?;
    validate_cross_side_identity(flow, &cpp.manifest, &rust.manifest)?;

    Ok(ValidatedRawPair { cpp, rust })
}

/// Bind the side-specific bot reports to the exact packet bodies selected from
/// the raw artifacts. A valid report SHA alone proves only that a report file
/// was retained; these body digests prove that the bot and packet importer are
/// describing the same isolated action execution.
pub fn validate_bot_report_capture_binding(
    flow: &str,
    raw: &ValidatedRawPair,
    cpp: &Capture,
    rust: &Capture,
) -> Result<()> {
    match flow {
        "detour-chase-around-obstacle" => {
            validate_detour_report_capture_binding("C++", &raw.cpp, cpp)?;
            validate_detour_report_capture_binding("Rust", &raw.rust, rust)
        }
        "creature-spell-casting" => {
            validate_creature_spell_report_capture_binding("C++", &raw.cpp, cpp)?;
            validate_creature_spell_report_capture_binding("Rust", &raw.rust, rust)
        }
        _ => Ok(()),
    }
}

fn validate_creature_spell_report_capture_binding(
    side_name: &str,
    side: &ValidatedRawSide,
    capture: &Capture,
) -> Result<()> {
    let report_bytes = side.bot_report_bytes.as_deref().with_context(|| {
        format!("{side_name} creature-spell capture has no validated bot report")
    })?;
    let report: serde_json::Value = serde_json::from_slice(report_bytes).with_context(|| {
        format!("parsing {side_name} creature-spell bot report for packet binding")
    })?;
    let result = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .and_then(|results| results.first())
        .context("creature-spell bot report has no result for packet binding")?;
    let string = |key: &str| {
        result
            .get(key)
            .and_then(serde_json::Value::as_str)
            .with_context(|| format!("creature-spell bot report {key} is missing"))
    };
    let u64_value = |key: &str| {
        result
            .get(key)
            .and_then(serde_json::Value::as_u64)
            .with_context(|| format!("creature-spell bot report {key} is missing"))
    };
    let [start_packet, go_packet] = capture.packets.as_slice() else {
        bail!(
            "{side_name} creature-spell packet/report binding requires the exact two-packet selected capture"
        );
    };
    ensure!(
        start_packet.opcode == SMSG_SPELL_START
            && go_packet.opcode == SMSG_SPELL_GO
            && u64_value("creature_spell_start_opcode")? == u64::from(start_packet.opcode)
            && u64_value("creature_spell_go_opcode")? == u64::from(go_packet.opcode)
            && u64_value("creature_spell_start_body_bytes")? == start_packet.body.len() as u64
            && u64_value("creature_spell_go_body_bytes")? == go_packet.body.len() as u64
            && string("creature_spell_start_body_sha256")? == sha256_bytes(&start_packet.body)
            && string("creature_spell_go_body_sha256")? == sha256_bytes(&go_packet.body),
        "{side_name} creature-spell bot report does not match selected RAW START/GO packet bodies"
    );

    let start = decode_spell_start_body(&start_packet.body)
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!("decoding {side_name} selected RAW SpellStart for report binding")
        })?;
    let go = decode_spell_go_body(&go_packet.body)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("decoding {side_name} selected RAW SpellGo for report binding"))?;
    let CorrelatedSpellGuidBody::Exact { guid: start_victim } = &start.body.cast.target.unit else {
        bail!("{side_name} selected RAW SpellStart does not carry the reported player victim")
    };
    let CorrelatedSpellGuidBody::Exact { guid: go_victim } = &go.body.target.unit else {
        bail!("{side_name} selected RAW SpellGo does not carry the reported player victim")
    };
    let [CorrelatedSpellGuidBody::Exact { guid: hit_victim }] = go.body.hit_targets.as_slice()
    else {
        bail!("{side_name} selected RAW SpellGo does not carry the reported single hit victim")
    };
    ensure!(
        start.cast_id == go.cast_id
            && start.exact_caster_guid == go.exact_caster_guid
            && start_victim == go_victim
            && go_victim == hit_victim
            && u64_value("creature_spell_cast_id_low")? == start.cast_id.low
            && u64_value("creature_spell_cast_id_high")? == start.cast_id.high
            && u64_value("creature_spell_caster_guid_low")? == start.exact_caster_guid.low
            && u64_value("creature_spell_caster_guid_high")? == start.exact_caster_guid.high
            && u64_value("creature_spell_victim_guid_low")? == start_victim.low
            && u64_value("creature_spell_victim_guid_high")? == start_victim.high
            && u64_value("creature_spell_target_runtime_counter")?
                == start.exact_caster_guid.low & 0x0000_00FF_FFFF_FFFF
            && u64_value("creature_spell_spell_id")? == start.body.cast.spell_id as u64
            && start.body.cast.spell_id == go.body.spell_id
            && u64_value("creature_spell_start_cast_flags")?
                == u64::from(start.body.cast.cast_flags)
            && u64_value("creature_spell_go_cast_flags")? == u64::from(go.body.cast_flags)
            && u64_value("creature_spell_cast_flags_ex")?
                == u64::from(start.body.cast.cast_flags_ex)
            && start.body.cast.cast_flags_ex == go.body.cast_flags_ex
            && u64_value("creature_spell_go_hit_target_count")? == go.body.hit_targets.len() as u64
            && u64_value("creature_spell_go_miss_target_count")?
                == go.body.miss_targets.len() as u64,
        "{side_name} creature-spell bot report GUID/cast topology does not match selected RAW"
    );
    Ok(())
}

fn validate_detour_report_capture_binding(
    side_name: &str,
    side: &ValidatedRawSide,
    capture: &Capture,
) -> Result<()> {
    let report_bytes = side
        .bot_report_bytes
        .as_deref()
        .with_context(|| format!("{side_name} detour capture has no validated bot report"))?;
    let report: serde_json::Value = serde_json::from_slice(report_bytes)
        .with_context(|| format!("parsing {side_name} detour bot report for packet binding"))?;
    let result = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .and_then(|results| results.first())
        .context("detour bot report has no result for packet binding")?;
    let heartbeat_sha256 = result
        .get("detour_chase_heartbeat_sha256")
        .and_then(serde_json::Value::as_str)
        .context("detour bot report has no heartbeat SHA-256 for packet binding")?;
    let monster_move_sha256 = result
        .get("detour_chase_monster_move_sha256")
        .and_then(serde_json::Value::as_str)
        .context("detour bot report has no MonsterMove SHA-256 for packet binding")?;
    let monster_move_bytes = result
        .get("detour_chase_monster_move_bytes")
        .and_then(serde_json::Value::as_u64)
        .context("detour bot report has no MonsterMove length for packet binding")?;
    ensure!(
        capture.packets.len() == 3,
        "{side_name} detour packet/report binding requires the exact three-packet selected capture"
    );
    ensure!(
        heartbeat_sha256 == sha256_bytes(&capture.packets[0].body),
        "{side_name} detour heartbeat report SHA-256 does not match selected RAW packet body"
    );
    ensure!(
        monster_move_sha256 == sha256_bytes(&capture.packets[1].body),
        "{side_name} detour MonsterMove report SHA-256 does not match selected RAW packet body"
    );
    ensure!(
        monster_move_bytes == capture.packets[1].body.len() as u64,
        "{side_name} detour MonsterMove report length does not match selected RAW packet body"
    );
    Ok(())
}

fn read_and_validate_raw_manifest(
    path: &Path,
    flow: &str,
    side: RawSide,
    require_pinned_exec: bool,
) -> Result<ValidatedRawSide> {
    let bytes = read_regular_file(path)?;
    let manifest: RawCaptureManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing raw manifest {}", path.display()))?;
    validate_raw_manifest_schema(&manifest, flow, side, require_pinned_exec)
        .with_context(|| format!("invalid raw manifest {}", path.display()))?;
    Ok(ValidatedRawSide {
        manifest_sha256: sha256_bytes(&bytes),
        manifest_bytes: bytes,
        manifest,
        bot_report_bytes: None,
    })
}

fn validate_raw_manifest_schema(
    manifest: &RawCaptureManifest,
    flow: &str,
    side: RawSide,
    require_pinned_exec: bool,
) -> Result<()> {
    ensure!(
        manifest.version == RAW_MANIFEST_VERSION,
        "unsupported raw manifest version {}",
        manifest.version
    );
    ensure!(
        manifest.flow == flow,
        "flow is {:?}, expected {flow:?}",
        manifest.flow
    );
    ensure!(
        manifest.side == side,
        "side is {:?}, expected {side:?}",
        manifest.side
    );
    ensure!(manifest.completed, "completed must be true");
    validate_utc_timestamp(&manifest.created_at)?;
    ensure!(
        valid_git_oid(&manifest.harness_repo_head),
        "harness_repo_head must be a lowercase 40- or 64-hex object id"
    );
    ensure!(
        valid_git_oid(&manifest.source_repo_head),
        "source_repo_head must be a lowercase 40- or 64-hex object id"
    );
    for (label, path) in [
        ("expected_exec_path", manifest.expected_exec_path.as_str()),
        ("source_exec_path", manifest.source_exec_path.as_str()),
        ("live_exec_path", manifest.live_exec_path.as_str()),
        ("pm2_exec_path", manifest.pm2_exec_path.as_str()),
        (
            "effective_config_path",
            manifest.effective_config_path.as_str(),
        ),
    ] {
        ensure!(Path::new(path).is_absolute(), "{label} must be absolute");
    }
    if let Some(bot) = &manifest.bot_report {
        ensure!(
            Path::new(&bot.exec_path).is_absolute(),
            "bot_report.exec_path must be absolute"
        );
        ensure!(
            Path::new(&bot.report_path).is_absolute(),
            "bot_report.report_path must be absolute"
        );
        validate_sha256(&bot.exec_sha256, "bot_report.exec_sha256")?;
        validate_sha256(&bot.report_sha256, "bot_report.report_sha256")?;
    }
    validate_sha256(&manifest.expected_exec_sha256, "expected_exec_sha256")?;
    validate_sha256(&manifest.source_exec_sha256, "source_exec_sha256")?;
    validate_sha256(&manifest.live_exec_sha256, "live_exec_sha256")?;
    validate_sha256(
        &manifest.harness_worktree_state_sha256,
        "harness_worktree_state_sha256",
    )?;
    validate_sha256(
        &manifest.source_worktree_state_sha256,
        "source_worktree_state_sha256",
    )?;
    validate_sha256(&manifest.pm2_exec_sha256, "pm2_exec_sha256")?;
    validate_sha256(
        &manifest.pm2_profile_redacted_sha256,
        "pm2_profile_redacted_sha256",
    )?;
    validate_sha256(
        &manifest.effective_config_redacted_sha256,
        "effective_config_redacted_sha256",
    )?;
    if let Some(derivation) = &manifest.source_derivation {
        validate_source_derivation_schema(derivation)?;
    }
    if flow != "creature-spell-casting" {
        ensure!(
            manifest.source_derivation.is_none(),
            "source_derivation is reserved for C++ creature-spell-casting evidence"
        );
    }
    ensure!(
        manifest.expected_exec_path == manifest.source_exec_path
            && manifest.source_exec_path == manifest.live_exec_path,
        "expected/source/live executable paths must be identical after canonicalization"
    );
    ensure!(
        manifest.expected_exec_sha256 == manifest.source_exec_sha256
            && manifest.source_exec_sha256 == manifest.live_exec_sha256,
        "expected/source/live executable SHA-256 values must be identical"
    );
    ensure!(
        manifest.harness_worktree_clean,
        "capture harness worktree must be clean"
    );
    ensure!(
        manifest.worktree_state_algorithm == "git-head-path-mode-content-sha256-v1",
        "unsupported worktree_state_algorithm {:?}",
        manifest.worktree_state_algorithm
    );
    ensure!(manifest.pm2_entry_pid != 0, "pm2_entry_pid must be nonzero");
    ensure!(
        manifest.pm2_entry_starttime != 0,
        "pm2_entry_starttime must be nonzero"
    );
    ensure!(
        manifest.listener_runtime_pid != 0,
        "listener_runtime_pid must be nonzero"
    );
    ensure!(
        manifest.listener_runtime_starttime != 0,
        "listener_runtime_starttime must be nonzero"
    );
    ensure!(
        manifest.listener_relationship_verified,
        "PM2 entry/listener self-or-descendant relationship was not verified"
    );
    ensure!(
        manifest.effective_config_algorithm == "capture-relevant-redacted-v1",
        "unsupported effective_config_algorithm {:?}",
        manifest.effective_config_algorithm
    );
    ensure!(
        manifest.runtime_cleanup_verified,
        "runtime_cleanup_verified must be true"
    );
    ensure!(
        manifest.normal_runtime_restored,
        "normal_runtime_restored must be true"
    );
    if require_pinned_exec {
        ensure!(
            manifest.executable_pin_enforced,
            "required evidence must set executable_pin_enforced=true"
        );
    }
    match flow {
        "loot-single-item-claim" => validate_canonical_loot_identity(manifest)?,
        "loot-two-session-atomic-race" => {
            validate_canonical_loot_race_identity(manifest)?;
        }
        "vendor-extended-cost-purchase" => validate_canonical_vendor_identity(manifest)?,
        "detour-chase-around-obstacle" => validate_canonical_detour_identity(manifest)?,
        "creature-spell-casting" => validate_canonical_creature_spell_identity(manifest)?,
        _ => {}
    }

    match side {
        RawSide::Cpp => {
            if flow == "detour-chase-around-obstacle" {
                ensure!(
                    !manifest.source_worktree_dirty,
                    "detour C++ source worktree must be clean"
                );
                ensure!(
                    manifest.source_exec_revision.as_deref()
                        == Some(manifest.source_repo_head.as_str()),
                    "detour C++ embedded executable revision must equal source_repo_head"
                );
            } else if flow == "creature-spell-casting" {
                ensure!(
                    !manifest.source_worktree_dirty,
                    "creature-spell-casting C++ source worktree must be clean"
                );
                ensure!(
                    manifest.source_exec_revision.as_deref()
                        == Some(manifest.source_repo_head.as_str()),
                    "creature-spell-casting C++ embedded executable revision must equal source_repo_head"
                );
                validate_canonical_creature_spell_source_derivation(manifest)?;
            } else {
                ensure!(
                    manifest.source_derivation.is_none(),
                    "C++ source_derivation is reserved for creature-spell-casting"
                );
            }
            ensure!(
                manifest.artifact.path == "cpp.pkt",
                "C++ artifact path must be cpp.pkt"
            );
            ensure!(
                manifest.artifact.size.is_some(),
                "C++ artifact size is missing"
            );
            validate_sha256(
                manifest.artifact.sha256.as_deref().unwrap_or_default(),
                "C++ artifact sha256",
            )?;
            ensure!(
                manifest.artifact.packet_count.is_none() && manifest.artifact.tree_sha256.is_none(),
                "C++ artifact contains Rust-only fields"
            );
        }
        RawSide::Rust => {
            ensure!(
                manifest.source_derivation.is_none(),
                "Rust raw manifests must not contain C++ source_derivation evidence"
            );
            ensure!(
                manifest.harness_repo_head == manifest.source_repo_head,
                "Rust harness/source repository HEAD values must match"
            );
            if matches!(
                flow,
                "detour-chase-around-obstacle" | "creature-spell-casting"
            ) {
                ensure!(
                    manifest.source_exec_revision.as_deref()
                        == Some(manifest.source_repo_head.as_str()),
                    "{flow} Rust embedded executable revision must equal source_repo_head"
                );
            }
            ensure!(
                !manifest.source_worktree_dirty
                    && manifest.harness_worktree_state_sha256
                        == manifest.source_worktree_state_sha256,
                "Rust harness/source worktree must be the same clean state"
            );
            ensure!(
                manifest.artifact.path == "rust",
                "Rust artifact path must be rust"
            );
            ensure!(
                manifest.artifact.packet_count.is_some(),
                "Rust artifact packet_count is missing"
            );
            validate_sha256(
                manifest.artifact.tree_sha256.as_deref().unwrap_or_default(),
                "Rust artifact tree_sha256",
            )?;
            ensure!(
                manifest.artifact.size.is_none() && manifest.artifact.sha256.is_none(),
                "Rust artifact contains C++-only fields"
            );
        }
    }
    Ok(())
}

fn validate_source_derivation_schema(derivation: &SourceDerivationEvidence) -> Result<()> {
    ensure!(
        !derivation.contract.is_empty(),
        "source_derivation.contract must not be empty"
    );
    ensure!(
        !derivation.remote_url.is_empty()
            && !derivation.remote_url.contains(['\r', '\n'])
            && derivation.remote_url.starts_with("https://"),
        "source_derivation.remote_url must be one HTTPS line"
    );
    ensure!(
        derivation.remote_ref.starts_with("refs/remotes/")
            && !derivation.remote_ref.contains(['\r', '\n'])
            && !derivation.remote_ref.contains(".."),
        "source_derivation.remote_ref must be one canonical remote-tracking ref"
    );
    for (label, oid) in [
        ("base_head", derivation.base_head.as_str()),
        ("base_tree", derivation.base_tree.as_str()),
        ("patched_head", derivation.patched_head.as_str()),
        ("patched_tree", derivation.patched_tree.as_str()),
    ] {
        ensure!(
            valid_git_oid(oid),
            "source_derivation.{label} must be a lowercase 40- or 64-hex object id"
        );
    }
    ensure!(
        derivation.base_head != derivation.patched_head,
        "source_derivation base and patched commits must differ"
    );
    ensure!(
        is_canonical_relative_path(&derivation.patch_path),
        "source_derivation.patch_path must be one canonical repository-relative path"
    );
    validate_sha256(&derivation.patch_sha256, "source_derivation.patch_sha256")?;
    ensure!(
        !derivation.changed_paths.is_empty()
            && derivation
                .changed_paths
                .iter()
                .all(|path| is_canonical_relative_path(path))
            && derivation
                .changed_paths
                .windows(2)
                .all(|pair| pair[0] < pair[1]),
        "source_derivation.changed_paths must be nonempty, canonical, sorted, and unique"
    );
    Ok(())
}

fn is_canonical_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.ends_with('/')
        && !path.contains(['\r', '\n'])
        && path.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
}

fn validate_canonical_creature_spell_source_derivation(
    manifest: &RawCaptureManifest,
) -> Result<()> {
    let derivation = manifest
        .source_derivation
        .as_ref()
        .context("creature-spell-casting C++ requires source_derivation evidence")?;
    ensure!(
        derivation.contract == CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT
            && derivation.remote_url == CREATURE_SPELL_CPP_REMOTE_URL
            && derivation.remote_ref == CREATURE_SPELL_CPP_REMOTE_REF
            && derivation.base_head == CREATURE_SPELL_CPP_BASE_HEAD
            && derivation.base_tree == CREATURE_SPELL_CPP_BASE_TREE
            && derivation.patched_head == CREATURE_SPELL_CPP_PATCHED_HEAD
            && derivation.patched_tree == CREATURE_SPELL_CPP_PATCHED_TREE
            && derivation.patch_path == CREATURE_SPELL_CPP_PATCH_PATH
            && derivation.patch_sha256 == CREATURE_SPELL_CPP_PATCH_SHA256
            && derivation.changed_paths == [CREATURE_SPELL_CPP_CHANGED_PATH],
        "creature-spell-casting C++ source_derivation differs from the reviewed canonical-base patch"
    );
    ensure!(
        manifest.source_repo_head == derivation.patched_head
            && manifest.source_exec_revision.as_deref() == Some(derivation.patched_head.as_str()),
        "creature-spell-casting C++ source/binary revision is not the reviewed patched HEAD"
    );
    Ok(())
}

fn validate_canonical_loot_identity(manifest: &RawCaptureManifest) -> Result<()> {
    let fixture = manifest
        .fixture_guard
        .as_ref()
        .context("loot-single-item-claim requires fixture_guard evidence")?;
    ensure!(fixture.enabled, "fixture_guard.enabled must be true");
    ensure!(
        fixture.contract == "loot-single-item-claim-fixture-v1",
        "unexpected fixture_guard contract"
    );
    ensure!(
        fixture.account == "TESTBOT2@bot.local"
            && fixture.account_id == 9
            && fixture.character_guid == 15
            && fixture.peer_account == "TESTBOT3@bot.local"
            && fixture.peer_account_id == 10
            && fixture.peer_character_guid == 16,
        "fixture_guard bot identity is not the canonical TESTBOT2/TESTBOT3 fixture"
    );
    ensure!(
        fixture.creature_entry == Some(21_779)
            && fixture.creature_spawn_guid == Some(1_117)
            && fixture.gameobject_entry.is_none()
            && fixture.gameobject_spawn_guid.is_none()
            && fixture.item_entry == 30_712,
        "fixture_guard world/item identity is not the canonical Doctor Maleficus fixture"
    );
    ensure!(
        fixture.cleanup_verified,
        "fixture_guard cleanup was not verified"
    );

    let bot = manifest
        .bot_report
        .as_ref()
        .context("loot-single-item-claim requires bot_report evidence")?;
    ensure!(
        bot.contract == "wow-test-bot-loot-item-capture-report-v1",
        "unexpected bot_report contract"
    );
    ensure!(bot.report_validated, "bot_report was not validated");
    ensure!(
        bot.account == fixture.account
            && bot.account_id == fixture.account_id
            && bot.character_guid == fixture.character_guid,
        "bot_report identity does not match fixture_guard identity"
    );
    Ok(())
}

fn validate_canonical_loot_race_identity(manifest: &RawCaptureManifest) -> Result<()> {
    let fixture = manifest
        .fixture_guard
        .as_ref()
        .context("loot-two-session-atomic-race requires fixture_guard evidence")?;
    ensure!(fixture.enabled, "fixture_guard.enabled must be true");
    ensure!(
        fixture.contract == "loot-two-session-atomic-race-fixture-v1",
        "unexpected fixture_guard contract"
    );
    ensure!(
        fixture.account == "TESTBOT2@bot.local"
            && fixture.account_id == 9
            && fixture.character_guid == 15
            && fixture.peer_account == "TESTBOT3@bot.local"
            && fixture.peer_account_id == 10
            && fixture.peer_character_guid == 16,
        "fixture_guard bot identity is not the canonical TESTBOT2/TESTBOT3 fixture"
    );
    ensure!(
        fixture.creature_entry.is_none()
            && fixture.creature_spawn_guid.is_none()
            && fixture.gameobject_entry == Some(2_846)
            && fixture.gameobject_spawn_guid == Some(9_106_001)
            && fixture.item_entry == 38,
        "fixture_guard world/item identity is not the canonical shared-chest race fixture"
    );
    ensure!(
        fixture.cleanup_verified,
        "fixture_guard cleanup was not verified"
    );

    let bot = manifest
        .bot_report
        .as_ref()
        .context("loot-two-session-atomic-race requires bot_report evidence")?;
    ensure!(
        bot.contract == "wow-test-bot-loot-two-session-atomic-race-report-v1",
        "unexpected bot_report contract"
    );
    ensure!(bot.report_validated, "bot_report was not validated");
    ensure!(
        bot.account == fixture.account
            && bot.account_id == fixture.account_id
            && bot.character_guid == fixture.character_guid,
        "bot_report identity does not match fixture_guard identity"
    );
    Ok(())
}

fn validate_canonical_vendor_identity(manifest: &RawCaptureManifest) -> Result<()> {
    ensure!(
        manifest.fixture_guard.is_none(),
        "vendor-extended-cost-purchase uses the bot-owned fixture and must not claim a wrapper fixture_guard"
    );
    let bot = manifest
        .bot_report
        .as_ref()
        .context("vendor-extended-cost-purchase requires bot_report evidence")?;
    ensure!(
        bot.contract == "wow-test-bot-vendor-extended-cost-purchase-report-v1",
        "unexpected bot_report contract"
    );
    ensure!(bot.report_validated, "bot_report was not validated");
    ensure!(
        bot.account == "TESTBOT2@bot.local" && bot.account_id == 9 && bot.character_guid == 15,
        "vendor bot report manifest identity is not canonical TESTBOT2"
    );
    Ok(())
}

fn validate_canonical_detour_identity(manifest: &RawCaptureManifest) -> Result<()> {
    let fixture = manifest
        .fixture_guard
        .as_ref()
        .context("detour-chase-around-obstacle requires fixture_guard evidence")?;
    ensure!(fixture.enabled, "fixture_guard.enabled must be true");
    ensure!(
        fixture.contract == "detour-chase-around-obstacle-shell-fixture-v1",
        "unexpected fixture_guard contract"
    );
    ensure!(
        fixture.account == "TESTBOT2@bot.local"
            && fixture.account_id == 9
            && fixture.character_guid == 15
            && fixture.character_account_id == Some(9)
            && fixture.peer_account.is_empty()
            && fixture.peer_account_id == 0
            && fixture.peer_character_guid == 0,
        "fixture_guard subject is not the canonical TESTBOT2 detour fixture"
    );
    ensure!(
        fixture.creature_entry == Some(15_271)
            && fixture.creature_spawn_guid == Some(9_102_401)
            && fixture.gameobject_entry.is_none()
            && fixture.gameobject_spawn_guid.is_none()
            && fixture.item_entry == 0,
        "fixture_guard world identity is not the canonical detour creature"
    );
    ensure!(
        fixture.cleanup_verified,
        "fixture_guard cleanup was not verified"
    );
    ensure!(
        fixture.private_data_dir_removed_before_normal_runtime == Some(true),
        "private detour DataDir was not removed before normal runtime restoration"
    );

    let normal_data_dir = fixture
        .normal_data_dir
        .as_deref()
        .context("fixture_guard.normal_data_dir is missing")?;
    let private_data_dir = fixture
        .private_data_dir
        .as_deref()
        .context("fixture_guard.private_data_dir is missing")?;
    let fixture_manifest_path = fixture
        .fixture_manifest_path
        .as_deref()
        .context("fixture_guard.fixture_manifest_path is missing")?;
    ensure!(
        Path::new(normal_data_dir).is_absolute()
            && Path::new(private_data_dir).is_absolute()
            && Path::new(fixture_manifest_path).is_absolute()
            && normal_data_dir != private_data_dir,
        "detour fixture paths must be distinct absolute paths"
    );
    ensure!(
        Path::new(fixture_manifest_path).ends_with(
            "crates/capture-diff/flows/detour-chase-around-obstacle/fixture/fixture.json"
        ),
        "fixture_guard manifest path is not the committed detour fixture"
    );
    ensure!(
        fixture.fixture_manifest_sha256.as_deref() == Some(DETOUR_FIXTURE_MANIFEST_SHA256),
        "fixture_guard.fixture_manifest_sha256 is not the exact reviewed detour manifest"
    );
    validate_sha256(
        fixture
            .journal_sha256
            .as_deref()
            .context("fixture_guard.journal_sha256 is missing")?,
        "fixture_guard.journal_sha256",
    )?;
    validate_sha256(
        fixture
            .database_snapshot_sha256
            .as_deref()
            .context("fixture_guard.database_snapshot_sha256 is missing")?,
        "fixture_guard.database_snapshot_sha256",
    )?;

    let synthetic_mmaps = fixture
        .synthetic_mmaps
        .as_deref()
        .context("fixture_guard.synthetic_mmaps is missing")?;
    ensure!(
        synthetic_mmaps
            == [
                SyntheticMmapEvidence {
                    path: "mmaps/0001.mmap".to_string(),
                    size: 28,
                    sha256: "3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a"
                        .to_string(),
                },
                SyntheticMmapEvidence {
                    path: "mmaps/00015026.mmtile".to_string(),
                    size: 1_496,
                    sha256: "693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47"
                        .to_string(),
                },
            ],
        "fixture_guard synthetic MMaps differ from the pinned obstacle assets"
    );
    let linked_read_only_data = fixture
        .linked_read_only_data
        .as_deref()
        .context("fixture_guard.linked_read_only_data is missing")?;
    let expected_links =
        ["dbc", "gt", "maps", "vmaps", "cameras"].map(|name| LinkedReadOnlyDataEvidence {
            name: name.to_string(),
            target_path: Path::new(normal_data_dir)
                .join(name)
                .to_string_lossy()
                .into_owned(),
        });
    ensure!(
        linked_read_only_data == expected_links,
        "fixture_guard read-only DataDir links differ from the normal runtime data"
    );

    let bot = manifest
        .bot_report
        .as_ref()
        .context("detour-chase-around-obstacle requires bot_report evidence")?;
    ensure!(
        bot.contract == "wow-test-bot-detour-chase-capture-report-v1",
        "unexpected bot_report contract"
    );
    ensure!(bot.report_validated, "bot_report was not validated");
    ensure!(
        bot.account == fixture.account
            && bot.account_id == fixture.account_id
            && bot.character_guid == fixture.character_guid,
        "bot_report identity does not match fixture_guard identity"
    );
    Ok(())
}

fn validate_canonical_creature_spell_identity(manifest: &RawCaptureManifest) -> Result<()> {
    let fixture = manifest
        .fixture_guard
        .as_ref()
        .context("creature-spell-casting requires fixture_guard evidence")?;
    ensure!(fixture.enabled, "fixture_guard.enabled must be true");
    ensure!(
        fixture.contract == "creature-spell-casting-shell-fixture-v1",
        "unexpected creature-spell fixture_guard contract"
    );
    ensure!(
        fixture.account == "TESTBOT2@bot.local"
            && fixture.account_id == 9
            && fixture.character_guid == 15
            && fixture.peer_account.is_empty()
            && fixture.peer_account_id == 0
            && fixture.peer_character_guid == 0
            && fixture.character_account_id.is_none(),
        "creature-spell fixture_guard is not the canonical TESTBOT2 identity"
    );
    ensure!(
        fixture.creature_entry == Some(22_378)
            && fixture.creature_spawn_guid == Some(78_686)
            && fixture.gameobject_entry.is_none()
            && fixture.gameobject_spawn_guid.is_none()
            && fixture.item_entry == 0,
        "creature-spell fixture_guard world identity is not Cabal Interrogator 22378/78686"
    );
    ensure!(
        fixture.normal_data_dir.is_none()
            && fixture.private_data_dir.is_none()
            && fixture
                .private_data_dir_removed_before_normal_runtime
                .is_none()
            && fixture.synthetic_mmaps.is_none()
            && fixture.linked_read_only_data.is_none(),
        "creature-spell fixture_guard contains unrelated filesystem evidence"
    );
    let fixture_manifest_path = fixture
        .fixture_manifest_path
        .as_deref()
        .context("creature-spell fixture_guard.fixture_manifest_path is missing")?;
    ensure!(
        Path::new(fixture_manifest_path).is_absolute()
            && Path::new(fixture_manifest_path)
                .ends_with("crates/capture-diff/flows/creature-spell-casting/fixture/fixture.json"),
        "creature-spell fixture_guard manifest path is not the committed fixture"
    );
    ensure!(
        fixture.fixture_manifest_sha256.as_deref() == Some(CREATURE_SPELL_FIXTURE_MANIFEST_SHA256),
        "creature-spell fixture_guard.fixture_manifest_sha256 is not the exact reviewed manifest"
    );
    validate_sha256(
        fixture
            .journal_sha256
            .as_deref()
            .context("creature-spell fixture_guard.journal_sha256 is missing")?,
        "creature-spell fixture_guard.journal_sha256",
    )?;
    validate_sha256(
        fixture
            .database_snapshot_sha256
            .as_deref()
            .context("creature-spell fixture_guard.database_snapshot_sha256 is missing")?,
        "creature-spell fixture_guard.database_snapshot_sha256",
    )?;
    ensure!(
        fixture.cleanup_verified,
        "creature-spell fixture_guard cleanup was not verified"
    );
    let bot = manifest
        .bot_report
        .as_ref()
        .context("creature-spell-casting requires bot_report evidence")?;
    ensure!(
        bot.contract == "wow-test-bot-creature-spell-casting-report-v1",
        "unexpected creature-spell bot_report contract"
    );
    ensure!(bot.report_validated, "bot_report was not validated");
    ensure!(
        bot.account == fixture.account
            && bot.account_id == fixture.account_id
            && bot.character_guid == fixture.character_guid,
        "creature-spell bot_report identity does not match fixture_guard identity"
    );
    Ok(())
}

fn detour_fixture_shared_identity_matches(
    cpp: &FixtureGuardEvidence,
    rust: &FixtureGuardEvidence,
) -> bool {
    cpp.enabled == rust.enabled
        && cpp.contract == rust.contract
        && cpp.account == rust.account
        && cpp.account_id == rust.account_id
        && cpp.character_guid == rust.character_guid
        && cpp.peer_account == rust.peer_account
        && cpp.peer_account_id == rust.peer_account_id
        && cpp.peer_character_guid == rust.peer_character_guid
        && cpp.creature_entry == rust.creature_entry
        && cpp.creature_spawn_guid == rust.creature_spawn_guid
        && cpp.gameobject_entry == rust.gameobject_entry
        && cpp.gameobject_spawn_guid == rust.gameobject_spawn_guid
        && cpp.item_entry == rust.item_entry
        && cpp.character_account_id == rust.character_account_id
        && cpp.normal_data_dir == rust.normal_data_dir
        && cpp.private_data_dir_removed_before_normal_runtime
            == rust.private_data_dir_removed_before_normal_runtime
        && cpp.fixture_manifest_path == rust.fixture_manifest_path
        && cpp.fixture_manifest_sha256 == rust.fixture_manifest_sha256
        && cpp.synthetic_mmaps == rust.synthetic_mmaps
        && cpp.linked_read_only_data == rust.linked_read_only_data
        && cpp.database_snapshot_sha256 == rust.database_snapshot_sha256
        && cpp.cleanup_verified == rust.cleanup_verified
}

fn creature_spell_fixture_shared_identity_matches(
    cpp: &FixtureGuardEvidence,
    rust: &FixtureGuardEvidence,
) -> bool {
    cpp.enabled == rust.enabled
        && cpp.contract == rust.contract
        && cpp.account == rust.account
        && cpp.account_id == rust.account_id
        && cpp.character_guid == rust.character_guid
        && cpp.peer_account == rust.peer_account
        && cpp.peer_account_id == rust.peer_account_id
        && cpp.peer_character_guid == rust.peer_character_guid
        && cpp.creature_entry == rust.creature_entry
        && cpp.creature_spawn_guid == rust.creature_spawn_guid
        && cpp.gameobject_entry == rust.gameobject_entry
        && cpp.gameobject_spawn_guid == rust.gameobject_spawn_guid
        && cpp.item_entry == rust.item_entry
        && cpp.character_account_id == rust.character_account_id
        && cpp.fixture_manifest_path == rust.fixture_manifest_path
        && cpp.fixture_manifest_sha256 == rust.fixture_manifest_sha256
        && cpp.database_snapshot_sha256 == rust.database_snapshot_sha256
        && cpp.cleanup_verified == rust.cleanup_verified
}

fn validate_cross_side_identity(
    flow: &str,
    cpp: &RawCaptureManifest,
    rust: &RawCaptureManifest,
) -> Result<()> {
    ensure!(
        cpp.harness_repo_head == rust.harness_repo_head,
        "C++ and Rust captures were produced from different harness HEAD values"
    );
    ensure!(
        cpp.harness_worktree_state_sha256 == rust.harness_worktree_state_sha256,
        "C++ and Rust captures were produced from different harness worktree digests"
    );
    ensure!(
        cpp.worktree_state_algorithm == rust.worktree_state_algorithm,
        "C++ and Rust harness digest algorithms differ"
    );
    if flow == "creature-spell-casting" {
        ensure!(
            cpp.fixture_guard
                .as_ref()
                .zip(rust.fixture_guard.as_ref())
                .is_some_and(|(cpp, rust)| {
                    creature_spell_fixture_shared_identity_matches(cpp, rust)
                }),
            "C++ and Rust creature-spell fixture identities differ"
        );
        let cpp_bot = cpp
            .bot_report
            .as_ref()
            .context("C++ creature-spell bot report missing")?;
        let rust_bot = rust
            .bot_report
            .as_ref()
            .context("Rust creature-spell bot report missing")?;
        ensure!(
            cpp_bot.contract == rust_bot.contract
                && cpp_bot.exec_path == rust_bot.exec_path
                && cpp_bot.exec_sha256 == rust_bot.exec_sha256
                && cpp_bot.account == rust_bot.account
                && cpp_bot.account_id == rust_bot.account_id
                && cpp_bot.character_guid == rust_bot.character_guid,
            "C++ and Rust creature-spell captures used different canonical bot identities"
        );
    } else if matches!(
        flow,
        "loot-single-item-claim"
            | "loot-two-session-atomic-race"
            | "vendor-extended-cost-purchase"
            | "detour-chase-around-obstacle"
    ) {
        if flow == "detour-chase-around-obstacle" {
            ensure!(
                cpp.fixture_guard
                    .as_ref()
                    .zip(rust.fixture_guard.as_ref())
                    .is_some_and(|(cpp, rust)| detour_fixture_shared_identity_matches(cpp, rust)),
                "C++ and Rust detour fixture identities differ"
            );
        } else if flow != "vendor-extended-cost-purchase" {
            ensure!(
                cpp.fixture_guard == rust.fixture_guard,
                "C++ and Rust guarded-loot fixture identities differ"
            );
        }
        let cpp_bot = cpp.bot_report.as_ref().context("C++ bot report missing")?;
        let rust_bot = rust
            .bot_report
            .as_ref()
            .context("Rust bot report missing")?;
        ensure!(
            cpp_bot.contract == rust_bot.contract
                && cpp_bot.exec_path == rust_bot.exec_path
                && cpp_bot.exec_sha256 == rust_bot.exec_sha256
                && cpp_bot.account == rust_bot.account
                && cpp_bot.account_id == rust_bot.account_id
                && cpp_bot.character_guid == rust_bot.character_guid,
            "C++ and Rust captures used different canonical bot identities"
        );
    }
    Ok(())
}

fn validate_bot_report_artifact(manifest: &RawCaptureManifest) -> Result<Option<Vec<u8>>> {
    let Some(evidence) = &manifest.bot_report else {
        return Ok(None);
    };
    let bytes = read_regular_file(Path::new(&evidence.report_path))
        .with_context(|| format!("reading bot report evidence {}", evidence.report_path))?;
    ensure!(
        sha256_bytes(&bytes) == evidence.report_sha256,
        "bot report SHA-256 does not match its manifest"
    );
    validate_bot_report_json(&bytes, evidence)?;
    Ok(Some(bytes))
}

fn validate_bot_report_json(bytes: &[u8], evidence: &BotReportEvidence) -> Result<()> {
    let report: serde_json::Value =
        serde_json::from_slice(bytes).context("parsing bot report evidence")?;
    match evidence.contract.as_str() {
        "wow-test-bot-loot-item-capture-report-v1" => {
            validate_loot_item_bot_report_json(&report, evidence)
        }
        "wow-test-bot-loot-two-session-atomic-race-report-v1" => {
            validate_loot_race_bot_report_json(&report, evidence)
        }
        "wow-test-bot-vendor-extended-cost-purchase-report-v1" => {
            validate_vendor_bot_report_json(&report, evidence)
        }
        "wow-test-bot-detour-chase-capture-report-v1" => {
            validate_detour_chase_bot_report_json(&report, evidence)
        }
        "wow-test-bot-creature-spell-casting-report-v1" => {
            validate_creature_spell_bot_report_json(&report, evidence)
        }
        contract => bail!("unsupported bot report contract {contract:?}"),
    }
}

fn validate_creature_spell_bot_report_json(
    report: &serde_json::Value,
    evidence: &BotReportEvidence,
) -> Result<()> {
    let results = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .context("bot report results must be an array")?;
    ensure!(
        report
            .get("creature_spell_capture")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && report
                .get("detour_chase_capture")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && report
                .get("loot_item_capture")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && report
                .get("loot_race_smoke")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && results.len() == 1,
        "bot report is not one isolated creature-spell capture"
    );
    let result = &results[0];
    let string = |key: &str| result.get(key).and_then(serde_json::Value::as_str);
    let u64_value = |key: &str| result.get(key).and_then(serde_json::Value::as_u64);
    let boolean = |key: &str| result.get(key).and_then(serde_json::Value::as_bool);
    ensure!(
        string("account") == Some(evidence.account.as_str())
            && u64_value("account_id") == Some(u64::from(evidence.account_id))
            && u64_value("character_guid") == Some(evidence.character_guid),
        "bot report subject does not match manifest identity"
    );
    for (field, label) in [
        (
            "creature_spell_heartbeat_sha256",
            "creature-spell heartbeat SHA-256",
        ),
        (
            "creature_spell_start_body_sha256",
            "creature-spell START body SHA-256",
        ),
        (
            "creature_spell_go_body_sha256",
            "creature-spell GO body SHA-256",
        ),
    ] {
        validate_sha256(
            string(field).with_context(|| format!("{label} is missing"))?,
            label,
        )?;
    }
    ensure!(
        boolean("world_auth") == Some(true)
            && boolean("enum_characters") == Some(true)
            && boolean("player_login_verified") == Some(true)
            && boolean("creature_spell_capture") == Some(true)
            && boolean("creature_spell_capture_passed") == Some(true)
            && string("creature_spell_fixture_manifest_sha256")
                == Some(CREATURE_SPELL_FIXTURE_MANIFEST_SHA256)
            && u64_value("creature_spell_target_entry") == Some(22_378)
            && u64_value("creature_spell_target_spawn_guid") == Some(78_686)
            && u64_value("creature_spell_target_runtime_counter")
                .is_some_and(|counter| counter > 0)
            && boolean("creature_spell_target_discovered") == Some(true)
            && boolean("creature_spell_heartbeat_sent") == Some(true)
            && u64_value("creature_spell_start_opcode") == Some(u64::from(SMSG_SPELL_START))
            && u64_value("creature_spell_start_body_bytes").is_some_and(|bytes| bytes > 0)
            && u64_value("creature_spell_go_opcode") == Some(u64::from(SMSG_SPELL_GO))
            && u64_value("creature_spell_go_body_bytes").is_some_and(|bytes| bytes > 0)
            && u64_value("creature_spell_cast_id_low").is_some_and(|value| value > 0)
            && u64_value("creature_spell_cast_id_high").is_some_and(|value| value > 0)
            && u64_value("creature_spell_caster_guid_low").is_some_and(|value| value > 0)
            && u64_value("creature_spell_caster_guid_low")
                == u64_value("creature_spell_target_runtime_counter")
            && u64_value("creature_spell_caster_guid_high").is_some_and(|value| value > 0)
            && u64_value("creature_spell_victim_guid_low") == Some(15)
            && u64_value("creature_spell_victim_guid_high")
                == Some(CREATURE_SPELL_PLAYER_GUID_HIGH)
            && u64_value("creature_spell_spell_id") == Some(15_691)
            && u64_value("creature_spell_start_cast_flags") == Some(2)
            && u64_value("creature_spell_go_cast_flags") == Some(0x100)
            && u64_value("creature_spell_cast_flags_ex") == Some(0)
            && u64_value("creature_spell_go_hit_target_count") == Some(1)
            && u64_value("creature_spell_go_miss_target_count") == Some(0)
            && boolean("creature_spell_full_combat_log") == Some(false)
            && boolean("creature_spell_advanced_logging_sent") == Some(false)
            && boolean("creature_spell_adjacent_start_go") == Some(true)
            && boolean("creature_spell_disconnect_confirmed") == Some(true)
            && boolean("creature_spell_logout_confirmed") == Some(false)
            && result
                .get("creature_spell_failure")
                .is_some_and(serde_json::Value::is_null),
        "bot report does not prove the canonical successful creature-spell window"
    );
    Ok(())
}

fn validate_detour_chase_bot_report_json(
    report: &serde_json::Value,
    evidence: &BotReportEvidence,
) -> Result<()> {
    let results = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .context("bot report results must be an array")?;
    ensure!(
        report
            .get("detour_chase_capture")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && report
                .get("loot_item_capture")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && report
                .get("loot_race_smoke")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && report
                .get("vendor_smoke")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && results.len() == 1,
        "bot report is not one isolated detour-chase capture"
    );
    let result = &results[0];
    let string = |key: &str| result.get(key).and_then(serde_json::Value::as_str);
    let u64_value = |key: &str| result.get(key).and_then(serde_json::Value::as_u64);
    let boolean = |key: &str| result.get(key).and_then(serde_json::Value::as_bool);
    ensure!(
        string("account") == Some(evidence.account.as_str())
            && u64_value("account_id") == Some(u64::from(evidence.account_id))
            && u64_value("character_guid") == Some(evidence.character_guid),
        "bot report subject does not match manifest identity"
    );
    let heartbeat_sha256 = string("detour_chase_heartbeat_sha256")
        .context("detour chase heartbeat SHA-256 is missing")?;
    let monster_move_sha256 = string("detour_chase_monster_move_sha256")
        .context("detour chase MonsterMove SHA-256 is missing")?;
    validate_sha256(heartbeat_sha256, "detour chase heartbeat SHA-256")?;
    validate_sha256(monster_move_sha256, "detour chase MonsterMove SHA-256")?;
    ensure!(
        boolean("world_auth") == Some(true)
            && boolean("enum_characters") == Some(true)
            && boolean("player_login_verified") == Some(true)
            && boolean("detour_chase_capture") == Some(true)
            && boolean("detour_chase_capture_passed") == Some(true)
            && u64_value("detour_chase_target_entry") == Some(15_271)
            && u64_value("detour_chase_target_spawn_guid") == Some(9_102_401)
            && u64_value("detour_chase_target_runtime_counter").is_some_and(|counter| counter > 0)
            && boolean("detour_chase_target_discovered") == Some(true)
            && boolean("detour_chase_active_mover_ack_sent") == Some(true)
            && boolean("detour_chase_attack_start_confirmed") == Some(true)
            && boolean("detour_chase_first_swing_confirmed") == Some(true)
            && u64_value("detour_chase_prewindow_target_moves") == Some(0)
            && boolean("detour_chase_heartbeat_sent") == Some(true)
            && u64_value("detour_chase_window_target_moves") == Some(1)
            && u64_value("detour_chase_monster_move_bytes").is_some_and(|bytes| bytes > 0)
            && u64_value("detour_chase_ping_serial") == Some(u64::from(ISSUE_24_PING_FENCE_SERIAL))
            && boolean("detour_chase_pong_confirmed") == Some(true)
            && boolean("detour_chase_logout_confirmed") == Some(true)
            && result
                .get("detour_chase_failure")
                .is_some_and(serde_json::Value::is_null),
        "bot report does not prove the canonical isolated detour-chase window"
    );
    Ok(())
}

fn validate_vendor_bot_report_json(
    report: &serde_json::Value,
    evidence: &BotReportEvidence,
) -> Result<()> {
    let results = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .context("bot report results must be an array")?;
    ensure!(
        report
            .get("vendor_smoke")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && report
                .get("loot_item_capture")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && report
                .get("loot_race_smoke")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && results.len() == 1,
        "bot report is not a single-session vendor capture"
    );
    let result = &results[0];
    let string = |key: &str| result.get(key).and_then(serde_json::Value::as_str);
    let u64_value = |key: &str| result.get(key).and_then(serde_json::Value::as_u64);
    let boolean = |key: &str| result.get(key).and_then(serde_json::Value::as_bool);
    ensure!(
        string("account") == Some(evidence.account.as_str())
            && u64_value("account_id") == Some(u64::from(evidence.account_id))
            && u64_value("character_guid") == Some(evidence.character_guid),
        "bot report subject does not match manifest identity"
    );
    ensure!(
        boolean("world_auth") == Some(true)
            && boolean("enum_characters") == Some(true)
            && boolean("player_login_verified") == Some(true)
            && boolean("vendor_smoke") == Some(true)
            && boolean("vendor_smoke_passed") == Some(true)
            && u64_value("vendor_entry") == Some(18_525)
            && u64_value("vendor_spawn_guid") == Some(96_654)
            && u64_value("vendor_runtime_counter").is_some_and(|counter| counter > 0)
            && u64_value("vendor_item_entry") == Some(30_183)
            && u64_value("vendor_extended_cost") == Some(1_642)
            && u64_value("vendor_currency_id") == Some(42)
            && u64_value("vendor_currency_before") == Some(30)
            && u64_value("vendor_currency_after") == Some(15)
            && u64_value("vendor_item_total_after") == Some(1)
            && boolean("vendor_inventory_seen") == Some(true)
            && boolean("vendor_buy_succeeded_seen") == Some(true)
            && boolean("vendor_set_currency_seen") == Some(true)
            && boolean("vendor_item_push_seen") == Some(true)
            && boolean("vendor_relogin_verified") == Some(true)
            && result
                .get("vendor_failure")
                .is_some_and(serde_json::Value::is_null),
        "bot report does not prove the canonical successful vendor flow"
    );
    Ok(())
}

fn validate_loot_item_bot_report_json(
    report: &serde_json::Value,
    evidence: &BotReportEvidence,
) -> Result<()> {
    let results = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .context("bot report results must be an array")?;
    ensure!(
        report
            .get("loot_item_capture")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
            && report
                .get("loot_race_smoke")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            && results.len() == 1,
        "bot report is not a single-session loot-item capture"
    );
    let result = &results[0];
    let string = |key: &str| result.get(key).and_then(serde_json::Value::as_str);
    let u64_value = |key: &str| result.get(key).and_then(serde_json::Value::as_u64);
    let boolean = |key: &str| result.get(key).and_then(serde_json::Value::as_bool);
    ensure!(
        string("account") == Some(evidence.account.as_str())
            && u64_value("account_id") == Some(u64::from(evidence.account_id))
            && u64_value("character_guid") == Some(evidence.character_guid),
        "bot report subject does not match manifest identity"
    );
    ensure!(
        boolean("world_auth") == Some(true)
            && boolean("enum_characters") == Some(true)
            && boolean("player_login_verified") == Some(true)
            && boolean("loot_race_smoke") == Some(true)
            && boolean("loot_race_smoke_passed") == Some(true)
            && u64_value("loot_race_target_entry") == Some(21_779)
            && u64_value("loot_race_target_spawn_guid") == Some(1_117)
            && boolean("loot_race_target_discovered") == Some(true)
            && boolean("loot_race_loot_opened") == Some(true)
            && boolean("loot_race_item_push_seen") == Some(true)
            && boolean("loot_race_loot_removed_seen") == Some(true)
            && u64_value("loot_race_loot_coins") == Some(0)
            && boolean("loot_race_coin_removed_seen") == Some(false)
            && u64_value("loot_race_db_item_total") == Some(1)
            && u64_value("loot_race_db_money_delta") == Some(0)
            && boolean("loot_race_relog_verified") == Some(true)
            && result
                .get("loot_race_failure")
                .is_some_and(serde_json::Value::is_null),
        "bot report does not prove the canonical successful loot-item flow"
    );
    Ok(())
}

fn validate_loot_race_bot_report_json(
    report: &serde_json::Value,
    evidence: &BotReportEvidence,
) -> Result<()> {
    let results = report
        .get("results")
        .and_then(serde_json::Value::as_array)
        .context("bot report results must be an array")?;
    ensure!(
        report
            .get("loot_item_capture")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
            && report
                .get("loot_race_smoke")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            && results.len() == 2,
        "bot report is not a two-session loot race capture"
    );
    ensure!(
        evidence.account == "TESTBOT2@bot.local"
            && evidence.account_id == 9
            && evidence.character_guid == 15,
        "race bot report manifest identity is not canonical TESTBOT2"
    );

    let mut by_account = BTreeMap::new();
    let mut runtime_counters = Vec::with_capacity(2);
    let mut loot_list_ids = Vec::with_capacity(2);
    let mut item_pushes = Vec::with_capacity(2);
    let mut money_notifications = Vec::with_capacity(2);
    for result in results {
        let string = |key: &str| result.get(key).and_then(serde_json::Value::as_str);
        let u64_value = |key: &str| result.get(key).and_then(serde_json::Value::as_u64);
        let boolean = |key: &str| result.get(key).and_then(serde_json::Value::as_bool);
        let account = string("account").context("race result account must be a string")?;
        ensure!(
            by_account.insert(account, result).is_none(),
            "race bot report contains a duplicate account"
        );
        let (account_id, character_guid) = match account {
            "TESTBOT2@bot.local" => (9, 15),
            "TESTBOT3@bot.local" => (10, 16),
            _ => bail!("race bot report contains unexpected account {account:?}"),
        };
        ensure!(
            u64_value("account_id") == Some(account_id)
                && u64_value("character_guid") == Some(character_guid),
            "race bot report account identity does not match the canonical fixture"
        );
        ensure!(
            boolean("world_auth") == Some(true)
                && boolean("enum_characters") == Some(true)
                && boolean("player_login_verified") == Some(true)
                && boolean("loot_race_smoke") == Some(true)
                && boolean("loot_race_smoke_passed") == Some(true)
                && result
                    .get("loot_race_failure")
                    .is_some_and(serde_json::Value::is_null)
                && u64_value("loot_race_target_entry") == Some(2_846)
                && u64_value("loot_race_target_spawn_guid") == Some(9_106_001)
                && u64_value("loot_race_target_runtime_counter").is_some_and(|value| value > 0)
                && boolean("loot_race_party_confirmed") == Some(true)
                && boolean("loot_race_target_discovered") == Some(true)
                && boolean("loot_race_loot_opened") == Some(true)
                && u64_value("loot_race_loot_list_id").is_some_and(|value| value <= 255)
                && u64_value("loot_race_loot_coins") == Some(10)
                && boolean("loot_race_loot_removed_seen") == Some(true)
                && boolean("loot_race_coin_removed_seen") == Some(true)
                && u64_value("loot_race_db_item_total") == Some(1)
                && u64_value("loot_race_db_money_delta") == Some(10)
                && boolean("loot_race_relog_verified") == Some(true),
            "bot report does not prove the exact successful two-session loot race"
        );
        runtime_counters.push(u64_value("loot_race_target_runtime_counter").unwrap());
        loot_list_ids.push(u64_value("loot_race_loot_list_id").unwrap());
        item_pushes.push(
            boolean("loot_race_item_push_seen")
                .context("race result item-push observation must be boolean")?,
        );
        money_notifications.push(
            u64_value("loot_race_money_notify_amount")
                .context("race result money notification must be unsigned")?,
        );
    }
    ensure!(
        by_account.len() == 2
            && by_account.contains_key("TESTBOT2@bot.local")
            && by_account.contains_key("TESTBOT3@bot.local"),
        "race bot report does not contain the exact two canonical accounts"
    );
    runtime_counters.sort_unstable();
    runtime_counters.dedup();
    loot_list_ids.sort_unstable();
    loot_list_ids.dedup();
    item_pushes.sort_unstable();
    money_notifications.sort_unstable();
    ensure!(
        runtime_counters.len() == 1
            && loot_list_ids.len() == 1
            && item_pushes == [false, true]
            && money_notifications == [0, 10],
        "race bot report does not prove one shared target/list, one item winner, and 0/10 money fanout"
    );
    Ok(())
}

fn validate_cpp_artifact(manifest: &RawCaptureManifest, path: &Path) -> Result<()> {
    let bytes = read_regular_file(path)
        .with_context(|| format!("reading raw C++ capture {}", path.display()))?;
    let actual_size = u64::try_from(bytes.len()).context("C++ capture size does not fit u64")?;
    ensure!(
        manifest.artifact.size == Some(actual_size),
        "raw C++ capture size is {actual_size}, manifest declares {:?}",
        manifest.artifact.size
    );
    ensure!(
        manifest.artifact.sha256.as_deref() == Some(sha256_bytes(&bytes).as_str()),
        "raw C++ capture SHA-256 does not match its manifest"
    );
    Ok(())
}

fn validate_rust_artifact(
    manifest: &RawCaptureManifest,
    path: &Path,
    excluded_manifest: Option<&Path>,
) -> Result<()> {
    let digest = digest_tree(path, excluded_manifest)
        .with_context(|| format!("hashing raw Rust capture {}", path.display()))?;
    ensure!(
        manifest.artifact.tree_sha256.as_deref() == Some(digest.sha256.as_str()),
        "raw Rust capture tree SHA-256 does not match its manifest"
    );
    ensure!(
        manifest.artifact.packet_count == Some(digest.packet_count),
        "raw Rust capture contains {} packet(s), manifest declares {:?}",
        digest.packet_count,
        manifest.artifact.packet_count
    );
    Ok(())
}

/// Copy exact raw manifests and write a derived lineage completion marker into
/// an otherwise complete staging flow.
pub fn write_derived_lineage(
    flow: &str,
    flow_dir: &Path,
    raw: &ValidatedRawPair,
    selection: ImportSelection,
) -> Result<()> {
    let provenance_dir = flow_dir.join(RAW_PROVENANCE_DIR);
    fs::create_dir_all(&provenance_dir)
        .with_context(|| format!("creating {}", provenance_dir.display()))?;
    write_synced_file(
        &provenance_dir.join(CPP_RAW_MANIFEST_FILE),
        &raw.cpp.manifest_bytes,
    )?;
    write_synced_file(
        &provenance_dir.join(RUST_RAW_MANIFEST_FILE),
        &raw.rust.manifest_bytes,
    )?;
    if let Some(bytes) = &raw.cpp.bot_report_bytes {
        write_synced_file(&provenance_dir.join(CPP_BOT_REPORT_FILE), bytes)?;
    }
    if let Some(bytes) = &raw.rust.bot_report_bytes {
        write_synced_file(&provenance_dir.join(RUST_BOT_REPORT_FILE), bytes)?;
    }

    let lineage = build_lineage(flow, flow_dir, raw, selection)?;
    let bytes = serde_json::to_vec_pretty(&lineage).context("serializing derived lineage")?;
    atomic_write(&flow_dir.join(LINEAGE_FILE), &bytes)?;
    sync_directory(&provenance_dir)?;
    sync_directory(flow_dir)?;
    Ok(())
}

fn build_lineage(
    flow: &str,
    flow_dir: &Path,
    raw: &ValidatedRawPair,
    selection: ImportSelection,
) -> Result<DerivedLineage> {
    let cpp_output = file_lineage(flow_dir, "cpp.pkt")?;
    let expected_output = file_lineage(flow_dir, "expected-divergences.json")?;
    let rust_digest = digest_tree(&flow_dir.join("rust"), None)?;

    Ok(DerivedLineage {
        version: LINEAGE_VERSION,
        flow: flow.to_string(),
        completed: true,
        sources: SourcePairLineage {
            cpp: source_lineage(
                &raw.cpp,
                format!("{RAW_PROVENANCE_DIR}/{CPP_RAW_MANIFEST_FILE}"),
            )?,
            rust: source_lineage(
                &raw.rust,
                format!("{RAW_PROVENANCE_DIR}/{RUST_RAW_MANIFEST_FILE}"),
            )?,
        },
        selection,
        outputs: DerivedOutputs {
            cpp_pkt: cpp_output,
            rust: TreeLineage {
                path: "rust".to_string(),
                file_count: rust_digest.file_count,
                packet_count: rust_digest.packet_count,
                tree_sha256: rust_digest.sha256,
            },
            expected_divergences: expected_output,
        },
    })
}

fn source_lineage(raw: &ValidatedRawSide, manifest_path: String) -> Result<SourceLineage> {
    let (raw_artifact_sha256, raw_artifact_size, raw_packet_count) = match raw.manifest.side {
        RawSide::Cpp => (
            raw.manifest
                .artifact
                .sha256
                .clone()
                .context("validated C++ SHA missing")?,
            raw.manifest.artifact.size,
            None,
        ),
        RawSide::Rust => (
            raw.manifest
                .artifact
                .tree_sha256
                .clone()
                .context("validated Rust tree SHA missing")?,
            None,
            raw.manifest.artifact.packet_count,
        ),
    };
    Ok(SourceLineage {
        manifest_path,
        manifest_sha256: raw.manifest_sha256.clone(),
        raw_artifact_sha256,
        raw_artifact_size,
        raw_packet_count,
        harness_repo_head: raw.manifest.harness_repo_head.clone(),
        source_repo_head: raw.manifest.source_repo_head.clone(),
        source_derivation: raw.manifest.source_derivation.clone(),
        harness_worktree_clean: raw.manifest.harness_worktree_clean,
        harness_worktree_state_sha256: raw.manifest.harness_worktree_state_sha256.clone(),
        source_worktree_dirty: raw.manifest.source_worktree_dirty,
        source_worktree_state_sha256: raw.manifest.source_worktree_state_sha256.clone(),
        worktree_state_algorithm: raw.manifest.worktree_state_algorithm.clone(),
        expected_exec_path: raw.manifest.expected_exec_path.clone(),
        expected_exec_sha256: raw.manifest.expected_exec_sha256.clone(),
        source_exec_path: raw.manifest.source_exec_path.clone(),
        source_exec_sha256: raw.manifest.source_exec_sha256.clone(),
        live_exec_path: raw.manifest.live_exec_path.clone(),
        live_exec_sha256: raw.manifest.live_exec_sha256.clone(),
        executable_pin_enforced: raw.manifest.executable_pin_enforced,
        pm2_entry_pid: raw.manifest.pm2_entry_pid,
        pm2_exec_path: raw.manifest.pm2_exec_path.clone(),
        pm2_exec_sha256: raw.manifest.pm2_exec_sha256.clone(),
        listener_runtime_pid: raw.manifest.listener_runtime_pid,
        listener_relationship_verified: raw.manifest.listener_relationship_verified,
        restart_count: raw.manifest.restart_count,
        effective_config_path: raw.manifest.effective_config_path.clone(),
        effective_config_redacted_sha256: raw.manifest.effective_config_redacted_sha256.clone(),
        effective_config_algorithm: raw.manifest.effective_config_algorithm.clone(),
        pm2_entry_starttime: raw.manifest.pm2_entry_starttime,
        pm2_profile_redacted_sha256: raw.manifest.pm2_profile_redacted_sha256.clone(),
        listener_runtime_starttime: raw.manifest.listener_runtime_starttime,
        runtime_cleanup_verified: raw.manifest.runtime_cleanup_verified,
        normal_runtime_restored: raw.manifest.normal_runtime_restored,
        fixture_guard: raw.manifest.fixture_guard.clone(),
        bot_report: raw.manifest.bot_report.clone(),
        retained_bot_report_path: raw.manifest.bot_report.as_ref().map(|_| {
            format!(
                "{RAW_PROVENANCE_DIR}/{}",
                match raw.manifest.side {
                    RawSide::Cpp => CPP_BOT_REPORT_FILE,
                    RawSide::Rust => RUST_BOT_REPORT_FILE,
                }
            )
        }),
    })
}

/// Verify the schema and every retained source/output hash for a required
/// flow. The raw captures themselves stay gitignored; their exact manifests
/// are committed and cross-bound to the hashes verified during import.
pub fn verify_required_lineage(
    flow: &str,
    flow_dir: &Path,
    expected_selection: &ImportSelection,
) -> Result<()> {
    let path = flow_dir.join(LINEAGE_FILE);
    let bytes = read_regular_file(&path)
        .with_context(|| format!("reading required lineage {}", path.display()))?;
    let lineage: DerivedLineage = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing required lineage {}", path.display()))?;
    ensure!(
        lineage.version == LINEAGE_VERSION,
        "required lineage has unsupported version {}",
        lineage.version
    );
    ensure!(
        lineage.flow == flow,
        "required lineage flow does not match {flow:?}"
    );
    ensure!(lineage.completed, "required lineage completed must be true");
    ensure!(
        lineage.selection.strict,
        "required lineage import was not strict"
    );
    ensure!(
        lineage.selection == *expected_selection,
        "required lineage selection does not match the reviewed import contract"
    );

    verify_retained_source(flow, flow_dir, RawSide::Cpp, &lineage.sources.cpp)?;
    verify_retained_source(flow, flow_dir, RawSide::Rust, &lineage.sources.rust)?;
    if flow == "creature-spell-casting" {
        validate_creature_spell_fixture_files(&flow_dir.join("fixture"))?;
    }
    ensure!(
        lineage.sources.cpp.harness_repo_head == lineage.sources.rust.harness_repo_head
            && lineage.sources.cpp.harness_worktree_state_sha256
                == lineage.sources.rust.harness_worktree_state_sha256
            && lineage.sources.cpp.worktree_state_algorithm
                == lineage.sources.rust.worktree_state_algorithm,
        "required lineage C++/Rust harness identities differ"
    );
    if flow == "creature-spell-casting" {
        ensure!(
            lineage
                .sources
                .cpp
                .fixture_guard
                .as_ref()
                .zip(lineage.sources.rust.fixture_guard.as_ref())
                .is_some_and(|(cpp, rust)| {
                    creature_spell_fixture_shared_identity_matches(cpp, rust)
                }),
            "required lineage C++/Rust creature-spell fixture identities differ"
        );
        let cpp_bot = lineage
            .sources
            .cpp
            .bot_report
            .as_ref()
            .context("required lineage C++ creature-spell bot report is missing")?;
        let rust_bot = lineage
            .sources
            .rust
            .bot_report
            .as_ref()
            .context("required lineage Rust creature-spell bot report is missing")?;
        ensure!(
            cpp_bot.contract == rust_bot.contract
                && cpp_bot.exec_path == rust_bot.exec_path
                && cpp_bot.exec_sha256 == rust_bot.exec_sha256
                && cpp_bot.account == rust_bot.account
                && cpp_bot.account_id == rust_bot.account_id
                && cpp_bot.character_guid == rust_bot.character_guid,
            "required lineage C++/Rust creature-spell bot identities differ"
        );
    } else if matches!(
        flow,
        "loot-single-item-claim"
            | "loot-two-session-atomic-race"
            | "vendor-extended-cost-purchase"
            | "detour-chase-around-obstacle"
    ) {
        if flow == "detour-chase-around-obstacle" {
            ensure!(
                lineage
                    .sources
                    .cpp
                    .fixture_guard
                    .as_ref()
                    .zip(lineage.sources.rust.fixture_guard.as_ref())
                    .is_some_and(|(cpp, rust)| detour_fixture_shared_identity_matches(cpp, rust)),
                "required lineage C++/Rust detour fixture identities differ"
            );
        } else if flow != "vendor-extended-cost-purchase" {
            ensure!(
                lineage.sources.cpp.fixture_guard == lineage.sources.rust.fixture_guard,
                "required lineage C++/Rust guarded-loot fixture identities differ"
            );
        }
        let cpp_bot = lineage
            .sources
            .cpp
            .bot_report
            .as_ref()
            .context("required lineage C++ source is missing canonical bot report identity")?;
        let rust_bot = lineage
            .sources
            .rust
            .bot_report
            .as_ref()
            .context("required lineage Rust source is missing canonical bot report identity")?;
        ensure!(
            cpp_bot.contract == rust_bot.contract
                && cpp_bot.exec_path == rust_bot.exec_path
                && cpp_bot.exec_sha256 == rust_bot.exec_sha256
                && cpp_bot.account == rust_bot.account
                && cpp_bot.account_id == rust_bot.account_id
                && cpp_bot.character_guid == rust_bot.character_guid,
            "required lineage C++/Rust bot identities differ"
        );
    }
    verify_file_lineage(flow_dir, &lineage.outputs.cpp_pkt, "cpp.pkt")?;
    verify_file_lineage(
        flow_dir,
        &lineage.outputs.expected_divergences,
        "expected-divergences.json",
    )?;

    ensure!(
        lineage.outputs.rust.path == "rust",
        "Rust output path must be rust"
    );
    let rust_digest = digest_tree(&flow_dir.join("rust"), None)?;
    ensure!(
        rust_digest.sha256 == lineage.outputs.rust.tree_sha256,
        "derived Rust output tree SHA-256 does not match lineage"
    );
    ensure!(
        rust_digest.file_count == lineage.outputs.rust.file_count,
        "derived Rust output file count does not match lineage"
    );
    ensure!(
        rust_digest.packet_count == lineage.outputs.rust.packet_count,
        "derived Rust output packet count does not match lineage"
    );
    Ok(())
}

fn verify_retained_source(
    flow: &str,
    flow_dir: &Path,
    side: RawSide,
    source: &SourceLineage,
) -> Result<()> {
    let expected_path = match side {
        RawSide::Cpp => format!("{RAW_PROVENANCE_DIR}/{CPP_RAW_MANIFEST_FILE}"),
        RawSide::Rust => format!("{RAW_PROVENANCE_DIR}/{RUST_RAW_MANIFEST_FILE}"),
    };
    ensure!(
        source.manifest_path == expected_path,
        "retained {side:?} manifest path is not canonical"
    );
    validate_sha256(&source.manifest_sha256, "retained raw manifest SHA-256")?;
    validate_sha256(&source.raw_artifact_sha256, "retained raw artifact SHA-256")?;
    validate_sha256(
        &source.expected_exec_sha256,
        "retained expected executable SHA-256",
    )?;
    validate_sha256(
        &source.source_exec_sha256,
        "retained source executable SHA-256",
    )?;
    validate_sha256(&source.live_exec_sha256, "retained live executable SHA-256")?;
    validate_sha256(
        &source.harness_worktree_state_sha256,
        "retained harness worktree state SHA-256",
    )?;
    validate_sha256(
        &source.source_worktree_state_sha256,
        "retained source worktree state SHA-256",
    )?;
    validate_sha256(&source.pm2_exec_sha256, "retained PM2 executable SHA-256")?;
    validate_sha256(
        &source.pm2_profile_redacted_sha256,
        "retained PM2 profile SHA-256",
    )?;
    validate_sha256(
        &source.effective_config_redacted_sha256,
        "retained effective config SHA-256",
    )?;
    ensure!(
        source.executable_pin_enforced,
        "required raw executable was not pinned"
    );
    match (flow, side) {
        ("creature-spell-casting", RawSide::Cpp) => {
            let derivation = source
                .source_derivation
                .as_ref()
                .context("required lineage C++ creature-spell source_derivation is missing")?;
            validate_source_derivation_schema(derivation)?;
            ensure!(
                derivation.contract == CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT
                    && derivation.remote_url == CREATURE_SPELL_CPP_REMOTE_URL
                    && derivation.remote_ref == CREATURE_SPELL_CPP_REMOTE_REF
                    && derivation.base_head == CREATURE_SPELL_CPP_BASE_HEAD
                    && derivation.base_tree == CREATURE_SPELL_CPP_BASE_TREE
                    && derivation.patched_head == CREATURE_SPELL_CPP_PATCHED_HEAD
                    && derivation.patched_tree == CREATURE_SPELL_CPP_PATCHED_TREE
                    && derivation.patch_path == CREATURE_SPELL_CPP_PATCH_PATH
                    && derivation.patch_sha256 == CREATURE_SPELL_CPP_PATCH_SHA256
                    && derivation.changed_paths == [CREATURE_SPELL_CPP_CHANGED_PATH]
                    && source.source_repo_head == derivation.patched_head,
                "required lineage C++ creature-spell source_derivation differs from the reviewed patch"
            );
        }
        _ => ensure!(
            source.source_derivation.is_none(),
            "required lineage contains source_derivation outside C++ creature-spell evidence"
        ),
    }

    let path = flow_dir.join(&source.manifest_path);
    let raw = read_and_validate_raw_manifest(&path, flow, side, true)?;
    ensure!(
        raw.manifest_sha256 == source.manifest_sha256,
        "retained {side:?} raw manifest SHA-256 does not match lineage"
    );
    ensure!(
        raw.manifest.harness_repo_head == source.harness_repo_head
            && raw.manifest.source_repo_head == source.source_repo_head
            && raw.manifest.source_derivation == source.source_derivation
            && raw.manifest.harness_worktree_clean == source.harness_worktree_clean
            && raw.manifest.harness_worktree_state_sha256 == source.harness_worktree_state_sha256
            && raw.manifest.source_worktree_dirty == source.source_worktree_dirty
            && raw.manifest.source_worktree_state_sha256 == source.source_worktree_state_sha256
            && raw.manifest.worktree_state_algorithm == source.worktree_state_algorithm
            && raw.manifest.expected_exec_path == source.expected_exec_path
            && raw.manifest.expected_exec_sha256 == source.expected_exec_sha256
            && raw.manifest.source_exec_path == source.source_exec_path
            && raw.manifest.source_exec_sha256 == source.source_exec_sha256
            && raw.manifest.live_exec_path == source.live_exec_path
            && raw.manifest.live_exec_sha256 == source.live_exec_sha256
            && raw.manifest.executable_pin_enforced == source.executable_pin_enforced
            && raw.manifest.pm2_entry_pid == source.pm2_entry_pid
            && raw.manifest.pm2_entry_starttime == source.pm2_entry_starttime
            && raw.manifest.pm2_exec_path == source.pm2_exec_path
            && raw.manifest.pm2_exec_sha256 == source.pm2_exec_sha256
            && raw.manifest.pm2_profile_redacted_sha256 == source.pm2_profile_redacted_sha256
            && raw.manifest.listener_runtime_pid == source.listener_runtime_pid
            && raw.manifest.listener_runtime_starttime == source.listener_runtime_starttime
            && raw.manifest.listener_relationship_verified == source.listener_relationship_verified
            && raw.manifest.restart_count == source.restart_count
            && raw.manifest.effective_config_path == source.effective_config_path
            && raw.manifest.effective_config_redacted_sha256
                == source.effective_config_redacted_sha256
            && raw.manifest.effective_config_algorithm == source.effective_config_algorithm
            && raw.manifest.runtime_cleanup_verified == source.runtime_cleanup_verified
            && raw.manifest.normal_runtime_restored == source.normal_runtime_restored
            && raw.manifest.fixture_guard == source.fixture_guard
            && raw.manifest.bot_report == source.bot_report,
        "retained {side:?} source/process/config provenance does not match lineage"
    );
    match (&raw.manifest.bot_report, &source.retained_bot_report_path) {
        (Some(evidence), Some(relative)) => {
            let canonical = match side {
                RawSide::Cpp => format!("{RAW_PROVENANCE_DIR}/{CPP_BOT_REPORT_FILE}"),
                RawSide::Rust => format!("{RAW_PROVENANCE_DIR}/{RUST_BOT_REPORT_FILE}"),
            };
            ensure!(
                relative == &canonical,
                "retained {side:?} bot report path is not canonical"
            );
            let bytes = read_regular_file(&flow_dir.join(relative))?;
            ensure!(
                sha256_bytes(&bytes) == evidence.report_sha256,
                "retained {side:?} bot report SHA-256 does not match manifest"
            );
            validate_bot_report_json(&bytes, evidence)?;
        }
        (None, None) => {}
        _ => bail!("retained {side:?} bot report presence does not match manifest"),
    }
    match side {
        RawSide::Cpp => {
            ensure!(
                raw.manifest.artifact.sha256.as_deref()
                    == Some(source.raw_artifact_sha256.as_str()),
                "retained C++ raw artifact SHA-256 does not match lineage"
            );
            ensure!(
                raw.manifest.artifact.size == source.raw_artifact_size,
                "retained C++ raw artifact size does not match lineage"
            );
            ensure!(
                source.raw_packet_count.is_none(),
                "C++ lineage has a packet count"
            );
        }
        RawSide::Rust => {
            ensure!(
                raw.manifest.artifact.tree_sha256.as_deref()
                    == Some(source.raw_artifact_sha256.as_str()),
                "retained Rust raw tree SHA-256 does not match lineage"
            );
            ensure!(
                raw.manifest.artifact.packet_count == source.raw_packet_count,
                "retained Rust raw packet count does not match lineage"
            );
            ensure!(
                source.raw_artifact_size.is_none(),
                "Rust lineage has an artifact size"
            );
        }
    }
    Ok(())
}

fn file_lineage(root: &Path, relative: &str) -> Result<FileLineage> {
    let bytes = read_regular_file(&root.join(relative))?;
    Ok(FileLineage {
        path: relative.to_string(),
        size: u64::try_from(bytes.len()).context("derived file size does not fit u64")?,
        sha256: sha256_bytes(&bytes),
    })
}

fn verify_file_lineage(root: &Path, expected: &FileLineage, canonical: &str) -> Result<()> {
    ensure!(
        expected.path == canonical,
        "derived output path must be {canonical}"
    );
    validate_sha256(&expected.sha256, "derived output SHA-256")?;
    let actual = file_lineage(root, canonical)?;
    ensure!(
        actual == *expected,
        "derived {canonical} size or SHA-256 does not match lineage"
    );
    Ok(())
}

fn validate_sha256(value: &str, label: &str) -> Result<()> {
    ensure!(
        value.len() == SHA256_HEX_LEN
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "{label} must be exactly 64 lowercase hexadecimal characters"
    );
    Ok(())
}

fn valid_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn validate_utc_timestamp(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    ensure!(
        bytes.len() == 20
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':'
            && bytes[19] == b'Z'
            && bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16 | 19)
                    || byte.is_ascii_digit()),
        "created_at must be canonical UTC RFC3339 (YYYY-MM-DDTHH:MM:SSZ)"
    );
    let number = |start: usize, end: usize| -> Result<u32> {
        std::str::from_utf8(&bytes[start..end])?
            .parse::<u32>()
            .map_err(Into::into)
    };
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;
    ensure!(year >= 1970, "created_at year is before 1970");
    ensure!((1..=12).contains(&month), "created_at month is invalid");
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    ensure!((1..=days).contains(&day), "created_at day is invalid");
    ensure!(hour < 24, "created_at hour is invalid");
    ensure!(minute < 60, "created_at minute is invalid");
    ensure!(second < 60, "created_at second is invalid");
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("inspecting {}", path.display()))?;
    ensure!(
        metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
        "{} is not a regular non-symlink file",
        path.display()
    );
    let mut file = open_read_no_follow(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(bytes)
}

#[cfg(unix)]
fn open_read_no_follow(path: &Path) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| format!("opening {} without following symlinks", path.display()))
}

#[cfg(not(unix))]
fn open_read_no_follow(path: &Path) -> Result<File> {
    File::open(path).with_context(|| format!("opening {}", path.display()))
}

fn digest_tree(root: &Path, excluded_manifest: Option<&Path>) -> Result<TreeDigest> {
    let root_meta = fs::symlink_metadata(root)
        .with_context(|| format!("inspecting tree {}", root.display()))?;
    ensure!(
        root_meta.file_type().is_dir() && !root_meta.file_type().is_symlink(),
        "{} is not a non-symlink directory",
        root.display()
    );
    let excluded = excluded_manifest.and_then(|path| path.canonicalize().ok());
    let mut files = BTreeMap::<Vec<u8>, PathBuf>::new();
    collect_tree_files(root, root, excluded.as_deref(), &mut files)?;
    ensure!(
        !files.is_empty(),
        "{} contains no capture files",
        root.display()
    );

    let mut hasher = Sha256::new();
    let mut packet_count = 0_u64;
    for (relative, path) in &files {
        let bytes = read_regular_file(path)?;
        hasher.update(relative);
        hasher.update([0]);
        hasher.update(sha256_bytes(&bytes).as_bytes());
        hasher.update([0]);
        if path.extension().and_then(|extension| extension.to_str()) == Some("meta") {
            packet_count = packet_count
                .checked_add(1)
                .context("packet count overflow")?;
        }
    }
    Ok(TreeDigest {
        sha256: format!("{:x}", hasher.finalize()),
        file_count: u64::try_from(files.len()).context("tree file count does not fit u64")?,
        packet_count,
    })
}

fn collect_tree_files(
    root: &Path,
    directory: &Path,
    excluded: Option<&Path>,
    files: &mut BTreeMap<Vec<u8>, PathBuf>,
) -> Result<()> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("reading directory {}", directory.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "capture tree contains symlink {}",
            path.display()
        );
        if metadata.file_type().is_dir() {
            collect_tree_files(root, &path, excluded, files)?;
        } else if metadata.file_type().is_file() {
            if path.file_name().and_then(|name| name.to_str()) == Some(RUST_RAW_MANIFEST_FILE) {
                if excluded.is_some_and(|excluded| {
                    path.canonicalize()
                        .is_ok_and(|canonical| canonical == excluded)
                }) {
                    continue;
                }
                bail!(
                    "capture tree contains unexpected or nested {RUST_RAW_MANIFEST_FILE} at {}",
                    path.display()
                );
            }
            if excluded.is_some_and(|excluded| {
                path.canonicalize()
                    .is_ok_and(|canonical| canonical == excluded)
            }) {
                continue;
            }
            let relative = relative_path_bytes(root, &path)?;
            ensure!(
                files.insert(relative, path).is_none(),
                "capture tree contains duplicate path bytes"
            );
        } else {
            bail!("capture tree contains unsupported entry {}", path.display());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn relative_path_bytes(root: &Path, path: &Path) -> Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt as _;
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?
        .as_os_str()
        .as_bytes()
        .to_vec())
}

#[cfg(not(unix))]
fn relative_path_bytes(root: &Path, path: &Path) -> Result<Vec<u8>> {
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?
        .to_string_lossy()
        .replace('\\', "/")
        .into_bytes())
}

fn write_synced_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("writing {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing {}", path.display()))?;
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("atomic output has no parent")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("atomic output name is not UTF-8")?;
    for attempt in 0..100_u64 {
        let temp = parent.join(format!(
            ".{name}.partial.{}.{}.{}",
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed),
            attempt
        ));
        match write_synced_file(&temp, bytes) {
            Ok(()) => {
                if let Err(error) = fs::rename(&temp, path) {
                    let _ = fs::remove_file(&temp);
                    return Err(error).with_context(|| format!("publishing {}", path.display()));
                }
                sync_directory(parent)?;
                return Ok(());
            }
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|io| io.kind() == std::io::ErrorKind::AlreadyExists) => {}
            Err(error) => return Err(error),
        }
    }
    bail!(
        "could not allocate an atomic staging file for {}",
        path.display()
    )
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("opening directory {} for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("syncing directory {}", path.display()))
}

/// A complete flow prepared outside its published path. Dropping this value
/// before [`AtomicFlowImport::publish`] is equivalent to an interrupted import:
/// the old flow remains byte-for-byte visible and the staging tree is removed.
pub struct AtomicFlowImport {
    root: PathBuf,
    target: PathBuf,
    staging: PathBuf,
    target_existed_at_prepare: bool,
    published: bool,
}

impl AtomicFlowImport {
    /// Prepare a private complete-tree staging directory, copying only the
    /// flow's hand-reviewed metadata from the previous generation.
    pub fn prepare(root: &Path, flow: &str) -> Result<Self> {
        fs::create_dir_all(root)
            .with_context(|| format!("creating flow root {}", root.display()))?;
        let target = root.join(flow);
        let target_existed_at_prepare = match fs::symlink_metadata(&target) {
            Ok(metadata) => {
                ensure!(
                    metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
                    "published flow {} is not a non-symlink directory",
                    target.display()
                );
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error).context("inspecting published flow target"),
        };
        let staging = allocate_staging_directory(root, flow)?;
        let mut transaction = Self {
            root: root.to_path_buf(),
            target,
            staging,
            target_existed_at_prepare,
            published: false,
        };
        if transaction.target_existed_at_prepare
            && let Err(error) = copy_reviewed_metadata(&transaction.target, &transaction.staging)
        {
            let _ = fs::remove_dir_all(&transaction.staging);
            transaction.published = true;
            return Err(error);
        }
        Ok(transaction)
    }

    /// Directory into which all derived artifacts and the lineage marker must
    /// be written and validated before publication.
    #[must_use]
    pub fn staging_dir(&self) -> &Path {
        &self.staging
    }

    /// Atomically publish the entire complete flow. Existing flows use Linux
    /// `RENAME_EXCHANGE`, so a process death exposes either the old complete
    /// tree or the new complete tree, never a mixture of their files.
    pub fn publish(mut self) -> Result<()> {
        sync_tree(&self.staging)?;
        if self.target_existed_at_prepare {
            atomic_exchange_directories(&self.staging, &self.target)?;
            self.published = true;
            sync_directory(&self.root)?;
            if let Err(error) = fs::remove_dir_all(&self.staging) {
                eprintln!(
                    "capture-diff: warning: imported flow is complete, but old staging tree {} could not be removed: {error}",
                    self.staging.display()
                );
            } else {
                sync_directory(&self.root)?;
            }
        } else {
            atomic_publish_new_directory(&self.staging, &self.target)?;
            self.published = true;
            sync_directory(&self.root)?;
        }
        Ok(())
    }
}

impl Drop for AtomicFlowImport {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.staging);
        }
    }
}

fn allocate_staging_directory(root: &Path, flow: &str) -> Result<PathBuf> {
    for _ in 0..100_u64 {
        let candidate = root.join(format!(
            ".{flow}.import-partial.{}.{}",
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("creating staging flow {}", candidate.display()));
            }
        }
    }
    bail!("could not allocate a staging directory for flow {flow:?}")
}

fn is_generated_entry(name: &str) -> bool {
    matches!(
        name,
        "cpp.pkt" | "rust" | "expected-divergences.json" | LINEAGE_FILE | RAW_PROVENANCE_DIR
    )
}

fn copy_reviewed_metadata(source: &Path, destination: &Path) -> Result<()> {
    let flow = source
        .file_name()
        .and_then(|name| name.to_str())
        .context("published flow name is not UTF-8")?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("flow metadata filename is not UTF-8"))?;
        if is_generated_entry(&name) {
            continue;
        }
        if flow == "detour-chase-around-obstacle" && name == "fixture" {
            copy_detour_fixture_metadata(&entry.path(), &destination.join(&name))?;
            continue;
        }
        if flow == "creature-spell-casting" && name == "fixture" {
            copy_creature_spell_fixture_metadata(&entry.path(), &destination.join(&name))?;
            continue;
        }
        ensure!(
            matches!(
                name.as_str(),
                "README.md" | "flow.json" | "requirement.json"
            ),
            "flow contains unknown non-generated entry {name:?}; import copies only reviewed flow metadata"
        );
        copy_tree_entry(&entry.path(), &destination.join(name))?;
    }
    Ok(())
}

fn copy_creature_spell_fixture_metadata(source: &Path, destination: &Path) -> Result<()> {
    validate_creature_spell_fixture_files(source)?;

    fs::create_dir(destination)?;
    for name in ["fixture.json", "cpp-reference.patch"] {
        copy_tree_entry(&source.join(name), &destination.join(name))?;
    }
    Ok(())
}

fn validate_creature_spell_fixture_files(source: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "creature-spell fixture is not a non-symlink directory"
    );
    let mut root_names = fs::read_dir(source)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("creature-spell fixture filename is not UTF-8"))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    root_names.sort();
    ensure!(
        root_names == ["cpp-reference.patch", "fixture.json"],
        "creature-spell fixture contains an unreviewed root entry"
    );
    let manifest_bytes = read_regular_file(&source.join("fixture.json"))?;
    let patch_bytes = read_regular_file(&source.join("cpp-reference.patch"))?;
    ensure!(
        sha256_bytes(&manifest_bytes) == CREATURE_SPELL_FIXTURE_MANIFEST_SHA256,
        "creature-spell fixture manifest differs from the reviewed bytes"
    );
    ensure!(
        sha256_bytes(&patch_bytes) == CREATURE_SPELL_CPP_PATCH_SHA256,
        "creature-spell C++ reference patch differs from the reviewed bytes"
    );
    let fixture: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .context("parsing reviewed creature-spell fixture manifest")?;
    let derivation = fixture
        .get("source_derivation")
        .context("reviewed creature-spell fixture source_derivation is missing")?;
    ensure!(
        derivation
            .get("contract")
            .and_then(serde_json::Value::as_str)
            == Some(CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT)
            && derivation
                .get("remote_url")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_REMOTE_URL)
            && derivation
                .get("remote_ref")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_REMOTE_REF)
            && derivation
                .get("base_head")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_BASE_HEAD)
            && derivation
                .get("base_tree")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_BASE_TREE)
            && derivation
                .get("patched_head")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCHED_HEAD)
            && derivation
                .get("patched_tree")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCHED_TREE)
            && derivation
                .get("patch_path")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCH_PATH)
            && derivation
                .get("patch_sha256")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCH_SHA256)
            && derivation.get("changed_paths")
                == Some(&serde_json::json!([CREATURE_SPELL_CPP_CHANGED_PATH])),
        "reviewed creature-spell fixture source_derivation metadata differs from the pinned patch"
    );
    Ok(())
}

fn copy_detour_fixture_metadata(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "detour fixture is not a non-symlink directory"
    );
    let mut root_names = fs::read_dir(source)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("detour fixture filename is not UTF-8"))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    root_names.sort();
    ensure!(
        root_names == ["fixture.json", "mmaps"],
        "detour fixture contains an unreviewed root entry"
    );
    let mmaps = source.join("mmaps");
    let mmaps_metadata = fs::symlink_metadata(&mmaps)?;
    ensure!(
        mmaps_metadata.file_type().is_dir() && !mmaps_metadata.file_type().is_symlink(),
        "detour fixture mmaps is not a non-symlink directory"
    );
    let mut mmap_names = fs::read_dir(&mmaps)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("detour MMap filename is not UTF-8"))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    mmap_names.sort();
    ensure!(
        mmap_names == ["0001.mmap", "00015026.mmtile"],
        "detour fixture contains an unreviewed MMap entry"
    );
    let manifest_bytes = read_regular_file(&source.join("fixture.json"))?;
    let map_bytes = read_regular_file(&mmaps.join("0001.mmap"))?;
    let tile_bytes = read_regular_file(&mmaps.join("00015026.mmtile"))?;
    ensure!(
        sha256_bytes(&manifest_bytes) == DETOUR_FIXTURE_MANIFEST_SHA256,
        "detour fixture manifest differs from the reviewed bytes"
    );
    ensure!(
        map_bytes.len() == 28 && sha256_bytes(&map_bytes) == DETOUR_FIXTURE_MAP_SHA256,
        "detour fixture map header differs from the reviewed bytes"
    );
    ensure!(
        tile_bytes.len() == 1_496 && sha256_bytes(&tile_bytes) == DETOUR_FIXTURE_TILE_SHA256,
        "detour fixture tile differs from the reviewed bytes"
    );

    fs::create_dir(destination)?;
    copy_tree_entry(
        &source.join("fixture.json"),
        &destination.join("fixture.json"),
    )?;
    fs::create_dir(destination.join("mmaps"))?;
    for name in mmap_names {
        copy_tree_entry(&mmaps.join(&name), &destination.join("mmaps").join(name))?;
    }
    Ok(())
}

fn copy_tree_entry(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "flow metadata contains symlink {}",
        source.display()
    );
    if metadata.file_type().is_file() {
        fs::copy(source, destination).with_context(|| {
            format!(
                "copying flow metadata {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        Ok(())
    } else {
        bail!(
            "reviewed flow metadata must be a regular file: {}",
            source.display()
        )
    }
}

fn sync_tree(root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(root)?;
    ensure!(
        metadata.file_type().is_dir(),
        "{} is not a directory",
        root.display()
    );
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "staging tree contains symlink {}",
            path.display()
        );
        if metadata.file_type().is_dir() {
            sync_tree(&path)?;
        } else if metadata.file_type().is_file() {
            File::open(&path)?.sync_all()?;
        } else {
            bail!("staging tree contains unsupported entry {}", path.display());
        }
    }
    sync_directory(root)
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn atomic_exchange_directories(left: &Path, right: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    let left = CString::new(left.as_os_str().as_bytes()).context("staging path contains NUL")?;
    let right = CString::new(right.as_os_str().as_bytes()).context("target path contains NUL")?;
    // SAFETY: both C strings remain alive for the call, contain terminating
    // NUL bytes supplied by `CString`, and `renameat2` does not retain them.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            left.as_ptr(),
            libc::AT_FDCWD,
            right.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "atomically exchanging staged flow {} with {}",
                left.to_string_lossy(),
                right.to_string_lossy()
            )
        })
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn atomic_publish_new_directory(source: &Path, target: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    let source =
        CString::new(source.as_os_str().as_bytes()).context("staging path contains NUL")?;
    let target = CString::new(target.as_os_str().as_bytes()).context("target path contains NUL")?;
    // SAFETY: both C strings remain alive and NUL-terminated for renameat2;
    // RENAME_NOREPLACE makes a concurrent target creation fail atomically.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "atomically publishing staged flow {} as new target {} without replacement",
                source.to_string_lossy(),
                target.to_string_lossy()
            )
        })
    }
}

#[cfg(not(target_os = "linux"))]
fn atomic_exchange_directories(_left: &Path, _right: &Path) -> Result<()> {
    bail!("replacing an existing flow atomically requires Linux renameat2(RENAME_EXCHANGE)")
}

#[cfg(not(target_os = "linux"))]
fn atomic_publish_new_directory(_source: &Path, _target: &Path) -> Result<()> {
    bail!("publishing a new flow without replacement requires Linux renameat2(RENAME_NOREPLACE)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CapturedPacket;

    fn test_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "capture-diff-lineage-{label}-{}-{}",
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn fake_oid() -> String {
        "a".repeat(40)
    }

    fn reviewed_cpp_source_derivation() -> serde_json::Value {
        serde_json::json!({
            "contract": CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT,
            "remote_url": CREATURE_SPELL_CPP_REMOTE_URL,
            "remote_ref": CREATURE_SPELL_CPP_REMOTE_REF,
            "base_head": CREATURE_SPELL_CPP_BASE_HEAD,
            "base_tree": CREATURE_SPELL_CPP_BASE_TREE,
            "patched_head": CREATURE_SPELL_CPP_PATCHED_HEAD,
            "patched_tree": CREATURE_SPELL_CPP_PATCHED_TREE,
            "patch_path": CREATURE_SPELL_CPP_PATCH_PATH,
            "patch_sha256": CREATURE_SPELL_CPP_PATCH_SHA256,
            "changed_paths": [CREATURE_SPELL_CPP_CHANGED_PATH]
        })
    }

    fn make_raw_pair(root: &Path, flow: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let raw = root.join("raw");
        let rust = raw.join("rust");
        fs::create_dir_all(&rust).unwrap();
        let cpp = raw.join("cpp.pkt");
        fs::write(&cpp, b"raw-cpp").unwrap();
        fs::write(
            rust.join("one.meta"),
            b"direction=c2s\nseq=0\nopcode=0x0001\n",
        )
        .unwrap();
        fs::write(rust.join("one.bin"), [1_u8, 0]).unwrap();
        let cpp_manifest = raw.join(CPP_RAW_MANIFEST_FILE);
        let rust_manifest = rust.join(RUST_RAW_MANIFEST_FILE);
        let rust_digest = digest_tree(&rust, Some(&rust_manifest)).unwrap();
        let cpp_json = serde_json::json!({
            "version": 3,
            "flow": flow,
            "side": "cpp",
            "completed": true,
            "created_at": "2026-07-19T00:00:00Z",
            "harness_repo_head": fake_oid(),
            "source_repo_head": "d".repeat(40),
            "source_exec_revision": "d".repeat(40),
            "harness_worktree_clean": true,
            "harness_worktree_state_sha256": "1".repeat(64),
            "source_worktree_dirty": true,
            "source_worktree_state_sha256": "2".repeat(64),
            "worktree_state_algorithm": "git-head-path-mode-content-sha256-v1",
            "expected_exec_path": "/opt/trinity/worldserver",
            "expected_exec_sha256": "b".repeat(64),
            "source_exec_path": "/opt/trinity/worldserver",
            "source_exec_sha256": "b".repeat(64),
            "live_exec_path": "/opt/trinity/worldserver",
            "live_exec_sha256": "b".repeat(64),
            "executable_pin_enforced": true,
            "pm2_entry_pid": 122,
            "pm2_entry_starttime": 1001,
            "pm2_exec_path": "/opt/trinity/worldserver-wrapper.sh",
            "pm2_exec_sha256": "3".repeat(64),
            "pm2_profile_redacted_sha256": "5".repeat(64),
            "listener_runtime_pid": 123,
            "listener_runtime_starttime": 1002,
            "listener_relationship_verified": true,
            "restart_count": 2,
            "effective_config_path": "/etc/trinity/worldserver.conf",
            "effective_config_redacted_sha256": "e".repeat(64),
            "effective_config_algorithm": "capture-relevant-redacted-v1",
            "runtime_cleanup_verified": true,
            "normal_runtime_restored": true,
            "artifact": {
                "path": "cpp.pkt",
                "size": 7,
                "sha256": sha256_bytes(b"raw-cpp")
            }
        });
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&cpp_json).unwrap()).unwrap();
        let rust_json = serde_json::json!({
            "version": 3,
            "flow": flow,
            "side": "rust",
            "completed": true,
            "created_at": "2026-07-19T00:00:01Z",
            "harness_repo_head": fake_oid(),
            "source_repo_head": fake_oid(),
            "source_exec_revision": fake_oid(),
            "harness_worktree_clean": true,
            "harness_worktree_state_sha256": "1".repeat(64),
            "source_worktree_dirty": false,
            "source_worktree_state_sha256": "1".repeat(64),
            "worktree_state_algorithm": "git-head-path-mode-content-sha256-v1",
            "expected_exec_path": "/opt/rustycore/world-server",
            "expected_exec_sha256": "c".repeat(64),
            "source_exec_path": "/opt/rustycore/world-server",
            "source_exec_sha256": "c".repeat(64),
            "live_exec_path": "/opt/rustycore/world-server",
            "live_exec_sha256": "c".repeat(64),
            "executable_pin_enforced": true,
            "pm2_entry_pid": 456,
            "pm2_entry_starttime": 2001,
            "pm2_exec_path": "/opt/rustycore/world-server",
            "pm2_exec_sha256": "c".repeat(64),
            "pm2_profile_redacted_sha256": "6".repeat(64),
            "listener_runtime_pid": 456,
            "listener_runtime_starttime": 2001,
            "listener_relationship_verified": true,
            "restart_count": 3,
            "effective_config_path": "/etc/rustycore/worldserver.conf",
            "effective_config_redacted_sha256": "f".repeat(64),
            "effective_config_algorithm": "capture-relevant-redacted-v1",
            "runtime_cleanup_verified": true,
            "normal_runtime_restored": true,
            "artifact": {
                "path": "rust",
                "packet_count": rust_digest.packet_count,
                "tree_sha256": rust_digest.sha256
            }
        });
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&rust_json).unwrap(),
        )
        .unwrap();
        if flow == "loot-single-item-claim" {
            let fixture = serde_json::json!({
                "enabled": true,
                "contract": "loot-single-item-claim-fixture-v1",
                "account": "TESTBOT2@bot.local",
                "account_id": 9,
                "character_guid": 15,
                "peer_account": "TESTBOT3@bot.local",
                "peer_account_id": 10,
                "peer_character_guid": 16,
                "creature_entry": 21779,
                "creature_spawn_guid": 1117,
                "item_entry": 30712,
                "cleanup_verified": true
            });
            let report_json = serde_json::json!({
                "loot_item_capture": true,
                "loot_race_smoke": false,
                "results": [{
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "world_auth": true,
                    "enum_characters": true,
                    "player_login_verified": true,
                    "loot_race_smoke": true,
                    "loot_race_smoke_passed": true,
                    "loot_race_target_entry": 21779,
                    "loot_race_target_spawn_guid": 1117,
                    "loot_race_target_discovered": true,
                    "loot_race_loot_opened": true,
                    "loot_race_item_push_seen": true,
                    "loot_race_loot_removed_seen": true,
                    "loot_race_loot_coins": 0,
                    "loot_race_coin_removed_seen": false,
                    "loot_race_db_item_total": 1,
                    "loot_race_db_money_delta": 0,
                    "loot_race_relog_verified": true,
                    "loot_race_failure": null
                }]
            });
            let report_bytes = serde_json::to_vec_pretty(&report_json).unwrap();
            let cpp_report = raw.join("cpp-report.json");
            let rust_report = raw.join("rust-report.json");
            fs::write(&cpp_report, &report_bytes).unwrap();
            fs::write(&rust_report, &report_bytes).unwrap();

            for (manifest_path, report_path) in
                [(&cpp_manifest, cpp_report), (&rust_manifest, rust_report)]
            {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
                manifest["fixture_guard"] = fixture.clone();
                manifest["bot_report"] = serde_json::json!({
                    "contract": "wow-test-bot-loot-item-capture-report-v1",
                    "exec_path": "/opt/rustycore/wow-test-bot",
                    "exec_sha256": "7".repeat(64),
                    "report_path": report_path.to_string_lossy(),
                    "report_sha256": sha256_bytes(&report_bytes),
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "report_validated": true
                });
                fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            }
        } else if flow == "loot-two-session-atomic-race" {
            let fixture = serde_json::json!({
                "enabled": true,
                "contract": "loot-two-session-atomic-race-fixture-v1",
                "account": "TESTBOT2@bot.local",
                "account_id": 9,
                "character_guid": 15,
                "peer_account": "TESTBOT3@bot.local",
                "peer_account_id": 10,
                "peer_character_guid": 16,
                "gameobject_entry": 2846,
                "gameobject_spawn_guid": 9106001,
                "item_entry": 38,
                "cleanup_verified": true
            });
            let result = |account: &str,
                          account_id: u32,
                          character_guid: u64,
                          item_push: bool,
                          money: u64| {
                serde_json::json!({
                    "account": account,
                    "account_id": account_id,
                    "character_guid": character_guid,
                    "world_auth": true,
                    "enum_characters": true,
                    "player_login_verified": true,
                    "loot_race_smoke": true,
                    "loot_race_smoke_passed": true,
                    "loot_race_failure": null,
                    "loot_race_target_entry": 2846,
                    "loot_race_target_spawn_guid": 9106001,
                    "loot_race_target_runtime_counter": 40,
                    "loot_race_party_confirmed": true,
                    "loot_race_target_discovered": true,
                    "loot_race_loot_opened": true,
                    "loot_race_loot_list_id": 0,
                    "loot_race_loot_coins": 10,
                    "loot_race_item_push_seen": item_push,
                    "loot_race_loot_removed_seen": true,
                    "loot_race_money_notify_amount": money,
                    "loot_race_coin_removed_seen": true,
                    "loot_race_db_item_total": 1,
                    "loot_race_db_money_delta": 10,
                    "loot_race_relog_verified": true
                })
            };
            let report_json = serde_json::json!({
                "loot_item_capture": false,
                "loot_race_smoke": true,
                "results": [
                    result("TESTBOT2@bot.local", 9, 15, true, 10),
                    result("TESTBOT3@bot.local", 10, 16, false, 0)
                ]
            });
            let report_bytes = serde_json::to_vec_pretty(&report_json).unwrap();
            let cpp_report = raw.join("cpp-race-report.json");
            let rust_report = raw.join("rust-race-report.json");
            fs::write(&cpp_report, &report_bytes).unwrap();
            fs::write(&rust_report, &report_bytes).unwrap();

            for (manifest_path, report_path) in
                [(&cpp_manifest, cpp_report), (&rust_manifest, rust_report)]
            {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
                manifest["fixture_guard"] = fixture.clone();
                manifest["bot_report"] = serde_json::json!({
                    "contract": "wow-test-bot-loot-two-session-atomic-race-report-v1",
                    "exec_path": "/opt/rustycore/wow-test-bot",
                    "exec_sha256": "7".repeat(64),
                    "report_path": report_path.to_string_lossy(),
                    "report_sha256": sha256_bytes(&report_bytes),
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "report_validated": true
                });
                fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            }
        } else if flow == "creature-spell-casting" {
            let fixture = serde_json::json!({
                "enabled": true,
                "contract": "creature-spell-casting-shell-fixture-v1",
                "account": "TESTBOT2@bot.local",
                "account_id": 9,
                "character_guid": 15,
                "peer_account": "",
                "peer_account_id": 0,
                "peer_character_guid": 0,
                "creature_entry": 22378,
                "creature_spawn_guid": 78686,
                "item_entry": 0,
                "fixture_manifest_path": "/workspace/rustycore/crates/capture-diff/flows/creature-spell-casting/fixture/fixture.json",
                "fixture_manifest_sha256": CREATURE_SPELL_FIXTURE_MANIFEST_SHA256,
                "journal_sha256": "8".repeat(64),
                "database_snapshot_sha256": "4".repeat(64),
                "cleanup_verified": true
            });
            let report_result = serde_json::json!({
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "world_auth": true,
                    "enum_characters": true,
                    "player_login_verified": true,
                    "creature_spell_capture": true,
                    "creature_spell_capture_passed": true,
                    "creature_spell_fixture_manifest_sha256": CREATURE_SPELL_FIXTURE_MANIFEST_SHA256,
                    "creature_spell_target_entry": 22378,
                    "creature_spell_target_spawn_guid": 78686,
                    "creature_spell_target_runtime_counter": 78686,
                    "creature_spell_target_discovered": true,
                    "creature_spell_heartbeat_sent": true,
                    "creature_spell_heartbeat_sha256": "7".repeat(64),
                    "creature_spell_start_opcode": SMSG_SPELL_START,
                    "creature_spell_start_body_sha256": "8".repeat(64),
                    "creature_spell_start_body_bytes": 100,
                    "creature_spell_go_opcode": SMSG_SPELL_GO,
                    "creature_spell_go_body_sha256": "9".repeat(64),
                    "creature_spell_go_body_bytes": 101,
                    "creature_spell_cast_id_low": 1,
                    "creature_spell_cast_id_high": 1,
                    "creature_spell_caster_guid_low": 78686,
                    "creature_spell_caster_guid_high": 1,
                    "creature_spell_victim_guid_low": 15,
                    "creature_spell_victim_guid_high": CREATURE_SPELL_PLAYER_GUID_HIGH,
                    "creature_spell_spell_id": 15691,
                    "creature_spell_start_cast_flags": 2,
                    "creature_spell_go_cast_flags": 256,
                    "creature_spell_cast_flags_ex": 0,
                    "creature_spell_go_hit_target_count": 1,
                    "creature_spell_go_miss_target_count": 0,
                    "creature_spell_full_combat_log": false,
                    "creature_spell_advanced_logging_sent": false,
                    "creature_spell_adjacent_start_go": true,
                    "creature_spell_disconnect_confirmed": true,
                    "creature_spell_logout_confirmed": false,
                    "creature_spell_failure": null
            });
            let report_json = serde_json::json!({
                "creature_spell_capture": true,
                "detour_chase_capture": false,
                "loot_item_capture": false,
                "loot_race_smoke": false,
                "results": [report_result]
            });
            let report_bytes = serde_json::to_vec_pretty(&report_json).unwrap();
            let cpp_report = raw.join("cpp-creature-spell-report.json");
            let rust_report = raw.join("rust-creature-spell-report.json");
            fs::write(&cpp_report, &report_bytes).unwrap();
            fs::write(&rust_report, &report_bytes).unwrap();
            for (manifest_path, report_path) in
                [(&cpp_manifest, cpp_report), (&rust_manifest, rust_report)]
            {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
                if manifest_path == &cpp_manifest {
                    manifest["source_worktree_dirty"] = serde_json::Value::Bool(false);
                    manifest["source_repo_head"] =
                        serde_json::Value::String(CREATURE_SPELL_CPP_PATCHED_HEAD.to_string());
                    manifest["source_exec_revision"] =
                        serde_json::Value::String(CREATURE_SPELL_CPP_PATCHED_HEAD.to_string());
                    manifest["source_derivation"] = reviewed_cpp_source_derivation();
                }
                manifest["fixture_guard"] = fixture.clone();
                manifest["bot_report"] = serde_json::json!({
                    "contract": "wow-test-bot-creature-spell-casting-report-v1",
                    "exec_path": "/opt/rustycore/wow-test-bot",
                    "exec_sha256": "6".repeat(64),
                    "report_path": report_path.to_string_lossy(),
                    "report_sha256": sha256_bytes(&report_bytes),
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "report_validated": true
                });
                fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            }
        } else if flow == "detour-chase-around-obstacle" {
            for (side, manifest_path, private_data_dir, journal_sha256) in [
                (
                    "cpp",
                    &cpp_manifest,
                    "/tmp/rustycore-detour-cpp.test",
                    "8".repeat(64),
                ),
                (
                    "rust",
                    &rust_manifest,
                    "/tmp/rustycore-detour-rust.test",
                    "9".repeat(64),
                ),
            ] {
                let report_json = serde_json::json!({
                    "detour_chase_capture": true,
                    "loot_item_capture": false,
                    "loot_race_smoke": false,
                    "vendor_smoke": false,
                    "capture_side": side,
                    "results": [{
                        "account": "TESTBOT2@bot.local",
                        "account_id": 9,
                        "character_guid": 15,
                        "world_auth": true,
                        "enum_characters": true,
                        "player_login_verified": true,
                        "detour_chase_capture": true,
                        "detour_chase_capture_passed": true,
                        "detour_chase_target_entry": 15271,
                        "detour_chase_target_spawn_guid": 9102401,
                        "detour_chase_target_runtime_counter": 9102401,
                        "detour_chase_target_discovered": true,
                        "detour_chase_active_mover_ack_sent": true,
                        "detour_chase_attack_start_confirmed": true,
                        "detour_chase_first_swing_confirmed": true,
                        "detour_chase_prewindow_target_moves": 0,
                        "detour_chase_heartbeat_sent": true,
                        "detour_chase_heartbeat_sha256": "a".repeat(64),
                        "detour_chase_window_target_moves": 1,
                        "detour_chase_monster_move_sha256": "b".repeat(64),
                        "detour_chase_monster_move_bytes": 128,
                        "detour_chase_ping_serial": ISSUE_24_PING_FENCE_SERIAL,
                        "detour_chase_pong_confirmed": true,
                        "detour_chase_logout_confirmed": true,
                        "detour_chase_failure": null
                    }]
                });
                let report_bytes = serde_json::to_vec_pretty(&report_json).unwrap();
                let report_path = raw.join(format!("{side}-detour-report.json"));
                fs::write(&report_path, &report_bytes).unwrap();
                let fixture = serde_json::json!({
                    "enabled": true,
                    "contract": "detour-chase-around-obstacle-shell-fixture-v1",
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "peer_account": "",
                    "peer_account_id": 0,
                    "peer_character_guid": 0,
                    "creature_entry": 15271,
                    "creature_spawn_guid": 9102401,
                    "character_account_id": 9,
                    "item_entry": 0,
                    "normal_data_dir": "/srv/wow-data",
                    "private_data_dir": private_data_dir,
                    "private_data_dir_removed_before_normal_runtime": true,
                    "fixture_manifest_path": "/workspace/rustycore/crates/capture-diff/flows/detour-chase-around-obstacle/fixture/fixture.json",
                    "fixture_manifest_sha256": DETOUR_FIXTURE_MANIFEST_SHA256,
                    "synthetic_mmaps": [
                        {
                            "path": "mmaps/0001.mmap",
                            "size": 28,
                            "sha256": "3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a"
                        },
                        {
                            "path": "mmaps/00015026.mmtile",
                            "size": 1496,
                            "sha256": "693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47"
                        }
                    ],
                    "linked_read_only_data": [
                        {"name": "dbc", "target_path": "/srv/wow-data/dbc"},
                        {"name": "gt", "target_path": "/srv/wow-data/gt"},
                        {"name": "maps", "target_path": "/srv/wow-data/maps"},
                        {"name": "vmaps", "target_path": "/srv/wow-data/vmaps"},
                        {"name": "cameras", "target_path": "/srv/wow-data/cameras"}
                    ],
                    "journal_sha256": journal_sha256,
                    "database_snapshot_sha256": "4".repeat(64),
                    "cleanup_verified": true
                });
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
                if side == "cpp" {
                    manifest["source_worktree_dirty"] = serde_json::Value::Bool(false);
                }
                manifest["fixture_guard"] = fixture;
                manifest["bot_report"] = serde_json::json!({
                    "contract": "wow-test-bot-detour-chase-capture-report-v1",
                    "exec_path": "/opt/rustycore/wow-test-bot",
                    "exec_sha256": "6".repeat(64),
                    "report_path": report_path.to_string_lossy(),
                    "report_sha256": sha256_bytes(&report_bytes),
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "report_validated": true
                });
                fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            }
        } else if flow == "vendor-extended-cost-purchase" {
            let report_json = serde_json::json!({
                "vendor_smoke": true,
                "loot_item_capture": false,
                "loot_race_smoke": false,
                "results": [{
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "world_auth": true,
                    "enum_characters": true,
                    "player_login_verified": true,
                    "vendor_smoke": true,
                    "vendor_smoke_passed": true,
                    "vendor_entry": 18525,
                    "vendor_spawn_guid": 96654,
                    "vendor_runtime_counter": 111,
                    "vendor_item_entry": 30183,
                    "vendor_extended_cost": 1642,
                    "vendor_currency_id": 42,
                    "vendor_currency_before": 30,
                    "vendor_currency_after": 15,
                    "vendor_item_total_after": 1,
                    "vendor_inventory_seen": true,
                    "vendor_buy_succeeded_seen": true,
                    "vendor_set_currency_seen": true,
                    "vendor_item_push_seen": true,
                    "vendor_relogin_verified": true,
                    "vendor_failure": null
                }]
            });
            let report_bytes = serde_json::to_vec_pretty(&report_json).unwrap();
            let cpp_report = raw.join("cpp-vendor-report.json");
            let rust_report = raw.join("rust-vendor-report.json");
            fs::write(&cpp_report, &report_bytes).unwrap();
            fs::write(&rust_report, &report_bytes).unwrap();

            for (manifest_path, report_path) in
                [(&cpp_manifest, cpp_report), (&rust_manifest, rust_report)]
            {
                let mut manifest: serde_json::Value =
                    serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
                manifest["bot_report"] = serde_json::json!({
                    "contract": "wow-test-bot-vendor-extended-cost-purchase-report-v1",
                    "exec_path": "/opt/rustycore/wow-test-bot",
                    "exec_sha256": "7".repeat(64),
                    "report_path": report_path.to_string_lossy(),
                    "report_sha256": sha256_bytes(&report_bytes),
                    "account": "TESTBOT2@bot.local",
                    "account_id": 9,
                    "character_guid": 15,
                    "report_validated": true
                });
                fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
            }
        }
        (cpp, cpp_manifest, rust, rust_manifest)
    }

    fn make_derived_flow(root: &Path, flow: &str, raw: &ValidatedRawPair) -> PathBuf {
        let flow_dir = root.join(flow);
        fs::create_dir_all(flow_dir.join("rust")).unwrap();
        if flow == "creature-spell-casting" {
            let committed_fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("flows/creature-spell-casting/fixture");
            fs::create_dir_all(flow_dir.join("fixture")).unwrap();
            for name in ["fixture.json", "cpp-reference.patch"] {
                fs::copy(
                    committed_fixture.join(name),
                    flow_dir.join("fixture").join(name),
                )
                .unwrap();
            }
        }
        fs::write(flow_dir.join("cpp.pkt"), b"filtered-cpp").unwrap();
        fs::write(flow_dir.join("rust/one.meta"), b"derived-meta").unwrap();
        fs::write(flow_dir.join("rust/one.bin"), b"derived-bin").unwrap();
        fs::write(flow_dir.join("expected-divergences.json"), b"[]").unwrap();
        write_derived_lineage(
            flow,
            &flow_dir,
            raw,
            ImportSelection::new(vec![Direction::S2C, Direction::C2S], None, None, &[], true),
        )
        .unwrap();
        flow_dir
    }

    fn required_selection() -> ImportSelection {
        ImportSelection::new(vec![Direction::S2C, Direction::C2S], None, None, &[], true)
    }

    #[test]
    fn raw_manifest_or_artifact_tamper_is_rejected() {
        let root = test_root("raw-tamper");
        let flow = "required-flow";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

        fs::write(&cpp, b"tampered").unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("raw artifact tamper must fail");
        assert!(error.to_string().contains("size") || error.to_string().contains("SHA-256"));
        fs::write(&cpp, b"raw-cpp").unwrap();

        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
        json["artifact"]["tree_sha256"] = serde_json::Value::String("d".repeat(64));
        fs::write(&rust_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("raw manifest hash tamper must fail");
        assert!(error.to_string().contains("tree SHA-256"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn raw_manifest_requires_complete_consistent_process_provenance() {
        let root = test_root("raw-provenance");
        let flow = "required-flow";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let original = fs::read(&cpp_manifest).unwrap();

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json.as_object_mut().unwrap().remove("source_repo_head");
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("missing source HEAD must fail schema validation");
        assert!(format!("{error:#}").contains("parsing raw manifest"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["live_exec_sha256"] = serde_json::Value::String("9".repeat(64));
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("source/live executable mismatch must fail");
        assert!(format!("{error:#}").contains("expected/source/live executable SHA-256"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["harness_worktree_clean"] = serde_json::Value::Bool(false);
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("dirty capture harness must fail");
        assert!(format!("{error:#}").contains("harness worktree must be clean"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json.as_object_mut().unwrap().remove("pm2_exec_sha256");
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("missing PM2 entrypoint hash must fail schema validation");
        assert!(format!("{error:#}").contains("parsing raw manifest"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["created_at"] = serde_json::Value::String("yesterday".to_string());
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("non-RFC3339 timestamp must fail");
        assert!(format!("{error:#}").contains("RFC3339"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["source_derivation"] = reviewed_cpp_source_derivation();
        fs::write(&cpp_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("an unrelated flow must not claim reviewed C++ derivation evidence");
        assert!(format!("{error:#}").contains("reserved for C++ creature-spell-casting"));

        fs::write(&cpp_manifest, &original).unwrap();
        let mut rust_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
        rust_json["source_worktree_dirty"] = serde_json::Value::Bool(true);
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&rust_json).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("dirty Rust source worktree must fail");
        assert!(format!("{error:#}").contains("same clean state"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nested_raw_manifest_is_rejected_instead_of_excluded_from_tree_hash() {
        let root = test_root("nested-raw-manifest");
        let flow = "required-flow";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let nested = rust.join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join(RUST_RAW_MANIFEST_FILE), b"{}").unwrap();

        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("nested raw manifest must fail rather than disappear from hashing");
        assert!(format!("{error:#}").contains("unexpected or nested"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_raw_or_derived_manifest_is_rejected() {
        let root = test_root("missing-manifest");
        let flow = "required-flow";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let raw =
            validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

        fs::remove_file(&cpp_manifest).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("missing C++ raw manifest must fail");
        assert!(error.to_string().contains("C++ raw manifest"));

        let flow_dir = make_derived_flow(&root, flow, &raw);
        fs::remove_file(flow_dir.join(LINEAGE_FILE)).unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("missing derived lineage must fail");
        assert!(error.to_string().contains("reading required lineage"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verify_rejects_retained_manifest_and_output_tamper() {
        let root = test_root("derived-tamper");
        let flow = "required-flow";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let raw =
            validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
        let flow_dir = make_derived_flow(&root, flow, &raw);
        verify_required_lineage(flow, &flow_dir, &required_selection()).unwrap();

        fs::write(flow_dir.join("rust/one.bin"), b"tampered-output").unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("derived output tamper must fail");
        assert!(error.to_string().contains("tree SHA-256"));
        fs::write(flow_dir.join("rust/one.bin"), b"derived-bin").unwrap();

        let retained = flow_dir
            .join(RAW_PROVENANCE_DIR)
            .join(CPP_RAW_MANIFEST_FILE);
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&retained).unwrap()).unwrap();
        json["artifact"]["sha256"] = serde_json::Value::String("e".repeat(64));
        fs::write(&retained, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("retained raw manifest tamper must fail");
        assert!(error.to_string().contains("raw manifest SHA-256"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verify_rejects_lineage_schema_and_hash_tamper() {
        let root = test_root("lineage-tamper");
        let flow = "required-flow";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let raw =
            validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
        let flow_dir = make_derived_flow(&root, flow, &raw);

        let path = flow_dir.join(LINEAGE_FILE);
        let original = fs::read(&path).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["outputs"]["cpp_pkt"]["sha256"] = serde_json::Value::String("f".repeat(64));
        fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("lineage output hash tamper must fail");
        assert!(error.to_string().contains("cpp.pkt"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["selection"]["ignored_opcodes"] = serde_json::json!([{
            "direction": "s2c",
            "opcode": 11732
        }]);
        fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("an extra derived-flow filter must fail the reviewed contract");
        assert!(error.to_string().contains("reviewed import contract"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["unexpected"] = serde_json::Value::Bool(true);
        fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("unknown lineage fields must fail schema validation");
        assert!(error.to_string().contains("parsing required lineage"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_import_before_exchange_leaves_old_flow_untouched() {
        let root = test_root("interrupted-import");
        let target = root.join("required-flow");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("README.md"), b"reviewed metadata").unwrap();
        fs::write(target.join("cpp.pkt"), b"old-complete-flow").unwrap();

        let staging_path;
        {
            let transaction = AtomicFlowImport::prepare(&root, "required-flow").unwrap();
            staging_path = transaction.staging_dir().to_path_buf();
            fs::write(
                transaction.staging_dir().join("cpp.pkt"),
                b"new-partial-flow",
            )
            .unwrap();
            // A signal/error before publish drops the transaction here.
        }

        assert_eq!(
            fs::read(target.join("cpp.pkt")).unwrap(),
            b"old-complete-flow"
        );
        assert_eq!(
            fs::read(target.join("README.md")).unwrap(),
            b"reviewed metadata"
        );
        assert!(!staging_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_rejects_unknown_existing_metadata_instead_of_carrying_it_forward() {
        let root = test_root("unknown-metadata");
        let target = root.join("required-flow");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("README.md"), b"reviewed").unwrap();
        fs::write(target.join("unreviewed.secret"), b"must not propagate").unwrap();

        let error = AtomicFlowImport::prepare(&root, "required-flow")
            .err()
            .expect("unknown metadata must fail closed");
        assert!(error.to_string().contains("unknown non-generated entry"));
        assert_eq!(
            fs::read(target.join("unreviewed.secret")).unwrap(),
            b"must not propagate"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detour_import_copies_only_the_reviewed_fixture_tree() {
        let root = test_root("detour-reviewed-fixture");
        let target = root.join("detour-chase-around-obstacle");
        let fixture = target.join("fixture");
        let mmaps = fixture.join("mmaps");
        let committed_fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("flows/detour-chase-around-obstacle/fixture");
        fs::create_dir_all(&mmaps).unwrap();
        fs::write(target.join("README.md"), b"reviewed").unwrap();
        fs::copy(
            committed_fixture.join("fixture.json"),
            fixture.join("fixture.json"),
        )
        .unwrap();
        fs::copy(
            committed_fixture.join("mmaps/0001.mmap"),
            mmaps.join("0001.mmap"),
        )
        .unwrap();
        fs::copy(
            committed_fixture.join("mmaps/00015026.mmtile"),
            mmaps.join("00015026.mmtile"),
        )
        .unwrap();

        {
            let transaction =
                AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle").unwrap();
            assert_eq!(
                fs::read(transaction.staging_dir().join("fixture/fixture.json")).unwrap(),
                fs::read(committed_fixture.join("fixture.json")).unwrap()
            );
            assert_eq!(
                fs::read(
                    transaction
                        .staging_dir()
                        .join("fixture/mmaps/00015026.mmtile")
                )
                .unwrap(),
                fs::read(committed_fixture.join("mmaps/00015026.mmtile")).unwrap()
            );
        }

        fs::write(mmaps.join("0001.mmap"), b"tampered").unwrap();
        let error = AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle")
            .err()
            .expect("tampered reviewed asset must fail closed");
        assert!(error.to_string().contains("map header differs"));
        fs::copy(
            committed_fixture.join("mmaps/0001.mmap"),
            mmaps.join("0001.mmap"),
        )
        .unwrap();
        fs::write(fixture.join("unreviewed.bin"), b"must not propagate").unwrap();
        let error = AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle")
            .err()
            .expect("unknown fixture entry must fail closed");
        assert!(error.to_string().contains("unreviewed root entry"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creature_spell_import_copies_only_the_reviewed_fixture_derivation() {
        let root = test_root("creature-spell-reviewed-fixture");
        let target = root.join("creature-spell-casting");
        let fixture = target.join("fixture");
        let committed_fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("flows/creature-spell-casting/fixture");
        fs::create_dir_all(&fixture).unwrap();
        fs::write(target.join("README.md"), b"reviewed").unwrap();
        for name in ["fixture.json", "cpp-reference.patch"] {
            fs::copy(committed_fixture.join(name), fixture.join(name)).unwrap();
        }

        {
            let transaction = AtomicFlowImport::prepare(&root, "creature-spell-casting").unwrap();
            assert_eq!(
                fs::read(transaction.staging_dir().join("fixture/fixture.json")).unwrap(),
                fs::read(committed_fixture.join("fixture.json")).unwrap()
            );
            assert_eq!(
                fs::read(
                    transaction
                        .staging_dir()
                        .join("fixture/cpp-reference.patch")
                )
                .unwrap(),
                fs::read(committed_fixture.join("cpp-reference.patch")).unwrap()
            );
        }

        fs::write(fixture.join("fixture.json"), b"tampered").unwrap();
        let error = AtomicFlowImport::prepare(&root, "creature-spell-casting")
            .err()
            .expect("tampered reviewed fixture manifest must fail closed");
        assert!(error.to_string().contains("manifest differs"));
        fs::copy(
            committed_fixture.join("fixture.json"),
            fixture.join("fixture.json"),
        )
        .unwrap();
        fs::write(fixture.join("cpp-reference.patch"), b"tampered").unwrap();
        let error = AtomicFlowImport::prepare(&root, "creature-spell-casting")
            .err()
            .expect("tampered reviewed source patch must fail closed");
        assert!(error.to_string().contains("reference patch differs"));
        fs::copy(
            committed_fixture.join("cpp-reference.patch"),
            fixture.join("cpp-reference.patch"),
        )
        .unwrap();
        fs::write(fixture.join("unreviewed.bin"), b"must not propagate").unwrap();
        let error = AtomicFlowImport::prepare(&root, "creature-spell-casting")
            .err()
            .expect("unknown creature-spell fixture entry must fail closed");
        assert!(error.to_string().contains("unreviewed root entry"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn complete_existing_flow_is_exchanged_as_one_generation() {
        let root = test_root("atomic-exchange");
        let target = root.join("required-flow");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("README.md"), b"reviewed metadata").unwrap();
        fs::write(target.join("cpp.pkt"), b"old").unwrap();

        let transaction = AtomicFlowImport::prepare(&root, "required-flow").unwrap();
        fs::write(transaction.staging_dir().join("cpp.pkt"), b"new").unwrap();
        fs::write(transaction.staging_dir().join(LINEAGE_FILE), b"complete").unwrap();
        transaction.publish().unwrap();

        assert_eq!(fs::read(target.join("cpp.pkt")).unwrap(), b"new");
        assert_eq!(fs::read(target.join(LINEAGE_FILE)).unwrap(), b"complete");
        assert_eq!(
            fs::read(target.join("README.md")).unwrap(),
            b"reviewed metadata"
        );
        assert!(fs::read_dir(&root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("partial")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loot_raw_pair_requires_canonical_guard_bot_report_and_cross_side_identity() {
        let root = test_root("loot-identity");
        let flow = "loot-single-item-claim";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

        let original = fs::read(&rust_manifest).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["fixture_guard"]["enabled"] = serde_json::Value::Bool(false);
        fs::write(&rust_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("disabled fixture guard must fail");
        assert!(format!("{error:#}").contains("fixture_guard.enabled"));

        let mut json: serde_json::Value = serde_json::from_slice(&original).unwrap();
        json["bot_report"]["exec_sha256"] = serde_json::Value::String("8".repeat(64));
        fs::write(&rust_manifest, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("different bot binary identity must fail");
        assert!(format!("{error:#}").contains("different canonical bot identities"));

        fs::write(&rust_manifest, &original).unwrap();
        let report_path = serde_json::from_slice::<serde_json::Value>(&original).unwrap()
            ["bot_report"]["report_path"]
            .as_str()
            .unwrap()
            .to_string();
        fs::write(&report_path, b"{}").unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("bot report tamper must fail");
        assert!(format!("{error:#}").contains("bot report SHA-256"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loot_race_raw_pair_accepts_gameobject_guard_and_rejects_split_runtime_target() {
        let root = test_root("loot-race-identity");
        let flow = "loot-two-session-atomic-race";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        report["results"][1]["loot_race_target_runtime_counter"] = serde_json::Value::from(41);
        let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
        fs::write(&report_path, &report_bytes).unwrap();
        manifest["bot_report"]["report_sha256"] =
            serde_json::Value::String(sha256_bytes(&report_bytes));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("split live target counter must fail");
        assert!(
            format!("{error:#}").contains("one shared target/list"),
            "unexpected error: {error:#}"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creature_spell_raw_pair_requires_reviewed_guard_and_shared_database_snapshot() {
        let root = test_root("creature-spell-identity");
        let flow = "creature-spell-casting";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let raw =
            validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
        let flow_dir = make_derived_flow(&root, flow, &raw);
        verify_required_lineage(flow, &flow_dir, &required_selection()).unwrap();
        assert!(
            flow_dir
                .join(RAW_PROVENANCE_DIR)
                .join(CPP_BOT_REPORT_FILE)
                .is_file()
                && flow_dir
                    .join(RAW_PROVENANCE_DIR)
                    .join(RUST_BOT_REPORT_FILE)
                    .is_file(),
            "derived creature-spell lineage did not retain both bot reports"
        );
        let lineage_path = flow_dir.join(LINEAGE_FILE);
        let lineage_original = fs::read(&lineage_path).unwrap();
        let lineage_json: serde_json::Value = serde_json::from_slice(&lineage_original).unwrap();
        assert_eq!(
            lineage_json["sources"]["cpp"]["source_derivation"],
            reviewed_cpp_source_derivation(),
            "derived lineage did not retain the exact reviewed C++ derivation"
        );
        assert!(
            lineage_json["sources"]["rust"]
                .get("source_derivation")
                .is_none(),
            "derived lineage invented C++ derivation evidence for Rust"
        );

        let mut tampered_lineage = lineage_json.clone();
        tampered_lineage["sources"]["cpp"]["source_derivation"]["patch_sha256"] =
            serde_json::Value::String("0".repeat(64));
        fs::write(
            &lineage_path,
            serde_json::to_vec_pretty(&tampered_lineage).unwrap(),
        )
        .unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("tampered derived source patch identity must fail");
        assert!(format!("{error:#}").contains("reviewed patch"));

        let mut missing_lineage = lineage_json;
        missing_lineage["sources"]["cpp"]
            .as_object_mut()
            .unwrap()
            .remove("source_derivation");
        fs::write(
            &lineage_path,
            serde_json::to_vec_pretty(&missing_lineage).unwrap(),
        )
        .unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("missing derived source derivation must fail");
        assert!(format!("{error:#}").contains("source_derivation is missing"));
        fs::write(&lineage_path, &lineage_original).unwrap();

        let cpp_original = fs::read(&cpp_manifest).unwrap();
        let mut missing_derivation: serde_json::Value =
            serde_json::from_slice(&cpp_original).unwrap();
        missing_derivation
            .as_object_mut()
            .unwrap()
            .remove("source_derivation");
        fs::write(
            &cpp_manifest,
            serde_json::to_vec_pretty(&missing_derivation).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("missing C++ source derivation must fail");
        assert!(format!("{error:#}").contains("requires source_derivation evidence"));

        for (field, value) in [
            (
                "contract",
                serde_json::Value::String("wrong-contract".into()),
            ),
            (
                "remote_url",
                serde_json::Value::String("https://example.invalid/reference.git".into()),
            ),
            (
                "remote_ref",
                serde_json::Value::String("refs/remotes/origin/other".into()),
            ),
            ("base_head", serde_json::Value::String("0".repeat(40))),
            ("base_tree", serde_json::Value::String("0".repeat(40))),
            ("patched_head", serde_json::Value::String("0".repeat(40))),
            ("patched_tree", serde_json::Value::String("0".repeat(40))),
            (
                "patch_path",
                serde_json::Value::String("fixture/other.patch".into()),
            ),
            ("patch_sha256", serde_json::Value::String("0".repeat(64))),
            (
                "changed_paths",
                serde_json::json!([
                    CREATURE_SPELL_CPP_CHANGED_PATH,
                    "src/server/game/DataStores/Unreviewed.cpp"
                ]),
            ),
        ] {
            let mut changed: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
            changed["source_derivation"][field] = value;
            fs::write(&cpp_manifest, serde_json::to_vec_pretty(&changed).unwrap()).unwrap();
            let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
                .expect_err("mutated reviewed C++ source derivation must fail");
            assert!(
                format!("{error:#}").contains("source_derivation"),
                "unexpected {field} mutation error: {error:#}"
            );
        }

        let rust_original_with_derivation = fs::read(&rust_manifest).unwrap();
        let mut rust_derivation: serde_json::Value =
            serde_json::from_slice(&rust_original_with_derivation).unwrap();
        rust_derivation["source_derivation"] = reviewed_cpp_source_derivation();
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&rust_derivation).unwrap(),
        )
        .unwrap();
        fs::write(&cpp_manifest, &cpp_original).unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("Rust must not claim the C++ source derivation");
        assert!(format!("{error:#}").contains("Rust raw manifests must not contain"));
        fs::write(&rust_manifest, rust_original_with_derivation).unwrap();

        let mut dirty_cpp: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
        dirty_cpp["source_worktree_dirty"] = serde_json::Value::Bool(true);
        fs::write(
            &cpp_manifest,
            serde_json::to_vec_pretty(&dirty_cpp).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("dirty creature-spell C++ source worktree must fail");
        assert!(
            format!("{error:#}")
                .contains("creature-spell-casting C++ source worktree must be clean"),
            "unexpected error: {error:#}"
        );

        let mut wrong_revision: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
        wrong_revision["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
        fs::write(
            &cpp_manifest,
            serde_json::to_vec_pretty(&wrong_revision).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("unrelated creature-spell C++ executable revision must fail");
        assert!(
            format!("{error:#}").contains(
                "creature-spell-casting C++ embedded executable revision must equal source_repo_head"
            ),
            "unexpected error: {error:#}"
        );
        fs::write(&cpp_manifest, &cpp_original).unwrap();

        let original = fs::read(&rust_manifest).unwrap();

        let mut missing_revision: serde_json::Value = serde_json::from_slice(&original).unwrap();
        missing_revision["source_exec_revision"] = serde_json::Value::Null;
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&missing_revision).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("missing creature-spell Rust executable revision must fail");
        assert!(
            format!("{error:#}").contains(
                "creature-spell-casting Rust embedded executable revision must equal source_repo_head"
            ),
            "unexpected error: {error:#}"
        );

        let mut wrong_revision: serde_json::Value = serde_json::from_slice(&original).unwrap();
        wrong_revision["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&wrong_revision).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("unrelated creature-spell Rust executable revision must fail");
        assert!(
            format!("{error:#}").contains(
                "creature-spell-casting Rust embedded executable revision must equal source_repo_head"
            ),
            "unexpected error: {error:#}"
        );
        fs::write(&rust_manifest, &original).unwrap();

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest.as_object_mut().unwrap().remove("fixture_guard");
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("missing creature-spell fixture guard must fail");
        assert!(
            format!("{error:#}").contains("requires fixture_guard evidence"),
            "unexpected error: {error:#}"
        );

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["fixture_manifest_sha256"] =
            serde_json::Value::String("5".repeat(64));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("unreviewed creature-spell fixture manifest must fail");
        assert!(
            format!("{error:#}").contains("exact reviewed manifest"),
            "unexpected error: {error:#}"
        );

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["database_snapshot_sha256"] =
            serde_json::Value::String("3".repeat(64));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("different creature-spell database snapshots must fail");
        assert!(
            format!("{error:#}").contains("creature-spell fixture identities differ"),
            "unexpected error: {error:#}"
        );

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["creature_entry"] = serde_json::Value::from(22_379);
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("wrong creature-spell world identity must fail");
        assert!(
            format!("{error:#}").contains("Cabal Interrogator"),
            "unexpected error: {error:#}"
        );

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["cleanup_verified"] = serde_json::Value::Bool(false);
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("unverified creature-spell cleanup must fail");
        assert!(
            format!("{error:#}").contains("cleanup was not verified"),
            "unexpected error: {error:#}"
        );

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["bot_report"] = serde_json::Value::Null;
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("creature-spell fixture must require bot evidence");
        assert!(
            format!("{error:#}").contains("requires bot_report evidence"),
            "unexpected error: {error:#}"
        );

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["account_id"] = serde_json::Value::from(10);
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("noncanonical creature-spell bot identity must fail");
        assert!(
            format!("{error:#}").contains("canonical TESTBOT2 identity"),
            "unexpected error: {error:#}"
        );

        let manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let report_original = fs::read(&report_path).unwrap();
        let mut report: serde_json::Value = serde_json::from_slice(&report_original).unwrap();
        report["results"][0]["creature_spell_go_hit_target_count"] = serde_json::Value::from(0);
        let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
        fs::write(&report_path, &report_bytes).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["bot_report"]["report_sha256"] =
            serde_json::Value::String(sha256_bytes(&report_bytes));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("creature-spell report without one hit must fail");
        assert!(
            format!("{error:#}").contains("canonical successful creature-spell window"),
            "unexpected error: {error:#}"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detour_raw_pair_allows_private_evidence_to_differ_but_pins_shared_fixture_and_window() {
        let root = test_root("detour-identity");
        let flow = "detour-chase-around-obstacle";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

        let cpp_original = fs::read(&cpp_manifest).unwrap();
        let mut cpp_dirty: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
        cpp_dirty["source_worktree_dirty"] = serde_json::Value::Bool(true);
        fs::write(
            &cpp_manifest,
            serde_json::to_vec_pretty(&cpp_dirty).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("dirty legacy C++ detour provenance must fail");
        assert!(
            format!("{error:#}").contains("detour C++ source worktree must be clean"),
            "unexpected error: {error:#}"
        );
        fs::write(&cpp_manifest, cpp_original).unwrap();

        let cpp_original = fs::read(&cpp_manifest).unwrap();
        let mut cpp_wrong_binary: serde_json::Value =
            serde_json::from_slice(&cpp_original).unwrap();
        cpp_wrong_binary["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
        fs::write(
            &cpp_manifest,
            serde_json::to_vec_pretty(&cpp_wrong_binary).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("C++ binary/source revision mismatch must fail");
        assert!(
            format!("{error:#}")
                .contains("embedded executable revision must equal source_repo_head"),
            "unexpected error: {error:#}"
        );
        fs::write(&cpp_manifest, cpp_original).unwrap();

        let rust_original = fs::read(&rust_manifest).unwrap();
        let mut rust_wrong_binary: serde_json::Value =
            serde_json::from_slice(&rust_original).unwrap();
        rust_wrong_binary["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&rust_wrong_binary).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("Rust binary/source revision mismatch must fail");
        assert!(
            format!("{error:#}")
                .contains("Rust embedded executable revision must equal source_repo_head"),
            "unexpected error: {error:#}"
        );
        fs::write(&rust_manifest, rust_original).unwrap();

        let original = fs::read(&rust_manifest).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["fixture_manifest_sha256"] =
            serde_json::Value::String("5".repeat(64));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("different committed fixture identity must fail");
        assert!(format!("{error:#}").contains("exact reviewed detour manifest"));

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        manifest["fixture_guard"]["database_snapshot_sha256"] =
            serde_json::Value::String("3".repeat(64));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("different initial database snapshots must fail");
        assert!(format!("{error:#}").contains("detour fixture identities differ"));

        let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        report["results"][0]["detour_chase_window_target_moves"] = serde_json::Value::from(2);
        let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
        fs::write(&report_path, &report_bytes).unwrap();
        manifest["bot_report"]["report_sha256"] =
            serde_json::Value::String(sha256_bytes(&report_bytes));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("a non-isolated movement window must fail");
        assert!(format!("{error:#}").contains("canonical isolated detour-chase window"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detour_bot_reports_are_cryptographically_bound_to_each_selected_raw_side() {
        fn bind_report(manifest_path: &Path, heartbeat: &[u8], movement: &[u8]) {
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
            let report_path =
                PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
            let mut report: serde_json::Value =
                serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
            report["results"][0]["detour_chase_heartbeat_sha256"] =
                serde_json::Value::String(sha256_bytes(heartbeat));
            report["results"][0]["detour_chase_monster_move_sha256"] =
                serde_json::Value::String(sha256_bytes(movement));
            report["results"][0]["detour_chase_monster_move_bytes"] =
                serde_json::Value::from(movement.len() as u64);
            let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
            fs::write(&report_path, &report_bytes).unwrap();
            manifest["bot_report"]["report_sha256"] =
                serde_json::Value::String(sha256_bytes(&report_bytes));
            fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        }

        fn selected_capture(source: &str, heartbeat: &[u8], movement: &[u8]) -> Capture {
            Capture::new(
                source,
                vec![
                    CapturedPacket {
                        direction: Direction::C2S,
                        connection_id: 1,
                        opcode: 0x3A10,
                        body: heartbeat.to_vec(),
                    },
                    CapturedPacket {
                        direction: Direction::S2C,
                        connection_id: 1,
                        opcode: 0x2DD4,
                        body: movement.to_vec(),
                    },
                    CapturedPacket {
                        direction: Direction::C2S,
                        connection_id: 1,
                        opcode: 0x3769,
                        body: b"fence".to_vec(),
                    },
                ],
            )
        }

        let root = test_root("detour-report-packet-binding");
        let flow = "detour-chase-around-obstacle";
        let (cpp_path, cpp_manifest, rust_path, rust_manifest) = make_raw_pair(&root, flow);
        let cpp_heartbeat = b"cpp-heartbeat";
        let cpp_movement = b"cpp-movement";
        let rust_heartbeat = b"rust-heartbeat";
        let rust_movement = b"rust-movement";
        bind_report(&cpp_manifest, cpp_heartbeat, cpp_movement);
        bind_report(&rust_manifest, rust_heartbeat, rust_movement);
        let raw = validate_raw_pair(
            flow,
            &cpp_path,
            &cpp_manifest,
            &rust_path,
            &rust_manifest,
            true,
        )
        .unwrap();
        let cpp = selected_capture("cpp", cpp_heartbeat, cpp_movement);
        let rust = selected_capture("rust", rust_heartbeat, rust_movement);
        validate_bot_report_capture_binding(flow, &raw, &cpp, &rust).unwrap();

        let mut mismatched = rust.clone();
        mismatched.packets[1].body.push(0);
        let error = validate_bot_report_capture_binding(flow, &raw, &cpp, &mismatched)
            .expect_err("a report from a different execution must fail");
        assert!(format!("{error:#}").contains("does not match selected RAW"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creature_spell_bot_reports_are_cryptographically_bound_to_selected_start_go() {
        #[derive(Default)]
        struct BitWriter {
            bytes: Vec<u8>,
            current: u8,
            used: u8,
        }

        impl BitWriter {
            fn bits(&mut self, value: u32, width: u8) {
                for shift in (0..width).rev() {
                    self.current |= (((value >> shift) & 1) as u8) << (7 - self.used);
                    self.used += 1;
                    if self.used == 8 {
                        self.bytes.push(self.current);
                        self.current = 0;
                        self.used = 0;
                    }
                }
            }

            fn finish(mut self) -> Vec<u8> {
                if self.used != 0 {
                    self.bytes.push(self.current);
                }
                self.bytes
            }
        }

        fn guid(
            high_type: u8,
            subtype: u8,
            realm: u16,
            map: u16,
            entry: u32,
            counter: u64,
        ) -> crate::semantic::ExactObjectGuid {
            crate::semantic::ExactObjectGuid {
                low: counter & 0x0000_00FF_FFFF_FFFF,
                high: (u64::from(high_type & 0x3F) << 58)
                    | (u64::from(realm & 0x1FFF) << 42)
                    | (u64::from(map & 0x1FFF) << 29)
                    | (u64::from(entry & 0x7F_FFFF) << 6)
                    | u64::from(subtype & 0x3F),
            }
        }

        fn push_guid(out: &mut Vec<u8>, guid: crate::semantic::ExactObjectGuid) {
            let low = guid.low.to_le_bytes();
            let high = guid.high.to_le_bytes();
            let low_mask = low.iter().enumerate().fold(0u8, |mask, (index, byte)| {
                mask | (u8::from(*byte != 0) << index)
            });
            let high_mask = high.iter().enumerate().fold(0u8, |mask, (index, byte)| {
                mask | (u8::from(*byte != 0) << index)
            });
            out.push(low_mask);
            out.push(high_mask);
            out.extend(low.into_iter().filter(|byte| *byte != 0));
            out.extend(high.into_iter().filter(|byte| *byte != 0));
        }

        fn spell_body(
            caster: crate::semantic::ExactObjectGuid,
            cast_id: crate::semantic::ExactObjectGuid,
            victim: crate::semantic::ExactObjectGuid,
            spell_go: bool,
        ) -> Vec<u8> {
            let mut out = Vec::new();
            for value in [
                caster,
                caster,
                cast_id,
                crate::semantic::ExactObjectGuid { low: 0, high: 0 },
            ] {
                push_guid(&mut out, value);
            }
            out.extend(15_691_i32.to_le_bytes());
            out.extend(244_493_i32.to_le_bytes());
            out.extend(if spell_go { 0x100_u32 } else { 2_u32 }.to_le_bytes());
            out.extend(0_u32.to_le_bytes());
            out.extend(if spell_go { 123_u32 } else { 0_u32 }.to_le_bytes());
            out.extend(0_u32.to_le_bytes());
            out.extend(0_f32.to_bits().to_le_bytes());
            out.push(0);
            out.extend(0_u32.to_le_bytes());
            out.extend(0_u32.to_le_bytes());
            out.extend(0_u32.to_le_bytes());
            out.push(0);
            push_guid(
                &mut out,
                crate::semantic::ExactObjectGuid { low: 0, high: 0 },
            );

            let mut counts = BitWriter::default();
            counts.bits(u32::from(spell_go), 16);
            counts.bits(0, 16);
            counts.bits(0, 16);
            counts.bits(0, 9);
            counts.bits(0, 1);
            counts.bits(0, 16);
            counts.bits(0, 1);
            counts.bits(0, 1);
            out.extend(counts.finish());

            let mut target = BitWriter::default();
            target.bits(2, 28);
            target.bits(0, 1);
            target.bits(0, 1);
            target.bits(0, 1);
            target.bits(0, 1);
            target.bits(0, 7);
            out.extend(target.finish());
            push_guid(&mut out, victim);
            push_guid(
                &mut out,
                crate::semantic::ExactObjectGuid { low: 0, high: 0 },
            );
            if spell_go {
                push_guid(&mut out, victim);
                out.push(0); // basic SpellGo combat-log bit plus canonical padding
            }
            out
        }

        fn bind_report(
            manifest_path: &Path,
            start: &[u8],
            go: &[u8],
            caster: crate::semantic::ExactObjectGuid,
            cast_id: crate::semantic::ExactObjectGuid,
            victim: crate::semantic::ExactObjectGuid,
        ) {
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
            let report_path =
                PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
            let mut report: serde_json::Value =
                serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
            let result = &mut report["results"][0];
            result["creature_spell_start_body_sha256"] =
                serde_json::Value::String(sha256_bytes(start));
            result["creature_spell_start_body_bytes"] = serde_json::Value::from(start.len() as u64);
            result["creature_spell_go_body_sha256"] = serde_json::Value::String(sha256_bytes(go));
            result["creature_spell_go_body_bytes"] = serde_json::Value::from(go.len() as u64);
            result["creature_spell_cast_id_low"] = serde_json::Value::from(cast_id.low);
            result["creature_spell_cast_id_high"] = serde_json::Value::from(cast_id.high);
            result["creature_spell_caster_guid_low"] = serde_json::Value::from(caster.low);
            result["creature_spell_caster_guid_high"] = serde_json::Value::from(caster.high);
            result["creature_spell_victim_guid_low"] = serde_json::Value::from(victim.low);
            result["creature_spell_victim_guid_high"] = serde_json::Value::from(victim.high);
            result["creature_spell_target_runtime_counter"] =
                serde_json::Value::from(caster.low & 0x0000_00FF_FFFF_FFFF);
            let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
            fs::write(&report_path, &report_bytes).unwrap();
            manifest["bot_report"]["report_sha256"] =
                serde_json::Value::String(sha256_bytes(&report_bytes));
            fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        }

        let caster = guid(8, 0, 1, 530, 22_378, 78_686);
        let cast_id = guid(47, 3, 1, 530, 15_691, 44);
        let victim = guid(2, 0, 1, 0, 0, 15);
        let start = spell_body(caster, cast_id, victim, false);
        let go = spell_body(caster, cast_id, victim, true);
        let selected = |source: &str| {
            Capture::new(
                source,
                vec![
                    CapturedPacket {
                        direction: Direction::S2C,
                        connection_id: 1,
                        opcode: SMSG_SPELL_START,
                        body: start.clone(),
                    },
                    CapturedPacket {
                        direction: Direction::S2C,
                        connection_id: 1,
                        opcode: SMSG_SPELL_GO,
                        body: go.clone(),
                    },
                ],
            )
        };
        crate::semantic::validate_creature_spell_casting_capture(&selected("shape")).unwrap();

        let root = test_root("creature-spell-report-packet-binding");
        let flow = "creature-spell-casting";
        let (cpp_path, cpp_manifest, rust_path, rust_manifest) = make_raw_pair(&root, flow);
        bind_report(&cpp_manifest, &start, &go, caster, cast_id, victim);
        bind_report(&rust_manifest, &start, &go, caster, cast_id, victim);
        let raw = validate_raw_pair(
            flow,
            &cpp_path,
            &cpp_manifest,
            &rust_path,
            &rust_manifest,
            true,
        )
        .unwrap();
        let cpp = selected("cpp");
        let rust = selected("rust");
        validate_bot_report_capture_binding(flow, &raw, &cpp, &rust).unwrap();

        let mut mismatched = rust.clone();
        mismatched.packets[1].body.push(0);
        let error = validate_bot_report_capture_binding(flow, &raw, &cpp, &mismatched)
            .expect_err("a creature-spell report from a different execution must fail");
        assert!(format!("{error:#}").contains("does not match selected RAW"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn vendor_raw_pair_requires_exact_bot_report_and_retains_both_reports() {
        let root = test_root("vendor-report");
        let flow = "vendor-extended-cost-purchase";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let raw =
            validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
        let flow_dir = make_derived_flow(&root, flow, &raw);
        assert!(
            flow_dir
                .join(RAW_PROVENANCE_DIR)
                .join(CPP_BOT_REPORT_FILE)
                .is_file()
        );
        assert!(
            flow_dir
                .join(RAW_PROVENANCE_DIR)
                .join(RUST_BOT_REPORT_FILE)
                .is_file()
        );

        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&rust_manifest).unwrap()).unwrap();
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        report["results"][0]["vendor_relogin_verified"] = serde_json::Value::Bool(false);
        let report_bytes = serde_json::to_vec_pretty(&report).unwrap();
        fs::write(&report_path, &report_bytes).unwrap();
        manifest["bot_report"]["report_sha256"] =
            serde_json::Value::String(sha256_bytes(&report_bytes));
        fs::write(
            &rust_manifest,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
            .expect_err("vendor report without relog proof must fail");
        assert!(format!("{error:#}").contains("canonical successful vendor flow"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn derived_loot_lineage_retains_and_revalidates_bot_reports() {
        let root = test_root("loot-report-retention");
        let flow = "loot-single-item-claim";
        let (cpp, cpp_manifest, rust, rust_manifest) = make_raw_pair(&root, flow);
        let raw =
            validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
        let flow_dir = make_derived_flow(&root, flow, &raw);
        verify_required_lineage(flow, &flow_dir, &required_selection()).unwrap();

        fs::write(
            flow_dir.join(RAW_PROVENANCE_DIR).join(RUST_BOT_REPORT_FILE),
            b"{}",
        )
        .unwrap();
        let error = verify_required_lineage(flow, &flow_dir, &required_selection())
            .expect_err("retained bot report tamper must fail");
        assert!(error.to_string().contains("bot report SHA-256"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn new_flow_publication_is_atomic_noreplace_under_target_race() {
        let root = test_root("atomic-noreplace");
        let transaction = AtomicFlowImport::prepare(&root, "new-flow").unwrap();
        fs::write(transaction.staging_dir().join("cpp.pkt"), b"candidate").unwrap();

        let target = root.join("new-flow");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("sentinel"), b"concurrent owner").unwrap();
        let error = transaction
            .publish()
            .expect_err("concurrent target must never be replaced");
        assert!(format!("{error:#}").contains("without replacement"));
        assert_eq!(
            fs::read(target.join("sentinel")).unwrap(),
            b"concurrent owner"
        );
        assert!(!target.join("cpp.pkt").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
