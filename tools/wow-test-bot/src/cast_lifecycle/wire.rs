//! Exact request builders and logical packet trace for the scripted runner.

use super::*;

pub(super) fn build_action_packet(action: &Action) -> (u16, Vec<u8>) {
    match action {
        Action::ReleaseSpirit { check_instance } => {
            let mut body = [0u8; 1];
            write_msb_bits(&mut body, 0, 1, u32::from(*check_instance));
            (CMSG_RELEASE_SPIRIT, body.to_vec())
        }
        Action::ReclaimCorpse { corpse } => (CMSG_RECLAIM_CORPSE, corpse.packed()),
        Action::SpiritHealerActivate { healer } => match healer {
            CreatureTarget::Guid(healer) => (CMSG_SPIRIT_HEALER_ACTIVATE, healer.packed()),
            CreatureTarget::Observed(_) => {
                unreachable!("observed creature targets must resolve before packet building")
            }
        },
        Action::ResurrectResponse { offerer, response } => {
            let mut body = offerer.packed();
            body.extend(response.to_le_bytes());
            (CMSG_RESURRECT_RESPONSE, body)
        }
        Action::SelfRes { spell_id } => (CMSG_SELF_RES, spell_id.to_le_bytes().to_vec()),
        Action::AttackSwing { target } => match target {
            CreatureTarget::Guid(target) => (CMSG_ATTACK_SWING, target.packed()),
            CreatureTarget::Observed(_) => {
                unreachable!("observed creature targets must resolve before packet building")
            }
        },
        _ => unreachable!("only typed non-cast actions use this builder"),
    }
}

pub(super) async fn send_traced_packet(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    evidence: &mut Evidence,
    connection: Connection,
    action_index: Option<usize>,
    action: &str,
    opcode: u16,
    body: &[u8],
) -> Result<()> {
    let sent_sequence =
        record_send_attempt(evidence, connection, action_index, action, opcode, body);
    match send_encrypted_packet(stream, crypt, opcode, body).await {
        Ok(()) => {
            record_write_completed(evidence, sent_sequence);
            Ok(())
        }
        Err(error) => {
            record_write_failed(evidence, sent_sequence, &error.to_string());
            Err(error)
        }
    }
}

pub(super) fn record_send_attempt(
    evidence: &mut Evidence,
    connection: Connection,
    action_index: Option<usize>,
    action: &str,
    opcode: u16,
    body: &[u8],
) -> usize {
    let sent_sequence = evidence.sent_packets.len();
    let connection_sequence = take_connection_sequence(evidence, connection);
    let observation_sequence = evidence.trace.len();
    evidence.sent_packets.push(SentPacket {
        sequence: sent_sequence,
        observation_sequence,
        action_index,
        connection,
        connection_sequence,
        action: action.to_string(),
        opcode,
        bytes: body.len(),
        body_hex: hex::encode(body),
        body_sha256: format!("{:x}", Sha256::digest(body)),
        write_attempted: true,
        write_completed: false,
        write_error: None,
    });
    push_trace(
        evidence,
        TraceKind::SendAttempted,
        action_index,
        Some(action.to_string()),
        Some(connection),
        Some(connection_sequence),
        Some(opcode),
        Some(sent_sequence),
        None,
        Some(body.len()),
        false,
        None,
    );
    sent_sequence
}

pub(super) fn record_write_completed(evidence: &mut Evidence, sent_sequence: usize) {
    let sent = &mut evidence.sent_packets[sent_sequence];
    sent.write_completed = true;
    let (action_index, action, connection, connection_sequence, opcode, body_bytes) = (
        sent.action_index,
        sent.action.clone(),
        sent.connection,
        sent.connection_sequence,
        sent.opcode,
        sent.bytes,
    );
    push_trace(
        evidence,
        TraceKind::LocalWriteCompleted,
        action_index,
        Some(action),
        Some(connection),
        Some(connection_sequence),
        Some(opcode),
        Some(sent_sequence),
        None,
        Some(body_bytes),
        false,
        None,
    );
}

pub(super) fn record_write_failed(evidence: &mut Evidence, sent_sequence: usize, error: &str) {
    let sent = &mut evidence.sent_packets[sent_sequence];
    sent.write_error = Some(error.to_string());
    let (action_index, action, connection, connection_sequence, opcode, body_bytes) = (
        sent.action_index,
        sent.action.clone(),
        sent.connection,
        sent.connection_sequence,
        sent.opcode,
        sent.bytes,
    );
    push_trace(
        evidence,
        TraceKind::LocalWriteFailed,
        action_index,
        Some(action),
        Some(connection),
        Some(connection_sequence),
        Some(opcode),
        Some(sent_sequence),
        None,
        Some(body_bytes),
        false,
        Some(error.to_string()),
    );
}

pub(super) fn record_action_boundary(
    evidence: &mut Evidence,
    action_index: Option<usize>,
    action: &str,
    kind: TraceKind,
    detail: Option<String>,
) {
    push_trace(
        evidence,
        kind,
        action_index,
        Some(action.to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        detail,
    );
}

pub(super) fn record_socket_close(
    evidence: &mut Evidence,
    connection: Connection,
    error: Option<&std::io::Error>,
) {
    push_trace(
        evidence,
        TraceKind::LocalSocketClosed,
        None,
        Some("finish".to_string()),
        Some(connection),
        None,
        None,
        None,
        None,
        None,
        false,
        error.map(|error| error.to_string()),
    );
}

fn push_trace(
    evidence: &mut Evidence,
    kind: TraceKind,
    action_index: Option<usize>,
    action: Option<String>,
    connection: Option<Connection>,
    connection_sequence: Option<usize>,
    opcode: Option<u16>,
    sent_packet_sequence: Option<usize>,
    event_sequence: Option<usize>,
    body_bytes: Option<usize>,
    body_redacted: bool,
    detail: Option<String>,
) {
    let sequence = evidence.trace.len();
    evidence.trace.push(TraceEntry {
        sequence,
        kind,
        action_index,
        action,
        connection,
        connection_sequence,
        opcode,
        sent_packet_sequence,
        event_sequence,
        body_bytes,
        body_redacted,
        detail,
    });
}

fn take_connection_sequence(evidence: &mut Evidence, connection: Connection) -> usize {
    let counter = match connection {
        Connection::Instance => &mut evidence.connection_sequences.instance,
        Connection::Realm => &mut evidence.connection_sequences.realm,
    };
    let sequence = *counter;
    *counter += 1;
    sequence
}

pub(super) fn record_observed_packet(
    evidence: &mut Evidence,
    connection: Connection,
    opcode: u16,
    payload: &[u8],
) -> Option<usize> {
    let connection_sequence = take_connection_sequence(evidence, connection);
    if matches!(
        opcode,
        REDACTED_SMSG_AUTH_CHALLENGE
            | REDACTED_SMSG_AUTH_FAILED
            | REDACTED_SMSG_AUTH_RESPONSE
            | REDACTED_SMSG_ENTER_ENCRYPTED_MODE
            | REDACTED_SMSG_CONNECT_TO
    ) {
        push_trace(
            evidence,
            TraceKind::PacketRedacted,
            None,
            Some("authentication_or_transfer_control".to_string()),
            Some(connection),
            Some(connection_sequence),
            Some(opcode),
            None,
            None,
            Some(payload.len()),
            true,
            Some("key-bearing control body omitted".to_string()),
        );
        return None;
    }

    let event_sequence = evidence.events.len();
    let observation_sequence = evidence.trace.len();
    let mut event = decode_event(event_sequence, connection, opcode, payload);
    event.connection_sequence = connection_sequence;
    event.observation_sequence = observation_sequence;
    push_trace(
        evidence,
        TraceKind::PacketObserved,
        None,
        None,
        Some(connection),
        Some(connection_sequence),
        Some(opcode),
        None,
        Some(event_sequence),
        Some(payload.len()),
        false,
        None,
    );
    correlate_event(evidence, &mut event);
    evidence.events.push(event);
    Some(event_sequence)
}

pub(super) fn build_cast_spell_payload(
    spell_id: i32,
    cast_id: Guid,
    target: Option<Guid>,
) -> Vec<u8> {
    let mut body = cast_id.packed();
    body.extend(0i32.to_le_bytes());
    body.extend(0i32.to_le_bytes());
    body.extend(spell_id.to_le_bytes());
    body.extend(0u32.to_le_bytes()); // SpellXSpellVisualID
    body.extend(0f32.to_le_bytes()); // MissileTrajectory.Pitch
    body.extend(0f32.to_le_bytes()); // MissileTrajectory.Speed
    body.extend(Guid { low: 0, high: 0 }.packed()); // CraftingNPC
    body.extend(0u32.to_le_bytes()); // OptionalCurrencies count
    body.extend(0u32.to_le_bytes()); // OptionalReagents count
    body.extend(0u32.to_le_bytes()); // RemovedModifications count
    body.extend([0u8; 2]); // SendCastFlags, movement, weights, crafting order

    // SpellTargetData: 28-bit flags, four option bits, 7-bit name length.
    // TARGET_FLAG_UNIT is 0x2. C++ always follows this bit section with the
    // unit and item packed GUIDs, even for an empty target.
    let mut target_header = [0u8; 5];
    if target.is_some() {
        write_msb_bits(&mut target_header, 0, 28, 0x2);
    }
    body.extend(target_header);
    body.extend(target.unwrap_or(Guid { low: 0, high: 0 }).packed());
    body.extend(Guid { low: 0, high: 0 }.packed());
    body
}

pub(super) fn build_cancel_cast_payload(cast_id: Guid, spell_id: u32) -> Vec<u8> {
    let mut body = cast_id.packed();
    body.extend(spell_id.to_le_bytes());
    body
}

pub(super) fn build_cancel_queued_spell_payload() -> Vec<u8> {
    Vec::new()
}

/// Classic registers SMSG_LOGOUT_COMPLETE on the realm connection with an empty
/// body (`Opcodes.cpp`, `LogoutComplete::Write`).  A logout seen on the instance
/// socket, or carrying any payload, is a transport defect and never confirmation.
pub(super) fn logout_complete_route(
    connection: Connection,
    payload: &[u8],
) -> Result<&'static str> {
    if connection != Connection::Realm {
        bail!("LogoutComplete arrived on the instance connection instead of realm");
    }
    if !payload.is_empty() {
        bail!(
            "LogoutComplete must have an empty body, got {} byte(s)",
            payload.len()
        );
    }
    Ok("realm")
}

pub(super) async fn collect_until(
    deadline: tokio::time::Instant,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    controller: &mut Controller,
) -> Result<()> {
    while tokio::time::Instant::now() < deadline {
        let Some((connection, opcode, payload)) =
            next_packet(deadline, stream, crypt, inflater, realm).await?
        else {
            break;
        };
        controller
            .observe_packet(
                connection == Connection::Instance,
                opcode,
                &payload,
                stream,
                crypt,
                realm,
            )
            .await?;
        if opcode == SMSG_LOGOUT_COMPLETE {
            if !controller.evidence.logout_request_completed {
                if let Some(event) = controller.evidence.events.last_mut() {
                    event.error = Some(
                        "LogoutComplete arrived before the scripted logout request".to_string(),
                    );
                }
                bail!("LogoutComplete arrived before the scripted logout request");
            }
            match logout_complete_route(connection, &payload) {
                Ok(route) => {
                    controller.evidence.logout_confirmed = true;
                    controller.evidence.logout_route = Some(route.to_string());
                }
                Err(error) => {
                    if let Some(event) = controller.evidence.events.last_mut() {
                        event.error.get_or_insert_with(|| error.to_string());
                    }
                    return Err(error);
                }
            }
        }
        if opcode == SMSG_LOGOUT_COMPLETE {
            break;
        }
    }
    Ok(())
}

pub(super) async fn drain_login_streams_with_controller(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
    controller: &mut Controller,
) -> Result<()> {
    if realm.is_none() {
        bail!("cast lifecycle login drain requires both sockets");
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut quiet_deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    loop {
        let read_deadline = quiet_deadline.min(deadline);
        let Some((connection, opcode, payload)) =
            next_packet(read_deadline, stream, crypt, inflater, realm).await?
        else {
            if result.login_instance_object_update_seen {
                result.login_stream_drained = true;
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                bail!("login streams did not publish an instance object update");
            }
            quiet_deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            continue;
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if connection == Connection::Instance && opcode == SMSG_UPDATE_OBJECT {
            result.login_instance_object_update_seen = true;
        }
        if opcode == crate::chat_say::SMSG_CHAT {
            crate::chat_say::record(0, &payload, result);
        }
        controller
            .observe_packet(
                connection == Connection::Instance,
                opcode,
                &payload,
                stream,
                crypt,
                realm,
            )
            .await?;
        if opcode != SMSG_ON_MONSTER_MOVE {
            quiet_deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        }
    }
}

async fn next_packet(
    deadline: tokio::time::Instant,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
) -> Result<Option<(Connection, u16, Vec<u8>)>> {
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        if let Some(realm) = realm.as_mut() {
            if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
                &mut realm.stream,
                &mut realm.crypt,
                &mut realm.inflater,
                remaining.min(Duration::from_millis(10)),
                remaining,
                "cast lifecycle realm packet",
            )
            .await?
            {
                return Ok(Some((Connection::Realm, opcode, payload)));
            }
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            remaining.min(Duration::from_millis(10)),
            remaining,
            "cast lifecycle instance packet",
        )
        .await?
        {
            return Ok(Some((Connection::Instance, opcode, payload)));
        }
    }
}

pub(super) fn decode_event(
    sequence: usize,
    connection: Connection,
    opcode: u16,
    payload: &[u8],
) -> ObservedEvent {
    let mut event = ObservedEvent {
        sequence,
        observation_sequence: 0,
        connection,
        connection_sequence: 0,
        opcode,
        kind: event_kind(opcode).to_string(),
        body_bytes: payload.len(),
        body_hex: hex::encode(payload),
        body_sha256: format!("{:x}", Sha256::digest(payload)),
        client_cast_id: None,
        server_cast_id: None,
        cast_id: None,
        caster: None,
        caster_unit: None,
        original_cast_id: None,
        spell_id: None,
        visual_id: None,
        cast_flags: None,
        cast_flags_ex: None,
        cast_time_ms: None,
        target: None,
        hit_targets: Vec::new(),
        miss_targets: Vec::new(),
        miss_statuses: Vec::new(),
        remaining_power: Vec::new(),
        remaining_runes: None,
        target_points: Vec::new(),
        ammo_display_id: None,
        ammo_inventory_type: None,
        full_combat_log: None,
        full_log_power_count: None,
        reason: None,
        failed_arg1: None,
        failed_arg2: None,
        error: None,
    };
    let parsed = match opcode {
        SMSG_SPELL_PREPARE => parse_prepare(payload, &mut event),
        SMSG_SPELL_START => parse_cast(payload, false, &mut event),
        SMSG_SPELL_GO => parse_cast(payload, true, &mut event),
        SMSG_CAST_FAILED => parse_cast_failed(payload, &mut event),
        SMSG_SPELL_FAILURE => parse_spell_failure(payload, &mut event, false),
        SMSG_SPELL_FAILED_OTHER => parse_spell_failure(payload, &mut event, true),
        SMSG_TIME_SYNC_REQUEST => parse_time_sync_request_sequence(payload).map(|_| ()),
        _ => Ok(()),
    };
    if let Err(error) = parsed {
        event.error = Some(error.to_string());
    }
    event
}

fn event_kind(opcode: u16) -> &'static str {
    match opcode {
        SMSG_LOGIN_VERIFY_WORLD => "login_verify_world",
        SMSG_NEW_WORLD => "new_world",
        SMSG_TRANSFER_PENDING => "transfer_pending",
        SMSG_RESUME_TOKEN => "resume_token",
        SMSG_SUSPEND_TOKEN => "suspend_token",
        SMSG_MOVE_ROOT => "move_root",
        SMSG_MOVE_UNROOT => "move_unroot",
        SMSG_MOVE_SET_WATER_WALK => "move_water_walk",
        SMSG_MOVE_SET_LAND_WALK => "move_land_walk",
        SMSG_MOVE_SET_RUN_SPEED => "move_run_speed",
        SMSG_MOVE_SET_SWIM_SPEED => "move_swim_speed",
        SMSG_MOVE_TELEPORT => "move_teleport",
        SMSG_SPELL_PREPARE => "spell_prepare",
        SMSG_SPELL_START => "spell_start",
        SMSG_SPELL_GO => "spell_go",
        SMSG_CAST_FAILED => "cast_failed",
        SMSG_SPELL_FAILURE => "spell_failure",
        SMSG_SPELL_FAILED_OTHER => "spell_failed_other",
        SMSG_TIME_SYNC_REQUEST => "time_sync_request",
        _ => "other",
    }
}

fn parse_prepare(payload: &[u8], event: &mut ObservedEvent) -> Result<()> {
    let mut cursor = Cursor::new(payload);
    let client = cursor.guid()?;
    let server = cursor.guid()?;
    cursor.finish("SpellPrepare")?;
    event.client_cast_id = Some(client);
    event.server_cast_id = Some(server);
    event.cast_id = Some(server);
    Ok(())
}

fn parse_cast(payload: &[u8], is_go: bool, event: &mut ObservedEvent) -> Result<()> {
    let mut cursor = Cursor::new(payload);
    event.caster = Some(cursor.guid()?);
    event.caster_unit = Some(cursor.guid()?);
    let cast_id = cursor.guid()?;
    event.cast_id = Some(cast_id);
    event.server_cast_id = Some(cast_id);
    event.original_cast_id = Some(cursor.guid()?);
    event.spell_id = Some(cursor.i32()?);
    event.visual_id = Some(cursor.u32()?);
    event.cast_flags = Some(cursor.u32()?);
    event.cast_flags_ex = Some(cursor.u32()?);
    event.cast_time_ms = Some(cursor.u32()?);
    let _travel_time = cursor.u32()?;
    let _pitch = cursor.f32()?;
    let _destination_index = cursor.u8()?;
    let _immunity_school = cursor.u32()?;
    let _immunity_value = cursor.u32()?;
    let _prediction_points = cursor.u32()?;
    let _prediction_type = cursor.u8()?;
    let _prediction_beacon = cursor.guid()?;

    let counts = cursor.bytes(10)?;
    let hit_count = read_msb_bits(counts, 0, 16).context("invalid hit count")? as usize;
    let miss_count = read_msb_bits(counts, 16, 16).context("invalid miss count")? as usize;
    let miss_status_count =
        read_msb_bits(counts, 32, 16).context("invalid miss-status count")? as usize;
    let power_count =
        read_msb_bits(counts, 48, 9).context("invalid remaining-power count")? as usize;
    let has_runes = read_msb_bits(counts, 57, 1).context("invalid rune bit")? != 0;
    let target_point_count =
        read_msb_bits(counts, 58, 16).context("invalid target-point count")? as usize;
    let has_ammo_display = read_msb_bits(counts, 74, 1).context("invalid ammo-display bit")? != 0;
    let has_ammo_inventory =
        read_msb_bits(counts, 75, 1).context("invalid ammo-inventory bit")? != 0;
    if read_msb_bits(counts, 76, 4) != Some(0) {
        bail!("SpellCastData count padding is nonzero");
    }

    event.target = Some(parse_target(&mut cursor)?);
    for _ in 0..hit_count {
        event.hit_targets.push(cursor.guid()?);
    }
    for _ in 0..miss_count {
        event.miss_targets.push(cursor.guid()?);
    }
    for _ in 0..miss_status_count {
        let reason = cursor.u8()?;
        let reflect_status = (reason == 11).then(|| cursor.u8()).transpose()?;
        event.miss_statuses.push(MissStatusFact {
            reason,
            reflect_status,
        });
    }
    for _ in 0..power_count {
        event.remaining_power.push(PowerFact {
            amount: cursor.i32()?,
            power_type: cursor.i8()?,
        });
    }
    if has_runes {
        let start = cursor.u8()?;
        let count = cursor.u8()?;
        let cooldown_count = cursor.u32()? as usize;
        if cooldown_count > cursor.remaining() {
            bail!("RuneData cooldown count exceeds remaining packet bytes");
        }
        event.remaining_runes = Some(RuneFact {
            start,
            count,
            cooldowns: cursor.bytes(cooldown_count)?.to_vec(),
        });
    }
    for _ in 0..target_point_count {
        event.target_points.push(parse_target_point(&mut cursor)?);
    }
    if has_ammo_display {
        event.ammo_display_id = Some(cursor.i32()?);
    }
    if has_ammo_inventory {
        event.ammo_inventory_type = Some(cursor.i32()?);
    }
    if is_go {
        let full_log = cursor.u8()?;
        if full_log > 1 {
            bail!("invalid FullCombatLog bit {full_log}");
        }
        event.full_combat_log = Some(full_log != 0);
        if full_log != 0 {
            let _health = cursor.i64()?;
            let _attack_power = cursor.i32()?;
            let _spell_power = cursor.i32()?;
            let _armor = cursor.i32()?;
            let power_count = read_msb_bits(cursor.bytes(2)?, 0, 9)
                .context("invalid full-log power count")? as usize;
            event.full_log_power_count = Some(power_count);
            for _ in 0..power_count {
                let _power_type = cursor.i32()?;
                let _amount = cursor.i32()?;
                let _cost = cursor.i32()?;
            }
        }
    }
    cursor.finish(if is_go { "SpellGo" } else { "SpellStart" })
}

fn parse_cast_failed(payload: &[u8], event: &mut ObservedEvent) -> Result<()> {
    let mut cursor = Cursor::new(payload);
    let cast_id = cursor.guid()?;
    event.cast_id = Some(cast_id);
    event.spell_id = Some(cursor.i32()?);
    event.visual_id = Some(cursor.u32()?);
    event.reason = Some(cursor.i32()? as u32);
    event.failed_arg1 = Some(cursor.i32()?);
    event.failed_arg2 = Some(cursor.i32()?);
    cursor.finish("CastFailed")
}

fn parse_spell_failure(
    payload: &[u8],
    event: &mut ObservedEvent,
    failed_other: bool,
) -> Result<()> {
    let mut cursor = Cursor::new(payload);
    event.caster = Some(cursor.guid()?);
    let cast_id = cursor.guid()?;
    event.cast_id = Some(cast_id);
    event.spell_id = Some(cursor.i32()?);
    event.visual_id = Some(cursor.u32()?);
    event.reason = Some(if failed_other {
        cursor.u8()? as u32
    } else {
        cursor.u16()? as u32
    });
    cursor.finish(if failed_other {
        "SpellFailedOther"
    } else {
        "SpellFailure"
    })
}

fn parse_target(cursor: &mut Cursor<'_>) -> Result<TargetFact> {
    let header = cursor.bytes(5)?;
    let flags = read_msb_bits(header, 0, 28).context("invalid target flags")?;
    let has_src = read_msb_bits(header, 28, 1).context("invalid target src bit")? != 0;
    let has_dst = read_msb_bits(header, 29, 1).context("invalid target dst bit")? != 0;
    let has_orientation =
        read_msb_bits(header, 30, 1).context("invalid target orientation bit")? != 0;
    let has_map = read_msb_bits(header, 31, 1).context("invalid target map bit")? != 0;
    let name_length = read_msb_bits(header, 32, 7).context("invalid target name length")? as usize;
    let unit = cursor.guid()?;
    let item = cursor.guid()?;
    let src_location = has_src.then(|| parse_target_point(cursor)).transpose()?;
    let dst_location = has_dst.then(|| parse_target_point(cursor)).transpose()?;
    let orientation = has_orientation.then(|| cursor.f32()).transpose()?;
    let map_id = has_map.then(|| cursor.i32()).transpose()?;
    let name = String::from_utf8_lossy(cursor.bytes(name_length)?).into_owned();
    Ok(TargetFact {
        flags,
        unit,
        item,
        src_location,
        dst_location,
        orientation,
        map_id,
        name,
    })
}

fn parse_target_point(cursor: &mut Cursor<'_>) -> Result<TargetPointFact> {
    Ok(TargetPointFact {
        transport: cursor.guid()?,
        x: cursor.f32()?,
        y: cursor.f32()?,
        z: cursor.f32()?,
    })
}

pub(super) struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub(super) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    pub(super) fn bytes(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .context("packet cursor overflow")?;
        let bytes = self
            .data
            .get(self.position..end)
            .context("truncated spell packet")?;
        self.position = end;
        Ok(bytes)
    }

    pub(super) fn guid(&mut self) -> Result<Guid> {
        let (consumed, low, high) = parse_packed_guid(&self.data[self.position..])
            .context("truncated packed ObjectGuid")?;
        self.position = self
            .position
            .checked_add(consumed)
            .context("ObjectGuid cursor overflow")?;
        Ok(Guid { low, high })
    }

    pub(super) fn u8(&mut self) -> Result<u8> {
        Ok(*self.bytes(1)?.first().unwrap())
    }

    fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    pub(super) fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    pub(super) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }

    pub(super) fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    pub(super) fn finish(&self, packet: &str) -> Result<()> {
        if self.position != self.data.len() {
            bail!(
                "{packet} has {} trailing byte(s)",
                self.data.len().saturating_sub(self.position)
            );
        }
        Ok(())
    }
}

pub(super) fn read_msb_bits(data: &[u8], bit_offset: usize, bit_count: usize) -> Option<u32> {
    if bit_count > 32 || bit_offset.checked_add(bit_count)? > data.len().checked_mul(8)? {
        return None;
    }
    let mut value = 0u32;
    for bit in bit_offset..bit_offset + bit_count {
        value = (value << 1) | u32::from((data[bit / 8] >> (7 - bit % 8)) & 1);
    }
    Some(value)
}

pub(super) fn write_msb_bits(data: &mut [u8], bit_offset: usize, bit_count: usize, value: u32) {
    for index in 0..bit_count {
        let source_bit = (value >> (bit_count - index - 1)) & 1;
        if source_bit != 0 {
            let destination = bit_offset + index;
            data[destination / 8] |= 1 << (7 - destination % 8);
        }
    }
}
