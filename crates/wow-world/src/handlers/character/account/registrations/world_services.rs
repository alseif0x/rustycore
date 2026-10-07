use super::*;

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::BuyItem::read(&mut pkt) {
                    Ok(buy) => {
                        session
                            .handle_buy_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                buy,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read BuyItem: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBackItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_back_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::BuyBackItem::read(&mut pkt) {
                    Ok(buyback) => {
                        session
                            .handle_buy_back_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                buyback,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read BuyBackItem: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SellItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_sell_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::SellItem::read(&mut pkt) {
                    Ok(sell) => {
                        session
                            .handle_sell_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                sell,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read SellItem: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ItemPurchaseRefund,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_item_purchase_refund",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::ItemPurchaseRefund::read(&mut pkt) {
                    Ok(refund) => {
                        session
                            .handle_item_purchase_refund_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                refund,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read ItemPurchaseRefund: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutobankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autobank_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AutoBankItem::read(&mut pkt) {
                    Ok(packet) => {
                        session
                            .handle_autobank_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                packet,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read AutobankItem: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::AutostoreBankItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_autostore_bank_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::AutoStoreBankItem::read(&mut pkt) {
                    Ok(packet) => {
                        session
                            .handle_autostore_bank_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                packet,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read AutostoreBankItem: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BuyBankSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_buy_bank_slot",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::BuyBankSlot::read(&mut pkt) {
                    Ok(buy) => {
                        session
                            .handle_buy_bank_slot_with_prices_and_generator_like_cpp(
                                catalogs.bank_bag_slot_prices.as_ref(),
                                catalogs.id_generators.item.as_ref(),
                                buy,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read BuyBankSlot: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BinderActivate,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_binder_activate",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => {
                        session
                            .handle_binder_activate_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                hello,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read BinderActivate: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RepairItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_repair_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::misc::RepairItem::read(&mut pkt) {
                    Ok(repair) => {
                        session
                            .handle_repair_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                repair,
                            )
                            .await
                    }
                    Err(e) => tracing::warn!("Failed to read RepairItem: {e}"),
                }
            })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusMultipleQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_status_multiple_query",
        handler: |session, catalogs, _pkt| {
            Box::pin(async move { session.handle_quest_giver_status_multiple_query_with_catalog_like_cpp(catalogs.quest_info.as_ref()).await })
        },
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusTrackedQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_quest_giver_status_tracked_query",
        handler: |session, catalogs, pkt| {
            Box::pin(async move { session.handle_quest_giver_status_tracked_query_with_catalog_like_cpp(catalogs.quest_info.as_ref(), pkt).await })
        },
    }
}
