//! Capture lineage tracking state definitions, part 3 of 4.
//!
//! Separated from the lineage.rs root under #658. Behaviour is preserved.

use super::*;

pub(super) fn validate_loot_race_bot_report_json(
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

pub(super) fn validate_cpp_artifact(manifest: &RawCaptureManifest, path: &Path) -> Result<()> {
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

pub(super) fn validate_rust_artifact(
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

pub(super) fn build_lineage(
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

pub(super) fn source_lineage(
    raw: &ValidatedRawSide,
    manifest_path: String,
) -> Result<SourceLineage> {
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

pub(super) fn verify_retained_source(
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

pub(super) fn file_lineage(root: &Path, relative: &str) -> Result<FileLineage> {
    let bytes = read_regular_file(&root.join(relative))?;
    Ok(FileLineage {
        path: relative.to_string(),
        size: u64::try_from(bytes.len()).context("derived file size does not fit u64")?,
        sha256: sha256_bytes(&bytes),
    })
}

pub(super) fn verify_file_lineage(
    root: &Path,
    expected: &FileLineage,
    canonical: &str,
) -> Result<()> {
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

pub(super) fn validate_sha256(value: &str, label: &str) -> Result<()> {
    ensure!(
        value.len() == SHA256_HEX_LEN
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "{label} must be exactly 64 lowercase hexadecimal characters"
    );
    Ok(())
}

pub(super) fn valid_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn validate_utc_timestamp(value: &str) -> Result<()> {
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

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn read_regular_file(path: &Path) -> Result<Vec<u8>> {
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
pub(super) fn open_read_no_follow(path: &Path) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| format!("opening {} without following symlinks", path.display()))
}

#[cfg(not(unix))]
pub(super) fn open_read_no_follow(path: &Path) -> Result<File> {
    File::open(path).with_context(|| format!("opening {}", path.display()))
}

pub(super) fn digest_tree(root: &Path, excluded_manifest: Option<&Path>) -> Result<TreeDigest> {
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

pub(super) fn collect_tree_files(
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
pub(super) fn relative_path_bytes(root: &Path, path: &Path) -> Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt as _;
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?
        .as_os_str()
        .as_bytes()
        .to_vec())
}

#[cfg(not(unix))]
pub(super) fn relative_path_bytes(root: &Path, path: &Path) -> Result<Vec<u8>> {
    Ok(path
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", path.display(), root.display()))?
        .to_string_lossy()
        .replace('\\', "/")
        .into_bytes())
}

pub(super) fn write_synced_file(path: &Path, bytes: &[u8]) -> Result<()> {
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

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
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

pub(super) fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("opening directory {} for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("syncing directory {}", path.display()))
}

/// A complete flow prepared outside its published path. Dropping this value
/// before [`AtomicFlowImport::publish`] is equivalent to an interrupted import:
/// the old flow remains byte-for-byte visible and the staging tree is removed.
pub struct AtomicFlowImport {
    pub(super) root: PathBuf,
    pub(super) target: PathBuf,
    pub(super) staging: PathBuf,
    pub(super) target_existed_at_prepare: bool,
    pub(super) published: bool,
}
