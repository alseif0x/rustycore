//! WoW Test Bot - TrinityCore 3.4.3 Modern Protocol with Full SRP6
//! Combines BNet SRP6 Auth + World Server AES-GCM Encryption + LFG

use anyhow::{anyhow, bail, Context, Result};
use flate2::{Decompress, FlushDecompress};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

mod bot_srp6;
mod cast_lifecycle;
mod run_report;
use run_report::{log_bot_summary, write_report_if_requested};
mod config;
mod login_save;
mod login_stream;
mod loot_race;
mod packet_parser;
mod protocol;
mod spell_acquisition;
mod srp6_auth;
mod wow_crypto;

use login_stream::drain_login_streams;
use packet_parser::*;
use protocol::*;
use wow_crypto::WorldCrypt;

mod bot;
#[allow(unused_imports)]
use bot::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("🎮 WoW Test Bot - TrinityCore 3.4.3 SRP6 + AES-GCM");
    info!("═══════════════════════════════════════════════════");

    let cli = parse_cli()?;
    validate_detour_chase_cli_values(
        cli.detour_chase_capture,
        cli.ack_disposable_detour_fixture,
        cli.single_account.as_deref(),
        cli.detour_chase_timeout_secs,
        cli.detour_fixture_manifest.as_deref(),
    )?;
    validate_creature_spell_capture_cli_values(
        cli.creature_spell_capture,
        cli.single_account.as_deref(),
        cli.creature_spell_capture_timeout_secs,
        cli.creature_spell_fixture_manifest.as_deref(),
    )?;
    if cli.recover_loot_fixture {
        let conflicting_mode = cli.ensure_test_accounts
            || cli.login_only
            || cli.stand_state_smoke
            || cli.bank_smoke
            || cli.void_storage_smoke
            || cli.void_storage_query_capture
            || cli.homebind_smoke
            || cli.inventory_swap_smoke
            || cli.vendor_smoke
            || cli.equipment_set_race_smoke
            || cli.rested_xp_smoke
            || cli.detour_chase_capture
            || cli.creature_spell_capture
            || cli.cast_lifecycle
            || cli.loot_race_smoke
            || cli.loot_item_capture
            || cli.group_capacity_race_smoke
            || cli.quest_smoke
            || cli.single_account.is_some();
        if conflicting_mode {
            bail!("--recover-loot-fixture must be used alone");
        }
        loot_race::recover_pending_fixture().await?;
        info!("Pending loot fixture recovered and cleanup marker written");
        return Ok(());
    }
    let detour_chase_options = if cli.detour_chase_capture {
        let manifest = cli
            .detour_fixture_manifest
            .as_deref()
            .context("Validated detour capture is missing its fixture manifest")?;
        validate_detour_fixture_manifest(Path::new(manifest))?;
        Some(detour_chase_options_from_pinned_manifest(
            &expected_detour_fixture_manifest(),
            cli.detour_chase_timeout_secs,
        )?)
    } else {
        None
    };
    let creature_spell_options = if cli.creature_spell_capture {
        let manifest = cli
            .creature_spell_fixture_manifest
            .as_deref()
            .context("Validated creature spell capture is missing its fixture manifest")?;
        let fixture_manifest_sha256 =
            validate_creature_spell_fixture_manifest(Path::new(manifest))?;
        Some(CreatureSpellCaptureOptions {
            fixture_manifest_sha256,
            timeout_secs: cli.creature_spell_capture_timeout_secs,
        })
    } else {
        None
    };
    let cast_lifecycle_options = if cli.cast_lifecycle {
        let path = cli.cast_lifecycle_plan.as_deref().context(
            "--cast-lifecycle requires --cast-lifecycle-plan or WOW_BOT_CAST_LIFECYCLE_PLAN",
        )?;
        Some(cast_lifecycle::load(Path::new(path))?)
    } else {
        None
    };
    let app_config = config::AppConfig::load_or_create(&cli.config_path)?;
    let mut bots: Vec<config::BotConfig> =
        app_config.get_enabled_bots().into_iter().cloned().collect();

    if let Some(account) = &cli.single_account {
        bots.retain(|bot| bot.account.eq_ignore_ascii_case(account));
    }
    add_pinned_detour_fixture_bot_if_missing(&mut bots, cli.detour_chase_capture);
    add_pinned_creature_spell_fixture_bot_if_missing(&mut bots, cli.creature_spell_capture);
    if cli.loot_race_smoke || cli.loot_item_capture {
        if cli.single_account.is_some() {
            bail!(
                "--single is incompatible with the loot workflows; select the guarded fixture with the loot account flags"
            );
        }
        bots.retain(|bot| {
            bot.account.eq_ignore_ascii_case(&cli.loot_race_account_a)
                || bot.account.eq_ignore_ascii_case(&cli.loot_race_account_b)
        });
    }
    if cli.group_capacity_race_smoke {
        if cli.single_account.is_some() {
            bail!("--single is incompatible with --group-capacity-race-smoke");
        }
        bots.retain(|bot| {
            bot.account
                .eq_ignore_ascii_case(&cli.group_capacity_leader_account)
                || bot
                    .account
                    .eq_ignore_ascii_case(&cli.group_capacity_candidate_a_account)
                || bot
                    .account
                    .eq_ignore_ascii_case(&cli.group_capacity_candidate_b_account)
        });
    }
    if cli.equipment_set_race_smoke {
        if cli.single_account.is_some() {
            bail!("--single is incompatible with --equipment-set-race-smoke");
        }
        bots.retain(|bot| {
            bot.account
                .eq_ignore_ascii_case(&cli.equipment_set_account_a)
                || bot
                    .account
                    .eq_ignore_ascii_case(&cli.equipment_set_account_b)
        });
    }
    apply_password_overrides(&mut bots);

    if bots.is_empty() {
        bail!("No enabled bots matched the current config/filter");
    }
    if cli.detour_chase_capture && bots.len() != 1 {
        bail!("--detour-chase-capture requires exactly one pinned fixture bot");
    }
    if cli.creature_spell_capture && bots.len() != 1 {
        bail!("--creature-spell-capture requires exactly one pinned fixture bot");
    }
    if cli.cast_lifecycle && bots.len() != 1 {
        bail!("--cast-lifecycle requires exactly one configured bot; select it with --single");
    }
    if cli.cast_lifecycle && cli.login_only {
        bail!("--cast-lifecycle and --login-only are separate post-login modes");
    }
    if cli.cast_lifecycle && cli.ensure_test_accounts {
        bail!("--cast-lifecycle never provisions accounts; remove --ensure-test-accounts");
    }
    let missing_passwords: Vec<&str> = bots
        .iter()
        .filter(|bot| {
            bot.password.is_empty()
                && !(cli.group_capacity_race_smoke && !bot.session_key_bnet.trim().is_empty())
                && !cli.equipment_set_race_smoke
        })
        .map(|bot| bot.account.as_str())
        .collect();
    if !missing_passwords.is_empty() {
        bail!(
            "Missing bot password for {}. Set WOW_BOT_PASSWORD, set {}, or use an ignored local config.json.",
            missing_passwords.join(", "),
            password_env_name(missing_passwords[0])
        );
    }
    let loot_mode = cli.loot_race_smoke || cli.loot_item_capture;
    let guarded_identity_mode = loot_mode
        || cli.detour_chase_capture
        || cli.creature_spell_capture
        || cli.cast_lifecycle
        || cli.group_capacity_race_smoke
        || cli.equipment_set_race_smoke;
    validate_provisioning_mode(guarded_identity_mode, cli.ensure_test_accounts)?;
    let post_login_mode_count = [
        cli.stand_state_smoke,
        cli.bank_smoke,
        cli.void_storage_smoke,
        cli.void_storage_query_capture,
        cli.homebind_smoke,
        cli.inventory_swap_smoke,
        cli.vendor_smoke,
        cli.equipment_set_race_smoke,
        cli.rested_xp_smoke,
        cli.detour_chase_capture,
        cli.creature_spell_capture,
        cli.cast_lifecycle,
        cli.loot_race_smoke,
        cli.loot_item_capture,
        cli.group_capacity_race_smoke,
        cli.quest_smoke,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if post_login_mode_count > 1 {
        bail!(
            "stand-state, bank, void-storage, homebind, inventory-swap, vendor, equipment-set-race, rested-xp, detour-chase-capture, creature-spell-capture, cast-lifecycle, loot-race, loot-item-capture, group-capacity-race, and quest smoke are separate post-login modes"
        );
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
    if (cli.void_storage_smoke || cli.void_storage_query_capture) && bots.len() != 1 {
        bail!("void-storage modes require exactly one bot; select it with --single");
    }
    if (cli.void_storage_smoke || cli.void_storage_query_capture)
        && cli.void_storage_timeout_secs == 0
    {
        bail!("--void-storage-timeout must be greater than zero");
    }
    if cli.homebind_smoke && bots.len() != 1 {
        bail!("--homebind-smoke requires exactly one bot; select it with --single");
    }
    if cli.homebind_smoke && cli.homebind_timeout_secs == 0 {
        bail!("--homebind-timeout must be greater than zero");
    }
    if cli.inventory_swap_smoke && bots.len() != 1 {
        bail!("--inventory-swap-smoke requires exactly one bot; select it with --single");
    }
    if cli.inventory_swap_smoke && cli.inventory_swap_timeout_secs == 0 {
        bail!("--inventory-swap-timeout must be greater than zero");
    }
    if cli.inventory_swap_smoke
        && cli.inventory_swap_item_entry_a == cli.inventory_swap_item_entry_b
    {
        bail!("inventory-swap fixture item entries must be different to avoid stack merging");
    }
    if cli.vendor_smoke && bots.len() != 1 {
        bail!("--vendor-smoke requires exactly one bot; select it with --single");
    }
    if cli.vendor_smoke && cli.vendor_timeout_secs == 0 {
        bail!("--vendor-timeout must be greater than zero");
    }
    if cli.vendor_smoke
        && (cli.vendor_entry == 0
            || cli.vendor_spawn_guid == 0
            || cli.vendor_item_entry == 0
            || cli.vendor_extended_cost == 0
            || cli.vendor_currency_id == 0
            || cli.vendor_currency_cost == 0
            || cli.vendor_currency_quantity <= cli.vendor_currency_cost)
    {
        bail!("vendor smoke requires nonzero fixture identifiers/cost and a seeded currency quantity greater than one purchase cost");
    }
    if cli.equipment_set_race_smoke {
        if bots.len() != 2 {
            bail!("--equipment-set-race-smoke requires exactly its two configured bots");
        }
        if cli.equipment_set_timeout_secs == 0 {
            bail!("--equipment-set-timeout must be greater than zero");
        }
        if cli
            .equipment_set_account_a
            .eq_ignore_ascii_case(&cli.equipment_set_account_b)
        {
            bail!("equipment-set race accounts must be distinct");
        }
    }
    validate_rested_xp_cli_values(
        cli.rested_xp_smoke,
        cli.ack_disposable_rested_xp,
        bots.len(),
        cli.rested_xp_creature_entry,
        cli.rested_xp_offline_secs,
        cli.rested_xp_timeout_secs,
        current_epoch_secs(),
    )?;
    let loot_race_cli = loot_race::LootRaceCli {
        account_a: cli.loot_race_account_a.clone(),
        account_b: cli.loot_race_account_b.clone(),
        entry: cli.loot_race_creature_entry,
        spawn_guid: cli.loot_race_creature_spawn_guid,
        runtime_counter: cli.loot_race_runtime_counter,
        item_entry: cli.loot_race_item_entry,
        timeout_secs: cli.loot_race_timeout_secs,
        workflow_deadline_secs: cli.loot_workflow_deadline_secs,
    };
    let group_capacity_cli = loot_race::GroupCapacityRaceCli {
        leader_account: cli.group_capacity_leader_account.clone(),
        candidate_a_account: cli.group_capacity_candidate_a_account.clone(),
        candidate_b_account: cli.group_capacity_candidate_b_account.clone(),
        group_db_store_id: cli.group_capacity_group_id,
        timeout_secs: cli.group_capacity_timeout_secs,
    };
    loot_race::validate_cli(
        cli.loot_race_smoke,
        cli.loot_item_capture,
        cli.ack_disposable_overworld_loot_race,
        &bots,
        &loot_race_cli,
    )?;
    if loot_mode {
        loot_race::validate_journal_contract()?;
        let bots_for_validation = bots.clone();
        tokio::task::spawn_blocking(move || {
            validate_exact_loot_bot_identities(&bots_for_validation)
        })
        .await
        .map_err(|e| anyhow!("Loot identity-preflight DB worker join failed: {e}"))??;
    } else if cli.group_capacity_race_smoke {
        let bots_for_validation = bots.clone();
        tokio::task::spawn_blocking(move || {
            validate_linked_group_capacity_bot_identities(&bots_for_validation)
        })
        .await
        .map_err(|e| anyhow!("Group-capacity identity preflight DB worker failed: {e}"))??;
    } else if cli.ensure_test_accounts {
        let bots_for_db = bots.clone();
        tokio::task::spawn_blocking(move || ensure_test_accounts(&bots_for_db))
            .await
            .map_err(|e| anyhow!("DB worker join failed while provisioning test accounts: {e}"))?
            .map_err(|e| anyhow!("Failed to provision test accounts: {e}"))?;
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
        } else if cli.void_storage_smoke {
            "void-storage-smoke"
        } else if cli.void_storage_query_capture {
            "void-storage-query-capture"
        } else if cli.homebind_smoke {
            "homebind-smoke"
        } else if cli.inventory_swap_smoke {
            "inventory-swap-smoke"
        } else if cli.vendor_smoke {
            "vendor-smoke"
        } else if cli.equipment_set_race_smoke {
            "equipment-set-race-smoke"
        } else if cli.rested_xp_smoke {
            "rested-xp-smoke"
        } else if cli.detour_chase_capture {
            "detour-chase-capture"
        } else if cli.creature_spell_capture {
            "creature-spell-capture"
        } else if cli.cast_lifecycle {
            "cast-lifecycle"
        } else if cli.loot_race_smoke {
            "loot-race-smoke"
        } else if cli.loot_item_capture {
            "loot-item-capture"
        } else if cli.group_capacity_race_smoke {
            "group-capacity-race-smoke"
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

    if cleanup_groups
        && !cli.login_only
        && !cli.stand_state_smoke
        && !cli.bank_smoke
        && !cli.void_storage_smoke
        && !cli.void_storage_query_capture
        && !cli.homebind_smoke
        && !cli.inventory_swap_smoke
        && !cli.vendor_smoke
        && !cli.equipment_set_race_smoke
        && !cli.rested_xp_smoke
        && !cli.detour_chase_capture
        && !cli.creature_spell_capture
        && !cli.cast_lifecycle
        && !cli.loot_race_smoke
        && !cli.loot_item_capture
        && !cli.group_capacity_race_smoke
    {
        cleanup_bot_group_state(&bots)?;
    }

    let expected_bot_count = if cli.loot_item_capture { 1 } else { bots.len() };
    let mut results = Vec::new();
    if cli.loot_item_capture {
        let shutdown = install_loot_termination_token()?;
        results = finish_guarded_loot_result(
            loot_race::run_single_item_capture_workflow(
                bots,
                loot_race_cli,
                dungeon_id,
                timeout_secs,
                auto_teleport,
                shutdown,
            )
            .await,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.loot_race_smoke {
        let shutdown = install_loot_termination_token()?;
        results = finish_guarded_loot_result(
            loot_race::run_workflow(
                bots,
                loot_race_cli,
                dungeon_id,
                timeout_secs,
                auto_teleport,
                shutdown,
            )
            .await,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.group_capacity_race_smoke {
        let shutdown = install_loot_termination_token()?;
        results = loot_race::run_group_capacity_workflow(
            bots,
            group_capacity_cli,
            dungeon_id,
            timeout_secs,
            auto_teleport,
            shutdown,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.equipment_set_race_smoke {
        results = run_equipment_set_race_workflow(
            bots,
            dungeon_id,
            timeout_secs,
            auto_teleport,
            cli.equipment_set_account_a.clone(),
            cli.equipment_set_account_b.clone(),
            cli.equipment_set_timeout_secs,
        )
        .await?;
        for result in &results {
            log_bot_summary(result, require_proposal, require_group, cli.login_only);
        }
    } else if cli.sequential || bots.len() == 1 {
        for bot in bots {
            info!("\n[Bot {}] Starting...", bot.account);
            let detour_failure_bot = cli.detour_chase_capture.then(|| bot.clone());
            let creature_spell_failure_bot = cli.creature_spell_capture.then(|| bot.clone());
            let cast_lifecycle_failure_bot = cli.cast_lifecycle.then(|| bot.clone());
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
            } else if cli.void_storage_smoke {
                run_void_storage_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.void_storage_item_entry,
                    cli.void_storage_runtime_counter,
                    cli.void_storage_timeout_secs,
                )
                .await
            } else if cli.void_storage_query_capture {
                run_void_storage_query_capture_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.void_storage_item_entry,
                    cli.void_storage_runtime_counter,
                    cli.void_storage_timeout_secs,
                )
                .await
            } else if cli.homebind_smoke {
                run_homebind_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.homebind_runtime_counter,
                    cli.homebind_timeout_secs,
                )
                .await
            } else if cli.inventory_swap_smoke {
                run_inventory_swap_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.inventory_swap_item_entry_a,
                    cli.inventory_swap_item_entry_b,
                    cli.inventory_swap_timeout_secs,
                )
                .await
            } else if cli.vendor_smoke {
                run_vendor_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.vendor_entry,
                    cli.vendor_spawn_guid,
                    cli.vendor_runtime_counter,
                    cli.vendor_item_entry,
                    cli.vendor_extended_cost,
                    cli.vendor_currency_id,
                    cli.vendor_currency_cost,
                    cli.vendor_currency_quantity,
                    cli.vendor_timeout_secs,
                )
                .await
            } else if cli.rested_xp_smoke {
                run_rested_xp_smoke_workflow(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.rested_xp_creature_entry,
                    cli.rested_xp_creature_guid,
                    cli.rested_xp_runtime_counter,
                    cli.rested_xp_offline_secs,
                    cli.rested_xp_timeout_secs,
                )
                .await
            } else if let Some(options) = detour_chase_options.clone() {
                run_bot_with_detour_chase(bot, dungeon_id, timeout_secs, auto_teleport, options)
                    .await
            } else if let Some(options) = creature_spell_options.clone() {
                run_bot_with_creature_spell_capture(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    options,
                )
                .await
            } else if let Some(options) = cast_lifecycle_options.clone() {
                cast_lifecycle::run(bot, dungeon_id, timeout_secs, options).await
            } else {
                run_bot(
                    bot,
                    dungeon_id,
                    timeout_secs,
                    auto_teleport,
                    cli.login_only,
                    stand_state_options.clone(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
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
                Err(error) => {
                    if let (Some(bot), Some(options)) =
                        (detour_failure_bot, detour_chase_options.as_ref())
                    {
                        let result = detour_chase_failure_result(
                            &bot,
                            dungeon_id,
                            options,
                            error.to_string(),
                        );
                        log_bot_summary(&result, require_proposal, require_group, cli.login_only);
                        results.push(result);
                    } else if let (Some(bot), Some(options)) =
                        (creature_spell_failure_bot, creature_spell_options.as_ref())
                    {
                        let result = creature_spell_failure_result(
                            &bot,
                            dungeon_id,
                            options,
                            error.to_string(),
                        );
                        log_bot_summary(&result, require_proposal, require_group, cli.login_only);
                        results.push(result);
                    } else if let Some(bot) = cast_lifecycle_failure_bot {
                        let result =
                            cast_lifecycle::failure_result(&bot, dungeon_id, error.to_string());
                        log_bot_summary(&result, require_proposal, require_group, cli.login_only);
                        results.push(result);
                    } else {
                        error!("❌ Bot run ERROR: {}", error);
                    }
                }
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
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
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
        cli.void_storage_smoke,
        cli.void_storage_query_capture,
        cli.homebind_smoke,
        cli.inventory_swap_smoke,
        cli.vendor_smoke,
        cli.equipment_set_race_smoke,
        cli.rested_xp_smoke,
        cli.detour_chase_capture,
        cli.creature_spell_capture,
        cli.cast_lifecycle,
        cli.loot_race_smoke,
        cli.loot_item_capture,
        cli.group_capacity_race_smoke,
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

// ═════════════════════════════════════════════════════════════════════════════
// Session key plumbing (DB sync for worldserver)
// ═════════════════════════════════════════════════════════════════════════════

// ═════════════════════════════════════════════════════════════════════════════
// Packet I/O helpers
// ═════════════════════════════════════════════════════════════════════════════

// ═════════════════════════════════════════════════════════════════════════════
// Auth & crypto helpers
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[path = "main_tests/mod.rs"]
mod tests;
