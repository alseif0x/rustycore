//! Exact 3.4.3 cast request builder, packet observer and decoder.

use super::*;

pub(super) fn record_sent(evidence: &mut Evidence, action: &str, opcode: u16, body: &[u8]) {
    evidence.sent_packets.push(SentPacket {
        sequence: evidence.sent_packets.len(),
        action: action.to_string(),
        opcode,
        bytes: body.len(),
        body_sha256: format!("{:x}", Sha256::digest(body)),
    });
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
    evidence: &mut Evidence,
) -> Result<()> {
    while tokio::time::Instant::now() < deadline {
        let Some((connection, opcode, payload)) =
            next_packet(deadline, stream, crypt, inflater, realm).await?
        else {
            break;
        };
        let sequence = evidence.events.len();
        let mut event = decode_event(sequence, connection, opcode, &payload);
        correlate_event(evidence, &mut event);
        if opcode == SMSG_LOGOUT_COMPLETE {
            match logout_complete_route(connection, &payload) {
                Ok(route) => {
                    evidence.logout_confirmed = true;
                    evidence.logout_route = Some(route.to_string());
                }
                Err(error) => {
                    event.error.get_or_insert_with(|| error.to_string());
                    evidence.events.push(event);
                    return Err(error);
                }
            }
        }
        evidence.events.push(event);
        if opcode == SMSG_LOGOUT_COMPLETE {
            break;
        }
    }
    Ok(())
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
                if opcode == SMSG_TIME_SYNC_REQUEST {
                    let sequence = parse_time_sync_request_sequence(&payload)?;
                    let response = build_time_sync_response_payload(sequence, 0);
                    send_encrypted_packet(
                        &mut realm.stream,
                        &mut realm.crypt,
                        CMSG_TIME_SYNC_RESPONSE,
                        &response,
                    )
                    .await?;
                }
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
            if opcode == SMSG_TIME_SYNC_REQUEST {
                let sequence = parse_time_sync_request_sequence(&payload)?;
                let response = build_time_sync_response_payload(sequence, 0);
                send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            }
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
        connection,
        opcode,
        kind: event_kind(opcode).to_string(),
        body_bytes: payload.len(),
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

struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8]> {
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

    fn guid(&mut self) -> Result<Guid> {
        let (consumed, low, high) = parse_packed_guid(&self.data[self.position..])
            .context("truncated packed ObjectGuid")?;
        self.position = self
            .position
            .checked_add(consumed)
            .context("ObjectGuid cursor overflow")?;
        Ok(Guid { low, high })
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(*self.bytes(1)?.first().unwrap())
    }

    fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.bytes(8)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn finish(&self, packet: &str) -> Result<()> {
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
