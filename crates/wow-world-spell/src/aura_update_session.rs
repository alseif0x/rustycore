use wow_constants::ServerOpcodes;
use wow_world_core::session::mailbox::SendPlayerAuraUpdateCommand;
use wow_world_core::session::{SessionHubHostLikeCpp, SessionState};

/// Deliver one AuraUpdate whose recipient and visibility were committed when
/// the source aura change was published.
pub fn handle_player_aura_update<H: SessionHubHostLikeCpp>(
    host: &mut H,
    command: SendPlayerAuraUpdateCommand,
) {
    let opcode = command
        .packet_bytes
        .get(..2)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes);
    if opcode != Some(ServerOpcodes::AuraUpdate as u16)
        || host.state() != SessionState::LoggedIn
        || host.core().player_handle_like_cpp != Some(command.recipient_handle)
        || host.core().player_guid() != Some(command.recipient_handle.guid())
        || !host
            .core()
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&command.committed_visibility)
    {
        return;
    }

    let residence = host
        .core()
        .canonical_map_manager
        .as_ref()
        .and_then(|manager| manager.lock().ok())
        .and_then(|manager| {
            manager.player_active_residence_revision_like_cpp(command.recipient_handle)
        });
    if residence != Some((command.map_key, command.residence_revision)) {
        return;
    }

    // Do not re-check the source or current client-visible membership here:
    // this command is already the publication-time recipient decision.
    host.send_raw_packet(&command.packet_bytes);
}
