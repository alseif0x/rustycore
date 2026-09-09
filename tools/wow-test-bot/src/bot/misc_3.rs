//! Misc operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreateOnlyProvisioningPlan {
    ValidateExisting,
    CreateBoth,
}
pub(crate) fn create_only_provisioning_plan(
    bnet_exists: bool,
    game_account_exists: bool,
) -> Result<CreateOnlyProvisioningPlan> {
    match (bnet_exists, game_account_exists) {
        (true, true) => Ok(CreateOnlyProvisioningPlan::ValidateExisting),
        (false, false) => Ok(CreateOnlyProvisioningPlan::CreateBoth),
        (true, false) => bail!(
            "Refusing create-only provisioning: a BNet identity already exists without the configured game account"
        ),
        (false, true) => bail!(
            "Refusing create-only provisioning: the numeric game-account ID already exists without the configured BNet identity"
        ),
    }
}
pub(crate) fn game_account_username(account: &str) -> Result<String> {
    let local_part = account
        .split('@')
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| anyhow!("Bot account {account} has no local part"))?;
    let username = bot_srp6::utf8_to_upper_only_latin_like_cpp(local_part);
    if username.len() > 32 {
        bail!("Bot username {username} exceeds Trinity account.username limit");
    }
    Ok(username)
}
pub(crate) fn fixed_le_32(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.resize(32, 0);
    bytes.truncate(32);
    bytes
}
pub(crate) fn random_32() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}
pub(crate) fn stand_state_smoke_options_from_cli(
    cli: &CliOptions,
) -> Result<StandStateSmokeOptions> {
    if cli.stand_state_timeout_secs == 0 {
        bail!("--stand-state-timeout must be greater than zero");
    }

    let states = match cli.stand_state {
        Some(state) if is_client_stand_state_like_cpp(state) => vec![state],
        Some(state) => bail!(
            "unsupported stand state {state}; expected 0 (Stand), 1 (Sit), 3 (Sleep), or 8 (Kneel)"
        ),
        None => vec![UNIT_STAND_STATE_SIT, UNIT_STAND_STATE_STAND],
    };

    Ok(StandStateSmokeOptions {
        states,
        timeout_secs: cli.stand_state_timeout_secs,
    })
}
pub(crate) fn is_client_stand_state_like_cpp(state: u8) -> bool {
    matches!(
        state,
        UNIT_STAND_STATE_STAND
            | UNIT_STAND_STATE_SIT
            | UNIT_STAND_STATE_SLEEP
            | UNIT_STAND_STATE_KNEEL
    )
}
pub(crate) fn detour_chase_failure_result(
    bot: &config::BotConfig,
    dungeon_id: u32,
    options: &DetourChaseCaptureOptions,
    failure: String,
) -> BotRunResult {
    BotRunResult {
        account: bot.account.clone(),
        account_id: bot.account_id,
        character_guid: bot.character_guid,
        dungeon_id,
        role: bot.lfg_role,
        detour_chase_capture: true,
        detour_chase_capture_passed: Some(false),
        detour_chase_target_entry: Some(options.target_entry),
        detour_chase_target_spawn_guid: Some(options.target_spawn_guid),
        detour_chase_failure: Some(failure),
        ..BotRunResult::default()
    }
}
/// Opcodes.cpp registers SMSG_STAND_STATE_UPDATE on CONNECTION_TYPE_REALM.
/// When login created a separate instance socket, accepting this opcode from
/// that instance connection would hide a routing-parity bug.
pub(crate) async fn wait_for_stand_state_update(
    bot_index: usize,
    connection_name: &str,
    separate_realm_connection: bool,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    expected_state: u8,
    result: &mut BotRunResult,
) -> Result<StandStateDrainSummary> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut side_effects = StandStateDrainSummary::default();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for realm SMSG_STAND_STATE_UPDATE state {}",
                expected_state
            );
        }

        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "timed out waiting for realm SMSG_STAND_STATE_UPDATE state {}",
                expected_state
            )
        })??;
        result.seen_opcodes.push(format!("0x{:04X}", opcode));
        info!(
            "[Bot {}] 📦 {} {}",
            bot_index,
            connection_name,
            parse_packet(opcode, &payload)
        );

        if opcode != SMSG_STAND_STATE_UPDATE {
            if matches!(opcode, SMSG_UPDATE_OBJECT | SMSG_AURA_UPDATE) {
                if separate_realm_connection {
                    bail!(
                        "{} arrived on the separate realm connection before StandStateUpdate; C++ routes stand side effects on instance",
                        parse_packet(opcode, &payload)
                    );
                }
                match opcode {
                    SMSG_UPDATE_OBJECT => side_effects.active_update_objects += 1,
                    SMSG_AURA_UPDATE => side_effects.active_aura_updates += 1,
                    _ => unreachable!(),
                }
            }
            continue;
        }

        validate_stand_state_update(&payload, expected_state)?;
        result.stand_states_confirmed.push(expected_state);
        info!(
            "[Bot {}] ✅ realm SMSG_STAND_STATE_UPDATE matched (animKitID=0, state={})",
            bot_index, expected_state
        );
        return Ok(side_effects);
    }
}
#[derive(Debug, Default)]
pub(crate) struct StandStateDrainSummary {
    pub(crate) active_update_objects: usize,
    pub(crate) active_aura_updates: usize,
}
pub(crate) async fn send_and_verify_stand_state_capture_fence(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let payload = build_ping_payload(STAND_STATE_CAPTURE_FENCE_SERIAL);
    send_encrypted_packet(stream, crypt, CMSG_PING, &payload).await?;
    info!(
        "[Bot {}] ✅ deterministic CMSG_PING capture fence sent (serial=0x{:08X})",
        bot_index, STAND_STATE_CAPTURE_FENCE_SERIAL
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for stand-state capture-fence SMSG_PONG");
        }
        let (opcode, payload) = tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        .map_err(|_| anyhow!("timed out waiting for stand-state capture-fence SMSG_PONG"))??;
        result.seen_opcodes.push(format!("0x{:04X}", opcode));
        info!(
            "[Bot {}] 📦 active capture-fence {}",
            bot_index,
            parse_packet(opcode, &payload)
        );
        if opcode != SMSG_PONG {
            continue;
        }
        if payload != STAND_STATE_CAPTURE_FENCE_SERIAL.to_le_bytes() {
            bail!(
                "capture-fence SMSG_PONG mismatch: expected serial 0x{:08X}, got {:02X?}",
                STAND_STATE_CAPTURE_FENCE_SERIAL,
                payload
            );
        }
        return Ok(());
    }
}
/// C++ `WorldPackets::Auth::Ping::Read`: uint32 serial then uint32 latency.
pub(crate) fn build_ping_payload(serial: u32) -> [u8; 8] {
    let mut payload = [0u8; 8];
    payload[..4].copy_from_slice(&serial.to_le_bytes());
    payload
}
/// C++ `TimeSyncResponse::Read`: request sequence then client ticks in ms.
pub(crate) fn build_time_sync_response_payload(sequence_index: u32, client_time: u32) -> [u8; 8] {
    let mut payload = [0u8; 8];
    payload[..4].copy_from_slice(&sequence_index.to_le_bytes());
    payload[4..].copy_from_slice(&client_time.to_le_bytes());
    payload
}
/// C++ WorldPackets::Misc::StandStateChange::Read: one little-endian uint32.
pub(crate) fn build_stand_state_change(state: u8) -> [u8; 4] {
    u32::from(state).to_le_bytes()
}
/// C++ StandStateUpdate::Write: uint32 AnimKitID followed by uint8 State.
pub(crate) fn validate_stand_state_update(payload: &[u8], expected_state: u8) -> Result<()> {
    if payload.len() != 5 {
        bail!(
            "SMSG_STAND_STATE_UPDATE payload length mismatch: expected 5, got {}",
            payload.len()
        );
    }

    let anim_kit_id = u32::from_le_bytes(payload[0..4].try_into()?);
    let state = payload[4];
    if anim_kit_id != 0 {
        bail!(
            "SMSG_STAND_STATE_UPDATE AnimKitID mismatch: expected 0, got {}",
            anim_kit_id
        );
    }
    if state != expected_state {
        bail!(
            "SMSG_STAND_STATE_UPDATE state mismatch: expected {}, got {}",
            expected_state,
            state
        );
    }

    Ok(())
}
pub(crate) async fn prepare_rested_xp_phase_async(
    bot: config::BotConfig,
    fixture: RestedXpSmokeFixture,
    phase: RestedXpSmokePhase,
) -> Result<()> {
    tokio::task::spawn_blocking(move || prepare_rested_xp_character_phase(&bot, &fixture, phase))
        .await
        .map_err(|error| anyhow!("Rested-XP phase setup worker failed: {error}"))?
}
pub(crate) async fn load_rested_xp_db_state_async(
    bot: config::BotConfig,
) -> Result<RestedXpDbState> {
    tokio::task::spawn_blocking(move || load_rested_xp_db_state(&bot))
        .await
        .map_err(|error| anyhow!("Rested-XP DB state worker failed: {error}"))?
}
pub(crate) fn set_rested_xp_failure(result: &mut BotRunResult, message: String) {
    result.rested_xp_smoke_passed = Some(false);
    result.rested_xp_failure = Some(message);
}
pub(crate) fn merge_rested_xp_results(combined: &mut BotRunResult, next: BotRunResult) {
    combined.world_auth &= next.world_auth;
    combined.enum_characters &= next.enum_characters;
    combined.player_login_verified &= next.player_login_verified;
    combined.seen_opcodes.extend(next.seen_opcodes);
    combined.rested_xp_smoke_passed = Some(
        combined.rested_xp_smoke_passed.unwrap_or(false)
            && next.rested_xp_smoke_passed.unwrap_or(false),
    );
    if next.rested_xp_target_guid_counter.is_some() {
        combined.rested_xp_target_guid_counter = next.rested_xp_target_guid_counter;
    }
    if next.rested_xp_packet_amount.is_some() {
        combined.rested_xp_packet_amount = next.rested_xp_packet_amount;
        combined.rested_xp_packet_original = next.rested_xp_packet_original;
        combined.rested_xp_db_xp_before = next.rested_xp_db_xp_before;
        combined.rested_xp_db_xp_after = next.rested_xp_db_xp_after;
        combined.rested_xp_db_rest_before = next.rested_xp_db_rest_before;
        combined.rested_xp_db_rest_after = next.rested_xp_db_rest_after;
    }
    combined.rested_xp_relog_verified |= next.rested_xp_relog_verified;
    if next.rested_xp_failure.is_some() {
        combined.rested_xp_failure = next.rested_xp_failure;
    }
}
pub(crate) fn offline_rest_bonus_like_cpp(
    next_level_xp: u32,
    offline_secs: u64,
    bubble: f32,
    rate: f32,
) -> f32 {
    let extra = offline_secs as f32 * next_level_xp as f32 / 72_000.0 * bubble * rate;
    extra.clamp(0.0, next_level_xp as f32 * REST_BONUS_CAP_NEXT_LEVEL_FACTOR)
}
pub(crate) fn offline_rest_bonus_matches_like_cpp(
    actual: f32,
    expected: f32,
    next_level_xp: u32,
    bubble: f32,
    rate: f32,
    timeout_secs: u64,
) -> bool {
    let per_second = next_level_xp as f32 / 72_000.0 * bubble * rate.abs();
    let timing_slop = timeout_secs.saturating_add(30) as f32 * per_second;
    (actual - expected).abs() <= timing_slop.max(0.05)
}
pub(crate) fn required_indexed_row_values<T, const N: usize>(
    row: &mysql::Row,
    prefix: &str,
) -> Result<[T; N]>
where
    T: mysql::prelude::FromValue,
{
    let mut values = Vec::with_capacity(N);
    for index in 0..N {
        values.push(required_row_value(row, &format!("{prefix}{index}"))?);
    }
    values
        .try_into()
        .map_err(|_| anyhow!("Expected exactly {N} `{prefix}` columns in QA fixture query"))
}
pub(crate) fn expected_transmog_outfit_db_row(
    options: &EquipmentSetSmokeOptions,
    expected_guid: u64,
) -> TransmogOutfitDbRow {
    TransmogOutfitDbRow {
        set_guid: expected_guid,
        set_index: options.set_id,
        name: options.set_name.clone(),
        icon_name: options.set_icon.clone(),
        ignore_mask: EQUIPMENT_SET_IGNORE_ALL_SLOTS_LIKE_CPP,
        appearances: [0; EQUIPMENT_SET_SLOTS_LIKE_CPP],
        main_hand_enchant: 0,
        off_hand_enchant: 0,
    }
}
pub(crate) fn push_msb_bits(data: &mut Vec<u8>, bit_offset: &mut usize, value: u32, count: usize) {
    for shift in (0..count).rev() {
        if *bit_offset % 8 == 0 {
            data.push(0);
        }
        if (value >> shift) & 1 != 0 {
            let byte = data.len() - 1;
            data[byte] |= 1 << (7 - (*bit_offset % 8));
        }
        *bit_offset += 1;
    }
}
pub(crate) fn parse_attack_start_guids_like_cpp(
    payload: &[u8],
) -> Result<((u64, u64), (u64, u64))> {
    // C++ `WorldPackets::Combat::AttackStart::Write` serializes two
    // ObjectGuids in attacker/victim order.
    let (attacker_len, attacker_low, attacker_high) =
        parse_packed_guid(payload).context("SMSG_ATTACK_START missing attacker ObjectGuid")?;
    let (victim_len, victim_low, victim_high) = parse_packed_guid(
        payload
            .get(attacker_len..)
            .context("SMSG_ATTACK_START attacker length exceeds payload")?,
    )
    .context("SMSG_ATTACK_START missing victim ObjectGuid")?;
    if attacker_len + victim_len != payload.len() {
        bail!(
            "SMSG_ATTACK_START has {} trailing bytes after its two C++ ObjectGuids",
            payload.len() - attacker_len - victim_len
        );
    }
    Ok(((attacker_low, attacker_high), (victim_low, victim_high)))
}
pub(crate) fn parse_pong_serial_like_cpp(payload: &[u8]) -> Result<u32> {
    let bytes: [u8; 4] = payload
        .try_into()
        .map_err(|_| anyhow!("SMSG_PONG payload must contain exactly one uint32 serial"))?;
    Ok(u32::from_le_bytes(bytes))
}
pub(crate) async fn respond_to_detour_time_sync_like_cpp(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    payload: &[u8],
    clock_origin: tokio::time::Instant,
    phase: &str,
) -> Result<()> {
    let sequence = parse_time_sync_request_sequence(payload)?;
    let client_time = u32::try_from(clock_origin.elapsed().as_millis()).unwrap_or(u32::MAX);
    let response = build_time_sync_response_payload(sequence, client_time);
    send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
    info!(
        "[Bot {}] ✅ detour {} CMSG_TIME_SYNC_RESPONSE sequence={} client_time={}",
        bot_index, phase, sequence, client_time
    );
    Ok(())
}
pub(crate) async fn drain_detour_instance_until_quiet(
    bot_index: usize,
    label: &str,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    target_guid: (u64, u64),
    deadline: tokio::time::Instant,
    clock_origin: tokio::time::Instant,
    result: &mut BotRunResult,
) -> Result<()> {
    let mut quiet_deadline = tokio::time::Instant::now() + DETOUR_CHASE_QUIET_PERIOD;
    loop {
        let now = tokio::time::Instant::now();
        if now >= quiet_deadline {
            return Ok(());
        }
        if now >= deadline {
            bail!("timed out draining {label} before the detour capture window");
        }
        let wait = quiet_deadline
            .saturating_duration_since(now)
            .min(deadline.saturating_duration_since(now));
        let packet = read_encrypted_packet_if_ready(
            stream,
            crypt,
            server_inflater,
            wait,
            deadline.saturating_duration_since(now),
            label,
        )
        .await?;
        let Some((opcode, payload)) = packet else {
            continue;
        };
        quiet_deadline = tokio::time::Instant::now() + DETOUR_CHASE_QUIET_PERIOD;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 instance detour {} {}",
            bot_index,
            label,
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
                    "pre-window",
                )
                .await?;
                result.detour_chase_time_sync_before_window += 1;
            }
            SMSG_ON_MONSTER_MOVE if monster_move_mover_guid_like_cpp(&payload)? == target_guid => {
                result.detour_chase_prewindow_target_moves += 1;
            }
            _ => {}
        }
    }
}
pub(crate) fn validate_rested_xp_persistence_state(
    state: RestedXpDbState,
    expected_level: u8,
    expected_xp: u32,
    expected_rest_bonus: f32,
    expected_online: u8,
    phase: &str,
) -> Result<()> {
    validate_rested_xp_saved_state_shape(
        state,
        expected_level,
        expected_xp,
        expected_online,
        phase,
    )?;
    if (state.rest_bonus - expected_rest_bonus).abs() > 0.05 {
        bail!(
            "rested-XP persistence mismatch {phase}: expected xp/rest {expected_xp}/{expected_rest_bonus:.4}, got {}/{:.4}",
            state.xp,
            state.rest_bonus
        );
    }
    Ok(())
}
pub(crate) fn validate_rested_xp_saved_state_shape(
    state: RestedXpDbState,
    expected_level: u8,
    expected_xp: u32,
    expected_online: u8,
    phase: &str,
) -> Result<()> {
    // Rest bonus accrual is time-sensitive within a small tolerance. Derive
    // the state relation from the persisted value itself so a legitimate
    // boundary crossing near 1.0 is not rejected while still enforcing C++'s
    // exact SetRestBonus state rule.
    let expected_rest_state = if state.rest_bonus >= 1.0 {
        REST_STATE_RESTED
    } else {
        REST_STATE_NORMAL
    };
    if state.level != expected_level
        || state.xp != expected_xp
        || state.rest_state != expected_rest_state
        || state.online != expected_online
    {
        bail!(
            "rested-XP saved-state mismatch {phase}: expected level/xp/state/online {expected_level}/{expected_xp}/{expected_rest_state}/{expected_online}, got {}/{}/{}/{}",
            state.level,
            state.xp,
            state.rest_state,
            state.online
        );
    }
    Ok(())
}
pub(crate) fn validate_rested_xp_target_template(
    entry: u32,
    creature_type: u8,
    vehicle_id: u32,
) -> Result<()> {
    if creature_type == CREATURE_TYPE_CRITTER {
        bail!("rested-XP target entry {entry} is a critter; C++ dynamically marks critters NO_XP");
    }
    if vehicle_id != 0 {
        bail!(
            "rested-XP target entry {entry} has VehicleId {vehicle_id}; C++ creates it with HighGuid::Vehicle, which this creature-only smoke does not support"
        );
    }
    Ok(())
}
pub(crate) fn vault_keeper_packed_guid(
    target: &ResolvedCreatureTarget,
    runtime_realm_id: u16,
) -> Vec<u8> {
    let (low, high) = create_void_storage_creature_guid_raw(
        target.map_id,
        target.entry,
        target.guid_counter,
        runtime_realm_id,
    );
    build_packed_guid(low, high)
}
pub(crate) async fn wait_for_bank_open(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        {
            Ok(Ok((opcode, payload))) => {
                result.seen_opcodes.push(format!("0x{opcode:04X}"));
                if opcode == SMSG_INVENTORY_CHANGE_FAILURE {
                    bail!("bank activation returned inventory failure payload {payload:?}");
                }
                if opcode == SMSG_NPC_INTERACTION_OPEN_RESULT {
                    result.bank_open_confirmed = true;
                    info!("[Bot {}] ✅ banker interaction open confirmed", bot_index);
                    return Ok(());
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => break,
        }
    }
    bail!("timed out waiting for SMSG_NPC_INTERACTION_OPEN_RESULT")
}
pub(crate) async fn logout_and_wait(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    result: &mut BotRunResult,
) -> Result<()> {
    send_encrypted_packet(stream, crypt, CMSG_LOGOUT_REQUEST, &[0]).await?;
    info!("[Bot {}] ✅ CMSG_LOGOUT_REQUEST sent", bot_index);
    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(NORMAL_LOGOUT_COMPLETE_WAIT_SECS);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(
            remaining,
            read_encrypted_packet(stream, crypt, server_inflater),
        )
        .await
        {
            Ok(Ok((opcode, payload))) => {
                result.seen_opcodes.push(format!("0x{opcode:04X}"));
                if opcode == SMSG_LOGOUT_COMPLETE {
                    if !payload.is_empty() {
                        bail!(
                            "SMSG_LOGOUT_COMPLETE carried {} bytes; C++ 3.4.3 writes an empty body",
                            payload.len()
                        );
                    }
                    info!("[Bot {}] ✅ SMSG_LOGOUT_COMPLETE received", bot_index);
                    return Ok(());
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => break,
        }
    }
    bail!("timed out waiting for SMSG_LOGOUT_COMPLETE")
}
pub(crate) async fn wait_for_vendor_purchase_result(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    expected_vendor_guid: &[u8],
    expected_muid: i32,
    options: &VendorSmokeOptions,
    expected_currency_after: u32,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection
        .as_mut()
        .context("vendor purchase requires the preserved realm connection")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    let mut fence_sent = false;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for vendor purchase result (buy={}, currency={}, item_push={}, fence={})",
                result.vendor_buy_succeeded_seen,
                result.vendor_set_currency_seen,
                result.vendor_item_push_seen,
                fence_sent
            );
        }
        // C++ splits this result across both encrypted connections. Poll for
        // readiness, then finish one selected frame without cancellation so
        // a losing `select!` branch cannot consume a partial encrypted frame.
        let routed_packet = if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(5)),
            remaining,
            "vendor realm purchase result",
        )
        .await?
        {
            Some(("realm", true, opcode, payload))
        } else {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            read_encrypted_packet_if_ready(
                stream,
                crypt,
                server_inflater,
                remaining.min(Duration::from_millis(5)),
                remaining,
                "vendor instance purchase result",
            )
            .await?
            .map(|(opcode, payload)| ("instance", false, opcode, payload))
        };
        let Some((connection, on_realm, opcode, payload)) = routed_packet else {
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        info!(
            "[Bot {}] 📦 {} vendor-buy {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );

        match opcode {
            SMSG_TIME_SYNC_REQUEST => {
                if on_realm {
                    bail!("SMSG_TIME_SYNC_REQUEST arrived on realm during vendor purchase");
                }
                let sequence = parse_time_sync_request_sequence(&payload)?;
                let response = build_time_sync_response_payload(sequence, 0);
                send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            }
            SMSG_BUY_FAILED => {
                let reason = payload.last().copied();
                bail!("vendor purchase returned SMSG_BUY_FAILED reason={reason:?}");
            }
            SMSG_INVENTORY_CHANGE_FAILURE => {
                bail!("vendor purchase returned SMSG_INVENTORY_CHANGE_FAILURE");
            }
            SMSG_BUY_SUCCEEDED => {
                if !on_realm {
                    bail!("SMSG_BUY_SUCCEEDED arrived on instance; C++ routes it on realm");
                }
                parse_vendor_buy_succeeded(&payload, expected_vendor_guid, expected_muid, 1, -1)?;
                result.vendor_buy_succeeded_seen = true;
            }
            SMSG_SET_CURRENCY => {
                if on_realm {
                    bail!("SMSG_SET_CURRENCY arrived on realm; C++ routes it on instance");
                }
                let (currency_id, quantity) = parse_set_currency_identity(&payload)?;
                if currency_id == options.currency_id {
                    if quantity != expected_currency_after {
                        bail!(
                            "SMSG_SET_CURRENCY quantity for {} is {}, expected {}",
                            currency_id,
                            quantity,
                            expected_currency_after
                        );
                    }
                    result.vendor_set_currency_seen = true;
                }
            }
            SMSG_ITEM_PUSH_RESULT => {
                if !on_realm {
                    bail!("SMSG_ITEM_PUSH_RESULT arrived on instance; C++ routes it on realm");
                }
                loot_race::validate_vendor_item_push_result_like_cpp(
                    &payload,
                    result.character_guid,
                    options.item_entry,
                    1,
                    realm_id(),
                )?;
                result.vendor_item_push_seen = true;
            }
            SMSG_PONG if fence_sent => {
                if on_realm {
                    bail!("vendor capture-fence SMSG_PONG arrived on realm");
                }
                if payload != VENDOR_CAPTURE_FENCE_SERIAL.to_le_bytes() {
                    bail!(
                        "vendor capture-fence SMSG_PONG mismatch: expected 0x{:08X}, got {:02X?}",
                        VENDOR_CAPTURE_FENCE_SERIAL,
                        payload
                    );
                }
                return Ok(());
            }
            _ => {}
        }

        if !fence_sent
            && result.vendor_buy_succeeded_seen
            && result.vendor_set_currency_seen
            && result.vendor_item_push_seen
        {
            let ping = build_ping_payload(VENDOR_CAPTURE_FENCE_SERIAL);
            send_encrypted_packet(stream, crypt, CMSG_PING, &ping).await?;
            fence_sent = true;
            info!(
                "[Bot {}] ✅ vendor capture-fence CMSG_PING serial=0x{:08X}",
                bot_index, VENDOR_CAPTURE_FENCE_SERIAL
            );
        }
    }
}
pub(crate) fn parse_set_currency_identity(payload: &[u8]) -> Result<(u32, u32)> {
    if payload.len() < 8 {
        bail!("SMSG_SET_CURRENCY payload is shorter than type/quantity");
    }
    let currency_id = i32::from_le_bytes(payload[0..4].try_into()?);
    let quantity = i32::from_le_bytes(payload[4..8].try_into()?);
    Ok((
        u32::try_from(currency_id).map_err(|_| anyhow!("negative currency id {currency_id}"))?,
        u32::try_from(quantity).map_err(|_| anyhow!("negative currency quantity {quantity}"))?,
    ))
}
pub(crate) fn parse_vendor_buy_succeeded(
    payload: &[u8],
    expected_vendor_guid: &[u8],
    expected_muid: i32,
    expected_quantity_bought: u32,
    expected_new_quantity: i32,
) -> Result<()> {
    let (guid_len, low, high) = parse_packed_guid(payload)
        .ok_or_else(|| anyhow!("SMSG_BUY_SUCCEEDED has an invalid packed vendor GUID"))?;
    let (expected_guid_len, expected_low, expected_high) = parse_packed_guid(expected_vendor_guid)
        .ok_or_else(|| anyhow!("fixture has an invalid packed vendor GUID"))?;
    if guid_len != expected_guid_len || low != expected_low || high != expected_high {
        bail!("SMSG_BUY_SUCCEEDED names a different vendor GUID");
    }

    let mut cursor = guid_len;
    let muid = take_vendor_u32(payload, &mut cursor)?;
    let new_quantity = take_vendor_i32(payload, &mut cursor)?;
    let quantity_bought = take_vendor_u32(payload, &mut cursor)?;
    let expected_muid = u32::try_from(expected_muid)
        .map_err(|_| anyhow!("fixture has invalid negative vendor MUID {expected_muid}"))?;
    if cursor != payload.len() {
        bail!(
            "SMSG_BUY_SUCCEEDED has {} trailing bytes",
            payload.len() - cursor
        );
    }
    if muid != expected_muid
        || new_quantity != expected_new_quantity
        || quantity_bought != expected_quantity_bought
    {
        bail!(
            "SMSG_BUY_SUCCEEDED fields are muid/new_quantity/quantity_bought {muid}/{new_quantity}/{quantity_bought}, expected {expected_muid}/{expected_new_quantity}/{expected_quantity_bought}"
        );
    }
    Ok(())
}
pub(crate) fn take_vendor_u32(payload: &[u8], cursor: &mut usize) -> Result<u32> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| anyhow!("vendor packet cursor overflow"))?;
    let bytes: [u8; 4] = payload
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("vendor packet truncated at byte {}", *cursor))?
        .try_into()?;
    *cursor = end;
    Ok(u32::from_le_bytes(bytes))
}
pub(crate) fn take_vendor_i32(payload: &[u8], cursor: &mut usize) -> Result<i32> {
    Ok(i32::from_le_bytes(
        take_vendor_u32(payload, cursor)?.to_le_bytes(),
    ))
}
pub(crate) fn take_vendor_u64(payload: &[u8], cursor: &mut usize) -> Result<u64> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| anyhow!("vendor packet cursor overflow"))?;
    let bytes: [u8; 8] = payload
        .get(*cursor..end)
        .ok_or_else(|| anyhow!("vendor packet truncated at byte {}", *cursor))?
        .try_into()?;
    *cursor = end;
    Ok(u64::from_le_bytes(bytes))
}
pub(crate) fn required_row_value<T>(row: &mysql::Row, column: &str) -> Result<T>
where
    T: mysql::prelude::FromValue,
{
    row.get(column)
        .ok_or_else(|| anyhow!("Missing/invalid `{column}` in QA fixture query"))
}
pub(crate) fn rested_xp_count_rows(
    conn: &mut mysql::Conn,
    sql: &str,
    key: u64,
    label: &str,
) -> Result<u64> {
    use mysql::prelude::Queryable;

    conn.exec_first(sql, (key,))
        .map_err(|error| anyhow!("Check rested-XP fixture state in {label}: {error}"))
        .map(|count| count.unwrap_or(0))
}
pub(crate) fn current_epoch_secs() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}
