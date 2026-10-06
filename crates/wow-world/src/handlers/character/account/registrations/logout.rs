use super::*;

crate::session::registry::register_packet_handler_like_cpp! {
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
