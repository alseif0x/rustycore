//! Capture lineage tracking state definitions, part 2 of 4.
//!
//! Separated from the lineage.rs root under #658. Behaviour is preserved.

use super::*;

pub(super) fn validate_canonical_loot_identity(manifest: &RawCaptureManifest) -> Result<()> {
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

pub(super) fn validate_canonical_loot_race_identity(manifest: &RawCaptureManifest) -> Result<()> {
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

pub(super) fn validate_canonical_vendor_identity(manifest: &RawCaptureManifest) -> Result<()> {
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

pub(super) fn validate_canonical_detour_identity(manifest: &RawCaptureManifest) -> Result<()> {
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

pub(super) fn validate_canonical_creature_spell_identity(
    manifest: &RawCaptureManifest,
) -> Result<()> {
    let fixture = manifest
        .fixture_guard
        .as_ref()
        .context("creature-spell-casting requires fixture_guard evidence")?;
    ensure!(fixture.enabled, "fixture_guard.enabled must be true");
    ensure!(
        fixture.contract == CREATURE_SPELL_FIXTURE_CONTRACT,
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

pub(super) fn detour_fixture_shared_identity_matches(
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

pub(super) fn creature_spell_fixture_shared_identity_matches(
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

pub(super) fn validate_cross_side_identity(
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

pub(super) fn validate_bot_report_artifact(
    manifest: &RawCaptureManifest,
) -> Result<Option<Vec<u8>>> {
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

pub(super) fn validate_bot_report_json(bytes: &[u8], evidence: &BotReportEvidence) -> Result<()> {
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

pub(super) fn validate_creature_spell_bot_report_json(
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

pub(super) fn validate_detour_chase_bot_report_json(
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

pub(super) fn validate_vendor_bot_report_json(
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

pub(super) fn validate_loot_item_bot_report_json(
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
