//! #585: observe a real pending worldport, deliberately never send its ACK.
//! C++ AreaTriggerPackets.cpp:66, MovementPackets.cpp:657/696/1005,
//! Opcodes.cpp:1811/2150/2173; WorldSession.cpp:544 completes it on logout.
use super::*;

const TRIGGER: u32 = 2173;
const SOURCE: [f32; 3] = [-8346.46, 514.031, 96.5989];
const DESTINATION: [f32; 3] = [67.7607, 2490.98, -4.29649];
const MAP: u32 = 369;

#[derive(Debug, Clone, Serialize)]
pub(super) struct Receipt {
    trigger_id: u32,
    destination_map: u32,
    transfer_pending_realm: String,
    suspend_token_instance: String,
    new_world_realm: String,
    worldport_ack_sent: bool,
    pub(super) persisted_destination: bool,
}

pub(super) fn enabled() -> bool {
    flag("WOW_BOT_LOGIN_PORTAL_CHECK")
}

fn near(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| a.is_finite() && (*a - e).abs() < 0.02)
}

pub(super) fn preflight(conn: &mut mysql::Conn, bot: &config::BotConfig) -> Result<()> {
    let row: Option<(u32, f32, f32, f32)> = conn.exec_first(
        "SELECT map, position_x, position_y, position_z FROM characters WHERE guid=? AND account=? AND online=0",
        (bot.character_guid, bot.account_id),
    )?;
    if !row.is_some_and(|(map, x, y, z)| map == 0 && near([x, y, z], SOURCE)) {
        bail!("portal QA requires the prepared offline Stormwind tram fixture");
    }
    Ok(())
}

fn check_pending(payload: &[u8]) -> Result<()> {
    if payload.len() != 17 || payload[0..4] != MAP.to_le_bytes() || payload[16] != 0 {
        bail!("unexpected transfer-pending map/layout/options");
    }
    Ok(())
}

fn check_new_world(payload: &[u8]) -> Result<()> {
    if payload.len() != 44 || payload[0..4] != MAP.to_le_bytes() {
        bail!("unexpected new-world map/layout");
    }
    let xyz = std::array::from_fn(|i| {
        f32::from_le_bytes(payload[4 + i * 4..8 + i * 4].try_into().unwrap())
    });
    if !near(xyz, DESTINATION) || payload[20..28] != [255; 8] || payload[28..44] != [0; 16] {
        bail!("unexpected new-world destination/reason/offset");
    }
    Ok(())
}

pub(super) async fn begin(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<Receipt> {
    let realm = realm.as_mut().context("portal requires both transports")?;
    let mut request = TRIGGER.to_le_bytes().to_vec();
    request.push(0xC0); // Entered=true, FromClient=true; MSB-first bits.
    send_encrypted_packet(stream, crypt, 0x31D6, &request).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let mut pending = None;
    let mut suspend = None;
    while tokio::time::Instant::now() < deadline {
        for is_realm in [true, false] {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let packet = if is_realm {
                read_encrypted_packet_if_ready(
                    &mut realm.stream,
                    &mut realm.crypt,
                    &mut realm.inflater,
                    Duration::from_millis(20),
                    remaining,
                    "portal realm",
                )
                .await?
            } else {
                read_encrypted_packet_if_ready(
                    stream,
                    crypt,
                    inflater,
                    Duration::from_millis(20),
                    remaining,
                    "portal instance",
                )
                .await?
            };
            let Some((opcode, payload)) = packet else {
                continue;
            };
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            match opcode {
                0x25CD => {
                    if !is_realm || pending.is_some() {
                        bail!("misrouted/duplicate transfer-pending")
                    }
                    check_pending(&payload)?;
                    pending = Some(hex::encode(payload));
                }
                0x25A8 => {
                    if is_realm || suspend.is_some() || payload.len() != 5 || payload[4] != 0x40 {
                        bail!("misrouted/duplicate/invalid suspend-token");
                    }
                    send_encrypted_packet(stream, crypt, 0x376A, &payload[..4]).await?;
                    suspend = Some(hex::encode(payload));
                }
                0x2594 => {
                    if !is_realm {
                        bail!("new-world must arrive on realm")
                    }
                    check_new_world(&payload)?;
                    return Ok(Receipt {
                        trigger_id: TRIGGER,
                        destination_map: MAP,
                        transfer_pending_realm: pending
                            .context("new-world without transfer-pending")?,
                        suspend_token_instance: suspend
                            .context("new-world without suspend response")?,
                        new_world_realm: hex::encode(payload),
                        worldport_ack_sent: false,
                        persisted_destination: false,
                    });
                }
                // Keep the represented time-sync exchange alive, without sending a worldport ACK.
                SMSG_TIME_SYNC_REQUEST if !is_realm => {
                    let sequence = parse_time_sync_request_sequence(&payload)?;
                    let response = build_time_sync_response_payload(sequence, 0);
                    send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response)
                        .await?;
                }
                _ => {}
            }
        }
    }
    bail!("portal never reached the pending new-world boundary")
}

pub(super) fn verify_saved(
    conn: &mut mysql::Conn,
    bot: &config::BotConfig,
    receipt: &mut Receipt,
) -> Result<()> {
    let row: Option<(u32, f32, f32, f32)> = conn.exec_first(
        "SELECT map, position_x, position_y, position_z FROM characters WHERE guid=? AND account=? AND online=0",
        (bot.character_guid, bot.account_id),
    )?;
    if !row.is_some_and(|(map, x, y, z)| map == MAP && near([x, y, z], DESTINATION)) {
        bail!("pending transfer did not save the intended destination");
    }
    receipt.persisted_destination = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_world_requires_exact_map_destination_and_shape() {
        let mut p = MAP.to_le_bytes().to_vec();
        for value in DESTINATION {
            p.extend_from_slice(&value.to_le_bytes());
        }
        p.extend_from_slice(&0f32.to_le_bytes());
        p.extend_from_slice(&[255; 8]);
        p.extend_from_slice(&[0; 16]);
        assert!(check_new_world(&p).is_ok());
        assert!(check_new_world(&p[..43]).is_err());
        p[0] ^= 1;
        assert!(check_new_world(&p).is_err());
        assert!(!near([f32::NAN, 0.0, 0.0], SOURCE));
    }
}
