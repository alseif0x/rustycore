//! Shared plumbing for the vendor-sell and basic-movement smokes.
//!
//! Both modes run one guarded pre-login fixture, a mutating login phase that
//! ends with the C++-routed logout, a fresh relogin that re-reads the
//! persisted state, and a cleanup that waits for the offline save before
//! restoring the snapshot. The mode-specific halves live in `vendor_sell.rs`
//! and `movement_smoke.rs`.

use super::*;

/// C++ `CharacterPackets.cpp` `LoginVerifyWorld::Write`; Opcodes.cpp routes it
/// on `CONNECTION_TYPE_INSTANCE`.
pub(crate) const SMSG_LOGIN_VERIFY_WORLD: u16 = 0x2597;

#[derive(Debug, Clone)]
pub(crate) enum PostLoginSmokeOptions {
    VendorSell(VendorSellOptions),
    Movement(MovementSmokeOptions),
}

/// `SMSG_LOGIN_VERIFY_WORLD` body: `int32 MapID`, `TaggedPosition<XYZO> Pos`
/// (four floats), `uint32 Reason` (`CharacterPackets.cpp`
/// `LoginVerifyWorld::Write`, filled from `Player::GetMapId`/`GetPosition`
/// in `CharacterHandler.cpp` `HandlePlayerLogin`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LoginVerifyWorldPosition {
    pub(crate) map_id: i32,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) orientation: f32,
    pub(crate) reason: u32,
}

pub(crate) fn parse_login_verify_world_like_cpp(
    payload: &[u8],
) -> Result<LoginVerifyWorldPosition> {
    if payload.len() != 24 {
        bail!(
            "SMSG_LOGIN_VERIFY_WORLD has {} bytes, C++ LoginVerifyWorld::Write emits 24",
            payload.len()
        );
    }
    let f32_at =
        |offset: usize| f32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
    Ok(LoginVerifyWorldPosition {
        map_id: i32::from_le_bytes(payload[0..4].try_into()?),
        x: f32_at(4),
        y: f32_at(8),
        z: f32_at(12),
        orientation: f32_at(16),
        reason: u32::from_le_bytes(payload[20..24].try_into()?),
    })
}

/// Login-window observations the post-login phases consume.
#[derive(Debug, Clone, Default)]
pub(crate) struct PostLoginSmokeLoginState {
    pub(crate) vendor_target_seen: Option<DiscoveredCreatureGuid>,
    pub(crate) login_verify_world: Option<LoginVerifyWorldPosition>,
}

pub(crate) fn observe_post_login_smoke_login_packet(
    options: Option<&PostLoginSmokeOptions>,
    opcode: u16,
    payload: &[u8],
    state: &mut PostLoginSmokeLoginState,
) -> Result<()> {
    let Some(options) = options else {
        return Ok(());
    };
    if opcode == SMSG_LOGIN_VERIFY_WORLD {
        state.login_verify_world = Some(parse_login_verify_world_like_cpp(payload)?);
    }
    if let PostLoginSmokeOptions::VendorSell(options) = options {
        observe_vendor_sell_login_update(options, opcode, payload, &mut state.vendor_target_seen)?;
    }
    Ok(())
}

/// The destructive fixtures only ever touch disposable local QA identities,
/// exactly like the bank/vendor fixtures.
pub(crate) fn require_bot_local_account(account: &str, mode: &str) -> Result<()> {
    if !account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
        bail!("refusing destructive {mode} fixture setup for non-local account {account}");
    }
    Ok(())
}

/// Load `(account, online, map, zone, instance_id, x, y, z, o, money)` and
/// require the fixture character to be owned by the bot account and offline.
pub(crate) fn load_offline_fixture_character(
    characters: &mut mysql::Conn,
    bot: &config::BotConfig,
    mode: &str,
) -> Result<(CharacterPositionSnapshot, u64)> {
    use mysql::prelude::Queryable;

    let character: Option<(u32, u8, u32, u32, u32, f64, f64, f64, f32, u64)> = characters
        .exec_first(
            "SELECT account, online, map, zone, instance_id, position_x, position_y, position_z, orientation, money \
             FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load {mode} bot character: {error}"))?;
    let (owner, online, map_id, zone_id, instance_id, x, y, z, orientation, money) =
        character.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "character {} belongs to account {}, expected {}",
            bot.character_guid,
            owner,
            bot.account_id
        );
    }
    if online != 0 {
        bail!(
            "character {} is online; log it out before {mode} smoke setup",
            bot.character_guid
        );
    }
    Ok((
        CharacterPositionSnapshot {
            map_id,
            zone_id,
            instance_id,
            x,
            y,
            z,
            orientation,
        },
        money,
    ))
}

/// A failed phase can drop the socket before the normal logout path. Wait
/// for the world server's disconnect save before cleanup, otherwise that late
/// save could recreate fixture rows or overwrite the restored snapshot
/// (same contract as `cleanup_bank_smoke_fixture`).
pub(crate) fn wait_for_fixture_character_offline(
    characters: &mut mysql::Conn,
    character_guid: u64,
    mode: &str,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let online: Option<u8> = characters
            .exec_first(
                "SELECT online FROM characters WHERE guid = ?",
                (character_guid,),
            )
            .map_err(|error| anyhow!("Check {mode} bot offline before cleanup: {error}"))?;
        match online {
            Some(0) => return Ok(()),
            Some(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Some(_) => bail!(
                "character {character_guid} remained online; refusing {mode} cleanup before disconnect save"
            ),
            None => bail!("No characters row for guid {character_guid} during {mode} cleanup"),
        }
    }
}

/// Answer `SMSG_TIME_SYNC_REQUEST` and otherwise drain the instance socket
/// for `window`. Returns every observed opcode for the report.
pub(crate) async fn pump_instance_socket(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    window: Duration,
    context: &str,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            remaining.min(Duration::from_millis(50)),
            Duration::from_secs(5),
            context,
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
        }
    }
}

pub(crate) async fn run_bot_with_post_login_smoke(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    options: PostLoginSmokeOptions,
) -> Result<BotRunResult> {
    run_bot_with_void_storage(
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
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(options),
    )
    .await
}

/// Post-login dispatch from `run_bot_with_void_storage`. A failing phase still
/// attempts the C++-routed logout so cleanup never races a live session.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_post_login_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &PostLoginSmokeOptions,
    login_state: PostLoginSmokeLoginState,
    result: &mut BotRunResult,
) {
    let mut logged_out = false;
    let outcome = match options {
        PostLoginSmokeOptions::VendorSell(options) => {
            run_vendor_sell_phase(
                bot_index,
                bot,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options,
                login_state.vendor_target_seen,
                &mut logged_out,
                result,
            )
            .await
        }
        PostLoginSmokeOptions::Movement(options) => {
            run_movement_smoke_phase(
                bot_index,
                bot,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options,
                login_state.login_verify_world,
                &mut logged_out,
                result,
            )
            .await
        }
    };
    let Err(error) = outcome else {
        return;
    };
    let mut failure = error.to_string();
    if !logged_out {
        if let Err(logout_error) = loot_race::logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection.as_mut(),
            bot.character_guid,
            result,
        )
        .await
        {
            failure.push_str(&format!(
                "; graceful logout after failure also failed: {logout_error}"
            ));
        }
    }
    match options {
        PostLoginSmokeOptions::VendorSell(_) => {
            result.vendor_sell.vendor_sell_failure = Some(failure);
            result.vendor_sell.vendor_sell_smoke_passed = Some(false);
        }
        PostLoginSmokeOptions::Movement(_) => {
            result.movement.movement_failure = Some(failure);
            result.movement.movement_smoke_passed = Some(false);
        }
    }
}

/// CLI validation for both modes (main.rs keeps one call).
pub(crate) fn validate_post_login_smoke_cli(cli: &CliOptions, bot_count: usize) -> Result<()> {
    for (enabled, flag) in [
        (cli.vendor_sell_smoke, "--vendor-sell-smoke"),
        (cli.movement_smoke, "--movement-smoke"),
    ] {
        if enabled && bot_count != 1 {
            bail!("{flag} requires exactly one bot; select it with --single");
        }
    }
    if cli.vendor_sell_smoke {
        if cli.vendor_timeout_secs == 0 {
            bail!("--vendor-timeout must be greater than zero");
        }
        if cli.vendor_entry == 0 || cli.vendor_spawn_guid == 0 || cli.vendor_sell_item_entry == 0 {
            bail!("vendor sell smoke requires nonzero vendor entry, spawn and item entry");
        }
        if cli.vendor_sell_expected_money == Some(0) {
            bail!("--vendor-sell-expected-money must be greater than zero when set");
        }
    }
    Ok(())
}

/// main.rs dispatch for the sequential single-bot loop.
pub(crate) async fn run_post_login_smoke(
    cli: &CliOptions,
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
) -> Result<BotRunResult> {
    if cli.vendor_sell_smoke {
        run_vendor_sell_smoke_workflow(
            bot,
            dungeon_id,
            lfg_secs,
            auto_teleport,
            VendorSellCli {
                vendor_entry: cli.vendor_entry,
                vendor_spawn_guid: cli.vendor_spawn_guid,
                runtime_counter: cli.vendor_runtime_counter,
                item_entry: cli.vendor_sell_item_entry,
                expected_money_delta: cli.vendor_sell_expected_money,
                timeout_secs: cli.vendor_timeout_secs,
            },
        )
        .await
    } else {
        run_movement_smoke_workflow(bot, dungeon_id, lfg_secs, auto_teleport).await
    }
}

/// Append a cleanup failure to an existing failure text.
pub(crate) fn append_failure(slot: &mut Option<String>, failure: String) {
    *slot = Some(match slot.take() {
        Some(previous) => format!("{previous}; {failure}"),
        None => failure,
    });
}

/// One-line human summary for `run_report::log_bot_summary`.
pub(crate) fn post_login_smoke_summary(result: &BotRunResult) -> Option<String> {
    if result.vendor_sell.vendor_sell_smoke {
        let evidence = &result.vendor_sell;
        return Some(format!(
            "vendor_sell_smoke vendor={:?}/{:?}/counter={:?} item={:?}/guid={:?} money={:?}->{:?} delta={:?}/expected={:?} sent={} sell_response={}/{:?} db={} relog={} failure={:?}",
            evidence.vendor_sell_entry,
            evidence.vendor_sell_spawn_guid,
            evidence.vendor_sell_runtime_counter,
            evidence.vendor_sell_item_entry,
            evidence.vendor_sell_item_guid,
            evidence.vendor_sell_money_before,
            evidence.vendor_sell_money_after,
            evidence.vendor_sell_money_delta,
            evidence.vendor_sell_expected_money_delta,
            evidence.vendor_sell_item_sent,
            evidence.vendor_sell_response_seen,
            evidence.vendor_sell_response_reason,
            evidence.vendor_sell_db_persisted,
            evidence.vendor_sell_relogin_verified,
            evidence.vendor_sell_failure,
        ));
    }
    if result.movement.movement_smoke {
        let evidence = &result.movement;
        return Some(format!(
            "movement_smoke login={:?} final={:?} sent={:?} db={:?}/{} relog={:?}/{} failure={:?}",
            evidence.movement_login_position,
            evidence.movement_final_position,
            evidence.movement_opcodes_sent,
            evidence.movement_db_position,
            evidence.movement_db_persisted,
            evidence.movement_relogin_position,
            evidence.movement_relogin_verified,
            evidence.movement_failure,
        ));
    }
    None
}
