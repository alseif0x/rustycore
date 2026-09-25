use super::*;

inventory::submit! {
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

inventory::submit! {
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CharDelete,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_char_delete",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::character::CharDelete::read(&mut pkt) {
                    Ok(del) => session.handle_char_delete(del).await,
                    Err(e) => tracing::warn!("Failed to read CharDelete: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CharacterRenameRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_character_rename_request",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::character::CharacterRenameRequest::read(&mut pkt) {
                    Ok(rename) => session.handle_character_rename_request(rename).await,
                    Err(e) => tracing::warn!("Failed to read CharacterRenameRequest: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
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

inventory::submit! {
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OpeningCinematic,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_opening_cinematic",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_opening_cinematic(pkt).await })
        },
    }
}

inventory::submit! {
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetUndeleteCharacterCooldownStatus,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_get_undelete_cooldown_status",
        handler: |session, _catalogs, _pkt| {
            Box::pin(async move { session.handle_get_undelete_cooldown_status().await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AlterAppearance,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_alter_appearance",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_alter_appearance(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmBarbersChoice,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_barbers_choice",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_confirm_barbers_choice(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetPlayerDeclinedNames,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_player_declined_names",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_set_player_declined_names(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SaveEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_save_equipment_set",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_save_equipment_set_with_generator_like_cpp(
                        catalogs.id_generators.equipment_set.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AssignEquipmentSetSpec,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_assign_equipment_set_spec",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_assign_equipment_set_spec(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DeleteEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_delete_equipment_set",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_delete_equipment_set(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UseEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_use_equipment_set",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_use_equipment_set(pkt).await })
        },
    }
}

// ── Stub registrations for character-select opcodes ──────────────────
