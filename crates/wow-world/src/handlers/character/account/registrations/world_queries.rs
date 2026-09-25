use super::*;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCreature,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_creature",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryCreature::read(&mut pkt) {
                    Ok(query) => {
                        session
                            .handle_query_creature_with_catalogs_like_cpp(
                                catalogs.object_mgr.as_ref(),
                                query,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read QueryCreature: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryGameObject,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_game_object",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryGameObject::read(&mut pkt) {
                    Ok(query) => {
                        session
                            .handle_query_game_object_with_catalogs_like_cpp(
                                catalogs.object_mgr.as_ref(),
                                query,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read QueryGameObject: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCorpseLocationFromClient,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_corpse_location",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryCorpseLocationFromClient::read(&mut pkt) {
                    Ok(query) => session.handle_query_corpse_location(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryCorpseLocationFromClient: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryCorpseTransport,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_corpse_transport",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryCorpseTransport::read(&mut pkt) {
                    Ok(query) => session.handle_query_corpse_transport(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryCorpseTransport: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPageText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_page_text",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryPageText::read(&mut pkt) {
                    Ok(query) => {
                        session
                            .handle_query_page_text_with_catalogs_like_cpp(
                                catalogs.object_mgr.as_ref(),
                                query,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read QueryPageText: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ItemTextQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_item_text_query",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::ItemTextQuery::read(&mut pkt) {
                    Ok(query) => session.handle_item_text_query(query).await,
                    Err(e) => tracing::warn!("Failed to read ItemTextQuery: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPetName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_pet_name",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryPetName::read(&mut pkt) {
                    Ok(query) => session.handle_query_pet_name(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryPetName: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryPlayerNames,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_player_names",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryPlayerNames::read(&mut pkt) {
                    Ok(query) => session.handle_query_player_names(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryPlayerNames: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryRealmName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_realm_name",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::query::QueryRealmName::read(&mut pkt) {
                    Ok(query) => session.handle_query_realm_name(query),
                    Err(e) => tracing::warn!("Failed to read QueryRealmName: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Ping,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ping",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::auth::Ping::read(&mut pkt) {
                    Ok(ping) => session.handle_ping(ping).await,
                    Err(e) => tracing::warn!("Failed to read Ping: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TalkToGossip,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_gossip_hello",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => session.handle_gossip_hello(hello).await,
                    Err(e) => tracing::warn!("Failed to read TalkToGossip: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GossipSelectOption,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_gossip_select_option",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::GossipSelectOption::read(&mut pkt) {
                    Ok(select) => session.handle_gossip_select_option(select).await,
                    Err(e) => tracing::warn!("Failed to read GossipSelectOption: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryNpcText,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_npc_text",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::QueryNpcText::read(&mut pkt) {
                    Ok(query) => session.handle_query_npc_text(query).await,
                    Err(e) => tracing::warn!("Failed to read QueryNpcText: {e}"),
                }
            })
        },
    }
}
