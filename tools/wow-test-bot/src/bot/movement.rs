//! Movement operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn monster_move_mover_guid_like_cpp(payload: &[u8]) -> Result<(u64, u64)> {
    // C++ `MonsterMove::Write` starts with MoverGUID, followed by Position and
    // MonsterMoveSpline. Parsing the leading packed GUID is sufficient to
    // prove that this movement belongs to the selected live fixture object;
    // capture-diff performs the full spline semantic validation.
    let (_, low, high) =
        parse_packed_guid(payload).context("SMSG_ON_MONSTER_MOVE missing MoverGUID")?;
    Ok((low, high))
}
pub(crate) async fn verify_inventory_swap_invalid_position_gate(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    valid_destination_slot: u8,
    timeout_secs: u64,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection.as_mut().context(
        "inventory validation smoke requires distinct realm/instance sockets to validate C++ routing",
    )?;

    // Establish a quiet action boundary before the probe. Login publication
    // can still be arriving on either socket after LOGIN_VERIFY_WORLD; those
    // packets are not causally part of CMSG_SWAP_ITEM and would otherwise sit
    // between the request and its immediate C++ error in the raw capture.
    let drain_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = drain_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("inventory validation login drain never reached a quiet period");
        }
        enum InventoryDrainReady {
            Instance,
            Realm,
        }
        let mut instance_peek = [0u8; 1];
        let mut realm_peek = [0u8; 1];
        let ready = tokio::time::timeout(Duration::from_millis(250).min(remaining), async {
            tokio::select! {
                ready = stream.peek(&mut instance_peek) => {
                    if ready.context("inventory login-drain instance peek failed")? == 0 {
                        bail!("instance connection closed during inventory login drain");
                    }
                    Ok(InventoryDrainReady::Instance)
                }
                ready = realm.stream.peek(&mut realm_peek) => {
                    if ready.context("inventory login-drain realm peek failed")? == 0 {
                        bail!("realm connection closed during inventory login drain");
                    }
                    Ok(InventoryDrainReady::Realm)
                }
            }
        })
        .await;
        let ready = match ready {
            Ok(ready) => ready?,
            Err(_) => break,
        };
        let (connection, opcode, payload) = match ready {
            InventoryDrainReady::Instance => {
                let (opcode, payload) =
                    read_encrypted_packet(stream, crypt, server_inflater).await?;
                ("instance", opcode, payload)
            }
            InventoryDrainReady::Realm => {
                let (opcode, payload) =
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater)
                        .await?;
                ("realm", opcode, payload)
            }
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
        }
        info!(
            "[Bot {}] 📦 {} inventory pre-action drain {}",
            bot_index,
            connection,
            parse_packet(opcode, &payload)
        );
    }

    let payload = build_swap_item_invalid_source_payload(valid_destination_slot);
    send_encrypted_packet(stream, crypt, CMSG_SWAP_ITEM, &payload).await?;
    info!(
        "[Bot {}] ✅ CMSG_SWAP_ITEM invalid-source validation probe sent",
        bot_index
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        enum InventoryValidationReady {
            Instance,
            Realm,
        }
        let mut instance_peek = [0u8; 1];
        let mut realm_peek = [0u8; 1];
        let ready = tokio::time::timeout(remaining, async {
            tokio::select! {
                ready = stream.peek(&mut instance_peek) => {
                    if ready.context("inventory validation instance peek failed")? == 0 {
                        bail!("instance connection closed during inventory validation smoke");
                    }
                    Ok(InventoryValidationReady::Instance)
                }
                ready = realm.stream.peek(&mut realm_peek) => {
                    if ready.context("inventory validation realm peek failed")? == 0 {
                        bail!("realm connection closed during inventory validation smoke");
                    }
                    Ok(InventoryValidationReady::Realm)
                }
            }
        })
        .await
        .map_err(|_| anyhow!("timed out waiting for inventory invalid-position failure"))??;
        let (connection, opcode, payload) = match ready {
            InventoryValidationReady::Instance => {
                let (opcode, payload) = tokio::time::timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(stream, crypt, server_inflater),
                )
                .await
                .map_err(|_| anyhow!("inventory validation instance packet read timed out"))??;
                ("instance", opcode, payload)
            }
            InventoryValidationReady::Realm => {
                let (opcode, payload) = tokio::time::timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater),
                )
                .await
                .map_err(|_| anyhow!("inventory validation realm packet read timed out"))??;
                ("realm", opcode, payload)
            }
        };
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        if opcode == SMSG_TIME_SYNC_REQUEST {
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response).await?;
            continue;
        }
        if opcode != SMSG_INVENTORY_CHANGE_FAILURE {
            info!(
                "[Bot {}] 📦 {} inventory validation probe {}",
                bot_index,
                connection,
                parse_packet(opcode, &payload)
            );
            continue;
        }
        if connection != "realm" {
            bail!(
                "inventory invalid-position failure arrived on {connection}; C++ Opcodes.cpp requires the realm connection"
            );
        }
        if payload.len() < 4 {
            bail!(
                "inventory invalid-position failure payload too short: {}",
                payload.len()
            );
        }
        let result_code = i32::from_le_bytes(payload[0..4].try_into()?);
        if result_code != 23 {
            bail!(
                "inventory invalid-position result mismatch: expected ITEM_NOT_FOUND(23), got {result_code}"
            );
        }
        info!(
            "[Bot {}] ✅ realm inventory invalid-position gate returned ITEM_NOT_FOUND",
            bot_index
        );
        return Ok(());
    }
    bail!("timed out waiting for inventory invalid-position failure")
}
pub(crate) fn load_bot_position(character_guid: u64) -> Result<(u16, f64, f64, f64)> {
    use mysql::prelude::Queryable;

    let characters_url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&characters_url)
        .map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;
    let row: Option<(u32, f64, f64, f64)> = conn
        .exec_first(
            "SELECT map, position_x, position_y, position_z FROM characters WHERE guid = ?",
            (character_guid,),
        )
        .map_err(|e| anyhow!("Lookup character {} position: {}", character_guid, e))?;
    row.map(|(map, x, y, z)| (map as u16, x, y, z))
        .ok_or_else(|| anyhow!("No characters row for guid {}", character_guid))
}
pub(crate) fn build_move_heartbeat_payload(
    player_low: u64,
    player_high: u64,
    x: f32,
    y: f32,
    z: f32,
    orientation: f32,
) -> Vec<u8> {
    // C++ MovementInfo wire order, mirrored by wow-packet::MovementInfo::read.
    let mut data = build_packed_guid(player_low, player_high);
    data.extend_from_slice(&0u32.to_le_bytes()); // MovementFlags
    data.extend_from_slice(&0u32.to_le_bytes()); // MovementFlags2
    data.extend_from_slice(&0u32.to_le_bytes()); // MovementFlags3
    data.extend_from_slice(&0u32.to_le_bytes()); // client time; server uses its fallback clock
    data.extend_from_slice(&x.to_le_bytes());
    data.extend_from_slice(&y.to_le_bytes());
    data.extend_from_slice(&z.to_le_bytes());
    data.extend_from_slice(&orientation.to_le_bytes());
    data.extend_from_slice(&0f32.to_le_bytes()); // pitch
    data.extend_from_slice(&0f32.to_le_bytes()); // step-up start elevation
    data.extend_from_slice(&0u32.to_le_bytes()); // remove movement forces count
    data.extend_from_slice(&0u32.to_le_bytes()); // move index
    data.push(0); // no transport/fall/spline/inertia/advanced-flying bits
    data
}
pub(crate) fn build_move_init_active_mover_complete_payload(ticks: u32) -> [u8; 4] {
    // C++ WorldPackets::Movement::MoveInitActiveMoverComplete::Read reads one
    // little-endian uint32. Zero is valid for this non-transport fixture.
    ticks.to_le_bytes()
}
pub(crate) fn find_creature_guids_near_position_in_update_object(
    payload: &[u8],
    map_id: u16,
    entry: u32,
    expected_x: f32,
    expected_y: f32,
    expected_z: f32,
    max_distance: f32,
    expected_counter: Option<u64>,
) -> Vec<DiscoveredCreatureGuid> {
    let mut candidates: Vec<(f32, DiscoveredCreatureGuid)> = Vec::new();
    for offset in 0..payload.len().saturating_sub(2) {
        if !matches!(payload[offset], 1 | 2) {
            continue;
        }
        let Some(guid_bytes) = payload.get(offset + 1..) else {
            continue;
        };
        let Some((guid_len, low, high)) = parse_packed_guid(guid_bytes) else {
            continue;
        };
        if ((high >> 58) & 0x3F) != 8
            || ((high >> 29) & 0x1FFF) != u64::from(map_id)
            || ((high >> 6) & 0x7F_FFFF) != u64::from(entry)
            || expected_counter.is_some_and(|expected| low != expected & OBJECT_GUID_COUNTER_MASK)
        {
            continue;
        }
        let Some(mut position) = offset.checked_add(1 + guid_len) else {
            continue;
        };
        if payload.get(position).copied() != Some(5) {
            continue;
        }
        position += 1;
        // C++ CreateObjectBits are 18 MSB-first bits, flushed to three bytes.
        let Some(next_position) = position.checked_add(3) else {
            continue;
        };
        position = next_position;
        let Some(mover_bytes) = payload.get(position..) else {
            continue;
        };
        let Some((mover_len, mover_low, mover_high)) = parse_packed_guid(mover_bytes) else {
            continue;
        };
        if (mover_low, mover_high) != (low, high) {
            continue;
        }
        let Some(next_position) = position.checked_add(mover_len + 12 + 4) else {
            continue;
        };
        position = next_position;
        let (Some(x), Some(y), Some(z)) = (
            read_f32_at(payload, position),
            read_f32_at(payload, position + 4),
            read_f32_at(payload, position + 8),
        ) else {
            continue;
        };
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            continue;
        }
        let distance =
            ((x - expected_x).powi(2) + (y - expected_y).powi(2) + (z - expected_z).powi(2)).sqrt();
        if distance > max_distance.max(0.0) {
            continue;
        }
        let candidate = DiscoveredCreatureGuid { low, high, x, y, z };
        if let Some(existing) = candidates
            .iter_mut()
            .find(|(_, existing)| (existing.low, existing.high) == (low, high))
        {
            if distance < existing.0 {
                *existing = (distance, candidate);
            }
        } else {
            candidates.push((distance, candidate));
        }
    }
    candidates.sort_by(|left, right| left.0.total_cmp(&right.0));
    candidates
        .into_iter()
        .map(|(_, candidate)| candidate)
        .collect()
}
pub(crate) fn find_creature_guid_near_position_in_update_object(
    payload: &[u8],
    map_id: u16,
    entry: u32,
    expected_x: f32,
    expected_y: f32,
    expected_z: f32,
    max_distance: f32,
    expected_counter: Option<u64>,
) -> Option<DiscoveredCreatureGuid> {
    find_creature_guids_near_position_in_update_object(
        payload,
        map_id,
        entry,
        expected_x,
        expected_y,
        expected_z,
        max_distance,
        expected_counter,
    )
    .into_iter()
    .next()
}
