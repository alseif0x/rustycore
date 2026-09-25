use super::*;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SwapInvItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_swap_inv_item",
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::SwapInvItem::read(&mut pkt) {
                    Ok(swap) => {
                        session
                            .handle_swap_inv_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                swap,
                            )
                            .await
                    }
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
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::AutoEquipItem::read(&mut pkt) {
                    Ok(equip) => {
                        session
                            .handle_auto_equip_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                equip,
                            )
                            .await
                    }
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
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::AutoEquipItemSlot::read(&mut pkt) {
                    Ok(equip) => {
                        session
                            .handle_auto_equip_item_slot_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                equip,
                            )
                            .await
                    }
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
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::SwapItem::read(&mut pkt) {
                    Ok(swap) => {
                        session
                            .handle_swap_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                swap,
                            )
                            .await
                    }
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
        handler: |session, catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::item::AutoStoreBagItem::read(&mut pkt) {
                    Ok(store) => {
                        session
                            .handle_auto_store_bag_item_with_generator_like_cpp(
                                catalogs.id_generators.item.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                store,
                            )
                            .await
                    }
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
