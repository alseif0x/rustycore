// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account-scoped character enumeration, offline marking and account collections.

use wow_packet::ClientPacket;

use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomRowLikeCpp, AccountMaskBlockLikeCpp, AccountMountRowLikeCpp,
    AccountToyRowLikeCpp, CharacterEnumerationLoadOutcomeLikeCpp,
    CharacterEnumerationRequestLikeCpp, PersistenceOutcomeLikeCpp, PlayerOfflineMarkLikeCpp,
};

use super::*;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::EnumCharacters,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_enum_characters",
        handler: |session, _catalogs, _pkt| Box::pin(async move { session.handle_enum_characters().await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CreateCharacter,
        status: SessionStatus::Authed,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_create_character",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::character::CreateCharacter::read(&mut pkt) {
                    Ok(create) => session.handle_create_character(create).await,
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
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_save_equipment_set(pkt).await })
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
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::DbQueryBulk::read(&mut pkt) {
                    Ok(query) => session.handle_db_query_bulk(query).await,
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
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::HotfixRequest::read(&mut pkt) {
                    Ok(req) => session.handle_hotfix_request(req).await,
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
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::LogoutRequest::read(&mut pkt) {
                    Ok(req) => session.handle_logout_request(req).await,
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
                            .handle_query_creature_with_catalogs_like_cpp(catalogs, query)
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
                            .handle_query_game_object_with_catalogs_like_cpp(catalogs, query)
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
                            .handle_query_page_text_with_catalogs_like_cpp(catalogs, query)
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ListInventory,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_list_inventory",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => session.handle_list_inventory(hello).await,
                    Err(e) => tracing::warn!("Failed to read ListInventory: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::BuyItem::read(&mut pkt) {
                    Ok(buy) => session.handle_buy_item(buy).await,
                    Err(e) => tracing::warn!("Failed to read BuyItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBackItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_back_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::BuyBackItem::read(&mut pkt) {
                    Ok(buyback) => session.handle_buy_back_item(buyback).await,
                    Err(e) => tracing::warn!("Failed to read BuyBackItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_sell_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::SellItem::read(&mut pkt) {
                    Ok(sell) => session.handle_sell_item(sell).await,
                    Err(e) => tracing::warn!("Failed to read SellItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ItemPurchaseRefund,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_item_purchase_refund",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::ItemPurchaseRefund::read(&mut pkt) {
                    Ok(refund) => session.handle_item_purchase_refund(refund).await,
                    Err(e) => tracing::warn!("Failed to read ItemPurchaseRefund: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AuctionHelloRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_auction_hello_request",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_auction_hello_request(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BankerActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_banker_activate",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => session.handle_banker_activate(hello).await,
                    Err(e) => tracing::warn!("Failed to read BankerActivate: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutobankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autobank_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AutoBankItem::read(&mut pkt) {
                    Ok(packet) => session.handle_autobank_item(packet).await,
                    Err(e) => tracing::warn!("Failed to read AutobankItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutostoreBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autostore_bank_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AutoStoreBankItem::read(&mut pkt) {
                    Ok(packet) => session.handle_autostore_bank_item(packet).await,
                    Err(e) => tracing::warn!("Failed to read AutostoreBankItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBankSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_bank_slot",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::BuyBankSlot::read(&mut pkt) {
                    Ok(buy) => session.handle_buy_bank_slot(buy).await,
                    Err(e) => tracing::warn!("Failed to read BuyBankSlot: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeBankBagSlotFlag,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_change_bank_bag_slot_flag",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::ChangeBankBagSlotFlag::read(&mut pkt) {
                    Ok(change) => session.handle_change_bank_bag_slot_flag(change).await,
                    Err(e) => tracing::warn!("Failed to read ChangeBankBagSlotFlag: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BinderActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_binder_activate",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => session.handle_binder_activate(hello).await,
                    Err(e) => tracing::warn!("Failed to read BinderActivate: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TabardVendorActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_tabard_vendor_activate",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_tabard_vendor_activate(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AreaSpiritHealerQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_spirit_healer_query",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_area_spirit_healer_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AreaSpiritHealerQueue,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_area_spirit_healer_queue",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_area_spirit_healer_queue(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::HearthAndResurrect,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_hearth_and_resurrect",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_hearth_and_resurrect(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpiritHealerActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_spirit_healer_activate",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_spirit_healer_activate(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RepairItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_repair_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::RepairItem::read(&mut pkt) {
                    Ok(repair) => session.handle_repair_item(repair).await,
                    Err(e) => tracing::warn!("Failed to read RepairItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestStabledPets,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_stabled_pets",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_request_stabled_pets(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusMultipleQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_status_multiple_query",
        handler: |session, _catalogs, _pkt| {
            Box::pin(async move { session.handle_quest_giver_status_multiple_query().await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusTrackedQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_tracked_query",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_status_tracked_query(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapInvItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_inv_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::SwapInvItem::read(&mut pkt) {
                    Ok(swap) => session.handle_swap_inv_item(swap).await,
                    Err(e) => tracing::warn!("Failed to read SwapInvItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoEquipItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_equip_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::AutoEquipItem::read(&mut pkt) {
                    Ok(equip) => session.handle_auto_equip_item(equip).await,
                    Err(e) => tracing::warn!("Failed to read AutoEquipItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoEquipItemSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_equip_item_slot",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::AutoEquipItemSlot::read(&mut pkt) {
                    Ok(equip) => session.handle_auto_equip_item_slot(equip).await,
                    Err(e) => tracing::warn!("Failed to read AutoEquipItemSlot: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::SwapItem::read(&mut pkt) {
                    Ok(swap) => session.handle_swap_item(swap).await,
                    Err(e) => tracing::warn!("Failed to read SwapItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutoStoreBagItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_auto_store_bag_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::AutoStoreBagItem::read(&mut pkt) {
                    Ok(store) => session.handle_auto_store_bag_item(store).await,
                    Err(e) => tracing::warn!("Failed to read AutoStoreBagItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DestroyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_destroy_item",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::DestroyItemPkt::read(&mut pkt) {
                    Ok(destroy) => session.handle_destroy_item(destroy).await,
                    Err(e) => tracing::warn!("Failed to read DestroyItem: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelTempEnchantment,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_temp_enchantment",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::CancelTempEnchantment::read(&mut pkt) {
                    Ok(cancel) => session.handle_cancel_temp_enchantment(cancel).await,
                    Err(e) => tracing::warn!("Failed to read CancelTempEnchantment: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ShowTradeSkill,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_show_trade_skill",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::ShowTradeSkill::read(&mut pkt) {
                    Ok(_) => session.handle_show_trade_skill().await,
                    Err(e) => tracing::warn!("Failed to read ShowTradeSkill: {e}"),
                }
            })
        },
    }
}

impl WorldSession {
    /// Handle CMSG_ENUM_CHARACTERS — list characters for this account.
    pub async fn handle_enum_characters(&mut self) {
        let port = match self.character_enumeration_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                warn!(
                    "No character enumeration persistence port for account {}",
                    self.account_id
                );
                self.send_packet(&EnumCharactersResult {
                    success: false,
                    characters: vec![],
                    race_unlock_data: vec![],
                });
                return;
            }
        };

        let request = CharacterEnumerationRequestLikeCpp {
            account_id: self.account_id,
            declined_names_used: self.declined_names_used_like_cpp(),
        };
        let (rows, cleanup_error) = match port.load_character_enumeration_like_cpp(request).await {
            CharacterEnumerationLoadOutcomeLikeCpp::Loaded {
                rows,
                expired_ban_cleanup_error,
            } => (rows, expired_ban_cleanup_error),
            CharacterEnumerationLoadOutcomeLikeCpp::Failed {
                reason,
                expired_ban_cleanup_error,
            } => {
                if let Some(error) = expired_ban_cleanup_error {
                    warn!(
                        "Failed to expire elapsed character bans before enum for account {}: {error}",
                        self.account_id
                    );
                }
                warn!(
                    "Failed to query characters for account {}: {reason}",
                    self.account_id
                );
                self.send_packet(&EnumCharactersResult {
                    success: false,
                    characters: vec![],
                    race_unlock_data: vec![],
                });
                return;
            }
        };
        if let Some(error) = cleanup_error {
            warn!(
                "Failed to expire elapsed character bans before enum for account {}: {error}",
                self.account_id
            );
        }

        let mut characters = Vec::new();
        let mut legit_guids = Vec::new();

        for row in rows {
            let realm_id = self.realm_id();
            let guid = ObjectGuid::create_player(realm_id, row.guid_low as i64);

            let enum_flags = enum_character_flags_like_cpp(
                row.player_flags,
                row.at_login_flags,
                row.banned_guid,
                (!row.declined_genitive.is_empty()).then_some(row.declined_genitive.as_str()),
                self.declined_names_used_like_cpp(),
            );
            let (pet_display_id, pet_level, pet_family) = enum_character_pet_data_like_cpp(
                row.player_flags,
                row.at_login_flags,
                row.class,
                row.pet_entry,
                row.pet_display_id,
                row.pet_level,
                self.creature_template_lifecycle_store_like_cpp()
                    .map(Arc::as_ref),
            );

            // Only add to legit list if not locked
            if (enum_flags.flags
                & (CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP
                    | CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP))
                == 0
            {
                legit_guids.push(guid);
            }

            let char_info = CharacterInfo {
                guid,
                guild_club_member_id: 0,
                name: row.name,
                list_position: row.list_slot,
                race_id: row.race,
                class_id: row.class,
                sex_id: row.gender,
                experience_level: row.level,
                zone_id: row.zone,
                map_id: row.map,
                position: Position::new(row.position_x, row.position_y, row.position_z, 0.0),
                guild_guid: if row.guild_id == 0 {
                    ObjectGuid::EMPTY
                } else {
                    ObjectGuid::create_guild(HighGuid::Guild, realm_id, row.guild_id as i64)
                },
                flags: enum_flags.flags,
                flags2: enum_flags.flags2,
                flags3: 0,
                flags4: 0,
                first_login: enum_flags.first_login,
                pet_display_id,
                pet_level,
                pet_family,
                profession_ids: [0; 2],
                equipment: parse_equipment_cache(&row.equipment_cache),
                last_played_time: row.last_played_time,
                spec_id: row.active_talent_group,
                last_login_version: row.last_login_build as i32,
                override_select_screen_file_data_id: 0,
            };

            characters.push(char_info);
        }

        self.set_legit_characters(legit_guids);

        debug!(
            "Sending {} characters to account {}",
            characters.len(),
            self.account_id
        );

        // Build RaceUnlockData — from race_unlock_requirement table.
        // All WotLK races: expansion 0 (Classic) or 1 (TBC).
        // HasExpansion = true if account expansion >= required expansion.
        let account_exp = self.account_expansion;
        let race_unlock_data: Vec<RaceUnlock> = [
            (1u8, 0u8), // Human — Classic
            (2, 0),     // Orc
            (3, 0),     // Dwarf
            (4, 0),     // Night Elf
            (5, 0),     // Undead
            (6, 0),     // Tauren
            (7, 0),     // Gnome
            (8, 0),     // Troll
            (10, 1),    // Blood Elf — TBC
            (11, 1),    // Draenei — TBC
        ]
        .iter()
        .map(|&(race_id, required_exp)| RaceUnlock {
            race_id,
            has_expansion: account_exp >= required_exp,
            has_achievement: false,
            has_heritage_armor: false,
            is_locked: false,
        })
        .collect();

        self.send_packet(&EnumCharactersResult {
            success: true,
            characters,
            race_unlock_data,
        });
    }

    /// Build and send SMSG_CONNECT_TO to the client.
    pub(super) fn send_connect_to(&mut self, serial: ConnectToSerial) {
        let session_mgr = match self.session_mgr() {
            Some(mgr) => Arc::clone(mgr),
            None => {
                warn!(
                    "No session manager for ConnectTo flow (account {}), sending login directly",
                    self.account_id
                );
                self.fallback_direct_login();
                return;
            }
        };

        // Generate ConnectToKey
        let key = ConnectToKey {
            account_id: self.account_id,
            connection_type: 1, // Instance
            key: rand::thread_rng().gen_range(0..0x7FFF_FFFF_u32),
        };
        let key_raw = key.raw();
        self.set_connect_to_key(Some(key_raw));
        self.set_connect_to_serial(Some(serial));

        // Register in SessionManager — returns oneshot receiver for instance link
        let rx = session_mgr.register(self.account_id, key_raw, self.session_key.clone());
        self.set_instance_link_rx(Some(rx));

        // Build the ConnectTo payload
        let addr = self.instance_address();
        let port = self.instance_port();

        // Build where_buffer for RSA signature: [type(1B)][ip(4B)]
        let mut where_buffer = Vec::with_capacity(5);
        where_buffer.push(1u8); // IPv4
        where_buffer.extend_from_slice(&addr);

        let signature = rsa_sign_connect_to(&where_buffer, 1, port);

        let connect_to = ConnectTo {
            signature,
            address: ConnectToAddress::IPv4(addr),
            port,
            serial,
            con: 1, // Instance
            key: key_raw,
        };

        info!(
            "Sending ConnectTo (serial={:?}) to account {} for instance {}:{port}",
            serial,
            self.account_id,
            format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
        );

        self.send_packet(&connect_to);
    }

    /// Handle CMSG_REQUEST_PLAYED_TIME (0x327A).
    ///
    /// C# ref: `MiscHandler.HandlePlayedTime`.
    /// Client sends this when the player types `/played`.
    /// We respond with total and level played time in seconds.
    /// `trigger_event` mirrors the client flag (TriggerScriptEvent).
    pub async fn handle_request_played_time(&mut self, trigger_event: bool) {
        use wow_packet::packets::misc::PlayedTime;

        // Session time elapsed since login (seconds).
        let session_secs: u32 = self
            .login_time
            .map(|t| t.elapsed().as_secs() as u32)
            .unwrap_or(0);

        // Add session time on top of DB-loaded base values.
        let total_time = self.total_played_time.saturating_add(session_secs);
        let level_time = self.level_played_time.saturating_add(session_secs);

        self.send_packet(&PlayedTime {
            total_time,
            level_time,
            trigger_event,
        });
    }

    /// Handle CMSG_HOTFIX_REQUEST — client requests hotfix data.
    pub async fn handle_hotfix_request(&mut self, req: wow_packet::packets::misc::HotfixRequest) {
        info!(
            "HotfixRequest: client_build={}, data_build={}, {} hotfixes for account {}, first={:?}, last={:?}",
            req.client_build,
            req.data_build,
            req.hotfixes.len(),
            self.account_id,
            req.hotfixes.first(),
            req.hotfixes.last()
        );

        let Some(cache) = self.hotfix_blob_cache().map(Arc::clone) else {
            self.send_packet(&HotfixConnect::empty());
            return;
        };

        let mut response = HotfixConnect::empty();
        let locale_mask = hotfix_locale_mask(&self.locale);
        for push_id in &req.hotfixes {
            let Some(push) = cache.hotfix_push(*push_id) else {
                continue;
            };

            for record in &push.records {
                if record.available_locales_mask & locale_mask == 0 {
                    continue;
                }

                let mut status = record.status as u8;
                let mut size = 0u32;

                if record.status == HotfixRecordStatus::Valid {
                    if let Some(blob) = cache.get_hotfix_blob(record.table_hash, record.record_id) {
                        let start = response.content.len();
                        response.content.extend_from_slice(blob);
                        if let Some(optional_entries) = cache.get_optional_data(
                            record.table_hash,
                            record.record_id,
                            &self.locale,
                        ) {
                            for optional_data in optional_entries {
                                response
                                    .content
                                    .extend_from_slice(&optional_data.key.to_le_bytes());
                                response.content.extend_from_slice(&optional_data.data);
                            }
                        }
                        size = (response.content.len() - start) as u32;
                    } else {
                        // C++ known-store hotfixes use DB2StorageBase::WriteRecord, not raw WDC4
                        // bytes. Until Rust has that typed serializer, fail closed so the client
                        // keeps its local DB2 cache instead of parsing a malformed Valid payload.
                        status = HotfixRecordStatus::Invalid as u8;
                    }
                }

                response.hotfixes.push(HotfixConnectData {
                    id: HotfixId {
                        push_id: record.id.push_id,
                        unique_id: record.id.unique_id,
                    },
                    table_hash: record.table_hash,
                    record_id: record.record_id,
                    size,
                    status,
                });
            }
        }

        self.send_packet(&response);
    }

    /// Mark the current character as offline (#200: through the lifecycle port).
    pub(crate) async fn mark_character_offline(&self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return;
        };

        match port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::Character {
                guid_low: guid.counter() as u32,
            })
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!("Marked character offline for guid {}", guid.counter());
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to mark character offline: {reason}");
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("Character offline mark outcome is unknown: {reason}");
            }
        }
    }

    /// Trinity marks every character for the active account offline after
    /// `SMSG_LOGOUT_COMPLETE` because one account can only have one online
    /// character.  See C++ `WorldSession::LogoutPlayer`.
    pub(crate) async fn mark_character_account_offline_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = self.account_id,
                "Character account offline save skipped: lifecycle persistence port unavailable"
            );
            return;
        };

        match port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: self.account_id,
            })
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { rows } => {
                info!(
                    account = self.account_id,
                    rows, "Marked character account offline like C++"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "Failed to mark character account offline like C++: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    "Character account offline mark outcome is unknown: {reason}"
                );
            }
        }
    }

    /// Mark the account as offline in the login database when the whole
    /// WorldSession is being destroyed, matching C++ `WorldSession::~WorldSession`.
    pub(crate) async fn mark_login_account_offline_on_disconnect_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!(
                account = self.account_id,
                "Disconnect account offline save skipped: lifecycle persistence port unavailable"
            );
            return;
        };

        match port
            .mark_offline_like_cpp(PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: self.account_id,
            })
            .await
        {
            PersistenceOutcomeLikeCpp::Applied { .. } => {
                info!(
                    account = self.account_id,
                    "Marked login account offline on disconnect"
                );
            }
            PersistenceOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "Failed to mark login account offline on disconnect: {reason}"
                );
            }
            PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(
                    account = self.account_id,
                    "Login account offline mark outcome is unknown: {reason}"
                );
            }
        }
    }

    pub(crate) async fn save_account_mounts_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account mount save because canonical Player collection ownership is unresolved"
            );
            return;
        }
        let Some(rows) = self.account_mount_save_rows_like_cpp() else {
            return;
        };
        let save = AccountCollectionSaveLikeCpp::Mounts(
            rows.into_iter()
                .map(|row| AccountMountRowLikeCpp {
                    bnet_account_id: row.bnet_account_id,
                    mount_spell_id: row.mount_spell_id,
                    flags: row.flags,
                })
                .collect(),
        );
        if save.is_empty() {
            return;
        }

        match port.save_account_collection_like_cpp(save).await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account mount flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Account mount flags save outcome is unknown: {reason}"
            ),
        }
    }

    pub(crate) async fn save_account_toys_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account toy save because canonical Player collection ownership is unresolved"
            );
            return;
        }
        let Some(rows) = self.account_toy_save_rows_like_cpp() else {
            return;
        };
        let save = AccountCollectionSaveLikeCpp::Toys(
            rows.into_iter()
                .map(|row| AccountToyRowLikeCpp {
                    bnet_account_id: row.bnet_account_id,
                    item_id: row.item_id,
                    is_favorite: row.is_favorite,
                    has_fanfare: row.has_fanfare,
                })
                .collect(),
        );
        if save.is_empty() {
            return;
        }

        match port.save_account_collection_like_cpp(save).await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account toy flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Account toy flags save outcome is unknown: {reason}"
            ),
        }
    }

    pub(crate) async fn save_account_heirlooms_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account heirloom save because canonical Player collection ownership is unresolved"
            );
            return;
        }
        let Some(rows) = self.account_heirloom_save_rows_like_cpp() else {
            return;
        };
        let save = AccountCollectionSaveLikeCpp::Heirlooms(
            rows.into_iter()
                .map(|row| AccountHeirloomRowLikeCpp {
                    bnet_account_id: row.bnet_account_id,
                    item_id: row.item_id,
                    flags: row.flags,
                })
                .collect(),
        );
        if save.is_empty() {
            return;
        }

        match port.save_account_collection_like_cpp(save).await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Failed to save account heirloom flags: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = self.battlenet_account_id(),
                "Account heirloom flags save outcome is unknown: {reason}"
            ),
        }
    }

    pub(crate) async fn save_account_item_appearances_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account appearance save because canonical Player collection ownership is unresolved"
            );
            return;
        }
        let Some(plan) = self.account_item_appearance_save_plan_like_cpp() else {
            return;
        };
        if plan.is_empty() {
            return;
        }

        let bnet_account_id = self.battlenet_account_id();
        let save = AccountCollectionSaveLikeCpp::ItemAppearances {
            bnet_account_id,
            appearance_blocks: plan
                .appearance_blocks
                .into_iter()
                .map(|(block_index, mask)| AccountMaskBlockLikeCpp { block_index, mask })
                .collect(),
            favorite_inserts: plan.favorite_inserts,
            favorite_deletes: plan.favorite_deletes,
        };

        match port.save_account_collection_like_cpp(save).await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account item appearances: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Account item appearance save outcome is unknown: {reason}"
            ),
        }
    }

    pub(crate) async fn save_account_transmog_illusions_like_cpp(&self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return;
        };
        if self.player_collection_state_snapshot_like_cpp().is_none() {
            warn!(
                account = self.account_id,
                "Skipping account illusion save because canonical Player collection ownership is unresolved"
            );
            return;
        }
        let Some(plan) = self.account_transmog_illusion_save_plan_like_cpp() else {
            return;
        };
        if plan.is_empty() {
            return;
        }

        let bnet_account_id = self.battlenet_account_id();
        let save = AccountCollectionSaveLikeCpp::TransmogIllusions {
            bnet_account_id,
            illusion_blocks: plan
                .illusion_blocks
                .into_iter()
                .map(|(block_index, mask)| AccountMaskBlockLikeCpp { block_index, mask })
                .collect(),
        };

        match port.save_account_collection_like_cpp(save).await {
            PersistenceOutcomeLikeCpp::Applied { .. } => {}
            PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Failed to save account transmog illusions: {reason}"
            ),
            PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Account transmog illusion save outcome is unknown: {reason}"
            ),
        }
    }

    /// Handle ConnectToFailed — client couldn't connect to instance port.
    ///
    /// Retry with the next serial, or fall back to direct login if all retries
    /// are exhausted.
    pub async fn handle_connect_to_failed(&mut self, pkt: ConnectToFailed) {
        warn!(
            "ConnectToFailed (serial={:?}) from account {}",
            pkt.serial, self.account_id
        );

        // Clean up the pending entry from SessionManager
        if let Some(mgr) = self.session_mgr() {
            mgr.remove(self.account_id);
        }
        self.set_instance_link_rx(None);

        // Try next serial
        if let Some(next_serial) = pkt.serial.next() {
            info!("Retrying ConnectTo with serial {:?}", next_serial);
            self.send_connect_to(next_serial);
        } else {
            warn!(
                "All ConnectTo retries exhausted for account {}, aborting login like C++",
                self.account_id
            );
            self.set_player_loading(None);
            self.release_character_login_claim_like_cpp();
            self.set_connect_to_key(None);
            self.set_connect_to_serial(None);
            self.send_packet(&CharacterLoginFailed {
                code: LoginFailureReasonLikeCpp::NoWorld,
            });
        }
    }

    pub(super) fn login_known_spells_after_account_collections_like_cpp(&self) -> Vec<i32> {
        // C++ `Player::HasSpell` includes inactive, non-disabled rows, while
        // `Player::SendKnownSpells` publishes only active rows. Prefer the
        // complete PlayerSpellMap when available so the internal mirror can
        // retain lower ranks without leaking them into the login packet.
        let mut spells = self
            .complete_represented_player_spell_rows_like_cpp()
            .map(|rows| {
                rows.values()
                    .filter(|spell| {
                        spell.state != crate::session::RepresentedPlayerSpellStateLikeCpp::Removed
                            && spell.active
                            && !spell.disabled
                    })
                    .map(|spell| spell.spell_id)
                    .collect()
            })
            .unwrap_or_else(|| self.known_spells_like_cpp().to_vec());
        for mount in self.account_mount_rows_like_cpp() {
            if !spells.contains(&mount.spell_id) {
                spells.push(mount.spell_id);
            }
        }
        spells
    }

    pub(super) async fn load_account_mounts_like_cpp(&mut self) -> bool {
        self.set_account_mounts_like_cpp(Vec::new());
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return false;
        };

        let bnet_account_id = self.battlenet_account_id();
        if bnet_account_id == 0 {
            warn!(
                account = self.account_id,
                "Skipping account mount load because the game account is not linked to a Battle.net account"
            );
            return false;
        }
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Mounts {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::Mounts(rows),
            ) => rows,
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account mounts: {reason}"
                );
                return false;
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for mounts"
                );
                return false;
            }
        };

        if rows.is_empty() {
            info!(
                account = self.account_id,
                bnet_account = bnet_account_id,
                "Loaded 0 account mounts from battlenet_account_mounts"
            );
            return true;
        }

        let mut mounts = Vec::new();
        let mut skipped_invalid_spell_id = 0usize;
        let mut skipped_missing_mount_db2 = 0usize;
        for row in rows {
            let spell_id = row.mount_spell_id;
            if spell_id <= 0 {
                skipped_invalid_spell_id += 1;
                continue;
            }

            let has_mount = spell_id > 0
                && self.mount_store().is_none_or(|store| {
                    store
                        .get_by_source_spell_id_like_cpp(spell_id as u32)
                        .is_some()
                });
            if has_mount {
                mounts.push(AccountMount {
                    spell_id,
                    flags: row.flags,
                });
            } else {
                skipped_missing_mount_db2 += 1;
            }
        }

        info!(
            account = self.account_id,
            bnet_account = bnet_account_id,
            loaded = mounts.len(),
            skipped_invalid_spell_id,
            skipped_missing_mount_db2,
            "Loaded represented account mounts like C++ CollectionMgr"
        );
        self.set_account_mounts_like_cpp(mounts.clone());
        true
    }

    pub(super) async fn load_account_toys_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_toys_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
                rows,
            )) => rows
                .into_iter()
                .filter_map(|row| {
                    u32::try_from(row.item_id)
                        .ok()
                        .map(|item_id| (item_id, row.is_favorite, row.has_fanfare))
                })
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account toys: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for toys"
                );
                Vec::new()
            }
        };

        self.load_represented_account_toys_like_cpp(rows);
    }

    pub(super) async fn load_account_heirlooms_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_heirlooms_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let rows = match port
            .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id,
            })
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::Heirlooms(rows),
            ) => rows
                .into_iter()
                .filter_map(|row| {
                    u32::try_from(row.item_id)
                        .ok()
                        .map(|item_id| (item_id, row.flags))
                })
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account heirlooms: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for heirlooms"
                );
                Vec::new()
            }
        };

        self.load_represented_account_heirlooms_like_cpp(rows);
    }

    pub(super) async fn load_account_item_appearances_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_item_appearances_like_cpp([], []);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let (appearance_blocks, favorite_appearances) = match port
            .load_account_collection_like_cpp(
                AccountCollectionLoadRequestLikeCpp::ItemAppearances { bnet_account_id },
            )
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::ItemAppearances {
                    appearance_blocks,
                    favorite_appearance_ids,
                },
            ) => {
                let appearance_blocks = match appearance_blocks {
                    AccountCollectionRowsLikeCpp::Loaded(rows) => rows
                        .into_iter()
                        .map(|row| (row.block_index, row.mask))
                        .collect(),
                    AccountCollectionRowsLikeCpp::Failed { reason } => {
                        warn!(
                            account = self.account_id,
                            bnet_account = bnet_account_id,
                            "Failed to load account item appearances: {reason}"
                        );
                        Vec::new()
                    }
                };
                let favorite_appearances = match favorite_appearance_ids {
                    AccountCollectionRowsLikeCpp::Loaded(rows) => rows,
                    AccountCollectionRowsLikeCpp::Failed { reason } => {
                        warn!(
                            account = self.account_id,
                            bnet_account = bnet_account_id,
                            "Failed to load account favorite item appearances: {reason}"
                        );
                        Vec::new()
                    }
                };
                (appearance_blocks, favorite_appearances)
            }
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account item appearances: {reason}"
                );
                (Vec::new(), Vec::new())
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for item appearances"
                );
                (Vec::new(), Vec::new())
            }
        };

        self.load_represented_account_item_appearances_like_cpp(
            appearance_blocks,
            favorite_appearances,
        );
    }

    pub(super) async fn load_account_transmog_illusions_like_cpp(&mut self) {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            self.load_represented_account_transmog_illusions_like_cpp([]);
            return;
        };

        let bnet_account_id = self.battlenet_account_id();
        let illusion_blocks = match port
            .load_account_collection_like_cpp(
                AccountCollectionLoadRequestLikeCpp::TransmogIllusions { bnet_account_id },
            )
            .await
        {
            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                AccountCollectionLoadedLikeCpp::TransmogIllusions { illusion_blocks },
            ) => illusion_blocks
                .into_iter()
                .map(|row| (row.block_index, row.mask))
                .collect(),
            AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Failed to load account transmog illusions: {reason}"
                );
                Vec::new()
            }
            AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    bnet_account = bnet_account_id,
                    "Player lifecycle port returned the wrong account collection for transmog illusions"
                );
                Vec::new()
            }
        };

        self.load_represented_account_transmog_illusions_like_cpp(illusion_blocks);
    }
}
