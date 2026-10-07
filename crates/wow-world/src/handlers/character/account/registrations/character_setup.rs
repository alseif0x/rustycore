use super::*;

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PlayerLogin,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_player_login",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::character::PlayerLogin::read(&mut pkt) {
                    Ok(login) => session.handle_player_login(login).await,
                    Err(e) => tracing::warn!("Failed to read PlayerLogin: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConnectToFailed,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_connect_to_failed",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::auth::ConnectToFailed::read(&mut pkt) {
                    Ok(failed) => session.handle_connect_to_failed(failed).await,
                    Err(e) => tracing::warn!("Failed to read ConnectToFailed: {e}"),
                }
            })
        },
    }
}

// ── Stub registrations for character-select opcodes ──────────────────
