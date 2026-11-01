//! Live creature ObjectGuid discovery after the client's active-mover ACK.
//!
//! C++ `Player::CanNeverSee` (Player.cpp) keeps every world object hidden
//! until `PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME` is set, and only
//! `WorldSession::HandleMoveInitActiveMoverComplete` (MovementHandler.cpp)
//! sets it before calling `UpdateObjectVisibility(false)`. A login that never
//! sends `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE` therefore never receives the
//! nearby creature CREATE blocks, however long it waits.

use super::{
    build_move_init_active_mover_complete_payload, build_packed_guid,
    build_time_sync_response_payload, find_creature_guid_near_position_in_update_object,
    parse_time_sync_request_sequence, read_encrypted_packet_if_ready, send_encrypted_packet,
    BotRunResult, ResolvedCreatureTarget, ServerPacketInflater, WorldCrypt,
    CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE, CMSG_TIME_SYNC_RESPONSE, OBJECT_GUID_COUNTER_MASK,
    SMSG_TIME_SYNC_REQUEST, SMSG_UPDATE_OBJECT,
};
use anyhow::{anyhow, Result};
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

/// Window for a SQL-resolved target whose live counter is unknown.
pub(crate) const RESOLVED_TARGET_DISCOVERY_WINDOW: Duration = Duration::from_secs(10);

/// Link a SQL spawn (`guid_counter == 0`) to its live ObjectGuid: the CREATE
/// must carry the same map/entry and lie within `match_radius` of the SQL
/// position. A target with an explicit counter is left untouched.
pub(crate) async fn discover_resolved_creature_target(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    target: &mut ResolvedCreatureTarget,
    match_radius: f32,
    context: &str,
    result: &mut BotRunResult,
) -> Result<()> {
    if target.guid_counter != 0 {
        return Ok(());
    }
    let (map_id, entry) = (target.map_id, target.entry);
    let (x, y, z) = (target.x as f32, target.y as f32, target.z as f32);
    let found = discover_creature_after_active_mover_ack(
        stream,
        crypt,
        inflater,
        RESOLVED_TARGET_DISCOVERY_WINDOW,
        context,
        result,
        |payload| {
            find_creature_guid_near_position_in_update_object(
                payload,
                map_id,
                entry,
                x,
                y,
                z,
                match_radius,
                None,
            )
            .map(|found| (found.low, found.high))
        },
    )
    .await?;
    let (low, high) = found.ok_or_else(|| {
        anyhow!(
            "{context}: entry {entry} spawn {} was not discovered near its SQL position after CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE",
            target.spawn_guid
        )
    })?;
    target.guid_counter = low & OBJECT_GUID_COUNTER_MASK;
    target.packed_guid = build_packed_guid(low, high);
    Ok(())
}

/// Match radius for a SQL spawn whose live counter will be discovered, with
/// the vendor/rested-XP fixture guard: position matching is only sound when
/// no other same-entry spawn's movement radius overlaps this one.
pub(crate) fn discovery_match_radius_for_spawn(
    world: &mut mysql::Conn,
    label: &str,
    target: &ResolvedCreatureTarget,
    wander_distance: f32,
) -> Result<f32> {
    use mysql::prelude::Queryable;

    let radius = wander_distance.max(0.0) + 2.0;
    let overlapping: Option<u64> = world
        .exec_first(
            "SELECT guid FROM creature \
             WHERE id = ? AND map = ? AND guid <> ? \
               AND SQRT(POW(position_x - ?, 2) + POW(position_y - ?, 2) + POW(position_z - ?, 2)) \
                   <= ? + GREATEST(wander_distance, 0) \
             ORDER BY guid LIMIT 1",
            (
                target.entry,
                u32::from(target.map_id),
                target.spawn_guid,
                target.x,
                target.y,
                target.z,
                radius,
            ),
        )
        .map_err(|error| anyhow!("Check {label} spawn ambiguity: {error}"))?;
    if let Some(overlapping) = overlapping {
        return Err(anyhow!(
            "{label} SQL spawn {} overlaps same-entry spawn {overlapping}; supply a trusted live runtime counter or choose an isolated spawn",
            target.spawn_guid
        ));
    }
    Ok(radius)
}

#[cfg(test)]
mod tests;
