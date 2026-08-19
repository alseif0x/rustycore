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
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();

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
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
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
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
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
    let committed_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("flows/detour-chase-around-obstacle/fixture");
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
        let transaction = AtomicFlowImport::prepare(&root, "detour-chase-around-obstacle").unwrap();
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
    let report_path = serde_json::from_slice::<serde_json::Value>(&original).unwrap()["bot_report"]
        ["report_path"]
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
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
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
    let mut missing_derivation: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
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
        format!("{error:#}").contains("creature-spell-casting C++ source worktree must be clean"),
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
    let mut cpp_wrong_binary: serde_json::Value = serde_json::from_slice(&cpp_original).unwrap();
    cpp_wrong_binary["source_exec_revision"] = serde_json::Value::String("e".repeat(40));
    fs::write(
        &cpp_manifest,
        serde_json::to_vec_pretty(&cpp_wrong_binary).unwrap(),
    )
    .unwrap();
    let error = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true)
        .expect_err("C++ binary/source revision mismatch must fail");
    assert!(
        format!("{error:#}").contains("embedded executable revision must equal source_repo_head"),
        "unexpected error: {error:#}"
    );
    fs::write(&cpp_manifest, cpp_original).unwrap();

    let rust_original = fs::read(&rust_manifest).unwrap();
    let mut rust_wrong_binary: serde_json::Value = serde_json::from_slice(&rust_original).unwrap();
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
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
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
        let report_path = PathBuf::from(manifest["bot_report"]["report_path"].as_str().unwrap());
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let result = &mut report["results"][0];
        result["creature_spell_start_body_sha256"] = serde_json::Value::String(sha256_bytes(start));
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
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
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
    let raw = validate_raw_pair(flow, &cpp, &cpp_manifest, &rust, &rust_manifest, true).unwrap();
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
