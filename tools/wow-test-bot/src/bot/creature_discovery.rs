//! Live creature ObjectGuid discovery after the client's active-mover ACK.
//!
//! C++ `Player::CanNeverSee` (Player.cpp) keeps every world object hidden
//! until `PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME` is set, and only
//! `WorldSession::HandleMoveInitActiveMoverComplete` (MovementHandler.cpp)
//! sets it before calling `UpdateObjectVisibility(false)`. A login that never
//! sends `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE` therefore never receives the
//! nearby creature CREATE blocks, however long it waits.

use super::{
    build_move_init_active_mover_complete_payload, build_time_sync_response_payload,
    parse_time_sync_request_sequence, read_encrypted_packet_if_ready, send_encrypted_packet,
    BotRunResult, ServerPacketInflater, WorldCrypt, CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
    CMSG_TIME_SYNC_RESPONSE, SMSG_TIME_SYNC_REQUEST, SMSG_UPDATE_OBJECT,
};
use anyhow::Result;
use std::time::Duration;
use tokio::net::TcpStream;

/// Poll granularity only. A quiet gap is not the end of discovery: the
/// visibility work triggered by the ACK may arrive after an idle period.
const DISCOVERY_POLL: Duration = Duration::from_millis(250);
const DISCOVERY_FRAME_TIMEOUT: Duration = Duration::from_secs(5);

/// Send the active-mover ACK on the INSTANCE socket, then read it until
/// `matcher` recognises the target in an `SMSG_UPDATE_OBJECT` or `window`
/// expires. Time-sync requests are answered so the session stays healthy.
pub(crate) async fn discover_creature_after_active_mover_ack<M>(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    window: Duration,
    context: &str,
    result: &mut BotRunResult,
    mut matcher: M,
) -> Result<Option<(u64, u64)>>
where
    M: FnMut(&[u8]) -> Option<(u64, u64)>,
{
    let ack = build_move_init_active_mover_complete_payload(0);
    send_encrypted_packet(stream, crypt, CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, &ack).await?;
    let deadline = tokio::time::Instant::now() + window;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            DISCOVERY_POLL.min(remaining),
            DISCOVERY_FRAME_TIMEOUT,
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
        } else if opcode == SMSG_UPDATE_OBJECT {
            if let Some(found) = matcher(&payload) {
                return Ok(Some(found));
            }
        }
    }
}

#[cfg(test)]
mod tests;
