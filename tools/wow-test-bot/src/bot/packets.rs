//! Packets operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn stand_state_quiet_drain_ambient_opcode(opcode: u16) -> bool {
    matches!(opcode, SMSG_TIME_SYNC_REQUEST | SMSG_ON_MONSTER_MOVE)
}
pub(crate) fn read_equipment_u32(payload: &[u8], offset: &mut usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        take_equipment_bytes(payload, offset, 4)?.try_into()?,
    ))
}
pub(crate) fn read_equipment_i32(payload: &[u8], offset: &mut usize) -> Result<i32> {
    Ok(i32::from_le_bytes(
        take_equipment_bytes(payload, offset, 4)?.try_into()?,
    ))
}
pub(crate) fn read_equipment_u64(payload: &[u8], offset: &mut usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        take_equipment_bytes(payload, offset, 8)?.try_into()?,
    ))
}
pub(crate) fn read_equipment_msb_bits(
    payload: &[u8],
    bit_offset: &mut usize,
    count: usize,
) -> Result<u32> {
    let mut value = 0_u32;
    for _ in 0..count {
        let byte = *payload
            .get(*bit_offset / 8)
            .ok_or_else(|| anyhow!("equipment-set bit section truncated"))?;
        value = (value << 1) | u32::from((byte >> (7 - (*bit_offset % 8))) & 1);
        *bit_offset += 1;
    }
    Ok(value)
}
pub(crate) async fn read_encrypted_packet_if_ready(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
    readiness_wait: Duration,
    frame_timeout: Duration,
    context: &str,
) -> Result<Option<(u16, Vec<u8>)>> {
    // Waiting on `peek` is cancellation-safe: a readiness timeout cannot
    // consume a partial encrypted frame and desynchronize framing/crypt state.
    let mut peek = [0u8; 1];
    match tokio::time::timeout(readiness_wait, stream.peek(&mut peek)).await {
        Err(_) => return Ok(None),
        Ok(Ok(0)) => bail!("{context}: connection closed"),
        Ok(Ok(_)) => {}
        Ok(Err(error)) => return Err(anyhow!("{context}: peek failed: {error}")),
    }

    // Once data is ready, finish the whole frame. A timeout is terminal for
    // this workflow, so a partially consumed frame is never reused.
    let packet = tokio::time::timeout(
        frame_timeout,
        read_encrypted_packet(stream, crypt, server_inflater),
    )
    .await
    .map_err(|_| anyhow!("{context}: encrypted frame read timed out"))??;
    Ok(Some(packet))
}
pub(crate) fn handle_quest_smoke_packet(op: u16, payload: &[u8], result: &mut BotRunResult) {
    match op {
        0x2A98 => {
            result.quest_gossip_message_seen = true;
            result.quest_gossip_id_seen = packet_parser::parse_gossip_id(payload);
            if let Some(offers) = packet_parser::parse_gossip_quest_offers(payload) {
                record_quest_offers(offers, result);
            }
        }
        0x2A9A => {
            result.quest_quest_list_seen = true;
            if let Some(offers) = packet_parser::parse_quest_list_offers(payload) {
                record_quest_offers(offers, result);
            }
        }
        0x2A92 => {
            result.quest_details_seen = true;
            if let Some(details) = packet_parser::parse_quest_details_summary(payload) {
                record_quest_id(details.quest_id, result);
            }
        }
        0x2A93 => {
            result.quest_request_items_seen = true;
            if let Some(summary) = packet_parser::parse_quest_request_items_summary(payload) {
                record_quest_id(summary.quest_id, result);
            }
        }
        0x2A83 => {
            result.quest_accept_confirm_seen = true;
            if payload.len() >= 4 {
                record_quest_id(
                    u32::from_le_bytes(payload[0..4].try_into().unwrap()),
                    result,
                );
            }
        }
        0x26DF => {
            result.trainer_list_seen = true;
            if let Some(summary) = packet_parser::parse_trainer_list_summary(payload) {
                result.trainer_id_seen = Some(summary.trainer_id);
                result.trainer_spell_count_seen = Some(summary.spell_count);
            }
        }
        _ => {}
    }
}
/// Read unencrypted packet (18-byte header: size + tag placeholder + opcode)
pub(crate) async fn read_unencrypted_packet(stream: &mut TcpStream) -> Result<(u16, Vec<u8>)> {
    let mut header = [0u8; 18];
    stream.read_exact(&mut header).await?;

    let size = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if size == 0 || size > 0x10000 {
        bail!("Invalid packet size: {}", size);
    }

    let payload_size = size.saturating_sub(2);
    let mut payload = vec![0u8; payload_size];
    if payload_size > 0 {
        stream.read_exact(&mut payload).await?;
    }

    let opcode = u16::from_le_bytes([header[16], header[17]]);
    Ok((opcode, payload))
}
/// Send unencrypted packet
pub(crate) async fn send_unencrypted_packet(
    stream: &mut TcpStream,
    opcode: u16,
    data: &[u8],
) -> Result<()> {
    let mut packet = vec![0u8; 18];
    let size = (2 + data.len()) as u32;
    packet[0..4].copy_from_slice(&size.to_le_bytes());
    // bytes 4-15 are zeros (tag placeholder for unencrypted phase)
    packet[16..18].copy_from_slice(&opcode.to_le_bytes());

    stream.write_all(&packet).await?;
    if !data.is_empty() {
        stream.write_all(data).await?;
    }
    stream.flush().await?;
    Ok(())
}
/// Read encrypted packet (16-byte header: size + tag)
pub(crate) async fn read_encrypted_packet(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    server_inflater: &mut ServerPacketInflater,
) -> Result<(u16, Vec<u8>)> {
    let mut header = [0u8; 16];
    stream.read_exact(&mut header).await?;

    let size = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    const MAX_ENCRYPTED_PACKET_SIZE: usize = 8 * 1024 * 1024;
    if size == 0 || size > MAX_ENCRYPTED_PACKET_SIZE {
        bail!("Invalid encrypted packet size: {}", size);
    }

    let mut tag = [0u8; 12];
    tag.copy_from_slice(&header[4..16]);

    let mut ciphertext = vec![0u8; size];
    stream.read_exact(&mut ciphertext).await?;

    let plaintext = crypt
        .decrypt_server(&ciphertext, &tag, &[])
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    if plaintext.len() < 2 {
        bail!("Decrypted payload too short");
    }

    let opcode = u16::from_le_bytes([plaintext[0], plaintext[1]]);
    let payload = plaintext[2..].to_vec();
    if opcode == SMSG_COMPRESSED_PACKET {
        return decompress_server_packet_like_cpp(&payload, server_inflater);
    }
    Ok((opcode, payload))
}
pub(crate) fn decompress_server_packet_like_cpp(
    payload: &[u8],
    inflater: &mut ServerPacketInflater,
) -> Result<(u16, Vec<u8>)> {
    if payload.len() < 12 {
        bail!("Compressed packet too short: {} bytes", payload.len());
    }
    let uncompressed_size = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    if uncompressed_size < 2 {
        bail!("Invalid compressed packet size: {uncompressed_size}");
    }
    let uncompressed_size = uncompressed_size as usize;
    let deflated = &payload[12..];
    let mut output = vec![0u8; uncompressed_size + 256];
    let base_out = inflater.decompressor.total_out() as usize;

    inflater
        .decompressor
        .decompress(deflated, &mut output, FlushDecompress::Sync)
        .map_err(|e| anyhow!("Compressed packet inflate failed: {e}"))?;
    let produced = inflater.decompressor.total_out() as usize - base_out;
    if produced != uncompressed_size {
        bail!(
            "Compressed packet size mismatch: expected {}, got {}",
            uncompressed_size,
            produced
        );
    }
    output.truncate(produced);
    let opcode = u16::from_le_bytes([output[0], output[1]]);
    Ok((opcode, output[2..].to_vec()))
}
/// Send encrypted packet
pub(crate) async fn send_encrypted_packet(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    opcode: u16,
    data: &[u8],
) -> Result<()> {
    let mut plaintext = Vec::with_capacity(2 + data.len());
    plaintext.extend_from_slice(&opcode.to_le_bytes());
    plaintext.extend_from_slice(data);

    let (ciphertext, tag) = crypt
        .encrypt_client(&plaintext, &[])
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Build header: size + tag + encrypted_opcode_hint
    let mut header = [0u8; 18];
    let size = ciphertext.len() as u32;
    header[0..4].copy_from_slice(&size.to_le_bytes());
    header[4..16].copy_from_slice(&tag);
    if ciphertext.len() >= 2 {
        header[16..18].copy_from_slice(&ciphertext[0..2]);
    }

    stream.write_all(&header).await?;
    if ciphertext.len() > 2 {
        stream.write_all(&ciphertext[2..]).await?;
    }
    stream.flush().await?;
    Ok(())
}
pub(crate) fn read_f32_at(data: &[u8], position: usize) -> Option<f32> {
    let bytes: [u8; 4] = data
        .get(position..position.checked_add(4)?)?
        .try_into()
        .ok()?;
    Some(f32::from_le_bytes(bytes))
}
pub(crate) fn read_msb_bits(data: &[u8], bit_offset: usize, bit_count: usize) -> Option<u32> {
    if bit_count > 32 || bit_offset.checked_add(bit_count)? > data.len().checked_mul(8)? {
        return None;
    }
    let mut value = 0u32;
    for bit in bit_offset..bit_offset + bit_count {
        value = (value << 1) | u32::from((data[bit / 8] >> (7 - bit % 8)) & 1);
    }
    Some(value)
}
pub(crate) fn homebind_spell_go_seen_after_packet(
    already_seen: bool,
    payload: &[u8],
    expected_caster_low: u64,
    expected_caster_high: u64,
    expected_player_low: u64,
    expected_player_high: u64,
) -> bool {
    already_seen
        || spell_go_matches_bind(
            payload,
            expected_caster_low,
            expected_caster_high,
            expected_player_low,
            expected_player_high,
        )
}
