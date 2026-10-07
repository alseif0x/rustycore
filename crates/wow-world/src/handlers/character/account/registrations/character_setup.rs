use super::*;

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::EnumCharacters,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_enum_characters",
        handler: |session, catalogs, _pkt| {
            Box::pin(async move {
                session
                    .handle_enum_characters_with_policy_like_cpp(
                        catalogs.support_feature_policy.as_ref(),
                    )
                    .await
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CreateCharacter,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_create_character",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::character::CreateCharacter::read(&mut pkt) {
                    Ok(create) => {
                        session
                            .handle_create_character_with_generator_like_cpp(
                                catalogs.id_generators.player.as_ref(),
                                create,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read CreateCharacter: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CharCustomize,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_customize",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::character::CharCustomize::read(&mut pkt) {
                    Ok(customize) => session.handle_char_customize(customize).await,
                    Err(e) => tracing::warn!("Failed to read CharCustomize: {e}"),
                }
            })
        },
    }
}

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
