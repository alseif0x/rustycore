//! Runtime operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) async fn run_detour_chase_capture_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &DetourChaseCaptureOptions,
    login_target: Option<DiscoveredCreatureGuid>,
    result: &mut BotRunResult,
) -> Result<()> {
    if realm_connection.is_none() {
        bail!("detour chase capture requires distinct authenticated realm/instance sockets");
    }
    validate_detour_fixture_identity(&bot.account, bot.character_guid)?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs.max(1));
    let clock_origin = tokio::time::Instant::now();
    let active_mover_complete = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &active_mover_complete,
    )
    .await?;
    result.detour_chase_active_mover_ack_sent = true;
    info!(
        "[Bot {}] ✅ detour CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE sent",
        bot_index
    );

    let mut target = login_target;
    let mut discovery_quiet_deadline = tokio::time::Instant::now() + DETOUR_CHASE_QUIET_PERIOD;
    while tokio::time::Instant::now() < deadline {
        let now = tokio::time::Instant::now();
        if target.is_some() && now >= discovery_quiet_deadline {
            break;
        }
        let readiness = if target.is_some() {
            discovery_quiet_deadline.saturating_duration_since(now)
        } else {
            Duration::from_millis(250)
        }
        .min(deadline.saturating_duration_since(now));
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            readiness,
            deadline.saturating_duration_since(now),
            "detour target discovery",
        )
        .await?
        else {
            continue;
        };
        discovery_quiet_deadline = tokio::time::Instant::now() + DETOUR_CHASE_QUIET_PERIOD;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance detour discovery {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            respond_to_detour_time_sync_like_cpp(
                bot_index,
                stream,
                crypt,
                &payload,
                clock_origin,
                "discovery",
            )
            .await?;
            result.detour_chase_time_sync_before_window += 1;
            continue;
        }
        if opcode != SMSG_UPDATE_OBJECT {
            continue;
        }
        let candidates = find_creature_guids_near_position_in_update_object(
            &payload,
            options.map_id,
            options.target_entry,
            options.target_x,
            options.target_y,
            options.target_z,
            DETOUR_CHASE_TARGET_MATCH_RADIUS,
            None,
        );
        if candidates.len() > 1 {
            bail!(
                "detour fixture discovery found {} same-entry live candidates inside the pinned spawn radius",
                candidates.len()
            );
        }
        let Some(candidate) = candidates.into_iter().next() else {
            continue;
        };
        if let Some(previous) = target {
            if (previous.low, previous.high) != (candidate.low, candidate.high) {
                bail!(
                    "detour fixture discovery produced more than one live target for spawn {}",
                    options.target_spawn_guid
                );
            }
        }
        target = Some(candidate);
    }
    let target = target.ok_or_else(|| {
        anyhow!(
            "did not discover exact detour fixture creature entry={} spawn={} within {:.2} yards of ({:.3},{:.3},{:.3})",
            options.target_entry,
            options.target_spawn_guid,
            DETOUR_CHASE_TARGET_MATCH_RADIUS,
            options.target_x,
            options.target_y,
            options.target_z
        )
    })?;
    // Creature::LoadFromDB keeps the persistent spawnId in m_spawnId but calls
    // Map::GenerateLowGuid<HighGuid::Creature>() for the network ObjectGuid
    // counter (legacy Creature.cpp). Therefore the runtime counter is evidence,
    // not a DB identity. The fixture identity is established independently by
    // the guarded spawn row plus the unique entry/map/position match above.
    result.detour_chase_target_runtime_counter = Some(target.low);
    result.detour_chase_target_discovered = true;
    let target_guid = (target.low, target.high);
    let player_guid = create_player_guid_raw(bot.character_guid, realm_id());
    info!(
        "[Bot {}] ✅ exact detour target discovered entry={} spawn={} counter={} at ({:.3},{:.3},{:.3})",
        bot_index,
        options.target_entry,
        options.target_spawn_guid,
        target.low,
        target.x,
        target.y,
        target.z
    );

    let packed_target = build_packed_guid(target.low, target.high);
    send_encrypted_packet(stream, crypt, CMSG_ATTACK_SWING, &packed_target).await?;
    info!(
        "[Bot {}] ✅ detour CMSG_ATTACK_SWING sent to exact fixture target",
        bot_index
    );
    let mut first_swing_confirmed = false;
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for matching SMSG_ATTACK_START"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance detour attack confirmation {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        match opcode {
            SMSG_TIME_SYNC_REQUEST => {
                respond_to_detour_time_sync_like_cpp(
                    bot_index,
                    stream,
                    crypt,
                    &payload,
                    clock_origin,
                    "attack confirmation",
                )
                .await?;
                result.detour_chase_time_sync_before_window += 1;
            }
            SMSG_ON_MONSTER_MOVE if monster_move_mover_guid_like_cpp(&payload)? == target_guid => {
                result.detour_chase_prewindow_target_moves += 1;
            }
            SMSG_ATTACKER_STATE_UPDATE => {
                let update = parse_attacker_state_update_summary(&payload)
                    .context("malformed detour SMSG_ATTACKER_STATE_UPDATE")?;
                first_swing_confirmed |= (update.attacker_guid_low, update.attacker_guid_high)
                    == player_guid
                    && (update.victim_guid_low, update.victim_guid_high) == target_guid;
            }
            SMSG_ATTACK_START => {
                let (attacker, victim) = parse_attack_start_guids_like_cpp(&payload)?;
                if attacker == player_guid && victim == target_guid {
                    result.detour_chase_attack_start_confirmed = true;
                    break;
                }
            }
            _ => {}
        }
    }
    if !result.detour_chase_attack_start_confirmed {
        bail!("matching SMSG_ATTACK_START was not observed before the detour deadline");
    }

    while !first_swing_confirmed {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for the first accepted detour player swing");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for the first accepted detour player swing"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance detour first-swing wait {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        match opcode {
            SMSG_TIME_SYNC_REQUEST => {
                respond_to_detour_time_sync_like_cpp(
                    bot_index,
                    stream,
                    crypt,
                    &payload,
                    clock_origin,
                    "first-swing wait",
                )
                .await?;
                result.detour_chase_time_sync_before_window += 1;
            }
            SMSG_ON_MONSTER_MOVE if monster_move_mover_guid_like_cpp(&payload)? == target_guid => {
                result.detour_chase_prewindow_target_moves += 1;
            }
            SMSG_ATTACKER_STATE_UPDATE => {
                let update = parse_attacker_state_update_summary(&payload)
                    .context("malformed detour SMSG_ATTACKER_STATE_UPDATE")?;
                first_swing_confirmed |= (update.attacker_guid_low, update.attacker_guid_high)
                    == player_guid
                    && (update.victim_guid_low, update.victim_guid_high) == target_guid;
            }
            _ => {}
        }
    }
    result.detour_chase_first_swing_confirmed = true;

    // Drain the accepted swing's immediate combat/chase side effects. The
    // capture starts only after a complete quiet period, so no pre-action
    // MonsterMove can be mistaken for the response to the pinned heartbeat.
    drain_detour_instance_until_quiet(
        bot_index,
        "accepted swing / initial chase",
        stream,
        crypt,
        server_inflater,
        target_guid,
        deadline,
        clock_origin,
        result,
    )
    .await?;

    let heartbeat = build_move_heartbeat_payload(
        player_guid.0,
        player_guid.1,
        options.destination_x,
        options.destination_y,
        options.destination_z,
        options.destination_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &heartbeat).await?;
    result.detour_chase_heartbeat_sent = true;
    result.detour_chase_heartbeat_sha256 = Some(format!("{:x}", Sha256::digest(&heartbeat)));
    info!(
        "[Bot {}] ✅ detour capture anchor CMSG_MOVE_HEARTBEAT sent exactly once to ({:.3},{:.3},{:.3},{:.6})",
        bot_index,
        options.destination_x,
        options.destination_y,
        options.destination_z,
        options.destination_orientation
    );

    // Exact capture window: after removing only the documented periodic
    // time-sync pair, the next server packet must be one target MonsterMove.
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for detour SMSG_ON_MONSTER_MOVE capture anchor");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for detour movement anchor"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance detour isolated window {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            respond_to_detour_time_sync_like_cpp(
                bot_index,
                stream,
                crypt,
                &payload,
                clock_origin,
                "isolated-window",
            )
            .await?;
            result.detour_chase_time_sync_during_window += 1;
            continue;
        }
        if opcode == SMSG_UPDATE_OBJECT {
            // Combat VALUES fanout is deliberately excluded symmetrically by
            // the flow import. Its ordering relative to chase movement differs
            // between runtimes and is not evidence for Detour path geometry.
            continue;
        }
        if opcode != SMSG_ON_MONSTER_MOVE {
            bail!(
                "unexpected opcode 0x{opcode:04X} inside detour capture window before movement anchor"
            );
        }
        let mover = monster_move_mover_guid_like_cpp(&payload)?;
        if mover != target_guid {
            bail!(
                "SMSG_ON_MONSTER_MOVE inside detour capture window belonged to another object ({:016X}:{:016X})",
                mover.1,
                mover.0
            );
        }
        result.detour_chase_window_target_moves += 1;
        result.detour_chase_monster_move_sha256 = Some(format!("{:x}", Sha256::digest(&payload)));
        result.detour_chase_monster_move_bytes = Some(payload.len());
        break;
    }

    let ping = build_ping_payload(ISSUE_24_PING_FENCE_SERIAL);
    send_encrypted_packet(stream, crypt, CMSG_PING, &ping).await?;
    result.detour_chase_ping_serial = Some(ISSUE_24_PING_FENCE_SERIAL);
    info!(
        "[Bot {}] ✅ detour capture fence CMSG_PING serial=0x{:08X}",
        bot_index, ISSUE_24_PING_FENCE_SERIAL
    );

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for detour capture-fence SMSG_PONG");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for detour capture-fence SMSG_PONG"))??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance detour post-fence {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        match opcode {
            SMSG_TIME_SYNC_REQUEST => {
                respond_to_detour_time_sync_like_cpp(
                    bot_index,
                    stream,
                    crypt,
                    &payload,
                    clock_origin,
                    "post-fence",
                )
                .await?;
                result.detour_chase_time_sync_after_fence += 1;
            }
            SMSG_PONG => {
                let serial = parse_pong_serial_like_cpp(&payload)?;
                if serial != ISSUE_24_PING_FENCE_SERIAL {
                    bail!(
                        "detour capture-fence SMSG_PONG serial mismatch: expected 0x{:08X}, got 0x{serial:08X}",
                        ISSUE_24_PING_FENCE_SERIAL
                    );
                }
                result.detour_chase_pong_confirmed = true;
                break;
            }
            // CMSG_PING is the exclusive end fence selected by capture-diff.
            // Packets observed after it are outside the imported window; keep
            // draining them until the correlated PONG proves server receipt.
            _ => {}
        }
    }

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
    result.detour_chase_logout_confirmed = true;
    result.detour_chase_capture_passed = Some(true);
    Ok(())
}
pub(crate) async fn run_rested_xp_smoke_phase(
    bot_index: usize,
    bot: &config::BotConfig,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    options: &RestedXpSmokeOptions,
    result: &mut BotRunResult,
) -> Result<()> {
    match options.phase {
        RestedXpSmokePhase::OfflineWilderness | RestedXpSmokePhase::OfflineResting => {
            disconnect_rested_xp_and_wait(
                bot_index,
                bot,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options.timeout_secs,
                result,
            )
            .await?;
            result.rested_xp_smoke_passed = Some(true);
            return Ok(());
        }
        RestedXpSmokePhase::VerifyRelog => {
            let expected_xp = options
                .expected_xp
                .context("rested-XP relog phase missing expected XP")?;
            let expected_rest_bonus = options
                .expected_rest_bonus
                .context("rested-XP relog phase missing expected rest bonus")?;
            let before_logout = load_rested_xp_db_state_async(bot.clone()).await?;
            validate_rested_xp_persistence_state(
                before_logout,
                options.test_level,
                expected_xp,
                expected_rest_bonus,
                1,
                "after relog",
            )?;
            disconnect_rested_xp_and_wait(
                bot_index,
                bot,
                stream,
                crypt,
                server_inflater,
                realm_connection,
                options.timeout_secs,
                result,
            )
            .await?;
            let after_logout = load_rested_xp_db_state_async(bot.clone()).await?;
            validate_rested_xp_persistence_state(
                after_logout,
                options.test_level,
                expected_xp,
                expected_rest_bonus,
                0,
                "after relog logout",
            )?;
            result.rested_xp_relog_verified = true;
            result.rested_xp_smoke_passed = Some(true);
            return Ok(());
        }
        RestedXpSmokePhase::ConsumeKill => {}
    }

    let realm = realm_connection.as_mut().context(
        "rested-XP smoke requires distinct realm/instance sockets to validate XP routing",
    )?;
    let realm_drain_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < realm_drain_deadline {
        let remaining = realm_drain_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "rested-XP realm login drain",
        )
        .await?
        else {
            break;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 realm rested-XP login drain {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
    }

    // C++ Player::CanNeverSee returns true until the client acknowledges that
    // its active mover is initialized. Send the ACK before discovery because
    // C++ may defer the target's visibility update until this packet, whereas
    // Rust currently can have queued the CREATE slightly earlier.
    let active_mover_complete = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(
        stream,
        crypt,
        CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
        &active_mover_complete,
    )
    .await?;
    info!(
        "[Bot {}] ✅ CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE sent before live target discovery",
        bot_index
    );

    // Search until the declared deadline, not merely until the first 250 ms
    // gap. This also drains the visibility work caused by the active-mover ACK.
    // Stop as soon as the CREATE_OBJECT candidate is decoded so its live
    // position is still fresh enough for deterministic engagement below.
    let expected_runtime_counter = (options.target.guid_counter != 0)
        .then_some(options.target.guid_counter & OBJECT_GUID_COUNTER_MASK);
    let mut discovered = None;
    let discovery_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while discovered.is_none() && tokio::time::Instant::now() < discovery_deadline {
        let remaining = discovery_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            Duration::from_millis(250).min(remaining),
            Duration::from_secs(5),
            "rested-XP instance login discovery",
        )
        .await?
        else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_UPDATE_OBJECT {
            discovered = find_creature_guid_near_position_in_update_object(
                &payload,
                options.target.map_id,
                options.target.entry,
                options.target.x as f32,
                options.target.y as f32,
                options.target.z as f32,
                options.target_match_radius,
                expected_runtime_counter,
            );
        }
    }

    let candidate = resolve_rested_xp_runtime_target(&options.target, discovered)?;
    let player_x = candidate.x + 1.0;
    let player_y = candidate.y;
    let player_z = candidate.z;
    let player_orientation = (candidate.y - player_y).atan2(candidate.x - player_x);
    let player_distance = ((candidate.x - player_x).powi(2)
        + (candidate.y - player_y).powi(2)
        + (candidate.z - player_z).powi(2))
    .sqrt();
    if player_distance > NOMINAL_MELEE_RANGE_LIKE_CPP {
        bail!(
            "rested-XP live engagement placement remained {player_distance:.2} yards from the target (C++ nominal melee range is {NOMINAL_MELEE_RANGE_LIKE_CPP:.2})"
        );
    }
    let (player_low, player_high) = create_player_guid_raw(bot.character_guid, realm_id());
    let movement = build_move_heartbeat_payload(
        player_low,
        player_high,
        player_x,
        player_y,
        player_z,
        player_orientation,
    );
    send_encrypted_packet(stream, crypt, CMSG_MOVE_HEARTBEAT, &movement).await?;
    info!(
        "[Bot {}] ✅ CMSG_MOVE_HEARTBEAT placed the bot {:.2} yards from the live target and facing it",
        bot_index, player_distance
    );
    let (runtime_low, runtime_high) = (candidate.low, candidate.high);
    result.rested_xp_target_guid_counter = Some(runtime_low & OBJECT_GUID_COUNTER_MASK);
    let packed_target = build_packed_guid(runtime_low, runtime_high);
    send_encrypted_packet(stream, crypt, CMSG_ATTACK_SWING, &packed_target).await?;
    info!(
        "[Bot {}] ✅ CMSG_ATTACK_SWING sent on instance to entry={} spawn={} counter={}",
        bot_index,
        options.target.entry,
        options.target.spawn_guid,
        runtime_low & OBJECT_GUID_COUNTER_MASK
    );

    enum RestedXpReady {
        Instance,
        Realm,
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    let client_clock_origin = tokio::time::Instant::now();
    let mut player_damage_observed = 0i64;
    let mut target_death_observed = false;
    let xp_gain = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for realm SMSG_LOG_XP_GAIN after observing {player_damage_observed} player damage (target_death={target_death_observed})"
            );
        }
        let mut instance_peek = [0u8; 1];
        let mut realm_peek = [0u8; 1];
        let ready = tokio::time::timeout(remaining, async {
            tokio::select! {
                ready = stream.peek(&mut instance_peek) => {
                    if ready.context("rested-XP instance peek failed")? == 0 {
                        bail!("instance connection closed during rested-XP combat");
                    }
                    Ok(RestedXpReady::Instance)
                }
                ready = realm.stream.peek(&mut realm_peek) => {
                    if ready.context("rested-XP realm peek failed")? == 0 {
                        bail!("realm connection closed during rested-XP combat");
                    }
                    Ok(RestedXpReady::Realm)
                }
            }
        })
        .await
        .map_err(|_| {
            anyhow!(
                "timed out waiting for rested-XP combat packets after observing {player_damage_observed} player damage (target_death={target_death_observed})"
            )
        })??;
        let (connection, opcode, payload) = match ready {
            RestedXpReady::Instance => {
                let (opcode, payload) = tokio::time::timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(stream, crypt, server_inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP instance packet read timed out"))??;
                ("instance", opcode, payload)
            }
            RestedXpReady::Realm => {
                let (opcode, payload) = tokio::time::timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("rested-XP realm packet read timed out"))??;
                ("realm", opcode, payload)
            }
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 {} rested-XP {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );
        if opcode == SMSG_TIME_SYNC_REQUEST {
            if connection != "instance" {
                bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during rested-XP combat");
            }
            let sequence_index = parse_time_sync_request_sequence(&payload)?;
            let client_time = client_clock_origin.elapsed().as_millis() as u32;
            let response = build_time_sync_response_payload(sequence_index, client_time);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            info!(
                "[Bot {}] ✅ rested-XP CMSG_TIME_SYNC_RESPONSE sent (sequence={}, client_time={})",
                bot_index, sequence_index, client_time
            );
            continue;
        }
        if opcode == SMSG_ATTACKER_STATE_UPDATE {
            let update = parse_attacker_state_update_summary(&payload)
                .context("malformed SMSG_ATTACKER_STATE_UPDATE during rested-XP combat")?;
            if (update.attacker_guid_low, update.attacker_guid_high) == (player_low, player_high)
                && (update.victim_guid_low, update.victim_guid_high) == (runtime_low, runtime_high)
            {
                if update.damage < 0 {
                    bail!(
                        "rested-XP player auto-attack reported negative damage {}",
                        update.damage
                    );
                }
                // C++ can emit zero-damage MISS/DODGE/PARRY swings. They are
                // valid combat progress, but only positive damage contributes
                // to the proof that this bot killed the selected target.
                if update.damage > 0 {
                    player_damage_observed += i64::from(update.damage);
                }
                target_death_observed |= update.over_damage >= 0;
            }
        }
        if opcode == SMSG_ATTACK_STOP {
            let stop = parse_attack_stop_summary(&payload)
                .context("malformed SMSG_ATTACK_STOP payload during rested-XP combat")?;
            let attacker = (stop.attacker_guid_low, stop.attacker_guid_high);
            let victim = (stop.victim_guid_low, stop.victim_guid_high);
            if attacker == (runtime_low, runtime_high) && victim == (player_low, player_high) {
                if stop.now_dead {
                    bail!("the rested-XP target killed the bot before XP was awarded");
                }
                // C++ CombatStop can emit the reciprocal creature->player stop
                // before the player's target-death stop.
                continue;
            }
            if attacker != (player_low, player_high) || victim != (runtime_low, runtime_high) {
                continue;
            }
            if stop.now_dead {
                // C++ sends AttackStop when the victim dies. XP can arrive on
                // the realm connection immediately before or after this
                // instance packet, so keep polling both sockets.
                target_death_observed = true;
                continue;
            }
            bail!("server stopped the rested-XP attack while the target was still alive");
        }
        if opcode != SMSG_LOG_XP_GAIN {
            continue;
        }
        if connection != "realm" {
            bail!("SMSG_LOG_XP_GAIN arrived on instance; C++ routes it on realm");
        }
        let gain =
            parse_log_xp_gain_summary(&payload).context("malformed SMSG_LOG_XP_GAIN payload")?;
        break gain;
    };

    if (xp_gain.victim_guid_low, xp_gain.victim_guid_high) != (runtime_low, runtime_high) {
        bail!("SMSG_LOG_XP_GAIN victim did not match the attacked creature");
    }
    if player_damage_observed <= 0 {
        bail!("rested-XP reward arrived without any positive player damage to the target");
    }
    if xp_gain.reason != 0 || xp_gain.amount <= 0 {
        bail!(
            "SMSG_LOG_XP_GAIN was not a positive kill reward: reason={} amount={}",
            xp_gain.reason,
            xp_gain.amount
        );
    }
    if xp_gain.original != xp_gain.amount.saturating_mul(2) {
        bail!(
            "rested kill was not exactly 200% XP: amount={} original={}",
            xp_gain.amount,
            xp_gain.original
        );
    }
    if (xp_gain.group_bonus - 1.0).abs() > f32::EPSILON {
        bail!("unexpected rested-XP group bonus: {}", xp_gain.group_bonus);
    }
    let expected_xp = u32::try_from(xp_gain.original)
        .context("positive SMSG_LOG_XP_GAIN original did not fit u32")?;
    let spent = xp_gain.amount as f32;
    let expected_rest_bonus = options.seeded_rest_bonus - spent;
    if expected_rest_bonus < 0.0 {
        bail!("fixture rest bonus was smaller than the awarded base XP");
    }
    observe_instance_after_realm_xp(bot_index, stream, crypt, server_inflater, result).await?;
    // C++ Player::GiveXP mutates update fields in memory; persistence occurs
    // during the later character save. Disconnect both sockets and wait for a
    // stable offline row before asserting DB state so this workflow is valid
    // against both the legacy server and RustyCore. This also avoids coupling
    // rested-XP QA to C++'s unrelated 20-second wilderness logout delay.
    disconnect_rested_xp_and_wait(
        bot_index,
        bot,
        stream,
        crypt,
        server_inflater,
        realm_connection,
        options.timeout_secs,
        result,
    )
    .await?;
    let persisted = load_rested_xp_db_state_async(bot.clone()).await?;
    validate_rested_xp_persistence_state(
        persisted,
        options.test_level,
        expected_xp,
        expected_rest_bonus,
        0,
        "after rested-XP disconnect save",
    )?;
    result.rested_xp_packet_amount = Some(xp_gain.amount);
    result.rested_xp_packet_original = Some(xp_gain.original);
    result.rested_xp_db_xp_before = Some(0);
    result.rested_xp_db_xp_after = Some(persisted.xp);
    result.rested_xp_db_rest_before = Some(options.seeded_rest_bonus);
    result.rested_xp_db_rest_after = Some(persisted.rest_bonus);
    result.rested_xp_smoke_passed = Some(true);
    Ok(())
}
