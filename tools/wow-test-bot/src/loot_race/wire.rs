//! Loot-race wire operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

/// C++ `Opcodes.cpp` maps these party packets to
/// `CONNECTION_TYPE_REALM`. Seeing one on the instance socket is a routing
/// defect, not harmless traffic that the atomic-loot harness may discard.
pub(crate) fn validate_party_packet_route_like_cpp(
    opcode: u16,
    route: PartyPacketRoute,
) -> Result<()> {
    if route == PartyPacketRoute::Instance
        && matches!(
            opcode,
            SMSG_PARTY_INVITE
                | SMSG_PARTY_UPDATE
                | SMSG_PARTY_COMMAND_RESULT
                | SMSG_PARTY_MEMBER_FULL_STATE
        )
    {
        bail!(
            "realm-only party opcode 0x{opcode:04X} arrived on instance while forming the loot-race party; C++ Opcodes.cpp requires CONNECTION_TYPE_REALM"
        );
    }
    Ok(())
}
pub(crate) async fn wait_for_realm_opcode(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &LootRaceOptions,
    expected: u16,
    result: &mut BotRunResult,
) -> Result<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for realm opcode 0x{expected:04X}");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "loot-race realm packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Realm)?;
            if opcode == SMSG_PARTY_UPDATE {
                validate_party_update_like_cpp(
                    &payload,
                    create_player_guid_raw(options.character_guid, realm_id()),
                    create_player_guid_raw(options.peer_character_guid, realm_id()),
                    create_player_guid_raw(options.killer_character_guid, realm_id()),
                )?;
            }
            if opcode == expected {
                return Ok(payload);
            }
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "loot-race instance packet while forming party",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}
pub(crate) async fn wait_for_group_capacity_realm_opcode(
    bot_index: usize,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut EncryptedWorldConnection,
    options: &GroupCapacityRaceOptions,
    expected: u16,
    result: &mut BotRunResult,
) -> Result<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.timeout_secs);
    loop {
        options.sync.cancellation_error()?;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for group-capacity realm opcode 0x{expected:04X}");
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(50)),
            remaining,
            "group-capacity realm packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Realm)?;
            if opcode == expected {
                return Ok(payload);
            }
        }
        if let Some((opcode, payload)) = read_encrypted_packet_if_ready(
            stream,
            crypt,
            inflater,
            Duration::from_millis(1),
            remaining,
            "group-capacity instance packet",
        )
        .await?
        {
            result.seen_opcodes.push(format!("0x{opcode:04X}"));
            validate_party_packet_route_like_cpp(opcode, PartyPacketRoute::Instance)?;
            handle_instance_housekeeping(bot_index, stream, crypt, opcode, &payload).await?;
        }
    }
}
