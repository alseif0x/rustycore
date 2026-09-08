//! Evidence that both login streams settled after an INSTANCE object update.

use super::{
    build_time_sync_response_payload, parse_time_sync_request_sequence, read_encrypted_packet,
    send_encrypted_packet, BotRunResult, EncryptedWorldConnection, ServerPacketInflater,
    WorldCrypt, CMSG_TIME_SYNC_RESPONSE, SMSG_TIME_SYNC_REQUEST, SMSG_UPDATE_OBJECT,
};
use anyhow::{bail, Context, Result};
use std::time::Duration;
use tokio::net::TcpStream;

/// Retain publication across the initial login loop and the subsequent drain.
/// Before redirection, the login loop still reads REALM and cannot grant this proof.
pub(super) fn observe_login_packet(instance: bool, opcode: u16, result: &mut BotRunResult) {
    result.login_instance_object_update_seen |= instance && opcode == SMSG_UPDATE_OBJECT;
}

// C++ Map::AddPlayerToMap / SendInitSelf publishes UPDATE_OBJECT after
// LOGIN_VERIFY_WORLD. Keep both sockets alive through that publication and a
// bounded quiet period; this is a stream-drain proof, not a gameplay acceptance.
pub(super) async fn drain_login_streams(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm_connection: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<()> {
    let realm = realm_connection
        .as_mut()
        .context("login drain requires both sockets")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("login streams did not settle after object publication");
        }
        let mut instance_peek = [0; 1];
        let mut realm_peek = [0; 1];
        // Peek is cancellation-safe: never race partial encrypted frame reads.
        let ready = tokio::time::timeout(remaining.min(Duration::from_secs(1)), async {
            tokio::select! {
                bytes = stream.peek(&mut instance_peek) => {
                    if bytes? == 0 { bail!("instance closed during login drain"); }
                    Ok::<bool, anyhow::Error>(true)
                }
                bytes = realm.stream.peek(&mut realm_peek) => {
                    if bytes? == 0 { bail!("realm closed during login drain"); }
                    Ok(false)
                }
            }
        })
        .await;
        let instance = match ready {
            Ok(ready) => ready?,
            Err(_)
                if result.login_instance_object_update_seen
                    && remaining >= Duration::from_secs(1) =>
            {
                result.login_stream_drained = true;
                return Ok(());
            }
            Err(_) => continue,
        };
        let (opcode, payload) = tokio::time::timeout_at(deadline, async {
            if instance {
                read_encrypted_packet(stream, crypt, inflater).await
            } else {
                read_encrypted_packet(&mut realm.stream, &mut realm.crypt, &mut realm.inflater)
                    .await
            }
        })
        .await
        .context("login drain frame deadline exceeded")??;
        result.seen_opcodes.push(format!("0x{opcode:04X}"));
        observe_login_packet(instance, opcode, result);
        if opcode == SMSG_TIME_SYNC_REQUEST {
            // MiscPackets.cpp TimeSyncRequest::Write / TimeSyncResponse::Read.
            let sequence = parse_time_sync_request_sequence(&payload)?;
            let response = build_time_sync_response_payload(sequence, 0);
            tokio::time::timeout_at(
                deadline,
                send_encrypted_packet(stream, crypt, CMSG_TIME_SYNC_RESPONSE, &response),
            )
            .await
            .context("login drain time-sync write deadline exceeded")??;
        }
    }
}

#[cfg(test)]
mod tests;
