//! Basic player MOVEMENT persistence smoke.
//!
//! C++ anchors (TrinityCore-derived 3.4.3 reference):
//! - Opcodes.h: `CMSG_MOVE_START_FORWARD = 0x39E4`, `CMSG_MOVE_HEARTBEAT =
//!   0x3A10`, `CMSG_MOVE_STOP = 0x39E6`; Opcodes.cpp registers all three as
//!   `STATUS_LOGGEDIN`/`PROCESS_THREADSAFE` -> `HandleMovementOpcodes`.
//!   Client opcodes carry no server connection type; like every other bot
//!   movement packet they go on the instance socket that owns the map session.
//! - `MovementPackets.cpp` `ClientPlayerMovement::Read` ->
//!   `operator>>(ByteBuffer&, MovementInfo&)`: packed mover GUID, flags,
//!   flags2, flags3, time, XYZO, pitch, step-up elevation, remove-forces count,
//!   move index, then eight presence bits (all zero here).
//! - `MovementHandler.cpp` `HandleMovementOpcode`: rejects a GUID that is not
//!   the active mover, ignores invalid positions, stores the info and calls
//!   `UpdatePosition` with no server echo to the mover (`SendMessageToSet`
//!   skips `_player`). There is no speed/anti-cheat check in this handler.
//! - `UnitDefines.h` `MOVEMENTFLAG_FORWARD = 0x1`; `Unit.cpp`
//!   `baseMoveSpeed[MOVE_RUN] = 7.0f` yards/second.
//! - `Player::SaveToDB` persists the current position at logout, and
//!   `CharacterHandler.cpp` `HandlePlayerLogin` publishes it again in
//!   `SMSG_LOGIN_VERIFY_WORLD` (`CharacterPackets.cpp` `LoginVerifyWorld::Write`).
//!
//! Assumption: the walk keeps the login Z for a short straight run along the
//! login facing. Over a few yards of ordinary ground this stays above
//! `Map::GetMinHeight` and the server trusts client Z; live testing must pick
//! a character standing on flat, open ground (the fixture uses the current
//! character position and does not relocate it).

use super::*;

pub(crate) const CMSG_MOVE_START_FORWARD: u16 = 0x39E4;
pub(crate) const CMSG_MOVE_STOP: u16 = 0x39E6;
pub(crate) const MOVEMENTFLAG_FORWARD: u32 = 0x0000_0001;
/// `Unit.cpp` `baseMoveSpeed[MOVE_RUN]`.
pub(crate) const MOVEMENT_SMOKE_RUN_SPEED: f32 = 7.0;
/// Client time between successive movement packets.
pub(crate) const MOVEMENT_SMOKE_STEP_MS: u32 = 200;
/// Heartbeats between START_FORWARD and STOP; the run covers
/// `(HEARTBEATS + 1) * STEP_MS * RUN_SPEED` = 4.2 yards.
pub(crate) const MOVEMENT_SMOKE_HEARTBEATS: u32 = 2;
/// Tolerance in yards for persisted/republished positions (DB columns are
/// FLOAT, the wire is f32).
pub(crate) const MOVEMENT_SMOKE_POSITION_TOLERANCE: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MovementSmokePhase {
    Move,
    VerifyRelog,
}

#[derive(Debug, Clone)]
pub(crate) struct MovementSmokeOptions {
    pub(crate) phase: MovementSmokePhase,
    pub(crate) original_map_id: u32,
    /// Final sent position, set for the relog phase.
    pub(crate) expected_final: Option<MovementSmokePoint>,
}

#[derive(Debug, Clone)]
pub(crate) struct MovementSmokeFixture {
    pub(crate) options: MovementSmokeOptions,
    pub(crate) original_position: CharacterPositionSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) struct MovementSmokePoint {
    pub(crate) map_id: u32,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) orientation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MovementStep {
    pub(crate) opcode: u16,
    pub(crate) movement_flags: u32,
    pub(crate) client_time: u32,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) orientation: f32,
}

/// Flattened into `BotRunResult` so the JSON report carries `movement_*`.
#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct MovementSmokeEvidence {
    pub(crate) movement_smoke: bool,
    pub(crate) movement_smoke_passed: Option<bool>,
    pub(crate) movement_login_position: Option<MovementSmokePoint>,
    pub(crate) movement_final_position: Option<MovementSmokePoint>,
    pub(crate) movement_opcodes_sent: Vec<String>,
    pub(crate) movement_db_position: Option<[f64; 3]>,
    pub(crate) movement_db_persisted: bool,
    pub(crate) movement_relogin_position: Option<MovementSmokePoint>,
    pub(crate) movement_relogin_verified: bool,
    pub(crate) movement_failure: Option<String>,
}

impl MovementSmokeEvidence {
    pub(crate) fn for_options(options: Option<&PostLoginSmokeOptions>) -> Self {
        Self {
            movement_smoke: matches!(options, Some(PostLoginSmokeOptions::Movement(_))),
            ..Self::default()
        }
    }

    pub(crate) fn passed(&self) -> bool {
        self.movement_smoke_passed.unwrap_or(false)
            && self.movement_opcodes_sent.len() == MOVEMENT_SMOKE_HEARTBEATS as usize + 2
            && self.movement_db_persisted
            && self.movement_relogin_verified
    }
}

/// START_FORWARD at the login position, `MOVEMENT_SMOKE_HEARTBEATS` forward
/// heartbeats, then STOP, each `MOVEMENT_SMOKE_STEP_MS` further along the
/// login facing at run speed. Z and facing stay at their login values.
pub(crate) fn build_movement_smoke_plan(start: &MovementSmokePoint) -> Vec<MovementStep> {
    let step_yards = MOVEMENT_SMOKE_RUN_SPEED * MOVEMENT_SMOKE_STEP_MS as f32 / 1000.0;
    let (sin, cos) = start.orientation.sin_cos();
    let total = MOVEMENT_SMOKE_HEARTBEATS + 2;
    (0..total)
        .map(|index| {
            let (opcode, movement_flags) = if index == 0 {
                (CMSG_MOVE_START_FORWARD, MOVEMENTFLAG_FORWARD)
            } else if index + 1 == total {
                (CMSG_MOVE_STOP, 0)
            } else {
                (CMSG_MOVE_HEARTBEAT, MOVEMENTFLAG_FORWARD)
            };
            let distance = step_yards * index as f32;
            MovementStep {
                opcode,
                movement_flags,
                client_time: MOVEMENT_SMOKE_STEP_MS * index,
                x: start.x + cos * distance,
                y: start.y + sin * distance,
                z: start.z,
                orientation: start.orientation,
            }
        })
        .collect()
}

pub(crate) fn movement_step_payload(
    step: &MovementStep,
    player_low: u64,
    player_high: u64,
) -> Vec<u8> {
    build_client_movement_payload(
        player_low,
        player_high,
        step.movement_flags,
        step.client_time,
        step.x,
        step.y,
        step.z,
        step.orientation,
    )
}

pub(crate) fn movement_opcode_name(opcode: u16) -> &'static str {
    match opcode {
        CMSG_MOVE_START_FORWARD => "CMSG_MOVE_START_FORWARD",
        CMSG_MOVE_HEARTBEAT => "CMSG_MOVE_HEARTBEAT",
        CMSG_MOVE_STOP => "CMSG_MOVE_STOP",
        _ => "unknown",
    }
}

/// Same map and every axis within `tolerance` yards.
pub(crate) fn position_within_tolerance(
    expected_map: u32,
    expected: [f64; 3],
    actual_map: u32,
    actual: [f64; 3],
    tolerance: f64,
) -> bool {
    expected_map == actual_map
        && expected
            .iter()
            .zip(actual.iter())
            .all(|(expected, actual)| (expected - actual).abs() <= tolerance)
}

fn point_from_login(login: &LoginVerifyWorldPosition) -> Result<MovementSmokePoint> {
    Ok(MovementSmokePoint {
        map_id: u32::try_from(login.map_id)
            .map_err(|_| anyhow!("SMSG_LOGIN_VERIFY_WORLD has negative map {}", login.map_id))?,
        x: login.x,
        y: login.y,
        z: login.z,
        orientation: login.orientation,
    })
}

fn point_xyz(point: &MovementSmokePoint) -> [f64; 3] {
    [f64::from(point.x), f64::from(point.y), f64::from(point.z)]
}

pub(crate) fn prepare_movement_smoke_fixture(
    bot: &config::BotConfig,
) -> Result<MovementSmokeFixture> {
    require_bot_local_account(&bot.account, "movement")?;
    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut characters = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    let (original_position, _money) =
        load_offline_fixture_character(&mut characters, bot, "movement")?;
    info!(
        "Movement fixture: character={} map={} position=({:.3},{:.3},{:.3},{:.3})",
        bot.character_guid,
        original_position.map_id,
        original_position.x,
        original_position.y,
        original_position.z,
        original_position.orientation
    );
    Ok(MovementSmokeFixture {
        options: MovementSmokeOptions {
            phase: MovementSmokePhase::Move,
            original_map_id: original_position.map_id,
            expected_final: None,
        },
        original_position,
    })
}

async fn load_persisted_position(character_guid: u64) -> Result<(u32, [f64; 3])> {
    let (map, x, y, z) = tokio::task::spawn_blocking(move || load_bot_position(character_guid))
        .await
        .map_err(|error| anyhow!("Movement DB worker join failed: {error}"))??;
    Ok((u32::from(map), [x, y, z]))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_movement_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &MovementSmokeOptions,
    login_verify_world: Option<LoginVerifyWorldPosition>,
    logged_out: &mut bool,
    result: &mut BotRunResult,
) -> Result<()> {
    let login = login_verify_world
        .as_ref()
        .context("movement smoke did not capture SMSG_LOGIN_VERIFY_WORLD")?;
    let login_point = point_from_login(login)?;

    if options.phase == MovementSmokePhase::VerifyRelog {
        let expected = options
            .expected_final
            .context("movement relog phase requires the final sent position")?;
        result.movement.movement_relogin_position = Some(login_point);
        if !position_within_tolerance(
            expected.map_id,
            point_xyz(&expected),
            login_point.map_id,
            point_xyz(&login_point),
            MOVEMENT_SMOKE_POSITION_TOLERANCE,
        ) {
            bail!("relogin SMSG_LOGIN_VERIFY_WORLD position {login_point:?} does not match the final sent position {expected:?}");
        }
        result.movement.movement_relogin_verified = true;
        loot_race::logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            server_inflater,
            realm_connection.as_mut(),
            bot.character_guid,
            result,
        )
        .await?;
        *logged_out = true;
        result.movement.movement_smoke_passed = Some(true);
        return Ok(());
    }

    result.movement.movement_login_position = Some(login_point);
    if login_point.map_id != options.original_map_id {
        bail!(
            "login map {} differs from the offline snapshot map {}",
            login_point.map_id,
            options.original_map_id
        );
    }
    let ack = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(stream, crypt, CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, &ack).await?;
    pump_instance_socket(
        stream,
        crypt,
        server_inflater,
        Duration::from_millis(500),
        "movement pre-move drain",
        result,
    )
    .await?;

    let (player_low, player_high) = create_player_guid_raw(bot.character_guid, realm_id());
    let plan = build_movement_smoke_plan(&login_point);
    for step in &plan {
        let payload = movement_step_payload(step, player_low, player_high);
        send_encrypted_packet(stream, crypt, step.opcode, &payload).await?;
        result
            .movement
            .movement_opcodes_sent
            .push(movement_opcode_name(step.opcode).to_string());
        info!(
            "[Bot {}] ✅ {} t={} ({:.3},{:.3},{:.3})",
            bot_index,
            movement_opcode_name(step.opcode),
            step.client_time,
            step.x,
            step.y,
            step.z
        );
        pump_instance_socket(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(u64::from(MOVEMENT_SMOKE_STEP_MS)),
            "movement step drain",
            result,
        )
        .await?;
    }
    let last = plan.last().context("movement plan is empty")?;
    let final_point = MovementSmokePoint {
        map_id: login_point.map_id,
        x: last.x,
        y: last.y,
        z: last.z,
        orientation: last.orientation,
    };
    result.movement.movement_final_position = Some(final_point);
    pump_instance_socket(
        stream,
        crypt,
        server_inflater,
        Duration::from_millis(500),
        "movement post-stop drain",
        result,
    )
    .await?;

    loot_race::logout_and_wait_routed_like_cpp(
        bot_index,
        stream,
        crypt,
        server_inflater,
        realm_connection.as_mut(),
        bot.character_guid,
        result,
    )
    .await?;
    *logged_out = true;
    let (db_map, db_xyz) = load_persisted_position(bot.character_guid).await?;
    result.movement.movement_db_position = Some(db_xyz);
    if !position_within_tolerance(
        final_point.map_id,
        point_xyz(&final_point),
        db_map,
        db_xyz,
        MOVEMENT_SMOKE_POSITION_TOLERANCE,
    ) {
        bail!(
            "persisted position map={db_map} {db_xyz:?} does not match the final sent position {final_point:?}"
        );
    }
    result.movement.movement_db_persisted = true;
    result.movement.movement_smoke_passed = Some(true);
    Ok(())
}

pub(crate) fn cleanup_movement_smoke_fixture(
    bot: &config::BotConfig,
    fixture: &MovementSmokeFixture,
) -> Result<()> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|error| anyhow!("Bad characters DB URL: {error}"))?;
    let mut conn = mysql::Conn::new(opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    wait_for_fixture_character_offline(&mut conn, bot.character_guid, "movement")?;
    let position = &fixture.original_position;
    conn.exec_drop(
        "UPDATE characters SET map = ?, zone = ?, instance_id = ?, position_x = ?, position_y = ?, position_z = ?, orientation = ? \
         WHERE guid = ? AND online = 0",
        (
            position.map_id,
            position.zone_id,
            position.instance_id,
            position.x,
            position.y,
            position.z,
            position.orientation,
            bot.character_guid,
        ),
    )
    .map_err(|error| anyhow!("Restore movement bot position: {error}"))?;
    let (map, x, y, z) = load_bot_position(bot.character_guid)?;
    if !position_within_tolerance(
        position.map_id,
        [position.x, position.y, position.z],
        u32::from(map),
        [x, y, z],
        MOVEMENT_SMOKE_POSITION_TOLERANCE,
    ) {
        bail!("movement cleanup did not restore the original position");
    }
    info!(
        "Movement fixture cleaned: character={} restored map={} ({:.3},{:.3},{:.3})",
        bot.character_guid, position.map_id, position.x, position.y, position.z
    );
    Ok(())
}

pub(crate) async fn run_movement_smoke_workflow(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
) -> Result<BotRunResult> {
    let bot_for_setup = bot.clone();
    let fixture =
        tokio::task::spawn_blocking(move || prepare_movement_smoke_fixture(&bot_for_setup))
            .await
            .map_err(|error| anyhow!("Movement setup DB worker join failed: {error}"))??;
    let cleanup = |bot: config::BotConfig, fixture: MovementSmokeFixture| async move {
        tokio::task::spawn_blocking(move || cleanup_movement_smoke_fixture(&bot, &fixture))
            .await
            .map_err(|error| anyhow!("Movement cleanup DB worker join failed: {error}"))?
    };

    let first = run_bot_with_post_login_smoke(
        bot.clone(),
        dungeon_id,
        lfg_secs,
        auto_teleport,
        PostLoginSmokeOptions::Movement(fixture.options.clone()),
    )
    .await;
    let mut combined = match first {
        Ok(result) => result,
        Err(error) => {
            if let Err(cleanup_error) = cleanup(bot.clone(), fixture.clone()).await {
                bail!("Movement login/phase failed: {error}; position restore failed: {cleanup_error}");
            }
            return Err(error.context("Movement login/phase failed"));
        }
    };

    if combined.movement.movement_smoke_passed.unwrap_or(false) {
        let mut relog = fixture.options.clone();
        relog.phase = MovementSmokePhase::VerifyRelog;
        relog.expected_final = combined.movement.movement_final_position;
        match run_bot_with_post_login_smoke(
            bot.clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            PostLoginSmokeOptions::Movement(relog),
        )
        .await
        {
            Ok(second) => {
                combined.world_auth &= second.world_auth;
                combined.enum_characters &= second.enum_characters;
                combined.player_login_verified &= second.player_login_verified;
                combined.seen_opcodes.extend(second.seen_opcodes);
                let evidence = &mut combined.movement;
                evidence.movement_relogin_position = second.movement.movement_relogin_position;
                evidence.movement_relogin_verified = second.movement.movement_relogin_verified;
                evidence.movement_failure = second.movement.movement_failure;
                evidence.movement_smoke_passed = Some(
                    second.movement.movement_smoke_passed.unwrap_or(false)
                        && evidence.movement_relogin_verified,
                );
            }
            Err(error) => {
                combined.movement.movement_failure =
                    Some(format!("Movement persistence relog failed: {error}"));
                combined.movement.movement_smoke_passed = Some(false);
            }
        }
    }

    if let Err(error) = cleanup(bot.clone(), fixture.clone()).await {
        append_failure(
            &mut combined.movement.movement_failure,
            format!("Movement position restore failed: {error}"),
        );
        combined.movement.movement_smoke_passed = Some(false);
    }
    Ok(combined)
}
