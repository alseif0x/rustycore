//! Capture lineage tracking state definitions, part 1 of 4.
//!
//! Separated from the lineage.rs root under #658. Behaviour is preserved.

use super::*;

/// Completion marker for one fully derived capture flow.
pub const LINEAGE_FILE: &str = "capture-lineage.json";

/// Exact raw manifests retained with the derived fixture.
pub const RAW_PROVENANCE_DIR: &str = "capture-provenance";

pub(super) const CPP_RAW_MANIFEST_FILE: &str = "cpp.capture-manifest.json";

pub(super) const RUST_RAW_MANIFEST_FILE: &str = "rust.capture-manifest.json";

pub(super) const CPP_BOT_REPORT_FILE: &str = "cpp.bot-report.json";

pub(super) const RUST_BOT_REPORT_FILE: &str = "rust.bot-report.json";

pub(super) const SHA256_HEX_LEN: usize = 64;

pub(super) const DETOUR_FIXTURE_MANIFEST_SHA256: &str =
    "3a6c2aa6081974ef9cf13b8f63f739c402b18799f9833f80819f5e7e0de8d013";

pub(super) const DETOUR_FIXTURE_MAP_SHA256: &str =
    "3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a";

pub(super) const DETOUR_FIXTURE_TILE_SHA256: &str =
    "693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47";

pub(super) const CREATURE_SPELL_FIXTURE_MANIFEST_SHA256: &str =
    "3cef5dd6201c88fc85c1c2cb767fec27cd11921ec7ecdc2c7705379fd54e356d";

pub(super) const CREATURE_SPELL_FIXTURE_CONTRACT: &str = "creature-spell-casting-shell-fixture-v2";

pub(super) const CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT: &str =
    "creature-spell-casting-cpp-source-patch-v1";

pub(super) const CREATURE_SPELL_CPP_REMOTE_URL: &str =
    "https://github.com/alseif0x/TrinityCoreLegacyTest.git";

pub(super) const CREATURE_SPELL_CPP_REMOTE_REF: &str = "refs/remotes/origin/3.4.3";

pub(super) const CREATURE_SPELL_CPP_BASE_HEAD: &str = "a5f8da2ebf5424bf0450ca4e08843ecbf72577bd";

pub(super) const CREATURE_SPELL_CPP_BASE_TREE: &str = "bb5c4746be7f9944b1a3f7a1eec5ea88d62fff67";

pub(super) const CREATURE_SPELL_CPP_PATCHED_HEAD: &str = "8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c";

pub(super) const CREATURE_SPELL_CPP_PATCHED_TREE: &str = "228e91ed36886593c85fb601d00a9f8eb0702137";

pub(super) const CREATURE_SPELL_CPP_PATCH_PATH: &str =
    "crates/capture-diff/flows/creature-spell-casting/fixture/cpp-reference.patch";

pub(super) const CREATURE_SPELL_CPP_PATCH_SHA256: &str =
    "ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262";

pub(super) const CREATURE_SPELL_CPP_CHANGED_PATH: &str = "src/server/game/DataStores/DB2Stores.cpp";

pub(super) const CREATURE_SPELL_PLAYER_GUID_HIGH: u64 = 0x0800_0400_0000_0000;

pub(super) const LINEAGE_VERSION: u32 = 3;

pub(super) const RAW_MANIFEST_VERSION: u32 = 3;

pub(super) static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum RawSide {
    Cpp,
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawArtifact {
    pub(super) path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) packet_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) tree_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SyntheticMmapEvidence {
    pub(super) path: String,
    pub(super) size: u64,
    pub(super) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LinkedReadOnlyDataEvidence {
    pub(super) name: String,
    pub(super) target_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FixtureGuardEvidence {
    pub(super) enabled: bool,
    pub(super) contract: String,
    pub(super) account: String,
    pub(super) account_id: u32,
    pub(super) character_guid: u64,
    pub(super) peer_account: String,
    pub(super) peer_account_id: u32,
    pub(super) peer_character_guid: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) creature_entry: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) creature_spawn_guid: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) gameobject_entry: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) gameobject_spawn_guid: Option<u64>,
    pub(super) item_entry: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) character_account_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) normal_data_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) private_data_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) private_data_dir_removed_before_normal_runtime: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) fixture_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) fixture_manifest_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) synthetic_mmaps: Option<Vec<SyntheticMmapEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) linked_read_only_data: Option<Vec<LinkedReadOnlyDataEvidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) journal_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) database_snapshot_sha256: Option<String>,
    pub(super) cleanup_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BotReportEvidence {
    pub(super) contract: String,
    pub(super) exec_path: String,
    pub(super) exec_sha256: String,
    pub(super) report_path: String,
    pub(super) report_sha256: String,
    pub(super) account: String,
    pub(super) account_id: u32,
    pub(super) character_guid: u64,
    pub(super) report_validated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceDerivationEvidence {
    pub(super) contract: String,
    pub(super) remote_url: String,
    pub(super) remote_ref: String,
    pub(super) base_head: String,
    pub(super) base_tree: String,
    pub(super) patched_head: String,
    pub(super) patched_tree: String,
    pub(super) patch_path: String,
    pub(super) patch_sha256: String,
    pub(super) changed_paths: Vec<String>,
}

/// Schema emitted by `capture-cpp.sh` and `capture-rust.sh`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawCaptureManifest {
    pub(super) version: u32,
    pub(super) flow: String,
    pub(super) side: RawSide,
    pub(super) completed: bool,
    pub(super) created_at: String,
    pub(super) harness_repo_head: String,
    pub(super) source_repo_head: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) source_exec_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) source_derivation: Option<SourceDerivationEvidence>,
    pub(super) harness_worktree_clean: bool,
    pub(super) harness_worktree_state_sha256: String,
    pub(super) source_worktree_dirty: bool,
    pub(super) source_worktree_state_sha256: String,
    pub(super) worktree_state_algorithm: String,
    pub(super) expected_exec_path: String,
    pub(super) expected_exec_sha256: String,
    pub(super) source_exec_path: String,
    pub(super) source_exec_sha256: String,
    pub(super) live_exec_path: String,
    pub(super) live_exec_sha256: String,
    pub(super) executable_pin_enforced: bool,
    pub(super) pm2_entry_pid: u32,
    pub(super) pm2_entry_starttime: u64,
    pub(super) pm2_exec_path: String,
    pub(super) pm2_exec_sha256: String,
    pub(super) pm2_profile_redacted_sha256: String,
    pub(super) listener_runtime_pid: u32,
    pub(super) listener_runtime_starttime: u64,
    pub(super) listener_relationship_verified: bool,
    pub(super) restart_count: u64,
    pub(super) effective_config_path: String,
    pub(super) effective_config_redacted_sha256: String,
    pub(super) effective_config_algorithm: String,
    pub(super) runtime_cleanup_verified: bool,
    pub(super) normal_runtime_restored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) fixture_guard: Option<FixtureGuardEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) bot_report: Option<BotReportEvidence>,
    pub(super) artifact: RawArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedRawSide {
    pub(super) manifest_bytes: Vec<u8>,
    pub(super) manifest_sha256: String,
    pub(super) manifest: RawCaptureManifest,
    pub(super) bot_report_bytes: Option<Vec<u8>>,
}

/// A pair of raw manifests whose declared artifacts matched the supplied raw
/// capture bytes at import time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRawPair {
    pub(super) cpp: ValidatedRawSide,
    pub(super) rust: ValidatedRawSide,
}

/// One boundary recorded in the derived import contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineageBoundary {
    pub(super) direction: Option<Direction>,
    pub(super) opcode: u16,
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
    pub(super) directions: Vec<Direction>,
    pub(super) from_opcode: Option<LineageBoundary>,
    pub(super) until_opcode: Option<LineageBoundary>,
    pub(super) ignored_opcodes: Vec<LineageBoundary>,
    pub(super) strict: bool,
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
pub(super) struct SourceLineage {
    pub(super) manifest_path: String,
    pub(super) manifest_sha256: String,
    pub(super) raw_artifact_sha256: String,
    pub(super) raw_artifact_size: Option<u64>,
    pub(super) raw_packet_count: Option<u64>,
    pub(super) harness_repo_head: String,
    pub(super) source_repo_head: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) source_derivation: Option<SourceDerivationEvidence>,
    pub(super) harness_worktree_clean: bool,
    pub(super) harness_worktree_state_sha256: String,
    pub(super) source_worktree_dirty: bool,
    pub(super) source_worktree_state_sha256: String,
    pub(super) worktree_state_algorithm: String,
    pub(super) expected_exec_path: String,
    pub(super) expected_exec_sha256: String,
    pub(super) source_exec_path: String,
    pub(super) source_exec_sha256: String,
    pub(super) live_exec_path: String,
    pub(super) live_exec_sha256: String,
    pub(super) executable_pin_enforced: bool,
    pub(super) pm2_entry_pid: u32,
    pub(super) pm2_entry_starttime: u64,
    pub(super) pm2_exec_path: String,
    pub(super) pm2_exec_sha256: String,
    pub(super) pm2_profile_redacted_sha256: String,
    pub(super) listener_runtime_pid: u32,
    pub(super) listener_runtime_starttime: u64,
    pub(super) listener_relationship_verified: bool,
    pub(super) restart_count: u64,
    pub(super) effective_config_path: String,
    pub(super) effective_config_redacted_sha256: String,
    pub(super) effective_config_algorithm: String,
    pub(super) runtime_cleanup_verified: bool,
    pub(super) normal_runtime_restored: bool,
    pub(super) fixture_guard: Option<FixtureGuardEvidence>,
    pub(super) bot_report: Option<BotReportEvidence>,
    pub(super) retained_bot_report_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourcePairLineage {
    pub(super) cpp: SourceLineage,
    pub(super) rust: SourceLineage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileLineage {
    pub(super) path: String,
    pub(super) size: u64,
    pub(super) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeLineage {
    pub(super) path: String,
    pub(super) file_count: u64,
    pub(super) packet_count: u64,
    pub(super) tree_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DerivedOutputs {
    pub(super) cpp_pkt: FileLineage,
    pub(super) rust: TreeLineage,
    pub(super) expected_divergences: FileLineage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DerivedLineage {
    pub(super) version: u32,
    pub(super) flow: String,
    pub(super) completed: bool,
    pub(super) sources: SourcePairLineage,
    pub(super) selection: ImportSelection,
    pub(super) outputs: DerivedOutputs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TreeDigest {
    pub(super) sha256: String,
    pub(super) file_count: u64,
    pub(super) packet_count: u64,
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

pub(super) fn validate_creature_spell_report_capture_binding(
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

pub(super) fn validate_detour_report_capture_binding(
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

pub(super) fn read_and_validate_raw_manifest(
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

pub(super) fn validate_raw_manifest_schema(
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

pub(super) fn validate_source_derivation_schema(
    derivation: &SourceDerivationEvidence,
) -> Result<()> {
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

pub(super) fn is_canonical_relative_path(path: &str) -> bool {
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

pub(super) fn validate_canonical_creature_spell_source_derivation(
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
