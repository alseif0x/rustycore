use super::*;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ServerTimeOffsetRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_server_time_offset_request",
        handler: |session, _catalogs, _pkt| {
            Box::pin(async move { session.handle_server_time_offset_request().await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPlayedTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_played_time",
 handler: |session, _catalogs, mut pkt| {
     Box::pin(async move { let trigger = pkt.read_uint8().unwrap_or(0) != 0; session.handle_request_played_time(trigger).await })
 },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetProductList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pay_stub",
        handler: |_session, _catalogs, _pkt| {
            Box::pin(async move { tracing::trace!("Stub handler for {:?} (0x{:04X}) — no response needed", ClientOpcodes::BattlePayGetProductList, ClientOpcodes::BattlePayGetProductList as u32) })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePayGetPurchaseList,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pay_stub",
        handler: |_session, _catalogs, _pkt| {
            Box::pin(async move { tracing::trace!("Stub handler for {:?} (0x{:04X}) — no response needed", ClientOpcodes::BattlePayGetPurchaseList, ClientOpcodes::BattlePayGetPurchaseList as u32) })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateVasPurchaseStates,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_vas_stub",
        handler: |_session, _catalogs, _pkt| {
            Box::pin(async move { tracing::trace!("Stub handler for {:?} (0x{:04X}) — no response needed", ClientOpcodes::UpdateVasPurchaseStates, ClientOpcodes::UpdateVasPurchaseStates as u32) })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DbQueryBulk,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_db_query_bulk",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::DbQueryBulk::read(&mut pkt) {
                    Ok(query) => {
                        session
                            .handle_db_query_bulk_with_tact_keys_like_cpp(
                                catalogs.tact_keys.as_ref(),
                                query,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read DbQueryBulk: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::HotfixRequest,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_hotfix_request",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::HotfixRequest::read(&mut pkt) {
                    Ok(req) => session.handle_hotfix_request(catalogs.hotfixes.as_ref(), req).await,
                    Err(e) => tracing::warn!("Failed to read HotfixRequest: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::TimeSyncResponse::read(&mut pkt) {
                    Ok(resp) => session.handle_time_sync_response(resp).await,
                    Err(e) => tracing::warn!("Failed to read TimeSyncResponse: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponseDropped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::TimeSyncResponse::read(&mut pkt) {
                    Ok(resp) => session.handle_time_sync_response(resp).await,
                    Err(e) => tracing::warn!("Failed to read TimeSyncResponse: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TimeSyncResponseFailed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_time_sync_response",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::TimeSyncResponse::read(&mut pkt) {
                    Ok(resp) => session.handle_time_sync_response(resp).await,
                    Err(e) => tracing::warn!("Failed to read TimeSyncResponse: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_request",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::LogoutRequest::read(&mut pkt) {
                    Ok(req) => {
                        session
                            .handle_logout_request_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                req,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read LogoutRequest: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LogoutCancel,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_logout_cancel",
        handler: |session, _catalogs, _pkt| Box::pin(async move { session.handle_logout_cancel().await }),
    }
}
