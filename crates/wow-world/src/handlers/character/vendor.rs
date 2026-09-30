// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor buy/sell/buyback, extended cost, repair and trainer interaction.

use super::*;

mod buy;
mod buyback;
mod list_inventory;
pub(super) mod rules;
mod sell;

use rules::{VendorBuyItem, vendor_currency_type_is_known};

mod repair;
mod refund;
mod catalog_resolution;

impl WorldSession {
    #[cfg(test)]
    pub async fn handle_sell_item(&mut self, sell: SellItem) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.handle_sell_item_with_generator_like_cpp(generator.as_ref(), sell)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_item_purchase_refund(&mut self, refund: ItemPurchaseRefund) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.handle_item_purchase_refund_with_generator_like_cpp(generator.as_ref(), refund)
            .await;
    }

    pub(crate) fn send_represented_creature_trainer_gossip_menu_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
        entry: u32,
        npc_flags: u32,
    ) -> bool {
        let mut gossip_options = Vec::new();
        let mut stored_options = Vec::new();
        if !add_represented_trainer_gossip_option_if_missing_like_cpp(
            &mut gossip_options,
            &mut stored_options,
            npc_flags,
        ) {
            return false;
        }

        let gossip_text = if npc_flags & NPCFlags1::QUEST_GIVER.bits() != 0 {
            self.represented_creature_gossip_text_like_cpp(entry)
        } else {
            Vec::new()
        };

        if !self.replace_player_gossip_options_like_cpp(stored_options) {
            return false;
        }
        self.set_player_interaction_source_like_cpp(npc_guid);
        self.send_packet(&GossipMessage {
            gossip_guid: npc_guid,
            gossip_id: 0,
            friendship_faction_id: 0,
            text_id: Some(DEFAULT_GOSSIP_MESSAGE_LIKE_CPP),
            broadcast_text_id: None,
            gossip_options,
            gossip_text,
        });
        true
    }

    /// CMSG_TABARD_VENDOR_ACTIVATE — player talks to a tabard designer.
    /// C++ refs: `HandleTabardVendorActivateOpcode` /
    /// `SendTabardVendorActivate` (`Handlers/NPCHandler.cpp:49-91`).
    pub async fn handle_tabard_vendor_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::NpcInteractionOpenResult;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "TabardVendorActivate {:?} account {}",
            guid, self.account_id
        );
        self.send_packet(&NpcInteractionOpenResult::new(guid, 14)); // GuildTabardVendor
    }
}
