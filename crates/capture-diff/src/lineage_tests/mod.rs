//! Capture-lineage regressions.
//!
//! Separated from the lineage_tests.rs root under #683.

//! Behaviour tests for [`super`].
//!
//! Extracted from `lineage.rs`, which was 4,949 lines of which
//! 1,846 — 37% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

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
        let result =
            |account: &str, account_id: u32, character_guid: u64, item_push: bool, money: u64| {
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
            "contract": CREATURE_SPELL_FIXTURE_CONTRACT,
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

mod scenarios_1;
mod scenarios_2;
